"""Respuestas ficticias de las cuatro pantallas de SITFA Lite y resultados del validador Python.

No se conecta a SITFA. Uso desde la raíz: python3 rust/oraculo/generar_sitfa.py
"""

import json
import sys
from dataclasses import asdict
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'descargador_lite/app'))
sys.path.insert(0, str(RAIZ / 'descargador_lite/tests'))

import motor as m  # noqa: E402
from perfiles import Selection, response_profile  # noqa: E402
from sitfa_ficticio import CATALOGS, FakeBridge  # noqa: E402


class PuenteFicticio(FakeBridge):
    def rows(self, p, page):
        # Año reservado por tools/check_contactos.py, también en calendarios de 2026.
        return [(rit.rsplit('-', 1)[0] + '-2099', nombre) for rit, nombre in super().rows(p, page)]


def main():
    casos = []
    selecciones = [
        Selection('901', 'Espera', '2'),
        Selection('901', 'Cumplimiento', '2', '4'),
        Selection('901', 'Informes', '2', '1'),
        Selection('901', 'Egresados', '2'),
        Selection('901', '', '', screen='litigantes'),
        Selection('901', '', '', screen='calendario_informes', month='10', year='2026'),
        Selection('901', '', '', screen='calendario_medidas', month='10', year='2026'),
    ]
    for seleccion in selecciones:
        for vacia in (False, True):
            s = asdict(seleccion)
            puente = PuenteFicticio(seleccion.screen, default_pages=0 if vacia else 2)
            pares = [tuple(p) for p in puente.prepare(s, None)]
            html = puente.page_html(dict(pares))
            perfil, pagina = response_profile(html.encode(), pares, seleccion, CATALOGS[seleccion.screen])
            caso = {
                'nombre': seleccion.screen + '/' + seleccion.tab + ('/vacia' if vacia else ''),
                'seleccion': s, 'catalogo': CATALOGS[seleccion.screen], 'pares': pares, 'html': html,
                'registros': [[rit, nombre, cantidad] for (rit, nombre), cantidad in sorted(pagina.records.items())] if pagina else [],
            }
            if pagina:
                excel = puente.excel(dict(pares))
                m.verified_download(excel, pagina, set(), [], perfil)
                caso.update(actual=pagina.current, total=pagina.total, url=pagina.excel_url, excel=excel.decode())
                if not seleccion.screen.startswith('calendario_'):
                    siguientes = [(k, v) for k, v in pares if k not in m.PAGINATION]
                    siguientes += [(k, v) for k, v in pagina.pagination.items() if k in m.SCREEN_FIELDS[seleccion.screen]]
                    siguientes = [(k, v) for k, v in siguientes if k not in (perfil.page_field, perfil.total_field)]
                    siguientes += [(perfil.page_field, '2'), (perfil.total_field, str(pagina.total))]
                    segundo = puente.page_html(dict(siguientes))
                    m.search_page(segundo.encode(), siguientes, 2, pagina.total, perfil)
                    caso.update(html2=segundo, excel2=puente.excel(dict(siguientes)).decode())
            casos.append(caso)
    salida = RAIZ / 'rust/crates/flow-captura/tests/datos'
    salida.mkdir(parents=True, exist_ok=True)
    (salida / 'sitfa_ficticio.json').write_text(json.dumps(casos, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    selector = (RAIZ / 'descargador_lite/tests/selector_vencimiento_sitfa.js').read_text(encoding='utf-8')
    (salida / 'selector_vencimiento.js').write_text(
        '\n'.join(linea.expandtabs(4).rstrip() for linea in selector.splitlines()) + '\n', encoding='utf-8')
    print(f'{len(casos)} respuestas ficticias comprobadas con el validador Python.')


if __name__ == '__main__':
    main()
