"""Genera Markdown, HTML autónomo y PDF del manual, sin conexiones externas.

  python tools/manual_descargador_publicar.py --demo-dir /tmp/manual-demo

El directorio de práctica debe proceder de manual_descargador_capturas.py.
Se publica únicamente la lista cerrada de fixtures; no se empaquetan datos ajenos.
"""
from __future__ import annotations

import argparse
import base64
import html
import json
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / 'docs/manual_descargador_2_7_4'
PDF_NAME = 'Manual_ilustrado_Descargador_SITFA_2.7.4.pdf'
HTML_NAME = 'Manual_ilustrado_Descargador_SITFA_2.7.4.html'
PRACTICE_NAME = 'Practica_ficticia_Descargador_2.7.4.zip'


def markdown(data):
    lines = ['# ' + data['titulo'], '', data['subtitulo'] + '.', '',
             '**Versión ' + data['version'] + ' · ' + data['fecha'] + '**', '',
             '[Descargar PDF](' + PDF_NAME + ') · [Abrir HTML sin conexión](' + HTML_NAME + ') · '
             '[Práctica ficticia](' + PRACTICE_NAME + ')', '',
             'En HTML puedes ampliar las imágenes. El PDF tiene una página por sección.', '', '## Índice', '']
    for i, page in enumerate(data['paginas'], 1):
        lines.append(f"- {page['titulo']} (página {i} del PDF)")
    for page in data['paginas']:
        lines += ['', '## ' + page['titulo'], '']
        if page.get('intro'): lines += [page['intro'], '']
        if page.get('tabla'):
            table = page['tabla']
            lines += ['| ' + ' | '.join(table['cabecera']) + ' |',
                      '| ' + ' | '.join('---' for _ in table['cabecera']) + ' |']
            lines += ['| ' + ' | '.join(row) + ' |' for row in table['filas']]
            lines += ['']
        for i, step in enumerate(page.get('pasos', []), 1): lines.append(f'{i}. {step}')
        if page.get('imagen'):
            lines += ['', f"![{page['pie']}](imagenes/{page['imagen']})", '', '*' + page['pie'] + '*']
        if page.get('nota'): lines += ['', '**Ten presente:** ' + page['nota']]
    lines += ['', '## Alcance y reproducción', '',
              'Capturas de la interfaz 2.7.4 en escritorio de prueba y Chromium. '
              'La conexión de demostración usa respuestas locales ficticias: cero consultas institucionales. '
              'Las instrucciones se contrastaron con los botones y exportaciones de esta versión. '
              'La práctica confirma 800 ingresos en Excel y un paquete con 4 registros de hojas '
              'y 2 bitácoras correspondientes; no sustituye la aceptación en SITFA real.', '',
              'Vista completa de la ventana principal (datos ficticios):', '',
              '![Ventana principal del Descargador](imagenes/01_ventana_principal.png)', '',
              'El bloque de conexión del Descargador:', '',
              '![Bloque Tu Chrome habitual](imagenes/04_conexion_descargador.png)', '',
              'Fuentes: [manual breve](../MANUAL_DESCARGADOR_MVP_20261008.md), '
              '[README del Descargador](../../descargador/README.md) y '
              '[correcciones de esta entrega](../REGRESIONES_PRUEBAS_USUARIO_20261009.md).', '',
              'Para actualizar capturas: `tools/manual_descargador_capturas.py`. '
              'Para regenerar este documento, HTML y PDF: `tools/manual_descargador_publicar.py`. '
              'El contenido editable está en `contenido.json`; los generadores requieren el entorno de desarrollo.']
    (DOCS / 'README.md').write_text('\n'.join(lines) + '\n', encoding='utf-8')


