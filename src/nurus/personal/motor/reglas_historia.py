"""Constancias de historia de la causa: cada función devuelve fragmentos de texto ([] si no aplica)."""

import re
from datetime import timedelta

from . import vocabulario as V
from .analisis_historia import busqueda, duplicidades, instituciones, posterior, primera_observacion  # noqa: F401
from .contexto import acumulado, bitacora, current, duplicidad, emitidos, evento, historia, marcar
from .contexto import datetime as Clock
from .historia import clave, ultimo
from .textos import con_grupo, render
from .utilidades import contiene_token, es_dce, fecha_es, fecha_mayoria, normalizar, titulo_programa

ORIGINAL = {
    'CURADOR': ('COMUN.CURADOR', 'MATERIA_CURADOR'),
    'SIN_FICHA': ('CUMPLIMIENTO.C07_SIN_FICHA', 'MATERIA_SIN_FICHA'),
    'SIN_FAE': ('CUMPLIMIENTO.C08_FICHA_FAE', 'MATERIA_SIN_FAE'),
    'VENCIDA': ('CUMPLIMIENTO.C04_VENCIDA', 'MATERIA_VENCIDA'),
}


# Constancias que citan trámites de la historia de la causa; con ellas, una causa de varios registros lleva
# la aclaración final de que la historia no se atribuye a un ingreso determinado (decisión x).
CITAN = frozenset(
    'HISTORIA.' + clave_texto
    for clave_texto in (
        'BASE_TRAMITE',
        'VARIOS',
        'E04_CON_FOLIO',
        'C2_PRORROGA_HASTA',
        'C2_PRORROGA_SIN_FECHA',
        'SERVICIO_INGRESO',
        'CURADOR_SIN_EFECTO',
        'CURADOR_DESIGNADO',
        'R11_BUSQUEDA_SIN_EFECTO',
        'R11_BUSQUEDA_TERMINO',
        'T2_MN',
        'C9_DCE',
        'C10_ESCRITO',
        'C4_FOLIO',
    )
)


def _h():
    h = historia()
    return h if h is not None and h.con_datos else None


def _id(t):
    return (t.clave, t.fecha, t.folio_txt)


def _folio(t):
    return t.folio_txt or 's/f'


def _d(valor):
    return valor.date() if hasattr(valor, 'date') else valor


def intervencion(programa):
    if es_dce(programa):
        return 'DCE'
    if normalizar(programa).startswith(V.RESIDENCIA_PREFIJOS):
        return 'RESIDENCIAL'
    if contiene_token(programa, 'fae', 'fas'):
        return 'FAE'
    return 'AMBULATORIO'


def tramites(*nombres):
    h = _h()
    return h.buscar(*nombres) if h else []


def constancia(lista):
    """Texto base de uno o varios trámites del mismo nombre; omite los ya citados ('' si no queda ninguno)."""
    lista = [t for t in lista if _id(t) not in emitidos()]
    if not lista:
        return ''
    emitidos().update(_id(t) for t in lista)
    ult = ultimo(lista)
    if len(lista) == 1:
        return render('HISTORIA', 'BASE_TRAMITE', DESC=ult.desc, FECHA=fecha_es(ult.fecha), FOLIO=_folio(ult))
    return render(
        'HISTORIA', 'VARIOS', N=len(lista), DESC=ult.desc, FECHA=fecha_es(ult.fecha), FOLIO=_folio(ult)
    )


def constancias(*nombres):
    """Una constancia por cada nombre presente, en el orden dado."""
    salida = []
    for nombre in nombres:
        texto = constancia(tramites(nombre))
        if texto:
            salida.append(texto)
    return salida


def folio_resolucion(fecha):
    """Regla 1: folio del trámite «Ing.Serv. …» (resolución de ingreso) fechado el día de la resolución de RUS.
    None si no hay o si hay folios distintos ese día. El trámite queda citado."""
    h = _h()
    if h is None or fecha is None:
        return None
    ingresos = [t for t in h.buscar(V.SERVICIO) if t.fecha == _d(fecha) and t.folio_txt]
    if len({t.folio_txt for t in ingresos}) != 1:
        return None
    emitidos().update(_id(t) for t in ingresos)
    return ingresos[0].folio_txt


