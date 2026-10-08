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
                origin=self.cors_origin()
                code=self.headers.get('X-Sitfa-Code','')
                if (not origin or (parent.origin and parent.origin!=origin)
                        or not re.fullmatch(r'[0-9a-f]{32}',code)
                        or not hmac.compare_digest(code,parent.code)):
                    self.reply(403,{'error':'Conexion no autorizada.'});return False
                parent.last_seen=time.monotonic()
                return True
            def do_OPTIONS(self):
                if not self.cors_origin():self.reply(403,{});return
                self.send_response(204);self.send_header('Access-Control-Allow-Origin',self.cors_origin())
                self.send_header('Access-Control-Allow-Methods','GET, POST, OPTIONS')
                self.send_header('Access-Control-Allow-Headers','Content-Type, X-Sitfa-Code')
                self.send_header('Access-Control-Allow-Private-Network','true')
                self.send_header('Cache-Control','no-store');self.end_headers()
            def do_GET(self):
                if not self.authorized():return
                if self.path!='/next' or not parent.origin:self.reply(404,{});return
                try:task=parent.tasks.get(timeout=4)
                except queue.Empty:self.reply(200,{'idle':True});return
                with parent.lock:active=task['id'] in parent.pending
                self.reply(200,task if active else {'idle':True})
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
                        if parent.origin and parent.origin!=self.cors_origin():self.reply(403,{});return
                        parent.origin=self.cors_origin()
                    parent.emit('connected','Chrome conectado. Entra a Seguimiento y pulsa Cargar opciones.')
                    self.reply(200,{'ok':True});return
                if self.path!='/complete':self.reply(404,{});return
                with parent.lock:entry=parent.pending.get(data.get('id'))
                if entry is None:self.reply(409,{'error':'Tarea vencida.'});return
                entry['result']=data;entry['event'].set();self.reply(200,{'ok':True})
        self.server=ThreadingHTTPServer(('127.0.0.1',port),Handler)
        self.server.daemon_threads=True
        self.port=self.server.server_port
        threading.Thread(target=self.server.serve_forever,daemon=True).start()

    @property
    def connection_code(self):return f'{self.port}-{self.code}'

    @property
    def connected(self):return bool(self.origin and time.monotonic()-self.last_seen<20 and not self.closed)

    def call(self,command,payload=None,timeout=75):
        if command not in ('catalog','lock','unlock','prepare','search','download'):
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
                raise PocError('La extension no pudo completar la consulta. Revise su pestaña de conexión y la sesión SITFA.')
            return result.get('result')
        finally:
            with self.lock:self.pending.pop(job,None)

    def close(self):
        self.closed=True
        with self.lock:
            for entry in self.pending.values():entry['event'].set()
        self.server.shutdown();self.server.server_close()
        self.code='';self.origin=None
