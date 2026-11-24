"""P7a: transporte localhost de preparación ficticia, sin escritor ni conexión RUS."""
from dataclasses import asdict
from hashlib import sha256
from hmac import compare_digest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import re
import secrets
from threading import Lock, Thread

from .contracts import RegistroFlowV1, CAPABILITIES

COMMANDS = frozenset(('get_capabilities', 'inspect_current', 'locate_exact_row', 'open_bitacora', 'prepare', 'readback'))
MAX_BODY = 4096  # Poll/ack solo contienen identificadores y resultados categóricos.
MAX_COMMAND = 65536  # Límite técnico del transporte, nunca truncar texto.


class BridgeError(ValueError):
    pass


def _json(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode('utf-8')


def _unique(pairs):
    out = {}
    for k, v in pairs:
        if k in out:
            raise BridgeError('CLAVE_DUPLICADA')
        out[k] = v
    return out


def _fields(value, keys):
    if type(value) is not dict or set(value) != set(keys.split()):
        raise BridgeError('MENSAJE_INVALIDO')


class Mailbox:
    """Una orden local por vez. La entrega se consume, no se repite automáticamente."""
    def __init__(self):
        self.session_id = secrets.token_hex(16)
        self._lock = Lock()
        self._command = None
        self._delivered = False
        self._result = None

    def submit(self, command, *, frozen=None, current=None):
        if not isinstance(command, str) or command not in COMMANDS:
            raise BridgeError('COMANDO_PROHIBIDO')
        envelope = dict(schema='BridgeCommandV1', session_id=self.session_id, command_id=secrets.token_hex(16),
                        command=command, simulation_only=True)
        if command == 'get_capabilities':
            if frozen is not None or current is not None:
                raise BridgeError('PARAMETROS_ADICIONALES')
        else:
            if not isinstance(frozen, RegistroFlowV1):
                raise BridgeError('APROBACION_REQUERIDA')
            frozen.require_same_approval(current)
            envelope.update(operation_id=frozen.operation_id, remote_identity=asdict(frozen.remote_identity),
                            origin=frozen.origin, approval_digest=sha256(_json(asdict(frozen))).hexdigest())
            if command == 'prepare':
                envelope['approved_operation'] = asdict(frozen)
        if len(_json(envelope)) > MAX_COMMAND:
            raise BridgeError('COMANDO_EXCEDE_TRANSPORTE')
        with self._lock:
            if self._command is not None or self._result is not None:
                raise BridgeError('COMANDO_ACTIVO_O_RESULTADO_PENDIENTE')
            self._command, self._delivered, self._result = envelope, False, None
        return envelope['command_id']

    def poll(self, body):
        _fields(body, 'schema session_id')
        if body['schema'] != 'BridgePollV1' or body['session_id'] != self.session_id:
            raise BridgeError('SESION_INVALIDA')
        with self._lock:
            if self._command is None or self._delivered:
                return {'command': None}
            self._delivered = True
            return {'command': self._command}

    def ack(self, body):
        _fields(body, 'schema session_id command_id status')
        if (body['schema'] != 'BridgeAckV1' or body['session_id'] != self.session_id or
                body['status'] not in ('OK', 'BLOCKED')):
            raise BridgeError('RESPUESTA_INVALIDA')
        with self._lock:
            if self._command is None or not self._delivered or body['command_id'] != self._command['command_id']:
                raise BridgeError('COMANDO_NO_CORRESPONDE')
            # Ack acredita recepción categórica, no identidad, preparación efectiva ni recibo RUS.
            self._result = dict(command_id=body['command_id'], status=body['status'], simulation_only=True)
            self._command = None
            return {'accepted': True}

    def take_result(self):
        with self._lock:
            result, self._result = self._result, None
            return result


class LocalBridge:
    def __init__(self, origin, mailbox=None, *, max_body=MAX_BODY):
        if not isinstance(origin, str) or not re.fullmatch(r'chrome-extension://[a-p]{32}', origin):
            raise BridgeError('ORIGEN_EXTENSION_INVALIDO')
        if type(max_body) is not int or not MAX_BODY <= max_body <= 16*1024*1024:
            raise BridgeError('LIMITE_TRANSPORTE_INVALIDO')
        self.origin = origin
        self.mailbox = mailbox or Mailbox()
        self.token = secrets.token_urlsafe(32)
        bridge = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass  # Nunca persistir rutas, cabeceras ni cuerpos.

            def _valid_origin(self):
                return (self.headers.get_all('Host') == [bridge.host] and
                        self.headers.get_all('Origin') == [bridge.origin])

            def _reply(self, status, body=None, *, cors=False):
                data = _json(body) if body is not None else b''
                self.send_response(status)
                self.send_header('Content-Type', 'application/json; charset=utf-8')
                self.send_header('Content-Length', str(len(data)))
                self.send_header('Cache-Control', 'no-store')
                self.send_header('X-Content-Type-Options', 'nosniff')
                if cors:
                    self.send_header('Access-Control-Allow-Origin', bridge.origin)
                    self.send_header('Vary', 'Origin')
                self.end_headers()
                if data:
                    self.wfile.write(data)

            def do_OPTIONS(self):
                requested = self.headers.get('Access-Control-Request-Headers', '')
                headers = {x.strip().lower() for x in requested.split(',') if x.strip()}
                if (not self._valid_origin() or self.path not in ('/v1/poll', '/v1/ack') or
                        self.headers.get('Access-Control-Request-Method') != 'POST' or
                        not headers <= {'authorization', 'content-type'}):
                    return self._reply(403)
                self.send_response(204)
                self.send_header('Access-Control-Allow-Origin', bridge.origin)
                self.send_header('Access-Control-Allow-Methods', 'POST')
                self.send_header('Access-Control-Allow-Headers', 'Authorization, Content-Type')
                self.send_header('Vary', 'Origin')
                self.send_header('Content-Length', '0')
                self.end_headers()

            def do_POST(self):
                if not self._valid_origin():
                    return self._reply(403)
                auth = self.headers.get_all('Authorization') or []
                if len(auth) != 1 or not compare_digest(auth[0].encode('utf-8'), ('Bearer ' + bridge.token).encode('utf-8')):
                    return self._reply(403, cors=True)
                if self.path not in ('/v1/poll', '/v1/ack'):
                    return self._reply(404, cors=True)
                lengths = self.headers.get_all('Content-Length') or []
                if (self.headers.get('Transfer-Encoding') is not None or len(lengths) != 1 or
                        not re.fullmatch(r'\d{1,8}', lengths[0]) or not 0 < int(lengths[0]) <= max_body or
                        self.headers.get_all('Content-Type') != ['application/json']):
                    return self._reply(400, cors=True)
                try:
                    raw = self.rfile.read(int(lengths[0]))
                    if len(raw) != int(lengths[0]):
                        raise BridgeError('CUERPO_INCOMPLETO')
                    body = json.loads(raw.decode('utf-8'), object_pairs_hook=_unique)
                    result = bridge.mailbox.poll(body) if self.path == '/v1/poll' else bridge.mailbox.ack(body)
                except (UnicodeError, ValueError, TypeError, OSError):
                    return self._reply(400, {'error': 'MENSAJE_RECHAZADO'}, cors=True)
                return self._reply(200, result, cors=True)

            def do_GET(self):
                return self._reply(405)

            def setup(self):
                super().setup()
                self.connection.settimeout(5)

        self.server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.host = f'127.0.0.1:{self.server.server_port}'
        self.thread = Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def capabilities(self):
        return {'simulation_only': True, **CAPABILITIES}

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=5)
