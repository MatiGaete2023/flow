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
        ('Pulsa el botón Instalar extensión. El programa copia la ubicación de la carpeta y abre la página de extensiones de Chrome.',
         sc('a0_inicio.png', a('a0_inicio.png', 'instalar'), 'Pulsa «Instalar extensión…»: copia la ruta y abre chrome://extensions.', 1, 7)),
        ('Arriba a la derecha, activa el Modo Desarrollador.',
         sc('r01_extensiones.png', b('dev'), 'Activa «Modo Desarrollador» (arriba a la derecha).', 2, 7)),
        ('Aparecen nuevos botones. Pulsa Cargar descomprimida.',
         sc('r02_modo_desarrollador.png', b('load'), 'Pulsa «Cargar descomprimida».', 3, 7)),
        ('Se abre una ventana para elegir carpeta. Pega la ruta con control ve en la barra de dirección, y pulsa Seleccionar carpeta.',
         cd('Paso 4 · Elige la carpeta extension', ['En la ventana de carpetas pega la ruta con Ctrl+V',
                                                    'en la barra de dirección y pulsa «Seleccionar carpeta»:'], ruta_ext)),
        ('Listo. Aparece la tarjeta de Sitfa Lite, versión uno punto cero, activada. No muevas esa carpeta, porque Chrome la usa desde ahí.',
         sc('r03_extension_cargada.png', b('card'), 'Aparece la tarjeta «SITFA Lite · Descargas RUS 1.0.0», activada.', 5, 7, zoom=1.5)),
        ('Para tenerla siempre a mano, abre el menú de extensiones, el ícono de rompecabezas, y pulsa el alfiler junto a Sitfa Lite.',
         sc('r04_menu_extensiones.png', (1086, 212, 24, 24), 'Rompecabezas → alfiler junto a SITFA Lite.', 6, 7)),
        ('Ahora el ícono azul queda fijo en la barra.',
         sc('r05_fijada.png', (1116, 49, 28, 28), 'El ícono azul queda fijo en la barra.', 7, 7)),
    ]
    conectar = [
        ('Pulsa Abrir Sitfa en Chrome, e inicia sesión como siempre.',
         sc('a0_inicio.png', a('a0_inicio.png', 'abrir'), 'Pulsa «Abrir SITFA en Chrome» e inicia sesión.', 1, 4)),
        ('Luego, en Sitfa Lite, pulsa Copiar código.',
         sc('a0_inicio.png', a('a0_inicio.png', 'copiar'), 'Pulsa «Copiar código».', 2, 4)),
        ('Vuelve a la pestaña de Sitfa y pulsa el ícono azul. Se abre la pestaña de conexión: pega el código y pulsa Conectar. Deja esa pestaña abierta.',
         sc('r07_codigo_pegado.png', b('connect'), 'Pulsa el ícono, pega el código y pulsa «Conectar».', 3, 4)),
        ('El programa muestra Conectado, en verde. Ya puedes descargar.',
         sc('a1_conectado.png', a('a1_conectado.png', 'estado'), 'SITFA Lite muestra «● Conectado».', 4, 4)),
    ]
    corto = (
        [('Hola. En un par de minutos vas a instalar la extensión de Chrome de Sitfa Lite. Se hace una sola vez y no necesitas permisos de administrador.',
          cd('Cómo instalar la extensión de Chrome', ['Una sola vez · 2 minutos · sin administrador',
                                                      'Antes: instala SITFA Lite con Instalar_SITFA_Lite_1.0.0.exe']))]
        + extension
        + conectar
        + [('Eso es todo. Mientras descargas, deja abiertas la pestaña de Sitfa y la de conexión. Para seguir, revisa el manual de uso en el menú Inicio.',
            cd('Extensión instalada', ['Mantén abiertas la pestaña de SITFA y la de conexión mientras descargas.',
                                       'Manual de uso: menú Inicio → SITFA Lite → Manual de uso.']))]
    )
    uso = [
        ('Para descargar, primero marca una de las cuatro consultas.',
         sc('a1_conectado.png', a('a1_conectado.png', 'radios'), 'Marca una de las 4 consultas.', 1, 8, zoom=1.4)),
        ('Pulsa Cargar opciones. Sitfa abre esa pantalla en tu Chrome, y el programa trae los tribunales y filtros de tu sesión.',
         sc('a1_conectado.png', a('a1_conectado.png', 'cargar'), 'Pulsa «Cargar opciones».', 2, 8)),
        ('En Informe de seguimiento, marca los tribunales con un clic. El buscador filtra la lista, y los botones Todos y Ninguno marcan lo que ves.',
         sc('a2_opciones_seguimiento.png', a('a2_opciones_seguimiento.png', 'tribunales'), 'Marca tribunales; «Buscar» filtra la lista.', 3, 8, zoom=1.3)),
        ('Elige las pestañas: Espera, Cumplimiento, Informes o Egresados. Y debajo, las modalidades: Residencia, Ambulatorio, FAE o DCE.',
         sc('a2_opciones_seguimiento.png', a('a2_opciones_seguimiento.png', 'pestanas'), 'Pestañas y modalidades.', 4, 8)),
        ('Si incluyes la pestaña Informes, elige el tipo de informe. Y si incluyes Cumplimiento, elige el filtro de medidas: todas, por vencer o vencidas.',
         sc('a3_informes_medidas_fechas.png', a('a3_informes_medidas_fechas.png', 'informes'), 'Tipo de informe y filtro de medidas.', 5, 8)),
        ('El filtro de fecha es opcional. Escribe desde y hasta, o usa los atajos: este mes, mes anterior o últimos treinta días.',
         sc('a3_informes_medidas_fechas.png', a('a3_informes_medidas_fechas.png', 'fechas'), 'Filtro de fecha opcional.', 6, 8, zoom=1.5)),
        ('Pulsa Descargar. Mientras trabaja, no uses la pestaña de Sitfa: verás el aviso Descargando el lote.',
         sc('a4_descarga_terminada.png', a('a4_descarga_terminada.png', 'descargar'), 'Pulsa «Descargar».', 7, 8)),
        ('Cada archivo aparece en la lista con sus registros. Con doble clic lo abres, y con Abrir carpeta del lote ves todo lo descargado.',
         sc('a4_descarga_terminada.png', a('a4_descarga_terminada.png', 'lista'), 'Doble clic abre el archivo.', 8, 8, zoom=1.4)),
        ('En Búsqueda de litigantes, marca tribunales y uno o varios estados. Las fechas también son opcionales.',
         sc('a5_busqueda_lit.png', a('a5_busqueda_lit.png', 'estados'), 'Búsqueda litigantes: tribunales y estado.', 0, 0, zoom=1.5)),
        ('Y en Informes por vencer o Medidas por vencer, marca tribunales, uno o varios meses, y el año.',
         sc('a6_medidas_por_vencer.png', a('a6_medidas_por_vencer.png', 'meses'), 'Informes / Medidas por vencer: mes(es) y año.', 0, 0, zoom=1.5)),
        ('Los archivos se guardan en una carpeta con fecha y hora, y su nombre dice exactamente qué consulta es: pestaña, modalidad, filtro, tribunal, fechas y página. Si una consulta no tiene registros, se guarda un pantallazo en pe de efe que dice sin registros.',
         cd('Nombres de los archivos', [
             'Subcarpeta por lote con fecha y hora, en Documentos\\SITFA Lite.',
             'Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia X - 01-10-2026 al 30-10-2026 - p2 de 3.xls',
             'Medidas por vencer - Octubre 2026 - Jdo Familia X.xls',
             'Sin registros → … - SIN REGISTROS.pdf (pantallazo de SITFA)'])),
        ('Unos consejos: deja Chrome visible mientras descarga. Si aparece sesión expirada o sin conexión, vuelve a entrar a Sitfa, pulsa el ícono y pega el código actual. Los manuales están en el menú Inicio. ¡Listo, a descargar!',
         cd('Consejos', [
             '• Deja Chrome visible (no minimizado): el pantallazo PDF lo necesita.',
             '• Cancelar detiene al terminar la página en curso; lo guardado se conserva.',
             '• «Sesión expirada» o «Sin conexión»: entra a SITFA, pulsa el ícono y pega el código actual.',
             '• Manuales PDF: menú Inicio → SITFA Lite.'])),
    ]
    completo = (
        [('Hola. Este video muestra cómo instalar y usar Sitfa Lite, la herramienta que descarga las planillas Excel que produce Rus, con todas sus páginas: Informe de seguimiento, Búsqueda de litigantes, Informes por vencer y Medidas por vencer.',
          cd('SITFA Lite · Instalación y uso', [
              'Descarga los XLS que produce RUS, con todas sus páginas:',
              '• Seguimiento · Informe seguimiento   • Seguimiento · Búsqueda litigantes',
              '• Informes por vencer   • Medidas por vencer',
              'Pantallazo PDF cuando una consulta no tiene registros.'])),
         ('Primero, instala el programa. Haz doble clic en el instalador. Si Windows muestra una advertencia, pulsa Más información, y luego Ejecutar de todas formas. Deja la carpeta propuesta y pulsa Instalar. No pide permisos de administrador.',
          cd('Parte 1 · Instalar el programa', [
              '1. Doble clic en Instalar_SITFA_Lite_1.0.0.exe.',
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
        + [('Y cuarto: descargar.', cd('Parte 4 · Descargar', ['Elegir consulta → Cargar opciones → marcar filtros → Descargar.']))]
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
