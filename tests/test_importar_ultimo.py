"""WP-44: «Importar último paquete» (solo lectura) y fecha de corte con «Usar hoy». Datos ficticios."""

import json
import lzma
import os
import subprocess
import time
import zipfile
import zlib

import pytest

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


@pytest.mark.parametrize('problema', ['deflate_corrupto', 'lzma_corrupto', 'cifrado'])
def test_zip_nuevo_ilegible_no_oculta_paquete_valido_anterior(tmp_path, problema):
    esperado = paquete(tmp_path / 'valido_anterior.zip', 600)
    ilegible = tmp_path / 'nuevo_ilegible.zip'
    compresion = zipfile.ZIP_LZMA if problema == 'lzma_corrupto' else zipfile.ZIP_DEFLATED
    with zipfile.ZipFile(ilegible, 'w', compression=compresion) as archivo:
        archivo.writestr('manifest.json', json.dumps({'schema_id': 'CSMP_CAPTURE'}))
    contenido = bytearray(ilegible.read_bytes())
    if problema in {'deflate_corrupto', 'lzma_corrupto'}:
        nombre = int.from_bytes(contenido[26:28], 'little')
        extra = int.from_bytes(contenido[28:30], 'little')
        inicio = 30 + nombre + extra
        if problema == 'deflate_corrupto':
            # Bloque DEFLATE de tipo reservado: el contenedor ZIP sigue siendo reconocible.
            contenido[inicio] = (contenido[inicio] & ~7) | 7
            error = zlib.error
        else:
            # Propiedades LZMA inválidas tras la cabecera de cuatro bytes.
            contenido[inicio + 4] = 255
            error = lzma.LZMAError
    else:
        # Marcar el archivo ficticio como cifrado basta para exigir una contraseña.
        central = contenido.index(b'PK\x01\x02')
        contenido[6] |= 1
        contenido[central + 8] |= 1
        error = RuntimeError
    ilegible.write_bytes(contenido)
    with zipfile.ZipFile(ilegible) as archivo, pytest.raises(error):
        archivo.read('manifest.json')
    assert modulo.latest_capture([tmp_path]) == esperado


def test_manifest_truncado_se_ignora_sin_ocultar_otro_paquete(tmp_path, monkeypatch):
    esperado = paquete(tmp_path / 'valido_anterior.zip', 600)
    ilegible = ajeno(tmp_path / 'nuevo_ilegible.zip')
    leer = zipfile.ZipFile.read

    def truncado(archivo, *args, **kwargs):
        if archivo.filename == str(ilegible):
            raise EOFError('manifiesto ficticio truncado')
        return leer(archivo, *args, **kwargs)

    monkeypatch.setattr(zipfile.ZipFile, 'read', truncado)
    assert modulo.latest_capture([tmp_path]) == esperado


def test_boton_importa_el_ultimo_y_fecha_de_corte_usar_hoy(tmp_path):
    run_gui(
        tmp_path,
        """
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
    limite=time.monotonic()+5
    while app.busy and time.monotonic()<limite:
        app.update();time.sleep(0.01)
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
""",
    )


def test_busqueda_del_ultimo_paquete_no_bloquea_tk_y_confirma_en_tk(tmp_path):
    run_gui(
        tmp_path,
        """
import threading,time
from unittest.mock import patch
entrada=threading.Event();continuar=threading.Event()
principal=threading.get_ident();hilos=[];confirmados=[];importados=[]
archivo=root/'captura.zip';archivo.write_bytes(b'ficticio')
def buscar():
    hilos.append(threading.get_ident());entrada.set()
    assert continuar.wait(2),'Tk no pudo liberar la búsqueda'
    return archivo
def confirmar(*args,**kwargs):
    assert threading.get_ident()==principal
    confirmados.append(True);return True
app._import_capture=lambda path=None:importados.append(path)
with patch('nurus.personal.latest_capture.latest_capture',side_effect=buscar), patch('nurus.personal.app_base.messagebox.askyesno',side_effect=confirmar):
    try:
        app._import_latest_capture()
        assert entrada.wait(1) and app.busy and hilos!=[principal]
        pulsos=[];app.after(0,lambda:pulsos.append(True));app.update()
        assert pulsos and not confirmados and not importados
        continuar.set()
        limite=time.monotonic()+5
        while app.busy and time.monotonic()<limite:
            app.update();time.sleep(0.01)
        assert not app.busy and confirmados and importados==[archivo]
        assert app.cancel_button.cget('state')=='disabled'
        entrada.clear();continuar.clear()
        app._import_latest_capture()
        assert entrada.wait(1) and app.busy
        app._cancel_operation();continuar.set()
        limite=time.monotonic()+5
        while app.busy and time.monotonic()<limite:
            app.update();time.sleep(0.01)
        assert not app.busy and confirmados==[True] and importados==[archivo]
        assert 'detenida' in app.status.get().lower()
    finally:
        continuar.set()
""",
    )
