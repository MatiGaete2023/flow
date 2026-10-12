"""Lotes ficticios: retención60d, protección y confirmación local."""

import os
import time
from pathlib import Path
from types import SimpleNamespace

import pytest
from motor import PocError
from preferencias import old_batches, manage_old_batches, onedrive_warning, RETENCION_DIAS


def _lote(root, name, days):
    folder = root / name
    folder.mkdir(parents=True)
    control = folder / 'resumen.json'
    control.write_text('{}')
    (folder / 'original.xlsx').write_bytes(b'ficticio')
    (folder / 'Bitacoras_RUS.xlsx').write_bytes(b'ficticio bitacoras')
    (folder / 'diagnostico_bitacoras.json').write_text('{}')
    age = time.time() - days * 86400
    os.utime(control, (age, age))
    return folder


def test_solo_lote_de_setenta_dias_se_borra_tras_confirmacion(tmp_path):
    assert RETENCION_DIAS == 60
    lots = [_lote(tmp_path, str(days), days) for days in (10, 40, 70)]
    assert [item['folder'] for item in old_batches(tmp_path)] == [lots[-1]]
    assert old_batches(tmp_path)[0]['bytes'] > 0
    assert manage_old_batches(tmp_path, [lots[-1]]) == [] and lots[-1].exists()
    assert manage_old_batches(tmp_path, [lots[-1]], confirmed=True) == [str(lots[-1])]
    assert not lots[-1].exists() and all(path.exists() for path in lots[:2])


def test_lote_en_uso_no_se_borra_ni_si_es_antiguo(tmp_path):
    current = _lote(tmp_path, 'vigente', 70)
    assert old_batches(tmp_path, protected=[current]) == []
    with pytest.raises(PocError):
        manage_old_batches(tmp_path, [current], confirmed=True, protected=[current])
    assert current.exists()


def test_conservar_archivos_anonimos_y_excel_opcionalmente(tmp_path):
    old = _lote(tmp_path, 'antiguo', 70)
    manage_old_batches(tmp_path, [old], confirmed=True, preserve=True)
    assert {path.name for path in old.iterdir()} == {'Bitacoras_RUS.xlsx', 'diagnostico_bitacoras.json'}
    assert old_batches(tmp_path) == []


def test_archivar_mueve_lote_completo_despues_de_confirmar(tmp_path):
    root = tmp_path / 'descargas'
    old = _lote(root, 'antiguo', 70)
    target = tmp_path / 'archivo'
    assert manage_old_batches(root, [old], archive=target) == [] and old.exists() and not target.exists()
    manage_old_batches(root, [old], archive=target, confirmed=True)
    assert not old.exists() and (target / 'antiguo' / 'original.xlsx').read_bytes() == b'ficticio'
    assert (target / 'antiguo' / 'resumen.json').exists()


def test_revalidar_seleccion_no_toca_lote_reciente(tmp_path):
    old = _lote(tmp_path, 'antiguo', 70)
    recent = _lote(tmp_path, 'reciente', 10)
    with pytest.raises(PocError):
        manage_old_batches(tmp_path, [old, recent], confirmed=True)
    assert old.exists() and recent.exists()
    os.utime(old / 'resumen.json', None)
    with pytest.raises(PocError):
        manage_old_batches(tmp_path, [old], confirmed=True)
    assert old.exists()


def test_ignora_enlaces_carpetas_ajenas_y_rutas_externas(tmp_path):
    root = tmp_path / 'descargas'
    root.mkdir()
    outside = _lote(tmp_path, 'externo', 70)
    (root / 'enlace').symlink_to(outside, target_is_directory=True)
    (root / 'ajeno').mkdir()
    assert old_batches(root) == []
    with pytest.raises(PocError):
        manage_old_batches(root, [outside], confirmed=True)
    assert outside.exists()


