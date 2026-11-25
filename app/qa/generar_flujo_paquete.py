"""E-Q9 · Oráculo del flujo «Consultar y crear paquete» (`descargador/flujo_paquete.py`).

Ejecuta `flujo_paquete.run` y `retomar` con un SITFA ficticio (listados, bitácoras y calendarios sintéticos, sin red) y
graba por caso: la secuencia de fases, el `respaldo/proceso.json` final (historial sin horas, carpetas normalizadas), el
mensaje de error exacto, el resultado y las tablas del ZIP. La prueba Rust corre el mismo mundo con su SITFA ficticio.

Mundo sintético (el mismo en ambos lados): una consulta de Seguimiento · Cumplimiento devuelve una página con tres
personas ficticias y sus enlaces de bitácora; cada bitácora trae una observación del centro y cada ingreso su calendario.
La historia y el histórico no se sirven (el puente devuelve error), así que el paquete queda parcial por esa vía.

Determinista: reloj fijo que avanza un segundo por llamada, JSON con LF, rutas relativas con `/` (también en
`proceso.json`), historial sin hora ni zona, entrada sin ZIP y la carpeta temporal tomada con `Path(tmp).resolve()`
(en Windows llega como `RUNNER~1`). Dos corridas y una carpeta de trabajo con espacios dan bytes idénticos.

Incluye por_hoja (E-Q10): un paquete por pestaña, con la hoja que falla, una sola hoja y la cancelación en la
segunda con su retomar. Uso (desde la raíz): python app/qa/generar_flujo_paquete.py [salida.json]
"""

import base64
import hashlib
import json
import re
import shutil
import sys
import tempfile
import threading
from datetime import date, datetime, timedelta
from pathlib import Path
from zoneinfo import ZoneInfo

RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'descargador'), str(RAIZ / 'src'), str(RAIZ / 'tests')]

from csmp_shared import parser  # noqa: E402

parser()

import generar_lote  # noqa: E402,F401  (reloj fijo y Path.write_text en LF)
import motor  # noqa: E402

AHORA = datetime(2026, 10, 8, 13, 0, 0, 123456, tzinfo=ZoneInfo('America/Santiago'))
_RELOJ = {'n': 0}


def ahora():
    """Reloj fijo que avanza un segundo por llamada: las carpetas de cada fase no chocan y todo sigue determinista."""
    _RELOJ['n'] += 1
    return AHORA + timedelta(seconds=_RELOJ['n'])


motor.now = ahora  # por encima del reloj fijo de generar_lote: las carpetas de cada fase no chocan

import flujo_paquete  # noqa: E402
from lotes import Batch  # noqa: E402
from motor import PocError  # noqa: E402
from nurus.capture import read_capture  # noqa: E402
from perfiles import Selection, selection_profile  # noqa: E402
import test_lotes  # noqa: E402

# Mismo catálogo que `descargador/sitfa_ficticio.rs`, para que los dos mundos coincidan.
CATALOGO = {
    'pantalla': 'seguimiento',
    'tribunales': {'901': 'Juzgado de Familia Ficticio Norte', '902': 'Juzgado de Familia Ficticio Sur'},
    'modalidades': {'1': 'Residencia', '2': 'Ambulatorio', '3': 'FAE', '4': 'DCE'},
    'informes': {'1': 'Por vencer (a 25 días)', '2': 'Vencidos', '3': 'Recibidos'},
    'medidas': {'4': 'Por vencer (a 45 días)', '5': 'Vencidos'},
    'pestanas': ['Espera', 'Cumplimiento', 'Informes', 'Egresados'],
}
MESES = ['Enero', 'Febrero', 'Marzo', 'Abril', 'Mayo', 'Junio', 'Julio', 'Agosto', 'Septiembre', 'Octubre',
         'Noviembre', 'Diciembre']
