"""Ventana: agregar archivos o carpetas y anonimizar. Todo el proceso es local; el original no se modifica."""

from __future__ import annotations

import queue
import threading
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, simpledialog, ttk

from . import __version__
from . import flowkit as ui
from .engine import SUPPORTED, report_text, run
from .textengine import Options

FILETYPES = [
    (
        "Datos del Asistente/Descargador",
        "*.xlsx *.xlsm *.xls *.html *.htm *.json *.jsonl *.csv *.tsv *.ics *.txt *.docx *.zip",
    ),
    ("Todos los archivos", "*.*"),
]
MODES = [("estricto", "Estricto"), ("conservador", "Conservador")]
ABOUT = (
    "Anonimizador CSMP {version}\n\n"
    "Acepta: {supported}.\n\n"
    "Estricto omite los textos libres completos. Conservador mantiene observaciones sustituyendo identidades "
    "detectables.\n\nEl original no se modifica y nada sale del equipo. Revisa la copia antes de compartirla: "
    "ninguna modalidad garantiza anonimización absoluta."
)


class App:
    def __init__(self, root):
        self.root = root
        self.inputs = []
        self.events = queue.Queue()
        self.busy = False
        self.report = ""
        self.mode = tk.StringVar(value="estricto")
        self.shift = tk.StringVar(value="0")
        self.seed = tk.StringVar()
        self.birth = tk.BooleanVar(value=False)
        self.strict_zip = tk.BooleanVar(value=False)
        self.out_dir = tk.StringVar()
        self.status = tk.StringVar()
        self.window = ui.Window(root, "Anonimizador CSMP", "Anonimizar", self.start, size="760x520", minsize=(600, 420))
        top = self.window.top
        self.modes = ui.Chips(top.extra, MODES, multi=False, command=lambda: self.mode.set(self.modes.get()[0]))
        self.modes.set([self.mode.get()])
        self.modes.pack(side="left", pady=(6, 0))
        top.add("Agregar archivos…", self.pick_files)
        top.add("Agregar carpeta…", self.pick_folder)
        top.add("Quitar todo", self.clear_inputs)
        top.add("Carpeta de destino…", self.pick_out, separator=True)
        top.add("Desplazar fechas…", self.ask_shift)
        top.add("Frase de semilla…", self.ask_seed)
        top.add_check("Fechas de nacimiento al 1 de enero", self.birth)
        top.add_check("Un ZIP con archivos no admitidos falla completo", self.strict_zip)
        top.add("Ver último informe", self.show_report, separator=True)
        top.add("Acerca de", self.about)
        footer = self.window.footer
        self.add_folder = footer.add("Agregar carpeta…", self.pick_folder)
        self.add_files = footer.add("Agregar archivos…", self.pick_files)
        self.go = footer.primary
        body = ttk.Frame(self.window.body, padding=(16, 14))
        body.pack(fill="both", expand=True)
        self.empty = ttk.Label(body, text="Agrega archivos o carpetas del Descargador o del Asistente.", style="Muted.TLabel")
        self.list = ui.ProgressList(body, height=12)
        self.status.trace_add("write", self._status_changed)
        self.out_dir.trace_add("write", lambda *_: self.refresh())
        self.refresh()

    def _status_changed(self, *_):
        text = self.status.get()
        self.window.banner.show(text, bad=not text.startswith(tuple("123456789"))) if text else self.window.banner.hide()

    def refresh(self):
        if self.inputs:
            self.empty.pack_forget()
            self.list.pack(fill="both", expand=True)
            # Solo número y tipo: el nombre y la ruta pueden identificar a personas.
            self.list.plan(
                [
                    (n, f"Entrada {n} · " + ("carpeta" if Path(p).is_dir() else (Path(p).suffix.lower() or "archivo")))
                    for n, p in enumerate(self.inputs, 1)
                ]
            )
        else:
            self.list.pack_forget()
            self.empty.pack(anchor="w")
        where = "en la carpeta elegida" if (self.out_dir.get() or "").strip() else "junto al original"
        count = len(self.inputs)
        self.window.footer.set(
            f"{count} entrada{'s' if count != 1 else ''} · copia {where}" if count else "",
            enabled=bool(count) and not self.busy,
        )

    def pick_files(self):
        if self.busy:  # el menú «Más» sigue activo mientras se procesa
            return
        names = filedialog.askopenfilenames(parent=self.root, title="Agregar archivos", filetypes=FILETYPES)
        self.inputs += [Path(n) for n in names if Path(n) not in self.inputs]
        self.refresh()

    def pick_folder(self):
        if self.busy:
            return
        name = filedialog.askdirectory(parent=self.root, title="Agregar carpeta")
        if name and Path(name) not in self.inputs:
            self.inputs.append(Path(name))
        self.refresh()

    def pick_out(self):
        name = filedialog.askdirectory(parent=self.root, title="Carpeta de destino")
        if name:
            self.out_dir.set(name)

    def ask_shift(self):
        try:
            actual = int((self.shift.get() or "0").strip())
        except ValueError:
            actual = 0  # un valor escrito a mano inválido no debe romper el diálogo
        value = simpledialog.askinteger(
            "Desplazar fechas", "Días (0 = sin desplazar):", parent=self.root, initialvalue=actual
        )
        if value is not None:
            self.shift.set(str(value))

    def ask_seed(self):
        value = simpledialog.askstring("Frase de semilla", "Misma frase = mismos seudónimos entre ejecuciones:",
                                       parent=self.root, show="•")
        if value is not None:
            self.seed.set(value)

    def about(self):
        self.show_text("Acerca de", ABOUT.format(version=__version__, supported=SUPPORTED))

    def show_report(self):
        self.show_text("Informe de anonimización", self.report or "Todavía no hay informe en esta sesión.")

    def show_text(self, title, text):
        window = tk.Toplevel(self.root)
        window.title(title)
        box = tk.Text(window, wrap="word", width=72, height=20, borderwidth=0, padx=12, pady=12)
        box.insert("1.0", text)
        box.configure(state="disabled")
        box.pack(fill="both", expand=True)

    def clear_inputs(self):
        if self.busy:
            return
        self.inputs = []
        self.window.banner.hide()
        self.refresh()

    def lock(self, locked):
        self.busy = locked
        for widget in (self.add_files, self.add_folder):
            widget.state(["disabled"] if locked else ["!disabled"])
        self.refresh() if not locked else self.go.state(["disabled"])

    def write(self, text):
        self.report = text

    def start(self):
        if not self.inputs:
            self.status.set("Debes agregar al menos un archivo o carpeta.")
            return
        try:
            days = int(self.shift.get().strip() or 0)
        except ValueError:
            self.status.set("Los días a desplazar deben ser un número entero.")
            return
        options = Options(
            mode=self.mode.get(),
            shift_days=days,
            seed=self.seed.get(),
            skip_unsupported=not self.strict_zip.get(),
            generalize_birth=self.birth.get(),
        )
        out_dir = Path(self.out_dir.get()) if (self.out_dir.get() or "").strip() else None
        self.lock(True)
        self.window.banner.hide()
        self.write("")

        def work():
            try:
                result = run(
                    list(self.inputs),
                    options,
                    out_dir,
                    lambda label, step, total: self.events.put(("progress", (label, step, total))),
                )
                self.events.put(("done", (result, options)))
            except Exception:
                self.events.put(("error", None))

        threading.Thread(target=work, daemon=True).start()
        self.root.after(100, self.poll)

    def poll(self):
        try:
            while True:
                event, value = self.events.get_nowait()
                if event == "progress":
                    label, step, total = value
                    self.list.bar.configure(maximum=max(1, total), value=step)
                    continue
                self.lock(False)
                if event == "error":
                    self.status.set("No se pudo completar el proceso. Los originales siguen intactos.")
                    return
                result, options = value
                self.list.clear()
                for number, item in enumerate(result.items, 1):
                    self.list.set(number, "ok" if item.ok else "bad",
                                  f"{item.changed} cambios" if item.ok else item.detail,
                                  name=f"{item.label} · {item.kind or 'sin formato'}")
                made = len(result.outputs)
                failed = sum(not item.ok for item in result.items)
                self.status.set(
                    f"{made} copia(s) creada(s)"
                    + (f"; {failed} no se pudo/pudieron crear" if failed else "")
                    + ". Revisión humana obligatoria antes de compartir."
                )
                self.write(report_text(result, options) + "\nUbicación: " + (str(result.outputs[0].parent) if made else "-"))
                folder = result.outputs[0].parent if made else None
                self.window.banner.show(
                    self.status.get(), bad=not made,
                    action=(lambda: _open(folder)) if folder else None, action_text="Abrir carpeta",
                )
                return
        except queue.Empty:
            self.root.after(100, self.poll)


def _open(path):
    import os
    import subprocess
    import sys

    if not Path(path).is_dir():
        return
    if sys.platform == "win32":
        os.startfile(path)  # noqa: S606 - carpeta local recién creada por la herramienta
    else:
        subprocess.Popen(["xdg-open", str(path)])


def main():
    root = tk.Tk()
    App(root)
    root.mainloop()
    return 0