def test_archivo_no_sobrescribe_y_no_se_mueve_dentro_del_lote(tmp_path):
    root = tmp_path / 'descargas'
    old = _lote(root, 'antiguo', 70)
    target = tmp_path / 'archivo'
    (target / old.name).mkdir(parents=True)
    with pytest.raises(PocError):
        manage_old_batches(root, [old], confirmed=True, archive=target)
    with pytest.raises(PocError):
        manage_old_batches(root, [old], confirmed=True, archive=old / 'interior')
    assert old.exists()


def test_interfaz_requiere_confirmacion_y_no_actua_ocupada(tmp_path, monkeypatch):
    import ventana_resultados
    from ventana_resultados import ResultsWindow

    old = _lote(tmp_path, 'antiguo', 70)
    messages = []
    window = SimpleNamespace(
        busy=False,
        app=SimpleNamespace(busy=False, dest=SimpleNamespace(get=lambda: str(tmp_path))),
        status=SimpleNamespace(set=messages.append),
        retention_protected=lambda: [],
    )
    monkeypatch.setattr(ventana_resultados.messagebox, 'askyesno', lambda *a, **kw: False)
    assert ResultsWindow.apply_batch_retention(window, [old], preserve=False) == [] and old.exists()
    monkeypatch.setattr(ventana_resultados.messagebox, 'askyesno', lambda *a, **kw: True)
    assert ResultsWindow.apply_batch_retention(window, [old], preserve=False) == [str(old)] and not old.exists()
    window.app.busy = True
    with pytest.raises(PocError):
        ResultsWindow.apply_batch_retention(window, [], preserve=False)


def test_carpeta_defecto_no_depende_de_frozen_y_onedrive_avisa(tmp_path, monkeypatch):
    import tkinter as tk
    import preferencias
    from descargador import Application

    monkeypatch.setenv('LOCALAPPDATA', str(tmp_path))
    expected = preferencias.data_directory() / 'descargas'
    root = tk.Tk()
    root.withdraw()
    try:
        app = Application(root, start_worker=False)
        assert Path(app.dest.get()) == expected
        app.dest.set(str(tmp_path / 'OneDrive' / 'descargas'))
        assert 'OneDrive' in app.destination_warning.get()
        app.dest.set(str(tmp_path / 'local'))
        assert app.destination_warning.get() == ''
    finally:
        root.destroy()
    assert 'OneDrive' in onedrive_warning('C:/ONEDRIVE/descargas')


def test_lote_conjunto_flujo_esta_en_lista_y_se_borra_solo_confirmado(tmp_path):
    joint = _lote(tmp_path, 'conjunto', 70)
    (joint / 'resumen.json').rename(joint / 'flujo.json')
    assert [item['folder'] for item in old_batches(tmp_path)] == [joint]
    assert manage_old_batches(tmp_path, [joint]) == [] and joint.exists()
    manage_old_batches(tmp_path, [joint], confirmed=True)
    assert not joint.exists()


def test_conjunto_raiz_y_bitacoras_de_paquetes_abiertos_se_protegen(tmp_path):
    from ventana_resultados import ResultsWindow

    joint = _lote(tmp_path, 'conjunto', 70)
    (joint / 'resumen.json').rename(joint / 'flujo.json')
    diary = _lote(tmp_path, 'bitacoras', 70)
    (diary / 'resumen.json').rename(diary / 'bitacoras.json')
    unlinked = _lote(tmp_path, 'sin_uso', 70)
    joint_window = SimpleNamespace(winfo_exists=lambda: True, root_folder=joint)
    package_window = SimpleNamespace(
        winfo_exists=lambda: True, package_folders=[], package_diaries=lambda: [diary / 'bitacoras.json']
    )
    app = SimpleNamespace(last_folder=None, result_windows=[joint_window, package_window], recovery=None)
    window = SimpleNamespace(app=app)
    protected = ResultsWindow.retention_protected(window)
    assert joint in protected and diary in protected
    assert [item['folder'] for item in old_batches(tmp_path, protected=protected)] == [unlinked]
    with pytest.raises(PocError):
        manage_old_batches(tmp_path, [joint, diary], protected=protected, confirmed=True)
    assert joint.exists() and diary.exists()


