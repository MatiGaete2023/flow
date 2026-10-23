"""Ventana Tkinter: elegir archivos o carpetas, modo y destino. Todo el proceso es local."""

from __future__ import annotations

import queue
import threading
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, ttk

from . import __version__
from .engine import SUPPORTED, report_text, run
from .textengine import Options

FILETYPES = [
    (
        "Datos del Asistente/Descargador",
        "*.xlsx *.xlsm *.xls *.html *.htm *.json *.jsonl *.csv *.tsv *.ics *.txt *.docx *.zip",
    ),
    ("Todos los archivos", "*.*"),
]


class App:
    def __init__(self, root):
        self.root = root
        root.title(f"Anonimizador CSMP {__version__}")
        root.geometry("720x640")
        root.minsize(640, 560)
        self.inputs = []
        self.events = queue.Queue()
        self.mode = tk.StringVar(value="estricto")
        self.shift = tk.StringVar(value="0")
        self.seed = tk.StringVar()
        self.birth = tk.BooleanVar(value=False)
        self.strict_zip = tk.BooleanVar(value=False)
        self.out_dir = tk.StringVar()
        self.status = tk.StringVar(value="Agrega archivos o una carpeta para comenzar.")
        frame = ttk.Frame(root, padding=16)
        frame.pack(fill="both", expand=True)
        ttk.Label(frame, text="Anonimizar copias locales", font=("Segoe UI", 14, "bold")).pack(anchor="w")
        ttk.Label(
            frame,
            text="Acepta lo que entregan el Descargador y el Asistente: " + SUPPORTED + ".\n"
            "El original no se modifica y nada sale del equipo.",
            wraplength=660,
        ).pack(anchor="w", pady=(2, 8))
        buttons = ttk.Frame(frame)
        buttons.pack(anchor="w")
        self.add_files = ttk.Button(buttons, text="Agregar archivos…", command=self.pick_files)
        self.add_files.pack(side="left")
        self.add_folder = ttk.Button(buttons, text="Agregar carpeta…", command=self.pick_folder)
        self.add_folder.pack(side="left", padx=6)
        self.clear = ttk.Button(buttons, text="Quitar todo", command=self.clear_inputs)
        self.clear.pack(side="left")
        self.listing = tk.Listbox(frame, height=6, activestyle="none")
        self.listing.pack(fill="x", pady=(8, 8))
        mode = ttk.LabelFrame(frame, text="Modo", padding=8)
        mode.pack(fill="x")
        ttk.Radiobutton(
            mode, text="Estricto (recomendado): omite los textos libres completos", value="estricto", variable=self.mode
        ).pack(anchor="w")
        ttk.Radiobutton(
            mode,
            text="Conservador: mantiene observaciones sustituyendo identidades detectables",
            value="conservador",
            variable=self.mode,
        ).pack(anchor="w")
        more = ttk.LabelFrame(frame, text="Opciones", padding=8)
        more.pack(fill="x", pady=(8, 0))
        ttk.Label(more, text="Desplazar fechas (días):").grid(row=0, column=0, sticky="w")
        ttk.Entry(more, textvariable=self.shift, width=8).grid(row=0, column=1, sticky="w", padx=6)
        ttk.Label(more, text="Frase de semilla (opcional):").grid(row=0, column=2, sticky="w", padx=(12, 0))
        ttk.Entry(more, textvariable=self.seed, width=22, show="•").grid(row=0, column=3, sticky="w", padx=6)
        ttk.Checkbutton(more, text="Fechas de nacimiento al 1 de enero", variable=self.birth).grid(
            row=1, column=0, columnspan=2, sticky="w", pady=(6, 0)
        )
        ttk.Checkbutton(more, text="Un ZIP con archivos no admitidos falla completo", variable=self.strict_zip).grid(
            row=1, column=2, columnspan=2, sticky="w", pady=(6, 0)
        )
        dest = ttk.Frame(frame)
        dest.pack(fill="x", pady=(8, 0))
        ttk.Label(dest, text="Carpeta de destino:").pack(side="left")
        ttk.Entry(dest, textvariable=self.out_dir).pack(side="left", fill="x", expand=True, padx=6)
        ttk.Button(dest, text="Elegir…", command=self.pick_out).pack(side="left")
        ttk.Label(frame, text="Vacío = junto al original.", foreground="#555").pack(anchor="w")
        self.go = ttk.Button(frame, text="Anonimizar", command=self.start)
        self.go.pack(anchor="w", pady=(10, 0))
        self.progress = ttk.Progressbar(frame, maximum=100)
        self.progress.pack(fill="x", pady=(8, 0))
        ttk.Label(frame, textvariable=self.status, wraplength=660).pack(anchor="w", pady=(6, 0))
        self.log = tk.Text(frame, height=8, state="disabled", wrap="word")
        self.log.pack(fill="both", expand=True, pady=(6, 0))
        ttk.Label(
            frame,
            text="Revisa manualmente la copia antes de compartirla: ninguna modalidad garantiza anonimización absoluta.",
            foreground="#6b4d09",
            wraplength=660,
        ).pack(anchor="w", pady=(6, 0))

    def refresh(self):
        self.listing.delete(0, "end")
        for number, path in enumerate(self.inputs, 1):
            # Solo número y tipo: el nombre y la ruta pueden identificar a personas.
            kind = "carpeta" if path.is_dir() else (path.suffix.lower() or "archivo")
            self.listing.insert("end", f"Entrada {number}: {kind}")

    def pick_files(self):
        names = filedialog.askopenfilenames(parent=self.root, title="Selecciona archivos locales", filetypes=FILETYPES)
        self.inputs += [Path(n) for n in names if Path(n) not in self.inputs]
        self.refresh()

    def pick_folder(self):
        name = filedialog.askdirectory(parent=self.root, title="Selecciona una carpeta local")
        if name and Path(name) not in self.inputs:
            self.inputs.append(Path(name))
        self.refresh()

    def pick_out(self):
        name = filedialog.askdirectory(parent=self.root, title="Carpeta de destino")
        if name:
            self.out_dir.set(name)

    def clear_inputs(self):
        self.inputs = []
        self.refresh()

    def lock(self, locked):
        state = ["disabled"] if locked else ["!disabled"]
        for widget in (self.go, self.add_files, self.add_folder, self.clear):
            widget.state(state)

    def write(self, text):
        self.log.configure(state="normal")
        self.log.delete("1.0", "end")
        self.log.insert("1.0", text)
        self.log.configure(state="disabled")

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
        out_dir = Path(self.out_dir.get()) if self.out_dir.get().strip() else None
        self.lock(True)
        self.status.set("Procesando localmente; ningún dato se envía fuera del equipo.")
        self.write("")

        def work():
            try:
                result = run(
                    list(self.inputs),
                    options,
                    out_dir,
                    lambda label, step, total: self.events.put(("progress", (step, total))),
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
                    step, total = value
                    self.progress["value"] = 100 * step / max(1, total)
                    continue
                self.lock(False)
                if event == "error":
                    self.status.set("No se pudo completar el proceso. Los originales siguen intactos.")
                else:
                    result, options = value
                    self.progress["value"] = 100
                    made = len(result.outputs)
                    failed = sum(not item.ok for item in result.items)
                    self.status.set(
                        f"{made} copia(s) creada(s)"
                        + (f"; {failed} no se pudo/pudieron crear" if failed else "")
                        + ". Revisión humana obligatoria antes de compartir."
                    )
                    self.write(
                        report_text(result, options)
                        + "\nUbicación: "
                        + (str(result.outputs[0].parent) if made else "-")
                    )
                return
        except queue.Empty:
            self.root.after(100, self.poll)


def main():
    root = tk.Tk()
    App(root)
    root.mainloop()
    return 0