def webpage(data):
    escape = html.escape
    toc = ''.join(f'<a href="#paso-{i}">{escape(page["titulo"])}</a>' for i, page in enumerate(data['paginas'], 1))
    sections = []
    for i, page in enumerate(data['paginas'], 1):
        body = [f'<section id="paso-{i}"><p class="eyebrow">SECCIÓN {i} / {len(data["paginas"])}</p>',
                '<h2>' + escape(page['titulo']) + '</h2>']
        if page.get('intro'): body.append('<p class="intro">' + escape(page['intro']) + '</p>')
        if page.get('tabla'):
            table = page['tabla']; body.append('<div class="table-wrap"><table><thead><tr>')
            body += ['<th>' + escape(value) + '</th>' for value in table['cabecera']]
            body.append('</tr></thead><tbody>')
            for row in table['filas']: body.append('<tr>' + ''.join('<td>' + escape(value) + '</td>' for value in row) + '</tr>')
            body.append('</tbody></table></div>')
        if page.get('pasos'): body.append('<ol>' + ''.join('<li>' + escape(step) + '</li>' for step in page['pasos']) + '</ol>')
        if page.get('imagen'):
            image = base64.b64encode((DOCS / 'imagenes' / page['imagen']).read_bytes()).decode()
            body.append('<figure><button class="ampliar" aria-label="Ampliar captura" onclick="ampliar(this)"><img src="data:image/png;base64,' + image + '" alt="' + escape(page['pie'], quote=True) + '"></button><figcaption>' + escape(page['pie']) + ' Pulsa la imagen para ampliarla.</figcaption></figure>')
        if page.get('nota'): body.append('<aside><strong>Ten presente</strong><p>' + escape(page['nota']) + '</p></aside>')
        body.append('<a class="volver" href="#indice">Volver al índice ↑</a></section>')
        sections.append(''.join(body))
    overview = ''.join('<details><summary>' + label + '</summary><figure><img src="data:image/png;base64,' + base64.b64encode((DOCS / 'imagenes' / name).read_bytes()).decode() + '" alt="' + label + '"></figure></details>' for label, name in [('Ver la ventana principal completa (ejemplo ficticio)', '01_ventana_principal.png'), ('Ver dónde copiar el código en el Descargador', '04_conexion_descargador.png')])
    sections[0] = sections[0].replace('</section>', overview + '</section>')
    source = '''<!doctype html><html lang="es"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Manual ilustrado del Descargador SITFA 2.7.4</title><style>
:root{color-scheme:light;--ink:#17324d;--accent:#007e84}*{box-sizing:border-box}body{margin:0;background:#edf2f7;color:var(--ink);font:17px/1.65 system-ui,-apple-system,Segoe UI,sans-serif}header{background:#17324d;color:white;padding:48px max(24px,calc((100vw - 1100px)/2));border-bottom:8px solid #19b7b1}h1{font-size:clamp(30px,4vw,48px);line-height:1.15;margin:14px 0}h2{font-size:clamp(25px,3vw,32px);line-height:1.3;margin:8px 0 24px}header p{max-width:920px}main{max-width:1160px;margin:auto;padding:28px 24px}nav,section{background:white;padding:32px;margin:0 0 28px;border-radius:12px;box-shadow:0 4px 18px #17324d08}nav{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:8px 24px}nav h2{grid-column:1/-1;margin-bottom:8px}a{color:#006b79;text-underline-offset:4px}nav a{font-size:16px;padding:6px 0}.eyebrow{font-size:12px;letter-spacing:2px;font-weight:700;margin:0;color:#648196}header .eyebrow{color:#9edcdb}.intro{font-size:19px}li{padding-left:8px;margin:0 0 14px}ol{padding-left:28px}table{width:100%;border-collapse:collapse;font-size:16px}th{background:var(--ink);color:white;text-align:left}td,th{padding:14px;vertical-align:top;border-bottom:1px solid #dce6ee}tbody tr:nth-child(even){background:#f4f7fa}.table-wrap{overflow-x:auto;margin-bottom:22px}figure{margin:26px 0}img{max-width:100%;height:auto;display:block;border:1px solid #cfdae4}.ampliar{padding:0;border:0;background:transparent;cursor:zoom-in;width:100%;display:flex;justify-content:center}figcaption{color:#526b83;font-size:14px;margin:10px 0}aside{background:#eaf7f6;border-left:5px solid var(--accent);padding:18px 22px;margin:20px 0}aside p{margin:4px 0}.volver{font-size:14px}footer{text-align:center;color:#526b83;padding:20px}dialog{max-width:96vw;max-height:94vh;padding:14px;border:1px solid #adc1d1;border-radius:8px}dialog::backdrop{background:#0a213bdd}dialog button{display:block;margin:0 0 12px auto;padding:8px 18px;font:inherit;cursor:pointer}dialog img{max-width:none;cursor:zoom-out}dialog .zoom{overflow:auto;max-height:78vh;max-width:90vw}body:has(dialog[open]){overflow:hidden}@media(max-width:600px){main{padding:14px 10px}nav,section{padding:22px 16px}td,th{padding:10px;font-size:14px}}@media print{header{padding:18px}nav,.volver,.ampliar::after{display:none}main{padding:0}section{break-before:page;box-shadow:none}aside{break-inside:avoid}figure{break-inside:avoid}body{background:white;font-size:12px}}
</style></head><body><header><p class="eyebrow">GUÍA DE USO · VERSIÓN 2.7.4 · 9 DE OCTUBRE DE 2026</p><h1>Manual ilustrado del Descargador SITFA</h1><p>Desde una primera descarga hasta el paquete para el Asistente. Capturas de la interfaz con datos ficticios; puedes leer este archivo sin conexión.</p></header><main><nav id="indice"><h2>Elige el paso que necesitas</h2>TOC</nav>CONTENT</main><footer>Descargador 2.7.4 · Asistente 0.5.0.dev4 · Imágenes y práctica exclusivamente ficticias.</footer><dialog id="visor"><button onclick="document.getElementById('visor').close()">Cerrar imagen ✕</button><div class="zoom"><img alt="Captura ampliada" onclick="document.getElementById('visor').close()"></div></dialog><script>function ampliar(button){const d=document.getElementById('visor');const image=button.querySelector('img');d.querySelector('img').src=image.src;d.querySelector('img').alt=image.alt;d.showModal();}document.getElementById('visor').addEventListener('click',function(e){if(e.target===this)this.close();});</script></body></html>'''
    (DOCS / HTML_NAME).write_text(source.replace('TOC', toc).replace('CONTENT', ''.join(sections)), encoding='utf-8')


