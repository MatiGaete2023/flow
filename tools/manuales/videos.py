"""Videos narrados (voz local Kokoro, es-419) de las herramientas Flow, con las capturas de los manuales.

    python tools/manuales/videos.py <capturas> <docs/manuales> <modelo_kokoro> <salida>

<capturas>/<producto>/cajas.json da los recuadros y raw/ las capturas sin marcas (el video dibuja un recuadro por paso).
El modelo (kokoro-v1.0.onnx, voices-v1.0.bin) se descarga de github.com/thewh1teagle/kokoro-onnx/releases.
Reutiliza las escenas animadas de descargador_lite/build/manual_visual (video.py y narrado.py).
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'descargador_lite' / 'build' / 'manual_visual'))
from PIL import Image, ImageDraw  # noqa: E402

import video  # noqa: E402
from narrado import build  # noqa: E402

PRODUCTOS = {
    'asistente': ('Asistente CSMP', 'Asistente 0.5.0.dev11', 'Flow_Video_Asistente_CSMP.mp4'),
    'descargador': ('Descargador SITFA', 'Descargador 2.9.0', 'Flow_Video_Descargador_SITFA.mp4'),
    'meta4': ('Meta 4', 'Meta 4 2.6.0', 'Flow_Video_Meta4.mp4'),
    'anonimizador': ('Anonimizador CSMP', 'Anonimizador 1.1.0', 'Flow_Video_Anonimizador_CSMP.mp4'),
}


def tarjeta(cabecera, titulo, lineas, segundos):
    """Portada o tarjeta de texto con la cabecera del producto (la de video.py dice SITFA Lite)."""
    W, H = video.W, video.H
    frame = Image.new('RGB', (W, H), (243, 246, 250))
    d = ImageDraw.Draw(frame)
    d.rectangle([0, 0, 14, H], fill=video.BLUE)
    d.text((80, 150), cabecera.upper(), font=video.font(video.BOLD, 20), fill=video.BLUE)
    y = 200
    for line in video.wrap(d, titulo, video.font(video.BOLD, 46), W - 160):
        d.text((80, y), line, font=video.font(video.BOLD, 46), fill=video.DARK)
        y += 60
    y += 20
    for texto in lineas:
        for line in video.wrap(d, texto, video.font(video.REG, 26), W - 160):
            d.text((80, y), line, font=video.font(video.REG, 26), fill=video.DARK)
            y += 40
    for _ in range(int(segundos * video.FPS)):
        yield frame


def guion(producto, capturas, manuales):
    cajas = json.loads((capturas / producto / 'cajas.json').read_text(encoding='utf-8'))
    nombre, version, _ = PRODUCTOS[producto]
    cabecera = f'Flow · {version}'

    def sc(imagen, n, texto, paso, total, zoom=1.5):
        caja = tuple(cajas[imagen][n - 1]) if n else None
        ruta = capturas / producto / 'raw' / imagen
        return lambda s: video.scene(ruta, caja, texto, paso, total, seconds=max(s, 4.2), zoom=zoom)

    def cd(titulo, lineas):
        return lambda s: tarjeta(cabecera, titulo, lineas, max(s, 3.0))

    if producto == 'asistente':
        t = 6
        return [
            (
                'Este video muestra el Asistente CSMP. Prepara el trabajo de seguimiento desde el paquete del Descargador, '
                'propone correos y resoluciones, y lo deja todo como borrador. Nunca envía correos ni escribe en Rus.',
                cd(
                    nombre,
                    [
                        'Paquete del Descargador → revisión → correos y resoluciones en borrador.',
                        'No envía correos ni escribe en RUS.',
                    ],
                ),
            ),
            (
                'Para empezar, pulsa Importar paquete, elige el archivo Captura CSMP punto zip y confirma la fecha de corte.',
                sc('01_inicio.png', 1, 'Importar paquete y confirmar la fecha de corte.', 1, t),
            ),
            (
                'En Trabajo ves cada registro con su estado y la observación propuesta. Los que tienen aviso quedan coloreados.',
                sc('02_trabajo.png', 1, 'Cada registro con su estado y observación.', 2, t, zoom=1.3),
            ),
            (
                'El botón de abajo indica la siguiente acción. Por ejemplo, continuar a Correos.',
                sc('02_trabajo.png', 2, 'Siguiente acción.', 3, t),
            ),
            (
                'En Correos, Preparar todos arma los correos de los registros elegibles. Guardar borradores los deja en Outlook, '
                'sin enviarlos. Revisa cada uno antes de enviarlo tú.',
                sc('03_correos.png', 1, 'Preparar todos y guardar borradores (no envía).', 4, t),
            ),
            (
                'En Resoluciones, prepara los proyectos y pulsa Generar Word. Revisa siempre el formato final en Word.',
                sc('04_resoluciones.png', 2, 'Preparar proyectos y generar Word.', 5, t),
            ),
            (
                'En Resultados abres lo generado, o preparas un zip con todo.',
                sc('05_resultados.png', 1, 'Abrir producto o preparar ZIP.', 6, t),
            ),
            (
                'Un consejo: control más ka busca cualquier acción. Los atajos de cada botón están ahí.',
                cd(
                    'Atajos',
                    [
                        'Ctrl+K busca cualquier acción.',
                        'Ctrl+O abre un Excel · F5 actualiza el Excel.',
                        'Ctrl+S guarda · Ctrl+Enter aplica la edición.',
                    ],
                ),
            ),
        ]
    if producto == 'descargador':
        t = 5
        return [
            (
                'Este video muestra el Descargador Sitfa. Consulta Sitfa desde tu Chrome, descarga las planillas de Rus con todas '
                'sus páginas, lee las bitácoras y crea el paquete para el Asistente.',
                cd(
                    nombre,
                    ['Consulta SITFA desde tu Chrome · bitácoras · paquete Captura_CSMP.zip.', 'No escribe en RUS.'],
                ),
            ),
            (
                'Primero conecta Chrome. Pulsa Abrir mi Chrome en Sitfa, inicia sesión, copia el código y pégalo en la extensión.',
                sc('01_sin_conexion.png', 1, 'Abrir SITFA y copiar el código.', 1, t),
            ),
            (
                'Al conectar, las opciones se cargan solas. A la izquierda eliges la consulta: Seguimiento crea el paquete.',
                sc('02_seguimiento.png', 1, 'La consulta se elige en la lista de la izquierda.', 2, t, zoom=1.3),
            ),
            (
                'Elige tribunales y modalidades. Las fechas son opcionales, con calendario.',
                sc('02_seguimiento.png', 3, 'Tribunales, modalidades y fechas.', 3, t),
            ),
            (
                'Con Consultar y crear paquete, el programa consulta, lee las bitácoras y entrega el zip. Las fichas definen qué '
                'se incluye, por ejemplo el pe de efe cuando una consulta no tiene registros.',
                sc('03_paquete.png', 2, 'Consultar y crear paquete · fichas de lo que se incluye.', 4, t, zoom=1.3),
            ),
            (
                'Si un paquete se corta, en el menú Más usa Retomar paquete. No repite lo que ya estaba listo.',
                sc('04_avanzado.png', 1, 'Avanzado y recuperación · Más › Retomar paquete…', 5, t, zoom=1.2),
            ),
            (
                'Eso es todo. Mientras descarga, deja abiertas la pestaña de Sitfa y la de conexión.',
                cd(
                    'Listo',
                    [
                        'Deja abiertas la pestaña de SITFA y la de conexión mientras descarga.',
                        'Manual: Manual_Descargador_SITFA.pdf.',
                    ],
                ),
            ),
        ]
    if producto == 'meta4':
        t = 4
        return [
            (
                'Este video muestra Meta 4. Arma la planilla Meta 4 del mes con los lotes y bitácoras del Descargador. '
                'No se conecta a Sitfa ni a Rus.',
                cd(nombre, ['Lotes y bitácoras del Descargador → planilla Meta 4.', 'Solo lee carpetas locales.']),
            ),
            (
                'Pulsa Agregar lote y elige la carpeta de un lote, o una carpeta con varios.',
                sc('01_inicio.png', 1, 'Agregar lote…', 1, t),
            ),
            (
                'Cada lote dice si trae bitácoras. Con todas las fichas de tribunales marcadas, se incluyen todos.',
                sc('02_lotes.png', 1, 'Lotes con sus bitácoras · tribunales en fichas.', 2, t, zoom=1.3),
            ),
            (
                'Revisa la fecha de corte y pulsa Generar Meta 4.',
                sc('02_lotes.png', 4, 'Fecha de corte y Generar Meta 4.', 3, t),
            ),
            (
                'Los avisos quedan en la ventana, y las filas afectadas en amarillo. Abrir resultados muestra la carpeta.',
                sc('03_generado.png', 1, 'Avisos y resultados.', 4, t, zoom=1.3),
            ),
        ]
    t = 3
    return [
        (
            'Este video muestra el Anonimizador. Crea copias anónimas de lo que entregan el Descargador y el Asistente. '
            'El original no se modifica y nada sale del equipo.',
            cd(nombre, ['Copias anónimas locales para compartir.', 'Revisa siempre la copia antes de compartirla.']),
        ),
        (
            'Elige el modo. Estricto omite los textos libres. Conservador mantiene observaciones, sustituyendo identidades.',
            sc('01_inicio.png', 1, 'Estricto o Conservador.', 1, t),
        ),
        ('Agrega archivos o una carpeta, y pulsa Anonimizar.', sc('02_listo.png', 2, 'Agregar y Anonimizar.', 2, t)),
        (
            'Cada entrada muestra si se creó su copia y cuántos cambios tuvo. Abrir carpeta muestra las copias y el informe.',
            sc('03_hecho.png', 1, 'Copias e informe.', 3, t, zoom=1.3),
        ),
    ]


def main(capturas, manuales, modelo, salida, voz='ef_dora'):
    from kokoro_onnx import Kokoro

    capturas, manuales, salida = Path(capturas), Path(manuales), Path(salida)
    salida.mkdir(parents=True, exist_ok=True)
    tts = Kokoro(str(Path(modelo) / 'kokoro-v1.0.onnx'), str(Path(modelo) / 'voices-v1.0.bin'))
    for producto, (_, _, archivo) in PRODUCTOS.items():
        segundos = build(guion(producto, capturas, manuales), voz, tts, salida / archivo)
        print(archivo, round(segundos, 1), 's')


if __name__ == '__main__':
    main(*sys.argv[1:6])