def ingreso_efectivo_espera(corte, espera=None):
    """Regla 3. Devuelve los fragmentos que reemplazan E-04/E-05/E-06, o None.
    Con T ESPERA válido solo cuenta el ingreso efectivo desde el inicio de la espera (corte - días); un ingreso
    anterior es de otra medida y no desplaza E-05 ni sus acciones (decisión u, 14-oct). Sin T ESPERA no se filtra."""
    lista = tramites(V.INGRESO_EFECTIVO)
    if espera is not None and espera >= 0:
        inicio = _d(corte) - timedelta(days=espera)
        lista = [t for t in lista if t.fecha >= inicio]
    if not lista:
        return None
    marcar(2, 'R3_INGRESO_EFECTIVO', 'Historia con ingreso efectivo y RUS en Espera')
    frags = [render('HISTORIA', 'MEDIDA_REVISADA'), constancia(lista), render('HISTORIA', 'R3_CIERRE', CORTE=fecha_es(corte))]
    return [f for f in frags if f]


def lista_larga(programa, espera):
    return espera is not None and intervencion(programa) == 'RESIDENCIAL' and espera > V.E10_DIAS


def lista_espera_larga(programa, espera):
    if lista_larga(programa, espera):
        return [render('HISTORIA', 'E10_LISTA', N=espera)]
    return []


def marcar_duplicidad():
    if duplicidad() == 'MISMO_PROGRAMA':
        marcar(1, 'R6_DUPLICIDAD', 'Más de un ingreso del mismo NNA al mismo programa')


def reitera(materia):
    """REITERA: '' si no hay observación previa de la materia o la regla original está desactivada."""
    original, clave_materia = ORIGINAL[materia]
    ctx = current.get()
    if ctx is None or original in ctx['config']['desactivadas']:
        return ''
    previa = primera_observacion(bitacora(), materia, Clock.now().date())
    if previa is None:
        return ''
    texto = render(
        'HISTORIA', 'REITERA', FECHA_ANTERIOR=fecha_es(previa.fecha), MATERIA=render('HISTORIA', clave_materia)
    )
    if texto:
        evento(original)
    # Reiterar la ausencia de curador o de ficha es gestión del Tribunal; reiterar el vencimiento es constancia.
    return con_grupo(str(texto), 'C' if materia == 'VENCIDA' else 'T')


def curador_designaciones(rus_sin_curador):
    """Última designación o baja de curador ad litem de la historia, solo si RUS no registra curador.
    Con curador en RUS no se puede verificar la vigencia desde la historia y se ignora (decisión x)."""
    if not rus_sin_curador:
        return []
    designa = ultimo(tramites(V.CURADOR[0]))
    baja = ultimo(tramites(V.CURADOR[1]))
    if designa is None and baja is None:
        return []
    ult = baja if designa is None else designa if baja is None or posterior(designa, baja) else baja
    if _id(ult) in emitidos():
        return []
    emitidos().add(_id(ult))
    llave = 'CURADOR_SIN_EFECTO' if ult is baja else 'CURADOR_DESIGNADO'
    marcar(2, llave, 'Curador ad litem en la historia y RUS sin curador registrado')
    return [render('HISTORIA', llave, FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha))]


def curador_instituciones(celda, pnombre):
    cantidad = len(instituciones(celda))
    if cantidad < 2:
        return []
    marcar(1, 'T5_INSTITUCIONES', 'Curador con varias instituciones en RUS')
    return [render('HISTORIA', 'T5_INSTITUCIONES', N=cantidad, PNOMBRE=pnombre)]


def dce_espera(programa):
    return constancias(*V.DCE) if intervencion(programa) == 'DCE' else []


def direccion_regional():
    return constancias(*V.DIRECCION_REGIONAL)


def servicio(programa):
    """Espera: resolución que dispone el ingreso al programa («Ing.Serv. …»); el programa es el de la fila de RUS."""
    ult = ultimo([t for t in tramites(V.SERVICIO) if _id(t) not in emitidos()])
    if ult is None:
        return []
    emitidos().add(_id(ult))
    return [render('HISTORIA', 'SERVICIO_INGRESO', PROGRAMA=titulo_programa(programa), FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha))]


def continuidad():
    return constancias(V.CONTINUIDAD)