def pdf(data):
    from reportlab.lib import colors
    from reportlab.lib.enums import TA_LEFT
    from reportlab.lib.pagesizes import A4, landscape
    from reportlab.lib.styles import ParagraphStyle
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle, Image, PageBreak
    fontdir = Path('/usr/share/fonts/truetype/dejavu')
    if fontdir.is_dir():
        regular, bold = fontdir / 'DejaVuSans.ttf', fontdir / 'DejaVuSans-Bold.ttf'
    else:
        import os
        fontdir = Path(os.environ.get('WINDIR', 'C:/Windows')) / 'Fonts'
        regular, bold = fontdir / 'segoeui.ttf', fontdir / 'segoeuib.ttf'
    pdfmetrics.registerFont(TTFont('Manual', str(regular)))
    pdfmetrics.registerFont(TTFont('ManualBold', str(bold)))
    pdfmetrics.registerFontFamily('Manual', normal='Manual', bold='ManualBold', italic='Manual', boldItalic='ManualBold')
    ink = colors.HexColor('#17324d'); teal = colors.HexColor('#007e84')
    styles = {
        'heading': ParagraphStyle('heading', fontName='ManualBold', fontSize=21, leading=27, textColor=ink, spaceAfter=12),
        'body': ParagraphStyle('body', fontName='Manual', fontSize=11, leading=15, textColor=ink, spaceAfter=8),
        'step': ParagraphStyle('step', fontName='Manual', fontSize=11, leading=15, textColor=ink, leftIndent=20, firstLineIndent=-20, spaceAfter=7),
        'cell': ParagraphStyle('cell', fontName='Manual', fontSize=10.5, leading=14, textColor=ink),
        'th': ParagraphStyle('th', fontName='ManualBold', fontSize=10.5, leading=14, textColor=colors.white),
        'caption': ParagraphStyle('caption', fontName='Manual', fontSize=9, leading=12, textColor=colors.HexColor('#526b83'), spaceBefore=5, spaceAfter=8),
        'note': ParagraphStyle('note', fontName='Manual', fontSize=10.5, leading=14, textColor=ink, alignment=TA_LEFT),
    }
    class ManualDocument(SimpleDocTemplate):
        def afterFlowable(self, flowable):
            if isinstance(flowable, Paragraph) and flowable.style.name == 'heading':
                key = f'paso-{self.page}'
                self.canv.bookmarkPage(key)
                self.canv.addOutlineEntry(flowable.getPlainText(), key, level=0)

    doc = ManualDocument(str(DOCS / PDF_NAME), pagesize=landscape(A4),
        rightMargin=32, leftMargin=32, topMargin=47, bottomMargin=38,
        title=data['titulo'] + ' · ' + data['version'], author='Flow · documentación de uso',
        subject='Guía ilustrada con práctica ficticia; sin datos institucionales')
    width = doc.width - 12
    frame_height = doc.height - 12
    elements = []

    def paragraph(text, style='body'): return Paragraph(html.escape(text), styles[style])
    def height(items):
        return sum(item.wrap(width, frame_height)[1] + item.getSpaceBefore() + item.getSpaceAfter() for item in items)
    for page in data['paginas']:
        before = [paragraph(page['titulo'], 'heading')]
        if page.get('intro'): before.append(paragraph(page['intro']))
        if page.get('tabla'):
            table = page['tabla']; count = len(table['cabecera'])
            cols = [width * .28, width * .72] if count == 2 else ([width * .08, width * .49, width * .43] if table['cabecera'][0] == 'Paso' else [width * .28, width * .42, width * .30])
            rows = [[paragraph(v, 'th') for v in table['cabecera']]] + [[paragraph(v, 'cell') for v in row] for row in table['filas']]
            t = Table(rows, colWidths=cols, hAlign='LEFT')
            t.setStyle(TableStyle([('BACKGROUND', (0,0), (-1,0), ink), ('ROWBACKGROUNDS', (0,1), (-1,-1), [colors.white, colors.HexColor('#f1f5f9')]),
                ('VALIGN', (0,0), (-1,-1), 'TOP'), ('TOPPADDING', (0,0), (-1,-1), 7), ('BOTTOMPADDING', (0,0), (-1,-1), 7),
                ('LEFTPADDING', (0,0), (-1,-1), 9), ('RIGHTPADDING', (0,0), (-1,-1), 9), ('LINEBELOW', (0,0), (-1,-1), .4, colors.HexColor('#dce6ee'))]))
            before += [t, Spacer(1, 10)]
        for i, step in enumerate(page.get('pasos', []), 1): before.append(paragraph(f'{i}. {step}', 'step'))
        after = []
        if page.get('imagen'): after.append(paragraph(page['pie'], 'caption'))
        if page.get('nota'):
            note = Table([[Paragraph('<b>Ten presente:</b> ' + html.escape(page['nota']), styles['note'])]], colWidths=[width])
            note.setStyle(TableStyle([('BACKGROUND', (0,0), (-1,-1), colors.HexColor('#eaf7f6')), ('LINEBEFORE', (0,0), (0,0), 3, teal),
                ('TOPPADDING', (0,0), (-1,-1), 9), ('BOTTOMPADDING', (0,0), (-1,-1), 9), ('LEFTPADDING', (0,0), (-1,-1), 12), ('RIGHTPADDING', (0,0), (-1,-1), 12)]))
            after.append(note)
        elements += before
        available = frame_height - height(before) - height(after) - 12
        if page.get('imagen'):
            from PIL import Image as PILImage
            path = DOCS / 'imagenes' / page['imagen']
            with PILImage.open(path) as original: iw, ih = original.size
            image_height = min(page.get('alto_imagen', 280), available, width * ih / iw)
            if image_height < 135: raise ValueError('Texto excesivo junto a imagen: ' + page['titulo'])
            img = Image(str(path), width=iw * image_height / ih, height=image_height, hAlign='LEFT')
            elements += [Spacer(1, 4), img]
        elif available < 0: raise ValueError('Texto excede una página: ' + page['titulo'])
        elements += after
        elements.append(PageBreak())

    def footer(canvas, document):
        canvas.saveState()
        page_width, page_height = landscape(A4)
        canvas.setFillColor(ink); canvas.rect(0, page_height - 30, page_width, 30, fill=1, stroke=0)
        canvas.setFillColor(colors.white); canvas.setFont('ManualBold', 9)
        canvas.drawString(32, page_height - 19, 'DESCARGADOR SITFA · MANUAL DE USO')
        canvas.setFont('Manual', 9); canvas.drawRightString(page_width - 32, page_height - 19, '2.7.4 / ' + data['fecha'])
        canvas.setFillColor(colors.HexColor('#526b83')); canvas.setFont('Manual', 8)
        canvas.drawString(32, 20, 'Capturas y práctica con datos ficticios · Flow')
        canvas.drawRightString(page_width - 32, 20, f'{document.page} / {len(data["paginas"])}')
        canvas.restoreState()
    doc.build(elements, onFirstPage=footer, onLaterPages=footer)


