"""Pruebas locales con datos ficticios. No acceden a SITFA ni requieren sesion."""
import contextlib
import io
import os
from pathlib import Path
import shutil
import uuid
import unittest

import motor as p


def pairs():
    return list({**p.TARGET, "irAccion": "Buscar medida", p.PAGE_FIELD: "1", p.TOTAL_FIELD: "3"}.items())


def table(number, tag="table", metadata=""):
    return (f'<{tag} id="tablaCumplimiento"><thead><tr><th>ID</th><th>RIT</th><th>Nombre</th></tr></thead>'
            f'<tbody><tr><td>{number}</td><td>X-{number}-2026</td><td>Persona ficticia {number}</td></tr></tbody></{tag}>{metadata}')


def search_html(number=1, total=3):
    return (f'<html><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">'
            '<input name="COD_Lengueta" value=""><select name="COD_Tribunal_sel"><option value="119" selected>Mulchen</option></select>'
            '<select name="TIP_Consulta"><option value="2" selected>Ambulatorio</option></select>'
            f'<input name="{p.PAGE_FIELD}" value="{number}"><input name="{p.TOTAL_FIELD}" value="{total}">'
            + table(number) + '</form><script>function ShowExcel(){window.open("https://familia.pjud.cl/sitfa/reportes/demo_amblistcump.xls");}</script></html>')


def excel(number):
    return ('<html><meta charset="utf-8">'+table(number)+'</html>').encode()


class FakeTransport:
    def __init__(self, fault=None):
        self.events = []
        self.current = None
        self.fault = fault

    def search(self, values, timeout):
        self.current = int(dict(values)[p.PAGE_FIELD])
        self.events.append(("POST", self.current))
        if self.fault == "page" and self.current == 2:
            return search_html(1)
        if self.fault == "total" and self.current == 2:
            return search_html(2, 4)
        return search_html(self.current)

    def download(self, url, timeout):
        self.events.append(("XLS", self.current))
        if self.current == 2 and self.fault == "stale":
            return excel(1)
        if self.current == 2 and self.fault == "login":
            return b'<html><form><input type="password"></form></html>'
        if self.current == 2 and self.fault == "timeout":
            raise p.PocError("Timeout simulado.")
        return excel(self.current)


