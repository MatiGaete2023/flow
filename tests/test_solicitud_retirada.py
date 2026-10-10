from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_botones_de_solicitud_de_antecedentes_retirados_y_modulo_declarativo_conservado():
    for ruta, prohibidos in {
        'src/nurus/personal/results_view.py': ('Solicitar antecedentes faltantes', '_capture_requests'),
        'src/nurus/personal/app_base.py': ('_capture_requests',),
        'descargador/ventana_resultados.py': ('Abrir solicitud de antecedentes', 'open_request', 'solicitud_antecedentes'),
    }.items():
        texto = (ROOT / ruta).read_text(encoding='utf-8')
        for prohibido in prohibidos:
            assert prohibido not in texto, (ruta, prohibido)
    assert not (ROOT / 'descargador' / 'solicitud_antecedentes.py').exists()
    from nurus.capture.requests import read_request
    from nurus.personal.capture_products import export_requests
    assert callable(read_request) and callable(export_requests)
