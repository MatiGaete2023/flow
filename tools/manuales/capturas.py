"""Capturas de las ventanas reales con datos ficticios para los manuales (Linux + Xvfb + ImageMagick).

    xvfb-run -a -s "-screen 0 1400x1000x24" python tools/manuales/capturas.py <producto> <salida>

producto: asistente | descargador | meta4 | anonimizador. Escribe <salida>/raw/*.png y <salida>/cajas.json
(recuadros numerados que anota tools/manuales/generar.py). Ningún dato es real.
"""

import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class Camara:
    def __init__(self, root, salida, ancho, alto, antes=None):
        self.root, self.salida, self.ancho, self.alto = root, Path(salida), ancho, alto
        self.antes = antes
        (self.salida / 'raw').mkdir(parents=True, exist_ok=True)
        self.cajas = {}
        root.geometry(f'{ancho}x{alto}+0+0')

    def esperar(self, condicion=lambda: True, segundos=30):
        limite = time.time() + segundos
        while time.time() < limite:
            self.root.update()
            if condicion():
                break
            time.sleep(0.03)
        self.root.update()

    def foto(self, nombre, **widgets):
        self.esperar()
        if self.antes:
            self.antes()
            self.esperar()
        time.sleep(0.6)
        self.root.update()
        subprocess.run(
            ['import', '-window', 'root', '-crop', f'{self.ancho}x{self.alto}+0+0', str(self.salida / 'raw' / nombre)],
            check=True,
        )
        self.cajas[nombre] = [
            [w.winfo_rootx(), w.winfo_rooty(), w.winfo_width(), w.winfo_height()] for w in widgets.values()
        ]

    def cerrar(self):
        (self.salida / 'cajas.json').write_text(json.dumps(self.cajas, indent=1), encoding='utf-8')


def asistente(salida):
    sys.path[:0] = [str(ROOT / 'src'), str(ROOT / 'tests')]
    from datetime import date
    from unittest.mock import patch
    import tkinter as tk

    from nurus.capture import write_capture
    from nurus.personal.app import App
    from nurus.personal.config import Configuration
    from test_capture_contract import fixture

    base = Path(tempfile.mkdtemp())
    app = App(Configuration(base / 'config'))

    def limpiar():
        # Aviso de primera vez (contactos de ejemplo) y rutas temporales fuera de la imagen.
        for w in app.winfo_children():
            if isinstance(w, tk.Toplevel):
                w.destroy()
        if app.work:
            app.status.set('Excel generado: Captura_CSMP - revisable.xlsx · Disponible en Correos y Resoluciones.')

    cam = Camara(app, salida, 1280, 800, antes=limpiar)
    cam.foto('01_inicio.png', importar=app.import_button)
    manifest, tablas = fixture()
    paquete = write_capture(base / 'Captura_CSMP.zip', manifest, tablas)
    app._run = lambda label, action, done=None: done(action()) if done else action()
    with patch.object(app, '_ask_cut_date', return_value=date(2026, 10, 8)):
        app._import_capture(paquete)
    cam.esperar()
    primera = app.work.rows[0].id
    app.records.selection_set(primera)
    app.records.focus(primera)
    app._detail()
    cam.foto('02_trabajo.png', tabla=app.records, siguiente=app.next_step.action_button, excel=app.work_primary_button)
    app.tabs.select(app.pages['Correos'])
    cam.foto(
        '03_correos.png',
        preparar=app.mail_action_buttons['preparar_todos'],
        guardar=app.mail_action_buttons['guardar_todos'],
    )
    app.tabs.select(app.pages['Resoluciones'])
    cam.foto('04_resoluciones.png', preparar=app.prepare_word_button, generar=app.generate_word_button)
    app.tabs.select(app.pages['Resultados'])
    cam.foto('05_resultados.png', descargar=app.results_download_button)
    app.tabs.select(app.pages['Configuración'])
    cam.foto('06_configuracion.png')
    cam.cerrar()
    app.destroy()