def practice(demo):
    verification = json.loads((demo / 'VERIFICACION_FICTICIA.json').read_text(encoding='utf-8'))
    if verification.get('datos') != 'EXCLUSIVAMENTE FICTICIOS' or verification.get('peticiones_reales_sitfa') != 0:
        raise ValueError('Solo se publica la práctica verificada ficticia')
    names = ['01_Cumplimiento', '02_Informes_Por_vencer', '03_Bitacoras_anteriores']
    instructions = '''PRÁCTICA EXCLUSIVAMENTE FICTICIA — DESCARGADOR 2.7.4

Extrae todo el ZIP en una carpeta nueva. Los nombres, RIT y observaciones
son artificiales; este material sirve solo para aprender la parte local.
No necesita Chrome ni acceso institucional. Sigue la página 14 del manual.

1. Resultados… → Añadir bitácoras anteriores…
   Selecciona 03_Bitacoras_anteriores/bitacoras.json y conserva todos sus HTML.
2. Con ese lote seleccionado: Excel bitácoras autónomo… → guardar XLSX nuevo.
3. Añadir descarga o lote anterior… → carpeta 01_Cumplimiento.
4. Añadir descarga o lote anterior… → carpeta 02_Informes_Por_vencer.
5. Crear paquete para Asistente (ZIP)… → guardar ZIP nuevo.
6. En el Asistente: Trabajo → Importar paquete → ese ZIP.

Resultados: 800 ingresos en el Excel; 4 registros de hojas en el ZIP;
2 bitácoras correspondientes y 798 no correspondientes.
Retira del paquete otras fuentes disponibles para repetir los conteos.
No se incluyen productos precreados: tú los generarás con los botones.
Conserva los originales, especialmente resumen.json, verificación y HTML.
'''
    with zipfile.ZipFile(DOCS / PRACTICE_NAME, 'w', zipfile.ZIP_DEFLATED) as z:
        z.writestr('Practica_ficticia/LEEME.txt', instructions)
        z.writestr('Practica_ficticia/VERIFICACION_FICTICIA.json', json.dumps(verification, indent=2, ensure_ascii=False))
        for name in names:
            for path in sorted((demo / name).iterdir()):
                if not path.is_file() or path.is_symlink(): raise ValueError('Fixture inesperada')
                z.write(path, 'Practica_ficticia/' + name + '/' + path.name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--demo-dir', type=Path, help='Práctica producida por el capturador; opcional al regenerar texto/PDF')
    args = parser.parse_args()
    data = json.loads((DOCS / 'contenido.json').read_text(encoding='utf-8'))
    markdown(data); webpage(data); pdf(data)
    if args.demo_dir: practice(args.demo_dir.resolve())
    print('Manual generado: Markdown, HTML autónomo, PDF' + (' y práctica ficticia.' if args.demo_dir else '.'))


if __name__ == '__main__': main()
