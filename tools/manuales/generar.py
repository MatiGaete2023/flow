"""Manuales PDF ilustrados de las herramientas Flow a partir de las capturas de capturas.py.

    python tools/manuales/generar.py <capturas> <salida>

<capturas>/<producto>/raw/*.png + cajas.json → <salida>/img/<producto>/*.png (anotadas) y <salida>/<Manual>.pdf.
Necesita Pillow y Playwright con Chromium. Todas las capturas usan datos ficticios.
"""

import json
import sys
from html import escape
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROJO = (220, 38, 38)
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'src'))
from nurus.services.policy import MANDATORY_CC as CC  # noqa: E402

FUENTE = '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
FECHA = '3 de noviembre de 2026'


def fuente(size):
    try:
        return ImageFont.truetype(FUENTE, size)
    except OSError:
        return ImageFont.load_default()


def anotar(origen, cajas, destino, pad=6):
    im = Image.open(origen).convert('RGB')
    d = ImageDraw.Draw(im)
    for n, (x, y, w, h) in enumerate(cajas, 1):
        d.rounded_rectangle([x - pad, y - pad, x + w + pad, y + h + pad], radius=8, outline=ROJO, width=4)
        cx, cy = max(17, x - pad), max(17, y - pad)
        d.ellipse([cx - 15, cy - 15, cx + 15, cy + 15], fill=ROJO)
        d.text((cx, cy), str(n), font=fuente(18), fill='white', anchor='mm')
    destino.parent.mkdir(parents=True, exist_ok=True)
    im.save(destino, optimize=True)


CSS = """
@page{size:A4;margin:16mm 14mm}
body{font:10.5pt/1.5 'DejaVu Sans','Segoe UI',Arial,sans-serif;color:#1b2330;margin:0}
.cover{height:250mm;display:flex;flex-direction:column;justify-content:center;border-left:10px solid #2f5bd3;padding-left:24px}
.cover h1{font-size:30pt;margin:0 0 8px}.cover p{font-size:12.5pt;margin:4px 0;color:#5d6878}
.eyebrow{letter-spacing:.15em;color:#2f5bd3;font-weight:bold;font-size:9.5pt}
h2{font-size:16pt;border-bottom:2px solid #2f5bd3;padding-bottom:4px;margin:18px 0 10px;break-after:avoid}
h2.first{margin-top:0}
.step{break-inside:avoid;margin:0 0 16px}
.step h3{font-size:12pt;margin:0 0 6px}
.n{display:inline-block;background:#2f5bd3;color:#fff;border-radius:50%;width:24px;height:24px;text-align:center;line-height:24px;font-size:10.5pt;margin-right:6px}
img{display:block;width:100%;max-height:160mm;object-fit:contain;margin:8px auto;border:1px solid #dde2e9;border-radius:6px}
ol,ul{margin:4px 0 4px 20px;padding:0}li{margin:2px 0}
.note{background:#e8eefc;border-left:4px solid #2f5bd3;padding:8px 12px;margin:8px 0;break-inside:avoid}
.warn{background:#fff7ed;border-left:4px solid #b4540f;padding:8px 12px;margin:8px 0;break-inside:avoid}
table{border-collapse:collapse;width:100%;font-size:9.5pt;margin:6px 0}
td,th{border:1px solid #dde2e9;padding:5px 7px;text-align:left;vertical-align:top}th{background:#eef0f3}
.page{break-before:page}
code{font-family:'DejaVu Sans Mono',Consolas,monospace;font-size:9pt;background:#eef0f3;padding:0 4px;border-radius:3px}
"""


def pagina(titulo, version, subtitulo, cuerpo):
    return (
        f'<!doctype html><html lang="es"><meta charset="utf-8"><title>{escape(titulo)}</title><style>{CSS}</style>'
        f'<div class="cover"><p class="eyebrow">FLOW · {escape(version)} · MANUAL DE USO</p><h1>{escape(titulo)}</h1>'
        f'<p>{subtitulo}</p><p>{FECHA} · capturas con datos ficticios</p></div>{cuerpo}</html>'
    )