def descargador(salida):
    sys.path[:0] = [str(ROOT / 'descargador')]
    import tkinter as tk

    import preferencias

    base = Path(tempfile.mkdtemp())
    preferencias.data_directory = lambda: base
    import descargador as d
    from test_flujos import catalog

    class Puente:
        connection_code = '51234-0123456789abcdef0123456789abcdef'
        connected = False

        def __init__(self, emit):
            pass

        def close(self):
            pass

    root = tk.Tk()
    app = d.Application(root, start_worker=False, bridge_factory=Puente)
    app.dest.set(r'C:\Users\usuario\Documents\Descargas SITFA')
    cam = Camara(root, salida, 1100, 860)
    cam.foto('01_sin_conexion.png', abrir=app.open_button, codigo=app.copy_button, mas=app.topbar.menu_button)
    app.process_event('connected', 'Chrome conectado.')
    app.process_event('catalog', catalog('seguimiento'))
    app.refresh()
    cam.foto(
        '02_seguimiento.png',
        consultas=app.side,
        estado=app.topbar.pill,
        tribunales=app.trib_button,
        paquete=app.pipeline_button,
    )
    cam.foto(
        '03_paquete.png', paquete=app.pipeline_button, incluir=app.calendar_choice.master, avanzado=app.advanced_toggle
    )
    app.toggle_advanced()
    cam.foto('04_avanzado.png', acciones=app.advanced_actions)
    app.toggle_advanced()
    app.process_event('catalog', catalog('litigantes'))
    app.refresh()
    cam.foto('05_litigantes.png', consultas=app.side, estado=app.state_combo)
    cam.cerrar()
    root.destroy()


def meta4(salida):
    carpeta = ROOT / 'META4_Bitacoras'
    sys.path[:0] = [str(carpeta), str(carpeta / 'tests')]
    import tkinter as tk

    import Meta4
    from test_meta4_bitacoras import cuatro_lotes

    base = Path(tempfile.mkdtemp())
    lotes, bitacoras = cuatro_lotes(base / 'Descargas SITFA')
    root = tk.Tk()
    app = Meta4.App(root)
    app.folder.set(r'C:\Users\usuario\Documents\Meta4')
    cam = Camara(root, salida, 820, 600)
    cam.foto('01_inicio.png', agregar=app.add_button, mas=app.window.top.menu_button)
    app.diaries.set(str(bitacoras))
    app.set_sources(lotes)
    app.cutoff.set('2026-10-16')
    cam.foto('02_lotes.png', lotes=app.lots, tribunales=app.courts, corte=app.cutoff_entry, generar=app.generate)
    app.folder.set(str(base / 'salidas'))
    app.start()
    cam.esperar(lambda: (app.poll() or True) and not app.busy and app.output, 90)
    cam.foto('03_generado.png', avisos=app.notices, resultado=app.window.banner)
    cam.cerrar()
    root.destroy()


def anonimizador(salida):
    sys.path[:0] = [str(ROOT / 'anonimizador'), str(ROOT / 'anonimizador' / 'tests')]
    import tkinter as tk

    from anonimizador_csmp import gui
    from conftest import make_xlsx

    base = Path(tempfile.mkdtemp())
    root = tk.Tk()
    app = gui.App(root)
    cam = Camara(root, salida, 760, 520)
    cam.foto('01_inicio.png', modo=app.modes, agregar=app.add_files, carpeta=app.add_folder)
    app.inputs = [
        make_xlsx(base / 'a.xlsx', [['RIT', 'NOMBRE'], ['P-1-2099', 'Persona ficticia uno']]),
        make_xlsx(base / 'b.xlsx', [['RIT', 'NOMBRE'], ['P-2-2099', 'Persona ficticia dos']]),
    ]
    app.refresh()
    cam.foto('02_listo.png', entradas=app.list, anonimizar=app.go, mas=app.window.top.menu_button)
    app.out_dir.set(str(base / 'salida'))
    app.start()
    cam.esperar(lambda: not app.busy, 60)
    cam.foto('03_hecho.png', resultado=app.list, aviso=app.window.banner)
    cam.cerrar()
    root.destroy()


if __name__ == '__main__':
    {'asistente': asistente, 'descargador': descargador, 'meta4': meta4, 'anonimizador': anonimizador}[sys.argv[1]](
        sys.argv[2]
    )
