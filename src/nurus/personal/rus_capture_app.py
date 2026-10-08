"""Herramienta separada para inspeccionar la captura del registro actual en Windows."""
import json
from pathlib import Path
import tkinter as tk
from tkinter import filedialog, messagebox, ttk

from .rus_capture import inspect_capture


def main():
    root = tk.Tk()
    root.title('RUS · Inspector del registro actual')
    root.geometry('760x510')
    root.minsize(640, 420)
    frame = ttk.Frame(root, padding=18)
    frame.pack(fill='both', expand=True)
    ttk.Label(frame, text='Preparar la conexión real con RUS', font=('Segoe UI', 16, 'bold')).pack(anchor='w')
    ttk.Label(frame, text='Abre la captura JSON descargada por la extensión. Esta herramienta inspecciona su estructura.',
              wraplength=700).pack(anchor='w', pady=10)
    text = tk.Text(frame, wrap='word', font=('Segoe UI', 10))
    text.pack(fill='both', expand=True, pady=10)
    text.insert('1.0', 'No hay captura abierta. El registro automático aún requiere el contrato real y su validación en RUS.')
    text.configure(state='disabled')

    def inspect():
        path = filedialog.askopenfilename(parent=root, title='Captura del formulario RUS',
                                         filetypes=[('Captura JSON', '*.json')])
        if not path:
            return
        try:
            report = inspect_capture(path)
        except (OSError, ValueError, TypeError, KeyError):
            messagebox.showerror('Captura no válida', 'No se pudo comprobar esta captura. Usa el archivo original de la extensión.', parent=root)
            return
        lines = [f"Pantalla: {report['stage']}", f"Fecha de captura: {report['captured_at']}",
                 f"Documentos: {report['documents']} · Formularios: {len(report['forms'])} · Tablas: {report['tables']}", '']
        for form in report['forms']:
            lines.append(f"Formulario: {form['name'] or form['id'] or form['index']} ({form['method']})")
            for field in form['text_fields']:
                limit = field['max_length'] if field['max_length'] is not None else 'sin maxlength declarado'
                lines.append(f"  Texto: {field['name'] or field['id']} · Límite: {limit}")
            lines.append(f"  Selectores: {len(form['selectors'])} · Campos ocultos de identidad: {len(form['hidden_identity_fields'])}")
        lines.extend(['', 'Para completar la conexión:', *report['next_checks'], '', report['limitations']])
        text.configure(state='normal')
        text.delete('1.0', 'end')
        text.insert('1.0', '\n'.join(lines))
        text.configure(state='disabled')
        destination = filedialog.asksaveasfilename(parent=root, title='Guardar informe de estructura',
            initialfile=Path(path).stem + '_inspeccion.json', defaultextension='.json', filetypes=[('Informe JSON', '*.json')])
        if destination:
            try:
                with Path(destination).open('x', encoding='utf-8') as handle:
                    json.dump(report, handle, ensure_ascii=False, indent=2)
            except FileExistsError:
                messagebox.showerror('El archivo existe', 'Selecciona un nombre nuevo para el informe.', parent=root)
            except OSError:
                messagebox.showerror('No se pudo guardar', 'Revisa la carpeta y vuelve a abrir la captura.', parent=root)

    ttk.Button(frame, text='Abrir captura e inspeccionar', command=inspect).pack(anchor='w')
    root.mainloop()


if __name__ == '__main__':
    main()