CATALOGO_CALENDARIO = {
    'pantalla': 'calendario_informes', 'tribunales': CATALOGO['tribunales'],
    'meses': {str(i + 1): n for i, n in enumerate(MESES)}, 'anios': {'2026': '2026', '2027': '2027'},
}
test_lotes.CATALOG = CATALOGO  # el mismo mundo que el SITFA ficticio de Rust
PERSONAS = [('X-1-2026', '11111111-0', 'Persona Ficticia 1'), ('X-2-2026', '11111112-0', 'Persona Ficticia 2'),
            ('X-3-2026', '11111113-0', 'Persona Ficticia 3')]
IDS = ['tablaEspera', 'tablaCumplimiento', 'tablaInforme', 'tablaEgresados']


def tabla(con_enlaces, tabla_id='tablaCumplimiento'):
    filas = ''
    for n, (rit, rut, nombre) in enumerate(PERSONAS, 1):
        causa, ingreso = str(100 + n), str(200 + n)
        acciones = ''
        if con_enlaces:
            acciones = (
                f"<a onclick=\"ShowHistoria('901','{rit}','{causa}')\">Causa</a>"
                f"<a onclick=\"ShowObservaciones('{causa}','901','{ingreso}','12','3','0')\">Obs</a>"
                f"<a onclick=\"ShowInformesProgramados('{causa}','300','901','400','{rit}','01/03/2026',"
                f"'PRM Centro Ficticio','{nombre}','6 Meses','3 Meses','{ingreso}','1')\">Inf</a>"
            )
        filas += (f'<tr><td>{rit}</td><td>{rut}</td><td>{nombre}</td><td>PRM Centro Ficticio</td>'
                  f'<td>{acciones}</td></tr>')
    return (f'<table id="{tabla_id}"><tr><td>RIT</td><td>RUT</td><td>Nombre</td><td>Derivacion</td>'
            f'<td>Acciones</td></tr>{filas}</table>')


def pagina_calendario():
    """Listado del calendario de informes: dos tablas y los RIT en el script, como SITFA."""
    filas = ''.join(f'<tr><td>{rit}</td><td>{nombre}</td></tr>' for rit, _, nombre in PERSONAS)
    tablas = ''.join(f'<table id="{i}">{filas}</table>' for i in ('TablaCalendarioSemana', 'TablaCalendarioDia'))
    scripts = ''.join(f'Ritxdias[dia] = "{rit}"; Nombresxdias[dia] = "{nombre}";' for rit, _, nombre in PERSONAS)
    return f'{tablas}<script>{scripts}</script>'


def documento(pares, pantalla, con_enlaces):
    """Documento de SITFA: formulario con los campos de la consulta, tablas y el script de descarga."""
    formulario = ''.join(f'<select name="{k}"><option value="{v}" selected>{v}</option></select>'
                         if k in ('COD_Tribunal_sel', 'TIP_Consulta', 'COD_Mes_sel', 'COD_Anio_Sel') else
                         f'<input name="{k}" value="{v}">' for k, v in pares.items())
    tablas = pagina_calendario() if pantalla.startswith('calendario_') else tabla(con_enlaces)
    return (f'<html><body><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">'
            f'{formulario}{tablas}</form>'
            '<script>function ShowExcel(){window.open("https://familia.pjud.cl/sitfa/reportes/demo.xls");}</script>'
            '</body></html>').encode()


def popup(params):
    """Bitácora sintética con una observación del centro; la identidad visible no siempre coincide."""
    from nurus.bitacora_html import HEADERS

    campos = {**params, 'tipo_popUp': '', 'RIT_Causa': PERSONAS[0][0], 'GLS_Nombre': 'Persona ficticia',
              'GLS_Centro': 'PRM Centro ficticio'}
    ocultos = {'HOBS_Centro_': 'Texto completo\nSegunda línea', 'HOBS_Tribunal_': '---', 'HCOD_Estado_': '6',
               'HFUN_Tribunal_': '', 'HFLG_Envio_': '1'}
    cabecera = ''.join(f'<td>{h}</td>' for h in HEADERS)
    fila = ('<td>Al Tribunal</td><td>Cumplimiento</td><td>12/08/2026 10:13</td><td>Usuario ficticio</td>'
            '<td>Texto recortado</td><td>---</td><td>---</td><td>---</td><td><img id="44"></td>')
    return ('<html><form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do">'
            + ''.join(f'<input type="hidden" name="{k}" value="{v}">' for k, v in campos.items())
            + f'<table id="TablaInforme"><tr>{cabecera}</tr><tr>{fila}</tr>'
            + ''.join(f'<input type="hidden" name="{k}44" value="{v}">' for k, v in ocultos.items())
            + '</table></form></html>').encode()


