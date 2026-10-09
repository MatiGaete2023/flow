"""Demostración reproducible 2.7.5: 20 referencias ausentes, 64 copias ficticias.

DISPLAY=:92 python docs/evidencia_referencias_20261009/generar_captura.py
Requiere el entorno de desarrollo con Tk/Pillow; jamás abre datos institucionales.
"""
from pathlib import Path
import json
import subprocess
import sys
import tempfile
import time
import tkinter as tk
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
sys.path[:0]=[str(ROOT/'src'),str(ROOT/'descargador')]
from PIL import ImageGrab
from descargador import Application
from test_lotes import dispose_interface
from test_reutilizacion_local import diaries,bound_lot
from preferencias import Preferences
from archivo_local import remember
from ventana_resultados import ResultsWindow
from nurus.capture import read_capture


def main():
    output=Path(__file__).resolve().parent
    with tempfile.TemporaryDirectory(prefix='Demo_ficticia_paquete_') as temp:
        base=Path(temp);control=diaries(base/'Bitacoras_ficticias_disponibles',64)
        sheets=bound_lot(base/'Hojas_ficticias_Cumplimiento')
        root=tk.Tk();app=Application(root,start_worker=False)
        app.code.set('CÓDIGO FICTICIO · NO VÁLIDO')
        def settle(window):
            deadline=time.monotonic()+20
            while window.busy and time.monotonic()<deadline:root.update();time.sleep(.02)
            assert not window.busy,window.status.get()
            root.update()
        try:
            app.prefs=Preferences(base/'preferencias_ficticias.json');remember(app.prefs,control.parent)
            app.prefs.data['historial_bitacoras'].extend(str(base/f'Lote_anterior_ficticio_{i:02d}'/'bitacoras.json') for i in range(20))
            app.prefs.save();before=app.prefs.path.read_bytes()
            app.dest.set(str(base/'Otra_carpeta'));app.last_folder=sheets.folder
            window=ResultsWindow(app);window.geometry('1450x950+0+0');settle(window)
            window.capture_package();popup=window.missing_window
            popup.geometry('1050x520+40+40');popup.lift();root.update()
            x,y=popup.winfo_rootx(),popup.winfo_rooty()
            ImageGrab.grab(bbox=(x,y,x+popup.winfo_width(),y+popup.winfo_height())).save(output/'revision_referencias_ficticias.png')
            popup.exclude_all()
            target=base/'Paquete_ficticio.zip'
            with patch('ventana_resultados.filedialog.asksaveasfilename',return_value=str(target)):popup.finish()
            settle(window);capture=read_capture(target)
            assert len(capture.rows('registros'))==2 and len(capture.rows('bitacoras'))==2
            assert app.prefs.path.read_bytes()==before
            report={'version':'2.7.5','source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                    'datos':'FICTICIOS','referencias_ausentes':20,'lecturas_disponibles':64,
                    'bitacoras_correspondientes_exportadas':2,'peticiones_institucionales':0,
                    'preferencias_conservadas':True,'resultado':'OK'}
            (output/'demostracion.json').write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
            window.destroy()
        finally:dispose_interface(app,root)


if __name__=='__main__':main()
