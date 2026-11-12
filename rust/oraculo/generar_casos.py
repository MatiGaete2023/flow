"""Oráculo: genera casos ficticios al azar y guarda lo que responde el motor Python.

Uso (desde la raíz del repo, con pandas<3 instalado):
    python3 rust/oraculo/generar_casos.py [casos_por_etapa] [semilla]

Escribe rust/casos/{espera,cumplimiento,informes}.json. Todos los datos son inventados: nombres de fantasía,
RUT con dígito verificador inválido y RIT del año 2099. `cargo test -p flow-motor` exige igualdad exacta.
"""

import json
import random
import sys
from datetime import date, datetime, timedelta
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'src'))

from nurus.personal.config import UMBRALES  # noqa: E402
from nurus.personal.contexto_excel import avisos_fila  # noqa: E402
from nurus.personal.motor import historia as H  # noqa: E402
from nurus.personal.motor.composicion import Incidencias  # noqa: E402
from nurus.personal.motor.contexto import current  # noqa: E402
from nurus.personal.motor.reglas_cumplimiento import generar_observacion_cumplimiento  # noqa: E402
from nurus.personal.motor.reglas_espera import generar_observacion_espera  # noqa: E402
from nurus.personal.motor.reglas_informes import generar_observacion_informes  # noqa: E402
from nurus.personal.work import actions_for  # noqa: E402
from nurus.rus.util import as_date, as_int, tribunal  # noqa: E402

TEXTOS = json.loads((RAIZ / 'src/nurus/personal/textos_base.json').read_text(encoding='utf-8'))
CORTE = date(2026, 11, 12)

PROGRAMAS = [
    'PRM Laja', 'PRM CIUDAD DEL NIÑO - MULCHÉN', 'DCE Los Ángeles', 'Diagnóstico Clínico Especializado',
    'FAE PRO Biobío', 'FAS Tomé', 'RES Residencia Familiar', 'RFA Hogar Ficticio', 'RVA Residencia del Valle',
    'OPD Laja', 'CESFAM Mulchén', 'Hospital de Tomé', 'PIE 24 Horas', 'PPF Viento Sur', 'Colegio Ejemplo',
    'DAM Biobío', 'pee los aromos', 'RTA lactantes (sede norte)', 'Programa de Protección Especializada',
]
NOMBRES = [
    'SOFÍA ANDREA PÉREZ SOTO', 'matías ignacio (hijo) rojas', 'ÁNGEL', 'Ñandú del Carmen Fuentes',
    'LUCAS  (2)  ARAYA', 'Emilia', 'JOSÉ-MIGUEL TORO', '',
]
TRIBUNALES = ['Juzgado de Letras de Laja', 'Juzgado de Letras y Gar. de Mulchén', 'Juzgado de Familia de Tomé',
              'LAJA', 'Tomé', 'Juzgado de Familia de Concepción']
CURADORES = [
    '', '---', 'NO POSEE', 'Sin designar', '(11111111-K)ANA EJEMPLO',
    '(11111111-K)ANA EJEMPLO - Institución: CAJ Biobío',
    '(11111111-K)ANA EJEMPLO - Institución: CAJ Biobío#(22222222-0)LUIS MUESTRA - Institución: ODL',
    '(11111111-K)ANA EJEMPLO - Institución: CAJ#(22222222-0)LUIS MUESTRA - Institución: caj',
    'Institución: Programa Mi Abogado',
]
TRAMITES = [
    'Ingreso efectivo', 'Prorroga', 'Ingreso informe de cumplimiento X', 'Orden de búsqueda', 'Pide cuenta búsqueda',
    'Termino de búsqueda', 'Deja sin efecto búsqueda', 'Deja sin efecto medida interconexión M.N.',
    'Deja sin efecto derivación D.R.', 'Resp. continuidad de atención', 'SPE. Resp. Solicitud DCE (escrito)',
    'Derecho a ser oído', 'Ing.Serv. protección especializada y otras redes', 'Conciliacion',
    'Dictación de sentencia', 'Egreso de servicio protección especializada y otras redes', 'Designa abogado',
    'Deja sin efecto designación C. Ad-litem', 'Inf. Cump Programa Ambulatorio', 'Inf. Cump FAE/FAS',
    'Inf. Cump Centro Residencial', 'Resp. Diagnóstico clínico especializado', 'Mero trámite',
]
MESES = ['enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre',
         'noviembre', 'diciembre']