def calendario(args):
    campos = {'COD_Tribunal': args[2], 'COD_Tribunal_sel': args[2], 'CRR_IdCausa': args[0], 'ID_Ingreso': args[10]}
    formulario = ''.join(f'<input type="hidden" name="{k}" value="{v}">' for k, v in campos.items())
    filas = ('<tr><td>01/03/2026</td><td>15/03/2026</td><td>ENVIADO</td></tr>'
             '<tr><td>01/04/2026</td><td>---</td><td>PENDIENTE</td></tr>')
    return ('<html><form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do">'
            f'{formulario}<table><tr><td>Fecha Proyección</td><td>Fecha Informe</td><td>Estado</td></tr>{filas}</table>'
            '<span>Plazo remisión: 6 meses</span></form></html>').encode()


class Puente:
    """SITFA ficticio: listados, bitácoras y calendarios sintéticos; historia e histórico fallan a propósito."""

    def __init__(self, fallar_bitacora=None, fallar_hoja=None):
        self.pantalla = 'seguimiento'
        self.mes = None
        self.fallar_hoja = fallar_hoja
        self.seleccion = None
        self.pares = {}
        self.seleccion = None
        self.llamadas = []
        self.fallar_bitacora = fallar_bitacora

    def documento(self, con_enlaces):
        """Listado con el formulario y el enlace Excel que el perfil espera, y la tabla de tres personas."""
        if self.pantalla == 'seguimiento':
            base = test_lotes.document(self.seleccion, 1, 1).decode('utf-8')
            inicio, fin = base.index('<table'), base.index('</table>') + len('</table>')
            tabla_id = re.search(r'id="([^"]+)"', base[inicio:fin]).group(1)
            return (base[:inicio] + tabla(con_enlaces, tabla_id) + base[fin:]).encode()
        formulario = ''.join(f'<select name="{k}"><option value="{v}" selected>{v}</option></select>'
                             if k in ('COD_Tribunal_sel', 'TIP_Consulta', 'COD_Mes_sel', 'COD_Anio_Sel') else
                             f'<input name="{k}" value="{v}">' for k, v in self.pares.items())
        return (f'<html><body><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">'
                f'{formulario}{pagina_calendario()}</form>'
                '<script>function ShowExcel(){window.open("https://familia.pjud.cl/sitfa/reportes/u1_InfoPorVencer.xls");}'
                '</script></body></html>').encode()

    def call(self, command, payload=None, timeout=75):
        payload = payload or {}
        self.llamadas.append(command)
        if command == 'capabilities':
            from contrato_integral import capabilities

            return capabilities()
        if command == 'catalog':
            pantalla = payload.get('screen', 'seguimiento')
            self.pantalla = pantalla
            return dict(CATALOGO_CALENDARIO if pantalla.startswith('calendario_') else CATALOGO)
        if command in ('lock', 'unlock'):
            return {'ok': True}
        if command == 'prepare':
            crudo = payload['selection']
            self.pantalla = crudo['screen']
            self.mes = crudo.get('month')
            if self.fallar_hoja and crudo.get('tab') == self.fallar_hoja:
                raise PocError(f'La hoja {self.fallar_hoja} no se pudo preparar.')
            self.seleccion = Selection(**{k: crudo.get(k) for k in
                                          ('tribunal', 'tab', 'modality', 'report', 'screen', 'state', 'month', 'year')
                                          if k in crudo})
            catalogo = CATALOGO_CALENDARIO if self.pantalla.startswith('calendario_') else CATALOGO
            perfil = selection_profile(self.seleccion, catalogo)
            self.pares = dict(perfil.target)
            self.pares['irAccion'] = 'Buscar medida' if self.pantalla == 'seguimiento' else 'Buscar Inf.'
            if perfil.page_field:
                self.pares[perfil.page_field] = '1'
            if perfil.total_field:
                self.pares[perfil.total_field] = '1'
            if self.pantalla == 'seguimiento':
                self.pares['FLG_Consulta'] = '0'
            return {'pairs': [list(p) for p in self.pares.items()]}
        if command in ('search', 'evidence_open'):
            self.pares.update({k: v for k, v in payload['pairs']})
            cuerpo = self.documento(command == 'search')
            resultado = {'status': 200, 'type': 'text/html', 'body': base64.b64encode(cuerpo).decode()}
            if command == 'evidence_open':
                resultado['digest'] = hashlib.sha256(cuerpo).hexdigest()
            return resultado
        if command == 'download':
            if self.pantalla == 'seguimiento':
                cuerpo = ('<html><meta charset="utf-8">' + tabla(False) + '</html>').encode()
            else:
                mes = int(self.mes or '1')
                filas = ''.join(f'<tr><td>{rit}</td><td>{nombre}</td><td>15/{mes:02}/2026</td></tr>'
                                for rit, _, nombre in PERSONAS)
                cuerpo = ('<html><meta charset="utf-8"><table><tr><td>RIT</td><td>Nombre</td>'
                          f'<td>Fecha Vencimiento</td></tr>{filas}</table></html>').encode()
            return {'status': 200, 'type': 'application/vnd.ms-excel', 'body': base64.b64encode(cuerpo).decode()}
        if command == 'evidence_capture':
            raise PocError('Sin capturas en el oráculo.')
        if command == 'diary_open':
            params = payload['params']
            if self.fallar_bitacora and params['ID_Ingreso'] == self.fallar_bitacora:
                self.fallar_bitacora = None
                raise PocError('La bitácora no cargó.')
            cuerpo = popup(params)
            return {'status': 200, 'type': 'text/html', 'body': base64.b64encode(cuerpo).decode(),
                    'digest': hashlib.sha256(cuerpo).hexdigest(), 'params': params,
                    'startedAt': '2026-10-08T12:00:00Z'}
        if command == 'calendar_open':
            args = payload['args']
            cuerpo = calendario(args)
            return {'status': 200, 'type': 'text/html', 'body': base64.b64encode(cuerpo).decode(),
                    'digest': hashlib.sha256(cuerpo).hexdigest(), 'args': args,
                    'startedAt': '2026-10-08T12:00:00Z'}
        if command in ('history_open', 'historico_open'):
            raise PocError('El oráculo no sirve historia ni histórico.')
        return {'ok': True}


