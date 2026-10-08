from .contexto import parametro
# -*- coding: utf-8 -*-
from datetime import datetime
from .contexto import datetime as Clock
from .composicion import componer, prefijo, fecha_valida, Incidencias
from .utilidades import (fecha_es, limpiar_nombre, es_dce, es_derivacion_sin_seg,
    tiene_curador_real, get_int, calcular_edad_exacta, dias_para_mayoria, fecha_mayoria,
    titulo_programa)
from .textos import render


def _rit(row, cols): return row.get(cols.get('rit'), '') if cols.get('rit') else ''
def _pnombre(nombre):
    n=limpiar_nombre(nombre); return n.split()[0] if n else ''
def _date(v): return v.date() if hasattr(v,'date') else v

def _audiencia(row, cols):
    """Retorna únicamente T-03 cuando existe una audiencia vigente."""
    fa=fecha_valida(row.get(cols.get('prox_aud'))) if cols.get('prox_aud') else None
    if fa and _date(fa) >= Clock.now().date():
        return render('COMUN','PROX_AUDIENCIA', FECHA_AUDIENCIA=fecha_es(fa))
    return ''

def _oido(row, cols):
    hoy=Clock.now().date()
    fo=fecha_valida(row.get(cols.get('oido'))) if cols.get('oido') else None
    if fo:
        d=(hoy-_date(fo)).days
        if 0 <= d <= parametro('oido'):
            return [render('COMUN','OIDO', FECHA_OIDO=fecha_es(fo))]
    return []


def _curador(row, cols):
    if cols.get('curador') and not tiene_curador_real(row.get(cols.get('curador'))):
        return [render('COMUN','CURADOR')]
    return []


def _curador_oido(row, cols):
    """Compatibilidad interna: hitos antes de sugerencias."""
    return _oido(row, cols) + _curador(row, cols)


def _complementarias(row, cols):
    out = _oido(row, cols)
    audiencia = _audiencia(row, cols)
    if audiencia:
        out.append(audiencia)
    out += _curador(row, cols)
    return out


def generar_observacion_espera(row, tribunal, cols, incidencias=None, fila_excel=None) -> str:
    if incidencias is None:
        incidencias = Incidencias()
    programa=str(row.get(cols.get('programa'), '')).strip(); nombre=str(row.get(cols.get('nombre'), '')).strip()
    pfx=prefijo(nombre, programa); pn=_pnombre(nombre); hoy=Clock.now().date()
    if es_derivacion_sin_seg(programa):
        return componer(pfx,[render('COMUN','NO_SEGUIMIENTO', PROGRAMA=titulo_programa(programa))])
    frags=[]
    fn=fecha_valida(row.get(cols.get('nacimiento'))) if cols.get('nacimiento') else None
    if fn and (calcular_edad_exacta(fn) or 0) >= 18:
        frags.append(render('COMUN','MAYORIA_EDAD', PNOMBRE=pn, FECHA_MAYORIA=fecha_es(fecha_mayoria(fn))))
        frags += _complementarias(row, cols)
        return componer(pfx, frags)
    dm=dias_para_mayoria(fn) if fn else None
    if dm is not None and 1 <= dm <= parametro('mayoria'):
        frags.append(render('COMUN','PROXIMA_MAYORIA', PNOMBRE=pn, FECHA_MAYORIA=fecha_es(fecha_mayoria(fn))))
    principal=False
    fres=fecha_valida(row.get(cols.get('resolucion'))) if cols.get('resolucion') else None
    if fres:
        d=(hoy-_date(fres)).days
        if 0 <= d < parametro('resolucion_reciente'):
            frags.append(render('ESPERA','E04_RESOLUCION_RECIENTE', PROGRAMA=titulo_programa(programa), FECHA_RESOLUCION=fecha_es(fres))); principal=True
    espera=get_int(row.get(cols.get('espera'))) if cols.get('espera') else None
    if cols.get('espera') and (espera is None or espera < 0):
        incidencias.agregar(fila_excel, _rit(row, cols), 'E-05/E-06',
                            'tiempo de espera ausente o inválido (G-05)')
    threshold = (parametro('espera_dce') if es_dce(programa) else
                 parametro('espera_' + tribunal.lower()) if tribunal in ('LAJA','MULCHEN','TOME') else 30)
    if espera is not None and espera >= threshold:
        if es_dce(programa):
            frags.append(render('ESPERA','E05_SOLO_CORREO')); principal=True
        elif tribunal in ('LAJA','MULCHEN'):
            frags.append(render('ESPERA','E05_PROYECTO_Y_CORREO')); principal=True
        elif tribunal == 'TOME':
            frags.append(render('ESPERA','E05_PROYECTO_Y_CORREO' if espera >= parametro('proyecto_tome') else 'E05_SOLO_CORREO')); principal=True
        else:
            incidencias.agregar(fila_excel, _rit(row, cols), 'E-05', 'tribunal no reconocido — regla omitida (G-06)')
    # E-06 es el fallback del §9.1: dispara siempre que no haya principal
    # (incluye T ESPERA ausente/ inválido y tribunal no reconocido — D11).
    if not principal:
        frags.append(render('ESPERA','E06_SIN_RESOLUCION', PROGRAMA=titulo_programa(programa)))
    frags += _complementarias(row, cols)
    return componer(pfx, frags)
