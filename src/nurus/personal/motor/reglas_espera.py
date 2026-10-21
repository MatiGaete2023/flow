from .contexto import marcar, parametro
# -*- coding: utf-8 -*-
from .contexto import datetime as Clock, current
from .composicion import componer, prefijo, fecha_valida, Incidencias
from .utilidades import (fecha_es, limpiar_nombre, es_dce, es_derivacion_sin_seg,
    tiene_curador_real, get_int, calcular_edad_exacta, dias_para_mayoria, fecha_mayoria,
    titulo_programa, fechas_varias, lista_fechas_es)
from .textos import render
from . import reglas_historia as H


def _rit(row, cols): return row.get(cols.get('rit'), '') if cols.get('rit') else ''
def _pnombre(nombre):
    n=limpiar_nombre(nombre); return n.split()[0] if n else ''
def _date(v): return v.date() if hasattr(v,'date') else v

def _audiencia(row, cols, incidencias=None, fila_excel=None):
    """T-03 con todas las audiencias vigentes; una fecha ilegible queda como incidencia, nunca en silencio."""
    if not cols.get('prox_aud'):
        return ''
    fechas, ilegibles = fechas_varias(row.get(cols.get('prox_aud')))
    if ilegibles and incidencias is not None:
        incidencias.agregar(fila_excel, _rit(row, cols), 'T-03', 'próxima audiencia con fecha ilegible (G-05)')
    hoy = Clock.now().date()
    futuras = sorted({_date(f) for f in fechas if _date(f) >= hoy})
    if len(futuras) == 1:
        return render('COMUN','PROX_AUDIENCIA', FECHA_AUDIENCIA=fecha_es(futuras[0]))
    if futuras and 'COMUN.PROX_AUDIENCIA' not in current.get()['config']['desactivadas']:
        return render('COMUN','PROX_AUDIENCIAS', FECHAS_AUDIENCIA=lista_fechas_es(futuras))
    return ''

def _oido(row, cols):
    hoy=Clock.now().date()
    fo=fecha_valida(row.get(cols.get('oido'))) if cols.get('oido') else None
    if fo:
        d=(hoy-_date(fo)).days
        if 0 <= d <= parametro('oido') and not H.oido_en_bitacora(fo):
            return [render('COMUN','OIDO', FECHA_OIDO=fecha_es(fo))]
    return []


def _oido_historia(row, cols):
    fo=fecha_valida(row.get(cols.get('oido'))) if cols.get('oido') else None
    return [] if fo else H.derecho_oido(fo)


def _rus_sin_curador(row, cols):
    """True solo si la nómina trae la columna de curador y RUS no registra uno vigente."""
    return bool(cols.get('curador')) and not tiene_curador_real(row.get(cols.get('curador')))


def _curador(row, cols):
    if not cols.get('curador'):
        return []
    celda=row.get(cols.get('curador'))
    if not tiene_curador_real(celda):
        return [H.reitera('CURADOR') or render('COMUN','CURADOR')]
    return H.curador_instituciones(celda, _pnombre(str(row.get(cols.get('nombre'), ''))))


def _complementarias(row, cols, incidencias=None, fila_excel=None):
    out = _oido(row, cols) or _oido_historia(row, cols)
    audiencia = _audiencia(row, cols, incidencias, fila_excel)
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
        marcar(3, 'REVISAR_MAYORIA', 'Mayoría de edad')
        return componer(pfx, frags)
    finalizada=H.causa_finalizada(_rit(row, cols))
    if finalizada: return componer(pfx, finalizada)
    dm=dias_para_mayoria(fn) if fn else None
    if dm is not None and 1 <= dm <= parametro('mayoria'):
        frags.append(render('COMUN','PROXIMA_MAYORIA', PNOMBRE=pn, FECHA_MAYORIA=fecha_es(fecha_mayoria(fn))))
    principal=False
    H.marcar_duplicidad()
    espera=get_int(row.get(cols.get('espera'))) if cols.get('espera') else None
    # «Ingreso efectivo» es de Cumplimiento: si la historia lo trae con RUS en Espera, reglas normales y solo marca.
    H.ingreso_efectivo_en_espera(hoy, espera)
    H.marcas_sin_efecto()
    fres=fecha_valida(row.get(cols.get('resolucion'))) if cols.get('resolucion') else None
    e04=False
    if fres:
        d=(hoy-_date(fres)).days
        if 0 <= d < parametro('resolucion_reciente'):
            folio=H.folio_resolucion(fres)
            if folio: frags.append(render('HISTORIA','E04_CON_FOLIO', PROGRAMA=titulo_programa(programa), FECHA_RESOLUCION=fecha_es(fres), FOLIO=folio))
            else: frags.append(render('ESPERA','E04_RESOLUCION_RECIENTE', PROGRAMA=titulo_programa(programa), FECHA_RESOLUCION=fecha_es(fres)))
            principal=e04=True
    if cols.get('espera') and (espera is None or espera < 0):
        incidencias.agregar(fila_excel, _rit(row, cols), 'E-05/E-06',
                            'tiempo de espera ausente o inválido (G-05)')
    # «Registra N días en lista de espera» (residencial, más de 100) dice lo mismo que los días de espera: se dice una vez.
    larga=H.lista_larga(programa, espera)
    threshold = (parametro('espera_dce') if es_dce(programa) else
                 parametro('espera_' + tribunal.lower()) if tribunal in ('LAJA','MULCHEN','TOME') else 30)
    if espera is not None and espera >= threshold:
        if es_dce(programa):
            frags.append(render('ESPERA','E05_SOLO_CORREO')); principal=True
            if not larga: frags.append(render('HISTORIA','E05_DIAS', CORTE=fecha_es(hoy), N=espera))
        elif tribunal in ('LAJA','MULCHEN'):
            frags.append(render('ESPERA','E05_PROYECTO_Y_CORREO')); principal=True
            if not larga: frags.append(render('HISTORIA','E05_DIAS', CORTE=fecha_es(hoy), N=espera))
        elif tribunal == 'TOME':
            frags.append(render('ESPERA','E05_PROYECTO_Y_CORREO' if espera >= parametro('proyecto_tome') else 'E05_SOLO_CORREO')); principal=True
            if not larga: frags.append(render('HISTORIA','E05_DIAS', CORTE=fecha_es(hoy), N=espera))
        else:
            incidencias.agregar(fila_excel, _rit(row, cols), 'E-05', 'tribunal no reconocido — regla omitida (G-06)')
    # E-06 es el fallback del §9.1: dispara siempre que no haya principal
    # (incluye T ESPERA ausente/ inválido y tribunal no reconocido — D11).
    if not principal:
        frags.append(render('ESPERA','E06_SIN_RESOLUCION', PROGRAMA=titulo_programa(programa)))
    frags += H.lista_espera_larga(programa, espera)
    # Con E-04 la resolución de ingreso ya se dijo: el trámite «Ing.Serv.» no la repite (un dato, una vez).
    frags += ([] if e04 else H.servicio(programa)) + H.busqueda_historia()
    frags += H.continuidad() + H.curador_designaciones(_rus_sin_curador(row, cols))
    frags += _complementarias(row, cols, incidencias, fila_excel)
    frags += H.aclaracion_causa()
    return componer(pfx, frags)