def normalizar(texto):
    """Horas fuera del historial y nombres de carpeta: la fecha del reloj fijo queda como <FECHA>."""
    texto = re.sub(r'^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[+-]\d{2}:\d{2} ', '', str(texto))
    return re.sub(r'\d{4}-\d{2}-\d{2} \d{2}-\d{2}-\d{2}(-\d{6})?', '<FECHA>', texto).replace('\\', '/')


def volcado_proceso(raiz):
    crudo = (raiz / 'respaldo' / 'proceso.json').read_text(encoding='utf-8')
    try:
        datos = json.loads(crudo)
    except ValueError:
        return {'invalido': True}
    if not isinstance(datos, dict):
        return {'invalido': True}
    return {
        'estado': datos.get('estado'), 'fase': datos.get('fase'), 'reintentos': datos.get('reintentos', 0),
        'paquete': normalizar(datos.get('paquete', '')),
        'carpetas': {k: normalizar(v) for k, v in (datos.get('carpetas') or {}).items()},
        'historial': [normalizar(h) for h in datos.get('historial', [])],
        'opciones': datos.get('opciones'),
    }


def volcado_zip(ruta):
    captura = read_capture(ruta)
    manifiesto = captura.manifest
    tablas = {tabla_nombre: len(captura.rows(tabla_nombre))
              for tabla_nombre in ('consultas', 'registros', 'bitacoras', 'calendarios', 'historia_tramites',
                                   'documentos', 'incidencias', 'escritos', 'historico_proteccion')}
    return {
        'nombre': Path(ruta).name,
        'tablas': tablas,
        'estados': manifiesto.get('table_states'),
        'consolidado': manifiesto.get('consolidated') is not None,
        'schema': manifiesto.get('schema_version'),
        'incidencias': sorted({fila['code'] for fila in captura.rows('incidencias')}),
    }


