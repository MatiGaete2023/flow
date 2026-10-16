from .contexto import parametro
# -*- coding: utf-8 -*-
from .contexto import datetime as Clock
from .composicion import componer, prefijo, fecha_valida, Incidencias
from .utilidades import fecha_es, titulo_programa, es_derivacion_sin_seg, es_dce
from .textos import render
from . import reglas_historia as H

def generar_observacion_informes(row, tribunal, cols, incidencias=None, fila_excel=None):
    """Informes (decisión x): solo lo que tiene que ver con informes (vencido, por vencer, ingresado, acumulados) y la
    gestión del Centro (correo y proyecto). Curador, oído, audiencia, continuidad, interconexión y otras constancias
    de la causa se hacen constar en Espera o Cumplimiento, según corresponda."""
    if incidencias is None:
        incidencias = Incidencias()
    programa=str(row.get(cols.get('programa'), '')).strip(); nombre=str(row.get(cols.get('nombre'), '')).strip()
    pfx=prefijo(nombre, programa); prog=titulo_programa(programa)
    if es_derivacion_sin_seg(programa): return componer(pfx,[render('COMUN','NO_SEGUIMIENTO', PROGRAMA=prog)])
    fv=fecha_valida(row.get(cols.get('vencimiento'))) if cols.get('vencimiento') else None
    if not fv:
        incidencias.agregar(fila_excel, row.get(cols.get('rit'), '') if cols.get('rit') else '', 'I-01/I-02', 'sin fecha de vencimiento (G-05)'); return ''
    vd=fv.date() if hasattr(fv,'date') else fv; dias=(vd-Clock.now().date()).days
    if dias < 0: key='I01_VENCIDO_DCE' if es_dce(programa) else 'I01_VENCIDO_GENERAL'
    elif 0 <= dias <= parametro('informe'): key='I02_POR_VENCER_DCE' if es_dce(programa) else 'I02_POR_VENCER_GENERAL'
    else: return ''
    finalizada = H.causa_finalizada(row.get(cols.get('rit'), '') if cols.get('rit') else '')
    if finalizada: return componer(pfx, finalizada)
    vencido = dias < 0
    i1 = H.informe_historia(fv, vencido)
    i3 = H.escrito_sin_ingreso(programa, desde=fv if vencido else None)
    if vencido and not (i1 or i3) and H.informe_previo(programa, fv):
        # Un ingreso o escrito anterior al vencimiento no acredita a qué período corresponde: se conservan las acciones.
        H.marcar(2, 'ENTREGA_ANTERIOR', 'Posible entrega anterior al vencimiento: no se acredita su período; se conservan las acciones para revisión')
    if vencido and (i1 or i3):
        H.marcar(2, 'I01_CONTRADICHA', 'Informe vencido en RUS contradicho por la historia')
        principal = render('HISTORIA', 'I01_SIN_ENVIO_DCE' if es_dce(programa) else 'I01_SIN_ENVIO_GENERAL', FECHA_VENCIMIENTO=fecha_es(fv))
    else:
        principal = render('INFORMES', key, PROGRAMA=prog, FECHA_VENCIMIENTO=fecha_es(fv))
    fr = [principal] + i1 + i3 + H.informes_acumulados()
    fr += H.aclaracion_causa()
    return componer(pfx, fr)
