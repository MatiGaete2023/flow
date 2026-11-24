"""Controlador unitario del producto real, sin transporte a RUS desde Python.

Se ejecuta desde el hilo de UI (SQLite y revalidación Work). El navegador
solo recibe create_once después de una reserva durable. Todo ACK es incierto.
"""
from datetime import date
from .journal import JournalError
from .pilot_bridge import expected_report


class PilotController:
    def __init__(self,journal,mailbox,op,*,revalidate=None,recovery=False):
        if mailbox.op!=op:raise JournalError('OPERACION_PUENTE_DISTINTA')
        self.journal=journal;self.mailbox=mailbox;self.op=op
        self.revalidate=revalidate;self.recovery=recovery
        self.context=None;self.report=None;self.confirmation=None
        self.phase='RECUPERACION' if recovery else 'NUEVA'
        if recovery:
            if journal.load_unresolved()!=op:raise JournalError('RECUPERACION_DISTINTA')
            if journal.state(op.operation_id)=='ENVIANDO':journal.mark_uncertain(op.operation_id)
        elif not callable(revalidate) or journal.state(op.operation_id)!='PREPARADA':
            raise JournalError('PILOTO_NO_PREPARADO')

    def _current(self):
        if self.recovery:raise JournalError('RECUPERACION_SOLO_LECTURA')
        current=self.revalidate()
        self.op.require_same_approval(current)
        return current

    def prepare(self,*,confirmed):
        if self.phase!='NUEVA' or confirmed is not True:raise JournalError('PREPARACION_NO_AUTORIZADA')
        self._current()
        self.mailbox.submit('prepare');self.phase='PREPARANDO'

    def authorize_and_verify(self,*,author,stage,server_day,confirmed,require_excel=False):
        if self.phase!='PREPARADA' or confirmed is not True or type(require_excel) is not bool or type(server_day) is not date or any(
                type(x) is not str or not x.strip() for x in (author,stage)):
            raise JournalError('CONFIRMACION_EXACTA_REQUERIDA')
        self._current()
        self.confirmation=(author,stage,server_day,require_excel)
        self.mailbox.submit('verify');self.phase='VERIFICANDO'

    def request_readback(self,*,confirmed):
        if self.phase not in ('INCIERTA','RECUPERACION','NO_HALLADA') or confirmed is not True:
            raise JournalError('RELECTURA_NO_ELEGIBLE')
        self.mailbox.submit('readback');self.phase='RELEYENDO'

    def poll(self):
        result=self.mailbox.take_result()
        if result is None:return self.phase
        command,status=result['command'],result['status']
        if command=='create_once' and self.phase=='ENVIANDO':
            # Nunca utilizar ACK como prueba de registro, ni repetir una orden.
            if self.journal.state(self.op.operation_id)=='ENVIANDO':self.journal.mark_uncertain(self.op.operation_id)
            self.phase='INCIERTA';return self.phase
        if command=='readback' and self.phase=='RELEYENDO':
            if status!='OK':self.phase='INCIERTA';return self.phase
            self.phase=self.journal.resolve_creation(self.op,result['raw'])
            return self.phase
        if status!='OK':
            self.phase='BLOQUEADA';return self.phase
        if command=='prepare' and self.phase=='PREPARANDO':
            self._current()
            if result['report']!=expected_report(self.op):raise JournalError('PREPARACION_NO_ACREDITADA')
            self.context=result['context'];self.report=result['report'];self.phase='PREPARADA';return self.phase
        if command=='verify' and self.phase=='VERIFICANDO':
            current=self._current()
            if result['context']!=self.context or result['report']!=self.report:
                self.phase='BLOQUEADA';raise JournalError('FORMULARIO_CAMBIADO')
            author,stage,day,require_excel=self.confirmation;self.confirmation=None
            self.journal.approve_preparation(self.op,current=current,prepared=self.report,author=author,confirmed=True)
            ticket=self.journal.reserve_creation(self.op,current=self._current(),prepared=self.report,raw=result['raw'],
                author=author,stage=stage,stage_code=self.context['COD_Etapa'],server_day=day,confirmed=True,require_excel=require_excel)
            self.phase='ENVIANDO'
            try:self.mailbox.submit('create_once',reservation=ticket)
            except Exception:
                self.journal.mark_uncertain(self.op.operation_id);self.phase='INCIERTA';raise
            return self.phase
        self.phase='BLOQUEADA';raise JournalError('RESPUESTA_FUERA_DE_FASE')

    def stop(self):
        self.mailbox.cancel_pending()
        # Si el usuario cierra después de reserva, siempre recuperación por lectura.
        if self.journal.state(self.op.operation_id)=='ENVIANDO':
            self.journal.mark_uncertain(self.op.operation_id)
        self.confirmation=None
        self.phase='DETENIDA'
