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
