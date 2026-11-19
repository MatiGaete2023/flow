"""Oráculo del plan de integración 18-nov-2026: otro programa vigente, informe de avance pendiente en Escritos y
asociación explícita del informe con su proyección. Mismos datos ficticios para Python y Rust.

Uso (desde la raíz del repo):
    python3 rust/oraculo/generar_integracion.py [casos_por_etapa] [semilla]

Escribe rust/casos/integracion.json con casos al azar (base de generar_casos.py + evidencia nueva) y casos dirigidos.
`cargo test -p flow-motor` exige igualdad exacta con el motor Python.
"""

import json
import sys
from datetime import date, timedelta
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import generar_casos as G

CORTE = G.CORTE
TIPOS_ESCRITO = ['Inf. Cump Programa Ambulatorio', 'Inf. Cump FAE/FAS', 'Inf. Cump Centro Residencial', 'Escrito ficticio']
ESTADOS = ['Pendiente', 'Pendiente', 'pendiente ', 'Resuelto', 'Proveído']
EGRESOS = [('\u2212\u2212', 'SIN INFORMACION'), ('--', 'SIN INFORMACION'), ('--', 'SIN INFORMACION'), ('--', 'sin información'), ('01/05/2026', 'SIN INFORMACION'),
           ('--', 'TERMINO DE LA MEDIDA'), ('', ''), ('-', 'SIN INFORMACION'), ('--', '')]
PROGRAMAS_H = ['PRM Laja', 'prm  LAJA', 'FAE PRO Biobío', 'FAS Tomé', 'PRM Laja Norte', 'PDE Mulchén', 'PIE 24 Horas', 'RES Residencia Familiar']
FRASES = [
    'Se hace presente que {pn} cuenta con ingreso efectivo al programa {prog}.',
    '{pn} ingresó efectivamente al programa {prog}.',
    '{pn} no cuenta con ingreso efectivo al programa {prog}.',
    'Se coordinó con el programa {prog}.',
    'Se hace presente que Otra cuenta con ingreso efectivo al programa {prog}.',
    'Medida revisada, sin observaciones.',
    '{pn} informa que Beto cuenta con ingreso efectivo al programa {prog}.',
    'Beto informa que {pn} cuenta con ingreso efectivo al programa {prog}.',
]


class Gen2(G.Gen):
    def dia_iso(self, lo, hi):
        return (CORTE + timedelta(days=self.r.randint(lo, hi))).isoformat()

    def historico(self):
        if self.r.random() < 0.35:
            return None
        cobertura = self.r.choice(['COMPLETA'] * 5 + ['PARCIAL', 'FALLIDA', 'NO_SOLICITADA', 'VACIA_COMPROBADA'])
        identidad = self.r.choice(['VERIFICADA'] * 6 + ['INCIERTA'])
        filas = []
        for n in range(self.r.randint(0, 5)):
            egreso, causal = self.r.choice(EGRESOS)
            filas.append({'programa': self.r.choice(PROGRAMAS_H), 'fecha_egreso': egreso, 'causal_egreso': causal, 'fila': n + 1})
        leido = CORTE + timedelta(days=self.r.choice([0, 0, 0, -3, 4]))
        return {'cobertura': cobertura, 'leido': leido.isoformat(), 'identidad': identidad, 'filas': filas}

    def escritos(self, vencimiento):
        if self.r.random() < 0.35:
            return None
        cobertura = self.r.choice(['COMPLETA'] * 4 + ['PARCIAL', 'FALLIDA', 'NO_SOLICITADA', 'VACIA_COMPROBADA', 'IDENTIDAD_INCIERTA'])
        filas = []
        for n in range(self.r.randint(0, 3)):
            fila = {'tipo': self.r.choice(TIPOS_ESCRITO), 'estado': self.r.choice(ESTADOS), 'doc': f'D{n}', 'fila': n + 1}
            x = self.r.random()
            if vencimiento and x < 0.4:
                fila.update(vinculo='EXPLICITO', fecha_proyectada=(vencimiento + timedelta(days=self.r.choice([0, 0, 0, 30]))).isoformat())
            elif x < 0.55:
                fila.update(vinculo='AJENO')
            filas.append(fila)
        leido = CORTE + timedelta(days=self.r.choice([-20, -5, 0, 0, 7]))
        return {'cobertura': cobertura, 'leido': leido.isoformat(), 'filas': filas}

    def evidencia(self, vencimiento):
        if self.r.random() < 0.6:
            return None
        return {'escrito_historia_folio': self.r.choice(['', '', str(self.r.randint(1, 60))]),
                'resolucion_folio': self.r.choice(['', '', str(self.r.randint(1, 60))]),
                'calendario': self.r.choice(['PENDIENTE', '']),
                'folios_ajenos': self.r.choice([[], [], [str(self.r.randint(1, 60))]]),
                'fecha_proyectada': vencimiento.isoformat() if vencimiento else None}

    def bitacora_extra(self, caso):
        nombre = caso['campos'].get('nombre', {}).get('v', '')
        pn = (nombre.split() or ['Persona'])[0].capitalize() if nombre else 'Persona'
        extra = []
        for h in (caso.get('historico') or {}).get('filas', []):
            if self.r.random() < 0.25:
                frase = self.r.choice(FRASES).format(pn=pn, prog=h['programa'])
                extra.append({'fecha': self.dia_iso(-100, 5), 'texto': frase})
        return extra

    def caso2(self, etapa, numero):
        caso = self.caso(etapa, numero)
        caso['id'] = 'INT-' + caso['id']
        venc = None
        celda = caso['campos'].get('vencimiento')
        if celda and celda['t'] == 'fecha':
            venc = date.fromisoformat(celda['v'])
        caso['historico'] = self.historico() if etapa != 'INFORMES' else None
        caso['escritos'] = self.escritos(venc) if etapa != 'ESPERA' else None
        caso['evidencia'] = self.evidencia(venc) if etapa == 'INFORMES' else None
        caso['bitacora'] = caso['bitacora'] + self.bitacora_extra(caso)
        caso['consignados'] = [G_clave(h['programa']) for h in (caso['historico'] or {}).get('filas', [])
                               if self.r.random() < 0.1]
        caso['bitacora_parcial'] = self.r.random() < 0.15
        return caso


