import json
import subprocess
import sys
import time
from pathlib import Path

R = Path('/home/user/flow/descargador_lite')
sys.path[:0] = [str(R / 'app'), str(R / 'tests')]
import tkinter as tk  # noqa: E402

import sitfa_lite as s  # noqa: E402
from sitfa_ficticio import FakeBridge  # noqa: E402

OUT = Path(sys.argv[1])
BOX = {}


class FB(FakeBridge):
    connection_code = '51234-0123456789abcdef0123456789abcdef'

    def __init__(self, emit):
        super().__init__('seguimiento', pages={('901', 'Espera', '1', None): 0, ('901', 'Cumplimiento', '2', '0'): 2})
        self.connected = False

    def close(self):
        pass


root = tk.Tk()
app = s.Application(root, bridge_factory=FB, preferences_path=OUT / 'prefs.json')
root.geometry('1000x1060+0+0')
app.dest.set(r'C:\Users\usuario\Documents\SITFA Lite')
app.connected = False
canvas = next(w for w in root.winfo_children()[1].winfo_children() if isinstance(w, tk.Canvas))


def shot(name, **widgets):
    root.update(); time.sleep(0.4); root.update()
    subprocess.run(['import', '-window', 'root', '-crop', '1000x1060+0+0', str(OUT / 'raw' / name)])
    BOX[name] = {
        k: [w.winfo_rootx(), w.winfo_rooty(), w.winfo_width(), w.winfo_height()] for k, w in widgets.items()
    }
    BOX[name]['_canvas'] = [canvas.winfo_rootx(), canvas.winfo_rooty(), canvas.winfo_width(), canvas.winfo_height()]


def wait(cond, then):
    def check():
        if cond():
            then()
        else:
            root.after(200, check)
    root.after(300, check)


def s0():
    app.status.set('1) Abre SITFA en Chrome  2) Pulsa el icono de la extensión  3) Pega el código.')
    shot('a0_inicio.png', copiar=app.copy_button, abrir=app.open_button, instalar=app.setup_button)
    app.bridge.connected = True
    app.process_event('connected', 'Chrome conectado. Elige la consulta y pulsa Cargar opciones.')
    root.after(600, s1)


def s1():
    shot('a1_conectado.png', estado=app.link_label, cargar=app.load_button, radios=app.screen_radios[0].master)
    app.enqueue('catalog', app.screen_code)
    wait(lambda: app.catalog and not app.busy, s2)


def s2():
    app.modalities.set_all(False)
    app.modalities.vars['2'].set(True)
    app.tabs.vars['Informes'].set(True)
    app.medidas.vars['5'].set(True)
    app.informes.vars['2'].set(True)
    app.refresh()
    shot('a2_opciones_seguimiento.png', tribunales=app.tribunals, pestanas=app.tabs, modalidades=app.modalities)
    canvas.yview_moveto(1.0)
    app.quick_dates('Mes anterior')
    root.after(400, s3)


def s3():
    shot('a3_informes_medidas_fechas.png', informes=app.informes, medidas=app.medidas, fechas=app.date_box)
    canvas.yview_moveto(0)
    app.start()
    wait(lambda: not app.busy and app.phase.get().startswith(('Listo', 'Detenido')), s4)


def s4():
    shot('a4_descarga_terminada.png', descargar=app.download_button, lista=app.tree, fase=app.progress)
    app.screen.set('Seguimiento · Búsqueda litigantes'); app.bridge.screen = 'litigantes'; app.change_screen()
    app.enqueue('catalog', app.screen_code)
    wait(lambda: app.catalog and not app.busy, s5)


def s5():
    app.tribunals.set_visible(True)
    app.states.set_all(True)
    shot('a5_busqueda_lit.png', estados=app.states, tribunales=app.tribunals)
    app.screen.set('Informes · Medidas por vencer'); app.bridge.screen = 'calendario_medidas'; app.change_screen()
    app.enqueue('catalog', app.screen_code)
    wait(lambda: app.catalog and not app.busy, s6)


def s6():
    app.tribunals.set_visible(True)
    app.months.vars['11'].set(True); app.refresh()
    shot('a6_medidas_por_vencer.png', meses=app.months, anio=app.year_box)
    (OUT / 'app_boxes.json').write_text(json.dumps(BOX, indent=1))
    root.destroy()


root.after(900, s0)
root.mainloop()
