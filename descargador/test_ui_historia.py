"""Casillas de historia: apagadas por defecto, se recuerdan, y el progreso se informa. Datos ficticios."""

import os
import subprocess
import sys
import textwrap
from pathlib import Path


def test_casillas_de_historia(tmp_path):
    source = textwrap.dedent("""
        from pathlib import Path
        import sys,tkinter as tk
        import preferencias
        from descargador import Application
        from test_lotes import dispose_interface
        salida=Path(sys.argv[1]);preferencias.data_directory=lambda:salida
        root=tk.Tk();root.withdraw();app=Application(root,start_worker=False)
        try:
            assert app.prefs.data['leer_historia'] is False and app.prefs.data['historia_en_paquete'] is True
            assert not app.read_history.get() and not app.history_package_choice.winfo_manager()
            assert 'prueba' in app.history_choice.cget('text')
            app.history_choice.invoke()
            assert app.read_history.get() and app.history_package_choice.winfo_manager()
            # Revisión Codex en el PR #50: todas las opciones caben a 1000 px (antes «Avanzado» quedaba fuera).
            filas = app.pipeline_button.master.master.pack_slaves()
            root.update_idletasks()
            assert len(filas) == 3 and max(fila.winfo_reqwidth() for fila in filas) <= 1000 - 24
            assert 'hasta 600' in app.status.get()
            assert app.prefs.data['leer_historia'] is True
            app.process_event('history_progress',{'causa':'C-1-2099','estado':'LEIDA'})
            app.process_event('history_progress',{'causa':'C-2-2099','estado':'FALLIDA'})
            assert 'Historia: 1 leídas · 1 fallidas' in app.status.get()
            app.process_event('history_done',{'estado':'INCOMPLETA','leidas':1,'fallidas':1,'tramites_en_ventana':3})
            assert 'INCOMPLETA' in app.status.get() and '3 trámites' in app.status.get()
            app.history_choice.invoke()
            assert not app.read_history.get() and not app.history_package_choice.winfo_manager()
            app.process_event('extension_caps',{'protocolos':{'bitacoras_lectura':1}})
            assert str(app.history_choice.cget('state'))=='disabled' and '2.8.0' in app.status.get()
            app.process_event('extension_caps',{'protocolos':{'historia_lectura':1}})
            assert str(app.history_choice.cget('state'))=='normal'
        finally:dispose_interface(app,root)
    """)
    result = subprocess.run(
        [sys.executable, '-c', source, str(tmp_path)],
        cwd=Path(__file__).parent,
        env={**os.environ, 'PYTHONIOENCODING': 'utf-8'},
        capture_output=True,
        text=True,
        encoding='utf-8',
        timeout=60,
    )
    assert result.returncode == 0, result.stderr


def test_e3_worker_solo_reintenta_historia_del_lote_elegido(tmp_path, monkeypatch):
    from queue import Queue
    from threading import Event
    from types import SimpleNamespace
    import lectura_bitacoras
    from descargador import Application

    control = tmp_path / 'historia' / 'historias.json'
    control.parent.mkdir()
    control.write_text('{}')
    recibidas = []

    class Runner:
        def __init__(self, *args, **kwargs):
            recibidas.append(kwargs)

        def run(self, *args):
            pass

    monkeypatch.setattr(lectura_bitacoras, 'DiaryRunner', Runner)
    for resume, leer in ((None, True), (tmp_path, True), (tmp_path, False)):
        commands = Queue()
        commands.put(('diary', ('lote ficticio', tmp_path, resume)))
        commands.put(('exit', None))
        eventos = []
        app = SimpleNamespace(
            commands=commands,
            bridge=SimpleNamespace(connected=False, close=lambda: None),
            catalog={},
            emit=lambda *e: eventos.append(e),
            cancel=Event(),
            prefs=SimpleNamespace(data={'pausa_consultas': 0, 'reutilizar_bitacoras': True, 'leer_historia': leer}),
        )
        Application.worker(app)
        assert not any(e[0] == 'error' for e in eventos)
    assert 'historia' not in recibidas[0]
    assert recibidas[1]['historia'].anterior == control
    assert recibidas[1]['reuse_base'] is None
    # §3.11: con «Leer historia» desmarcada el reintento no abre historias aunque exista un control previo.
    assert 'historia' not in recibidas[2]