CAMPOS_FECHA = ('nacimiento', 'resolucion', 'ingreso', 'egreso_proy', 'oido', 'ficha_ind', 'ficha_fae', 'vencimiento')


def fecha_es(d):
    return f'{d.day} de {MESES[d.month - 1]} de {d.year}'


class Gen:
    def __init__(self, semilla):
        self.r = random.Random(semilla)

    def dia(self, lo, hi):
        return CORTE + timedelta(days=self.r.randint(lo, hi))

    def celda_fecha(self, d):
        """Una fecha en alguna de las formas que entrega RUS; a veces vacía o ilegible."""
        x = self.r.random()
        if d is None or x < 0.07:
            return self.r.choice([{'t': 'vacia'}, {'t': 'texto', 'v': ''}, {'t': 'texto', 'v': '---'}])
        if x < 0.11:
            return {'t': 'texto', 'v': self.r.choice(['pendiente', 'sin fecha', '31/02/2026'])}
        if x < 0.45:
            return {'t': 'fecha', 'v': d.isoformat()}
        if x < 0.75:
            return {'t': 'texto', 'v': d.strftime('%d/%m/%Y')}
        if x < 0.9:
            return {'t': 'texto', 'v': d.isoformat()}
        return {'t': 'texto', 'v': d.strftime('%d-%m-%Y')}

    def celda_entero(self, n):
        x = self.r.random()
        if n is None or x < 0.06:
            return self.r.choice([{'t': 'vacia'}, {'t': 'texto', 'v': '---'}, {'t': 'texto', 'v': 'nan'}])
        if x < 0.1:
            return self.r.choice([{'t': 'texto', 'v': 'muchos'}, {'t': 'numero', 'v': n + 0.5}])
        if x < 0.6:
            return {'t': 'numero', 'v': n}
        if x < 0.8:
            return {'t': 'texto', 'v': str(n)}
        return {'t': 'texto', 'v': f' {n}.0 '}

    def texto(self, opciones):
        return {'t': 'texto', 'v': self.r.choice(opciones)}

    def audiencias(self):
        x = self.r.random()
        if x < 0.35:
            return {'t': 'texto', 'v': ''}
        if x < 0.6:
            return self.celda_fecha(self.dia(-20, 90))
        partes = [self.dia(-20, 120).strftime('%d/%m/%Y') for _ in range(self.r.randint(2, 3))]
        if self.r.random() < 0.2:
            partes.append('por definir')
        return {'t': 'texto', 'v': self.r.choice([', ', ';', ' , ', '\n']).join(partes)}

    def rit(self):
        return {'t': 'texto', 'v': f"{self.r.choice('PPPCXM')}-{self.r.randint(1, 900)}-2099"}

    def comunes(self):
        campos = {
            'programa': self.texto(PROGRAMAS),
            'tribunal': self.texto(TRIBUNALES),
            'nombre': self.texto(NOMBRES),
            'rit': self.rit(),
            'rut': self.texto(['11111111-K', '22.222.222-0', '']),
        }
        edad = self.r.choice([5, 11, 16, 17, 17, 18, 18, 19])
        nac = date(CORTE.year - edad, self.r.randint(1, 12), self.r.randint(1, 28))
        if self.r.random() < 0.3:
            nac = date(CORTE.year - 18, CORTE.month, CORTE.day) + timedelta(days=self.r.randint(-70, 70))
        if self.r.random() < 0.05:
            nac = date(2008, 2, 29)
        self.opcional(campos, 'nacimiento', lambda: self.celda_fecha(nac), 0.9)
        self.opcional(campos, 'curador', lambda: self.texto(CURADORES), 0.85)
        self.opcional(campos, 'oido', lambda: self.celda_fecha(self.dia(-90, 5) if self.r.random() < 0.6 else None), 0.8)
        self.opcional(campos, 'prox_aud', self.audiencias, 0.8)
        return campos

    def opcional(self, campos, nombre, hacer, prob):
        if self.r.random() < prob:
            campos[nombre] = hacer()

    def historia(self, anclas=()):
        x = self.r.random()
        if x < 0.3:
            return None
        cobertura = self.r.choice(['COMPLETA', 'COMPLETA', 'PARCIAL', 'NO_CONSULTADA', 'FALLIDA'])
        tramites = []
        for _ in range(self.r.randint(0, 7)):
            folio = self.r.choice([str(self.r.randint(1, 60)), str(self.r.randint(1, 60)), '', '12-A'])
            dia = self.dia(-200, 15)
            if anclas and self.r.random() < 0.35:
                # Trámite fechado el mismo día (o cerca) de una fecha de RUS: resolución, vencimiento, ingreso.
                dia = self.r.choice(anclas) + timedelta(days=self.r.choice([0, 0, 0, 1, 5, 20, -1]))
            tramites.append({'desc': self.r.choice(TRAMITES), 'fecha': dia.isoformat(), 'folio_txt': folio})
        desde = self.dia(-130, -100) if self.r.random() < 0.8 else None
        if self.r.random() < 0.05:
            desde = self.dia(1, 30)
        return {'cobertura': cobertura, 'desde': desde.isoformat() if desde else None, 'hasta': CORTE.isoformat(),
                'tramites': tramites}

    def bitacora(self, historia):
        salida = []
        frases = [
            'No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.',
            'No registra ficha individual en RUS. Se sugiere confeccionarla.',
            'Sofía no registra ficha FAE en RUS.', 'Medida vencida en RUS desde el 1 de enero de 2026.',
            'Se reitera la observación sobre la medida vencida en RUS.', 'Medida revisada, sin observaciones.',
        ]
        for _ in range(self.r.randint(0, 3)):
            salida.append({'fecha': self.dia(-120, 10).isoformat(), 'texto': self.r.choice(frases)})
        if self.r.random() < 0.15:
            salida.append({'fecha': self.dia(-60, -1).isoformat(), 'texto': 'Emilia no registra ficha FAE en RUS. Se sugiere confeccionarla.'})
        if historia and historia['tramites'] and self.r.random() < 0.5:
            t = self.r.choice(historia['tramites'])
            d = date.fromisoformat(t['fecha'])
            tema = self.r.choice(['prórroga de la medida', 'orden de búsqueda', 'el oído del niño', 'otra materia'])
            cita = self.r.choice([f"folio {t['folio_txt'] or 3}", f"Folio N° {t['folio_txt'] or 3}", fecha_es(d),
                                  d.strftime('%d/%m/%Y'), 'sin cita'])
            salida.append({'fecha': (d + timedelta(days=self.r.randint(-3, 20))).isoformat(),
                           'texto': f'Se hace presente {tema}, {cita}.'})
        return salida

    def caso(self, etapa, numero):
        campos = self.comunes()
        hoja2 = None
        if etapa == 'ESPERA':
            dias = self.r.choice([0, 5, 29, 30, 31, 59, 60, 61, 100, 101, 250, -3])
            self.opcional(campos, 'espera', lambda: self.celda_entero(dias), 0.93)
            self.opcional(campos, 'resolucion', lambda: self.celda_fecha(self.dia(-70, 3)), 0.9)
        elif etapa == 'CUMPLIMIENTO':
            ingreso = self.dia(-400, 0)
            egreso = self.dia(-60, 200) if self.r.random() < 0.9 else None
            dc = (CORTE - ingreso).days if self.r.random() < 0.85 else self.r.choice([-5, 0, 10, 30, 31])
            dpe = (egreso - CORTE).days if egreso and self.r.random() < 0.85 else self.r.choice([-10, 0, 20, 45, 46, 200])
            self.opcional(campos, 'ingreso', lambda: self.celda_fecha(ingreso), 0.92)
            self.opcional(campos, 'egreso_proy', lambda: self.celda_fecha(egreso), 0.95)
            self.opcional(campos, 'dias_cumpl', lambda: self.celda_entero(dc), 0.93)
            self.opcional(campos, 'dias_egresar', lambda: self.celda_entero(dpe), 0.93)
            self.opcional(campos, 'ficha_ind', lambda: self.celda_fecha(self.dia(-300, 0) if self.r.random() < 0.7 else None), 0.85)
            self.opcional(campos, 'ficha_fae', lambda: self.celda_fecha(self.dia(-300, 0) if self.r.random() < 0.5 else None), 0.8)
            if self.r.random() < 0.5:
                hoja2 = egreso if egreso and self.r.random() < 0.2 else self.dia(1, 150)
        else:
            self.opcional(campos, 'vencimiento', lambda: self.celda_fecha(self.dia(-60, 60)), 0.95)
            self.opcional(campos, 'ingreso', lambda: self.celda_fecha(self.dia(-400, 0)), 0.5)
        anclas = []
        for nombre in ('resolucion', 'vencimiento', 'ingreso', 'egreso_proy'):
            celda = campos.get(nombre)
            if celda and celda['t'] == 'fecha':
                anclas.append(date.fromisoformat(celda['v']))
            elif celda and celda['t'] == 'texto' and len(celda['v']) == 10 and celda['v'][2] == '/':
                try:
                    anclas.append(datetime.strptime(celda['v'], '%d/%m/%Y').date())
                except ValueError:
                    pass
        historia = self.historia(anclas)
        desactivadas = []
        if self.r.random() < 0.12:
            claves = [m + '.' + k for m, v in TEXTOS.items() if m != '_meta' for k in v]
            desactivadas = sorted(self.r.sample(claves, self.r.randint(1, 4)))
        duplicidad = self.r.choice([None, None, None, 'MISMO_PROGRAMA', 'OTRO_REGISTRO'])
        acumulado = None
        if etapa == 'INFORMES' and self.r.random() < 0.3:
            acumulado = [self.r.randint(1, 4), self.dia(-200, -1).isoformat()]
        return {
            'id': f'{etapa[:3]}-{numero:04d}', 'etapa': etapa, 'corte': CORTE.isoformat(), 'desactivadas': desactivadas,
            'campos': campos, 'historia': historia, 'bitacora': self.bitacora(historia), 'duplicidad': duplicidad,
            'acumulado': acumulado, 'fecha_hoja2': hoja2.isoformat() if hoja2 else None,
        }


