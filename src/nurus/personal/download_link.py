"""Devolución recuperable de un flujo completo; el Asistente no inicia el Descargador (WP-23)."""

from pathlib import Path
import hashlib
import json

from .download_transfer import transfers, recovery_work


def verify_reply(path, *, expected_mode=None):
    path = Path(path)
    if path.stat().st_size > 64_000:
        raise ValueError('La respuesta del descargador excede el tamaño permitido.')
    data = json.loads(path.read_text(encoding='utf-8'))
    if not isinstance(data, dict) or not isinstance(data.get('archivo'), str):
        raise ValueError('La respuesta no identifica un libro descargado.')
    source = Path(data['archivo']).resolve()
    if source.suffix.lower() != '.xlsx' or not source.is_file() or source.stat().st_size > 80 * 1024 * 1024:
        raise ValueError('El resultado no es un libro disponible.')
    if hashlib.sha256(source.read_bytes()).hexdigest() != data.get('sha256'):
        raise ValueError('El libro cambió después de la descarga.')
    if data.get('modo') not in ('ESPERA', 'CUMPLIMIENTO'):
        raise ValueError('Modo de retorno no reconocido.')
    if expected_mode is not None and data['modo'] != expected_mode:
        raise ValueError('La devolución no corresponde al modo de esta solicitud.')
    return source, data['modo']


def incorporate(app, transfer):
    if app.busy:
        raise ValueError('Espera a que termine la operación actual antes de recuperar la descarga.')
    recovered = recovery_work(transfer, app.work, app.cfg.data)
    if recovered is not None:
        if recovered is not app.work:
            app._save_session()
            app._resume_work = recovered
            app._continue_resume()
        app.status.set('Descarga recuperada con sus ediciones. Exportar copia actual permite continuar.')
        return
    source, mode = verify_reply(transfer.reply, expected_mode=transfer.data['mode'])
    expected = hashlib.sha256(source.read_bytes()).hexdigest()
    transfer.update(transfer.data['state'], source_hash=expected, source=str(source))
    if app.work:
        app._save_session()
        # La sesión única puede cambiar después: conservar también la anterior.
        from uuid import uuid4

        app.work.save(transfer.recovery_folder / 'anteriores' / uuid4().hex, snapshot=True)

    def analyze():
        from .sitfa import inspect_input
        from .work import Work

        checked, _ = verify_reply(transfer.reply, expected_mode=mode)
        if checked != source or hashlib.sha256(checked.read_bytes()).hexdigest() != expected:
            raise ValueError('El resultado cambió mientras se preparaba su incorporación.')
        report = inspect_input(source, mode, mode)
        if report['faltan']:
            raise ValueError('Faltan columnas para analizar: ' + ', '.join(report['faltan']))
        work = Work(app.cfg.data).analyze(source, mode, sheet=mode)
        if work.source_hash != expected:
            raise ValueError('El libro cambió durante la lectura; la solicitud sigue pendiente.')
        return work

    def done(work):
        work.download_transfer_id = transfer.identifier
        app.work = work
        app._clear_drafts()
        app.observation_id = None
        app.folder.set(transfer.data['destination'])
        app.sheet.set(mode)
        app._show_work()
        app._save_session()
        work.save(transfer.recovery_folder / 'incorporada', snapshot=True)
        # Un fallo posterior de Excel no provoca otro análisis ni otra descarga.
        transfer.update('INCORPORADA', source_hash=expected, source=str(source))
        app._export_current()

    app._run('Incorporando descarga comprobada…', analyze, done)


def resume(app):
    """Elegir una solicitud conservada; no abre el navegador ni repite consultas."""
    items = transfers(app.cfg.directory / 'intercambio')
    if not items:
        raise ValueError('No hay solicitudes de descarga conservadas.')
    if len(items) == 1:
        incorporate(app, items[0])
        return
    import tkinter as tk
    import customtkinter as ctk
    from . import ui

    window = ctk.CTkToplevel(app)
    window.title('Recuperar descarga')
    window.geometry('780x240')
    window.transient(app)
    labels = {
        f"{item.data['created_at']} · {item.data['mode']} · {item.data['state']} · {item.identifier[:8]}": item
        for item in items
    }
    ui.Label(window, text='Selecciona la solicitud que quieres recuperar.', wraplength=730).pack(padx=18, pady=18)
    selected = tk.StringVar(value='')
    ui.Combobox(window, textvariable=selected, values=list(labels), state='readonly', width=95).pack(fill='x', padx=18)

    def apply():
        if selected.get() not in labels:
            raise ValueError('Selecciona una solicitud.')
        incorporate(app, labels[selected.get()])
        window.destroy()

    ui.Button(window, text='Recuperar resultado', command=lambda: app._guard(apply)).pack(pady=20)
