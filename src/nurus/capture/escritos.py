"""Escritos de la causa e Ingreso Histórico Protección de la persona: del paquete de captura a la evidencia del motor.

Plan de integración 18-nov-2026. Solo transforma lo ya capturado; no consulta nada. Cada fuente conserva su estado
(no solicitada, leída sin filas, con filas, parcial, fallida, identidad incierta); nunca se reduce a «lista vacía».
"""

from __future__ import annotations

import json
from collections import defaultdict
from datetime import UTC, date

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
    """(día de la lectura tal como se registró, instante comparable en UTC) de `captured_at` (E3B-01).

    El día del corte se compara con la fecha local escrita en la lectura (zona del tribunal, America/Santiago en la
    práctica). El orden entre lecturas usa el instante convertido a UTC: dos husos distintos se comparan bien. Un valor sin
    zona se compara tal cual. Mismo criterio en Rust (`fuentes_nuevas.rs::instante`)."""
    from datetime import datetime

    texto = str(texto or '').strip()
    dia = texto[:10]
    try:
        valor = datetime.fromisoformat(texto)
        if not 1900 <= valor.year <= 2199:
            raise ValueError  # fuera del dominio pactado (E3C-08): se compara como texto, igual que en Rust
        if valor.tzinfo is not None:
            valor = valor.astimezone(UTC).replace(tzinfo=None)
    except (ValueError, OverflowError):
        return (dia, texto)
    return (dia, valor.strftime('%Y-%m-%dT%H:%M:%S.%f'))


_FIRMA = {
    'escritos': ('escrito_type', 'escrito_status', 'doc', 'requester', 'linked_ingreso_id', 'linked_projection'),
    'historico_proteccion': ('program_name', 'egreso_date', 'egreso_cause', 'ownership', 'person_id'),
}


def _firmas(capture, pantalla):
    """{query_id: filas normalizadas ordenadas}: para saber si dos lecturas del mismo instante dicen lo mismo. Es un
    MULTICONJUNTO (E3C-02): dos escritos iguales no valen lo mismo que uno; el orden de las filas no importa."""
    salida = defaultdict(list)
    for fila in capture.rows(pantalla):
        salida[fila['query_id']].append(tuple(clave_texto(fila.get(k, '')) for k in _FIRMA[pantalla]))
    return {q: tuple(sorted(filas)) for q, filas in salida.items()}


def _mejores_consultas(capture, pantalla, filtro, corte=None):
    """{clave: consulta} con la ÚLTIMA lectura fiable al corte por clave (E3-07); clave = tupla declarada en
    filters_json[filtro].

    Se descartan las lecturas cuyo día es posterior al corte. Entre lecturas completas (con filas o vacías comprobadas) manda la
    más reciente en UTC. Si varias completas comparten ese instante y discrepan (cobertura o filas), la elegida (la de menor
    `query_id`, igual en cualquier orden del manifiesto) queda PARCIAL y marcada `_discordante` (E3B-02). Sin completas, la
    de mejor cobertura, luego la más reciente y, ante empate, la de menor `query_id`."""
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
    firmas = _firmas(capture, pantalla) if grupos else {}
    mejor = {}
    for clave, consultas in grupos.items():
        completas = [q for q in consultas if q['coverage'] in ADECUADAS]
        if completas:
            ultimo = max(_instante(q['captured_at'])[1] for q in completas)
            mismas = sorted((q for q in completas if _instante(q['captured_at'])[1] == ultimo), key=lambda q: q['query_id'])
            elegida = mismas[0]
            discordantes = len({q['coverage'] for q in mismas}) > 1 or len({firmas.get(q['query_id'], ()) for q in mismas}) > 1
            if discordantes:
                empatadas = [q['query_id'] for q in mismas]
                elegida = {**elegida, 'coverage': 'PARCIAL', '_discordante': True, '_empatadas': empatadas}
            mejor[clave] = elegida
        else:
            rango = max(_RANGO.get(q['coverage'], 0) for q in consultas)
            candidatas = [q for q in consultas if _RANGO.get(q['coverage'], 0) == rango]
            ultimo = max(_instante(q['captured_at'])[1] for q in candidatas)
            mejor[clave] = min((q for q in candidatas if _instante(q['captured_at'])[1] == ultimo), key=lambda q: q['query_id'])
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
    salida = {}
    for clave, consulta in _mejores_consultas(capture, 'escritos', 'escritos_causa', corte).items():
        if consulta.get('_discordante'):
            # Las filas de TODAS las lecturas empatadas: sus referencias quedan bloqueadas (E3C-01).
            consulta = {**consulta, '_filas_empate': [f for q in consulta['_empatadas'] for f in por_consulta.get(q, [])]}
        salida[clave] = (consulta, por_consulta.get(consulta['query_id'], []))
    return salida