def valor(celda):
    tipo = celda['t']
    if tipo == 'vacia':
        return ''
    if tipo == 'fecha':
        d = date.fromisoformat(celda['v'])
        return datetime(d.year, d.month, d.day)
    return celda['v']


def esperado(caso):
    """Lo que hace `Work.analyze` por fila, con el motor Python real."""
    etapa = caso['etapa']
    corte = date.fromisoformat(caso['corte'])
    values = {k: valor(c) for k, c in caso['campos'].items()}
    cols = {k: k for k in caso['campos']}
    config = {'textos': TEXTOS, 'umbrales': dict(UMBRALES), 'desactivadas': set(caso['desactivadas'])}
    historia = None
    if caso['historia'] is not None:
        h = caso['historia']
        filas = [{'event_at': t['fecha'], 'description': t['desc'], 'folio': t['folio_txt']} for t in h['tramites']]
        historia = H.construir(filas, h['cobertura'], H.dia_iso(h['desde']), H.dia_iso(h['hasta']))
    bitacora = H.observaciones([{'event_at': o['fecha'], 'text': o['texto']} for o in caso['bitacora']])
    hoja2 = date.fromisoformat(caso['fecha_hoja2']) if caso['fecha_hoja2'] else None
    acumulado = caso['acumulado']
    for name in CAMPOS_FECHA + ('prox_aud',):
        if isinstance(values.get(name), (int, float)):
            values[name] = as_date(values[name])
    ctx = {
        'config': config, 'events': [], 'date': corte, 'invalid': [],
        'historia': historia.al_corte(corte) if historia is not None else None,
        'bitacora': tuple(o for o in bitacora if o.fecha <= corte), 'marcas': [],
        'duplicidad': caso['duplicidad'],
        'acumulado': (acumulado[0], date.fromisoformat(acumulado[1])) if acumulado else None,
    }
    token = current.set(ctx)
    inc = Incidencias()
    avisos = []
    try:
        if etapa == 'CUMPLIMIENTO':
            end = as_date(values.get('egreso_proy'))
            counts = [as_int(values.get(k)) for k in ('dias_cumpl', 'dias_egresar')]
            if end and end > corte and any(n is not None and n < 0 for n in counts):
                avisos.append('Días negativos y egreso futuro: verifica la contradicción; no se afirma que la medida esté vencida.')
                ctx['invalid'].append('CUMPLIMIENTO.C04_VENCIDA')
        court = tribunal(values.get('tribunal', '')) or ''
        if not court:
            avisos.append('Tribunal fuera de Laja, Mulchén y Tomé: sin reglas configuradas; requiere revisión manual.')
        args = {'incidencias': inc, 'fila_excel': 1}
        if etapa == 'CUMPLIMIENTO':
            args['fecha_hoja2'] = hoja2
        fn = {'ESPERA': generar_observacion_espera, 'CUMPLIMIENTO': generar_observacion_cumplimiento,
              'INFORMES': generar_observacion_informes}[etapa]
        observacion = fn(values, court, cols, **args) if court else ''
        avisos.extend(i['MOTIVO'] for i in inc._items)
        if observacion and 'COMUN.NO_SEGUIMIENTO' not in ctx['events']:
            avisos.extend(avisos_fila(values, cols, etapa, corte, hoja2))
    finally:
        current.reset(token)
    events = [e for e in ctx['events'] if e not in config['desactivadas']]
    excluido = 'COMUN.NO_SEGUIMIENTO' in events
    return {
        'observacion': str(observacion),
        'reglas': list(dict.fromkeys(events)),
        'acciones': [] if excluido else actions_for(events, hoja2, corte),
        'avisos': avisos,
        'marcas': [{'nivel': n, 'codigo': c, 'texto': t} for n, c, t in dict.fromkeys(ctx['marcas'])],
        'excluido': excluido,
    }


