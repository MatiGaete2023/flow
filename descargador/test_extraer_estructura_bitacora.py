import json

from extraer_estructura_bitacora import main
from test_bitacora_tabla_cortada import PARAMS
from test_lectura_bitacoras import popup


def test_estructura_sin_textos_ni_identificadores(tmp_path, capsys):
    path = tmp_path / 'b.html'
    path.write_text(popup(PARAMS), encoding='utf-8')
    assert main([str(path)]) == 0
    salida = capsys.readouterr().out
    copia = json.loads(salida)['copias'][0]
    assert copia['lector_entradas'] == 1 and copia['lxml']['filas_en_tabla'] == 2
    assert copia['bruto']['hobs_centro_total'] == 1 and copia['COD_Etapa'] == '3'
    for dato in ('Texto completo', 'Persona ficticia', 'X-1-2026', '201', 'PRM Centro', 'Usuario ficticio'):
        assert dato not in salida
