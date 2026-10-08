"""Pruebas de aislamiento y secuencia del nuevo caso, con datos ficticios."""
import contextlib
import io
import json
import unittest
from urllib.parse import urlencode

import motor as p
import test_poc as fixture


def fae_pairs():
    return list({**dict(fixture.pairs()), **p.LAJA_FAE.target}.items())


def fae_html(number=1, total=3):
    return (fixture.search_html(number, total)
            .replace('value="119" selected>Mulchen', 'value="113" selected>Laja')
            .replace('value="2" selected>Ambulatorio', 'value="3" selected>FAE-FAS')
            .replace('demo_amblistcump.xls', 'demo_faefaslistcump.xls'))


class FaeTransport(fixture.FakeTransport):
    def search(self, values, timeout):
        raw = super().search(values, timeout)
        return (raw.replace('value="119" selected>Mulchen', 'value="113" selected>Laja')
                .replace('value="2" selected>Ambulatorio', 'value="3" selected>FAE-FAS')
                .replace('demo_amblistcump.xls', 'demo_faefaslistcump.xls'))


class Phase2Tests(unittest.TestCase):
    setUp = fixture.PocTests.setUp
    tearDown = fixture.PocTests.tearDown

    def test_profiles_do_not_accept_each_other(self):
        with self.assertRaises(p.PocError): p.query_pairs(urlencode(fixture.pairs()), p.LAJA_FAE)
        with self.assertRaises(p.PocError): p.query_pairs(urlencode(fae_pairs()), p.ORIGINAL)

    def test_wrong_excel_template_rejected(self):
        content = fae_html().replace('demo_faefaslistcump.xls', 'demo_amblistcump.xls')
        with self.assertRaises(p.PocError): p.search_page(content, fae_pairs(), profile=p.LAJA_FAE)

    def test_three_fictitious_pages_and_manifest(self):
        transport = FaeTransport()
        captured = fae_pairs()+[("FEC_Inicio", "01/01/2026"), ("FEC_Fin", "30/09/2026")]
        first = p.search_page(transport.search(captured,60000), captured, profile=p.LAJA_FAE)
        with contextlib.redirect_stdout(io.StringIO()):
            result = p.run_sequence(transport, captured, first, self.folder, profile=p.LAJA_FAE)
        self.assertEqual(transport.events, [("POST",1),("XLS",1),("POST",2),("XLS",2),("POST",3),("XLS",3)])
        self.assertEqual(len(result), 3)
        self.assertTrue(all("Laja_Cumplimiento_FAE-FAS" in item["archivo"] for item in result))
        manifest = json.loads((self.folder/"verificacion.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["caso"], p.LAJA_FAE.key)
        self.assertEqual(manifest["consulta"], p.LAJA_FAE.label)
        self.assertEqual(manifest["estado"], "VALIDADA")
        self.assertNotIn("FEC_Inicio", str(manifest))
        self.assertNotIn("faefaslistcump", str(manifest))

    def test_fae_transport_rejects_other_profile_before_chrome(self):
        from lotes import ChromeTransport
        t = ChromeTransport(None, p.LAJA_FAE)
        with self.assertRaises(p.PocError): t.search(fixture.pairs(), 1000)
        with self.assertRaises(p.PocError): t.download("https://familia.pjud.cl/sitfa/reportes/demo_amblistcump.xls", 1000)



if __name__ == "__main__":
    unittest.main()
