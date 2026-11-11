"""Constancias de historia de la causa: cada función devuelve fragmentos de texto ([] si no aplica)."""

import re
from datetime import timedelta

from . import vocabulario as V
from .analisis_historia import busqueda, duplicidades, instituciones, posterior, primera_observacion  # noqa: F401
from .contexto import acumulado, bitacora, current, duplicidad, emitidos, evento, historia, marcar, parametro
from .contexto import datetime as Clock
from .historia import clave, ultimo
from .textos import Frag, con_grupo, render
from .utilidades import contiene_token, es_dce, fecha_es, fecha_mayoria, normalizar, titulo_programa

# La medida vencida no se reitera (11-nov-2026): su frase es igual de breve y lleva la fecha vigente.
ORIGINAL = {
    'CURADOR': ('COMUN.CURADOR', 'MATERIA_CURADOR'),
    'SIN_FICHA': ('CUMPLIMIENTO.C07_SIN_FICHA', 'MATERIA_SIN_FICHA'),
    'SIN_FAE': ('CUMPLIMIENTO.C08_FICHA_FAE', 'MATERIA_SIN_FAE'),
}

_RESOLUCION = 'en resolución de fecha'
_TIPOS = {'escrito': 'en escrito de fecha', 'actuacion': 'en actuación de fecha'}
_MESES = (
    'enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre', 'noviembre',
    'diciembre',
)  # fmt: skip
_PROYECCION = re.compile(
    r'proyecta\s+(?:el\s+)?ingreso(?:\s+efectivo)?\s+para\s+(?:el\s+mes\s+de\s+|el\s+|)'
    r'(' + '|'.join((*_MESES, 'setiembre')) + r')\b(?:\s+(?:de\s+|del\s+)?(\d{4}))?',
    re.IGNORECASE,
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


def varios_registros():
    """True si la causa tiene más de un registro en la nómina: la historia, que es de la causa, no se puede atribuir
    a esta fila. No se afirman hechos personales ni se retiran gestiones por un trámite (11-nov-2026)."""
    return duplicidad() is not None


def _cita(fragmento, tramite):
    """Nombra el trámite por lo que es (11-nov-2026): la fórmula «en resolución de fecha» pasa a «en escrito de
    fecha» o «en actuación de fecha» según la columna «Trámite» de SITFA. Sin ese dato se conserva «resolución»."""
    nuevo = _TIPOS.get(clave(getattr(tramite, 'tipo', '')))
    if not fragmento or not nuevo or _RESOLUCION not in str(fragmento):
        return fragmento
    salida = Frag(str(fragmento).replace(_RESOLUCION, nuevo, 1))
    salida.grupo = getattr(fragmento, 'grupo', 'C')
    salida.dato = getattr(fragmento, 'dato', '')
    salida.partes = tuple((texto.replace(_RESOLUCION, nuevo, 1), grupo) for texto, grupo in getattr(fragmento, 'partes', ()))
    return salida


def intervencion(programa):
    if es_dce(programa):
        return 'DCE'
    if normalizar(programa).startswith(V.RESIDENCIA_PREFIJOS):
        return 'RESIDENCIAL'
    if contiene_token(programa, 'fae', 'fas'):
        return 'FAE'
    return 'AMBULATORIO'


def tramites(*nombres, contexto=False):
    h = _h()
    return h.buscar(*nombres, contexto=contexto) if h else []


def ultima_observacion():
    """Fecha de la última observación de la bitácora del ingreso, o None si no hay bitácora."""
    observaciones = bitacora()
    return observaciones[-1].fecha if observaciones else None


def _citado(tramite):
    """True si una observación de la bitácora, del día del trámite o posterior, ya cita su folio («folio N»)."""
    if not tramite.folio_txt:
        return False
    folio = re.escape(tramite.folio_txt)
    return any(
        obs.fecha >= tramite.fecha and re.search(rf'\bfolio (?:n[o°º]? )?{folio}(?!\d)', clave(obs.texto))
        for obs in bitacora()
    )


def novedad(tramite):
    """Un solo criterio de novedad (11-nov-2026): una constancia de historia se dice solo si el trámite es de la fecha
    de la última observación de la bitácora o posterior y esa bitácora no citó ya su folio. Lo ya revisado no se
    repite en cada revisión. Sin bitácora, todo trámite de la ventana es novedad."""
    ultima = ultima_observacion()
    if ultima is not None and tramite.fecha < ultima:
        return False
    return not _citado(tramite)


def _nuevos(lista):
    return [t for t in lista if _id(t) not in emitidos() and novedad(t)]


def constancia(lista):
    """Texto base de uno o varios trámites del mismo nombre; omite los ya citados ('' si no queda ninguno)."""
    lista = [t for t in lista if _id(t) not in emitidos()]
    if not lista:
        return ''
    emitidos().update(_id(t) for t in lista)
    ult = ultimo(lista)
    if len(lista) == 1:
        texto = render('HISTORIA', 'BASE_TRAMITE', DESC=ult.desc, FECHA=fecha_es(ult.fecha), FOLIO=_folio(ult))
    else:
        texto = render(
            'HISTORIA', 'VARIOS', N=len(lista), DESC=ult.desc, FECHA=fecha_es(ult.fecha), FOLIO=_folio(ult)
        )
    return _cita(texto, ult)


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


def proyeccion_ingreso(corte):
    """Espera (decisión del usuario, 11-nov-2026): si la última observación de la bitácora dice «…proyecta ingreso
    efectivo para el mes de M», devuelve esa oración tal como está escrita mientras M sea posterior al mes del corte.
    Con el mes del corte o uno ya pasado devuelve '' y rigen las reglas de espera por defecto."""
    observaciones = bitacora()
    if not observaciones:
        return ''
    ultima = observaciones[-1]
    texto = ' '.join(str(ultima.texto or '').split())
    hallazgo = _PROYECCION.search(texto)
    if not hallazgo:
        return ''
    nombre = hallazgo.group(1).lower()
    mes = 9 if nombre == 'setiembre' else _MESES.index(nombre) + 1
    if hallazgo.group(2):
        anio = int(hallazgo.group(2))
    else:
        anio = ultima.fecha.year + (1 if mes < ultima.fecha.month else 0)
    corte = _d(corte)
    if (anio, mes) <= (corte.year, corte.month):
        return ''
    corte_previo = max(texto.rfind('. ', 0, hallazgo.start()), texto.rfind(': ', 0, hallazgo.start()))
    inicio = corte_previo + 2 if corte_previo >= 0 else 0
    fin = texto.find('.', hallazgo.end())
    oracion = texto[inicio : fin + 1 if fin >= 0 else len(texto)].strip()
    return oracion


def marcar_duplicidad():
    if duplicidad() == 'MISMO_PROGRAMA':
        marcar(1, 'R6_DUPLICIDAD', 'Más de un ingreso del mismo NNA al mismo programa')


def reitera(materia):
    """REITERA: '' si no hay observación previa de la materia o la regla original está desactivada. La frase es
    autosuficiente: dice lo que falta y que la sugerencia se reitera. Con un trámite de curador posterior a la
    observación citada la situación cambió y no se reitera."""
    original, clave_materia = ORIGINAL[materia]
    ctx = current.get()
    if ctx is None or original in ctx['config']['desactivadas']:
        return ''
    previa = primera_observacion(bitacora(), materia, Clock.now().date())
    if previa is None:
        return ''
    if materia == 'CURADOR' and any(t.fecha > previa.fecha for t in tramites(*V.CURADOR, contexto=True)):
        return ''
    texto = render(
        'HISTORIA', 'REITERA', FECHA_ANTERIOR=fecha_es(previa.fecha), MATERIA=render('HISTORIA', clave_materia)
    )
    if texto:
        evento(original)
    # Reiterar la ausencia de curador o de ficha es gestión del Tribunal.
    return con_grupo(str(texto), 'T')


def _ultimo_curador():
    """(trámite, es_baja) del último trámite de curador de la causa, incluido el contexto anterior a la ventana."""
    designa = ultimo(tramites(V.CURADOR[0], contexto=True))
    baja = ultimo(tramites(V.CURADOR[1], contexto=True))
    if designa is None and baja is None:
        return None, False
    ult = baja if designa is None else designa if baja is None or posterior(designa, baja) else baja
    return ult, ult is baja


def curador_sin_efecto(rus_sin_curador):
    """True si RUS no registra curador y lo último de la causa es «Deja sin efecto designación C. Ad-litem», en una
    causa de un solo registro: no hay a quién asociar y la observación no lo sugiere."""
    if not rus_sin_curador or varios_registros():
        return False
    return _ultimo_curador()[1]


def curador_designaciones(rus_sin_curador):
    """Último trámite de curador de la historia, solo si RUS no registra curador y la causa tiene un solo registro.
    Se cita con su nombre literal («Designa abogado»): la nomenclatura no acredita que sea curador ni de quién."""
    if not rus_sin_curador or varios_registros():
        return []
    ult, es_baja = _ultimo_curador()
    h = _h()
    if ult is None or h is None or not h.en_ventana(ult) or _id(ult) in emitidos() or not novedad(ult):
        return []
    marcar(2, 'CURADOR_SIN_EFECTO' if es_baja else 'CURADOR_DESIGNADO', 'Curador ad litem en la historia y RUS sin curador registrado')
    texto = constancia([ult])
    return [texto] if texto else []


def curador_instituciones(celda, pnombre):
    cantidad = len(instituciones(celda))
    if cantidad < 2:
        return []
    marcar(1, 'T5_INSTITUCIONES', 'Curador con varias instituciones en RUS')
    return [render('HISTORIA', 'T5_INSTITUCIONES', N=cantidad, PNOMBRE=pnombre)]


def servicio(programa):
    """Espera: resolución que dispone el ingreso al programa («Ing.Serv. …»); el programa es el de la fila de RUS,
    por eso solo se dice en causas de un registro."""
    if varios_registros():
        return []
    ult = ultimo(_nuevos(tramites(V.SERVICIO)))
    if ult is None:
        return []
    emitidos().add(_id(ult))
    texto = render('HISTORIA', 'SERVICIO_INGRESO', PROGRAMA=titulo_programa(programa), FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha))
    return [_cita(texto, ult)]


