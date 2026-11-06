#!/usr/bin/env python3
"""zip2exe: toma el ZIP de un repositorio Python y genera un ejecutable con PyInstaller.

Uso:
    python zip2exe.py repo.zip [--entry main.py] [--name App] [--windowed] [--icon app.ico]
                              [--onedir] [--out dist] [--add-data carpeta] [--keep-work]

Solo usa la biblioteca estándar. Crea un venv aislado por build, instala requirements
del repo + PyInstaller y compila. PyInstaller no hace cross-compile: para obtener .exe
hay que correrlo en Windows.
"""
from __future__ import annotations

import argparse
import ast
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import zipfile
from pathlib import Path

IS_WIN = os.name == "nt"
ENTRY_CANDIDATES = ("main.py", "app.py", "run.py", "__main__.py", "cli.py", "gui.py", "main.pyw", "app.pyw")
SKIP_DIRS = {".git", ".github", "venv", ".venv", "env", "__pycache__", "build", "dist", "tests", "test", "docs"}
GUI_MODULES = {"tkinter", "PyQt5", "PyQt6", "PySide2", "PySide6", "wx", "customtkinter", "kivy", "flet", "pygame"}


class BuildError(Exception):
    pass


def log(msg: str) -> None:
    print(f"[zip2exe] {msg}", flush=True)


def safe_extract(zip_path: Path, dest: Path) -> None:
    dest = dest.resolve()
    with zipfile.ZipFile(zip_path) as zf:
        for member in zf.infolist():
            target = (dest / member.filename).resolve()
            if target != dest and dest not in target.parents:
                raise BuildError(f"Entrada ZIP fuera del destino (zip-slip): {member.filename}")
        zf.extractall(dest)


def project_root(extract_dir: Path) -> Path:
    entries = [p for p in extract_dir.iterdir() if p.name not in {"__MACOSX"}]
    if len(entries) == 1 and entries[0].is_dir():
        return entries[0]
    return extract_dir


def has_main_guard(path: Path) -> bool:
    try:
        tree = ast.parse(path.read_text(encoding="utf-8", errors="replace"))
    except SyntaxError:
        return False
    for node in tree.body:
        if isinstance(node, ast.If):
            src = ast.unparse(node.test)
            if "__name__" in src and "__main__" in src:
                return True
    return False


