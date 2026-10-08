"""Puente local. No conoce cookies ni contrasenas; tareas y codigo solo en RAM."""
import hmac
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
import json
import queue
import re
import secrets
import threading
import time

from motor import PocError


class Bridge:
    def __init__(self,emit=lambda *args:None,port=0):
        self.emit=emit;self.code=secrets.token_hex(16);self.origin=None
        self.tasks=queue.Queue();self.pending={};self.lock=threading.Lock();self.closed=False
        self.last_seen=0
        parent=self
        class Handler(BaseHTTPRequestHandler):
            def log_message(self,*args):pass
            def cors_origin(self):
                origin=self.headers.get('Origin','')
                return origin if re.fullmatch(r'chrome-extension://[a-p]{32}',origin) else None
            def extension_origin(self):
                # Chrome puede omitir Origin en sus peticiones privilegiadas.
                # La identidad declarada nunca sustituye el codigo secreto.
                declared=self.headers.get('X-Sitfa-Extension','')
                origin=self.headers.get('Origin')
                if declared and not re.fullmatch(r'[a-p]{32}',declared):return None
                if origin is not None:
                    if not self.cors_origin():return None
                    if declared and origin!='chrome-extension://'+declared:return None
                    return origin
                return 'chrome-extension://'+declared if declared else None
            def reply(self,status,data):
                payload=json.dumps(data,ensure_ascii=False).encode()
                self.send_response(status)
                origin=self.cors_origin()
                if origin:self.send_header('Access-Control-Allow-Origin',origin)
                self.send_header('Vary','Origin');self.send_header('Cache-Control','no-store')
                self.send_header('Content-Type','application/json; charset=utf-8')
                self.send_header('Content-Length',str(len(payload)));self.end_headers()
                try:self.wfile.write(payload)
                except (BrokenPipeError,ConnectionResetError):pass
            def authorized(self):
                origin=self.extension_origin()
                code=self.headers.get('X-Sitfa-Code','')
                if not origin:
                    self.deny('EXTENSION_ORIGIN');return False
                if not re.fullmatch(r'[0-9a-f]{32}',code) or not hmac.compare_digest(code,parent.code):
                    self.deny('CONNECTION_CODE');return False
                if parent.origin and parent.origin!=origin:
                    self.deny('EXTENSION_CHANGED');return False
                parent.last_seen=time.monotonic()
                return True
            def deny(self,reason):
                # Evita que Windows reinicie el socket por un POST pequeño sin leer.
                # No interpreta ni guarda el contenido rechazado; la espera es acotada.
                try:n=int(self.headers.get('Content-Length','0'))
                except ValueError:n=0
                if self.command=='POST' and 0<n<=4096:
                    previous=self.connection.gettimeout()
                    try:
                        self.connection.settimeout(1);self.rfile.read(n)
                    except OSError:pass
                    finally:self.connection.settimeout(previous)
                self.reply(403,{'error':reason})
            def do_OPTIONS(self):
                if not self.cors_origin():self.reply(403,{});return
                self.send_response(204);self.send_header('Access-Control-Allow-Origin',self.cors_origin())
                self.send_header('Access-Control-Allow-Methods','GET, POST, OPTIONS')
                self.send_header('Access-Control-Allow-Headers','Content-Type, X-Sitfa-Code, X-Sitfa-Extension')
                self.send_header('Access-Control-Allow-Private-Network','true')
                self.send_header('Cache-Control','no-store');self.end_headers()
            def do_GET(self):
                if not self.authorized():return
                if self.path!='/next' or not parent.origin:self.reply(404,{});return
                deadline=time.monotonic()+4
                while True:
                    try:task=parent.tasks.get(timeout=max(0,deadline-time.monotonic()))
                    except queue.Empty:self.reply(200,{'idle':True});return
                    with parent.lock:active=task['id'] in parent.pending
                    if active:self.reply(200,task);return
            def do_POST(self):
                if not self.authorized():return
                try:
                    n=int(self.headers.get('Content-Length','0'))
                    if not 0<n<=112*1024*1024:raise ValueError()
                    self.connection.settimeout(70)
                    data=json.loads(self.rfile.read(n))
                    if not isinstance(data,dict):raise ValueError()
                except Exception:self.reply(400,{'error':'Mensaje invalido.'});return
                if self.path=='/connect':
                    with parent.lock:
                        if parent.origin and parent.origin!=self.extension_origin():
                            self.reply(403,{'error':'EXTENSION_CHANGED'});return
                        parent.origin=self.extension_origin()
                    parent.emit('connected','Chrome conectado. Entra a Seguimiento y pulsa Cargar opciones.')
                    self.reply(200,{'ok':True});return
                if self.path!='/complete':self.reply(404,{});return
                if not parent.origin:self.reply(409,{'error':'NOT_CONNECTED'});return
                if not isinstance(data.get('id'),str) or type(data.get('ok')) is not bool:
                    self.reply(400,{'error':'Mensaje invalido.'});return
                with parent.lock:
                    entry=parent.pending.get(data['id'])
                    if entry is None or entry['event'].is_set():
                        self.reply(409,{'error':'Tarea vencida.'});return
                    entry['result']=data;entry['event'].set()
                self.reply(200,{'ok':True})
        self.server=ThreadingHTTPServer(('127.0.0.1',port),Handler)
        self.server.daemon_threads=True
        self.port=self.server.server_port
        threading.Thread(target=self.server.serve_forever,daemon=True).start()

    @property
    def connection_code(self):return f'{self.port}-{self.code}'

    @property
    def connected(self):return bool(self.origin and time.monotonic()-self.last_seen<20 and not self.closed)

    def call(self,command,payload=None,timeout=75):
        if command not in ('catalog','capabilities','lock','unlock','prepare','search','download','evidence_open','evidence_capture','evidence_close','diary_open'):
            raise PocError('Accion no permitida.')
        if self.closed or not self.connected:
            raise PocError('Conecta la extension desde la pestaña SITFA de tu Chrome. Mantén abierta su pestaña de conexión.')
        job=secrets.token_hex(12);entry={'event':threading.Event(),'result':None}
        with self.lock:self.pending[job]=entry
        self.tasks.put({'id':job,'command':command,'payload':payload or {}})
        try:
            if not entry['event'].wait(timeout):
                raise PocError('Chrome no respondio a tiempo. Los archivos ya verificados se conservan.')
            result=entry['result']
            if not result or not result.get('ok'):
                messages={
                    'DIARY_CONTEXT':'No se conserva el listado actual de bitácoras en la extensión. Repite el lote sin navegar durante la lectura.',
                    'DIARY_LINK':'El enlace de bitácora falta o está repetido en el listado actual. No se abrió una causa por suposición.',
                    'DIARY_IDENTITY':'La identidad de bitácora no coincide con el tribunal, modalidad e ingreso del listado actual.',
                    'DIARY_NETWORK':'Falló el contacto con RUS al abrir la bitácora o hubo una redirección. Comprueba la sesión y reintenta.',
                    'DIARY_TIMEOUT':'RUS no respondió a tiempo al abrir la bitácora. Reintenta las lecturas fallidas.',
                    'DIARY_RESPONSE':'RUS no entregó una respuesta válida de bitácora. Revisa la sesión y reintenta.',
                    'FOLLOWUP_REQUIRED':'El enlace con Chrome funciona, pero no se encuentra Seguimiento. Entra a Seguimiento en la pestaña original de SITFA y vuelve a Cargar opciones.',
                    'SITE_ACCESS':'Chrome no permite acceder a la pestaña SITFA. Revisa que siga abierta y que la extensión tenga acceso a familia.pjud.cl; abre de nuevo la conexión desde esa pestaña.',
                    'MULTIPLE_FORMS':'Hay más de un formulario de Seguimiento en esa pestaña. Abre la conexión desde una pestaña con un solo formulario.',
                    'SCREEN_OPEN':'No se pudo abrir la pantalla elegida. Revisa la sesión SITFA y vuelve a Cargar opciones.',
                    'EVIDENCE_CAPTURE':'No se pudo capturar la consulta vacía. Mantén SITFA visible, pulsa el icono de la extensión desde SITFA y vuelve a conectar. Los archivos guardados se conservan.',
                    'EVIDENCE_OPEN':'No se pudo cargar la vista SITFA para generar el PDF de la consulta vacía. Revisa la sesión y actualiza la extensión y el descargador juntos. Los archivos guardados se conservan.',
                }
                error=PocError(messages.get(result.get('error'),'La extension no pudo completar la consulta. Revise su pestaña de conexión y la sesión SITFA.'))
                error.code=result.get('error') if result.get('error') in messages else 'SITFA_TASK'
                raise error
            return result.get('result')
        finally:
            with self.lock:self.pending.pop(job,None)

    def close(self):
        self.closed=True
        with self.lock:
            for entry in self.pending.values():entry['event'].set()
        self.server.shutdown();self.server.server_close()
        self.code='';self.origin=None
