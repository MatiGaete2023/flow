"""E-Q4 · Oráculo de grabación del lote de bitácoras de Flow Python (`descargador/lectura_bitacoras.py::DiaryRunner`).

Ejecuta `DiaryRunner` con el puente ficticio de `descargador/test_lectura_bitacoras.py` (SITFA ficticio, sin red) y
graba, por escenario, cada orden al puente con su respuesta, el manifiesto `bitacoras.json` resultante y las huellas
de las instantáneas HTML. La prueba Rust reproduce esas respuestas y compara órdenes y manifiesto.

Uso (desde la raíz): python app/qa/generar_bitacoras.py [salida.json]   (por omisión app/tests/datos/bitacoras.json)
Campos no deterministas declarados: `fecha` del manifiesto y la carpeta del lote (marca de tiempo local).
"""

import hashlib
import json
import sys
import tempfile
import threading
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'descargador'), str(RAIZ / 'src'), str(RAIZ / 'tests')]

from csmp_shared import parser  # noqa: E402

parser()

import base64  # noqa: E402
from datetime import datetime  # noqa: E402
from zoneinfo import ZoneInfo  # noqa: E402

import motor  # noqa: E402

# Reloj fijo: carpetas, `fecha` y la ventana de reutilización (12 h del mismo día local) quedan deterministas. La
# captura ficticia es 2026-10-08T12:00:00Z (09:00 en Santiago).
AHORA = datetime(2026, 10, 8, 13, 0, 0, 123456, tzinfo=ZoneInfo('America/Santiago'))
motor.now = lambda: AHORA

from lectura_bitacoras import DiaryRunner  # noqa: E402
from export_csmp_capture import export_capture  # noqa: E402
from lotes import Batch  # noqa: E402
from nurus.capture import read_capture  # noqa: E402
from resultados import load_lot  # noqa: E402
from test_lectura_bitacoras import Bridge  # noqa: E402
from test_lotes import CATALOG  # noqa: E402


from test_calendario_lectura import CalendarBridge, calendar_page  # noqa: E402


def grabador(base):
    class Grabador(base):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            self.guion = []

        def call(self, command, payload=None, timeout=75):
            try:
                respuesta = super().call(command, payload, timeout)
            except Exception as e:  # el error del puente también es parte del guion
                self.guion.append({'comando': command, 'payload': payload, 'error': str(e)})
                raise
            self.guion.append({'comando': command, 'payload': payload, 'respuesta': respuesta})
            return respuesta

    return Grabador


Grabador = grabador(Bridge)


class CalendarioConFilas(CalendarBridge):
    def __init__(self, fila=None, **opciones):
        super().__init__(**opciones)
        self.fila = fila

    def call(self, command, payload=None, timeout=75):
        respuesta = super().call(command, payload, timeout)
        if command == 'calendar_open' and self.fila is not None and payload['args'][10] == '202':
            causa, _, tribunal, *_, ingreso, _ = payload['args']
            datos = calendar_page(causa, tribunal, ingreso, (self.fila,)).encode('utf-8')
            respuesta = dict(respuesta, body=base64.b64encode(datos).decode(), digest=hashlib.sha256(datos).hexdigest())
        return respuesta


GrabadorCalendario = grabador(CalendarioConFilas)

HISTORIAL = (RAIZ / 'tests/fixtures_integracion/historial_menores_ficticio.html').read_bytes().replace(b'\r\n', b'\n')
SIN_TABLA = (b'<html><head><title>Historial Menores</title></head><body>'
             b'<iframe src="/SITFAWEB/x.jsp?RUT=1"></iframe></body></html>')


def rut_invalido(cuerpo):
    """RUT ficticio con dígito verificador INVÁLIDO a propósito (nunca una identidad real)."""
    suma, factor = 0, 2
    for d in reversed(str(cuerpo)):
        suma += int(d) * factor
        factor = 2 if factor == 7 else factor + 1
    valido = {11: '0', 10: 'K'}.get(11 - suma % 11, str(11 - suma % 11))
    return f'{cuerpo}-' + next(c for c in '0123456789' if c != valido)


