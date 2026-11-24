"""Protocolo local de piloto unitario. Sin acceso institucional desde Python.

El transporte no autoriza nada por sí. El controlador de UI entrega comandos
de una vez, el diario reserva ANTES de create_once. No almacenar HTML.
"""
from dataclasses import asdict
from datetime import datetime,timezone
from threading import Lock
from time import monotonic,time
import re
import secrets

from .bridge import LocalBridge,BridgeError,_fields
from .contracts import RegistroFlowV1,CAPABILITIES
from .live_read import KEYS,MAX_HTML
from .readback import Snapshot
from .state_policy import require_state

COMMANDS=frozenset(('prepare','verify','create_once','readback'))
LEASE_SECONDS=90


def _enabled():
    if CAPABILITIES['write_one'] is not True:
        raise BridgeError('PILOTO_REAL_NO_HABILITADO')


def expected_report(op):
    return dict(operation_id=op.operation_id,origin=op.origin,state=op.observation_state,cc=op.cc,saved=False)


def parse_context(op,context):
    if type(context) is not dict or set(context)!=set(KEYS) or any(
            type(v) is not str or not re.fullmatch(r'[0-9]{1,20}',v) for v in context.values()):
        raise BridgeError('CONTEXTO_INVALIDO')
    i=op.remote_identity
    if (context['COD_Tribunal'],context['CRR_IdCausa'],context['ID_Ingreso'])!=(i.tribunal_id,i.causa_id,i.ingreso_id):
        raise BridgeError('IDENTIDAD_DISTINTA')
    if context['tipo_popUp'] not in ('11','12','13') or any(int(context[k])<=0 for k in KEYS[:-1]):
        raise BridgeError('CONTEXTO_INVALIDO')
    return dict(context)


def parse_live_snapshot(op,context,html):
    from nurus.bitacora_html import read_snapshot
    context=parse_context(op,context)
    if type(html) is not str or not html or len(html.encode('utf-8'))>MAX_HTML:
        raise BridgeError('LECTURA_INVALIDA')
    raw=read_snapshot(html,context,source='piloto-local-dom',captured_at=datetime.now(timezone.utc).isoformat())
    snap=Snapshot.from_reader(raw)
    if snap.identity!=op.remote_identity or snap.stage_code!=context['COD_Etapa']:
        raise BridgeError('IDENTIDAD_DISTINTA')
    return raw