def deja_sin_efecto_mn():
    """T2: «Deja sin efecto M.N.» posterior a un ingreso o prórroga; cita ambos trámites."""
    salida = []
    anteriores = tramites(*V.ANTERIORES_MN)
    for mn in sorted(tramites(V.SIN_EFECTO_MN), key=lambda t: (t.fecha, t.folio)):
        previos = [a for a in anteriores if posterior(mn, a)]
        if not previos or _id(mn) in emitidos():
            continue
        ant = max(previos, key=lambda a: (a.fecha, a.folio))
        emitidos().update((_id(mn), _id(ant)))
        marcar(2, 'T2_MN', 'Deja sin efecto M.N. posterior a un ingreso o prórroga')
        salida.append(
            render(
                'HISTORIA', 'T2_MN', FECHA=fecha_es(mn.fecha), FOLIO=_folio(mn), DESC_ANTERIOR=ant.desc,
                FECHA_ANTERIOR=fecha_es(ant.fecha), FOLIO_ANTERIOR=_folio(ant),
            )
        )
    return [s for s in salida if s]


def busqueda_historia():
    """R11: constancias de búsqueda; marca nivel 1 si hay una orden vigente."""
    h = _h()
    if h is None:
        return []
    pares, vigentes = busqueda(h)
    if vigentes:
        marcar(1, 'R11_BUSQUEDA_VIGENTE', 'Orden de búsqueda vigente')
    salida = []
    for cierre, orden in pares:
        if _id(cierre) in emitidos():
            continue
        if orden is None:
            texto = constancia([cierre])
        else:
            emitidos().update((_id(cierre), _id(orden)))
            llave = 'R11_BUSQUEDA_TERMINO' if cierre.clave == clave(V.TERMINO_BUSQUEDA) else 'R11_BUSQUEDA_SIN_EFECTO'
            texto = render(
                'HISTORIA', llave, FECHA=fecha_es(cierre.fecha), FOLIO=_folio(cierre),
                FECHA_ORDEN=fecha_es(orden.fecha), FOLIO_ORDEN=_folio(orden),
            )
        if texto:
            salida.append(texto)
    salida += constancias(*V.BUSQUEDA_CITADA)
    return salida


def derecho_oido(fecha_rus):
    """C3: solo si RUS no tiene fecha de oído; usa la última aplicación de la historia."""
    lista = tramites(V.DERECHO_OIDO)
    if fecha_rus or not lista or oido_en_bitacora(ultimo(lista).fecha):
        return []
    return [render('COMUN', 'OIDO', FECHA_OIDO=fecha_es(ultimo(lista).fecha))]


def oido_en_bitacora(fecha):
    """True si una observación anterior de la bitácora ya dejó constancia del oído en esa fecha."""
    fecha = _d(fecha)
    formas = (clave(fecha_es(fecha)), fecha.strftime('%d/%m/%Y'))
    for obs in bitacora():
        texto = clave(obs.texto)
        if 'oido' in texto and any(forma in texto for forma in formas):
            return True
    return False


def ingreso_verificado(fecha_ingreso):
    """«Ingreso Efectivo» de la historia fechado entre la fecha de RUS y 30 días después: la nomenclatura se aplicó.
    La diferencia de días es normal (RUS guarda el día real; la historia, la resolución que lo tiene presente)."""
    h = _h()
    if h is None or fecha_ingreso is None:
        return None
    inicio = _d(fecha_ingreso)
    limite = inicio + timedelta(days=V.VENTANA_INGRESO_DIAS)
    cercanos = [t for t in h.buscar(V.INGRESO_EFECTIVO) if inicio <= t.fecha <= limite]
    return min(cercanos, key=lambda t: (t.fecha, t.folio)) if cercanos else None


def ingreso_informado(fecha_ingreso):
    """Cumplimiento sin ingreso reciente: si la historia confirma el ingreso, se dice la fecha de RUS (sin contraste)."""
    if ingreso_verificado(fecha_ingreso) is None:
        return []
    return [render('HISTORIA', 'INGRESO_INFORMADO', FECHA_INGRESO=fecha_es(fecha_ingreso))]


def _prorroga_en_bitacora(prorroga):
    """True si la bitácora ya menciona esa prórroga (su folio o su fecha): se revisó en una observación anterior."""
    folio = re.escape(prorroga.folio_txt) if prorroga.folio_txt else None
    fecha = clave(fecha_es(prorroga.fecha))
    for obs in bitacora():
        texto = clave(obs.texto)
        if 'prorrog' in texto and ((folio and re.search(rf'(?<!\d){folio}(?!\d)', texto)) or fecha in texto):
            return True
    return False