class ConHistorico(Bridge):
    """`test_lectura_bitacoras`: histórico ficticio (`historico_open`), con falla del puente o ventana sin tabla."""

    def __init__(self, modo='ok'):
        super().__init__()
        self.modo = modo

    def call(self, command, payload=None, timeout=75):
        if command == 'historico_open':
            if self.modo == 'falla':
                raise motor.PocError('HISTORICO_LINK')
            datos = SIN_TABLA if self.modo == 'sin_tabla' else HISTORIAL
            return {'status': 200, 'type': 'text/html', 'body': base64.b64encode(datos).decode(),
                    'digest': hashlib.sha256(datos).hexdigest(), 'params': payload['params'],
                    'startedAt': '2026-10-08T12:00:00Z'}
        resultado = super().call(command, payload, timeout)
        if command == 'search' and self.modo == 'sin_tabla':
            # Una persona distinta por consulta (RUT y persona_id), como en la prueba de suspensión de Python.
            bruto = base64.b64decode(resultado['body']).replace(b'11111111-1', rut_invalido(10000000 + self.queries).encode())
            bruto = bruto.replace(b"'300'", f"'{300 + self.queries}'".encode())
            resultado['body'] = base64.b64encode(bruto).decode()
        return resultado


GrabadorHistorico = grabador(ConHistorico)

import lectura_historia  # noqa: E402
from lectura_historia import LectorHistoria  # noqa: E402


class _FechaSantiago(datetime):
    """`LectorHistoria` expresa `leida` en la zona local (`astimezone()`); se fija la de Santiago para que el oráculo
    sea el mismo en cualquier máquina (Linux o Windows)."""

    @classmethod
    def fromisoformat(cls, texto):
        d = datetime.fromisoformat(texto)
        return cls(d.year, d.month, d.day, d.hour, d.minute, d.second, d.microsecond, tzinfo=d.tzinfo)

    def astimezone(self, tz=None):
        return datetime.astimezone(self, tz or ZoneInfo('America/Santiago'))


lectura_historia.datetime = _FechaSantiago
from test_lectura_historia import BridgeConHistoria  # noqa: E402


class HistoriaCorrecta(BridgeConHistoria):
    """Ventana de historia con el RIT y el tribunal visibles de la causa pedida (según el catálogo del lote)."""

    def call(self, command, payload=None, timeout=75):
        if command == 'history_open':
            from test_lectura_historia import FakeBridge, VENTANA

            self.events.append((command, None))
            p = payload['params']
            ventana = VENTANA.replace(b'\r\n', b'\n').replace(b'C-99999-2099', p['rit'].encode()).replace(
                b'Juzgado Canario de Prueba', CATALOG['tribunales'][p['tribunal']].encode('cp1252'))  # la ventana declara windows-1252
            return FakeBridge(respuesta=ventana).call(command, payload)
        return super().call(command, payload, timeout)


class HistoriaAjena(HistoriaCorrecta):
    """La ventana muestra otro RIT (`visible='rit'`) u otro tribunal (`visible='tribunal'`) que la causa pedida."""

    def __init__(self, visible):
        super().__init__()
        self.visible = visible

    def call(self, command, payload=None, timeout=75):
        respuesta = super().call(command, payload, timeout)
        if command == 'history_open':
            datos = base64.b64decode(respuesta['body'])
            if self.visible == 'rit':
                datos = datos.replace(payload['params']['rit'].encode(), b'X-9-2099')
            else:
                datos = datos.replace(CATALOG['tribunales'][payload['params']['tribunal']].encode('cp1252'),
                                      b'Juzgado Ficticio Otro')
            respuesta = dict(respuesta, body=base64.b64encode(datos).decode(), digest=hashlib.sha256(datos).hexdigest())
        return respuesta


GrabadorConHistoria = grabador(HistoriaCorrecta)
GrabadorHistoriaAjena = grabador(HistoriaAjena)


