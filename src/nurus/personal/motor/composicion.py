# -*- coding: utf-8 -*-
"""Composición segura de observaciones e incidencias de validación."""
from datetime import datetime
import re
import pandas as pd
from .textos import ORDEN_GRUPOS
from .utilidades import prefijo_observacion, get_date

def prefijo(nombre, programa) -> str:
    return prefijo_observacion(nombre, programa)

def _normalizar_fragmento(fragmento):
    if fragmento is None:
        return ""
    texto = str(fragmento).strip()
    if not texto:
        return ""
    texto = re.sub(r"\s+", " ", texto)
    texto = texto.rstrip(". ") + "."
    texto = re.sub(r"\.{2,}", ".", texto)
    return texto

def _en_orden(fragmentos):
    """Expande los fragmentos mixtos y ordena por grupo (constancia, acción del Centro, Tribunal, cierre)."""
    piezas = []
    dichos = set()
    for fragmento in fragmentos:
        dato = getattr(fragmento, "dato", "")
        if dato:
            if dato in dichos:
                continue
            dichos.add(dato)
        for texto, grupo in getattr(fragmento, "partes", None) or ((fragmento, getattr(fragmento, "grupo", "C")),):
            piezas.append((ORDEN_GRUPOS[grupo], len(piezas), texto))
    return _enlazar([texto for _, _, texto in sorted(piezas, key=lambda pieza: pieza[:2])])

_PRESENTE = "Se hace presente que "

def _enlazar(textos):
    """Pulido al aplicarse varias reglas: desde la segunda constancia «Se hace presente que…» pasa a «Asimismo, se hace presente que…»."""
    salida, vistas = [], 0
    for texto in textos:
        if str(texto).startswith(_PRESENTE):
            vistas += 1
            if vistas > 1:
                texto = "Asimismo, se hace presente que " + str(texto)[len(_PRESENTE):]
        salida.append(texto)
    return salida

def componer(pfx, fragmentos) -> str:
    partes = [_normalizar_fragmento(f) for f in _en_orden(fragmentos)]
    partes = [p for p in partes if p]
    if not partes:
        return ""
    texto = " ".join(partes)
    texto = re.sub(r"\s+", " ", texto).replace("..", ".")
    return f"{pfx or ''}{texto}"

class Incidencias:
    def __init__(self):
        self._items = []
    def agregar(self, fila_excel, rit, regla, motivo):
        self._items.append({"FILA_EXCEL": fila_excel, "RIT": rit or "", "REGLA": regla, "MOTIVO": motivo})
    def __len__(self):
        return len(self._items)

def fecha_valida(valor):
    fec = get_date(valor)
    if fec is None:
        return None
    return fec if isinstance(fec, datetime) else pd.to_datetime(fec).to_pydatetime()