def dirigidos(etapa):
    """Casos armados a mano para reglas que el azar casi nunca alcanza."""
    def t(v):
        return {'t': 'texto', 'v': v}

    def f(dias):
        return {'t': 'fecha', 'v': (CORTE + timedelta(days=dias)).isoformat()}

    def tr(desc, dias, folio):
        return {'desc': desc, 'fecha': (CORTE + timedelta(days=dias)).isoformat(), 'folio_txt': folio}

    def caso(numero, campos, tramites, bitacora=(), duplicidad=None):
        base = {'tribunal': t('Juzgado de Letras de Laja'), 'nombre': t('SOFÍA PÉREZ'), 'rit': t('X-10-2099'),
                'rut': t('11111111-K'), 'curador': t('(11111111-K)ANA EJEMPLO')}
        base.update(campos)
        return {'id': f'{etapa[:3]}-D{numero:02d}', 'etapa': etapa, 'corte': CORTE.isoformat(), 'desactivadas': [],
                'campos': base, 'bitacora': list(bitacora), 'duplicidad': duplicidad, 'acumulado': None,
                'fecha_hoja2': None,
                'historia': {'cobertura': 'COMPLETA', 'desde': (CORTE - timedelta(days=120)).isoformat(),
                             'hasta': CORTE.isoformat(), 'tramites': list(tramites)}}

    busqueda = [tr('Orden de búsqueda', -30, '10'), tr('Pide cuenta búsqueda', -20, '11'),
                tr('Pide cuenta búsqueda', -15, '12'), tr('Termino de búsqueda', -10, '13')]
    servicio = 'Ing.Serv. protección especializada y otras redes'
    if etapa == 'ESPERA':
        e = {'programa': t('PRM Laja'), 'espera': {'t': 'numero', 'v': 5}, 'resolucion': f(-5)}
        return [
            caso(1, e, [tr(servicio, -5, '7')]),
            caso(2, e, [tr(servicio, -5, '7'), tr(servicio, -5, '8')]),
            caso(3, e, [tr(servicio, -5, '7')] + busqueda, duplicidad='OTRO_REGISTRO'),
            caso(4, e, busqueda, [{'fecha': (CORTE - timedelta(days=9)).isoformat(),
                                   'texto': 'Consta término de la búsqueda en folio 13.'}]),
            caso(5, e, [tr('Pide cuenta búsqueda', -3, '40'), tr('Orden de búsqueda', -100, '2')]),
        ]
    if etapa == 'CUMPLIMIENTO':
        dce = {'programa': t('DCE Los Ángeles'), 'ingreso': f(-50), 'egreso_proy': f(-10),
               'dias_cumpl': {'t': 'numero', 'v': 50}, 'dias_egresar': {'t': 'numero', 'v': -10}}
        fae = {'programa': t('FAE PRO Biobío'), 'ingreso': f(-200), 'egreso_proy': f(100),
               'dias_cumpl': {'t': 'numero', 'v': 200}, 'dias_egresar': {'t': 'numero', 'v': 100},
               'ficha_fae': t('')}
        previa = [{'fecha': (CORTE - timedelta(days=40)).isoformat(), 'texto': 'Sofía no registra ficha FAE en RUS.'}]
        return [
            caso(1, dce, [tr('SPE. Resp. Solicitud DCE (escrito)', -20, '15')]),
            caso(2, dce, [tr('SPE. Resp. Solicitud DCE (escrito)', -20, '15')] + busqueda),
            caso(3, fae, [], previa),
            caso(4, fae, [tr('Prorroga', -15, '30'), tr('Inf. Cump FAE/FAS', -8, '31')], previa),
        ]
    dce = {'programa': t('DCE Los Ángeles'), 'vencimiento': f(-20)}
    gen = {'programa': t('PRM Laja'), 'vencimiento': f(-20)}
    return [
        caso(1, dce, [tr('Ingreso informe de cumplimiento X', -5, '21')]),
        caso(2, gen, [tr('Ingreso informe de cumplimiento X', -5, '21')], duplicidad='OTRO_REGISTRO'),
        caso(3, gen, [tr('Inf. Cump Programa Ambulatorio', -5, '22')]),
        caso(4, gen, [tr('Ingreso informe de cumplimiento X', -40, '9')]),
    ]


