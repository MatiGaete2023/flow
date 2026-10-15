"""Funciones puras de las reglas de historia: sin textos ni contexto, fáciles de probar."""

from __future__ import annotations

import re

from .historia import clave

ORDEN = 'Orden de búsqueda'
CIERRES = ('Termino de búsqueda', 'Deja sin efecto búsqueda')
MATERIAS = {
    'CURADOR': ('no registra curador ad litem',),
    'SIN_FICHA': ('no registra ficha individual',),
    'SIN_FAE': ('no registra ficha fae',),
    'VENCIDA': ('medida vencida en rus',),
}
_INSTITUCION = re.compile(r'Instituci[oó]n:\s*(.+)$', re.IGNORECASE)


def posterior(a, b):
    """True si el trámite `a` es posterior a `b`: por folio si ambos son numéricos; si no, por fecha."""
    if a.folio >= 0 and b.folio >= 0:
        return a.folio > b.folio
    return a.fecha > b.fecha


def busqueda(historia):
    """(cierres, vigentes). cierres: lista de (cierre, orden|None), la orden es la anterior más cercana.
    vigentes: órdenes sin ningún cierre posterior."""
    ordenes = historia.buscar(ORDEN)
    cierres = historia.buscar(*CIERRES)
    pares = []
    for cierre in cierres:
        previas = [o for o in ordenes if posterior(cierre, o)]
        pares.append((cierre, max(previas, key=lambda o: (o.fecha, o.folio)) if previas else None))
    vigentes = [o for o in ordenes if not any(posterior(c, o) for c in cierres)]
    return pares, vigentes


def instituciones(celda):
    """Categorías distintas tras «Institución:» en una celda «(RUT)NOMBRE - Institución: X#(RUT)…»."""
    categorias = set()
    for entrada in str(celda or '').split('#'):
        encontrada = _INSTITUCION.search(entrada.strip())
        if encontrada:
            categorias.add(clave(encontrada.group(1)))
    return categorias


def materias(texto):
    """Materias de REITERA presentes en el texto de una observación previa."""
    limpio = clave(texto)
    if limpio.startswith('se reitera'):
        return set()
    return {m for m, patrones in MATERIAS.items() if any(p in limpio for p in patrones)}


def primera_observacion(bitacora, materia, antes_de):
    """Primera observación (la de texto completo) de la materia, anterior a la fecha de corte."""
    for obs in bitacora:
        if obs.fecha < antes_de and materia in materias(obs.texto):
            return obs
    return None


def rut_clave(rut):
    """RUT comparable: solo dígitos y K, sin puntos, guion ni espacios."""
    return re.sub(r'[^0-9K]', '', str(rut or '').upper())


def duplicidades(registros):
    """registros: iterable de (record_id, tribunal, rit, rut, programa) de UNA etapa.
    Devuelve {record_id: 'MISMO_PROGRAMA' | 'OTRO_REGISTRO'} solo para causas con más de un registro.
    MISMO_PROGRAMA: el mismo NNA (RUT) tiene otro registro en el mismo programa (regla 6, por NNA; sin RUT no
    se afirma). OTRO_REGISTRO: la causa tiene otros registros (otro programa u otro NNA, p. ej. hermanos)."""
    causas = {}
    for record_id, tribunal, rit, rut, programa in registros:
        if not clave(rit):
            continue
        causas.setdefault((clave(tribunal), clave(rit)), []).append((record_id, rut_clave(rut), clave(programa)))
    salida = {}
    for items in causas.values():
        if len(items) < 2:
            continue
        for record_id, rut, programa in items:
            repetido = bool(rut) and sum(1 for _, otro_rut, otro in items if otro_rut == rut and otro == programa) > 1
            salida[record_id] = 'MISMO_PROGRAMA' if repetido else 'OTRO_REGISTRO'
    return salida


def causas_con_varios(registros):
    """registros: iterable de (record_id, tribunal, rit, rut, programa). Para Informes, donde cada fila es un informe:
    {record_id: 'OTRO_REGISTRO'} si la causa tiene más de un niño o programa en la nómina (no más de un informe)."""
    causas = {}
    items = []
    for record_id, tribunal, rit, rut, programa in registros:
        if not clave(rit):
            continue
        causa = (clave(tribunal), clave(rit))
        causas.setdefault(causa, set()).add((rut_clave(rut), clave(programa)))
        items.append((record_id, causa))
    return {record_id: 'OTRO_REGISTRO' for record_id, causa in items if len(causas[causa]) > 1}
