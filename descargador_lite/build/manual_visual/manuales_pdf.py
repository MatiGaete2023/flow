"""Manuales ilustrados (HTML listos para imprimir a PDF). Uso: manuales_pdf.py <docs/img> <salida>."""

import sys
from html import escape
from pathlib import Path

VERSION = '1.0.0'
CSS = """
@page{size:A4;margin:16mm 14mm}
body{font:11pt/1.5 'DejaVu Sans','Segoe UI',Arial,sans-serif;color:#20324d;margin:0}
.cover{height:250mm;display:flex;flex-direction:column;justify-content:center;border-left:10px solid #195fa8;padding-left:24px}
.cover h1{font-size:32pt;margin:0 0 8px}.cover p{font-size:13pt;margin:4px 0}.eyebrow{letter-spacing:.15em;color:#195fa8;font-weight:bold;font-size:10pt}
h2{font-size:17pt;border-bottom:2px solid #195fa8;padding-bottom:4px;margin:0 0 10px;break-after:avoid}
.step{break-inside:avoid;margin:0 0 16px}
.step h3{font-size:12.5pt;margin:0 0 6px}.n{display:inline-block;background:#195fa8;color:#fff;border-radius:50%;width:24px;height:24px;text-align:center;line-height:24px;font-size:11pt;margin-right:6px}
img{display:block;max-width:100%;max-height:108mm;width:auto;margin:6px auto;border:1px solid #c9d4e2;border-radius:6px}
img.half{max-height:100mm}
ol,ul{margin:4px 0 4px 20px;padding:0}li{margin:2px 0}
.note{background:#f3f8ff;border-left:4px solid #195fa8;padding:8px 12px;margin:8px 0;break-inside:avoid}
.warn{background:#fff7ed;border-left:4px solid #d97706;padding:8px 12px;margin:8px 0;break-inside:avoid}
.path{font-family:'DejaVu Sans Mono',Consolas,monospace;background:#eef3f8;padding:8px 10px;border-radius:6px;font-size:10pt;word-break:break-all}
table{border-collapse:collapse;width:100%;font-size:9.5pt;margin:6px 0}td,th{border:1px solid #c9d4e2;padding:5px 7px;text-align:left;vertical-align:top}th{background:#eef3f8}
.page{break-before:page}
.red{color:#dc2626;font-weight:bold}
code{font-family:'DejaVu Sans Mono',Consolas,monospace;font-size:9.5pt;background:#eef3f8;padding:0 4px;border-radius:3px}
"""


def page(title, subtitle, body):
    return (
        f'<!doctype html><html lang="es"><meta charset="utf-8"><title>{escape(title)}</title><style>{CSS}</style>'
        f'<div class="cover"><p class="eyebrow">SITFA LITE {VERSION} · SOLO DESCARGAS RUS</p><h1>{escape(title)}</h1>'
        f'<p>{subtitle}</p></div>{body}</html>'
    )


def step(n, title, html, img=None, half=False):
    pic = f'<img class="{"half" if half else ""}" src="img/{img}">' if img else ''
    return f'<div class="step"><h3><span class="n">{n}</span>{title}</h3>{html}{pic}</div>'


RED = '<span class="red">●</span>'


