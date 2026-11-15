"""Historia de la causa y bitácora vistas por el motor: solo datos y búsquedas, sin reglas."""

from __future__ import annotations

from dataclasses import dataclass, replace
from datetime import date
import json

from nurus.rus.columns import normalize

PRESENCIA = frozenset(('COMPLETA', 'PARCIAL'))


def clave(texto):
    return ' '.join(normalize(texto).split())


@dataclass(frozen=True)
class Tramite:
    desc: str
    clave: str
    fecha: date
    folio_txt: str
    folio: int
    tipo: str = ''


@dataclass(frozen=True)
class Observacion:
    fecha: date
    texto: str


@dataclass(frozen=True)
class Historia:
    cobertura: str
    desde: date | None
    hasta: date | None
    tramites: tuple

    @property
    def con_datos(self):
        return self.cobertura in PRESENCIA


    def en_ventana(self, tramite):
        """False para el último trámite anterior a la ventana que el paquete conserva solo como contexto."""
        return self.desde is None or tramite.fecha >= self.desde

    def buscar(self, *nombres, contexto=False):
        """Trámites con esos nombres. Los anteriores a la ventana solo se entregan con contexto=True: sirven para
        saber si una situación sigue vigente, nunca para redactar una constancia nueva."""
        claves = {clave(n) for n in nombres}
        return [t for t in self.tramites if t.clave in claves and (contexto or self.en_ventana(t))]

    def al_corte(self, corte):
        """Vista sustantiva: conserva la fuente y nunca acredita ausencia antes de su ventana."""
        if self.desde is not None and corte < self.desde:
            return Historia('NO_CONSULTADA', None, None, ())
        hasta = min(self.hasta, corte) if self.hasta is not None else None
        return replace(self, hasta=hasta, tramites=tuple(t for t in self.tramites if t.fecha <= corte))


def _folio(valor):
    texto = str(valor or '').strip()
    return int(texto) if texto.isdigit() else -1


def dia_iso(valor):
    """Fecha ISO (aaaa-mm-dd…) o None si no se puede leer."""
    try:
        return date.fromisoformat(str(valor or '')[:10])
    except ValueError:
        return None


def ultimo(tramites):
    return max(tramites, key=lambda t: (t.fecha, t.folio)) if tramites else None


def _tipo(fila):
    """Columna «Trámite» de SITFA (Resolución, Escrito, Actuación); viaja en raw_json. '' si no se conoce."""
    try:
        crudo = json.loads(fila.get('raw_json') or '{}')
    except ValueError:
        return ''
    return str(crudo.get('Trámite', '') if isinstance(crudo, dict) else '').strip()


def construir(filas, cobertura, desde=None, hasta=None):
    """filas: dicts de la tabla historia_tramites. Descarta filas sin fecha ISO o sin descripción."""
    tramites = []
    for fila in filas:
        fecha = dia_iso(fila.get('event_at'))
        desc = str(fila.get('description', '')).strip()
        if fecha is None or not desc:
            continue
        folio_txt = str(fila.get('folio', '')).strip()
        tramites.append(Tramite(desc, clave(desc), fecha, folio_txt, _folio(folio_txt), _tipo(fila)))
    tramites.sort(key=lambda t: (t.fecha, t.folio))
    return Historia(cobertura, desde, hasta, tuple(tramites))


def observaciones(filas):
    """filas: dicts de la tabla bitacoras (campos event_at y text), ordenadas por fecha."""
    salida = []
    for fila in filas:
        fecha = dia_iso(fila.get('event_at'))
        if fecha is not None:
            salida.append(Observacion(fecha, str(fila.get('text', ''))))
    return tuple(sorted(salida, key=lambda o: o.fecha))
