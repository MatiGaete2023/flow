"""WP-44: «Importar último paquete» (solo lectura) y fecha de corte con «Usar hoy». Datos ficticios."""
import json
import os
import subprocess
import time
import zipfile

from nurus.capture import write_capture
from nurus.personal import latest_capture as modulo
from test_capture_contract import fixture
from test_integral_ui import run_gui


def paquete(ruta, segundos_atras):
    m, t = fixture()
    write_capture(ruta, m, t)
    marca = time.time() - segundos_atras
    os.utime(ruta, (marca, marca))
    return ruta


def ajeno(ruta, segundos_atras=0):
    with zipfile.ZipFile(ruta, 'w') as archivo:
        archivo.writestr('manifest.json', json.dumps({'schema_id': 'OTRO'}))
    marca = time.time() - segundos_atras
    os.utime(ruta, (marca, marca))
    return ruta


def test_elige_el_mas_reciente_valido_e_ignora_ajenos_y_danados(tmp_path, monkeypatch):
    raiz = tmp_path / 'SITFA_Descargador_Integral'
    descargas = raiz / 'descargas' / 'Lote 2026-10-09'
    descargas.mkdir(parents=True)
    destino = tmp_path / 'Escritorio' / 'Capturas'
    destino.mkdir(parents=True)
    (raiz / 'preferencias.json').write_text(json.dumps({'destino': str(destino)}), encoding='utf-8')
    monkeypatch.setenv('LOCALAPPDATA', str(tmp_path))
    paquete(descargas / 'antiguo.zip', 3600)
    esperado = paquete(destino / 'reciente.zip', 600)
    ajeno(destino / 'ajeno_mas_nuevo.zip')
    (descargas / 'danado.zip').write_bytes(b'no es zip')
    llamadas = []
    monkeypatch.setattr(subprocess, 'Popen', lambda *a, **k: llamadas.append(a))
    assert modulo.search_folders() == [raiz / 'descargas', destino]
    assert modulo.latest_capture() == esperado
    assert not llamadas
    assert modulo.latest_capture([tmp_path / 'vacio']) is None


def test_boton_importa_el_ultimo_y_fecha_de_corte_usar_hoy(tmp_path):
    run_gui(tmp_path, """
import os,time
from unittest.mock import patch
from nurus.capture import write_capture
from test_capture_contract import fixture
os.environ['LOCALAPPDATA']=str(root)
carpeta=root/'SITFA_Descargador_Integral'/'descargas';carpeta.mkdir(parents=True)
m,t=fixture();ruta=write_capture(carpeta/'captura.zip',m,t)
importados=[]
app._import_capture=lambda path=None:importados.append(path)
with patch('nurus.personal.app_base.messagebox.askyesno',return_value=True):
    app._import_latest_capture()
assert importados==[ruta]
for c in app.pages['Trabajo'].winfo_children():pass
from nurus.personal.commands import commands
assert any('Importar último' in c.name for c in commands(app))
# Diálogo de fecha: hoy por defecto, «Usar hoy» restaura, se puede escribir otra.
def cuando_listo(accion):
    win=getattr(app,'cut_date_dialog',None)
    if win is not None and win.winfo_exists() and win.winfo_ismapped():
        try:accion(win)
        except Exception:win.destroy();raise
    else:app.after(100,lambda:cuando_listo(accion))
def preguntar(accion):
    app.cut_date_dialog=None;app.after(100,lambda:cuando_listo(accion))
    return app._ask_cut_date()
def escribir_otra(win):
    assert win.cut_value.get()==date.today().isoformat()
    win.cut_value.set('2026-10-02');win.accept()
assert preguntar(escribir_otra)==date(2026,10,2)
def usar_hoy(win):
    win.cut_value.set('2020-01-01');win.today_button.invoke();win.accept()
assert preguntar(usar_hoy)==date.today()
def invalida_y_cancelar(win):
    win.cut_value.set('10/02/2026');win.accept()
    assert win.winfo_exists()
    win.cancel()
assert preguntar(invalida_y_cancelar) is None
""")
