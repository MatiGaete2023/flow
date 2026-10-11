"""Herramientas del trabajo actual: revisión, diagnóstico y entrega."""

from pathlib import Path
import json, os, sys
from tkinter import messagebox, filedialog
from .outputs import emails, CC
from .product_checks import check_products
from nurus.services.file_output import write_new_file


def summary(drafts):
    lines = []
    for draft in drafts:
        to = emails(draft.to)
        cc = emails(CC + ';' + draft.cc)
        if not to:
            raise ValueError('Completa el destinatario: ' + draft.subject)
        if not draft.roster_reviewed:
            raise ValueError('Revisa primero la nómina de: ' + draft.subject)
        if draft.required and not draft.attachments:
            raise ValueError('Falta adjunto requerido: ' + draft.subject)
        if not draft.subject.strip() or not draft.body.strip():
            raise ValueError('Completa asunto y cuerpo del correo.')
        lines += [
            'ASUNTO: ' + draft.subject,
            'PARA: ' + '; '.join(to),
            'CC: ' + '; '.join(cc),
            'Registros de origen: ' + str(len(draft.record_ids)),
        ]
        for group in draft.roster:
            lines.append('NÓMINA · ' + group['program'])
            lines.extend(' | '.join(row['cells']) for row in group['rows'] if row['include'])
        for attachment in draft.attachments:
            path = Path(attachment)
            if not path.is_file():
                raise ValueError('Adjunto no disponible: ' + str(path))
            lines.append('ADJUNTO: ' + str(path) + ' · ' + str(path.stat().st_size) + ' bytes')
        lines += ['CUERPO:\n' + draft.body, '']
    return '\n'.join(lines)


def confirm_drafts(app, drafts, excluded=()):
    import tkinter as tk
    from tkinter import ttk

    content = summary(drafts)
    if excluded:
        content += '\n\nPENDIENTES QUE NO SE GUARDARÁN EN ESTE LOTE:\n' + '\n'.join(excluded)
    accepted = []
    win = tk.Toplevel(app)
    win.title('Resumen antes de guardar en Outlook')
    win.geometry('850x650')
    win.transient(app)
    tabs = ttk.Notebook(win)
    tabs.pack(fill='both', expand=True, padx=12, pady=12)
    overview = ttk.Frame(tabs)
    tabs.add(overview, text='Destinatarios y adjuntos')
    table = ttk.Treeview(overview, columns=('asunto', 'para', 'adjuntos'), show='headings')
    for key, label in [('asunto', 'Asunto'), ('para', 'Para'), ('adjuntos', 'Adjuntos revisados')]:
        table.heading(key, text=label)
    table.pack(fill='both', expand=True)
    for i, draft in enumerate(drafts):
        table.insert('', 'end', iid=str(i), values=(draft.subject, draft.to, len(draft.attachments)))
    detail = ttk.Frame(tabs)
    tabs.add(detail, text='Contenido completo')
    text = tk.Text(detail, wrap='word', font=('Segoe UI', 11))
    text.pack(fill='both', expand=True)
    text.insert('1.0', content)
    text.configure(state='disabled')
    ttk.Label(
        overview,
        text='Los destinatarios, CC, cuerpo y nombres de adjuntos completos están en Contenido completo.',
        wraplength=780,
    ).pack(pady=10)

    ttk.Label(win, text='Se crearán borradores en Outlook. Revisa destinatarios, nómina, cuerpo y archivos.').pack()
    buttons = ttk.Frame(win)
    buttons.pack(pady=12)
    ttk.Button(
        buttons, text='Confirmar y guardar borradores', command=lambda: (accepted.append(True), win.destroy())
    ).pack(side='left', padx=4)
    ttk.Button(buttons, text='Cancelar', command=win.destroy).pack(side='left', padx=4)
    win.grab_set()
    app.wait_window(win)
    return bool(accepted)


def diagnosis(app):
    from nurus import __version__
    from importlib.metadata import version, PackageNotFoundError

    result = {
        'producto': 'CSMP Windows',
        'version': __version__,
        'python': sys.version.split()[0],
        'rutas': {
            'ejecutable': sys.executable,
            'configuracion': str(app.cfg.directory),
            'matrices': str(app.template_dir),
        },
        'mediciones_operacion': list(getattr(app, 'operation_timings', [])),
        'plantillas': len(list(app.template_dir.rglob('*.docx'))),
        'dependencias': {},
        'office': {},
    }
    for name in ('pandas', 'openpyxl', 'customtkinter', 'python-docx', 'pywin32'):
        try:
            result['dependencias'][name] = version(name)
        except PackageNotFoundError:
            result['dependencias'][name] = 'No instalado'
    if os.name == 'nt':
        import winreg

        for name in ('Excel.Application', 'Word.Application', 'Outlook.Application'):
            try:
                with winreg.OpenKey(winreg.HKEY_CLASSES_ROOT, name):
                    result['office'][name] = 'Registrado'
            except OSError:
                result['office'][name] = 'No registrado'
    else:
        result['office']['estado'] = 'Comprobar en el equipo Windows de destino'
    return result


def export_diagnosis(app):
    path = filedialog.asksaveasfilename(defaultextension='.json', initialfile='Diagnostico_CSMP.json', parent=app)
    if path:
        write_new_file(
            Path(path),
            lambda p: p.write_text(json.dumps(diagnosis(app), ensure_ascii=False, indent=2), encoding='utf-8'),
        )
        app.status.set('Diagnóstico creado: ' + path)


from .delivery import selected_zip


def deliver(app):
    from .product_checks import current_drafts

    checks = check_products(app.work, current_drafts(app), app.projects)
    files = {c.path: c for c in checks if c.path and not c.issues}
    paths = filedialog.askopenfilenames(
        title='Selecciona los archivos de productos para entregar',
        parent=app,
        initialdir=Path(app.work.output).parent if app.work and app.work.output else app.cfg.directory,
        filetypes=[('Productos', '*.xlsx *.docx *.pdf *.csv *.ics *.zip')],
    )
    if not paths:
        return
    blocked = {str(Path(c.path).resolve()): c for c in checks if c.path and c.issues}
    for path in paths:
        if str(Path(path).resolve()) in blocked:
            raise ValueError('Revisa el producto antes de entregarlo: ' + Path(path).name)
    from .delivery import validate_product

    for path in paths:
        validate_product(path)
    preview = 'Archivos seleccionados: ' + str(len(paths)) + '\n\n' + '\n'.join(Path(p).name for p in paths)
    preview += '\n\nSe incluirán un índice y un manifiesto de comprobación. Los borradores permanecen en Outlook.'
    if not messagebox.askokcancel('Revisar entrega actual', preview, parent=app):
        return
    destination = filedialog.asksaveasfilename(defaultextension='.zip', initialfile='Entrega_CSMP.zip', parent=app)
    if destination:
        metadata = {}
        for p, c in files.items():
            info = {'producto': c.label}
            if c.record_count is not None:
                info['registros'] = c.record_count
            elif c.target is not None:
                ids = {
                    rid
                    for item in checks
                    if item.path == p and not item.issues
                    for rid in getattr(item.target, 'record_ids', [])
                }
                info['registros'] = len(ids)
            metadata[str(Path(p).resolve())] = info
        app._run(
            'Preparando entrega…',
            lambda: selected_zip(paths, destination, metadata),
            lambda p: app.status.set('Entrega verificada: ' + p),
        )