def paso(n, titulo, html, imagen=None):
    foto = f'<img src="{imagen}">' if imagen else ''
    return f'<div class="step"><h3><span class="n">{n}</span>{titulo}</h3>{html}{foto}</div>'


def seccion(titulo):
    # Solo «Qué hace» abre página nueva (tras la portada); el resto fluye y cada paso no se corta.
    if titulo == 'Qué hace':
        return f'<div class="page"></div><h2 class="first">{titulo}</h2>'
    return f'<h2>{titulo}</h2>'


def asistente(img):
    i = lambda n: f'{img}/asistente/{n}'  # noqa: E731
    c = seccion('Qué hace')
    c += f"""<p>El Asistente CSMP prepara el trabajo de seguimiento a partir del paquete del Descargador (o de la planilla RUS):
revisa cada registro con las reglas vigentes, propone correos y resoluciones, y los deja como <b>borradores</b>.
Nunca envía correos ni escribe en RUS: Outlook solo guarda borradores, con copia fija a {CC}.</p>
<table><tr><th>Paso</th><th>Para qué</th></tr>
<tr><td>1 Trabajo</td><td>Revisar registros, observaciones y avisos.</td></tr>
<tr><td>2 Correos</td><td>Preparar y guardar borradores de Outlook.</td></tr>
<tr><td>3 Resoluciones</td><td>Preparar y generar los proyectos Word.</td></tr>
<tr><td>4 Resultados</td><td>Abrir y entregar lo generado.</td></tr>
<tr><td>Configuración</td><td>Apariencia, parámetros, plantillas y contactos.</td></tr></table>
<div class="note">Atajos: <b>Ctrl+K</b> busca cualquier acción, <b>Ctrl+O</b> abre un Excel, <b>F5</b> actualiza el Excel,
<b>Ctrl+S</b> guarda y <b>Ctrl+Enter</b> aplica la edición. La barra lateral muestra cuánto queda en cada paso
(por ejemplo, «1 Trabajo · 4 avisos»).</div>"""
    c += seccion('1. Importar el paquete')
    c += paso(
        1,
        'Pulsa «Importar paquete»',
        """<p>Elige el <code>Captura_CSMP.zip</code> que entregó el Descargador
y confirma la fecha de corte. El Asistente crea una copia Excel revisable; el original no se modifica.</p>
<p>El botón <b>Más</b>, junto a «Importar paquete», tiene «Importar el último paquete», «Actualizar Excel» y «Archivo y
opciones…» (elegir un Excel, hoja y carpeta de salida).</p>""",
        i('01_inicio.png'),
    )
    c += seccion('2. Revisar en Trabajo')
    c += paso(
        2,
        'Cada registro con su estado y observación',
        """<ol>
<li>La tabla muestra estado, observación propuesta, RIT, nombre, programa y tribunal. Los avisos quedan coloreados.</li>
<li><b>Siguiente acción</b> lleva al paso que corresponde (por ejemplo, continuar a Correos).</li>
<li>El botón azul de arriba exporta la copia Excel actualizada.</li></ol>
<p>Al elegir un registro, abajo se edita la observación; <b>Aplicar</b> la guarda en el registro y <b>Guardar</b> guarda el
trabajo. «Qué requiere mi atención» y «Siguiente aviso» recorren solo lo pendiente.</p>""",
        i('02_trabajo.png'),
    )
    c += seccion('3. Correos')
    c += paso(
        3,
        'Preparar y guardar borradores',
        """<ol><li><b>Preparar todos</b> arma los correos de los registros
elegibles con la plantilla elegida; cada uno se puede editar.</li>
<li><b>Guardar borradores</b> los deja en Outlook sin enviarlos.</li></ol>
<div class="warn">El Asistente nunca envía. Revisa cada borrador en Outlook antes de enviarlo tú.</div>""",
        i('03_correos.png'),
    )
    c += seccion('4. Resoluciones')
    c += paso(
        4,
        'Preparar y generar Word',
        """<ol><li><b>Preparar / actualizar proyectos</b> arma un proyecto por
registro con el tipo sugerido; puedes cambiar el tipo y editar el texto.</li>
<li><b>Generar Word</b> crea los documentos; <b>Abrir Word</b> abre el último.</li></ol>
<p>Revisa siempre el formato final en Word antes de firmar.</p>""",
        i('04_resoluciones.png'),
    )
    c += seccion('5. Resultados y configuración')
    c += paso(
        5,
        'Abrir lo generado',
        """<p><b>Abrir producto</b> (1) abre el archivo elegido; <b>Preparar ZIP</b> y
<b>Abrir carpeta</b> entregan todo junto.</p>""",
        i('05_resultados.png'),
    )
    c += paso(
        6,
        'Configuración',
        """<p>Apariencia, parámetros, plantillas de correo, contactos y Outlook. Los contactos
se importan desde tu <code>contactos_base.json</code> o desde el catastro Excel.</p>""",
        i('06_configuracion.png'),
    )
    return pagina(
        'Asistente CSMP', 'Asistente 0.5.0.dev11', 'Trabajo · Correos · Resoluciones · Resultados · Configuración', c
    )


