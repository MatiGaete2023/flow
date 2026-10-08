"""Evidencias vacías, cambios de resultado, errores y nombres; datos ficticios."""
import base64
import hashlib
import json
from pathlib import Path
import threading
import unittest
import re
import zlib

import motor as m
from lotes import Batch,BatchRunner
from evidencias import screenshot_pdf
import test_lotes as fixtures
from test_lotes import FakeBridge,CATALOG,document


class EvidenceTests(unittest.TestCase):
    setUp=fixtures.BatchTests.setUp;tearDown=fixtures.BatchTests.tearDown;run_batch=fixtures.BatchTests.run_batch

    def test_empty_pdf_then_excel_sequentially_and_only_local_metadata(self):
        result,folder,bridge,events=self.run_batch(Batch(('49',),('1','2'),('Espera',)),FakeBridge(empty_first=True))
        pdf=folder/'ESP Residencia Jdo Familia Tome.pdf'
        self.assertTrue(pdf.read_bytes().startswith(b'%PDF-'))
        proof=result[0]['evidencia'];self.assertEqual(proof['sha256'],hashlib.sha256(pdf.read_bytes()).hexdigest())
        self.assertEqual(len(list(folder.glob('*.xls'))),3)
        self.assertEqual(len(list(folder.glob('*.pdf'))),1)
        self.assertEqual(events[-1][1]['pdfs'],1)
        names=[x[0] for x in bridge.events]
        self.assertLess(names.index('evidence_close'),names.index('download'))
        self.assertEqual(names[-1],'unlock')
        info=json.loads((folder/'verificacion 001.json').read_text(encoding='utf-8'))
        self.assertEqual(info['evidencia'],proof);self.assertEqual(info['archivos'],[])
        self.assertFalse(list(folder.glob('*.png')));self.assertFalse(list(folder.glob('*.html')))

    def test_pdf_contains_only_the_image_without_header_or_added_margin(self):
        bridge=FakeBridge();capture=bridge.call('evidence_capture',{'digest':'a'*64})
        pdf,_=screenshot_pdf(capture,'a'*64,'Etiqueta que no debe imprimirse')
        self.assertRegex(pdf,rb'/MediaBox\s*\[\s*0\s+0\s+450\s+300\s*\]')
        content_id=re.search(rb'/Contents\s+(\d+)\s+0\s+R',pdf).group(1)
        obj=re.search(rb'\b'+content_id+rb' 0 obj\b(.*?)endobj',pdf,re.S).group(1)
        stream=re.search(rb'stream\r?\n(.*?)endstream',obj,re.S).group(1).strip()
        content=zlib.decompress(base64.a85decode(stream,adobe=True))
        self.assertNotRegex(content,rb'\b(?:Tj|TJ)\b')
        self.assertRegex(content,rb'/FormXob\.[^ ]+ Do')

    def test_td_headers_and_empty_messages_do_not_interrupt_batch(self):
        class LegacyEmpty(FakeBridge):
            def call(self,command,payload=None,timeout=75):
                r=super().call(command,payload,timeout)
                if command in ('search','evidence_open') and self.queries==1:
                    p=fixtures.selection_profile(self.selection,CATALOG)
                    data=base64.b64decode(r['body']).replace(
                        f'<table id="{p.table_id}"></table>'.encode(),
                        f'<table id="{p.table_id}"><tr><td>RIT</td><td>Nombre</td></tr>'
                        '<tr><td colspan="2">No se encontraron registros.</td></tr></table>'
                        '<table id="tablaCumplimiento"><tr><td>RIT</td><td>Nombre</td></tr>'
                        '<tr><td colspan="2">Sin registros</td></tr></table>'.encode())
                    r['body']=base64.b64encode(data).decode()
                    if command=='evidence_open':r['digest']=hashlib.sha256(data).hexdigest()
                return r
        result,folder,_,events=self.run_batch(Batch(('49',),('1','2'),('Espera',)),LegacyEmpty(empty_first=True))
        self.assertEqual([r['estado'] for r in result],['SIN_RESULTADOS','VALIDADA'])
        self.assertEqual(len(list(folder.glob('*.pdf'))),1)
        self.assertEqual(len(list(folder.glob('*.xls'))),3)
        self.assertEqual(events[-1][1]['queries'],2)

    def test_td_headers_with_records_are_downloaded_completely(self):
        class LegacyHeaders(FakeBridge):
            def call(self,command,payload=None,timeout=75):
                r=super().call(command,payload,timeout)
                if command=='search':
                    data=base64.b64decode(r['body']).replace(b'<th>',b'<td>').replace(b'</th>',b'</td>')
                    # Cabecera antigua dentro de TBODY, tal como las filas de datos.
                    data=data.replace(b'<thead>',b'<tbody>').replace(b'</thead><tbody>',b'')
                    r['body']=base64.b64encode(data).decode()
                return r
        result,folder,_,_=self.run_batch(Batch(('49',),('1',),('Espera',)),LegacyHeaders())
        self.assertEqual(result[0]['paginas'],3)
        self.assertEqual(len(list(folder.glob('*.xls'))),3)

    def test_multiple_pages_for_same_person_with_different_programs_complete(self):
        class Measures(FakeBridge):
            def call(self,command,payload=None,timeout=75):
                r=super().call(command,payload,timeout)
                if command in ('search','download'):
                    data=base64.b64decode(r['body'])
                    data=data.replace(f'X-{self.number}-2026'.encode(),b'X-1-2026')
                    data=data.replace(f'Persona ficticia {self.number}'.encode(),b'Persona ficticia 1')
                    data=data.replace(f'<td>{self.number}</td>'.encode(),b'<td>1</td>')
                    if command=='download':
                        data=data.replace(b'</tr></thead>',b'<th>Programa</th></tr></thead>')
                        data=data.replace(b'</td></tr></tbody>',f'</td><td>Programa {self.number}</td></tr></tbody>'.encode())
                    r['body']=base64.b64encode(data).decode()
                return r
        result,folder,_,events=self.run_batch(Batch(('49',),('1',),('Espera',)),Measures(total=2))
        self.assertEqual(result[0]['paginas'],2)
        self.assertEqual(len(list(folder.glob('*.xls'))),2)
        self.assertEqual(events[-1][1]['queries'],1)

    def test_unknown_empty_table_row_is_not_treated_as_no_records(self):
        class Unknown(FakeBridge):
            def call(self,command,payload=None,timeout=75):
                r=super().call(command,payload,timeout)
                if command=='search':
                    data=base64.b64decode(r['body']).replace(b'</table>',b'<tr><td colspan="3">Respuesta no reconocida</td></tr></table>')
                    r['body']=base64.b64encode(data).decode()
                return r
        self.assert_failed_without_pdf(Unknown(empty_first=True),'identificar')

    def test_result_changed_between_initial_search_and_native_capture_stops_without_pdf(self):
        class Changed(FakeBridge):
            def call(self,command,payload=None,timeout=75):
                r=super().call(command,payload,timeout)
                if command=='evidence_open':
                    data=document(self.selection)
                    r.update(body=base64.b64encode(data).decode(),digest=hashlib.sha256(data).hexdigest())
                return r
        self.assert_failed_without_pdf(Changed(empty_first=True),'cambió')

    def assert_failed_without_pdf(self,bridge,message):
        events=[]
        with self.assertRaisesRegex(m.PocError,message):
            BatchRunner(bridge,CATALOG,lambda n,v:events.append((n,v)),threading.Event()).run(Batch(('49',),('1','2'),('Espera',)),self.temp)
        folder=Path(next(v for n,v in events if n=='folder'))
        self.assertFalse(list(folder.glob('*.pdf')));self.assertEqual(bridge.queries,1)
        self.assertEqual(bridge.events[-1][0],'unlock')
        self.assertEqual(json.loads((folder/'resumen.json').read_text(encoding='utf-8'))['estado'],'INCOMPLETA')

    def test_capture_error_stops_and_unlocks(self):
        class Failed(FakeBridge):
            def call(self,command,payload=None,timeout=75):
                if command=='evidence_capture':raise m.PocError('captura rechazada')
                return super().call(command,payload,timeout)
        self.assert_failed_without_pdf(Failed(empty_first=True),'captura rechazada')

    def test_foreign_tribunal_in_native_response_rejected(self):
        class Wrong(FakeBridge):
            def call(self,command,payload=None,timeout=75):
                r=super().call(command,payload,timeout)
                if command=='evidence_open':
                    data=base64.b64decode(r['body']).replace(b'name="COD_Tribunal_sel" value="49"',b'name="COD_Tribunal_sel" value="113"')
                    r.update(body=base64.b64encode(data).decode(),digest=hashlib.sha256(data).hexdigest())
                return r
        self.assert_failed_without_pdf(Wrong(empty_first=True),'seleccion')

    def test_informes_pdf_and_excel_same_base_get_distinct_ordinals(self):
        _,folder,_,_=self.run_batch(Batch(('49',),('1','2'),('Informes',),'2'),FakeBridge(empty_first=True))
        self.assertTrue((folder/'Informes Jdo Familia Tome 1.pdf').is_file())
        self.assertTrue((folder/'Informes Jdo Familia Tome 2.xls').is_file())
        self.assertTrue((folder/'Informes Jdo Familia Tome 4.xls').is_file())

    def test_invalid_or_wrong_capture_never_produces_pdf(self):
        digest='a'*64
        for response in ({'png':'not-base64','digest':digest},{'png':base64.b64encode(b'not PNG').decode(),'digest':digest},{'png':'','digest':'b'*64}):
            with self.subTest(response=response):
                with self.assertRaises(m.PocError):screenshot_pdf(response,digest,'Ficticio')


if __name__=='__main__':unittest.main()