def test_lote_antiguo_con_fuente_anidada_en_uso_se_conserva(tmp_path):
    joint = _lote(tmp_path, 'Conjunto antiguo', 70)
    source = joint / 'descarga'
    source.mkdir()
    (source / 'original.xls').write_bytes(b'fuente ficticia en uso')
    assert old_batches(tmp_path, protected=[source]) == []
    with pytest.raises(PocError):
        manage_old_batches(tmp_path, [joint], confirmed=True, protected=[source])
    assert (source / 'original.xls').read_bytes() == b'fuente ficticia en uso'


def _paquete_por_flujo(tmp_path, monkeypatch, days):
    # Revisión externa H3: paquete creado por «Consultar y crear paquete» real, con runners ficticios.
    from threading import Event

    import flujo_paquete
    from test_flujo_paquete import Scope, runners

    runners(monkeypatch, Event())
    result = flujo_paquete.run(None, {}, lambda *a: None, Event(), Scope(), tmp_path)
    folder = Path(result['path']).parent
    age = time.time() - days * 86400
    for path in [folder, *folder.rglob('*')]:
        os.utime(path, (age, age))
    return folder


def test_paquete_del_flujo_principal_entra_en_la_retencion(tmp_path, monkeypatch):
    folder = _paquete_por_flujo(tmp_path, monkeypatch, 61)
    assert [item['folder'] for item in old_batches(tmp_path)] == [folder]
    manage_old_batches(tmp_path, [folder], confirmed=True)
    assert not folder.exists()


def test_paquete_reciente_o_protegido_no_se_ofrece(tmp_path, monkeypatch):
    recent = _paquete_por_flujo(tmp_path / 'a', monkeypatch, 10)
    assert old_batches(tmp_path / 'a') == []
    old = _paquete_por_flujo(tmp_path / 'b', monkeypatch, 61)
    assert old_batches(tmp_path / 'b', protected=[old / 'Captura_CSMP.zip']) == []
    assert recent.exists() and old.exists()


def test_respaldo_enlazado_no_cuenta_como_paquete(tmp_path):
    outside = tmp_path / 'fuera'
    (outside).mkdir()
    (outside / 'proceso.json').write_text('{}')
    folder = tmp_path / 'destino' / 'Paquete CSMP x'
    folder.mkdir(parents=True)
    (folder / 'respaldo').symlink_to(outside, target_is_directory=True)
    age = time.time() - 70 * 86400
    os.utime(outside / 'proceso.json', (age, age))
    assert old_batches(tmp_path / 'destino') == []


def test_conservar_en_un_paquete_conserva_su_zip(tmp_path, monkeypatch):
    # E3 WP-94 C2: en un paquete las bitácoras están en el ZIP; «conservar» no puede borrarlo.
    folder = _paquete_por_flujo(tmp_path, monkeypatch, 61)
    manage_old_batches(tmp_path, [folder], confirmed=True, preserve=True)
    assert (folder / 'Captura_CSMP.zip').is_file() and not (folder / 'respaldo').exists()


def test_paquete_con_archivo_reciente_o_carpeta_ajena_no_se_ofrece(tmp_path, monkeypatch):
    # E3 WP-94 C4.
    folder = _paquete_por_flujo(tmp_path, monkeypatch, 61)
    (folder / 'nota_del_usuario.txt').write_text('ficticia')
    assert old_batches(tmp_path) == []
    alien = tmp_path / 'otra' / 'Mi carpeta'
    (alien / 'respaldo').mkdir(parents=True)
    (alien / 'respaldo' / 'proceso.json').write_text('{}')
    age = time.time() - 70 * 86400
    for path in [alien, *alien.rglob('*')]:
        os.utime(path, (age, age))
    assert old_batches(tmp_path / 'otra') == []
