"""Constancias de historia de la causa: cada función devuelve fragmentos de texto ([] si no aplica)."""

import re
from datetime import timedelta

from . import vocabulario as V
from .analisis_historia import busqueda, duplicidades, instituciones, posterior, primera_observacion  # noqa: F401
from .contexto import acumulado, bitacora, current, duplicidad, emitidos, evento, historia, marcar, parametro
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
        'SERVICIO_INGRESO',
        'CURADOR_SIN_EFECTO',
        'CURADOR_DESIGNADO',
        'R11_ORDEN',
        'R11_PIDE_CUENTA',
        'R11_SIN_EFECTO',
        'R11_TERMINO',
        'SOLICITUD_DCE',
        'CONTINUIDAD_ATENCION',
        'INFORME_INGRESADO',
        'C10_ESCRITO',
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


def ingreso_efectivo_en_espera(corte, espera=None):
    """Espera (decisión x): «Ingreso efectivo» aparece solo en Cumplimiento; si la historia lo trae mientras RUS sigue
    en Espera se aplican las reglas normales y solo se marca para revisión. Con T ESPERA válido cuenta únicamente el
    ingreso efectivo desde el inicio de la espera (corte - días): uno anterior es de otra medida."""
    lista = tramites(V.INGRESO_EFECTIVO)
    if espera is not None and espera >= 0:
        inicio = _d(corte) - timedelta(days=espera)
        lista = [t for t in lista if t.fecha >= inicio]
    if lista:
        marcar(2, 'R3_INGRESO_EFECTIVO', 'Historia con ingreso efectivo y RUS en Espera')


def lista_larga(programa, espera):
    return espera is not None and intervencion(programa) == 'RESIDENCIAL' and espera > parametro('lista_espera_larga')


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


def servicio(programa):
    """Espera: resolución que dispone el ingreso al programa («Ing.Serv. …»); el programa es el de la fila de RUS."""
    ult = ultimo([t for t in tramites(V.SERVICIO) if _id(t) not in emitidos()])
    if ult is None:
        return []
    emitidos().add(_id(ult))
    return [render('HISTORIA', 'SERVICIO_INGRESO', PROGRAMA=titulo_programa(programa), FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha))]


def continuidad():
    """Espera y Cumplimiento: respuesta sobre la continuidad de atención (se hace constar lo resuelto)."""
    ult = ultimo([t for t in tramites(V.CONTINUIDAD) if _id(t) not in emitidos()])
    if ult is None:
        return []
    emitidos().add(_id(ult))
    return [render('HISTORIA', 'CONTINUIDAD_ATENCION', FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha))]


def solicitud_dce(programa, vencida):
    """Cumplimiento DCE con la medida vencida: informe clínico diagnóstico ordenado en autos (SPE. Resp. Solicitud DCE)."""
    if not vencida or intervencion(programa) != 'DCE':
        return []
    ult = ultimo([t for t in tramites(V.SOLICITUD_DCE) if _id(t) not in emitidos()])
    if ult is None:
        return []
    emitidos().add(_id(ult))
    return [render('HISTORIA', 'SOLICITUD_DCE', FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha))]


def marcas_sin_efecto():
    """Espera y Cumplimiento: «Deja sin efecto derivación D.R.» y «Deja sin efecto medida interconexión M.N.» solo
    marcan para revisión (nivel 2), sin texto (decisión x)."""
    if tramites(V.SIN_EFECTO_DR):
        marcar(2, 'DEJA_SIN_EFECTO_DR', 'Deja sin efecto derivación D.R. en la historia')
    if tramites(V.SIN_EFECTO_MN):
        marcar(2, 'DEJA_SIN_EFECTO_MN', 'Deja sin efecto medida interconexión M.N. en la historia')


