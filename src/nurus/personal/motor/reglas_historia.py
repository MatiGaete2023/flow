"""Constancias de historia de la causa: cada función devuelve fragmentos de texto ([] si no aplica)."""

from datetime import timedelta

from . import vocabulario as V
from .analisis_historia import busqueda, duplicidades, instituciones, posterior, primera_observacion  # noqa: F401
from .contexto import acumulado, bitacora, current, duplicidad, emitidos, evento, historia, marcar
from .contexto import datetime as Clock
from .historia import clave, ultimo
from .textos import render
from .utilidades import contiene_token, es_dce, fecha_es, normalizar

ORIGINAL = {
    'CURADOR': ('COMUN.CURADOR', 'MATERIA_CURADOR'),
    'SIN_FICHA': ('CUMPLIMIENTO.C07_SIN_FICHA', 'MATERIA_SIN_FICHA'),
    'SIN_FAE': ('CUMPLIMIENTO.C08_FICHA_FAE', 'MATERIA_SIN_FAE'),
    'VENCIDA': ('CUMPLIMIENTO.C04_VENCIDA', 'MATERIA_VENCIDA'),
}


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
    """Regla 1: folio único de los trámites con la fecha de la resolución de RUS; None si no es unívoco."""
    h = _h()
    if h is None or fecha is None:
        return None
    folios = {t.folio_txt for t in h.tramites if t.fecha == _d(fecha) and t.folio_txt}
    return next(iter(folios)) if len(folios) == 1 else None


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
    if duplicidad() is not None:
        frags.append(render('HISTORIA', 'R3_MULTI'))
    return [f for f in frags if f]


def lista_espera_larga(programa, espera):
    if espera is not None and intervencion(programa) == 'RESIDENCIAL' and espera > V.E10_DIAS:
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
    return texto


def curador_designaciones():
    return constancias(*V.CURADOR)


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


def servicio():
    return constancias(V.SERVICIO)


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
            llave = 'R11_TERMINO' if cierre.clave == clave(V.TERMINO_BUSQUEDA) else 'R11_SIN_EFECTO'
            texto = render(
                'HISTORIA', llave, DESC=cierre.desc, FECHA=fecha_es(cierre.fecha), FOLIO=_folio(cierre),
                FECHA_ORDEN=fecha_es(orden.fecha), FOLIO_ORDEN=_folio(orden),
            )
        if texto:
            salida.append(texto)
    salida += constancias(*V.BUSQUEDA_CITADA)
    return salida


def derecho_oido(fecha_rus):
    """C3: solo si RUS no tiene fecha de oído; usa la última aplicación de la historia."""
    lista = tramites(V.DERECHO_OIDO)
    if fecha_rus or not lista:
        return []
    return [render('COMUN', 'OIDO', FECHA_OIDO=fecha_es(ultimo(lista).fecha))]


def folio_ingreso(fecha_ingreso):
    """C-03 ampliado: folio del «Ingreso efectivo» cuya fecha coincide con la de RUS."""
    if fecha_ingreso is None:
        return None
    ult = ultimo(tramites(V.INGRESO_EFECTIVO))
    if ult is not None and ult.fecha == _d(fecha_ingreso) and ult.folio_txt:
        emitidos().add(_id(ult))
        return ult.folio_txt
    return None


def ingreso_efectivo_c1(fecha_ingreso):
    """C1: constancia del ingreso efectivo con la fecha que registra RUS (si difieren o falta C-03)."""
    texto = constancia(tramites(V.INGRESO_EFECTIVO))
    if not texto:
        return []
    salida = [texto]
    if fecha_ingreso is not None:
        salida.append(render('HISTORIA', 'C1_RUS', FECHA_INGRESO=fecha_es(fecha_ingreso)))
    return [s for s in salida if s]


def prorroga(corte, egreso_proyectado):
    """C2: «Prorroga» presente en la historia junto al egreso proyectado de RUS."""
    lista = [t for t in tramites(V.PRORROGA) if _id(t) not in emitidos()]
    if not lista or egreso_proyectado is None:
        return []
    emitidos().update(_id(t) for t in lista)
    ult = ultimo(lista)
    return [
        render(
            'HISTORIA', 'C2_PRORROGA', FECHA=fecha_es(ult.fecha), FOLIO=_folio(ult), CORTE=fecha_es(corte),
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
    return [texto, render('HISTORIA', 'C4_FICHA_SIN_RUS')]


def no_uso_prorroga(egreso_proyectado):
    """C8 con guardia G3: exige cobertura que permita ausencia y período consultado."""
    h = historia()
    if h is None or not h.permite_ausencia or h.desde is None or h.hasta is None or egreso_proyectado is None:
        return []
    if h.buscar(V.PRORROGA):
        return []
    marcar(3, 'C8_NO_USO', 'Medida vencida sin Prórroga en el período consultado')
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