def prorroga(corte, egreso_proyectado, vencida, por_vencer, egreso_dicho):
    """C2 (decisión x). Con la medida vencida, o con el egreso de RUS anterior a la prórroga (RUS no la refleja), no
    se afirma la prórroga: solo se marca para revisión prioritaria si es reciente (90 días). Con la medida por vencer
    y una prórroga reciente se afirma y se marca. Si la bitácora ya la mencionó, se ignora todo."""
    lista = [t for t in tramites(V.PRORROGA) if _id(t) not in emitidos()]
    if not lista or egreso_proyectado is None:
        return []
    ult = ultimo(lista)
    if _prorroga_en_bitacora(ult):
        return []
    emitidos().update(_id(t) for t in lista)
    reciente = (_d(corte) - ult.fecha).days <= V.PRORROGA_RECIENTE_DIAS
    sin_reflejar = _d(egreso_proyectado) < ult.fecha
    if vencida or sin_reflejar or (por_vencer and reciente):
        if reciente:
            marcar(1, 'PRORROGA_REVISAR', 'Prórroga reciente con medida vencida o por vencer: verificar su registro en RUS')
        if vencida or sin_reflejar:
            return []
    if egreso_dicho:
        return [render('HISTORIA', 'C2_PRORROGA_SIN_FECHA', FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha))]
    return [
        render(
            'HISTORIA', 'C2_PRORROGA_HASTA', FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha),
            FECHA_EGRESO_PROYECTADO=fecha_es(egreso_proyectado),
        )
    ]


def fichas(nombre, fecha_rus, original_salio):
    """C4: ficha de la historia contra la de RUS. Ficha Ambulatoria queda fuera."""
    ult = ultimo(tramites(nombre))
    if ult is None:
        return []
    if original_salio and fecha_rus is not None and _d(fecha_rus) == ult.fecha:
        if _id(ult) in emitidos():
            return []
        emitidos().add(_id(ult))
        return [render('HISTORIA', 'C4_FOLIO', FOLIO=_folio(ult))]
    texto = constancia([ult])
    if not texto:
        return []
    if fecha_rus is not None:
        return [texto, render('HISTORIA', 'C4_FICHA_RUS', FECHA_FICHA=fecha_es(fecha_rus))]
    if original_salio:
        return [texto]  # la regla original ya dijo que RUS no registra la ficha: no se repite (decisión x)
    return [texto, render('HISTORIA', 'C4_FICHA_SIN_RUS')]


def no_uso_prorroga(egreso_proyectado, vencimiento_dicho=False):
    """C8 con guardia G3: exige cobertura que permita ausencia y período consultado."""
    h = historia()
    if h is None or not h.permite_ausencia or h.desde is None or h.hasta is None or egreso_proyectado is None:
        return []
    if h.buscar(V.PRORROGA):
        return []
    marcar(3, 'C8_NO_USO', 'Medida vencida sin Prórroga en el período consultado')
    if vencimiento_dicho:
        return [render('HISTORIA', 'C8_NO_USO_BREVE', DESDE=fecha_es(h.desde), HASTA=fecha_es(h.hasta))]
    return [
        render(
            'HISTORIA', 'C8_NO_USO', DESDE=fecha_es(h.desde), HASTA=fecha_es(h.hasta),
            FECHA=fecha_es(egreso_proyectado),
        )
    ]


def diagnostico_dce(programa, vencida):
    """C9: informe diagnóstico ordenado en autos con la medida vencida."""
    if not vencida or intervencion(programa) != 'DCE':
        return []
    lista = [t for t in tramites(V.DIAGNOSTICO) if _id(t) not in emitidos()]
    if not lista:
        return []
    emitidos().update(_id(t) for t in lista)
    ult = ultimo(lista)
    return [render('HISTORIA', 'C9_DCE', FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha))]


def escrito_sin_ingreso(programa, desde=None):
    """C10/I3: último escrito de informe sin «Ingreso informe de cumplimiento X» posterior (por folio)."""
    nombre = V.ESCRITOS.get(intervencion(programa))
    candidatos = tramites(nombre) if nombre else []
    if desde is not None:
        candidatos = [t for t in candidatos if t.fecha >= _d(desde)]
    escrito = ultimo(candidatos)
    if escrito is None or _id(escrito) in emitidos():
        return []
    if any(posterior(i, escrito) for i in tramites(V.INGRESO_INFORME_X)):
        return []
    emitidos().add(_id(escrito))
    return [render('HISTORIA', 'C10_ESCRITO', FOLIO_ESCRITO=_folio(escrito), FECHA_ESCRITO=fecha_es(escrito.fecha))]