def instalacion():
    body = '<div class="page"></div><h2>A. Instalar el programa</h2>'
    body += step(1, 'Ejecutar el instalador', """
<ol><li>Doble clic en <b>Instalar_SITFA_Lite_1.0.0.exe</b>.</li>
<li>Si aparece «Windows protegió su PC» (SmartScreen): <b>Más información</b> → <b>Ejecutar de todas formas</b>. Aparece porque el instalador no tiene firma digital comercial.</li>
<li>Lee el aviso inicial → <b>Siguiente</b> → deja la carpeta propuesta → marca «Crear un acceso directo en el escritorio» → <b>Instalar</b>.</li>
<li>Al terminar deja marcadas las casillas: se abre el manual de instalación y SITFA Lite.</li></ol>
<div class="note">No pide permisos de administrador: se instala en tu usuario, en
<div class="path">C:\\Users\\&lt;tu usuario&gt;\\AppData\\Local\\Programs\\SITFA_Lite</div>
El menú Inicio queda con la carpeta <b>SITFA Lite</b>: programa, manuales y «Carpeta de la extensión de Chrome».</div>
<div class="warn">Las ventanas del instalador son las estándar de Windows (Inno Setup); no se incluyen capturas porque este manual se generó fuera de Windows.</div>
""")
    body += '<div class="page"></div><h2>B. Cargar la extensión en Chrome (una sola vez)</h2>'
    body += step(2, 'En SITFA Lite pulsa «Instalar extensión…»', f"""
<p>Copia al portapapeles la ruta de la carpeta <code>extension</code> y abre <code>chrome://extensions</code>. Recuadro {RED} 2.</p>""",
                 'a0_inicio.png', half=True)
    body += step(3, 'Activa «Modo Desarrollador»', '<p>Interruptor arriba a la derecha de <code>chrome://extensions</code>.</p>',
                 'r01_extensiones.png')
    body += step(4, 'Pulsa «Cargar descomprimida»', """
<p>Aparece al activar el modo desarrollador. En la ventana de carpetas pega la ruta (<b>Ctrl+V</b> en la barra de dirección) y pulsa <b>Seleccionar carpeta</b>:</p>
<div class="path">C:\\Users\\&lt;tu usuario&gt;\\AppData\\Local\\Programs\\SITFA_Lite\\extension</div>""", 'r02_modo_desarrollador.png')
    body += step(5, 'Comprueba la tarjeta «SITFA Lite · Descargas RUS»', """
<p>Debe decir <b>1.0.0 Lite · solo descargas RUS</b> y estar activada (interruptor azul).</p>
<div class="warn">No muevas ni borres la carpeta <code>extension</code>: Chrome la usa desde ahí.</div>""", 'r03_extension_cargada.png')
    body += step(6, 'Fija el ícono en la barra', '<p>Pulsa el rompecabezas (1) y luego el alfiler (2) junto a SITFA Lite.</p>',
                 'r04_menu_extensiones.png')
    body += step(7, 'Ícono fijado', '<p>El ícono azul de descarga queda visible junto a la barra de direcciones.</p>',
                 'r05_fijada.png')
    body += '<div class="page"></div><h2>C. Conectar Chrome con SITFA Lite</h2>'
    body += step(8, 'Abre SITFA y copia el código', """
<ol><li><b>Abrir SITFA en Chrome</b> (1) e inicia sesión como siempre.</li>
<li><b>Copiar código</b> (3). El código cambia cada vez que abres el programa o pulsas Desconectar.</li></ol>""",
                 'a0_inicio.png', half=True)
    body += step(9, 'Pulsa el ícono SITFA Lite estando en la pestaña de SITFA', """
<p>Se abre la pestaña «Conectar con SITFA Lite». Pega el código (1) y pulsa <b>Conectar</b> (2). Deja esa pestaña abierta.</p>""",
                 'r07_codigo_pegado.png')
    body += step(10, 'Listo: «● Conectado»', '<p>El programa muestra el estado en verde (1). Sigue con el <b>Manual de uso</b>.</p>',
                 'a1_conectado.png', half=True)
    body += '<div class="page"></div><h2>D. Actualizar, desinstalar y problemas frecuentes</h2>'
    body += """
<p><b>Actualizar:</b> ejecuta el nuevo instalador y en <code>chrome://extensions</code> pulsa ↻ en la tarjeta SITFA Lite. El programa avisa si la extensión es de otra versión.</p>
<p><b>Desinstalar:</b> Configuración → Aplicaciones → SITFA Lite → Desinstalar; en <code>chrome://extensions</code> pulsa <b>Quitar</b>. Tus descargas no se borran.</p>
<table><tr><th>Síntoma</th><th>Solución</th></tr>
<tr><td>No aparece «Cargar descomprimida»</td><td>Activa Modo Desarrollador. Si tu equipo lo bloquea por política institucional, pide a soporte habilitarlo para esta extensión.</td></tr>
<tr><td>«Chrome usa otra versión de la extensión»</td><td>Quita la tarjeta antigua y carga la carpeta <code>extension</code> de esta instalación.</td></tr>
<tr><td>«El descargador rechazó el código»</td><td>Copia el código actual de la ventana abierta.</td></tr>
<tr><td>«No se encontró Chrome instalado»</td><td>Abre SITFA manualmente en Chrome; el resto funciona igual.</td></tr>
<tr><td>El antivirus bloquea el instalador</td><td>Verifica el SHA-256 publicado junto al instalador y pide la excepción a soporte.</td></tr></table>"""
    return page('Manual de instalación', 'Programa SITFA Lite + extensión de Chrome · 5 minutos, una sola vez · sin administrador', body)