class PilotMailbox:
    def __init__(self,op):
        _enabled()
        if type(op) is not RegistroFlowV1 or op.source.tool!='FLOW' or not re.fullmatch(r'[a-f0-9]{64}',op.operation_id):
            raise BridgeError('OPERACION_FLOW_REQUERIDA')
        require_state(op.origin,op.observation_state)
        self.op=op
        self.session_id=secrets.token_hex(16)
        self._lock=Lock()
        self._active=None
        self._deadline=None
        self._delivered=False
        self._result=None
        self._creation_issued=False
        self._closed=False

    def submit(self,command,*,reservation=None,context=None):
        _enabled()
        if command not in COMMANDS:
            raise BridgeError('COMANDO_PROHIBIDO')
        envelope=dict(schema='PilotCommandV1',session_id=self.session_id,command_id=secrets.token_hex(16),
                      command=command,operation=asdict(self.op),expires_at_ms=int((time()+LEASE_SECONDS)*1000))
        if command=='create_once':
            _fields(reservation,'operation_id approved_text_sha256 approval_version state reservation_id')
            if (reservation['operation_id']!=self.op.operation_id or reservation['approved_text_sha256']!=self.op.approved_text_sha256 or
                    reservation['approval_version']!=self.op.approval_version or reservation['state']!='ENVIANDO' or
                    type(reservation['reservation_id']) is not str or not re.fullmatch(r'[a-f0-9]{64}',reservation['reservation_id'])):
                raise BridgeError('RESERVA_NO_ACREDITADA')
            envelope['reservation']=dict(reservation)
        elif reservation is not None:
            raise BridgeError('RESERVA_FUERA_DE_COMANDO')
        if command=='readback':envelope['context']=parse_context(self.op,context)
        elif context is not None:raise BridgeError('CONTEXTO_FUERA_DE_COMANDO')
        with self._lock:
            if self._closed:raise BridgeError('SESION_CERRADA')
            if self._active is not None or self._result is not None:
                raise BridgeError('COMANDO_ACTIVO_O_RESULTADO_PENDIENTE')
            if command=='create_once':
                if self._creation_issued:raise BridgeError('CREACION_YA_EMITIDA')
                self._creation_issued=True
            self._active=envelope;self._deadline=monotonic()+LEASE_SECONDS;self._delivered=False
        return envelope['command_id']

    def _expire(self):
        if self._active is not None and monotonic()>self._deadline:
            self._result=dict(command=self._active['command'],status='EXPIRADA')
            self._active=None

    def poll(self,body):
        _fields(body,'schema session_id')
        if body['schema']!='BridgePollV1' or body['session_id']!=self.session_id:
            raise BridgeError('SESION_INVALIDA')
        with self._lock:
            self._expire()
            if self._closed or self._active is None or self._delivered:return {'command':None}
            self._delivered=True
            return {'command':self._active}

    def _response(self,body,command):
        common='schema session_id command_id status'
        if body.get('status')=='BLOCKED':
            _fields(body,common)
            return dict(command=command,status='BLOCKED')
        if body.get('status')!='OK':
            raise BridgeError('RESPUESTA_INVALIDA')
        if command=='prepare':
            _fields(body,common+' report context')
            if type(body['report']) is not dict or body['report']!=expected_report(self.op):
                raise BridgeError('PREPARACION_NO_ACREDITADA')
            return dict(command=command,status='OK',report=dict(body['report']),context=parse_context(self.op,body['context']))
        if command=='verify':
            _fields(body,common+' report context html')
            if type(body['report']) is not dict or body['report']!=expected_report(self.op):
                raise BridgeError('PREPARACION_NO_ACREDITADA')
            return dict(command=command,status='OK',report=dict(body['report']),context=parse_context(self.op,body['context']),
                        raw=parse_live_snapshot(self.op,body['context'],body['html']))
        if command=='create_once':
            _fields(body,common)
            # Incluso OK solo acredita que la orden llegó al navegador.
            return dict(command=command,status='INCIERTA')
        if command=='readback':
            _fields(body,common+' context html')
            return dict(command=command,status='OK',context=parse_context(self.op,body['context']),
                        raw=parse_live_snapshot(self.op,body['context'],body['html']))
        raise BridgeError('COMANDO_PROHIBIDO')

    def ack(self,body):
        if type(body) is not dict or body.get('schema')!='PilotAckV1' or body.get('session_id')!=self.session_id:
            raise BridgeError('RESPUESTA_INVALIDA')
        with self._lock:
            self._expire()
            if self._active is None or not self._delivered or body.get('command_id')!=self._active['command_id']:
                raise BridgeError('COMANDO_NO_CORRESPONDE')
            command=self._active['command']
            try:
                self._result=self._response(body,command)
                if command=='readback' and self._result['status']=='OK' and self._result['context']!=self._active['context']:
                    raise BridgeError('CONTEXTO_RESERVA_INVALIDO')
            except (ValueError,RuntimeError,TypeError,KeyError,UnicodeError):
                self._result=dict(command=command,status='BLOCKED')
            self._active=None
            return {'accepted':True}

    def cancel_pending(self):
        with self._lock:
            # No se puede revocar lo ya entregado; ese intento queda incierto.
            self._closed=True;self._active=None;self._result=None

    def take_result(self):
        with self._lock:
            self._expire()
            value,self._result=self._result,None
            return value


def open_pilot_bridge(origin,op):
    return LocalBridge(origin,PilotMailbox(op),max_body=16*1024*1024)