def informe_historia(fecha_vencimiento, vencido):
    """I1: «Ingreso informe de cumplimiento X» y estado que RUS registra. [] si no aplica.
    Con el informe vencido solo cuenta un ingreso del día del vencimiento o posterior: uno anterior es de otro
    período y no desmiente el vencimiento (decisión u, 14-oct)."""
    lista = tramites(V.INGRESO_INFORME_X)
    if vencido:
        lista = [t for t in lista if t.fecha >= _d(fecha_vencimiento)]
    texto = constancia(lista)
    if not texto:
        return []
    llave = 'I1_VENCIDO' if vencido else 'I1_POR_VENCER'
    return [texto, render('HISTORIA', llave, FECHA_VENCIMIENTO=fecha_es(fecha_vencimiento))]


def informe_previo(programa, fecha_vencimiento):
    """Presencia anterior al vencimiento, sin inferir a qué período corresponde."""
    nombres = [V.INGRESO_INFORME_X]
    escrito = V.ESCRITOS.get(intervencion(programa))
    if escrito:
        nombres.append(escrito)
    return any(t.fecha < _d(fecha_vencimiento) for t in tramites(*nombres))


def prorroga_informe():
    """I2: la Prórroga queda solo como constancia; nunca desmiente el informe vencido (decisión u, 14-oct)."""
    return constancias(V.PRORROGA)


def informes_acumulados():
    """J2: informes vencidos del mismo ingreso según el pre-pase de Work.analyze."""
    dato = acumulado()
    if not dato or dato[0] < 2:
        return []
    return [render('HISTORIA', 'J2_ACUMULADOS', N=dato[0], FECHA=fecha_es(dato[1]))]


def egreso_posterior_mayoria(nacimiento, egreso, mayoria_dicha):
    """Cumplimiento: el egreso proyectado de RUS es posterior a la mayoría de edad de un niño que aún es menor."""
    if nacimiento is None or egreso is None:
        return []
    mayoria = fecha_mayoria(nacimiento)
    if _d(egreso) <= _d(mayoria):
        return []
    if mayoria_dicha:
        return [render('HISTORIA', 'EGRESO_POSTERIOR_MAYORIA')]
    return [render('HISTORIA', 'EGRESO_POSTERIOR_MAYORIA_FECHA', FECHA_MAYORIA=fecha_es(mayoria))]


def aclaracion_causa():
    """Transversal: la historia es de la causa; con varios registros en la nómina no se atribuye a un ingreso."""
    ctx = current.get()
    if ctx is None or duplicidad() is None:
        return []
    emitidos_ahora = {e for e in ctx['events'] if e not in ctx['config']['desactivadas']}
    if not CITAN & emitidos_ahora:
        return []
    texto = render('HISTORIA', 'R3_MULTI')
    return [texto] if texto else []


def causa_finalizada(rit):
    """Causa P con «Conciliacion» o «Dictación de sentencia» sin «Egreso de servicio…» en esa fecha o posterior.
    Solo con la cobertura disponible (completa o parcial): el egreso debe ser posterior al cierre."""
    if not str(rit or '').strip().upper().startswith('P'):
        return []
    h = _h()
    if h is None:
        return []
    cierre = ultimo(h.buscar(V.CONCILIACION, V.DICTACION_SENTENCIA))
    if cierre is None or any(t.fecha >= cierre.fecha for t in h.buscar(V.EGRESO_SERVICIO)):
        return []
    return [render('HISTORIA', 'P_FINALIZADA')]


def vencida_valida(vencida):
    """La medida vencida sigue siendo afirmable (no hay contradicción días negativos / egreso futuro)."""
    ctx = current.get()
    return bool(vencida and ctx is not None and 'CUMPLIMIENTO.C04_VENCIDA' not in ctx.get('invalid', ()))


def solo_historia():
    """True si todo lo emitido hasta ahora en la fila proviene de reglas de historia."""
    ctx = current.get()
    eventos = [e for e in ctx['events'] if e not in ctx['config']['desactivadas']] if ctx else []
    return bool(eventos) and all(e.startswith('HISTORIA.') for e in eventos)