class PocTests(unittest.TestCase):
    def setUp(self):
        self.base = Path(os.environ.get("SITFA_TEST_TMP", Path.cwd()/"work")).resolve()
        self.base.mkdir(parents=True, exist_ok=True)
        self.temp = self.base / ("sitfa_test_"+uuid.uuid4().hex)
        self.temp.mkdir()
        self.folder = self.temp/"run"

    def tearDown(self):
        target = self.temp.resolve()
        if target.parent != self.base or not target.name.startswith("sitfa_test_"):
            raise RuntimeError("Destino temporal fuera de la carpeta de pruebas")
        shutil.rmtree(target)

    def run_sequence(self, transport):
        with contextlib.redirect_stdout(io.StringIO()):
            return p.run_sequence(transport, pairs(), p.search_page(transport.search(pairs(), 60000), pairs()), self.folder)

    def test_complete_and_strict_sequence(self):
        transport = FakeTransport()
        result = self.run_sequence(transport)
        self.assertEqual(transport.events, [("POST",1),("XLS",1),("POST",2),("XLS",2),("POST",3),("XLS",3)])
        self.assertEqual(len(result), 3)
        self.assertEqual(len(list(self.folder.glob("*.xls"))), 3)
        self.assertEqual(len({x["sha256"] for x in result}), 3)
        import json
        manifest = json.loads((self.folder/"verificacion.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["estado"], "VALIDADA")
        self.assertNotIn("demo_amblistcump", str(manifest))

    def test_stale_excel_stops_before_page_three(self):
        transport = FakeTransport("stale")
        with self.assertRaises(p.PocError): self.run_sequence(transport)
        self.assertNotIn(("POST",3), transport.events)
        self.assertEqual(len(list(self.folder.glob("*.xls"))), 1)

    def test_login_not_saved(self):
        transport = FakeTransport("login")
        with self.assertRaisesRegex(p.PocError, "Sesion expirada"): self.run_sequence(transport)
        self.assertEqual(len(list(self.folder.glob("*.xls"))), 1)

    def test_page_and_total_checks_before_download(self):
        for fault in ("page", "total"):
            with self.subTest(fault=fault):
                self.folder = self.temp/fault
                transport = FakeTransport(fault)
                with self.assertRaises(p.PocError): self.run_sequence(transport)
                self.assertNotIn(("XLS",2), transport.events)

    def test_no_retry_after_timeout(self):
        transport = FakeTransport("timeout")
        with self.assertRaises(p.PocError): self.run_sequence(transport)
        self.assertEqual(transport.events.count(("XLS",2)), 1)
        self.assertNotIn(("POST",3), transport.events)

    def test_initial_page_must_be_first_before_downloading(self):
        transport=FakeTransport()
        initial=p.search_page(search_html(2),pairs())
        with self.assertRaises(p.PocError):
            p.run_sequence(transport,pairs(),initial,self.folder)
        self.assertFalse(transport.events)
        self.assertFalse(list(self.folder.glob('*.xls')))

    def test_scope_and_unknown_fields(self):
        from urllib.parse import urlencode
        for extra in ({"irAccion":"Grabar"}, {"TIP_Consulta":"3"}, {"CSRF_token":"ficticio"}, {"COD_Tribunal_sel":"113"}):
            with self.subTest(extra=extra):
                with self.assertRaises(p.PocError): p.query_pairs(urlencode({**dict(pairs()), **extra}))

    def test_repeated_parameter(self):
        from urllib.parse import urlencode
        with self.assertRaises(p.PocError): p.query_pairs(urlencode(pairs()+[(p.PAGE_FIELD,"2")]))

    def test_url_scope(self):
        for bad in ("https://otro.example/export.xls", "https://familia.pjud.cl/sitfa/reportes/demo_amblistcump.xls?token=x",
                    "https://familia.pjud.cl/SITFAWEB/Grabar.do"):
            with self.subTest(url=bad):
                bad_html = search_html().replace("https://familia.pjud.cl/sitfa/reportes/demo_amblistcump.xls", bad)
                with self.assertRaises(p.PocError): p.search_page(bad_html, pairs())

    def test_same_records_despite_different_bytes(self):
        first = p.search_page(search_html(), pairs())
        altered = excel(1) + b'<!-- distinta fecha -->'
        with self.assertRaisesRegex(p.PocError, "mismos registros"):
            p.verified_download(altered, first, set(), [p.excel_record_rows(p.read_excel(excel(1)))])

    def test_same_case_and_name_with_different_measure_is_not_a_repeated_page(self):
        first = p.search_page(search_html(), pairs())
        original = excel(1).replace(b'</tr></thead>',b'<th>Programa</th></tr></thead>')
        original = original.replace(b'</td></tr></tbody>',b'</td><td>Programa A</td></tr></tbody>')
        changed = original.replace(b'Programa A',b'Programa B')
        previous = p.excel_record_rows(p.read_excel(original))
        info,_ = p.verified_download(changed,first,set(),[previous])
        self.assertEqual(info.records,first.records)
        self.assertNotEqual(p.excel_record_rows(info),previous)

    def test_export_entire_query_rejected(self):
        whole = excel(1).replace(b'</tbody>', b'<tr><td>2</td><td>X-2-2026</td><td>Persona ficticia 2</td></tr></tbody>')
        with self.assertRaisesRegex(p.PocError, "no coinciden"):
            p.verified_download(whole, p.search_page(search_html(), pairs()), set(), [])

    def test_corrupt_empty_active_html(self):
        for data in (b'', b'not an excel', b'<html><script>x()</script><table></table></html>'):
            with self.subTest(data=data):
                with self.assertRaises(p.PocError): p.read_excel(data)

    def test_no_overwrite(self):
        self.folder.mkdir()
        path = self.folder / "old.xls"
        path.write_bytes(b"original")
        with self.assertRaises(p.PocError): p.save_exclusive(path, b"nuevo")
        self.assertEqual(path.read_bytes(), b"original")


if __name__ == "__main__":
    unittest.main()