def continuidad():
    """Espera y Cumplimiento: respuesta sobre la continuidad de atención (se hace constar lo resuelto)."""
    ult = ultimo(_nuevos(tramites(V.CONTINUIDAD)))
    if ult is None:
        return []
    emitidos().add(_id(ult))
    return [_cita(render('HISTORIA', 'CONTINUIDAD_ATENCION', FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha)), ult)]


def solicitud_dce(programa, vencida):
    """Cumplimiento DCE con la medida vencida: informe clínico diagnóstico ordenado en autos (SPE. Resp. Solicitud DCE)."""
    if not vencida or intervencion(programa) != 'DCE' or varios_registros():
        return []
    ult = ultimo(_nuevos(tramites(V.SOLICITUD_DCE)))
    if ult is None:
        return []
    emitidos().add(_id(ult))
    return [_cita(render('HISTORIA', 'SOLICITUD_DCE', FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha)), ult)]


def marcas_sin_efecto():
    """Espera y Cumplimiento: «Deja sin efecto derivación D.R.» y «Deja sin efecto medida interconexión M.N.» solo
    marcan para revisión (nivel 2), sin texto (decisión x)."""
    if tramites(V.SIN_EFECTO_DR):
        marcar(2, 'DEJA_SIN_EFECTO_DR', 'Deja sin efecto derivación D.R. en la historia')
    if tramites(V.SIN_EFECTO_MN):
        marcar(2, 'DEJA_SIN_EFECTO_MN', 'Deja sin efecto medida interconexión M.N. en la historia')
    tramites_sin_regla()


