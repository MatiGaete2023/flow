"""Diario de piloto real, candidato no habilitado en esta versión.

No navega ni guarda en RUS. El controlador debe aportar lecturas vivas
procedentes del puente autenticado. No convertir fixtures en recibos reales.
"""
from dataclasses import dataclass,asdict
from datetime import datetime,timezone
from hashlib import sha256
import json
from pathlib import Path
import re
import secrets

from .contracts import CAPABILITIES, RegistroFlowV1
from .confirmed_preview import ConfirmedPreview
from .journal import _JournalCore, JournalError

# Acta de aceptación y condiciones G01–G14: esta versión no puede reservar
# intentos institucionales. No permitir flags de CLI ni archivos de config.
def _require_accepted():
    if CAPABILITIES['write_one'] is not True:
        raise JournalError('PILOTO_REAL_NO_HABILITADO')


@dataclass(frozen=True)
class PilotPolicy:
    directory: Path
    retention_days: int
    acceptance_reference: str
    authority_confirmed: bool

    def __post_init__(self):
        if (not isinstance(self.directory,Path) or not self.directory.is_absolute() or
            type(self.retention_days) is not int or self.retention_days<1 or
            type(self.acceptance_reference) is not str or not self.acceptance_reference.strip() or
            self.authority_confirmed is not True):
            raise JournalError('CUSTODIA_O_AUTORIZACION_NO_ACORDADA')


def _hash(text):
    return sha256(text.encode('utf-8')).hexdigest()


