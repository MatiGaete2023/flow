"""Genera los manuales HTML (y el aviso previo del instalador) desde docs/*.md."""

from pathlib import Path
import sys

import markdown

ROOT = Path(__file__).resolve().parent.parent
STYLE = """
body{font:16px/1.6 'Segoe UI',Arial,sans-serif;color:#20324d;background:#f3f6fa;margin:0}
main{max-width:860px;margin:32px auto;background:#fff;padding:32px 40px;border:1px solid #dce4ee;border-radius:12px}
h1{font-size:30px;margin-top:0}h2{margin-top:32px;border-bottom:1px solid #dce4ee;padding-bottom:4px}
code{background:#eef3f8;padding:1px 5px;border-radius:4px}pre{background:#eef3f8;padding:12px;border-radius:6px;overflow:auto}
table{border-collapse:collapse}td,th{border:1px solid #dce4ee;padding:6px 10px;text-align:left}
blockquote{border-left:4px solid #195fa8;margin:0;padding:4px 16px;background:#f3f8ff}
"""


def main(out):
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    for name in ('MANUAL_INSTALACION', 'MANUAL_USO'):
        text = (ROOT / 'docs' / f'{name}.md').read_text(encoding='utf-8')
        body = markdown.markdown(text, extensions=['tables', 'fenced_code', 'sane_lists'])
        title = text.splitlines()[0].lstrip('# ').strip()
        html = (
            f'<!doctype html><html lang="es"><meta charset="utf-8"><title>{title}</title>'
            f'<style>{STYLE}</style><main>{body}</main></html>'
        )
        (out / f'{name}.html').write_text(html, encoding='utf-8')
    (out / 'ANTES_DE_INSTALAR.txt').write_text(
        'SITFA Lite descarga desde tu propio Chrome los XLS que produce RUS/SITFA:\n'
        '  - Seguimiento (Informe seguimiento): pestañas, modalidades, informes, medidas y fechas.\n'
        '  - Seguimiento (Búsqueda litigantes): tribunal, estado y fechas.\n'
        '  - Informes por vencer y Medidas por vencer: tribunal, mes y año.\n'
        'Si una consulta no tiene registros guarda un pantallazo PDF de SITFA.\n\n'
        'No crea paquetes consolidados, no lee bitácoras, historias de la causa ni calendarios por ingreso,\n'
        'y no escribe en RUS. No lee contraseñas ni cookies.\n\n'
        'Se instala solo para tu usuario (no requiere administrador).\n'
        'Después de instalar debes cargar UNA VEZ la extensión de Chrome: el manual de instalación se abre al final.\n',
        encoding='utf-8-sig',  # Inno Setup lee como UTF-8 solo con BOM
    )


if __name__ == '__main__':
    main(sys.argv[1] if len(sys.argv) > 1 else ROOT / 'dist' / 'manuales')