def historia_json(carpeta):
    """`historia/`: manifiesto, diagnóstico y estructura anónima de Escritos, más las huellas de las ventanas."""
    salida = {}
    for nombre in ('historias.json', 'diagnostico_historia.json', 'estructura_escritos.json'):
        ruta = carpeta / 'historia' / nombre
        if ruta.exists():
            salida[nombre] = json.loads(ruta.read_text(encoding='utf-8'))
    salida['ventanas'] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                          for p in sorted((carpeta / 'historia').glob('historia-*.html'))}
    return salida


def historico_json(carpeta):
    """`historico.json`, la estructura anónima y los respaldos; `leida` se declara en las entradas sin respaldo."""
    salida = {}
    ruta = carpeta / 'historico.json'
    if ruta.exists():
        datos = json.loads(ruta.read_text(encoding='utf-8'))
        for entrada in datos['capturas'].values():
            if not entrada.get('archivo'):  # rama de excepción: `leida` es datetime.now() (no determinista)
                entrada['leida'] = '<LEIDA>'
        salida['historico.json'] = datos
    estructura = carpeta / 'historico' / 'estructura_historico.json'
    if estructura.exists():
        salida['estructura_historico.json'] = json.loads(estructura.read_text(encoding='utf-8'))
    salida['respaldos'] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                           for p in sorted((carpeta / 'historico').glob('historico-*.html'))}
    return salida


ESCENARIOS = {
    'basico': (dict(), Batch(('49',), ('2',), ('Cumplimiento',))),
    'identidad_ajena': (dict(wrong=True), Batch(('49',), ('2',), ('Cumplimiento',))),
    'sin_vinculo': (dict(missing=True), Batch(('49',), ('2',), ('Cumplimiento',))),
    'cuatro_modalidades': (dict(), Batch(tuple(CATALOG['tribunales']), ('1', '2', '3', '4'), ('Espera', 'Cumplimiento'))),
}


def escribir_json(ruta, valor):
    ruta.write_bytes(json.dumps(valor, ensure_ascii=False).encode('utf-8'))


def lote_para_paquete(base):
    seleccion = {'screen': 'seguimiento', 'tribunal': '49', 'modality': '2', 'tab': 'Cumplimiento'}
    consulta = {'estado': 'SIN_RESULTADOS', 'seleccion': seleccion, 'verificacion': 'verificacion 001.json'}
    archivos = {
        'resumen.json': {'estado': 'VALIDADA', 'consultas_esperadas': 1, 'consultas': [consulta],
                         'plan': [seleccion], 'fecha_hora': AHORA.isoformat()},
        consulta['verificacion']: {'estado': 'SIN_RESULTADOS', 'seleccion': seleccion,
                                   'archivos': [], 'paginas_esperadas': 0},
    }
    base.mkdir()
    for nombre, valor in archivos.items():
        escribir_json(base / nombre, valor)
    return archivos


def cobertura_paquete(base, manifiesto, historico):
    listados = base / 'listados'
    archivos = lote_para_paquete(listados)
    ruta = export_capture([load_lot(listados)], base / 'paquete.zip', diaries=[manifiesto],
                          historico=manifiesto.parent if historico else None)
    captura = read_capture(ruta)
    tablas = ('consultas', 'registros', 'bitacoras', 'calendarios', 'historico_proteccion', 'incidencias')
    return {'archivos': archivos, 'cobertura': {t: captura.manifest['table_states'].get(t, 'NO_SOLICITADA') for t in tablas},
            'incidencias': [fila['code'] for fila in captura.rows('incidencias')]}


def mensual_incompleto():
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        seleccion = {'screen': 'calendario_informes', 'tribunal': '901', 'month': '11', 'year': '2026'}
        escribir_json(base / 'resumen.json', {'estado': 'INCOMPLETA', 'consultas': [], 'plan': [seleccion]})
        try:
            load_lot(base)
        except motor.PocError as error:
            return str(error)
        raise AssertionError('Python permitió empaquetar un calendario mensual sin completar')