def main():
    cantidad = int(sys.argv[1]) if len(sys.argv) > 1 else 700
    semilla = int(sys.argv[2]) if len(sys.argv) > 2 else 20261112
    destino = RAIZ / 'rust' / 'casos'
    destino.mkdir(exist_ok=True)
    for indice, etapa in enumerate(('ESPERA', 'CUMPLIMIENTO', 'INFORMES')):
        gen = Gen(semilla + indice)
        casos = []
        for numero in range(cantidad):
            caso = gen.caso(etapa, numero)
            caso['esperado'] = esperado(caso)
            casos.append(caso)
        for caso in dirigidos(etapa):
            caso['esperado'] = esperado(caso)
            casos.append(caso)
        con_texto = sum(1 for c in casos if c['esperado']['observacion'])
        reglas = {r for c in casos for r in c['esperado']['reglas']}
        (destino / f'{etapa.lower()}.json').write_text(
            json.dumps(casos, ensure_ascii=False, indent=0, separators=(',', ':')), encoding='utf-8')
        print(f'{etapa}: {len(casos)} casos, {con_texto} con observación, {len(reglas)} reglas distintas')
    todas = {m + '.' + k for m, v in TEXTOS.items() if m != '_meta' for k in v}
    vistas = set()
    for etapa in ('espera', 'cumplimiento', 'informes'):
        for c in json.loads((destino / f'{etapa}.json').read_text(encoding='utf-8')):
            vistas.update(c['esperado']['reglas'])
    print('Reglas sin ningún caso:', sorted(todas - vistas))


if __name__ == '__main__':
    main()