def G_clave(texto):
    return ' '.join(texto.lower().split())


def dirigidos():
    def t(v):
        return {'t': 'texto', 'v': v}

    def f(dias):
        return {'t': 'fecha', 'v': (CORTE + timedelta(days=dias)).isoformat()}

    def n(v):
        return {'t': 'numero', 'v': v}

    def base(etapa, numero, campos, **kw):
        comunes = {'tribunal': t('Juzgado de Letras de Laja'), 'nombre': t('SOFÍA PÉREZ'), 'rit': t('X-10-2099'),
                   'rut': t('11111111-K'), 'curador': t('(11111111-K)ANA EJEMPLO'), 'programa': t('PRM Laja')}
        comunes.update(campos)
        caso = {'id': f'INT-{etapa[:3]}-D{numero:02d}', 'etapa': etapa, 'corte': CORTE.isoformat(), 'desactivadas': [],
                'campos': comunes, 'historia': None, 'bitacora': [], 'duplicidad': None, 'acumulado': None,
                'fecha_hoja2': None}
        caso.update(kw)
        return caso

    vig = ('--', 'SIN INFORMACION')

    def hist(*filas, cobertura='COMPLETA', identidad='VERIFICADA'):
        return {'cobertura': cobertura, 'leido': CORTE.isoformat(), 'identidad': identidad,
                'filas': [{'programa': p, 'fecha_egreso': e, 'causal_egreso': c, 'fila': i + 1} for i, (p, e, c) in enumerate(filas)]}

    def esc(*filas, cobertura='COMPLETA', leido=0, discordante=False):
        salida = {'cobertura': cobertura, 'leido': (CORTE + timedelta(days=leido)).isoformat(),
                  'filas': [{'tipo': a, 'estado': b, 'fila': i + 1, **(x[0] if x else {})} for i, (a, b, *x) in enumerate(filas)]}
        return {**salida, 'discordante': True} if discordante else salida

    cum = {'ingreso': f(-200), 'egreso_proy': f(100), 'dias_cumpl': n(200), 'dias_egresar': n(100)}
    amb = 'Inf. Cump Programa Ambulatorio'
    vencimientos = {'vencimiento': f(-30)}
    historia = {'cobertura': 'COMPLETA', 'desde': (CORTE - timedelta(days=120)).isoformat(), 'hasta': CORTE.isoformat()}

    def con_historia(*tramites, hasta=0):
        h = dict(historia, hasta=(CORTE + timedelta(days=hasta)).isoformat())
        h['tramites'] = [{'desc': d, 'fecha': (CORTE + timedelta(days=dias)).isoformat(), 'folio_txt': fol, 'tipo': ''}
                         for d, dias, fol in tramites]
        return h

    casos = [
        base('ESPERA', 1, {'espera': n(5), 'resolucion': f(-5)}, historico=hist(('PRM Laja', *vig), ('PRM Laja', *vig), ('FAE X', *vig), ('FAE X', *vig), ('PDE Y', '01/05/2026', 'EGRESO'))),
        base('CUMPLIMIENTO', 2, cum, historico=hist(('PRM Laja', *vig), ('FAE X', '01/05/2025', 'EGRESO'), ('FAE X', *vig))),
        base('CUMPLIMIENTO', 3, cum, historico=hist(('FAE X', '--', 'EGRESO'), ('PIE Y', '01/05/2026', 'SIN INFORMACION'))),
        base('CUMPLIMIENTO', 4, cum, historico=hist(('FAE X', *vig), ('PIE Y', *vig))),
        base('CUMPLIMIENTO', 5, cum, historico=hist(('PRM Laja Norte', *vig), ('prm  laja', *vig))),
        base('CUMPLIMIENTO', 6, cum, historico=hist(('FAE X', *vig)),
             bitacora=[{'fecha': (CORTE - timedelta(days=9)).isoformat(), 'texto': 'Sofía PRM: Se hace presente que Sofía cuenta con ingreso efectivo al programa FAE X.'}]),
        base('CUMPLIMIENTO', 7, cum, historico=hist(('FAE X', *vig)),
             bitacora=[{'fecha': (CORTE - timedelta(days=9)).isoformat(), 'texto': 'Sofía no cuenta con ingreso efectivo al programa FAE X.'}]),
        base('CUMPLIMIENTO', 8, cum, historico=hist(('FAE X', *vig)), consignados=['fae x']),
        base('CUMPLIMIENTO', 9, cum, historico=hist(('FAE X', *vig)), bitacora_parcial=True),
        base('CUMPLIMIENTO', 10, cum, historico=hist(('FAE X', *vig)),
             bitacora=[{'fecha': (CORTE - timedelta(days=9)).isoformat(), 'texto': 'Se coordinó con el programa FAE X.'}]),
        base('CUMPLIMIENTO', 11, cum, historico=hist(('FAE X', *vig), cobertura='FALLIDA')),
        base('CUMPLIMIENTO', 12, cum, historico=hist(('FAE X', *vig), identidad='INCIERTA')),
        base('CUMPLIMIENTO', 13, {**cum, 'nombre': t('')}, historico=hist(('FAE X', *vig))),
        base('CUMPLIMIENTO', 14, cum, escritos=esc((amb, 'Pendiente'))),
        base('CUMPLIMIENTO', 15, cum, escritos=esc((amb, 'Pendiente'), (amb, 'Pendiente'))),
        base('INFORMES', 16, vencimientos, escritos=esc((amb, 'Pendiente'))),
        base('INFORMES', 17, {**vencimientos, 'programa': t('FAE PRO Biobío')}, escritos=esc(('Inf. Cump FAE/FAS', 'Pendiente'))),
        base('INFORMES', 18, {**vencimientos, 'programa': t('RES Residencia Familiar')}, escritos=esc(('Inf. Cump Centro Residencial', 'Pendiente'))),
        base('INFORMES', 19, vencimientos, escritos=esc(('Inf. Cump FAE/FAS', 'Pendiente'))),
        base('INFORMES', 20, vencimientos, escritos=esc((amb, 'Resuelto'))),
        base('INFORMES', 21, vencimientos, escritos=esc((amb, 'Pendiente'), (amb, 'Pendiente'))),
        base('INFORMES', 22, vencimientos, escritos=esc((amb, 'Pendiente'), leido=-10), historia=con_historia((amb, -20, '30'))),
        base('INFORMES', 23, vencimientos, escritos=esc((amb, 'Pendiente'), leido=0), historia=con_historia((amb, -20, '30'))),
        base('INFORMES', 24, vencimientos, historia=con_historia(('Ingreso informe de cumplimiento X', -20, '58'))),
        base('INFORMES', 25, vencimientos, historia=con_historia((amb, -20, '30'))),
        base('INFORMES', 26, vencimientos, acumulado=[3, (CORTE - timedelta(days=90)).isoformat()],
             escritos=esc((amb, 'Pendiente', {'vinculo': 'EXPLICITO', 'fecha_proyectada': (CORTE - timedelta(days=30)).isoformat()}))),
        base('INFORMES', 27, vencimientos, acumulado=[3, (CORTE - timedelta(days=90)).isoformat()],
             historia=con_historia((amb, -20, '30'), ('Ingreso informe de cumplimiento X', -10, '41')),
             evidencia={'escrito_historia_folio': '30'}),
        base('INFORMES', 28, vencimientos, acumulado=[3, (CORTE - timedelta(days=90)).isoformat()],
             historia=con_historia(('Ingreso informe de cumplimiento X', -10, '41')),
             evidencia={'resolucion_folio': '41', 'calendario': 'PENDIENTE', 'fecha_proyectada': (CORTE - timedelta(days=30)).isoformat()}),
        base('INFORMES', 29, vencimientos, acumulado=[3, (CORTE - timedelta(days=90)).isoformat()]),
        # Correcciones E3 (19-nov-2026).
        base('INFORMES', 30, vencimientos, escritos=esc((amb, 'Pendiente', {'vinculo': 'AJENO'}))),
        base('CUMPLIMIENTO', 31, cum, escritos=esc((amb, 'Pendiente', {'vinculo': 'AJENO'}))),
        base('INFORMES', 32, vencimientos, escritos=esc((amb, 'Pendiente'), cobertura='PARCIAL')),
        base('INFORMES', 33, vencimientos, escritos=esc((amb, 'Pendiente'), cobertura='IDENTIDAD_INCIERTA')),
        base('INFORMES', 34, vencimientos, escritos=esc((amb, 'Pendiente', {'vinculo': 'EXPLICITO', 'fecha_proyectada': (CORTE - timedelta(days=30)).isoformat()}), cobertura='PARCIAL')),
        base('INFORMES', 35, vencimientos, escritos=esc((amb, 'Pendiente'), leido=10)),
        base('CUMPLIMIENTO', 36, cum, historico={**hist(('FAE X', *vig)), 'leido': (CORTE + timedelta(days=10)).isoformat()}),
        base('INFORMES', 37, vencimientos, historia=con_historia(('Ingreso informe de cumplimiento X', -10, '41')),
             evidencia={'folios_ajenos': ['41'], 'fecha_proyectada': (CORTE - timedelta(days=30)).isoformat()}),
        base('INFORMES', 38, vencimientos, historia=con_historia((amb, -25, '30'), ('Ingreso informe de cumplimiento X', -10, '41')),
             escritos=esc((amb, 'Pendiente', {'vinculo': 'EXPLICITO', 'fecha_proyectada': (CORTE - timedelta(days=30)).isoformat()})),
             evidencia={'escrito_historia_folio': '30', 'resolucion_folio': '41', 'fecha_proyectada': (CORTE - timedelta(days=30)).isoformat()}),
        {**base('INFORMES', 39, vencimientos, escritos=esc((amb, 'Pendiente'))), 'desactivadas': ['HISTORIA.INFORME_AVANCE_PENDIENTE']},
        base('CUMPLIMIENTO', 40, cum, historico=hist(('FAE X', *vig)),
             bitacora=[{'fecha': (CORTE - timedelta(days=9)).isoformat(), 'texto': 'Sofía informa que Beto cuenta con ingreso efectivo al programa FAE X.'}]),
        base('CUMPLIMIENTO', 41, cum, historico=hist(('FAE X', '\u2212\u2212', 'SIN INFORMACION'))),
        # E3C-01: lecturas simultáneas discordantes; el vínculo explícito de una de ellas no retira el pide cuenta.
        base('INFORMES', 42, vencimientos, escritos=esc((amb, 'Pendiente', {'vinculo': 'EXPLICITO', 'fecha_proyectada': (CORTE - timedelta(days=30)).isoformat()}), cobertura='PARCIAL', discordante=True)),
        base('CUMPLIMIENTO', 43, cum, escritos=esc((amb, 'Pendiente'), cobertura='PARCIAL', discordante=True)),
    ]
    return casos