def carpeta_paquete(destino):
    return next(Path(destino).glob('Paquete CSMP *'))


def caso(nombre, *, opciones=None, batch=None, resume=None, fallar_bitacora=None, cancelar_tras_consultas=False,
         guardar_entrada=False):
    """Ejecuta un caso y devuelve lo observable: fases, proceso final, error, resultado y ZIP."""
    opciones = opciones or {}
    base = Path(tempfile.mkdtemp(prefix='flujo-')).resolve()
    registro = {'caso': nombre, 'entrada': None, 'opciones': opciones}
    resume = Path(str(resume)).resolve() if resume is not None else None
    entrada_previa = archivos(resume) if resume is not None else None
    puente = Puente(fallar_bitacora)
    eventos = []
    cancelar = threading.Event()
    if cancelar_tras_consultas:
        original = puente.call

        def cortar(command, payload=None, timeout=75):
            respuesta = original(command, payload, timeout)
            if command == 'download':
                cancelar.set()
            return respuesta

        puente.call = cortar
    try:
        if resume is None:
            destino = base / 'destino'
            destino.mkdir()
            resultado = flujo_paquete.run(puente, CATALOGO, lambda *e: eventos.append(e), cancelar,
                                          batch or Batch(('901',), ('2',), ('Cumplimiento',)), destino,
                                          pause=0, today=date(2026, 10, 14), **opciones)
        else:
            resultado = flujo_paquete.retomar(puente, CATALOGO, lambda *e: eventos.append(e), cancelar, resume,
                                             pause=0, today=date(2026, 10, 14), **opciones)
        ruta = Path(resultado['path'])
        registro['resultado'] = {'parcial': bool(resultado.get('partial')),
                                 'historia': (resultado.get('historia') or {}).get('estado'),
                                 'zip': ruta.name}
        registro['zip'] = volcado_zip(ruta)
        registro['error'] = None
    except (PocError, ValueError, OSError) as exc:
        registro['error'] = str(exc)
        registro['resultado'] = None
        registro['zip'] = None
        ruta = None
    registro['fases'] = [valor for nombre_evento, valor in eventos if nombre_evento == 'phase']
    registro['avisos'] = [valor for nombre_evento, valor in eventos if nombre_evento == 'warning']
    if resume is not None:
        paquete = resume
    else:
        paquetes = list((base / 'destino').glob('Paquete CSMP *'))
        paquete = ruta.parent if ruta else (paquetes[0] if paquetes else None)
    registro['proceso'] = volcado_proceso(paquete) if paquete else None
    respaldo = paquete / 'respaldo' if paquete else None
    registro['subcarpetas'] = sorted(normalizar(p.name) for p in respaldo.iterdir() if p.is_dir()) \
        if respaldo and respaldo.is_dir() else []
    registro['raiz'] = normalizar(paquete.name) if paquete else ''
    registro['retomar'] = resume is not None
    registro['otra_pantalla'] = (batch or Batch(('901',), ('2',), ('Cumplimiento',))).screen != 'seguimiento'
    if resume is not None:
        # La carpeta de entrada se guarda antes de retomarla: retomar la completa en el lugar.
        registro['entrada'] = entrada_previa
    elif guardar_entrada:
        registro['entrada'] = archivos(paquete)
    return registro


