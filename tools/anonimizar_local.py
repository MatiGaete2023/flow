"""Ventana local para anonimizar Excel CSMP y ZIP CSMP_CAPTURE_V1.

Windows 10 + Python 3.12–3.14 + dependencias instaladas de flow.
Sin navegador, red, credenciales, registros ni escritura en RUS/SITFA.
"""
from __future__ import annotations

import queue
import secrets
import sys
import threading
from pathlib import Path
import tkinter as tk
from tkinter import filedialog, ttk

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))

from nurus.personal.anonymize import anonymize
from nurus.personal.anonymize_package import anonymize_capture


def destination(source):
    extension = '.zip' if source.suffix.lower() == '.zip' else '.xlsx'
    for _ in range(10):
        new = source.parent / ('Copia_CSMP_anon_' + secrets.token_hex(6) + extension)
        if not new.exists():
            return new
    raise FileExistsError('No se pudo reservar un nombre de salida.')


class App:
    def __init__(self, root):
        self.root = root
        root.title('Anonimizador local CSMP / SITFA')
        root.resizable(False, False)
        root.geometry('535x315')
        self.source = None
        self.events = queue.Queue()
        self.mode = tk.StringVar(value='conservador')
        self.status = tk.StringVar(value='Selecciona un archivo Excel o ZIP para comenzar.')
        self.chosen = tk.StringVar(value='Ningún archivo seleccionado.')
        frame = ttk.Frame(root, padding=18)
        frame.pack(fill='both', expand=True)
        ttk.Label(frame, text='Anonimizar una copia local', font=('Segoe UI', 13, 'bold')).pack(anchor='w')
        ttk.Label(frame, text='El original no se modifica. No se utilizan servicios externos.').pack(anchor='w', pady=(4, 10))
        self.select = ttk.Button(frame, text='Seleccionar Excel o ZIP', command=self.pick)
        self.select.pack(anchor='w')
        ttk.Label(frame, textvariable=self.chosen).pack(anchor='w', pady=(5, 12))
        ttk.Radiobutton(frame, text='Conservar observaciones útiles (sustituir identidades detectables)',
                        value='conservador', variable=self.mode).pack(anchor='w')
        ttk.Radiobutton(frame, text='Estricto: omitir textos libres completos',
                        value='estricto', variable=self.mode).pack(anchor='w', pady=(5, 10))
        self.go = ttk.Button(frame, text='Anonimizar', command=self.start)
        self.go.pack(anchor='w')
        ttk.Label(frame, textvariable=self.status, wraplength=495).pack(anchor='w', pady=(10, 0))
        ttk.Label(frame, text='Revisa manualmente la copia antes de compartirla.\n'
                             'El modo conservador no garantiza anonimización absoluta.',
                  foreground='#6b4d09', wraplength=495).pack(anchor='w', pady=(9, 0))

    def pick(self):
        filename = filedialog.askopenfilename(
            parent=self.root, title='Selecciona un archivo local',
            filetypes=[('Captura CSMP o Excel', '*.zip *.xlsx *.xls *.xlsm'),
                       ('Todos los archivos', '*.*')],
        )
        if filename:
            source = Path(filename)
            if source.suffix.lower() not in ('.zip', '.xlsx', '.xls', '.xlsm'):
                self.status.set('Formato no admitido. Selecciona Excel o ZIP de captura.')
                return
            self.source = source
            # No revelar el nombre ni la ruta potencialmente identificables.
            self.chosen.set('Archivo seleccionado para proceso local.')
            self.status.set('Selecciona un modo y pulsa Anonimizar.')

    def start(self):
        if self.source is None:
            self.status.set('Debes seleccionar un archivo.')
            return
        mode, source = self.mode.get(), self.source
        self.go.state(['disabled'])
        self.select.state(['disabled'])
        self.status.set('Procesando localmente; ningún dato se envía fuera del equipo.')
        def work():
            try:
                target = destination(source)
                if source.suffix.lower() == '.zip':
                    anonymize_capture(source, target, mode=mode)
                else:
                    anonymize(source, target, text_mode=mode)
            except (ValueError, OSError, RuntimeError, KeyError, TypeError):
                # Los mensajes internos pueden incluir nombres/rutas; no mostrarlos ni registrarlos.
                self.events.put(('error', None))
            except Exception:
                self.events.put(('error', None))
            else:
                self.events.put(('ok', target.name))
        threading.Thread(target=work, daemon=True).start()
        self.root.after(100, self.poll)

    def poll(self):
        try:
            event, value = self.events.get_nowait()
        except queue.Empty:
            self.root.after(100, self.poll)
            return
        self.go.state(['!disabled'])
        self.select.state(['!disabled'])
        if event == 'ok':
            self.status.set('Copia creada en la misma carpeta: ' + value +
                            '. Revisión humana obligatoria antes de compartir.')
        else:
            self.status.set('No se creó una copia válida. Comprueba que el Excel/ZIP corresponda al '
                            'formato admitido, que el ZIP no contenga originales o binarios y que '
                            'el destino pueda escribirse. El original sigue intacto.')


def main():
    root = tk.Tk()
    App(root)
    root.mainloop()


if __name__ == '__main__':
    main()
