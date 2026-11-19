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


ADECUADAS = ('COMPLETA', 'VACIA_COMPROBADA')


def _instante(texto):
    """(día, instante) comparables de `captured_at`: el mismo criterio en Python y Rust (sin depender del parser)."""
    texto = str(texto or '')
    return (texto[:10], texto.replace('Z', '+00:00'))


def _mejores_consultas(capture, pantalla, filtro, corte=None):
    """{clave: consulta} con la ÚLTIMA lectura fiable al corte por clave (E3-07); clave = tupla declarada en
    filters_json[filtro].

    Se descartan las lecturas posteriores al corte. Entre lecturas completas (con filas o vacías comprobadas) manda la más
    reciente; si dos completas del mismo instante discrepan en cobertura, la elegida queda PARCIAL (no acredita unicidad).
    Sin lecturas completas, la de mejor cobertura (y la más reciente ante empate)."""
    grupos = defaultdict(list)
    for consulta in capture.manifest['queries']:
        if consulta['screen'] != pantalla:
            continue
        alcance = json.loads(consulta['filters_json']).get(filtro)
        if not alcance:
            continue
        if corte is not None and _instante(consulta['captured_at'])[0] > corte.isoformat():
            continue
        clave = tuple(alcance.get(k, '') for k in ('source_system', 'court_code', 'cause_id' if filtro == 'escritos_causa' else 'person_id'))
        grupos[clave].append(consulta)
    mejor = {}
    for clave, consultas in grupos.items():
        completas = [q for q in consultas if q['coverage'] in ADECUADAS]
        if completas:
            ultimo = max(_instante(q['captured_at']) for q in completas)
            mismas = [q for q in completas if _instante(q['captured_at']) == ultimo]
            elegida = mismas[-1]
            if len({q['coverage'] for q in mismas}) > 1:
                elegida = {**elegida, 'coverage': 'PARCIAL'}  # lecturas discordantes del mismo instante
            mejor[clave] = elegida
        else:
            mejor[clave] = max(consultas, key=lambda q: (_RANGO.get(q['coverage'], 0), _instante(q['captured_at'])))
    return mejor


def _cobertura(consulta):
    """Histórico: la identidad incierta se lleva aparte (campo `identidad`); su cobertura cuenta como parcial."""
    estado = consulta['coverage']
    return 'PARCIAL' if estado == 'IDENTIDAD_INCIERTA' else estado


def filas_de_escritos(capture, corte=None):
    """{(sistema, tribunal, causa): (consulta, [filas])} de la tabla `escritos`, con la lectura vigente al corte."""
    por_consulta = defaultdict(list)
    for fila in capture.rows('escritos'):
        por_consulta[fila['query_id']].append(fila)
    return {
        clave: (consulta, por_consulta.get(consulta['query_id'], []))
        for clave, consulta in _mejores_consultas(capture, 'escritos', 'escritos_causa', corte).items()
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
        ajeno = bool(fila['linked_ingreso_id']) and fila['linked_ingreso_id'] != registro['ingreso_id']
        escritos.append(
            Escrito(
                tipo=fila['escrito_type'],
                estado=fila['escrito_status'],
                doc=fila['doc'],
                solicitante=fila['requester'],
                fila=int(fila['original_row'] or numero),
                vinculo='EXPLICITO' if propio else 'AJENO' if ajeno else '',
                fecha_proyectada=_dia(fila['linked_projection']) if propio else None,
            )
        )
    leido = _dia(consulta['captured_at'])
    # La identidad incierta de Escritos no se rebaja a «parcial»: no acredita nada (E3-05).
    return Escritos(consulta['coverage'], leido, tuple(escritos))


def historicos_por_persona(capture, corte=None):
    """{(sistema, tribunal, persona): Historico}; la pertenencia a la persona seleccionada viene declarada por fila."""
    por_consulta = defaultdict(list)
    for fila in capture.rows('historico_proteccion'):
        por_consulta[fila['query_id']].append(fila)
    salida = {}
    for clave, consulta in _mejores_consultas(capture, 'historico_proteccion', 'historico_persona', corte).items():
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

    Vínculo: el paquete asocia un escrito de la causa a un ingreso y a una fecha proyectada; las filas de Historia que llevan
    la misma referencia documental (`Doc.`) se enlazan a ese informe. Los folios enlazados a OTRO informe de la causa se
    devuelven como `folios_ajenos` para que la vía genérica no los atribuya a este (E3-10). Sin referencia no se enlaza nada."""
    if vencimiento is None:
        return None
    hallado = fuentes.get((registro['source_system'], registro['court_code'], registro['cause_id']))
    if hallado is None:
        return None
    propio = (registro['ingreso_id'], vencimiento.isoformat())
    docs = defaultdict(set)  # (ingreso, proyección) -> referencias documentales
    for f in hallado[1]:
        if f['linked_ingreso_id'] and f['linked_projection'] and clave_texto(f['doc']):
            docs[(f['linked_ingreso_id'], f['linked_projection'][:10])].add(clave_texto(f['doc']))
    escritos_informe = {clave_texto(n) for n in ESCRITOS_DE_INFORME}
    folio_escrito = folio_resolucion = ''
    fecha_resolucion = None
    ajenos = set()
    for fila in historia_filas:
        referencia = clave_texto(fila.get('reference', ''))
        folio = str(fila.get('folio', '')).strip()
        if not referencia:
            continue
        duenos = [k for k, d in docs.items() if referencia in d]
        if any(k != propio for k in duenos) and folio:
            ajenos.add(folio)
        if propio not in duenos:
            continue
        descripcion = clave_texto(fila['description'])
        if descripcion in escritos_informe:
            folio_escrito = folio or folio_escrito
        elif descripcion == clave_texto(INGRESO_INFORME):
            folio_resolucion = folio or folio_resolucion
            fecha_resolucion = _dia(fila.get('event_at'))
    ajenos -= {folio_escrito, folio_resolucion}
    if not (folio_escrito or folio_resolucion or ajenos):
        return None
    return EvidenciaInforme(folio_escrito, folio_resolucion, fecha_resolucion, '', vencimiento, tuple(sorted(ajenos)))