def descargador(img):
    i = lambda n: f'{img}/descargador/{n}'  # noqa: E731
    c = seccion('Qué hace')
    c += """<p>El Descargador SITFA consulta SITFA desde tu propio Chrome, descarga los XLS de RUS con todas sus páginas, lee
las bitácoras y crea el paquete <code>Captura_CSMP.zip</code> para el Asistente. No escribe en RUS.</p>
<div class="note">Instalación de la extensión (una vez): botón <b>Instalar extensión…</b> (o menú Más), activa el modo desarrollador
en <code>chrome://extensions</code>, pulsa «Cargar extensión sin empaquetar» y elige la carpeta <code>extension</code>. El programa y la
extensión deben tener la misma versión (2.9.0).</div>"""
    c += seccion('1. Conectar Chrome')
    c += paso(
        1,
        'Abrir SITFA y pegar el código',
        """<p>Tres pasos; cada uno muestra ✓ al cumplirse.</p>
<ol><li><b>Abrir mi Chrome en SITFA</b> e inicia sesión.</li>
<li><b>Copiar</b> el código; en la pestaña de SITFA pulsa el ícono de la extensión, pégalo y pulsa Conectar.</li>
<li><b>Instalar extensión…</b> solo la primera vez.</li></ol>
<p>Al conectar, esta pantalla desaparece y las opciones de SITFA se cargan solas.</p>""",
        i('01_sin_conexion.png'),
    )
    c += seccion('2. Elegir qué descargar')
    c += paso(
        2,
        'Consulta y filtros como fichas',
        """<ol><li>La lista de la izquierda elige la consulta:
<b>Seguimiento</b> crea el paquete; las demás se descargan sueltas.</li>
<li>Arriba a la derecha, el estado de Chrome.</li>
<li>Todo se marca con un clic en la misma pantalla: tribunales (con «Todos»), modalidad, registros, filtro de
cumplimiento o tipo de informe, período (sin filtro o por fecha, con calendario) y otros filtros. Una fecha mal escrita
se marca en rojo en el lugar.</li>
<li>Abajo, una sola acción: <b>Consultar y crear paquete</b> (o <b>Descargar</b> en las otras consultas). Enter la pulsa;
Esc detiene.</li></ol>""",
        i('02_seguimiento.png'),
    )
    c += seccion('3. Crear el paquete')
    c += paso(
        3,
        'Qué incluye el paquete',
        """<ol><li><b>Consultar y crear paquete</b>: consulta, lee bitácoras y entrega
el ZIP con sus originales en <code>respaldo</code>. Mientras trabaja aparecen <b>Pausar</b> y <b>Detener</b>.</li>
<li>Fila «Incluir»: XLSX consolidado, reutilizar bitácoras de hoy, calendario de informes, informes por vencer, un paquete
por hoja, <b>PDF si no hay registros</b>, historia de la causa (prueba) y meses de bitácora.</li>
<li>La carpeta de descarga se ve abajo a la izquierda; un clic la cambia.</li></ol>
<p>Con «PDF si no hay registros» desmarcado, la consulta vacía queda registrada sin pantallazo; el paquete lo informa como
<code>VACIA_SIN_PDF</code> y el Asistente lo muestra.</p>""",
        i('03_paquete.png'),
    )
    c += paso(
        4,
        'Retomar un paquete cortado',
        """<ol><li>Si un paquete reciente quedó sin terminar, al abrir el programa un aviso
ofrece <b>Retomar</b>: no repite lo ya validado ni las bitácoras leídas.</li>
<li>El menú <b>Más</b> reúne lo demás: descargar sin crear paquete, descarga conjunta CSMP, leer bitácoras del lote,
reintentar lecturas fallidas, crear paquete de un lote ya descargado, abrir carpetas, favoritas, <b>Retomar
paquete…</b> (para elegir otra carpeta), manual y video.</li></ol>""",
        i('04_retomar.png'),
    )
    c += seccion('4. Otras consultas')
    c += paso(
        5,
        'Litigantes, calendarios y resoluciones firmadas',
        """<p>Al elegir otra consulta en la lista (1), SITFA
abre esa pantalla y aparecen solo sus fichas: estado de órdenes (2) en litigantes, mes y año en los calendarios. La
acción principal pasa a <b>Descargar</b>.</p>""",
        i('05_litigantes.png'),
    )
    c += """<h2 style="margin-top:14px">Problemas frecuentes</h2><table><tr><th>Mensaje</th><th>Qué hacer</th></tr>
<tr><td>Sin conexión</td><td>En la pestaña de SITFA pulsa el ícono de la extensión y pega el código actual.</td></tr>
<tr><td>5 bitácoras seguidas fallaron</td><td>Revisa la sesión SITFA y usa <b>Más › Retomar paquete…</b>.</td></tr>
<tr><td>Falló la escritura local del archivo</td><td>El mensaje dice la causa (ruta larga, permiso o disco); elige otra
carpeta en Más › Carpeta de descarga.</td></tr></table>"""
    return pagina('Descargador SITFA', 'Descargador 2.9.0', 'Paquete CSMP · bitácoras · descargas sueltas', c)