def main():
    cantidad = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    semilla = int(sys.argv[2]) if len(sys.argv) > 2 else 20261118
    casos = []
    for indice, etapa in enumerate(('ESPERA', 'CUMPLIMIENTO', 'INFORMES')):
        gen = Gen2(semilla + indice)
        casos += [gen.caso2(etapa, i) for i in range(cantidad)]
    casos += dirigidos()
    for caso in casos:
        caso.setdefault('historico', None)
        caso.setdefault('escritos', None)
        caso.setdefault('evidencia', None)
        caso.setdefault('consignados', [])
        caso.setdefault('bitacora_parcial', False)
        caso['esperado'] = G.esperado(caso)
    destino = G.RAIZ / 'rust' / 'casos' / 'integracion.json'
    destino.write_text(json.dumps(casos, ensure_ascii=False, indent=0, separators=(',', ':')), encoding='utf-8', newline='\n')
    nuevas = ('HISTORIA.OTRO_PROGRAMA', 'HISTORIA.INFORME_AVANCE_PENDIENTE', 'HISTORIA.INFORME_REVISAR_ASOCIACION')
    vistas = {r for c in casos for r in c['esperado']['reglas']}
    marcas = sorted({m['codigo'] for c in casos for m in c['esperado']['marcas']})
    print(f'{len(casos)} casos; reglas nuevas cubiertas: {[r for r in nuevas if r in vistas]}; sin cubrir: {[r for r in nuevas if r not in vistas]}')
    print('Marcas:', marcas)


if __name__ == '__main__':
    main()