def escenario(nombre, opciones, lote, calendarios=False, historico=None, historia=None, paquete=False):
    lector = None
    if historia:
        puente = GrabadorConHistoria() if historia == 'ok' else GrabadorHistoriaAjena(historia)
        lector = LectorHistoria(puente, lambda *a: None, threading.Event(), pause=0)
    elif historico:
        puente = GrabadorHistorico(historico)
    else:
        puente = (GrabadorCalendario if calendarios else Grabador)(**opciones)
    with tempfile.TemporaryDirectory() as tmp:
        corredor = DiaryRunner(puente, CATALOG, lambda *a: None, threading.Event(), pause=0, calendars=calendarios,
                               historico=bool(historico), historia=lector)
        resultado = corredor.run(lote, Path(tmp))
        historicos = historico_json(Path(resultado['folder']))
        historias = historia_json(Path(resultado['folder'])) if historia else None
        diagnostico = json.loads((Path(resultado['folder']) / 'diagnostico_bitacoras.json').read_text(encoding='utf-8'))
        carpeta = Path(resultado['manifest']).parent
        carpeta_nombre = carpeta.name
        manifiesto = json.loads(Path(resultado['manifest']).read_text(encoding='utf-8'))
        instantaneas = {
            p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(carpeta.glob('*.html'))
        }
        cobertura = cobertura_paquete(Path(tmp), Path(resultado['manifest']), historico) if paquete else None
    manifiesto['fecha'] = '<FECHA>'
    claves = ('estado', 'leidas', 'fallidas', 'consultas_fallidas', 'detalle', 'faltantes_por_tribunal',
              'calendarios_leidos', 'calendarios_fallidos', 'historicos')
    escenario = {
        'escenario': nombre,
        'calendarios': calendarios,
        'historico': bool(historico),
        'historicos': historicos,
        'historia': bool(historia),
        'historias': historias,
        'diagnostico': diagnostico,
        'ahora': AHORA.isoformat(),
        'carpeta': carpeta_nombre,
        'lote': {'tribunales': list(lote.tribunals), 'modalidades': list(lote.modalities), 'pestanas': list(lote.tabs)},
        'guion': puente.guion,
        'manifiesto': manifiesto,
        'instantaneas': instantaneas,
        'resultado': {k: resultado[k] for k in claves},
    }
    if cobertura is not None:
        escenario['paquete'] = cobertura
    return escenario


def carpeta_json(carpeta):
    """Archivos de una carpeta de lote (manifiesto e instantáneas) para reconstruirla en la prueba Rust."""
    archivos = {}
    for p in sorted(carpeta.iterdir()):
        if p.name == 'bitacoras.json':
            datos = p.read_text(encoding='utf-8').encode('utf-8')
        elif p.suffix == '.html':
            datos = p.read_bytes()
        else:
            continue
        archivos[p.name] = base64.b64encode(datos).decode()
    return archivos