def escritos_para(registro, fuentes):
    """`Escritos` del motor para un registro de ingreso verificado, con la asociación explícita de ESE ingreso.

    Un escrito solo queda «EXPLICITO» si el paquete lo asoció a este ingreso y a una fecha proyectada. Los demás son
    escritos de la causa sin atribución."""
    hallado = fuentes.get((registro['source_system'], registro['court_code'], registro['cause_id']))
    if hallado is None:
        return None
    consulta, filas = hallado
    # Lecturas simultáneas que se contradicen: ninguna asociación explícita vale (E3C-01); el motor pide revisión.
    discordante = bool(consulta.get('_discordante'))
    escritos = []
    for numero, fila in enumerate(filas, 1):
        propio = fila['linked_ingreso_id'] == registro['ingreso_id'] and fila['linked_projection'] and not discordante
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
    return Escritos(consulta['coverage'], leido, tuple(escritos), discordante)


def historicos_por_persona(capture, corte=None):
    """{(sistema, tribunal, persona): Historico}; la pertenencia a la persona seleccionada viene declarada por fila."""
    por_consulta = defaultdict(list)
    for fila in capture.rows('historico_proteccion'):
        por_consulta[fila['query_id']].append(fila)
    salida = {}
    for clave, consulta in _mejores_consultas(capture, 'historico_proteccion', 'historico_persona', corte).items():
        filas = por_consulta.get(consulta['query_id'], [])
        verificada = bool(filas) and all(f['ownership'] == 'VERIFICADA' for f in filas)
        incierta = (
            consulta['coverage'] == 'IDENTIDAD_INCIERTA' or (bool(filas) and not verificada) or consulta.get('_discordante')
        )
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
    la misma referencia documental (`Doc.`) se enlazan a ese informe. Sin referencia no se enlaza nada.
    - Una lectura de Escritos con identidad incierta no asocia nada (E3B-03).
    - Una referencia con un único dueño se atribuye a ese informe; los folios de un dueño distinto vuelven como
      `folios_ajenos` para que la vía genérica no los atribuya aquí (E3-10).
    - Una referencia con VARIOS dueños es ambigua: no se atribuye positivamente a ninguno y sus folios quedan bloqueados
      para la vía genérica de todos (E3B-04).
    - Si la lectura elegida discrepa de otra del mismo instante, nada se atribuye y los folios enlazados en CUALQUIERA de
      ellas quedan bloqueados (E3C-01)."""
    if vencimiento is None:
        return None
    hallado = fuentes.get((registro['source_system'], registro['court_code'], registro['cause_id']))
    if hallado is None or hallado[0]['coverage'] == 'IDENTIDAD_INCIERTA':
        return None
    propio = (registro['ingreso_id'], vencimiento.isoformat())
    discordante = bool(hallado[0].get('_discordante'))
    docs = defaultdict(set)  # referencia documental -> {(ingreso, proyección)}
    for f in hallado[0].get('_filas_empate', hallado[1]):
        if f['linked_ingreso_id'] and f['linked_projection'] and clave_texto(f['doc']):
            docs[clave_texto(f['doc'])].add((f['linked_ingreso_id'], f['linked_projection'][:10]))
    escritos_informe = {clave_texto(n) for n in ESCRITOS_DE_INFORME}
    folio_escrito = folio_resolucion = ''
    fecha_resolucion = None
    ajenos = set()
    for fila in historia_filas:
        referencia = clave_texto(fila.get('reference', ''))
        folio = str(fila.get('folio', '')).strip()
        duenos = docs.get(referencia, set()) if referencia else set()
        if not duenos:
            continue
        if duenos != {propio} or discordante:
            if folio:
                ajenos.add(folio)  # de otro informe, ambigua (varios dueños) o de lecturas discordantes (E3C-01)
            continue
        descripcion = clave_texto(fila['description'])
        if descripcion in escritos_informe:
            folio_escrito = folio or folio_escrito
        elif descripcion == clave_texto(INGRESO_INFORME):
            folio_resolucion = folio or folio_resolucion
            fecha_resolucion = _dia(fila.get('event_at'))
    ajenos -= {folio_escrito, folio_resolucion} - {''}
    if not (folio_escrito or folio_resolucion or ajenos):
        return None
    return EvidenciaInforme(folio_escrito, folio_resolucion, fecha_resolucion, '', vencimiento, tuple(sorted(ajenos)))
