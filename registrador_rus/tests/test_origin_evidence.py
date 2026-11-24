from unittest import TestCase
import json
from registrador_rus.app.contracts import ContractError
from registrador_rus.app.flow_work import verify_origin


class OriginEvidenceTests(TestCase):
    def test_followup_tabs_and_report_catalog(self):
        base=dict(source_system='RUS',court_code='900000',query_id='demo-q')
        for origin,tab,report,label in (
            ('ESPERA','Espera',None,None),
            ('CUMPLIMIENTO','Cumplimiento',None,None),
            ('INFORMES_POR_VENCER','Informes','demo-p','Por vencer (a 25 días)'),
            ('INFORMES_VENCIDOS','Informes','demo-v','Vencidos')):
            source=base|dict(tab=tab)
            q=source|dict(screen='seguimiento',filters_json=json.dumps({'report':report}))
            verify_origin(source,[q],origin,report_catalog={report:label} if report else None)
            for bad in (q|dict(screen='calendario_informes'),q|dict(court_code='999999'),q|dict(tab='Egresados')):
                with self.assertRaises(ContractError): verify_origin(source,[bad],origin,report_catalog={report:label})
            with self.assertRaises(ContractError): verify_origin(source,[q,q],origin)
            if report:
                with self.assertRaises(ContractError): verify_origin(source,[q],origin)
                with self.assertRaises(ContractError): verify_origin(source,[q],origin,report_catalog={report:'Recibidos'})
