"""Cambio de pestaña: datos ficticios, sin acceso a SITFA."""
import contextlib
import io
import json
import unittest
from urllib.parse import urlencode

import motor as p
import test_poc as fixture


def espera_pairs():
    return list({**p.LAJA_ESPERA.target, "irAccion": "Buscar medida",
                 "NUM_PaginaEspera": "", "NUM_TotalEspera": "",
                 "NUM_PaginaCumplimiento": "2", "NUM_TotalCumplimiento": "3"}.items())


def espera_html(number=1, total=3, obsolete=True):
    result = (fixture.search_html(number, total)
              .replace('value="119" selected>Mulchen', 'value="113" selected>Laja')
              .replace('tablaCumplimiento', 'tablaEspera')
              .replace('NUM_PaginaCumplimiento', 'NUM_PaginaEspera')
              .replace('NUM_TotalCumplimiento', 'NUM_TotalEspera')
              .replace('demo_amblistcump.xls', 'demo_amblistesp.xls'))
    if obsolete:
        result = result.replace('</form>', '<input name="NUM_PaginaCumplimiento" value="2"><input name="NUM_TotalCumplimiento" value="7"></form>')
    return result


class EsperaTransport(fixture.FakeTransport):
    def __init__(self, total=3, fault=None):
        super().__init__(fault)
        self.total = total
        self.sent = []

    def search(self, values, timeout):
        self.sent.append(dict(values))
        self.current = int(dict(values)[p.LAJA_ESPERA.page_field] or '1')
        self.events.append(("POST", self.current))
        if self.fault == "page" and self.current == 2:
            return espera_html(1, self.total)
        return espera_html(self.current, self.total)


class Phase3Tests(unittest.TestCase):
    setUp = fixture.PocTests.setUp
    tearDown = fixture.PocTests.tearDown

    def test_active_tab_total_overrides_obsolete_other_tab(self):
        first = p.search_page(espera_html(1,1), espera_pairs(), profile=p.LAJA_ESPERA)
        self.assertEqual(first.current, 1)
        self.assertEqual(first.total, 1)
        transport = EsperaTransport(total=1)
        transport.search(espera_pairs(),60000)
        with contextlib.redirect_stdout(io.StringIO()):
            result = p.run_sequence(transport, espera_pairs(), first, self.folder, profile=p.LAJA_ESPERA)
        self.assertEqual(transport.events, [("POST",1),("XLS",1)])
        self.assertEqual(len(result), 1)

    def test_three_pages_use_espera_field_and_keep_filters(self):
        transport = EsperaTransport()
        pairs = espera_pairs()+[("COD_TiempoEspera","-1"),("FEC_Inicio","01/01/2026")]
        first = p.search_page(transport.search(pairs,60000), pairs, profile=p.LAJA_ESPERA)
        with contextlib.redirect_stdout(io.StringIO()):
            result = p.run_sequence(transport, pairs, first, self.folder, profile=p.LAJA_ESPERA)
        self.assertEqual(transport.events, [("POST",1),("XLS",1),("POST",2),("XLS",2),("POST",3),("XLS",3)])
        self.assertEqual([x["NUM_PaginaEspera"] for x in transport.sent], ["","2","3"])
        self.assertTrue(all(x["COD_Lengueta"]=="tdEspera" and x["COD_TiempoEspera"]=="-1" and x["FEC_Inicio"]=="01/01/2026" for x in transport.sent))
        self.assertTrue(all("Laja_Espera_Ambulatorio" in x["archivo"] for x in result))
        manifest = json.loads((self.folder/"verificacion.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["caso"], p.LAJA_ESPERA.key)
        self.assertNotIn("FEC_Inicio", str(manifest))

    def test_wrong_tab_and_template_rejected(self):
        with self.assertRaises(p.PocError):
            p.query_pairs(urlencode({**dict(espera_pairs()),"COD_Lengueta":"tdCumplimiento"}),p.LAJA_ESPERA)
        with self.assertRaises(p.PocError):
            p.search_page(espera_html().replace('demo_amblistesp.xls','demo_amblistcump.xls'), espera_pairs(), profile=p.LAJA_ESPERA)

    def test_missing_active_total_does_not_fall_back_to_other_tab(self):
        content=espera_html().replace('<input name="NUM_TotalEspera" value="3">','')
        with self.assertRaises(p.PocError): p.search_page(content, espera_pairs(), profile=p.LAJA_ESPERA)

    def test_wrong_page_stops_before_download(self):
        transport = EsperaTransport(fault="page")
        first = p.search_page(transport.search(espera_pairs(),60000), espera_pairs(), profile=p.LAJA_ESPERA)
        with contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(p.PocError): p.run_sequence(transport,espera_pairs(),first,self.folder,profile=p.LAJA_ESPERA)
        self.assertNotIn(("XLS",2),transport.events)

    def test_wrong_profile_transport_does_not_call_chrome(self):
        from lotes import ChromeTransport
        transport=ChromeTransport(None,p.LAJA_ESPERA)
        with self.assertRaises(p.PocError): transport.search(fixture.pairs(),1000)
        with self.assertRaises(p.PocError): transport.download('https://familia.pjud.cl/sitfa/reportes/demo_faefaslistcump.xls',1000)


if __name__ == "__main__":
    unittest.main()
