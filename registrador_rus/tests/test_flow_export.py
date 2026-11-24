from types import SimpleNamespace as NS
import unittest
from registrador_rus.app.contracts import ContractError
from registrador_rus.app.flow_export import candidate, approve_candidate, TEMPORAL_NOTICE


def work():
    row = NS(id='demo-1', values=dict(SITFA_TRIBUNAL_CODIGO='90', SITFA_CAUSA_RUS='91', SITFA_INGRESO_RUS='92'),
             review={'OBSERVACION': '  Constancia ficticia.  '}, observation='Propuesta', excluded=False, warnings=[], sync_state='')
    return NS(rows=[row], revision=3, capture_digest='a'*64, capture_sources={'demo-1': dict(identity_status='VERIFICADA',
              record_type='ESPERA', court_code='90', cause_id='91', ingreso_id='92')})


def review(w, origin='ESPERA', destination='ADMINISTRATIVA'):
    return candidate(w, 'demo-1', work_id='demo', origin=origin, destination=destination)


def approve(w, c, **kwargs):
    options = dict(displayed_digest=c.digest, destination='ADMINISTRATIVA', actor='operador ficticio',
                   stamp='2026-11-24T12:00:00Z', version=1, operation_id='demo-op', confirmed=True)
    return approve_candidate(w, c, **(options | kwargs))


class FlowExportTests(unittest.TestCase):
    def test_explicit_approval_preserves_text(self):
        w = work(); c = review(w); op = approve(w, c)
        self.assertEqual(op.approved_text, '  Constancia ficticia.  ')
        self.assertEqual(op.remote_identity.ingreso_id, '92')
        self.assertEqual(op.cc, 0)
        self.assertEqual(op.observation_state,'4')
        self.assertEqual(approve(w, review(w, destination='AL_TRIBUNAL'), destination='AL_TRIBUNAL').cc, 1)
        for changes in (dict(confirmed=False), dict(displayed_digest='wrong'), dict(destination=''), dict(destination='AL_TRIBUNAL'), dict(observation_state='otro'), dict(observation_state='')):
            with self.assertRaises(ContractError): approve(w, c, **changes)

    def test_revalidation(self):
        for change in ('text', 'revision', 'capture', 'identity', 'excluded', 'warnings', 'sync'):
            w = work(); c = review(w); row = w.rows[0]
            if change == 'text': row.review['OBSERVACION'] = 'otro'
            if change == 'revision': w.revision += 1
            if change == 'capture': w.capture_digest = 'b'*64
            if change == 'identity': row.values['SITFA_INGRESO_RUS'] = '93'
            if change == 'excluded': row.excluded = True
            if change == 'warnings': row.warnings.append('demo')
            if change == 'sync': row.sync_state = 'CONFLICTO'
            with self.subTest(change=change), self.assertRaises(ContractError): approve(w, c)

    def test_provenance_and_origin(self):
        w = work(); p = w.capture_sources['demo-1']
        for origin in ('CUMPLIMIENTO','INFORMES_POR_VENCER','INFORMES_VENCIDOS','CALENDARIO'):
            with self.assertRaises(ContractError): review(w, origin)
        p['record_type'] = 'INFORMES'
        for origin in ('INFORMES_POR_VENCER','INFORMES_VENCIDOS'):
            self.assertEqual(review(w, origin).origin, origin)
        p['identity_status'] = 'INCIERTA'
        with self.assertRaises(ContractError): review(w,'INFORMES_VENCIDOS')

    def test_missing_duplicate_numeric_identity(self):
        for change in ('missing','duplicate','integer','mismatch'):
            w=work()
            if change=='missing': w.capture_sources={}
            if change=='duplicate': w.rows.append(w.rows[0])
            if change=='integer': w.rows[0].values['SITFA_INGRESO_RUS']=92
            if change=='mismatch': w.capture_sources['demo-1']['ingreso_id']='93'
            with self.subTest(change=change), self.assertRaises(ContractError): review(w)

    def test_required_state_by_origin(self):
        for origin, record_type, code in (('ESPERA','ESPERA','4'),('CUMPLIMIENTO','CUMPLIMIENTO','6'),('INFORMES_POR_VENCER','INFORMES','9'),('INFORMES_VENCIDOS','INFORMES','9')):
            w=work(); w.capture_sources['demo-1']['record_type']=record_type
            c=review(w,origin)
            self.assertEqual(c.observation_state,code)
            self.assertEqual(approve(w,c).observation_state,code)
            for wrong in ('3','5','8','10','demo',int(code)):
                if wrong != code:
                    with self.assertRaises(ContractError):
                        candidate(w,'demo-1',work_id='demo',origin=origin,destination='ADMINISTRATIVA',observation_state=wrong)

    def test_only_accredited_temporal_notice_in_preview(self):
        w=work(); row=w.rows[0]
        row.warnings=[TEMPORAL_NOTICE]
        with self.assertRaisesRegex(ContractError,'AVISO_TEMPORAL_NO_ACREDITADO'): review(w)
        w.capture_context={row.id:dict(reason=TEMPORAL_NOTICE,**{'class':'NO_EVALUABLE'},identity=['RUS','90','91','92'])}
        c=review(w)
        self.assertEqual(c.notices,(TEMPORAL_NOTICE,))
        self.assertEqual(approve(w,c).observation_state,'4')
        row.warnings=[]
        with self.assertRaisesRegex(ContractError,'TRABAJO_CAMBIADO_REVISAR'): approve(w,c)
        row.warnings=[TEMPORAL_NOTICE,'Q-03: contradicción ficticia']
        with self.assertRaises(ContractError): review(w)
        row.warnings=[TEMPORAL_NOTICE,TEMPORAL_NOTICE]
        with self.assertRaises(ContractError): review(w)
