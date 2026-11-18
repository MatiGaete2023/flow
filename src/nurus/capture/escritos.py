"""Escritos de la causa e Ingreso Histórico Protección de la persona: del paquete de captura a la evidencia del motor.

Plan de integración 18-nov-2026. Solo transforma lo ya capturado; no consulta nada. Cada fuente conserva su estado
(no solicitada, leída sin filas, con filas, parcial, fallida, identidad incierta); nunca se reduce a «lista vacía».
"""

from __future__ import annotations

import json
from collections import defaultdict
from datetime import date

from nurus.personal.motor.evidencia import (
    Escrito,
    Escritos,
    EvidenciaInforme,
    Historico,
    ProgramaHistorico,
    clave_texto,
)

_RANGO = {'COMPLETA': 3, 'PARCIAL': 2, 'VACIA_COMPROBADA': 1}
ESCRITOS_DE_INFORME = {
    'Inf. Cump Programa Ambulatorio',
    'Inf. Cump FAE/FAS',
    'Inf. Cump Centro Residencial',
}
INGRESO_INFORME = 'Ingreso informe de cumplimiento X'


def _dia(texto):
    try:
        return date.fromisoformat(str(texto or '')[:10])
    except ValueError:
        return None


def _mejores_consultas(capture, pantalla, filtro):
    """{clave: consulta} con la mejor cobertura por clave; clave = tupla declarada en filters_json[filtro]."""
    mejor = {}
    for consulta in capture.manifest['queries']:
        if consulta['screen'] != pantalla:
            continue
        alcance = json.loads(consulta['filters_json']).get(filtro)
        if not alcance:
            continue
        clave = tuple(alcance.get(k, '') for k in ('source_system', 'court_code', 'cause_id' if filtro == 'escritos_causa' else 'person_id'))
        actual = mejor.get(clave)
        if actual is None or _RANGO.get(consulta['coverage'], 0) > _RANGO.get(actual['coverage'], 0):
            mejor[clave] = consulta
    return mejor


def _cobertura(consulta):
    estado = consulta['coverage']
    return 'PARCIAL' if estado == 'IDENTIDAD_INCIERTA' else estado


def filas_de_escritos(capture):
    """{(sistema, tribunal, causa): (consulta, [filas])} de la tabla `escritos`."""
    por_consulta = defaultdict(list)
    for fila in capture.rows('escritos'):
        por_consulta[fila['query_id']].append(fila)
    return {
        clave: (consulta, por_consulta.get(consulta['query_id'], []))
        for clave, consulta in _mejores_consultas(capture, 'escritos', 'escritos_causa').items()
    }


def escritos_para(registro, fuentes):
    """`Escritos` del motor para un registro de ingreso verificado, con la asociación explícita de ESE ingreso.

    Un escrito solo queda «EXPLICITO» si el paquete lo asoció a este ingreso y a una fecha proyectada. Los demás son
    escritos de la causa sin atribución."""
    hallado = fuentes.get((registro['source_system'], registro['court_code'], registro['cause_id']))
    if hallado is None:
        return None
    consulta, filas = hallado
    escritos = []
    for numero, fila in enumerate(filas, 1):
        propio = fila['linked_ingreso_id'] == registro['ingreso_id'] and fila['linked_projection']
        escritos.append(
            Escrito(
                tipo=fila['escrito_type'],
                estado=fila['escrito_status'],
                doc=fila['doc'],
                solicitante=fila['requester'],
                fila=int(fila['original_row'] or numero),
                vinculo='EXPLICITO' if propio else '',
                fecha_proyectada=_dia(fila['linked_projection']) if propio else None,
            )
        )
    leido = _dia(consulta['captured_at'])
    return Escritos(_cobertura(consulta), leido, tuple(escritos))


def historicos_por_persona(capture):
    """{(sistema, tribunal, persona): Historico}; la pertenencia a la persona seleccionada viene declarada por fila."""
    por_consulta = defaultdict(list)
    for fila in capture.rows('historico_proteccion'):
        por_consulta[fila['query_id']].append(fila)
    salida = {}
    for clave, consulta in _mejores_consultas(capture, 'historico_proteccion', 'historico_persona').items():
        filas = por_consulta.get(consulta['query_id'], [])
        verificada = bool(filas) and all(f['ownership'] == 'VERIFICADA' for f in filas)
        incierta = consulta['coverage'] == 'IDENTIDAD_INCIERTA' or (bool(filas) and not verificada)
        salida[clave] = Historico(
            _cobertura(consulta),
            _dia(consulta['captured_at']),
            'INCIERTA' if incierta else 'VERIFICADA',
            tuple(
                ProgramaHistorico(f['program_name'], f['egreso_date'], f['egreso_cause'], int(f['original_row'] or n))
                for n, f in enumerate(filas, 1)
            ),
        )
    return salida


def historico_para(registro, historicos):
    return historicos.get((registro['source_system'], registro['court_code'], registro['person_id']))


def evidencia_informe(registro, vencimiento, fuentes, historia_filas):
    """Asociación explícita de un informe (ingreso + fecha proyectada) con sus evidencias, o None si no hay vínculo.

    Vínculo: el paquete asocia un escrito de la causa a este ingreso y a la fecha proyectada; las filas de Historia que
    llevan la misma referencia documental (`Doc.`) se enlazan al mismo informe. Sin esa referencia no se enlaza nada."""
    if vencimiento is None:
        return None
    hallado = fuentes.get((registro['source_system'], registro['court_code'], registro['cause_id']))
    if hallado is None:
        return None
    iso = vencimiento.isoformat()
    vinculados = [
        f for f in hallado[1]
        if f['linked_ingreso_id'] == registro['ingreso_id'] and f['linked_projection'][:10] == iso
    ]  # fmt: skip
    docs = {clave_texto(f['doc']) for f in vinculados if clave_texto(f['doc'])}
    folio_escrito = folio_resolucion = ''
    fecha_resolucion = None
    for fila in historia_filas:
        if clave_texto(fila.get('reference', '')) not in docs or not docs:
            continue
        descripcion = clave_texto(fila['description'])
        if descripcion in {clave_texto(n) for n in ESCRITOS_DE_INFORME}:
            folio_escrito = str(fila.get('folio', '')).strip() or folio_escrito
        elif descripcion == clave_texto(INGRESO_INFORME):
            folio_resolucion = str(fila.get('folio', '')).strip() or folio_resolucion
            fecha_resolucion = _dia(fila.get('event_at'))
    if not (folio_escrito or folio_resolucion):
        return None
    return EvidenciaInforme(folio_escrito, folio_resolucion, fecha_resolucion, '', vencimiento)
