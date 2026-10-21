"""Avisos solo mostrar a partir del Excel RUS (decisión del usuario 19-oct-2026, WA-4).

No cambian la observación propuesta, reglas, acciones ni umbrales del motor. N-01 y N-02 se evalúan por fila;
N-03 y N-04 necesitan identidad remota verificada y se marcan al importar un paquete.
"""

from collections import defaultdict
from datetime import timedelta

from .motor.utilidades import fecha_es, fecha_mayoria, fechas_varias, get_date

VENTANA_INFORME_AUDIENCIA = 30


def _dia(valor):
    fecha = get_date(valor)
    return fecha.date() if hasattr(fecha, 'date') else fecha


def _valor(values, cols, key):
    return values.get(cols[key]) if cols.get(key) else None


def avisos_fila(values, cols, mode, as_of, fecha_hoja2=None):
    out = []
    if mode == 'CUMPLIMIENTO':
        mayoria = _dia(fecha_mayoria(_valor(values, cols, 'nacimiento')))
        egreso = _dia(_valor(values, cols, 'egreso_proy'))
        if mayoria and egreso and as_of < mayoria < egreso:
            out.append(
                f'N-01: la medida se proyecta hasta el {fecha_es(egreso)}, posterior a la mayoría de edad '
                f'({fecha_es(mayoria)}).'
            )
    vencimiento = _dia(_valor(values, cols, 'vencimiento')) if mode == 'INFORMES' else _dia(fecha_hoja2)
    if vencimiento and cols.get('prox_aud'):
        audiencias = sorted({_dia(f) for f in fechas_varias(_valor(values, cols, 'prox_aud'))[0]})
        for audiencia in audiencias:
            if as_of <= audiencia < vencimiento <= audiencia + timedelta(days=VENTANA_INFORME_AUDIENCIA):
                out.append(
                    f'N-02: audiencia el {fecha_es(audiencia)} y el informe vence el {fecha_es(vencimiento)}; '
                    'se sugiere solicitarlo antes de la audiencia.'
                )
                break
    return out


def marcar_paquete(rows, provenance, identity):
    """N-03 (varios ingresos de una causa) y N-04 (egresado con informe pendiente), solo con identidad verificada."""
    por_causa = defaultdict(set)
    tipos = defaultdict(set)
    for row in rows:
        record = provenance[row.id]
        if record['identity_status'] != 'VERIFICADA':
            continue
        key = identity(record)
        por_causa[key[:3]].add(key[3])
        tipos[key].add(record['record_type'])
    for row in rows:
        record = provenance[row.id]
        if record['identity_status'] != 'VERIFICADA':
            continue
        key = identity(record)
        ingresos = por_causa[key[:3]]
        if len(ingresos) > 1 and record['record_type'] in ('ESPERA', 'CUMPLIMIENTO', 'INFORMES'):
            row.warnings.append(
                f'N-03: la causa tiene {len(ingresos)} ingresos en este trabajo; considerar un solo correo o proyecto.'
            )
        if {'EGRESADOS', 'INFORMES'} <= tipos[key] and record['record_type'] in ('EGRESADOS', 'INFORMES'):
            row.warnings.append('N-04: medida egresada que sigue con informe pendiente en RUS; revisar.')