def pyproject_entry(root: Path) -> Path | None:
    pp = root / "pyproject.toml"
    if not pp.is_file():
        return None
    try:
        data = tomllib.loads(pp.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError:
        return None
    scripts = data.get("project", {}).get("scripts", {}) or \
        data.get("tool", {}).get("poetry", {}).get("scripts", {})
    for spec in scripts.values():
        if not isinstance(spec, str) or ":" not in spec:
            continue
        module = spec.split(":", 1)[0].strip()
        rel = Path(*module.split("."))
        for base in (root, root / "src"):
            for cand in (base / rel.with_suffix(".py"), base / rel / "__main__.py"):
                if cand.is_file():
                    return cand
    return None


def find_entry(root: Path, explicit: str | None) -> Path:
    if explicit:
        p = (root / explicit).resolve()
        if not p.is_file():
            raise BuildError(f"Entry point no existe en el repo: {explicit}")
        return p

    for name in ENTRY_CANDIDATES:
        if (root / name).is_file():
            return root / name

    pe = pyproject_entry(root)
    if pe:
        return pe

    for base in (root / "src", root):
        if base.is_dir():
            for pkg in sorted(base.iterdir()):
                if pkg.is_dir() and (pkg / "__main__.py").is_file():
                    return pkg / "__main__.py"

    guarded = sorted(p for p in list(root.glob("*.py")) + list(root.glob("*.pyw")) if has_main_guard(p))
    if len(guarded) == 1:
        return guarded[0]
    if len(guarded) > 1:
        opts = ", ".join(p.name for p in guarded)
        raise BuildError(f"Varios entry points posibles ({opts}). Indica uno con --entry.")

    pys = sorted(root.glob("*.py"))
    if len(pys) == 1:
        return pys[0]
    raise BuildError("No se detectó entry point. Indica uno con --entry (ruta relativa a la raíz del repo).")


def imports_of(path: Path) -> set[str]:
    try:
        tree = ast.parse(path.read_text(encoding="utf-8", errors="replace"))
    except SyntaxError:
        return set()
    mods: set[str] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            mods.update(a.name.split(".")[0] for a in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module and node.level == 0:
            mods.add(node.module.split(".")[0])
    return mods


def is_gui(entry: Path) -> bool:
    return entry.suffix == ".pyw" or bool(imports_of(entry) & GUI_MODULES)


def venv_python(venv_dir: Path) -> Path:
    return venv_dir / ("Scripts/python.exe" if IS_WIN else "bin/python")


def run(cmd: list[str | Path], cwd: Path | None = None) -> None:
    log("$ " + " ".join(str(c) for c in cmd))
    r = subprocess.run([str(c) for c in cmd], cwd=cwd)
    if r.returncode != 0:
        raise BuildError(f"Comando falló (código {r.returncode}).")


def install_deps(py: Path, root: Path) -> None:
    run([py, "-m", "pip", "install", "--upgrade", "pip", "wheel", "pyinstaller"])
    req = next((root / n for n in ("requirements.txt", "requirements-prod.txt", "requirements/base.txt")
                if (root / n).is_file()), None)
    if req:
        run([py, "-m", "pip", "install", "-r", req], cwd=root)
    elif (root / "pyproject.toml").is_file() or (root / "setup.py").is_file():
        run([py, "-m", "pip", "install", root], cwd=root)
    else:
        log("Sin requirements.txt / pyproject.toml / setup.py: no se instalan dependencias extra.")


def default_data_dirs(root: Path) -> list[Path]:
    names = ("assets", "data", "resources", "res", "static", "templates", "images", "img", "icons", "config")
    return [root / n for n in names if (root / n).is_dir()]


def find_icon(root: Path) -> Path | None:
    for pat in ("*.ico", "assets/*.ico", "resources/*.ico", "icons/*.ico", "img/*.ico"):
        hits = sorted(root.glob(pat))
        if hits:
            return hits[0]
    return None


def sanitize_name(name: str) -> str:
    name = re.sub(r"-(main|master)$", "", name)
    return re.sub(r"[^A-Za-z0-9._-]", "_", name) or "app"


def build(args: argparse.Namespace) -> Path:
    zip_path = Path(args.zip).resolve()
    if not zip_path.is_file() or not zipfile.is_zipfile(zip_path):
        raise BuildError(f"No es un ZIP válido: {zip_path}")

    out_dir = Path(args.out).resolve() if args.out else zip_path.parent / "dist"
    out_dir.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="zip2exe_"))
    log(f"Directorio de trabajo: {work}")

    try:
        src_dir = work / "src"
        src_dir.mkdir()
        safe_extract(zip_path, src_dir)
        root = project_root(src_dir)
        log(f"Raíz del proyecto: {root.name}")

        default_name = root.name if root != src_dir else zip_path.stem
        name = sanitize_name(args.name or default_name)

        spec_files = sorted(root.glob("*.spec"))
        use_spec = spec_files[0] if spec_files and not args.ignore_spec else None

        venv_dir = work / "venv"
        log("Creando venv aislado…")
        run([sys.executable, "-m", "venv", venv_dir])
        py = venv_python(venv_dir)
        install_deps(py, root)

        pyi_dist = work / "dist"
        cmd: list[str | Path] = [py, "-m", "PyInstaller", "--noconfirm", "--clean",
                                 "--distpath", pyi_dist, "--workpath", work / "build"]

        if use_spec:
            log(f"Usando spec del repo: {use_spec.name}")
            cmd.append(use_spec)
        else:
            entry = find_entry(root, args.entry)
            log(f"Entry point: {entry.relative_to(root)}")
            windowed = args.windowed or (not args.console and is_gui(entry))
            log(f"Modo: {'ventana (sin consola)' if windowed else 'consola'}")

            cmd += ["--specpath", work, "--name", name, "--onedir" if args.onedir else "--onefile"]
            if windowed:
                cmd.append("--windowed")
            for p in dict.fromkeys((root, root / "src", entry.parent)):
                if p.is_dir():
                    cmd += ["--paths", p]

            icon = Path(args.icon).resolve() if args.icon else find_icon(root)
            if icon:
                log(f"Ícono: {icon.name}")
                cmd += ["--icon", icon]

            data_dirs = [root / d for d in args.add_data] if args.add_data else default_data_dirs(root)
            for d in data_dirs:
                if not d.exists():
                    raise BuildError(f"--add-data no existe: {d.relative_to(root)}")
                dest = d.name if d.is_dir() else "."
                cmd += ["--add-data", f"{d}{os.pathsep}{dest}"]
                log(f"Datos incluidos: {d.relative_to(root)}")

            for h in args.hidden_import:
                cmd += ["--hidden-import", h]
            cmd.append(entry)

        run(cmd, cwd=root)

        produced = list(pyi_dist.iterdir())
        if not produced:
            raise BuildError("PyInstaller no generó salida.")
        results = []
        for item in produced:
            target = out_dir / item.name
            if target.is_dir():
                shutil.rmtree(target)
            elif target.exists():
                target.unlink()
            shutil.move(str(item), target)
            results.append(target)
        main_out = results[0]
        if main_out.is_dir():
            exe = next(main_out.glob("*.exe" if IS_WIN else name), main_out)
            main_out = exe
        return main_out
    finally:
        if args.keep_work:
            log(f"Directorio de trabajo conservado: {work}")
        else:
            shutil.rmtree(work, ignore_errors=True)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    ap = argparse.ArgumentParser(description="Genera un ejecutable a partir del ZIP de un repositorio Python.")
    ap.add_argument("zip", help="Ruta al ZIP del repositorio (ej. descargado con 'Code > Download ZIP').")
    ap.add_argument("--entry", help="Script principal, relativo a la raíz del repo (autodetecta si se omite).")
    ap.add_argument("--name", help="Nombre del ejecutable (default: nombre del repo).")
    ap.add_argument("--out", help="Carpeta de salida (default: 'dist' junto al ZIP).")
    ap.add_argument("--icon", help="Archivo .ico (default: primer .ico encontrado en el repo).")
    ap.add_argument("--onedir", action="store_true", help="Carpeta con exe + DLLs en vez de un único exe.")
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument("--windowed", action="store_true", help="Forzar sin consola (apps GUI).")
    mode.add_argument("--console", action="store_true", help="Forzar con consola.")
    ap.add_argument("--add-data", action="append", default=[], metavar="RUTA",
                    help="Carpeta/archivo del repo a incluir (repetible). Reemplaza la autodetección.")
    ap.add_argument("--hidden-import", action="append", default=[], metavar="MOD",
                    help="Módulo que PyInstaller no detecta (repetible).")
    ap.add_argument("--ignore-spec", action="store_true", help="Ignorar .spec presente en el repo.")
    ap.add_argument("--keep-work", action="store_true", help="No borrar el directorio temporal (debug).")
    return ap.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if not IS_WIN:
        log("AVISO: no es Windows; se generará un binario nativo de este SO, no un .exe.")
    try:
        result = build(args)
    except BuildError as e:
        log(f"ERROR: {e}")
        return 1
    log(f"OK -> {result}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
