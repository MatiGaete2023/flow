"""R-T29, R-T30 y R-T32: edición humana, nóminas por programa y reglas desactivadas."""
from nurus.personal.config import defaults
from nurus.personal.outputs import prepare_drafts
from nurus.personal.record_edits import apply

from reglas_rt_util import analizar, persona

ESPERA = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'T ESPERA', 'CURADOR']


def test_rt29_recalculo_no_sobrescribe_texto_humano(tmp_path):
    trabajo = analizar(tmp_path, 'ESPERA', ESPERA, [persona(1) + [10, 'SI']])
    fila = trabajo.rows[0]
    apply(trabajo, [fila.id], review={'OBSERVACION': 'Texto revisado por la persona usuaria.'})
    apply(trabajo, [trabajo.rows[0].id], overrides={'espera': 45})
    fila = trabajo.rows[0]
    assert 'ESPERA.E05_PROYECTO_Y_CORREO' in fila.rules and 'ESPERA.E06_SIN_RESOLUCION' not in fila.rules
    assert fila.review['OBSERVACION'] == 'Texto revisado por la persona usuaria.'


def test_rt30_nominas_exactas_por_programa(tmp_path):
    filas = [persona(1) + [45, 'SI'], persona(2) + [60, 'SI'], persona(3) + [90, 'SI'],
             persona(4, programa='PRM DOS') + [45, 'SI'], persona(5) + [10, 'SI']]
    trabajo = analizar(tmp_path, 'ESPERA', ESPERA, filas)
    borradores = prepare_drafts(trabajo, 'programa_espera', directory=tmp_path / 'salida')
    rit = {fila.id: fila.values['RIT'] for fila in trabajo.rows}
    por_programa = {b.program: b for b in borradores}
    assert len(borradores) == 2 and set(por_programa) == {'PRM UNO', 'PRM DOS'}
    for programa, esperadas in (('PRM UNO', ['X-1-2026', 'X-2-2026', 'X-3-2026']), ('PRM DOS', ['X-4-2026'])):
        ids = por_programa[programa].record_ids
        assert len(ids) == len(set(ids)) == len(esperadas)
        assert sorted(rit[i] for i in ids) == esperadas


def test_rt32_regla_desactivada_se_omite_y_se_conserva_la_configuracion(tmp_path):
    configuracion = defaults()
    configuracion['desactivadas'] = ['COMUN.CURADOR']
    umbrales = dict(configuracion['umbrales'])
    trabajo = analizar(tmp_path, 'ESPERA', ESPERA, [persona(1) + [45, '']], configuracion=configuracion)
    assert 'COMUN.CURADOR' not in trabajo.rows[0].rules
    assert trabajo.rows[0].rules == ['ESPERA.E05_PROYECTO_Y_CORREO']
    assert configuracion['umbrales'] == umbrales and configuracion['desactivadas'] == ['COMUN.CURADOR']
    activo = analizar(tmp_path, 'ESPERA', ESPERA, [persona(1) + [45, '']])
    assert 'COMUN.CURADOR' in activo.rows[0].rules