def _en_bitacora(tramite, materia):
    """True si una observación de la bitácora posterior al trámite ya lo mencionó: habla de la materia y cita su folio
    («folio N») o su fecha completa. Un número suelto (el día de otra fecha) no basta."""
    fecha = clave(fecha_es(tramite.fecha))
    folio = re.escape(tramite.folio_txt) if tramite.folio_txt else None
    for obs in bitacora():
        if obs.fecha < tramite.fecha:
            continue
        texto = clave(obs.texto)
        if materia not in texto:
            continue
        if fecha in texto or (folio and re.search(rf'\bfolio (?:n[o°º]? )?{folio}(?!\d)', texto)):
            return True
    return False


def _busqueda_en_bitacora(ultimo_tramite):
    """True si la bitácora ya menciona la búsqueda hasta su último trámite."""
    return _en_bitacora(ultimo_tramite, 'busqueda')


CONECTORES = ('Posteriormente, ', 'Luego, ')


def _frase_busqueda(tramite, posicion):
    """Trámite posterior a la orden; el primero abre con «Posteriormente» y los siguientes con «Luego» (decisión y8)."""
    llaves = {
        clave(V.PIDE_CUENTA_BUSQUEDA): 'R11_PIDE_CUENTA',
        clave(V.SIN_EFECTO_BUSQUEDA): 'R11_SIN_EFECTO',
        clave(V.TERMINO_BUSQUEDA): 'R11_TERMINO',
    }
    texto = render('HISTORIA', llaves[tramite.clave], FECHA=fecha_es(tramite.fecha), FOLIO=_folio(tramite))
    actual = next((c for c in CONECTORES if str(texto).startswith(c)), None)
    if not texto or actual is None:
        return texto
    deseado = CONECTORES[0 if posicion == 0 else 1]
    return con_grupo(deseado + str(texto)[len(actual) :], 'C')


def busqueda_historia():
    """R11: la observación nombra primero la Orden de búsqueda y luego sus trámites relacionados (pide cuenta, deja sin
    efecto, término). Solo cuenta lo de los últimos `ventana_busqueda` días: una orden anterior, o una búsqueda que la
    bitácora ya observó, no genera texto. Marca nivel 1 si hay una orden vigente."""
    h = _h()
    if h is None:
        return []
    _, vigentes = busqueda(h)
    if vigentes:
        marcar(1, 'R11_BUSQUEDA_VIGENTE', 'Orden de búsqueda vigente')
    desde = current.get()['date'] - timedelta(days=parametro('ventana_busqueda'))
    desde = desde.date() if hasattr(desde, 'date') else desde
    ordenes = sorted(h.buscar(V.ORDEN_BUSQUEDA), key=lambda t: (t.fecha, t.folio))
    seguimiento = [t for t in h.buscar(*V.BUSQUEDA_SEGUIMIENTO)]
    cadenas = {_id(o): [o] for o in ordenes}
    huerfanos = []
    for t in sorted(seguimiento, key=lambda t: (t.fecha, t.folio)):
        previas = [o for o in ordenes if posterior(t, o)]
        if previas:
            cadenas[_id(max(previas, key=lambda o: (o.fecha, o.folio)))].append(t)
        else:
            huerfanos.append(t)
    salida = []
    for orden in ordenes:
        cadena = cadenas[_id(orden)]
        if orden.fecha < desde or _id(orden) in emitidos() or _busqueda_en_bitacora(cadena[-1]):
            continue
        emitidos().update(_id(t) for t in cadena)
        texto = render('HISTORIA', 'R11_ORDEN', FECHA=fecha_es(orden.fecha), FOLIO=_folio(orden))
        if not texto:
            continue
        salida.append(texto)
        salida += [f for f in (_frase_busqueda(t, n) for n, t in enumerate(cadena[1:])) if f]
    # Cierre o consulta sin la orden en la historia: constancia neutra, solo si es reciente y no se observó antes.
    for t in huerfanos:
        if t.fecha >= desde and not _busqueda_en_bitacora(t):
            texto = constancia([t])
            if texto:
                salida.append(texto)
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
    """«Ingreso Efectivo» de la historia fechado entre la fecha de RUS y 30 días después (umbral ventana_ingreso): la nomenclatura se aplicó.
    La diferencia de días es normal (RUS guarda el día real; la historia, la resolución que lo tiene presente)."""
    h = _h()
    if h is None or fecha_ingreso is None:
        return None
    inicio = _d(fecha_ingreso)
    limite = inicio + timedelta(days=parametro('ventana_ingreso'))
    cercanos = [t for t in h.buscar(V.INGRESO_EFECTIVO) if inicio <= t.fecha <= limite]
    return min(cercanos, key=lambda t: (t.fecha, t.folio)) if cercanos else None