def meta4(img):
    i = lambda n: f'{img}/meta4/{n}'  # noqa: E731
    c = seccion('Qué hace')
    c += """<p>Meta 4 arma la planilla Meta 4 del mes a partir de los lotes de registros y bitácoras que entregó el
Descargador. No se conecta a SITFA ni a RUS: solo lee carpetas locales.</p>"""
    c += seccion('1. Agregar los lotes')
    c += paso(
        1,
        'Agregar lote…',
        """<ol><li><b>Agregar lote…</b>: elige la carpeta de un lote del Descargador o una
carpeta con varios (FAE y Residencia, Espera y Cumplimiento). Desde la segunda vez aparece también <b>Usar el último
lote</b>.</li>
<li>El menú <b>Más</b> tiene quitar o vaciar lotes, bitácoras en otra carpeta, destino de resultados, regenerar desde
un Registros revisado, manual y video.</li></ol>""",
        i('01_inicio.png'),
    )
    c += paso(
        2,
        'Revisar lotes, tribunales y corte',
        """<ol><li>Cada lote dice cuántos lotes encontró y si trae
bitácoras (en naranja si faltan).</li>
<li>Tribunales: todas las fichas marcadas incluyen todos; si desmarcas alguno, la planilla queda marcada como parcial.
La ficha «Todos» marca o desmarca todos.</li>
<li>Fecha de corte DD/MM/AAAA, con calendario; se ajusta sola al día de lectura de las bitácoras y avisa en el lugar si
está mal escrita o es futura.</li>
<li><b>Generar Meta 4</b>.</li></ol>""",
        i('02_lotes.png'),
    )
    c += seccion('2. Resultado')
    c += paso(
        3,
        'Avisos y resultados',
        """<ol><li>Los avisos (por ejemplo, lectura de bitácoras incompleta) quedan en
la ventana; las filas afectadas quedan en amarillo en la planilla.</li>
<li>El resumen dice cuántos registros y alertas hubo; <b>Abrir resultados</b> abre la carpeta. Doble clic en un aviso
abre <code>Registros.xlsx</code>, con el detalle en la hoja ERRORES_Y_ADVERTENCIAS.</li></ol>""",
        i('03_generado.png'),
    )
    return pagina('Meta 4', 'Meta 4 2.6.0', 'Registros y bitácoras del Descargador → planilla Meta 4', c)


