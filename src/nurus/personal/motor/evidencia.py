"""Evidencia nueva de la integración 18-nov-2026: Escritos de la causa, Ingreso Histórico Protección y asociación de
cada informe con su proyección. Solo datos y búsquedas puras, sin reglas ni textos.

Estados de fuente (mismos del contrato de captura): COMPLETA, VACIA_COMPROBADA, PARCIAL, FALLIDA, NO_SOLICITADA,
NO_CONSULTADA e IDENTIDAD_INCIERTA. Solo hay filas utilizables con COMPLETA o PARCIAL; VACIA_COMPROBADA acredita una
bandeja vacía al instante de la lectura; los demás estados no acreditan nada.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from datetime import date

from nurus.rus.columns import normalize

ESTADOS_FUENTE = (
    'COMPLETA', 'VACIA_COMPROBADA', 'PARCIAL', 'FALLIDA', 'NO_SOLICITADA', 'NO_CONSULTADA', 'IDENTIDAD_INCIERTA',
)  # fmt: skip
CON_FILAS = frozenset(('COMPLETA', 'PARCIAL'))

ESTADO_PENDIENTE = 'pendiente'
# Histórico de Protección: ambos indicadores sin egreso expresan vigencia (confirmado por el usuario, 18-nov-2026).
CAUSAL_VIGENTE = 'sin informacion'


def clave_texto(texto):
    """Comparación estable: sin acentos, minúsculas, espacios colapsados (también NBSP y Unicode equivalente)."""
    return ' '.join(normalize(str(texto or '').replace(' ', ' ')).split())


_GUIONES = '-\u2010\u2011\u2012\u2013\u2014\u2212'


def _es_sin_fecha(texto):
    """«--» tal como lo muestra SITFA (también con guiones Unicode equivalentes). Un guion solo o vacío no basta."""
    limpio = str(texto or '').replace('\u00a0', ' ').strip()
    return len(limpio) == 2 and all(c in _GUIONES for c in limpio)


@dataclass(frozen=True)
class Escrito:
    """Fila de la tabla Escritos de la causa. Los valores salen de la misma fila, sin importar el orden de columnas."""

    tipo: str
    estado: str
    doc: str = ''
    solicitante: str = ''
    fila: int = 0
    # Asociación explícita con una proyección concreta (referencia documental o vínculo del calendario):
    # 'EXPLICITO' = de este ingreso; 'AJENO' = asociado expresamente a OTRO ingreso (nunca es candidato aquí); '' = sin vínculo.
    vinculo: str = ''
    fecha_proyectada: date | None = None
    periodo: str = ''


@dataclass(frozen=True)
class Escritos:
    cobertura: str
    leido: date | None
    filas: tuple = ()

    @property
    def con_filas(self):
        return self.cobertura in CON_FILAS


@dataclass(frozen=True)
class ProgramaHistorico:
    """Fila del Ingreso Histórico Protección de la persona."""

    programa: str
    fecha_egreso: str = ''
    causal_egreso: str = ''
    fila: int = 0

    @property
    def vigente(self):
        """True solo si la fecha es «--» y la causal «SIN INFORMACION» en la misma fila. Un dato ilegible o vacío no
        acredita vigencia."""
        return _es_sin_fecha(self.fecha_egreso) and clave_texto(self.causal_egreso) == CAUSAL_VIGENTE


@dataclass(frozen=True)
class Historico:
    cobertura: str
    leido: date | None = None
    # VERIFICADA solo si se probó que la tabla visible es la de la persona de la fila seleccionada.
    identidad: str = 'VERIFICADA'
    filas: tuple = ()

    @property
    def utilizable(self):
        return self.cobertura in CON_FILAS and self.identidad == 'VERIFICADA'


@dataclass(frozen=True)
class EvidenciaInforme:
    """Asociación explícita de un informe esperado (ingreso + fecha proyectada) con sus evidencias, al mismo corte."""

    escrito_historia_folio: str = ''
    resolucion_folio: str = ''
    resolucion_fecha: date | None = None
    calendario: str = ''  # 'PENDIENTE', 'ENVIADO' o ''
    fecha_proyectada: date | None = None
    # Folios de Historia asociados expresamente a OTRO informe de la causa: no se atribuyen a este por la vía genérica.
    folios_ajenos: tuple = ()


def clave_programa(nombre):
    """Identidad del programa por su denominación completa: dos PRM distintos son programas distintos."""
    return clave_texto(nombre)


def programas_vigentes_otros(historico, programa_actual):
    """Nombres (tal como se muestran) de los otros programas vigentes del mismo niño, sin repetir y sin el actual.

    Un programa con una fila antigua egresada y otra vigente cuenta como vigente; las filas vigentes repetidas del mismo
    nombre se colapsan. Devuelve [] si el histórico no es utilizable."""
    if historico is None or not historico.utilizable:
        return []
    actual = clave_programa(programa_actual)
    vistos = {}
    for fila in historico.filas:
        llave = clave_programa(fila.programa)
        if not llave or llave == actual or not fila.vigente:
            continue
        vistos.setdefault(llave, ' '.join(str(fila.programa).split()))
    return list(vistos.values())


_NEGACION = re.compile(r'\b(?:no|nunca|ni|sin)\b[^.]{0,40}$')
_CONSTANCIA = re.compile(
    r'(?:cuenta\s+con\s+)?ingres(?:o\s+efectivo|[oó]\s+efectivamente)\s+(?:al|a)\s+(?:programa\s+)?(.+?)\s*(?:[.;:\n]|$)',
    re.IGNORECASE,
)


def _sin_aplanar(texto):
    return ' '.join(str(texto or '').replace(' ', ' ').split())


def constancia_en_bitacora(observaciones, primer_nombre, programa):
    """Observación registrada que ya hizo constar que ese niño cuenta con ingreso efectivo a ese programa.

    La coincidencia es del nombre completo del programa (no por subcadena) y exige que el SUJETO de la constancia sea el
    niño: su primer nombre es la palabra inmediatamente anterior a «cuenta con ingreso…»/«ingresó efectivamente…»
    («Ana informa que Beto cuenta con…» no es constancia de Ana). Una negación o la mera mención no bastan."""
    objetivo = clave_programa(programa)
    nombre = clave_texto(primer_nombre)
    if not objetivo:
        return None
    for obs in observaciones:
        for oracion in re.split(r'(?<=\.)\s+', _sin_aplanar(obs.texto)):
            for encontrado in _CONSTANCIA.finditer(oracion):
                if clave_programa(encontrado.group(1)) != objetivo:
                    continue
                previo = clave_texto(oracion[: encontrado.start()])
                if _NEGACION.search(previo):
                    continue
                if not nombre or previo.split()[-1:] != [nombre]:
                    continue
                return obs
    return None
