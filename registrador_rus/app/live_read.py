"""Validación de ingreso/lectura viva entre Work y extensión local.

No crea observaciones, no acepta órdenes de escritura ni conserva HTML en disco.
El ACK no es recibo de registro. El operador abre manualmente listado y popup.
"""
from dataclasses import asdict
from datetime import datetime, timezone
import re
import secrets
from threading import Lock

from .bridge import BridgeError, LocalBridge, _fields
from .contracts import RegistroFlowV1
from .state_policy import require_state
from .readback import Snapshot

KEYS=('COD_Tribunal','CRR_IdCausa','ID_Ingreso','COD_Etapa','TIP_Consulta','tipo_popUp')
MAX_HTML=8*1024*1024


class LiveReadMailbox:
    def __init__(self, op, *, prepare_only=False):
        if type(prepare_only) is not bool:raise BridgeError('MODO_VALIDACION_INVALIDO')
        self.prepare_only=prepare_only
        if type(op) is not RegistroFlowV1 or op.source.tool!='FLOW' or not re.fullmatch(r'[a-f0-9]{64}',op.operation_id):
            raise BridgeError('OPERACION_FLOW_REQUERIDA')
        require_state(op.origin,op.observation_state)
        self.session_id=secrets.token_hex(16)
        self.command_id=secrets.token_hex(16)
        self.op=op
        self._lock=Lock()
        self._delivered=False
        self._finished=False
        self._result=None

    def poll(self, body):
        _fields(body,'schema session_id')
        if body['schema']!='BridgePollV1' or body['session_id']!=self.session_id:
            raise BridgeError('SESION_INVALIDA')
        with self._lock:
            if self._delivered or self._finished:
                return {'command':None}
            self._delivered=True
            return {'command':dict(schema='LiveReadCommandV1',session_id=self.session_id,command_id=self.command_id,
                                   command='validate_prepare_only' if self.prepare_only else 'validate_read_only',operation=asdict(self.op),write_enabled=False)}

    def _parse(self,body):
        from nurus.bitacora_html import read_snapshot
        context=body['context']
        if type(context) is not dict or set(context)!=set(KEYS) or any(
                type(v) is not str or not re.fullmatch(r'[0-9]{1,20}',v) for v in context.values()):
            raise BridgeError('CONTEXTO_INVALIDO')
        identity=self.op.remote_identity
        if (context['COD_Tribunal'],context['CRR_IdCausa'],context['ID_Ingreso'])!=(
                identity.tribunal_id,identity.causa_id,identity.ingreso_id):
            raise BridgeError('IDENTIDAD_DISTINTA')
        html=body['html']
        if type(html) is not str or not html or len(html.encode('utf-8'))>MAX_HTML:
            raise BridgeError('LECTURA_INVALIDA')
        raw=read_snapshot(html,context,source='validacion-local-dom',captured_at=datetime.now(timezone.utc).isoformat())
        snap=Snapshot.from_reader(raw)
        if snap.identity!=identity or snap.stage_code!=context['COD_Etapa']:
            raise BridgeError('IDENTIDAD_DISTINTA')
        # No persistir raw, HTML, autores, textos ni identificadores de entradas.
        result=dict(status='LECTURA_COMPLETA' if snap.complete else 'LECTURA_PARCIAL',
                    entries=len(snap.entries),operation_id=self.op.operation_id,write_enabled=False)
        if self.prepare_only:
            from .pilot_bridge import expected_report
            if type(body['report']) is not dict or body['report']!=expected_report(self.op):
                raise BridgeError('PREPARACION_NO_ACREDITADA')
            result.update(prepared=True,saved=False)
        return result

    def ack(self,body):
        if type(body) is not dict or body.get('schema')!='LiveReadAckV1' or body.get('session_id')!=self.session_id or body.get('command_id')!=self.command_id:
            raise BridgeError('RESPUESTA_INVALIDA')
        if body.get('status')=='OK':
            _fields(body,'schema session_id command_id status context html'+(' report' if self.prepare_only else ''))
        elif body.get('status')=='BLOCKED':
            _fields(body,'schema session_id command_id status')
        else:
            raise BridgeError('RESPUESTA_INVALIDA')
        with self._lock:
            if not self._delivered or self._finished:
                raise BridgeError('COMANDO_NO_CORRESPONDE')
            self._finished=True
            if body['status']=='BLOCKED':
                self._result=dict(status='BLOQUEADA',write_enabled=False)
            else:
                try:self._result=self._parse(body)
                except (ValueError,RuntimeError,TypeError,KeyError,UnicodeError):
                    self._result=dict(status='LECTURA_NO_VERIFICADA',write_enabled=False)
            return {'accepted':True}

    def take_result(self):
        with self._lock:
            value,self._result=self._result,None
            return value


def open_live_read_bridge(origin,op,*,prepare_only=False):
    return LocalBridge(origin,LiveReadMailbox(op,prepare_only=prepare_only),max_body=16*1024*1024)
