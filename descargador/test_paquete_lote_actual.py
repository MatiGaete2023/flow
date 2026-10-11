"""WP-43: «Crear paquete de este lote» sin abrir Resultados. Datos ficticios."""

import os
import subprocess
import sys
import textwrap
from pathlib import Path

import pytest

from archivo_local import load_sources
from descargador import package_current_lot
from export_csmp_capture import export_capture
from motor import PocError
from nurus.capture import read_capture
from test_reutilizacion_local import bound_lot, diaries

ROOT = Path(__file__).resolve().parents[1]


def tablas(path):
    capture = read_capture(path)
    claves = ('registros', 'bitacoras', 'incidencias')
    return {t: sorted(map(repr, capture.rows(t))) for t in claves}, capture.manifest['table_states']


def test_paquete_del_lote_actual_igual_a_la_ruta_de_resultados(tmp_path):
    lote = bound_lot(tmp_path / 'principal')
    manifiesto = diaries(tmp_path / 'Bitacoras 2026-10-10', 2)
    nuevo = package_current_lot(lote.folder, manifiesto, tmp_path / 'nuevo.zip')
    actual = export_capture(
        load_sources([lote.folder]), tmp_path / 'actual.zip', diaries=[manifiesto], match_diaries=True
    )
    assert tablas(nuevo['path']) == tablas(actual)
    assert len(read_capture(nuevo['path']).rows('bitacoras')) == 2
    validado = subprocess.run(
        [sys.executable, str(ROOT / 'tools' / 'flow_capture_cli.py'), 'validate', nuevo['path']],
        capture_output=True,
        text=True,
        cwd=ROOT,
        check=False,
    )
    assert validado.returncode == 0, validado.stdout + validado.stderr
    with pytest.raises(PocError, match='ya existe'):
        package_current_lot(lote.folder, manifiesto, tmp_path / 'nuevo.zip')


def test_boton_se_habilita_tras_lote_y_bitacoras_y_no_abre_resultados(tmp_path):
    lote = bound_lot(tmp_path / 'principal')
    manifiesto = diaries(tmp_path / 'Bitacoras 2026-10-10', 1)
    source = textwrap.dedent('''
        from pathlib import Path
        import sys,tkinter as tk
        import preferencias
        from descargador import Application
        from test_lotes import dispose_interface
        lote,bitacoras,salida=map(Path,sys.argv[1:4]);preferencias.data_directory=lambda:salida.parent
        root=tk.Tk();root.withdraw();app=Application(root,start_worker=False)
        try:
            assert str(app.package_button.cget('state'))=='disabled'
            app.process_event('folder',str(lote))
            app.process_event('done',{'queries':1,'files':1,'records':2})
            assert str(app.package_button.cget('state'))=='disabled'
            app.process_event('diary_done',{'folder':str(bitacoras),'leidas':1,'fallidas':0,'estado':'COMPLETA'})
            assert str(app.package_button.cget('state'))=='normal'
            pedidos=[]
            app.enqueue=lambda comando,arg=None:pedidos.append((comando,arg))
            app.show_results=lambda:(_ for _ in ()).throw(AssertionError('no debe abrir Resultados'))
            import descargador
            descargador.filedialog.asksaveasfilename=lambda **kw:str(salida)
            app.create_package()
            assert pedidos==[('package',(lote,bitacoras/'bitacoras.json',salida,True))]
            resultado=descargador.package_current_lot(*pedidos[0][1])
            app.process_event('package_done',resultado)
            assert 'Paquete creado: '+salida.name in app.status.get()
            app.create_package()
            assert 'ya existe' in app.status.get() and len(pedidos)==1
        finally:dispose_interface(app,root)
    ''')
    result = subprocess.run(
        [sys.executable, '-c', source, str(lote.folder), str(manifiesto.parent), str(tmp_path / 'p.zip')],
        cwd=Path(__file__).parent,
        env={**os.environ, 'PYTHONIOENCODING': 'utf-8'},
        capture_output=True,
        text=True,
        encoding='utf-8',
        timeout=60,
    )
    assert result.returncode == 0, result.stderr


def test_paquete_con_consolidado_si_el_lote_es_apto(tmp_path):
    lote = bound_lot(tmp_path / 'principal')
    manifiesto = diaries(tmp_path / 'Bitacoras 2026-10-10', 1)
    resultado = package_current_lot(lote.folder, manifiesto, tmp_path / 'con.zip', consolidated=True)
    capture = read_capture(resultado['path'])
    assert capture.manifest['schema_version'] == '1.1.0' and capture.consolidated
    assert 'Incluye el XLSX consolidado' in resultado['reuse']
    sin = package_current_lot(lote.folder, manifiesto, tmp_path / 'sin.zip')
    assert read_capture(sin['path']).manifest['schema_version'] == '1.0.0'


def test_lote_no_apto_crea_el_paquete_sin_consolidado_y_lo_dice(tmp_path, monkeypatch):
    import descargador
    import export_csmp_capture
    from motor import PocError as Error

    def no_apto(lots):
        raise Error('No apto para CSMP: consulta ficticia')

    monkeypatch.setattr(export_csmp_capture, 'consolidated_workbook', no_apto)
    lote = bound_lot(tmp_path / 'principal')
    manifiesto = diaries(tmp_path / 'Bitacoras 2026-10-10', 1)
    resultado = descargador.package_current_lot(lote.folder, manifiesto, tmp_path / 'p.zip', consolidated=True)
    assert read_capture(resultado['path']).manifest['schema_version'] == '1.0.0'
    assert 'Sin XLSX consolidado: No apto para CSMP' in resultado['reuse']


def test_evento_que_falla_no_detiene_la_cola(tmp_path):
    source = textwrap.dedent('''
        from pathlib import Path
        import sys,tkinter as tk
        from descargador import Application
        from test_lotes import dispose_interface
        carpeta=Path(sys.argv[1])
        root=tk.Tk();root.withdraw();app=Application(root,start_worker=False)
        try:
            # Respuesta a CSMP en carpeta inexistente: joint_done falla al escribir.
            app.csmp_reply=str(carpeta/'no'/'existe'/'respuesta.json')
            app.busy=True
            app.events.put(('joint_done',{'folder':str(carpeta),'archivo':str(carpeta/'libro.xlsx'),'estado':'x','modo':'ESPERA'}))
            app.events.put(('idle',None))
            app.poll()
            assert app.events.empty() and not app.busy
            assert 'joint_done' in app.status.get()
        finally:dispose_interface(app,root)
    ''')
    (tmp_path / 'libro.xlsx').write_bytes(b'ficticio')
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


def test_paquete_genera_el_consolidado_una_sola_vez(tmp_path, monkeypatch):
    import export_csmp_capture

    lote = bound_lot(tmp_path / 'principal')
    manifiesto = diaries(tmp_path / 'Bitacoras 2026-10-10', 1)
    calls = []
    original = export_csmp_capture.consolidated_workbook

    def counted(lots):
        calls.append(True)
        return original(lots)

    monkeypatch.setattr(export_csmp_capture, 'consolidated_workbook', counted)
    resultado = package_current_lot(lote.folder, manifiesto, tmp_path / 'p.zip', consolidated=True)
    assert read_capture(resultado['path']).consolidated
    assert len(calls) == 1