def tramites_sin_regla():
    """Deja rastro de lo que el motor no reconoce (11-nov-2026): trámites nuevos de la ventana cuyo nombre no está en
    el vocabulario. Solo marca (nivel 3); nunca va al texto para RUS."""
    h = _h()
    if h is None:
        return
    conocidas = {clave(n) for n in V.CONOCIDAS}
    nombres = {}
    for t in h.tramites:
        if h.en_ventana(t) and t.clave not in conocidas and novedad(t):
            nombres[t.clave] = t.desc
    if nombres:
        lista = sorted(nombres.values())
        detalle = ', '.join(lista[:5]) + (f' y {len(lista) - 5} más' if len(lista) > 5 else '')
        marcar(3, 'TRAMITES_SIN_REGLA', f'Trámites sin regla en la historia ({len(lista)}): {detalle}')


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
    efecto, término). Con bitácora, la búsqueda se dice si su último trámite es novedad (posterior a la última
    observación y no mencionado); sin bitácora, si cae en los últimos `ventana_busqueda` días. La orden puede ser el
    último trámite anterior a la ventana conservado como contexto. Marca nivel 1 si hay una orden vigente."""
    h = _h()
    if h is None:
        return []
    _, vigentes = busqueda(h)
    if vigentes:
        marcar(1, 'R11_BUSQUEDA_VIGENTE', 'Orden de búsqueda vigente')
    desde = current.get()['date'] - timedelta(days=parametro('ventana_busqueda'))
    desde = desde.date() if hasattr(desde, 'date') else desde
    con_bitacora = ultima_observacion() is not None

    def reciente(t):
        if not h.en_ventana(t):
            return False
        return novedad(t) if con_bitacora else t.fecha >= desde

    ordenes = sorted(h.buscar(V.ORDEN_BUSQUEDA, contexto=True), key=lambda t: (t.fecha, t.folio))
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
        # La ventana se mide por el último trámite de la búsqueda (decisión del usuario, 16-oct): una orden antigua con
        # un término reciente se dice completa; si todo es anterior, o la bitácora ya lo observó, no se dice nada.
        if not reciente(cadena[-1]) or _id(orden) in emitidos() or _busqueda_en_bitacora(cadena[-1]):
            continue
        emitidos().update(_id(t) for t in cadena)
        texto = render('HISTORIA', 'R11_ORDEN', FECHA=fecha_es(orden.fecha), FOLIO=_folio(orden))
        if not texto:
            continue
        salida.append(texto)
        salida += [f for f in (_frase_busqueda(t, n) for n, t in enumerate(cadena[1:])) if f]
    # Cierre o consulta sin la orden en la historia: constancia neutra, solo si es reciente y no se observó antes.
    for t in huerfanos:
        if reciente(t) and not _busqueda_en_bitacora(t):
            texto = constancia([t])
            if texto:
                salida.append(texto)
    return salida


def derecho_oido(fecha_rus):
    """C3: solo si RUS no tiene fecha de oído. La historia acredita que se aplicó la nomenclatura en la causa, no la
    fecha en que la persona fue oída: se cita como trámite (11-nov-2026)."""
    lista = tramites(V.DERECHO_OIDO)
    if fecha_rus or not lista:
        return []
    ult = ultimo(lista)
    if oido_en_bitacora(ult.fecha) or not novedad(ult):
        return []
    texto = constancia([ult])
    return [texto] if texto else []


def oido_en_bitacora(fecha):
    """True si una observación anterior de la bitácora ya dejó constancia del oído en esa fecha."""
    fecha = _d(fecha)
    formas = (clave(fecha_es(fecha)), fecha.strftime('%d/%m/%Y'))
    for obs in bitacora():
        texto = clave(obs.texto)
        if 'oido' in texto and any(forma in texto for forma in formas):
            return True
    return False


def _prorroga_en_bitacora(prorroga):
    """True si la bitácora ya menciona esa prórroga: se revisó en una observación anterior."""
    return _en_bitacora(prorroga, 'prorrog')


def prorroga(corte, egreso_proyectado, vencida, por_vencer, egreso_dicho):
    """Cumplimiento (11-nov-2026). La historia acredita que en la causa consta el trámite «Prorroga», no su término:
    se cita como trámite y el egreso proyectado de RUS se dice aparte, sin unirlos con «hasta». Con la medida vencida,
    por vencer o con el egreso de RUS anterior a la prórroga se marca para revisión prioritaria si es reciente
    (prorroga_reciente) y, en causas de un registro, se hace constar junto al estado de la medida. En causas con
    varios registros solo se cita con la medida vigente. Si la bitácora ya la mencionó, se ignoran frase y marca."""
    lista = [t for t in tramites(V.PRORROGA) if _id(t) not in emitidos()]
    if not lista or egreso_proyectado is None:
        return []
    ult = ultimo(lista)
    if _prorroga_en_bitacora(ult):
        return []
    reciente = (_d(corte) - ult.fecha).days <= parametro('prorroga_reciente')
    sin_reflejar = _d(egreso_proyectado) < ult.fecha
    pendiente = vencida or por_vencer or sin_reflejar
    if reciente and pendiente:
        marcar(1, 'PRORROGA_REVISAR', 'Prórroga reciente con medida vencida o por vencer: verificar su registro en RUS')
    # La marca avisa de un registro pendiente en RUS y se mantiene; la frase solo se dice si el trámite es novedad.
    if not novedad(ult) or (pendiente and (not reciente or varios_registros())):
        emitidos().update(_id(t) for t in lista)
        return []
    salida = [constancia([ult])]
    emitidos().update(_id(t) for t in lista)
    if not pendiente and not egreso_dicho and not varios_registros():
        salida.append(render('HISTORIA', 'EGRESO_PROYECTADO', FECHA_EGRESO_PROYECTADO=fecha_es(egreso_proyectado)))
    return [f for f in salida if f]


def escrito_sin_ingreso(programa, desde=None):
    """C10/I3: último escrito de informe sin «Ingreso informe de cumplimiento X» posterior (por folio). El escrito es
    de la causa: con varios registros no se sabe de quién es el informe y no se propone el proyecto."""
    if varios_registros():
        return []
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
    texto = render('HISTORIA', 'C10_ESCRITO', FOLIO_ESCRITO=_folio(escrito), FECHA_ESCRITO=fecha_es(escrito.fecha))
    return [_cita(texto, escrito)]


def informe_historia(fecha_vencimiento, vencido, atribuible=True):
    """Informes: «Ingreso Informe de Cumplimiento X» y estado que RUS registra. [] si no aplica.
    Con el informe vencido solo cuenta un ingreso del día del vencimiento o posterior: uno anterior es de otro
    período y no desmiente el vencimiento (decisión u, 14-oct). Si el trámite no se puede atribuir a este informe
    (varios registros en la causa o varios informes vencidos), se cita como trámite de la causa."""
    lista = tramites(V.INGRESO_INFORME_X)
    if vencido:
        lista = [t for t in lista if t.fecha >= _d(fecha_vencimiento)]
    lista = [t for t in lista if _id(t) not in emitidos()]
    if not lista:
        return []
    ult = ultimo(lista)
    if not atribuible:
        texto = constancia([ult]) if novedad(ult) else ''
        emitidos().update(_id(t) for t in lista)
        return [texto] if texto else []
    emitidos().update(_id(t) for t in lista)
    # El vencimiento lo dice la frase principal del informe: no se repite (decisión x).
    return [_cita(render('HISTORIA', 'INFORME_INGRESADO', FOLIO=_folio(ult), FECHA=fecha_es(ult.fecha)), ult)]


def informe_previo(programa, fecha_vencimiento):
    """Presencia anterior al vencimiento, sin inferir a qué período corresponde."""
    nombres = [V.INGRESO_INFORME_X]
    escrito = V.ESCRITOS.get(intervencion(programa))
    if escrito:
        nombres.append(escrito)
    return any(t.fecha < _d(fecha_vencimiento) for t in tramites(*nombres))


def varios_vencidos():
    """True si el pre-pase encontró más de un informe vencido (fechas distintas) del mismo ingreso."""
    dato = acumulado()
    return bool(dato and dato[0] >= 2)


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


def causa_finalizada(rit):
    """Causa P con «Conciliacion» o «Dictación de sentencia» sin «Egreso de servicio…» en esa fecha o posterior.
    El cierre puede ser el último trámite anterior a la ventana conservado como contexto: la regla no caduca cuando
    la sentencia sale de la ventana."""
    if not str(rit or '').strip().upper().startswith('P'):
        return []
    h = _h()
    if h is None:
        return []
    cierre = ultimo(h.buscar(V.CONCILIACION, V.DICTACION_SENTENCIA, contexto=True))
    if cierre is None or any(t.fecha >= cierre.fecha for t in h.buscar(V.EGRESO_SERVICIO, contexto=True)):
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