def dos_corridas(nombre, primera, segunda, lote, retomar, cancelada=False, calendarios=False):
    """Primera corrida (estado inicial) y segunda grabada: retomando la misma carpeta o reutilizando otro lote."""
    clase = GrabadorCalendario if calendarios else Grabador
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        inicial = DiaryRunner(clase(**primera), CATALOG, lambda *a: None, threading.Event(), pause=0,
                              calendars=calendarios).run(lote, base)
        carpeta_inicial = Path(inicial['folder'])
        archivos = carpeta_json(carpeta_inicial)
        puente = clase(**segunda)
        if retomar:
            cancelar = threading.Event()
            if cancelada:
                cancelar.set()  # se cancela antes de volver a enumerar: lo leído debe quedar en anteriores_no_reenumeradas
            corredor = DiaryRunner(puente, CATALOG, lambda *a: None, cancelar, pause=0, calendars=calendarios)
            resultado = corredor.run(lote, base, resume=carpeta_inicial)
        else:
            AHORA_SEGUNDA = AHORA.replace(minute=30)
            motor.now = lambda: AHORA_SEGUNDA
            try:
                corredor = DiaryRunner(puente, CATALOG, lambda *a: None, threading.Event(), pause=0, reuse_base=base)
                resultado = corredor.run(lote, base)
            finally:
                motor.now = lambda: AHORA
        carpeta = Path(resultado['manifest']).parent
        manifiesto = json.loads(Path(resultado['manifest']).read_text(encoding='utf-8'))
        diagnostico = json.loads((carpeta / 'diagnostico_bitacoras.json').read_text(encoding='utf-8'))
        instantaneas = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(carpeta.glob('*.html'))}
    manifiesto['fecha'] = '<FECHA>'
    claves = ('estado', 'leidas', 'fallidas', 'consultas_fallidas', 'detalle', 'faltantes_por_tribunal',
              'calendarios_leidos', 'calendarios_fallidos')
    return {
        'escenario': nombre,
        'calendarios': calendarios,
        'lote': {'tribunales': list(lote.tribunals), 'modalidades': list(lote.modalities), 'pestanas': list(lote.tabs)},
        'inicial': {'carpeta': carpeta_inicial.name, 'archivos': archivos},
        'modo': 'retomar' if retomar else 'reutilizar',
        'diagnostico': diagnostico,
        'cancelada': cancelada,
        'ahora': (AHORA if retomar else AHORA.replace(minute=30)).isoformat(),
        'guion': puente.guion,
        'carpeta': carpeta.name,
        'manifiesto': manifiesto,
        'instantaneas': instantaneas,
        'resultado': {k: resultado[k] for k in claves},
    }


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'bitacoras.json'
    escenarios = [escenario(n, o, b) for n, (o, b) in ESCENARIOS.items()]
    uno = Batch(('49',), ('2',), ('Cumplimiento',))
    escenarios.append(dos_corridas('retomar_tras_falla', dict(wrong=True), dict(), uno, retomar=True))
    escenarios.append(dos_corridas('reutilizar_de_otro_lote', dict(), dict(), uno, retomar=False))
    escenarios.append(dos_corridas('retomar_cancelado', dict(wrong=True), dict(), uno, retomar=True, cancelada=True))
    escenarios.append(escenario('calendario', dict(), uno, calendarios=True))
    escenarios.append(escenario('calendario_ajeno', dict(wrong=('202',)), uno, calendarios=True))
    escenarios.append(dos_corridas('retomar_con_calendario', dict(wrong=('202',)), dict(), uno, retomar=True, calendarios=True))
    todos = Batch(tuple(CATALOG['tribunales']), ('2',), ('Cumplimiento',))
    escenarios.append(escenario('historico', dict(), todos, historico='ok'))
    escenarios.append(escenario('historico_falla', dict(), todos, historico='falla'))
    escenarios.append(escenario('historia', dict(), uno, historia='ok'))
    escenarios.append(escenario('historia_rit_distinto', dict(), uno, historia='rit'))
    escenarios.append(escenario('historia_tribunal_distinto', dict(), uno, historia='tribunal'))
    escenarios.append(escenario('historico_sin_tabla', dict(), Batch(tuple(CATALOG['tribunales']), ('1', '2', '3', '4'),
                                                                   ('Cumplimiento',)), historico='sin_tabla'))
    for nombre, fila in {
        'cal_pendiente_guiones': ('10/11/2026', '- - -', 'PENDIENTE'),
        'cal_enviado_guiones': ('10/11/2026', '- - -', 'ENVIADO'),
        'cal_proyeccion_guiones': ('- - -', '', 'PENDIENTE'),
        'cal_estado_desconocido': ('10/11/2026', '', 'ANULADO'),
    }.items():
        escenarios.append(escenario(nombre, dict(fila=fila), uno, calendarios=True, paquete=True))
    escenarios.append(escenario('historico_no_verificado', dict(), uno, historico='sin_tabla', paquete=True))
    datos = {'catalogo': CATALOG, 'ahora': AHORA.isoformat(), 'escenarios': escenarios,
             'error_mensual_incompleto': mensual_incompleto()}
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_bytes((json.dumps(datos, ensure_ascii=False, indent=1) + '\n').encode('utf-8'))
    for e in datos['escenarios']:
        print(e['escenario'], e['resultado']['estado'], len(e['guion']), 'órdenes', len(e['manifiesto']['registros']),
              'registros')


if __name__ == '__main__':
    main()