def ingreso_informado(fecha_ingreso):
    """Cumplimiento sin ingreso reciente: si la historia confirma el ingreso, se dice la fecha de RUS (sin contraste)."""
    if ingreso_verificado(fecha_ingreso) is None:
        return []
    return [render('HISTORIA', 'INGRESO_INFORMADO', FECHA_INGRESO=fecha_es(fecha_ingreso))]


def _prorroga_en_bitacora(prorroga):
    """True si la bitácora ya menciona esa prórroga: se revisó en una observación anterior."""
    return _en_bitacora(prorroga, 'prorrog')


def prorroga(corte, egreso_proyectado, vencida, por_vencer, egreso_dicho):
    """Cumplimiento (decisión x). Con la medida vencida, por vencer, o con el egreso de RUS anterior a la prórroga (RUS
    no la refleja), no se afirma la prórroga: prevalece el texto de la medida y solo se marca para revisión
    prioritaria si es reciente (prorroga_reciente, 90 días). Con la medida vigente se hace constar lo resuelto, salvo
    que otra frase ya diga el egreso proyectado. Si la bitácora ya mencionó la prórroga, se ignoran frase y marca."""
    lista = [t for t in tramites(V.PRORROGA) if _id(t) not in emitidos()]
    if not lista or egreso_proyectado is None:
        return []
    ult = ultimo(lista)
    if _prorroga_en_bitacora(ult):
        return []
    emitidos().update(_id(t) for t in lista)
    reciente = (_d(corte) - ult.fecha).days <= parametro('prorroga_reciente')
    sin_reflejar = _d(egreso_proyectado) < ult.fecha
    if reciente and (vencida or por_vencer or sin_reflejar):
        marcar(1, 'PRORROGA_REVISAR', 'Prórroga reciente con medida vencida o por vencer: verificar su registro en RUS')
    if vencida or por_vencer or sin_reflejar or egreso_dicho:
        return []
    return [
        render(
            'HISTORIA', 'C2_PRORROGA_HASTA', FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha),
            FECHA_EGRESO_PROYECTADO=fecha_es(egreso_proyectado),
        )
    ]


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
    """Informes: «Ingreso Informe de Cumplimiento X» y estado que RUS registra. [] si no aplica.
    Con el informe vencido solo cuenta un ingreso del día del vencimiento o posterior: uno anterior es de otro
    período y no desmiente el vencimiento (decisión u, 14-oct)."""
    lista = tramites(V.INGRESO_INFORME_X)
    if vencido:
        lista = [t for t in lista if t.fecha >= _d(fecha_vencimiento)]
    lista = [t for t in lista if _id(t) not in emitidos()]
    if not lista:
        return []
    ult = ultimo(lista)
    emitidos().update(_id(t) for t in lista)
    # El vencimiento lo dice la frase principal del informe: no se repite (decisión x).
    return [render('HISTORIA', 'INFORME_INGRESADO', FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha))]


def informe_previo(programa, fecha_vencimiento):
    """Presencia anterior al vencimiento, sin inferir a qué período corresponde."""
    nombres = [V.INGRESO_INFORME_X]
    escrito = V.ESCRITOS.get(intervencion(programa))
    if escrito:
        nombres.append(escrito)
    return any(t.fecha < _d(fecha_vencimiento) for t in tramites(*nombres))


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
