"""Videos narrados (voz neuronal Kokoro, local) a partir de las mismas capturas de video.py.

Uso: narrado.py <capturas> <docs/img> <modelo_kokoro> <salida_dir> [voz]
El modelo (kokoro-v1.0.onnx, voices-v1.0.bin) se descarga de github.com/thewh1teagle/kokoro-onnx/releases.
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf
from kokoro_onnx import Kokoro
from PIL import Image

from video import FPS, H, W, card, scene

SR = 24000
FADE = 8  # cuadros de transición entre escenas
PAUSE = 0.7  # segundos de silencio al final de cada escena


def build(parts, voice, tts, out):
    """parts = [(narración, fábrica(segundos) -> generador de cuadros)]."""
    with tempfile.TemporaryDirectory() as tmp:
        silent = Path(tmp) / 'video.mp4'
        audio_track = []
        ff = subprocess.Popen(
            ['ffmpeg', '-y', '-loglevel', 'error', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{W}x{H}', '-r', str(FPS),
             '-i', '-', '-c:v', 'libx264', '-preset', 'medium', '-crf', '22', '-pix_fmt', 'yuv420p', str(silent)],
            stdin=subprocess.PIPE,
        )
        previous = None
        total_frames = 0
        for text, factory in parts:
            audio, sr = tts.create(text, voice=voice, speed=1.05, lang='es-419')  # seseo y yeísmo: más cercano al habla chilena que 'es'
            assert sr == SR
            seconds = len(audio) / SR + PAUSE + 0.4
            frames = 0
            for i, frame in enumerate(factory(seconds)):
                if previous is not None and i < FADE:
                    frame = Image.blend(previous, frame, (i + 1) / (FADE + 1))
                ff.stdin.write(frame.tobytes())
                last = frame
                frames += 1
            previous = last
            # La voz empieza 0,4 s después del corte; el bloque dura exactamente lo que duran sus cuadros.
            block = np.zeros(round(frames / FPS * SR), dtype=np.float32)
            start = int(0.4 * SR)
            block[start:start + len(audio)] = audio[: len(block) - start]
            audio_track.append(block)
            total_frames += frames
        ff.stdin.close()
        if ff.wait() != 0:
            raise SystemExit('ffmpeg falló (video)')
        voice_wav = Path(tmp) / 'voz.wav'
        track = np.concatenate(audio_track)
        track = track / max(1e-6, float(np.abs(track).max())) * 0.89
        sf.write(voice_wav, track, SR)
        subprocess.run(
            ['ffmpeg', '-y', '-loglevel', 'error', '-i', str(silent), '-i', str(voice_wav), '-c:v', 'copy',
             '-c:a', 'aac', '-b:a', '128k', '-shortest', '-movflags', '+faststart', str(out)],
            check=True,
        )
    return total_frames / FPS


def guiones(src, img):
    web = json.loads((src / 'boxes.json').read_text())
    app = json.loads((src / 'app_boxes.json').read_text())
    b = lambda k: (web[k]['x'], web[k]['y'], web[k]['width'], web[k]['height'])  # noqa: E731
    a = lambda shot, k: tuple(app[shot][k])  # noqa: E731
    i = lambda name: img / name  # noqa: E731

    def sc(name, target, caption, step, total, **kw):
        return lambda s: scene(i(name), target, caption, step, total, seconds=max(s, 4.2), **kw)

    def cd(title, lines, mono=None):
        return lambda s: card(title, lines, max(s, 3.0), mono=mono)

    ruta_ext = r'C:\Users\<tu usuario>\AppData\Local\Programs\SITFA_Lite\extension'
    extension = [
        ('Abre el menú Más y elige Instalar extensión. El programa copia la ubicación de la carpeta y abre la página de extensiones de Chrome.',
         sc('a0_inicio.png', a('a0_inicio.png', 'mas'), 'Menú «Más» → «Instalar extensión…»: copia la ruta y abre chrome://extensions.', 1, 7)),
        ('Arriba a la derecha, activa el Modo Desarrollador.',
         sc('r01_extensiones.png', b('dev'), 'Activa «Modo Desarrollador» (arriba a la derecha).', 2, 7)),
        ('Aparecen nuevos botones. Pulsa Cargar descomprimida.',
         sc('r02_modo_desarrollador.png', b('load'), 'Pulsa «Cargar descomprimida».', 3, 7)),
        ('Se abre una ventana para elegir carpeta. Pega la ruta con control ve en la barra de dirección, y pulsa Seleccionar carpeta.',
         cd('Paso 4 · Elige la carpeta extension', ['En la ventana de carpetas pega la ruta con Ctrl+V',
                                                    'en la barra de dirección y pulsa «Seleccionar carpeta»:'], ruta_ext)),
        ('Listo. Aparece la tarjeta de Sitfa Lite, activada. No muevas esa carpeta, porque Chrome la usa desde ahí.',
         sc('r03_extension_cargada.png', b('card'), 'Aparece la tarjeta «SITFA Lite · Descargas RUS», activada.', 5, 7, zoom=1.5)),
        ('Para tenerla siempre a mano, abre el menú de extensiones, el ícono de rompecabezas, y pulsa el alfiler junto a Sitfa Lite.',
         sc('r04_menu_extensiones.png', (1086, 212, 24, 24), 'Rompecabezas → alfiler junto a SITFA Lite.', 6, 7)),
        ('Ahora el ícono azul queda fijo en la barra.',
         sc('r05_fijada.png', (1116, 49, 28, 28), 'El ícono azul queda fijo en la barra.', 7, 7)),
    ]
    conectar = [
        ('Pulsa Abrir Sitfa, e inicia sesión como siempre.',
         sc('a0_inicio.png', a('a0_inicio.png', 'abrir'), 'Pulsa «Abrir SITFA» e inicia sesión.', 1, 4)),
        ('Luego, en Sitfa Lite, pulsa Copiar, junto al código.',
         sc('a0_inicio.png', a('a0_inicio.png', 'copiar'), 'Pulsa «Copiar».', 2, 4)),
        ('Vuelve a la pestaña de Sitfa y pulsa el ícono azul. Se abre la pestaña de conexión: pega el código y pulsa Conectar. Deja esa pestaña abierta.',
         sc('r07_codigo_pegado.png', b('connect'), 'Pulsa el ícono, pega el código y pulsa «Conectar».', 3, 4)),
        ('Arriba a la derecha, el programa muestra Chrome conectado, y trae solo las opciones de tu sesión. Ya puedes descargar.',
         sc('a1_conectado.png', a('a1_conectado.png', 'estado'), 'SITFA Lite muestra «Chrome conectado».', 4, 4)),
    ]
    corto = (
        [('Hola. En un par de minutos vas a instalar la extensión de Chrome de Sitfa Lite. Se hace una sola vez y no necesitas permisos de administrador.',
          cd('Cómo instalar la extensión de Chrome', ['Una sola vez · 2 minutos · sin administrador',
                                                      'Antes: instala SITFA Lite con Instalar_SITFA_Lite_1.1.0.exe']))]
        + extension
        + conectar
        + [('Eso es todo. Mientras descargas, deja abiertas la pestaña de Sitfa y la de conexión. Para seguir, revisa el manual de uso en el menú Inicio.',
            cd('Extensión instalada', ['Mantén abiertas la pestaña de SITFA y la de conexión mientras descargas.',
                                       'Manual de uso: menú Inicio → SITFA Lite → Manual de uso.']))]
    )
    uso = [
        ('Para descargar, elige una de las cuatro consultas en la lista de la izquierda. Sitfa abre esa pantalla en tu Chrome, y el programa trae solo sus tribunales y filtros.',
         sc('a1_conectado.png', a('a1_conectado.png', 'consultas'), 'Elige la consulta en la lista de la izquierda.', 1, 8, zoom=1.4)),
        ('Marca los tribunales con un clic en cada ficha. Un clic en el nombre de la fila marca o desmarca todos.',
         sc('a2_opciones_seguimiento.png', a('a2_opciones_seguimiento.png', 'tribunales'), 'Tribunales: clic en cada ficha.', 2, 8, zoom=1.3)),
        ('Elige las pestañas: Espera, Cumplimiento, Informes o Egresados. Y la modalidad: Residencia, Ambulatorio, FAE o DCE.',
         sc('a2_opciones_seguimiento.png', a('a2_opciones_seguimiento.png', 'pestanas'), 'Pestañas y modalidad.', 3, 8)),
        ('Si marcas la pestaña Informes, aparece el tipo de informe. Y si marcas Cumplimiento, aparece el filtro de medidas.',
         sc('a3_informes_medidas_fechas.png', a('a3_informes_medidas_fechas.png', 'informes'), 'Tipo de informe y filtro de medidas.', 4, 8)),
        ('El período es opcional: sin filtro, o por fecha, con desde y hasta, y el botón calendario.',
         sc('a3_informes_medidas_fechas.png', a('a3_informes_medidas_fechas.png', 'periodo'), 'Período: sin filtro o por fecha.', 5, 8, zoom=1.4)),
        ('En Incluir decides si se guarda un pantallazo en pe de efe cuando una consulta no tiene registros. Si lo desmarcas, la consulta solo queda anotada como sin registros.',
         sc('a3_informes_medidas_fechas.png', a('a3_informes_medidas_fechas.png', 'incluir'), 'PDF si no hay registros: opcional.', 6, 8, zoom=1.4)),
        ('Pulsa Descargar. Cada consulta aparece en la lista con su estado y sus registros. Mientras trabaja, no uses la pestaña de Sitfa.',
         sc('a4_descarga_terminada.png', a('a4_descarga_terminada.png', 'lista'), 'Cada consulta con su estado.', 7, 8, zoom=1.2)),
        ('Al terminar, abajo ves el resumen del lote, con Abrir carpeta y Nuevo lote. Con doble clic en una consulta abres su archivo.',
         sc('a4_descarga_terminada.png', a('a4_descarga_terminada.png', 'resumen'), 'Abrir carpeta o Nuevo lote.', 8, 8)),
        ('En Búsqueda de litigantes, marca tribunales y uno o varios estados. El período también es opcional.',
         sc('a5_busqueda_lit.png', a('a5_busqueda_lit.png', 'estados'), 'Búsqueda litigantes: tribunales y estado.', 0, 0, zoom=1.5)),
        ('Y en Informes por vencer o Medidas por vencer, marca tribunales, uno o varios meses, y el año.',
         sc('a6_medidas_por_vencer.png', a('a6_medidas_por_vencer.png', 'meses'), 'Informes / Medidas por vencer: mes(es) y año.', 0, 0, zoom=1.5)),
        ('Los archivos se guardan en una carpeta con fecha y hora, y su nombre dice exactamente qué consulta es: pestaña, modalidad, filtro, tribunal, fechas y página.',
         cd('Nombres de los archivos', [
             'Subcarpeta por lote con fecha y hora, en Documentos\\SITFA Lite.',
             'Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia X - 01-10-2026 al 30-10-2026 - p2 de 3.xls',
             'Medidas por vencer - Octubre 2026 - Jdo Familia X.xls',
             'Sin registros, con PDF marcado → … - SIN REGISTROS.pdf'])),
        ('Unos consejos: deja Chrome visible mientras descarga. Si aparece sesión expirada o sin conexión, vuelve a entrar a Sitfa, pulsa el ícono y pega el código actual. Los manuales están en el menú Inicio. ¡Listo, a descargar!',
         cd('Consejos', [
             '• Deja Chrome visible (no minimizado): el pantallazo PDF lo necesita.',
             '• Detener termina la página en curso; lo guardado se conserva.',
             '• «Sesión expirada» o «Sin conexión»: entra a SITFA, pulsa el ícono y pega el código actual.',
             '• Manuales PDF: menú Inicio → SITFA Lite.'])),
    ]
    completo = (
        [('Hola. Este video muestra cómo instalar y usar Sitfa Lite, la herramienta que descarga las planillas Excel que produce Rus, con todas sus páginas: Informe de seguimiento, Búsqueda de litigantes, Informes por vencer y Medidas por vencer.',
          cd('SITFA Lite · Instalación y uso', [
              'Descarga los XLS que produce RUS, con todas sus páginas:',
              '• Seguimiento · Informe seguimiento   • Seguimiento · Búsqueda litigantes',
              '• Informes por vencer   • Medidas por vencer',
              'Pantallazo PDF opcional cuando una consulta no tiene registros.'])),
         ('Primero, instala el programa. Haz doble clic en el instalador. Si Windows muestra una advertencia, pulsa Más información, y luego Ejecutar de todas formas. Deja la carpeta propuesta y pulsa Instalar. No pide permisos de administrador.',
          cd('Parte 1 · Instalar el programa', [
              '1. Doble clic en Instalar_SITFA_Lite_1.1.0.exe.',
              '2. «Windows protegió su PC»: Más información → Ejecutar de todas formas.',
              '3. Siguiente → deja la carpeta propuesta → Instalar (no pide administrador).',
              '4. Al terminar deja marcada «Abrir SITFA Lite».'],
             r'C:\Users\<tu usuario>\AppData\Local\Programs\SITFA_Lite')),
         ('Segundo: carga la extensión de Chrome. Se hace una sola vez. La extensión consulta Sitfa con tu propia sesión, sin ver tu contraseña.',
          cd('Parte 2 · Cargar la extensión de Chrome', ['Una sola vez. La extensión consulta SITFA con tu propia sesión;',
                                                         'el programa no ve tu contraseña ni tus cookies.']))]
        + extension
        + [('Tercero: conecta Chrome con el programa. Esto se hace cada vez que abres Sitfa Lite.',
            cd('Parte 3 · Conectar', ['Cada vez que abras SITFA Lite.']))]
        + conectar
        + [('Y cuarto: descargar.', cd('Parte 4 · Descargar', ['Elegir consulta → marcar filtros → Descargar.']))]
        + uso
    )
    return corto, completo


def main(src, img, modelo, out, voice='ef_dora'):
    src, img, modelo, out = Path(src), Path(img), Path(modelo), Path(out)
    tts = Kokoro(str(modelo / 'kokoro-v1.0.onnx'), str(modelo / 'voices-v1.0.bin'))
    corto, completo = guiones(src, img)
    for name, parts in (('SITFA_Lite_Video_Instalar_Extension.mp4', corto),
                        ('SITFA_Lite_Video_Instalacion_y_Uso.mp4', completo)):
        print(name, round(build(parts, voice, tts, out / name), 1), 's')


if __name__ == '__main__':
    main(*sys.argv[1:6])
