from .contexto import parametro
# -*- coding: utf-8 -*-
from datetime import timedelta
from .contexto import evidencia_informe
from .contexto import datetime as Clock
from .composicion import componer, prefijo, fecha_valida, Incidencias
from .utilidades import fecha_es, titulo_programa, es_derivacion_sin_seg, es_dce
from .textos import render
from . import reglas_historia as H

# Manual CSMP: el diagnóstico DCE (plazo RUS de 1 mes) se pide en cuenta pasados 40 días desde el ingreso, es decir,
# 10 días después del vencimiento RUS. Entretanto se avisa por correo con esa fecha límite (decisión 19-oct-2026).
MARGEN_DCE = 10

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
    fecha=fv
    margen = es_dce(programa) and -MARGEN_DCE <= dias < 0
    if margen:
        key='I02_MARGEN_DCE'; fecha=vd+timedelta(days=MARGEN_DCE)
    elif dias < 0: key='I01_VENCIDO_DCE' if es_dce(programa) else 'I01_VENCIDO_GENERAL'
    elif 0 <= dias <= parametro('informe'): key='I02_POR_VENCER_DCE' if es_dce(programa) else 'I02_POR_VENCER_GENERAL'
    else: return ''
    finalizada = H.causa_finalizada(row.get(cols.get('rit'), '') if cols.get('rit') else '')
    if finalizada: return componer(pfx, finalizada)
    # El vencimiento para contrastar la historia es el de RUS (decisión u); el margen DCE de WA-7 solo cambia la
    # frase y su plazo. Un informe ingresado tras el vencimiento desmiente también al DCE dentro del margen.
    vencido = dias < 0
    # Un trámite de la causa solo retira la gestión si se puede vincular a este informe (11-nov-2026): causa de un
    # registro y un único informe vencido. Si no, se conservan correo y proyecto y el trámite queda como constancia.
    atribuible = not H.varios_registros() and not H.varios_vencidos()
    # Asociación explícita de esta proyección con su evidencia (18-nov-2026): atribuye aunque haya varios vencimientos.
    ev = evidencia_informe()
    folio_res = ev.resolucion_folio if ev else ''
    folio_esc = ev.escrito_historia_folio if ev else ''
    # Escrito de informe de avance pendiente de resolver, atribuible a este informe: no se pide cuenta de él (18-nov-2026).
    # Se evalúa primero: contrasta la bandeja con la historia al mismo corte y deja marca si no hay secuencia fiable.
    pendiente = H.informe_pendiente(programa, fv, atribuible) if vencido else []
    if pendiente:
        H.marcar(2, 'I01_ESCRITO_PENDIENTE', 'Informe vencido en RUS con escrito de informe pendiente de resolver: no se pide cuenta')
        return componer(pfx, pendiente + H.informes_acumulados())
    i1 = H.informe_historia(fv, vencido, atribuible, programa, folio_res)
    i3 = H.escrito_sin_ingreso(programa, desde=fv if vencido else None, folio=folio_esc) if (atribuible or folio_esc) else []
    if vencido and not (atribuible or folio_res or folio_esc) and i1:
        H.marcar(2, 'INFORME_SIN_ATRIBUIR', 'Ingreso de informe en la causa que no se puede atribuir a este informe: se conservan las acciones')
    if vencido and not (i1 or i3) and H.informe_previo(programa, fv):
        # Un ingreso o escrito anterior al vencimiento no acredita a qué período corresponde: se conservan las acciones.
        H.marcar(2, 'ENTREGA_ANTERIOR', 'Posible entrega anterior al vencimiento: no se acredita su período; se conservan las acciones para revisión')
    if vencido and (atribuible or folio_res or folio_esc) and (i1 or i3):
        H.marcar(2, 'I01_CONTRADICHA', 'Informe vencido en RUS contradicho por la historia')
        principal = render('HISTORIA', 'I01_SIN_ENVIO_DCE' if es_dce(programa) else 'I01_SIN_ENVIO_GENERAL', FECHA_VENCIMIENTO=fecha_es(fv))
    elif margen:
        # El vencimiento es el que RUS registra; el margen solo fija el plazo del correo.
        principal = render('INFORMES', key, PROGRAMA=prog, FECHA_VENCIMIENTO=fecha_es(fv), FECHA_LIMITE=fecha_es(fecha))
    else:
        principal = render('INFORMES', key, PROGRAMA=prog, FECHA_VENCIMIENTO=fecha_es(fecha))
    fr = ([principal] if principal else []) + i1 + i3 + H.informes_acumulados()
    return componer(pfx, fr)
