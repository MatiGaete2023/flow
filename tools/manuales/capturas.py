"""Capturas de las ventanas reales con datos ficticios para los manuales.

    python tools/manuales/capturas.py <producto> <salida>            # Windows (como las ve el usuario)
    xvfb-run -a python tools/manuales/capturas.py <producto> <salida>  # Linux, solo para revisar el flujo

producto: asistente | descargador | meta4 | anonimizador | lite. Escribe <salida>/raw/*.png y <salida>/cajas.json
(recuadros relativos a la ventana, que anota tools/manuales/generar.py); en «lite» también app_boxes.json para
descargador_lite/build/manual_visual. Las capturas para publicar se toman en Windows (workflow manuales-windows.yml):
en Linux, Tk dibuja con otra tipografía y otros controles. Ningún dato es real.
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
        root.geometry(f'{ancho}x{alto}+40+40')
        try:
            root.attributes('-topmost', True)  # nada tapa la ventana durante la captura
        except Exception:
            pass

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
        x, y = self.root.winfo_rootx(), self.root.winfo_rooty()
        w, h = self.root.winfo_width(), self.root.winfo_height()
        destino = self.salida / 'raw' / nombre
        if sys.platform == 'win32':
            from PIL import ImageGrab

            ImageGrab.grab(bbox=(x, y, x + w, y + h), all_screens=True).save(destino)
        else:
            subprocess.run(['import', '-window', 'root', '-crop', f'{w}x{h}+{x}+{y}', str(destino)], check=True)
        # Recuadros relativos al área de la ventana capturada.
        self.cajas[nombre] = {
            clave: [v.winfo_rootx() - x, v.winfo_rooty() - y, v.winfo_width(), v.winfo_height()]
            for clave, v in widgets.items()
        }

    def cerrar(self):
        listas = {nombre: list(cajas.values()) for nombre, cajas in self.cajas.items()}
        (self.salida / 'cajas.json').write_text(json.dumps(listas, indent=1), encoding='utf-8')
        (self.salida / 'app_boxes.json').write_text(json.dumps(self.cajas, indent=1), encoding='utf-8')


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
    app.dest.set(str(base / 'Documents' / 'Descargas SITFA'))
    cam = Camara(root, salida, 1100, 860)
    cam.foto('01_sin_conexion.png', abrir=app.open_button, codigo=app.copy_button, instalar=app.setup_button)
    app.process_event('connected', 'Chrome conectado.')
    app.process_event('catalog', catalog('seguimiento'))
    app.refresh()
    cam.foto(
        '02_seguimiento.png',
        consultas=app.side,
        estado=app.topbar.pill,
        tribunales=app.trib_chips,
        paquete=app.pipeline_button,
    )
    cam.foto(
        '03_paquete.png', paquete=app.pipeline_button, incluir=app.form.rows['incluir'][1], carpeta=app.folder_link
    )
    # Paquete cortado: el aviso ofrece retomarlo sin buscar la carpeta.
    cortado = Path(app.dest.get()) / 'Paquete CSMP 2026-11-03 10-15-00-000000' / 'respaldo'
    cortado.mkdir(parents=True, exist_ok=True)
    (cortado / 'proceso.json').write_text('{"estado": "INCOMPLETO", "fase": "bitacoras", "carpetas": {}}')
    app.offer_resume()
    cam.foto('04_retomar.png', aviso=app.banner, mas=app.topbar.menu_button)
    app.banner.hide()
    app.side.select('Seguimiento de litigantes / órdenes')
    app._side_selected('Seguimiento de litigantes / órdenes')
    app.process_event('catalog', catalog('litigantes'))
    app.refresh()
    cam.foto('05_litigantes.png', consultas=app.side, estado=app.state_chips)
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
    app = Meta4.App(root, prefs_path=base / 'prefs.json')
    app.folder.set(r'C:\Users\usuario\Documents\Meta4')
    cam = Camara(root, salida, 820, 600)
    cam.foto('01_inicio.png', agregar=app.empty_add, mas=app.window.top.menu_button)
    app.diaries.set(str(bitacoras))
    app.set_sources(lotes)
    app.cutoff.set('16/10/2026')
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


def lite(salida):
    """Mismas capturas que descargador_lite/build/manual_visual (a0 a a6), con SITFA ficticio."""
    app_dir = ROOT / 'descargador_lite'
    sys.path[:0] = [str(app_dir / 'app'), str(app_dir / 'tests')]
    import tkinter as tk

    import sitfa_lite
    from test_ventana import Puente

    base = Path(tempfile.mkdtemp())
    root = tk.Tk()
    app = sitfa_lite.Application(root, bridge_factory=Puente, preferences_path=base / 'prefs.json')
    app.dest.set(str(base / 'Documents' / 'SITFA Lite'))
    cam = Camara(root, salida, 1000, 680)

    def fila(nombre):
        return app.form.rows[nombre][1]

    cam.foto(
        'a0_inicio.png',
        abrir=app.open_button,
        copiar=app.copy_button,
        mas=app.window.top.menu_button,
        instalar=app.setup_button,
    )
    app.bridge.connected = True
    cam.esperar(lambda: app.catalog is not None and not app.busy)
    cam.foto('a1_conectado.png', estado=app.window.top.pill, consultas=app.side, descargar=app.download_button)
    app.tribunals.set(['901'])
    app.tabs.set(['Espera', 'Cumplimiento', 'Informes'])
    app.modalities.set(['2'])
    app.refresh()
    cam.foto(
        'a2_opciones_seguimiento.png',
        tribunales=fila('Tribunales'),
        pestanas=fila('Pestañas'),
        modalidades=fila('Modalidad'),
    )
    app.informes.set(['1', '2'])
    app.medidas.set(['0', '5'])
    app.from_date.set('01/10/2026')
    app.to_date.set('31/10/2026')
    app.period.set(['Por fecha'])
    app.refresh()
    cam.foto(
        'a3_informes_medidas_fechas.png',
        informes=fila('informes'),
        medidas=fila('medidas'),
        periodo=fila('Período'),
        incluir=fila('Incluir'),
    )
    app.primary()
    cam.esperar(lambda: app.showing_results and not app.busy, 60)
    cam.foto('a4_descarga_terminada.png', lista=app.results, resumen=app.window.footer)
    app.primary()
    app.side.select('litigantes', notify=True)
    cam.esperar(lambda: app.catalog is not None and app.catalog['pantalla'] == 'litigantes' and not app.busy)
    app.tribunals.set(['901', '902'])
    app.states.set(list(app.states.buttons))
    app.refresh()
    cam.foto('a5_busqueda_lit.png', tribunales=fila('Tribunales'), estados=fila('Estado'))
    app.side.select('calendario_medidas', notify=True)
    cam.esperar(lambda: app.catalog is not None and app.catalog['pantalla'] == 'calendario_medidas' and not app.busy)
    app.tribunals.set(['901'])
    app.months.set(['11'])
    app.refresh()
    cam.foto('a6_medidas_por_vencer.png', meses=fila('Mes'), anio=fila('Año'))
    cam.cerrar()
    root.destroy()


if __name__ == '__main__':
    {'asistente': asistente, 'descargador': descargador, 'meta4': meta4, 'anonimizador': anonimizador, 'lite': lite}[
        sys.argv[1]
    ](sys.argv[2])