def uso():
    body = '<div class="page"></div><h2>Qué hace y qué no</h2>'
    body += """
<p><b>Hace:</b> descarga los XLS que produce RUS/SITFA —todas las páginas— de 4 consultas, los nombra según la consulta y guarda un pantallazo PDF cuando no hay registros.</p>
<p><b>No hace:</b> paquetes consolidados, bitácoras, historia de la causa, calendario de informes por ingreso, ni escritura en RUS.</p>
<table><tr><th>Consulta</th><th>Opciones</th><th>Una descarga por cada…</th></tr>
<tr><td>Seguimiento · Informe seguimiento</td><td>Tribunales · Pestañas · Modalidades · Informe (pestaña Informes) · Medidas (pestaña Cumplimiento) · Fechas</td><td>tribunal × pestaña × modalidad × informe/medida</td></tr>
<tr><td>Seguimiento · Búsqueda litigantes</td><td>Tribunales · Estado · Fechas</td><td>tribunal × estado</td></tr>
<tr><td>Informes · Informes por vencer</td><td>Tribunales · Mes(es) · Año</td><td>tribunal × mes</td></tr>
<tr><td>Informes · Medidas por vencer</td><td>Tribunales · Mes(es) · Año</td><td>tribunal × mes</td></tr></table>
<div class="note">Antes de empezar: SITFA Lite debe mostrar <b>● Conectado</b> (ver Manual de instalación, sección C).</div>"""
    body += '<div class="page"></div><h2>1. Elegir la consulta</h2>'
    body += step(1, 'Marca la consulta y pulsa «Cargar opciones»', """
<p>SITFA abre esa pantalla en tu Chrome y el programa trae sus tribunales y filtros (2 y 3). El estado de conexión es (1).</p>""",
                 'a1_conectado.png')
    body += '<div class="page"></div><h2>2. Seguimiento · Informe seguimiento</h2>'
    body += step(2, 'Tribunales, pestañas y modalidades', """
<ol><li><b>Tribunales:</b> clic para marcar o desmarcar; <b>Buscar</b> filtra la lista; <b>Todos/Ninguno</b> actúa sobre lo visible.</li>
<li><b>Pestañas:</b> Espera, Cumplimiento, Informes, Egresados.</li>
<li><b>Modalidades:</b> Residencia, Ambulatorio, FAE, DCE.</li></ol>""", 'a2_opciones_seguimiento.png')
    body += step(3, 'Informe, medidas y fechas', """
<ol><li><b>Informe</b> (solo pestaña Informes): Por vencer, Vencidos, Recibidos.</li>
<li><b>Medidas</b> (solo pestaña Cumplimiento): Todas las medidas, Por vencer, Vencidos.</li>
<li><b>Filtrar por fecha</b> (opcional): Desde/Hasta DD/MM/AAAA, botón <b>Elegir…</b> o atajos. «Conservar los otros filtros preparados en SITFA»: prepara antes en SITFA el centro, RUT, causa o litigante y marca la casilla.</li></ol>
<p>Abajo se indica cuántas consultas se harán.</p>""", 'a3_informes_medidas_fechas.png')
    body += '<div class="page"></div><h2>3. Descargar</h2>'
    body += step(4, 'Pulsa «Descargar»', """
<p>(1) inicia el lote; (2) muestra cada archivo guardado y sus registros. Mientras descarga, la pestaña de SITFA queda cubierta con «Descargando el lote»: no la uses ni la cierres. <b>Cancelar</b> detiene al terminar la página en curso. Doble clic en un archivo lo abre; <b>Abrir carpeta del lote</b> abre la subcarpeta.</p>""",
                 'a4_descarga_terminada.png')
    body += '<div class="page"></div><h2>4. Búsqueda litigantes</h2>'
    body += step(5, 'Tribunales y estado', '<p>Marca tribunales (1) y uno o varios estados (2). Fechas opcionales, igual que en Seguimiento.</p>',
                 'a5_busqueda_lit.png')
    body += '<div class="page"></div><h2>5. Informes por vencer · Medidas por vencer</h2>'
    body += step(6, 'Mes(es) y año', '<p>Marca tribunales, uno o varios meses (1) y el año (2). Estas consultas no usan rango de fechas.</p>',
                 'a6_medidas_por_vencer.png')
    body += '<div class="page"></div><h2>6. Archivos que se generan</h2>'
    body += """
<p>Cada lote crea una subcarpeta con fecha y hora (<code>2026-10-30 09-15-02</code>) dentro de la carpeta elegida (por defecto <code>Documentos\\SITFA Lite</code>).</p>
<p class="path">Consulta - [Pestaña - Modalidad - Informe/Medidas] - Tribunal - [Fechas] - [pN de M].xls</p>
<table><tr><th>Ejemplo</th><th>Qué es</th></tr>
<tr><td><code>Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia Mulchen - 01-10-2026 al 30-10-2026 - p2 de 3.xls</code></td><td>Página 2 de 3 de esa consulta</td></tr>
<tr><td><code>Seguimiento - Informes - FAE - Por vencer (a 25 dias) - Jdo Familia Laja.xls</code></td><td>Sin filtro de fecha, una página</td></tr>
<tr><td><code>Seguimiento - Espera - Residencia - Jdo Familia Laja - SIN REGISTROS.pdf</code></td><td>Pantallazo: la consulta no tenía filas</td></tr>
<tr><td><code>Busqueda Lit - Pendientes - Jdo Familia Mulchen - 01-09-2026 al 30-09-2026.xls</code></td><td>Búsqueda litigantes</td></tr>
<tr><td><code>Informes por vencer - Noviembre 2026 - Jdo Familia Mulchen.xls</code></td><td>Calendario mensual</td></tr>
<tr><td><code>Medidas por vencer - Octubre 2026 - Jdo Familia Laja.xls</code></td><td>Calendario mensual</td></tr>
<tr><td><code>resumen.json</code>, <code>verificacion NNN.json</code></td><td>Estado del lote y huella SHA-256 de cada archivo</td></tr></table>
<div class="note"><b>Pantallazo SIN REGISTROS:</b> muestra la pantalla de SITFA con la selección aplicada y una franja azul «Consulta solicitada · Tribunal … · Pestaña … · Fechas …». Para capturarlo, la ventana de Chrome debe estar visible (no minimizada).</div>
<h2 style="margin-top:18px">7. Controles y errores</h2>
<p>Cada página se verifica antes de pedir la siguiente (tribunal, pestaña, modalidad y filtro correctos; XLS con las mismas causas que la página; sin páginas repetidas; archivo releído desde disco). Si algo no calza, el lote se detiene y lo informa; lo ya verificado se conserva. Nunca sobrescribe archivos.</p>
<table><tr><th>Mensaje</th><th>Qué hacer</th></tr>
<tr><td>Sesión expirada</td><td>Inicia sesión de nuevo en SITFA, reconecta desde el ícono y repite el lote.</td></tr>
<tr><td>No se pudo capturar la consulta vacía</td><td>Deja Chrome visible durante el lote.</td></tr>
<tr><td>● Sin conexión</td><td>Pulsa el ícono SITFA Lite en la pestaña de SITFA y pega el código actual.</td></tr>
<tr><td>Lote cancelado / detenido</td><td>Los archivos guardados quedan; repite en un lote nuevo.</td></tr></table>
<p><b>Privacidad:</b> la extensión usa tu sesión de Chrome; el programa no ve contraseñas ni cookies. Comunicación solo local (127.0.0.1) con código de conexión. Los archivos quedan solo en tu equipo.</p>"""
    return page('Manual de uso', 'Seguimiento · Búsqueda litigantes · Informes por vencer · Medidas por vencer', body)


def main(img, out):
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    link = out / 'img'
    if not link.exists():
        link.symlink_to(Path(img).resolve())
    (out / 'SITFA_Lite_Manual_Instalacion.html').write_text(instalacion(), encoding='utf-8')
    (out / 'SITFA_Lite_Manual_Uso.html').write_text(uso(), encoding='utf-8')


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