def rutas_normalizadas(valor):
    """`proceso.json` guarda `respaldo\\<fecha>` en Windows: las rutas relativas van con `/` en cualquier sistema."""
    if isinstance(valor, str):
        if '\\' in valor and ('respaldo\\' in valor or valor.startswith('Paquete CSMP ')):
            return valor.replace('\\', '/')
        return valor
    if isinstance(valor, list):
        return [rutas_normalizadas(v) for v in valor]
    if isinstance(valor, dict):
        salida = {k: rutas_normalizadas(v) for k, v in valor.items()}
        # notar de Python usa stimezone(): la hora del historial cambia con la zona del sistema. Se fija aquí.
        if isinstance(salida.get('historial'), list):
            salida['historial'] = [normalizar(h) for h in salida['historial']]
        return salida
    return valor


def archivos(carpeta):
    """Archivos de la carpeta del paquete como base64: JSON normalizado (LF y rutas con `/`), sin el ZIP."""
    salida = {}
    for ruta in sorted(Path(carpeta).rglob('*')):
        if ruta.is_file() and ruta.suffix in ('.json', '.xls'):
            if ruta.suffix == '.json':
                try:
                    datos = json.dumps(rutas_normalizadas(json.loads(ruta.read_text(encoding='utf-8'))), ensure_ascii=False, indent=2).replace('\r\n', '\n').encode('utf-8')
                except ValueError:
                    datos = ruta.read_bytes().replace(b'\r\n', b'\n')
            else:
                datos = ruta.read_bytes()
            salida[ruta.relative_to(Path(carpeta).parent).as_posix()] = base64.b64encode(datos).decode()
    return salida


def volcado_resultado(resultado):
    """Lo observable de un paquete del flujo por hoja: resultado, proceso final y ZIP."""
    ruta = Path(resultado['path'])
    return {
        'resultado': {'parcial': bool(resultado.get('partial')),
                      'historia': (resultado.get('historia') or {}).get('estado'),
                      'zip': ruta.name},
        'proceso': volcado_proceso(ruta.parent),
        'zip': volcado_zip(ruta),
    }


def caso_por_hoja(nombre, hojas, *, opciones=None, fallar_hoja=None, cancelar_en_hoja=None):
    """Un `run_por_hoja` completo: fases globales, error y un volcado por paquete; si se cancela, la entrada cortada."""
    opciones = opciones or {}
    base = Path(tempfile.mkdtemp(prefix='flujo-')).resolve()
    destino = base / 'destino'
    destino.mkdir()
    puente = Puente(fallar_hoja=fallar_hoja)
    eventos = []
    cancelar = threading.Event()
    registro = {'caso': nombre, 'hojas': list(hojas), 'error': None, 'paquetes': []}
    if cancelar_en_hoja:
        original = puente.call

        def cortar(command, payload=None, timeout=75):
            if command == 'download' and puente.seleccion is not None and puente.seleccion.tab == cancelar_en_hoja:
                cancelar.set()
            return original(command, payload, timeout)

        puente.call = cortar
    try:
        resultados = flujo_paquete.run_por_hoja(
            puente, CATALOGO, lambda *e: eventos.append(e), cancelar,
            Batch(('901',), ('2',), tuple(hojas)), destino, pause=0, today=date(2026, 10, 14), **opciones)
        registro['paquetes'] = [volcado_resultado(r) for r in resultados]
    except (PocError, ValueError, OSError) as exc:
        registro['error'] = str(exc)
        paquetes = sorted(p for p in destino.glob('Paquete CSMP *') if p.is_dir())
        registro['paquetes'] = [{'resultado': None, 'proceso': volcado_proceso(p), 'zip': None} for p in paquetes]
        cortado = paquetes[-1] if paquetes else None
        if cortado:
            registro['entrada'] = archivos(cortado)
            registro['retomado'] = caso('retomado_por_hoja', resume=Paquete_de(cortado))
    registro['fases'] = [v for n, v in eventos if n == 'phase']
    registro['avisos'] = [v for n, v in eventos if n == 'warning']
    return registro


