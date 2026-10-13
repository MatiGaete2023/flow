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
            assert 'hasta 600' in app.status.get()
            assert app.prefs.data['leer_historia'] is True
            app.process_event('history_progress',{'causa':'C-1-2099','estado':'LEIDA'})
            app.process_event('history_progress',{'causa':'C-2-2099','estado':'FALLIDA'})
            assert 'Historia: 1 leídas · 1 fallidas' in app.status.get()
            app.process_event('history_done',{'estado':'INCOMPLETA','leidas':1,'fallidas':1,'tramites_en_ventana':3})
            assert 'INCOMPLETA' in app.status.get() and '3 trámites' in app.status.get()
            app.history_choice.invoke()
            assert not app.read_history.get() and not app.history_package_choice.winfo_manager()
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