def anonimizador(img):
    i = lambda n: f'{img}/anonimizador/{n}'  # noqa: E731
    c = seccion('Qué hace')
    c += """<p>El Anonimizador crea copias anónimas de lo que entregan el Descargador y el Asistente (Excel, CSV, HTML,
JSON, Word, calendario, texto y ZIP) para compartirlas, por ejemplo, al informar un caso. El original no se modifica y
nada sale del equipo.</p>
<div class="warn">Ninguna modalidad garantiza anonimización absoluta: revisa la copia antes de compartirla.</div>"""
    c += seccion('1. Elegir modo y archivos')
    c += paso(
        1,
        'Modo y entradas',
        """<ol><li><b>Estricto</b> (recomendado) omite los textos libres completos;
<b>Conservador</b> mantiene observaciones sustituyendo identidades detectables. Bajo la barra, una línea explica el modo
elegido.</li>
<li><b>Agregar archivos…</b></li><li><b>Agregar carpeta…</b></li></ol>""",
        i('01_inicio.png'),
    )
    c += paso(
        2,
        'Anonimizar',
        """<ol><li>Las entradas se listan por número y tipo, sin nombres (pueden identificar
personas); el nombre aparece solo al pasar el mouse. <b>Supr</b> quita la entrada elegida.</li>
<li><b>Anonimizar</b>: la copia queda junto al original o en la carpeta de destino. El pie resume las opciones activas
(por ejemplo, «fechas +7 días · semilla definida»).</li>
<li>El menú <b>Más</b> tiene carpeta de destino, desplazar fechas, frase de semilla, fechas de nacimiento al 1 de enero,
ZIP estricto, quitar entradas, el último informe, manual y video.</li></ol>""",
        i('02_listo.png'),
    )
    c += seccion('2. Resultado')
    c += paso(
        3,
        'Copias e informe',
        """<ol><li>Cada entrada muestra si se creó su copia y cuántos cambios tuvo.</li>
<li>El aviso resume el resultado; <b>Abrir carpeta</b> muestra las copias y el
<code>informe_anonimizacion.txt</code>.</li></ol>""",
        i('03_hecho.png'),
    )
    return pagina('Anonimizador CSMP', 'Anonimizador 1.1.0', 'Copias anónimas locales para compartir', c)


MANUALES = {
    'asistente': ('Manual_Asistente_CSMP', asistente),
    'descargador': ('Manual_Descargador_SITFA', descargador),
    'meta4': ('Manual_Meta4', meta4),
    'anonimizador': ('Manual_Anonimizador_CSMP', anonimizador),
}


def main(capturas, salida):
    capturas, salida = Path(capturas), Path(salida)
    for producto in MANUALES:
        cajas = json.loads((capturas / producto / 'cajas.json').read_text(encoding='utf-8'))
        for nombre, lista in cajas.items():
            anotar(capturas / producto / 'raw' / nombre, lista, salida / 'img' / producto / nombre)
    html = []
    for producto, (nombre, armar) in MANUALES.items():
        path = salida / f'{nombre}.html'
        path.write_text(armar('img'), encoding='utf-8')
        html.append((path, salida / f'{nombre}.pdf'))
    from playwright.sync_api import sync_playwright

    with sync_playwright() as pw:
        navegador = pw.chromium.launch()
        pagina_web = navegador.new_page()
        for origen, destino in html:
            pagina_web.goto(origen.resolve().as_uri())
            pagina_web.wait_for_load_state('networkidle')
            pagina_web.pdf(
                path=str(destino),
                format='A4',
                print_background=True,
                display_header_footer=True,
                header_template='<span></span>',
                footer_template='<div style="font-size:8px;width:100%;text-align:center;color:#5d6878">'
                'Flow · <span class="title"></span> · página <span class="pageNumber"></span> de '
                '<span class="totalPages"></span></div>',
                margin={'top': '14mm', 'bottom': '16mm', 'left': '0', 'right': '0'},
            )
            origen.unlink()
        navegador.close()
    for _, destino in html:
        print(destino)


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