class Paquete_de:
    """Copia de una carpeta Â«Paquete CSMP â€¦Â» ya existente, para retomarla (sin ZIP y con `proceso.json` normalizado)."""

    def __init__(self, carpeta):
        self.carpeta = Path(tempfile.mkdtemp(prefix='flujo-entrada-')).resolve()
        destino = self.carpeta / Path(carpeta).name
        shutil.copytree(carpeta, destino)
        for zip_viejo in destino.glob('*.zip'):
            zip_viejo.unlink()
        control = destino / 'respaldo' / 'proceso.json'
        control.write_bytes(json.dumps(rutas_normalizadas(json.loads(control.read_text(encoding='utf-8'))),
                                       ensure_ascii=False, indent=2).encode('utf-8'))
        self.ruta = destino

    def __str__(self):
        return str(self.ruta)


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'flujo_paquete.json'
    completo = caso('todo_completo')
    sin_calendarios = caso('sin_calendarios', opciones={'calendars': False})
    con_calendarios = caso('con_calendarios', opciones={'calendars': True})
    con_informes = caso('con_informes', opciones={'informes': True})
    con_historia = caso('con_historia_y_historico', opciones={'historia': True, 'historico': True})
    cancelado = caso('cancelado_tras_consultas', cancelar_tras_consultas=True, guardar_entrada=True)
    retomado = caso('retomado', resume=Paquete(cancelado))
    fallida = caso('bitacora_fallida', opciones={'consolidated': False}, fallar_bitacora='201', guardar_entrada=True)
    retomada = caso('bitacora_retomada', resume=Paquete(fallida), opciones={'consolidated': False})
    danado = caso('proceso_danado', resume=Paquete(cancelado, danado=True))
    sin_alcance = caso('proceso_sin_alcance', resume=Paquete(cancelado, sin_alcance=True))
    pantalla = caso('otra_pantalla', batch=Batch(('901',), (), (), screen='calendario_informes'))
    casos = [completo, sin_calendarios, con_calendarios, con_informes, con_historia, cancelado, retomado, fallida,
             retomada, danado, sin_alcance, pantalla]
    destino.parent.mkdir(parents=True, exist_ok=True)
    por_hoja = [
        caso_por_hoja('dos_hojas', ('Cumplimiento', 'Espera')),
        caso_por_hoja('una_hoja_falla', ('Cumplimiento', 'Espera'), fallar_hoja='Espera'),
        caso_por_hoja('una_sola_hoja', ('Cumplimiento',)),
        caso_por_hoja('cancelada_y_retomada', ('Cumplimiento', 'Espera'), cancelar_en_hoja='Espera'),
    ]
    destino.write_bytes((json.dumps({'catalogo': CATALOGO, 'casos': casos, 'por_hoja': por_hoja}, ensure_ascii=False, indent=1) + '\n').encode('utf-8'))
    for c in casos:
        print(c['caso'], '->', c['error'] or (c['fases'], (c['resultado'] or {}).get('parcial')))
    for c in por_hoja:
        print('por hoja', c['caso'], '->', c['error'] or len(c['paquetes']), 'paquete(s)')


class Paquete:
    """Carpeta «Paquete CSMP …» de un caso anterior, para retomarla o alterarla."""

    def __init__(self, registro, danado=False, sin_alcance=False):
        self.carpeta = Path(tempfile.mkdtemp(prefix='flujo-entrada-')).resolve()
        for ruta, b64 in (registro.get('entrada') or {}).items():
            p = self.carpeta / ruta
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(base64.b64decode(b64))
        self.ruta = next(self.carpeta.glob('Paquete CSMP *'))
        for zip_viejo in self.ruta.glob('*.zip'):
            zip_viejo.unlink()
        control = self.ruta / 'respaldo' / 'proceso.json'
        if control.is_file():
            control.write_bytes(
                json.dumps(rutas_normalizadas(json.loads(control.read_text(encoding='utf-8'))), ensure_ascii=False, indent=2).encode('utf-8')
            )
        if danado:
            control.write_bytes(b'{ esto no es JSON')
        elif sin_alcance:
            proceso = json.loads(control.read_text(encoding='utf-8'))
            proceso.pop('seleccion', None)
            control.write_bytes(json.dumps(proceso, ensure_ascii=False, indent=1).encode('utf-8'))

    def __str__(self):
        return str(self.ruta)


if __name__ == '__main__':
    main()
