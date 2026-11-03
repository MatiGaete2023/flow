"""Capturas de SITFA Lite 1.1 con SITFA ficticio (Linux + Xvfb). Uso: capturas_programa.py <salida>.

Escribe <salida>/raw/a*.png y <salida>/app_boxes.json con los recuadros que anotar.py dibuja.
"""

import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

R = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(R / 'app'), str(R / 'tests')]
import tkinter as tk  # noqa: E402

import sitfa_lite as s  # noqa: E402
from test_ventana import Puente  # noqa: E402

OUT = Path(sys.argv[1])
(OUT / 'raw').mkdir(parents=True, exist_ok=True)
W, H = 1000, 680
BOX = {}
work = Path(tempfile.mkdtemp())

root = tk.Tk()
app = s.Application(root, bridge_factory=Puente, preferences_path=work / 'prefs.json')
app.dest.set(str(work))
root.geometry(f'{W}x{H}+0+0')


def box(widget):
    return [widget.winfo_rootx(), widget.winfo_rooty(), widget.winfo_width(), widget.winfo_height()]


def shot(name, **widgets):
    root.update()
    time.sleep(0.5)
    root.update()
    subprocess.run(['import', '-window', 'root', '-crop', f'{W}x{H}+0+0', str(OUT / 'raw' / name)], check=True)
    BOX[name] = {k: box(w) for k, w in widgets.items()}


def wait(condition):
    deadline = time.time() + 30
    while not condition() and time.time() < deadline:
        root.update()
        time.sleep(0.02)


def row(name):
    return app.form.rows[name][1]


shot('a0_inicio.png', abrir=app.open_button, copiar=app.copy_button, mas=app.window.top.menu_button)
app.bridge.connected = True
wait(lambda: app.catalog is not None and not app.busy)
shot('a1_conectado.png', estado=app.window.top.pill, consultas=app.side, descargar=app.download_button)
app.tribunals.set(['901'])
app.tabs.set(['Espera', 'Cumplimiento', 'Informes'])
app.modalities.set(['2'])
app.refresh()
shot('a2_opciones_seguimiento.png', tribunales=row('Tribunales'), pestanas=row('Pestañas'), modalidades=row('Modalidad'))
app.informes.set(['1', '2'])
app.medidas.set(['0', '5'])
app.quick_dates('Mes anterior')
shot('a3_informes_medidas_fechas.png', informes=row('informes'), medidas=row('medidas'), periodo=row('Período'),
     incluir=row('Incluir'))
app.primary()
wait(lambda: app.showing_results and not app.busy)
shot('a4_descarga_terminada.png', lista=app.results, resumen=app.window.footer)
app.primary()
app.side.select('litigantes', notify=True)
wait(lambda: app.catalog is not None and app.catalog['pantalla'] == 'litigantes' and not app.busy)
app.tribunals.set(['901', '902'])
app.states.set(list(app.states.buttons))
app.refresh()
shot('a5_busqueda_lit.png', tribunales=row('Tribunales'), estados=row('Estado'))
app.side.select('calendario_medidas', notify=True)
wait(lambda: app.catalog is not None and app.catalog['pantalla'] == 'calendario_medidas' and not app.busy)
app.tribunals.set(['901'])
app.months.set(['11'])
app.refresh()
shot('a6_medidas_por_vencer.png', meses=row('Mes'), anio=row('Año'))
(OUT / 'app_boxes.json').write_text(json.dumps(BOX, indent=1))
root.destroy()