class PilotJournal(_JournalCore):
    _simulated=0

    def __init__(self,path,*,policy,clock=None):
        _require_accepted()
        if type(policy) is not PilotPolicy or Path(path).parent!=policy.directory:
            raise JournalError('CUSTODIA_O_AUTORIZACION_NO_ACORDADA')
        super().__init__(path,clock=clock,protected_demo=True)
        self.policy=policy
        try:
            self.db.execute('''CREATE TABLE IF NOT EXISTS pilot_policy(
                singleton INTEGER PRIMARY KEY CHECK(singleton=1), policy BLOB NOT NULL, digest TEXT NOT NULL)''')
            self.db.execute('''CREATE TABLE IF NOT EXISTS pilot_review(
                operation_id TEXT PRIMARY KEY REFERENCES operations(operation_id),
                evidence BLOB NOT NULL, digest TEXT NOT NULL)''')
            self.db.execute('''CREATE TABLE IF NOT EXISTS pilot_reservations(
                operation_id TEXT PRIMARY KEY REFERENCES write_attempts(operation_id),
                reservation_digest TEXT NOT NULL, created_at TEXT NOT NULL, context BLOB NOT NULL, context_digest TEXT NOT NULL)''')
            value=json.dumps(dict(directory=str(policy.directory),retention_days=policy.retention_days,
                                  acceptance_reference=policy.acceptance_reference,authority_confirmed=True),
                             ensure_ascii=False,sort_keys=True,separators=(',',':'))
            with self._tx():
                existing=self.db.execute('SELECT policy,digest FROM pilot_policy WHERE singleton=1').fetchone()
                if existing is None:
                    self.db.execute('INSERT INTO pilot_policy VALUES(1,?,?)',(self.storage.seal('policy','pilot',value),self.storage.digest('policy',value)))
                elif self.storage.unseal('policy','pilot',existing['policy'])!=value or existing['digest']!=self.storage.digest('policy',value):
                    raise JournalError('POLITICA_DIARIO_CAMBIADA')
        except BaseException:
            self.close();raise

    def freeze_confirmed(self,preview,work,requests,*,report_catalog):
        _require_accepted()
        if type(preview) is not ConfirmedPreview:
            raise JournalError('REVISION_EXACTA_REQUERIDA')
        batch=preview.revalidate(work,requests,report_catalog=report_catalog)
        # P8: piloto unitario; no ampliar escala por parámetro.
        if len(batch.operations)!=1:
            raise JournalError('PILOTO_UNITARIO_REQUERIDO')
        op=batch.operations[0]
        proof=json.dumps(asdict(preview.review.candidates[0]),ensure_ascii=False,sort_keys=True,separators=(',',':'))
        if op.operation_id!=_hash(json.dumps([preview.review.digest,op.source.row_ref,op.approval_version],ensure_ascii=False,sort_keys=True,separators=(',',':'))):
            raise JournalError('ID_APROBACION_NO_ACREDITADO')
        state=super().freeze(op)
        with self._tx():
            self._require_current(op.operation_id,op)
            old=self.db.execute('SELECT evidence,digest FROM pilot_review WHERE operation_id=?',(op.operation_id,)).fetchone()
            digest=self.storage.digest('review',proof)
            if old is None:
                if state!='PREPARADA':raise JournalError('REVISION_PILOTO_AUSENTE')
                self.db.execute('INSERT INTO pilot_review VALUES(?,?,?)',(op.operation_id,self.storage.seal('review',op.operation_id,proof),digest))
            elif old['digest']!=digest or self.storage.unseal('review',op.operation_id,old['evidence'])!=proof:
                raise JournalError('REVISION_PILOTO_CAMBIADA')
        return op

    def freeze(self,op):
        raise JournalError('USAR_REVISION_FLOW_CONFIRMADA')

    def _review_valid(self,op):
        row=self.db.execute('SELECT evidence,digest FROM pilot_review WHERE operation_id=?',(op.operation_id,)).fetchone()
        if row is None:raise JournalError('REVISION_PILOTO_AUSENTE')
        plain=self.storage.unseal('review',op.operation_id,row['evidence'])
        if self.storage.digest('review',plain)!=row['digest']:raise JournalError('REVISION_PILOTO_CAMBIADA')
        return row['digest']

    def approve_preparation(self,op,*,current,prepared,author,confirmed):
        _require_accepted()
        op.require_same_approval(current)
        self._review_valid(op)
        expected=dict(operation_id=op.operation_id,origin=op.origin,state=op.observation_state,cc=op.cc,saved=False)
        if type(prepared) is not dict or prepared!=expected or confirmed is not True:
            raise JournalError('PREPARACION_O_CONFIRMACION_NO_ACREDITADA')
        # Actor declarado por el operador; la relectura exige su coincidencia exacta.
        self.mark_ready(op)
        self._authorize(op,human_actor=author,confirmed=True)

    def reserve_creation(self,op,*,current,prepared,raw,author,stage,stage_code,server_day,confirmed,require_excel=False,context=None):
        _require_accepted()
        op.require_same_approval(current)
        self._review_valid(op)
        expected=dict(operation_id=op.operation_id,origin=op.origin,state=op.observation_state,cc=op.cc,saved=False)
        if type(prepared) is not dict or prepared!=expected or confirmed is not True:
            raise JournalError('PREPARACION_O_CONFIRMACION_NO_ACREDITADA')
        auth=self.db.execute('SELECT human_actor FROM authorizations WHERE operation_id=?',(op.operation_id,)).fetchone()
        if auth is None or self.storage.unseal('actor',op.operation_id,auth['human_actor'])!=author:
            raise JournalError('AUTOR_NO_COINCIDE')
        from .pilot_bridge import parse_context
        context=parse_context(op,context)
        if context['COD_Etapa']!=stage_code:raise JournalError('CONTEXTO_NO_COINCIDE')
        result=self.bind_baseline(op,raw,author=author,stage=stage,stage_code=stage_code,server_day=server_day,require_excel=require_excel)
        if result!='SIN_PREVIA':
            raise JournalError('LECTURA_PREVIA_NO_ELEGIBLE')
        # Vincular el baseline que se acaba de examinar con la reserva transaccional.
        row=self.db.execute('SELECT evidence_digest FROM readback_baselines WHERE operation_id=?',(op.operation_id,)).fetchone()
        if row is None:raise JournalError('BASELINE_REQUERIDA')
        reservation=secrets.token_hex(32)
        context_json=json.dumps(context,sort_keys=True,separators=(',',':'))
        def record(current_row,now):
            self.db.execute('INSERT INTO pilot_reservations VALUES(?,?,?,?,?)',
                (op.operation_id,self.storage.digest('reservation',reservation),now,
                 self.storage.seal('reservation-context',op.operation_id,context_json),self.storage.digest('reservation-context',context_json)))
        self._reserve_attempt(op,expected_evidence_digest=row[0],record_reservation=record)
        # Solo se devuelve tras COMMIT; no existe función para reemitirlo.
        return dict(operation_id=op.operation_id,approved_text_sha256=op.approved_text_sha256,
                    approval_version=op.approval_version,state='ENVIANDO',reservation_id=reservation)

    def reservation_context(self,op):
        from .pilot_bridge import parse_context
        self._require_current(op.operation_id,op)
        row=self.db.execute('SELECT context,context_digest FROM pilot_reservations WHERE operation_id=?',(op.operation_id,)).fetchone()
        if row is None:raise JournalError('RESERVA_NO_ACREDITADA')
        value=self.storage.unseal('reservation-context',op.operation_id,row['context'])
        if self.storage.digest('reservation-context',value)!=row['context_digest']:raise JournalError('CONTEXTO_RESERVA_INVALIDO')
        return parse_context(op,json.loads(value))

    def load_unresolved(self):
        ids=tuple(row[0] for row in self.db.execute("SELECT operation_id FROM operations WHERE state IN ('ENVIANDO','INCIERTA','NO_HALLADA','AMBIGUA','PENDIENTE_EXCEL') ORDER BY operation_id"))
        if len(ids)!=1:raise JournalError('RECUPERACION_REQUIERE_INTENTO_UNICO')
        row=self._row(ids[0])
        payload=self.storage.unseal('payload',ids[0],row['payload'])
        if self.storage.digest('approval',payload)!=row['digest']:
            raise JournalError('APROBACION_CAMBIADA')
        op=RegistroFlowV1.from_mapping(json.loads(payload))
        self._require_current(ids[0],op);self._review_valid(op)
        return op

    def resolve_creation(self,op,raw):
        # Solo lectura posterior a una reserva ya existente, nunca otro envío.
        self._review_valid(op)
        return self._resolve(op,raw)
