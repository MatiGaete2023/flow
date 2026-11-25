"""E-R1b · Oráculo de Meta 4 sobre lotes del Descargador (`META4_Bitacoras/scripts/meta4_bitacoras.py`).

Parte del escenario «contexto» de `app/tests/datos/paquete.json` (lote de listados y carpeta de bitácoras que produce el
Descargador ficticio de Flow Python) y le aplica lo que el SITFA ficticio no emite pero Meta 4 exige: pasa la consulta a
Residencia, nombra el programa como residencia y agrega la columna TRIBUNAL al listado (con su huella). Cada caso altera
ese lote de una manera (identidad visible, observación en el mes, archivo alterado, lectura fallida, calendario fallido,
ambigüedad, lote duplicado, filtros…) y registra qué entrega `meta4_bitacoras.generar` con `_escribir` capturado: las filas
ya enriquecidas (`enriquecer`), las lecturas de bitácora, los filtros y avisos de lectura, o el mensaje del error.

La prueba Rust (`descargador::meta4::tests`) reconstruye los mismos archivos, calcula con `meta4::calcular` y compara fila
a fila. Archivos en base64 con separadores `/`; sin rutas ni horas de la máquina.

Uso (desde la raíz): python app/qa/generar_meta4.py [salida.json]   (por omisión app/tests/datos/meta4.json)
"""

import base64
import hashlib
import io
import json
import shutil
import sys
import tempfile
from datetime import date, datetime
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'META4_Bitacoras' / 'scripts')]  # su `nurus` propio (META4_Bitacoras/src) es el que usa Meta 4

import meta4_bitacoras as mb  # noqa: E402
from openpyxl import load_workbook  # noqa: E402

PROGRAMA = 'Residencia Centro Ficticio'
TRIBUNAL = 'Juzgado de Familia Tomé'
CORTE = date(2026, 8, 31)  # ventana 31-07 a 31-08: contiene la observación del 12-08 de las bitácoras ficticias


def reconstruir(destino, archivos):
    for ruta, b64 in archivos.items():
        p = destino / ruta
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(base64.b64decode(b64))


def volcar_carpeta(carpeta, solo=None):
    return {p.relative_to(carpeta).as_posix(): base64.b64encode(p.read_bytes()).decode()
            for p in sorted(carpeta.rglob('*')) if p.is_file() and (solo is None or solo(p))}


def lee_meta4(p):
    """Lo que Meta 4 abre de una carpeta de bitácoras (no su historia, histórico ni calendarios)."""
    return p.name == 'bitacoras.json' or (p.name.startswith('bitacora-') and p.suffix == '.html')


def leer(ruta):
    return json.loads(ruta.read_text(encoding='utf-8'))


def poner(ruta, texto):
    """Bytes UTF-8 con `\\n`: `write_text` los cambiaría a `\\r\\n` en Windows y con ello las huellas."""
    ruta.write_bytes(texto.encode('utf-8'))


def escribir(ruta, datos):
    poner(ruta, json.dumps(datos, ensure_ascii=False, indent=2))


def sha(ruta):
    return hashlib.sha256(ruta.read_bytes()).hexdigest()


def preparar(base, escenario):
    """Lote y bitácoras del Descargador ficticio, pasados a Residencia y con la columna TRIBUNAL."""
    lote, bitacoras = base / 'lote', base / 'bitacoras'
    reconstruir(lote, escenario['lote']['archivos'])
    reconstruir(bitacoras, escenario['bitacoras']['archivos'])
    for ruta in (lote / 'resumen.json', lote / 'verificacion 001.json', bitacoras / 'bitacoras.json'):
        t = ruta.read_text(encoding='utf-8')
        for a, b in (('"modality": "2"', '"modality": "1"'), ('"modalidad": "2"', '"modalidad": "1"'),
                     ('"modalities": ["2"]', '"modalities": ["1"]'), (': "Ambulatorio"', ': "Residencia"'),
                     ('/ Ambulatorio"', '/ Residencia"'), ('"modalities": [\n      "2"\n    ]', '"modalities": [\n      "1"\n    ]'),
                     ('PRM Centro Ficticio', PROGRAMA)):
            t = t.replace(a, b)
        poner(ruta, t)
    verificacion = leer(lote / 'verificacion 001.json')
    for item in verificacion['archivos']:
        pagina = lote / item['archivo']
        t = pagina.read_text(encoding='utf-8')
        t = t.replace('PRM Centro Ficticio', PROGRAMA).replace('<th>RIT</th>', '<th>RIT</th><th>Tribunal</th>')
        filas = []
        for fila in t.split('<tr>'):
            if '<td>' in fila and '<th>' not in fila:
                fila = fila.replace('</td>', f'</td><td>{TRIBUNAL}</td>', 1)
            filas.append(fila)
        t = '<tr>'.join(filas)
        poner(pagina, t)
        item['sha256'], item['bytes'] = sha(pagina), pagina.stat().st_size
    escribir(lote / 'verificacion 001.json', verificacion)
    return lote, bitacoras


# -- variantes: cada una recibe (lote, bitacoras) y las altera -----------------------------------------------------------

def html_de(bitacoras, n):
    return sorted(bitacoras.glob('bitacora-*.html'))[n]


def reescribir_html(bitacoras, n, cambios, huella=True):
    ruta = html_de(bitacoras, n)
    t = ruta.read_text(encoding='utf-8')
    for a, b in cambios:
        assert a in t, a
        t = t.replace(a, b)
    poner(ruta, t)
    if huella:
        manifiesto = leer(bitacoras / 'bitacoras.json')
        for r in manifiesto['registros']:
            if r['archivo'] == ruta.name:
                r['sha256'] = sha(ruta)
        escribir(bitacoras / 'bitacoras.json', manifiesto)


def editar_bitacoras(bitacoras, fn):
    manifiesto = leer(bitacoras / 'bitacoras.json')
    fn(manifiesto)
    escribir(bitacoras / 'bitacoras.json', manifiesto)


def coincide_identidad(lote, bitacoras):
    """El HTML nombra el mismo RIT de cada fila del listado y el aviso de identidad del manifiesto desaparece."""
    reescribir_html(bitacoras, 0, [('X-1-2026', 'X-1-2099')])
    reescribir_html(bitacoras, 1, [('X-1-2026', 'X-2-2099')])

    def quitar(m):
        for r in m['registros']:
            r['advertencias'], r['error'] = [], ''
    editar_bitacoras(bitacoras, quitar)


def nombre_visible(lote, bitacoras):
    coincide_identidad(lote, bitacoras)
    reescribir_html(bitacoras, 0, [('value="Persona ficticia"', 'value="Otra Cualquiera Distinta"')])


def espera_con_alerta(lote, bitacoras):
    coincide_identidad(lote, bitacoras)
    reescribir_html(bitacoras, 0, [('<td>Cumplimiento</td><td>12/08/2026', '<td>Espera</td><td>12/08/2026')])


def archivo_alterado(lote, bitacoras):
    coincide_identidad(lote, bitacoras)
    reescribir_html(bitacoras, 0, [('Texto recortado', 'Texto alterado')], huella=False)


def lectura_fallida(lote, bitacoras):
    coincide_identidad(lote, bitacoras)

    def fallar(m):
        m['registros'][0]['estado'] = 'FALLIDA'
        m['registros'][0]['error'] = 'La bitácora no cargó.'
    editar_bitacoras(bitacoras, fallar)


def calendario_fallido(lote, bitacoras):
    coincide_identidad(lote, bitacoras)

    def fallar(m):
        m['registros'][0]['calendario_lectura'] = {'estado': 'FALLIDA', 'error': 'El calendario no cargó.'}
    editar_bitacoras(bitacoras, fallar)


def sin_bitacora(lote, bitacoras):
    coincide_identidad(lote, bitacoras)
    editar_bitacoras(bitacoras, lambda m: m.update(registros=m['registros'][:1]))


def ambigua(lote, bitacoras):
    """Sin vínculos de ingreso en el listado, dos bitácoras de la misma persona y programa no se atribuyen."""
    coincide_identidad(lote, bitacoras)
    verificacion = leer(lote / 'verificacion 001.json')
    for item in verificacion['archivos']:
        item['vinculos_ingreso'] = []
    escribir(lote / 'verificacion 001.json', verificacion)

    def duplicar(m):
        otra = dict(m['registros'][0], ingreso_id='299', causa_id='199')
        m['registros'].append(otra)
    editar_bitacoras(bitacoras, duplicar)


def sin_alcance(lote, bitacoras):
    coincide_identidad(lote, bitacoras)
    editar_bitacoras(bitacoras, lambda m: m.update(consultas=[]))


def bitacoras_con_filtro(lote, bitacoras):
    coincide_identidad(lote, bitacoras)
    editar_bitacoras(bitacoras, lambda m: m['seleccion'].update(dates={'desde': '01/08/2026', 'hasta': '31/08/2026'}))


def lote_con_filtros(lote, bitacoras):
    coincide_identidad(lote, bitacoras)
    resumen = leer(lote / 'resumen.json')
    resumen['filtro_fecha'] = {'desde': '01/08/2026', 'hasta': '31/08/2026'}
    resumen['otros_filtros'] = 'estado: vigentes'
    escribir(lote / 'resumen.json', resumen)


def lote_incompleto(lote, bitacoras):
    resumen = leer(lote / 'resumen.json')
    resumen['estado'] = 'INCOMPLETA'
    escribir(lote / 'resumen.json', resumen)


def faltan_paginas(lote, bitacoras):
    verificacion = leer(lote / 'verificacion 001.json')
    verificacion['paginas_esperadas'] = 3
    escribir(lote / 'verificacion 001.json', verificacion)


def consulta_no_validada(lote, bitacoras):
    verificacion = leer(lote / 'verificacion 001.json')
    verificacion['estado'] = 'INCOMPLETA'
    escribir(lote / 'verificacion 001.json', verificacion)


def pagina_alterada(lote, bitacoras):
    pagina = next(lote.glob('CUMP*1.xls'))
    pagina.write_bytes(pagina.read_bytes().replace(b'Ficticia 1', b'Ficticia X'))


# -- lotes sintéticos compuestos: Espera, FAE y residencia, otros tribunales y dos lotes --------------------------------

FECHA_HORA = '2026-10-08T13:00:00.123456-03:00'
TOME, LAJA, MULCHEN = 'Juzgado de Familia Tomé', 'Juzgado de Familia Laja', 'Juzgado de Familia Mulchén'
CORTOS = {TOME: 'Jdo Familia Tome', LAJA: 'Jdo Familia Laja', MULCHEN: 'Jdo Familia Mulchen'}


def seleccion(tribunal, tribunal_nombre, tab, modality, modalidad_nombre):
    return {'tribunal': tribunal, 'tab': tab, 'modality': modality, 'report': None, 'screen': 'seguimiento',
            'state': '1', 'month': None, 'year': None, 'start': None, 'end': None,
            'tribunal_nombre': tribunal_nombre, 'modalidad_nombre': modalidad_nombre, 'fecha_descarga': FECHA_HORA}


def texto_consulta(tribunal_nombre, tab, modalidad_nombre):
    return f'{tribunal_nombre} / {tab} / {modalidad_nombre}'


def pagina_html(filas, cabecera):
    """Página tabular mínima del Descargador: una fila de encabezados y una por registro."""
    th = ''.join(f'<th>{h}</th>' for h in cabecera)
    tr = ''.join('<tr>' + ''.join(f'<td>{c}</td>' for c in fila) + '</tr>' for fila in filas)
    return f'<html><meta charset="utf-8"><table id="x"><tr>{th}</tr>{tr}</table></html>'


def agregar_consulta(lote, tribunal_codigo, tribunal_nombre, tab, modality, modalidad_nombre, prefijo, filas):
    """Agrega una consulta validada al lote: manifiesto propio, página propia y su entrada en `resumen.json`.
    `filas` son (RIT, RUT, NOMBRE, DERIVACIÓN); la columna TRIBUNAL y T ESPERA se agregan como en el Descargador."""
    resumen = leer(lote / 'resumen.json')
    numero = len(resumen['consultas']) + 1
    manifiesto = f'verificacion {numero:03d}.json'
    archivo = f'{prefijo} {modalidad_nombre} {CORTOS[tribunal_nombre]} {numero}.xls'
    cabecera = ['RIT', 'Tribunal', 'RUT', 'Nombre', 'Derivación'] + (['T ESPERA'] if tab == 'Espera' else []) + ['Acciones']
    cuerpo = [[rit, tribunal_nombre, rut, nombre, derivacion] + (['35'] if tab == 'Espera' else []) + ['']
              for rit, rut, nombre, derivacion in filas]
    poner(lote / archivo, pagina_html(cuerpo, cabecera))
    datos = seleccion(tribunal_codigo, tribunal_nombre, tab, modality, modalidad_nombre)
    escribir(lote / manifiesto, {
        'fecha_hora': FECHA_HORA, 'caso': f'{tribunal_codigo}-{tab}-M{modality}',
        'consulta': texto_consulta(tribunal_nombre, tab, modalidad_nombre), 'estado': 'VALIDADA',
        'paginas_esperadas': 1, 'paginas_verificadas': 1,
        'archivos': [{'pagina': 1, 'total': len(filas), 'archivo': archivo, 'bytes': (lote / archivo).stat().st_size,
                      'sha256': sha(lote / archivo), 'formato': 'HTML-tabular-con-extension-xls',
                      'registros': len(filas), 'coincide_con_html': True, 'estado': 'OK', 'vinculos_ingreso': []}],
        'seleccion': datos,
    })
    resumen['plan'].append(datos)
    resumen['consultas'].append({'consulta': texto_consulta(tribunal_nombre, tab, modalidad_nombre), 'estado': 'VALIDADA',
                                 'paginas': 1, 'registros': len(filas), 'verificacion': manifiesto, 'seleccion': datos})
    resumen['consultas_esperadas'] = len(resumen['consultas'])
    resumen['consultas_completadas'] = len(resumen['consultas'])
    escribir(lote / 'resumen.json', resumen)


def crear_lote(destino, consultas):
    """Lote del Descargador desde cero: `consultas` son los argumentos de `agregar_consulta`."""
    destino.mkdir(parents=True, exist_ok=True)
    escribir(destino / 'resumen.json', {
        'version_contrato': 1, 'fecha_hora': FECHA_HORA, 'estado': 'VALIDADA', 'plan': [],
        'consultas_esperadas': 0, 'consultas_completadas': 0, 'consultas': [],
        'filtro_fecha': None, 'otros_filtros': 'todos los registros',
    })
    for consulta in consultas:
        agregar_consulta(destino, *consulta)


def _pasar_a_espera(ruta):
    """Cambia la pestaña de una consulta de Cumplimiento a Espera en un JSON del lote o de bitácoras."""
    t = ruta.read_text(encoding='utf-8')
    for a, b in (('"tab": "Cumplimiento"', '"tab": "Espera"'),
                 ('"tabs": [\n      "Cumplimiento"\n    ]', '"tabs": [\n      "Espera"\n    ]'),
                 ('/ Cumplimiento /', '/ Espera /')):
        t = t.replace(a, b)
    poner(ruta, t)


def a_espera(lote, bitacoras):
    """El lote pasa a la lista de Espera: páginas ESP con T ESPERA y bitácoras que enumeran esa consulta."""
    coincide_identidad(lote, bitacoras)
    manifiesto = leer(lote / 'verificacion 001.json')
    for item in manifiesto['archivos']:
        ruta = lote / item['archivo']
        t = ruta.read_text(encoding='utf-8')
        assert '<th>Acciones</th>' in t and '<td></td></tr>' in t, item['archivo']
        t = t.replace('<th>Acciones</th>', '<th>T ESPERA</th><th>Acciones</th>').replace('<td></td></tr>', '<td>35</td><td></td></tr>')
        nuevo = item['archivo'].replace('CUMP', 'ESP')
        poner(lote / nuevo, t)
        ruta.unlink()
        item.update(archivo=nuevo, sha256=sha(lote / nuevo), bytes=(lote / nuevo).stat().st_size)
    escribir(lote / 'verificacion 001.json', manifiesto)
    _pasar_a_espera(lote / 'resumen.json')
    _pasar_a_espera(bitacoras / 'bitacoras.json')


def fae_en_el_lote(lote, bitacoras):
    """FAE y Residencia en el mismo lote: cada modalidad es una consulta distinta."""
    coincide_identidad(lote, bitacoras)
    agregar_consulta(lote, '49', TOME, 'Cumplimiento', '3', 'FAE', 'CUMP',
                     [('X-3-2099', '11111113-0', 'Persona Ficticia 3', 'FAE PRO Ficticio')])


def laja_en_el_lote(lote, bitacoras):
    """Un segundo tribunal (Laja) en el mismo lote, con Residencia y sin bitácoras enumeradas."""
    coincide_identidad(lote, bitacoras)
    agregar_consulta(lote, '133', LAJA, 'Cumplimiento', '1', 'Residencia', 'CUMP',
                     [('L-7-2099', '22222222-1', 'Persona Ficticia Laja', 'Residencia Centro Ficticio')])


def mulchen_en_el_lote(lote, bitacoras):
    """Mulchén reemplaza a Tomé: el lote queda con un tribunal que las bitácoras no enumeran."""
    coincide_identidad(lote, bitacoras)
    resumen = leer(lote / 'resumen.json')

    def cambiar(parte):
        if parte.get('tribunal') == '49':
            parte['tribunal'] = '141'
        if parte.get('tribunal_nombre') == TOME:
            parte['tribunal_nombre'] = MULCHEN
        if isinstance(parte.get('consulta'), str):
            parte['consulta'] = parte['consulta'].replace(TOME, MULCHEN)

    for parte in resumen['plan'] + [c['seleccion'] for c in resumen['consultas']]:
        cambiar(parte)
    for consulta in resumen['consultas']:
        consulta['consulta'] = consulta['consulta'].replace(TOME, MULCHEN)
    escribir(lote / 'resumen.json', resumen)
    manifiesto = leer(lote / 'verificacion 001.json')
    for item in manifiesto['archivos']:
        ruta = lote / item['archivo']
        poner(ruta, ruta.read_text(encoding='utf-8').replace(f'<td>{TOME}</td>', f'<td>{MULCHEN}</td>'))
        item['sha256'], item['bytes'] = sha(ruta), ruta.stat().st_size
    cambiar(manifiesto['seleccion'])
    escribir(lote / 'verificacion 001.json', manifiesto)


def programa_ajeno(lote, bitacoras):
    """Una fila con una derivación que no es FAE ni Residencia queda fuera de META 4 (y, en Python, también de
    Registros). Va en la página que conserva una fila de Residencia: una página sin ninguna derivación reconocida
    es un error del lector."""
    coincide_identidad(lote, bitacoras)
    pagina = lote / 'CUMP Ambulatorio Jdo Familia Tome 1.xls'
    t = pagina.read_text(encoding='utf-8')
    fila = (f'<tr><td>X-4-2099</td><td>{TRIBUNAL}</td><td>11111114-0</td><td>Persona Ficticia 4</td>'
            '<td>OPD Programa Ficticio</td><td></td></tr>')
    assert '</table>' in t
    poner(pagina, t.replace('</table>', fila + '</table>'))
    manifiesto = leer(lote / 'verificacion 001.json')
    for item in manifiesto['archivos']:
        if item['archivo'] == pagina.name:
            item['sha256'], item['bytes'] = sha(pagina), pagina.stat().st_size
            item['total'] = item['registros'] = 2
    escribir(lote / 'verificacion 001.json', manifiesto)
    resumen = leer(lote / 'resumen.json')
    resumen['consultas'][0]['registros'] = 3
    escribir(lote / 'resumen.json', resumen)


def actualizar_huella(bitacoras, ruta):
    manifiesto = leer(bitacoras / 'bitacoras.json')
    for registro in manifiesto['registros']:
        if registro['archivo'] == ruta.name:
            registro['sha256'] = sha(ruta)
    escribir(bitacoras / 'bitacoras.json', manifiesto)


def agregar_entrada(bitacoras, n, fecha, etapa, texto, ident):
    """Agrega una observación a la bitácora n con sus campos ocultos, como la tabla de RUS."""
    ruta = html_de(bitacoras, n)
    t = ruta.read_text(encoding='utf-8')
    fila = (f'<tr><td>Al Tribunal</td><td>{etapa}</td><td>{fecha}</td><td>Usuario ficticio</td><td>{texto}</td>'
            f'<td>---</td><td>---</td><td>---</td><td><img id="{ident}"></td></tr>'
            f'<input type="hidden" name="HOBS_Centro_{ident}" value="{texto}">'
            f'<input type="hidden" name="HOBS_Tribunal_{ident}" value="---">'
            f'<input type="hidden" name="HCOD_Estado_{ident}" value="6">'
            f'<input type="hidden" name="HFUN_Tribunal_{ident}" value="">'
            f'<input type="hidden" name="HFLG_Envio_{ident}" value="1">')
    assert '</table>' in t
    poner(ruta, t.replace('</table>', fila + '</table>'))
    actualizar_huella(bitacoras, ruta)


def etapas_distintas(lote, bitacoras):
    """Dos etapas con la misma fecha y hora: la última observación es un empate y se pide revisión."""
    coincide_identidad(lote, bitacoras)
    agregar_entrada(bitacoras, 0, '12/08/2026 10:13', 'Espera', 'Texto simultáneo', 45)


def bordes_de_la_ventana(lote, bitacoras):
    """Observaciones justo en el primer y el último día de la ventana (31-07 a 31-08)."""
    coincide_identidad(lote, bitacoras)
    agregar_entrada(bitacoras, 0, '31/07/2026 09:00', 'Cumplimiento', 'Primer día de la ventana', 45)
    agregar_entrada(bitacoras, 0, '31/08/2026 10:00', 'Cumplimiento', 'Último día de la ventana', 46)


def fuera_de_la_ventana(lote, bitacoras):
    """Todas las observaciones de la primera bitácora quedan fuera del período: sin fecha de observación."""
    coincide_identidad(lote, bitacoras)
    reescribir_html(bitacoras, 0, [('12/08/2026 10:13', '30/07/2026 10:13')])
    agregar_entrada(bitacoras, 0, '01/09/2026 10:00', 'Cumplimiento', 'Después del corte', 45)


def dos_lotes(base, escenario):
    """Un lote de Tomé (Cumplimiento, Residencia) y otro de Laja (Espera, FAE) que no se solapan."""
    lote, bitacoras = preparar(base, escenario)
    coincide_identidad(lote, bitacoras)
    otro = base / 'lote_laja'
    crear_lote(otro, [('133', LAJA, 'Espera', '3', 'FAE', 'ESP', [('L-7-2099', '22222222-1', 'Persona Ficticia Laja', 'FAE PRO Ficticio')])])
    return [lote, otro], bitacoras


CON_PRODUCTOS = ('identidad_visible_distinta', 'observacion_en_el_mes', 'cumplimiento_con_espera_alerta',
                 'consulta_sin_alcance_de_bitacoras', 'tribunal_elegido_con_registros')

# -- regeneración desde un Registros.xlsx revisado y plantilla propia ---------------------------------------------------

CORE_FIJO = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
             '<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" '
             'xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" '
             'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">'
             '<dcterms:created xsi:type="dcterms:W3CDTF">2026-01-01T00:00:00Z</dcterms:created>'
             '<dcterms:modified xsi:type="dcterms:W3CDTF">2026-01-01T00:00:00Z</dcterms:modified>'
             '</cp:coreProperties>')


def fijar_xlsx(datos):
    """Libro XLSX con bytes deterministas: sin fechas de openpyxl ni compresión dependiente del zlib del ejecutor."""
    from zipfile import ZIP_STORED, ZipFile, ZipInfo

    with ZipFile(io.BytesIO(datos)) as libro:
        partes = [(i.filename, libro.read(i.filename)) for i in libro.infolist()]
    salida = io.BytesIO()
    with ZipFile(salida, 'w', ZIP_STORED) as libro:
        for nombre, contenido in partes:
            if nombre == 'docProps/core.xml':
                contenido = CORE_FIJO.encode('utf-8')
            info = ZipInfo(nombre, date_time=(2026, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.compress_type = ZIP_STORED
            libro.writestr(info, contenido)
    return salida.getvalue()


def guardar_xlsx(libro):
    buffer = io.BytesIO()
    libro.save(buffer)
    return fijar_xlsx(buffer.getvalue())


def editar_registros(ruta):
    """Edita a mano las celdas de revisión de la fila 2 de CUMPLIMIENTO: control, fecha y etapa. La ALERTA no se
    edita: se recalcula al regenerar, igual que en Python."""
    libro = load_workbook(ruta)
    hoja = libro['CUMPLIMIENTO']
    columnas = {celda.value: indice for indice, celda in enumerate(hoja[1], 1)}
    hoja.cell(2, columnas['CONTROL BITÁCORA'], 'REVISADO: control revisado por el usuario')
    hoja.cell(2, columnas['FECHA OBS.'], date(2026, 8, 20))
    hoja.cell(2, columnas['ETAPA ÚLTIMA OBS.'], 'Espera')
    ruta.write_bytes(guardar_xlsx(libro))


def reporte_para_regenerar(previo):
    """Un solo JSON con las claves que lee Python (`regenerar`) y las que lee `flow_meta4::regenerar::cargar`."""
    informe = dict(previo)
    informe.update({
        'herramienta': 'Flow · Meta 4',
        'periodo': {'desde': previo['desde'], 'hasta': previo['hasta']},
        'fecha_emision': previo['hasta'],
        'tribunales_incluidos': 'Todos' if previo.get('tribunales', 'TODOS') == 'TODOS' else previo['tribunales'],
        'lotes': list(previo.get('lotes_registros', [])),
        'avisos': [],
        'consultas_sin_bitacoras_detalle': [
            {'consulta': etiqueta,
             'tribunal': mb.meta.tribunal_name(etiqueta.split(' / ')[0]) or mb.SIN_TRIBUNAL}
            for etiqueta in previo.get('consultas_sin_bitacoras', [])],
    })
    return informe


def proyeccion_reporte(informe):
    """Cifras del reporte que se contrastan con las del reporte de Rust (los nombres de campo difieren)."""
    return {clave: informe[clave] for clave in
            ('desde', 'hasta', 'mes_emision', 'registros', 'alertas', 'revision_bitacora', 'controles_bitacora',
             'resumen_tribunales', 'totales_meta4', 'excluidos_meta4', 'tribunales')}


def normalizar(valor, reemplazos):
    if isinstance(valor, str):
        for antes, despues in reemplazos:
            valor = valor.replace(antes, despues)
        return valor
    if isinstance(valor, list):
        return [normalizar(v, reemplazos) for v in valor]
    if isinstance(valor, dict):
        return {k: normalizar(v, reemplazos) for k, v in valor.items()}
    return valor


def b64(ruta):
    return base64.b64encode(ruta.read_bytes()).decode()


def regeneracion(escenario, con_bitacoras, con_plantilla):
    """Genera Meta 4, edita las celdas de revisión y regenera con Python; el resultado se compara con Rust."""
    base = Path(tempfile.mkdtemp(prefix='meta4-reg-'))
    try:
        lote, bitacoras = preparar(base, escenario)
        coincide_identidad(lote, bitacoras)
        generado = base / 'Generado'
        mb.generar([lote], generado, CORTE, emission=CORTE, bitacoras=[bitacoras])
        registros = generado / 'Registros.xlsx'
        registros.write_bytes(fijar_xlsx(registros.read_bytes()))
        del_mes = generado / 'Bitacoras_del_mes.xlsx'
        del_mes.write_bytes(fijar_xlsx(del_mes.read_bytes()))
        editar_registros(registros)
        plantilla = None
        if con_plantilla:
            plantilla = base / 'plantilla.xlsx'
            plantilla.write_bytes(plantilla_valida())
        escribir(generado / 'Reporte META4.json', reporte_para_regenerar(leer(generado / 'Reporte META4.json')))
        editado = datetime.fromtimestamp(registros.stat().st_mtime).strftime('%d-%m-%Y %H:%M')
        if not con_bitacoras:
            del_mes.unlink()
        salida = base / 'Regenerado'
        mb.regenerar(registros, salida, template=plantilla)
        entrada = {'Registros.xlsx': b64(registros), 'Reporte META4.json': b64(generado / 'Reporte META4.json')}
        if con_bitacoras:
            entrada['Bitacoras_del_mes.xlsx'] = b64(del_mes)
        productos = {p.name: volcar(p) for p in sorted(salida.glob('*.xlsx'))}
        return {
            'entrada': entrada,
            'python': {
                'productos': {'libros': normalizar(productos, [(str(base), '<BASE>'), (editado, '<EDITADO>')])},
                'reporte': proyeccion_reporte(leer(salida / 'Reporte META4.json')),
            },
        }
    finally:
        shutil.rmtree(base, ignore_errors=True)


def plantilla_valida():
    return guardar_xlsx(mb.meta.build_default_template())


def plantillas():
    """Plantillas sintéticas: válida, sin una hoja, sin anchos de columna, con celdas combinadas y un archivo que no
    es Excel. Se compara lo que acepta `validar_plantilla` de Python con lo que acepta Rust al publicar."""
    base = Path(tempfile.mkdtemp(prefix='meta4-plantillas-'))
    try:
        valida = base / 'valida.xlsx'
        valida.write_bytes(plantilla_valida())

        sin_hoja = base / 'sin_hoja.xlsx'
        libro = load_workbook(valida)
        del libro['FAE Cumplimiento']
        sin_hoja.write_bytes(guardar_xlsx(libro))

        sin_anchos = base / 'sin_anchos.xlsx'
        libro = load_workbook(valida)
        for hoja in libro.worksheets:
            hoja.column_dimensions.clear()
        sin_anchos.write_bytes(guardar_xlsx(libro))

        combinadas = base / 'combinadas.xlsx'
        libro = load_workbook(valida)
        libro['Residencia Espera'].merge_cells('A1:B1')
        combinadas.write_bytes(guardar_xlsx(libro))

        corrupta = base / 'corrupta.xlsx'
        corrupta.write_bytes(b'esto no es un libro de Excel')

        casos = []
        for nombre, ruta in [('plantilla_valida', valida), ('plantilla_sin_una_hoja', sin_hoja),
                             ('plantilla_sin_anchos', sin_anchos), ('plantilla_con_combinadas', combinadas),
                             ('plantilla_ilegible', corrupta)]:
            try:
                mb.validar_plantilla(ruta)
                python = {'acepta': True, 'detalle': ''}
            except ValueError as error:
                python = {'acepta': False, 'detalle': str(error)}
            casos.append({'caso': nombre, 'archivo': b64(ruta), 'python': python})
        return casos
    finally:
        shutil.rmtree(base, ignore_errors=True)


def regeneraciones(escenario):
    return [
        dict(escenario='con_bitacoras', **regeneracion(escenario, True, False)),
        dict(escenario='sin_bitacoras', **regeneracion(escenario, False, False)),
        dict(escenario='con_plantilla', **regeneracion(escenario, True, True)),
    ]


CASOS = [
    # nombre, variante, corte, tribunales, lotes extra (copias del lote), con carpeta de bitácoras
    ('identidad_visible_distinta', None, CORTE, [], 0, True),
    ('observacion_en_el_mes', coincide_identidad, CORTE, [], 0, True),
    ('observacion_fuera_del_mes', coincide_identidad, date(2026, 10, 8), [], 0, True),
    ('leida_antes_del_corte', coincide_identidad, date(2026, 10, 20), [], 0, True),
    ('nombre_visible_distinto', nombre_visible, CORTE, [], 0, True),
    ('cumplimiento_con_espera_alerta', espera_con_alerta, CORTE, [], 0, True),
    ('archivo_de_bitacora_alterado', archivo_alterado, CORTE, [], 0, True),
    ('lectura_fallida', lectura_fallida, CORTE, [], 0, True),
    ('calendario_fallido_con_bitacora_leida', calendario_fallido, CORTE, [], 0, True),
    ('sin_bitacora_asociada', sin_bitacora, CORTE, [], 0, True),
    ('identidad_ambigua', ambigua, CORTE, [], 0, True),
    ('consulta_sin_alcance_de_bitacoras', sin_alcance, CORTE, [], 0, True),
    ('bitacoras_con_filtro_de_fechas', bitacoras_con_filtro, CORTE, [], 0, True),
    ('lote_con_filtros', lote_con_filtros, CORTE, [], 0, True),
    ('lote_incompleto', lote_incompleto, CORTE, [], 0, True),
    ('faltan_paginas_del_descargador', faltan_paginas, CORTE, [], 0, True),
    ('consulta_no_validada', consulta_no_validada, CORTE, [], 0, True),
    ('pagina_alterada_tras_verificar', pagina_alterada, CORTE, [], 0, True),
    ('consulta_en_dos_lotes', coincide_identidad, CORTE, [], 1, True),
    ('tribunal_elegido_sin_registros', coincide_identidad, CORTE, ['Laja'], 0, True),
    ('tribunal_elegido_con_registros', coincide_identidad, CORTE, ['Tomé'], 0, True),
    ('sin_carpeta_de_bitacoras', coincide_identidad, CORTE, [], 0, False),
    # -- contraste ampliado (Meta 4: Espera, FAE, otros tribunales, ventana y alcance) --
    ('espera_residencia_con_bitacora', a_espera, CORTE, [], 0, True),
    ('fae_y_residencia_en_el_mismo_lote', fae_en_el_lote, CORTE, [], 0, True),
    ('tome_y_laja_en_el_mismo_lote', laja_en_el_lote, CORTE, [], 0, True),
    ('filtro_con_dos_tribunales', laja_en_el_lote, CORTE, ['TOMÉ', 'LAJA'], 0, True),
    ('filtro_de_laja_sin_tome', laja_en_el_lote, CORTE, ['LAJA'], 0, True),
    ('solo_mulchen_sin_bitacoras', mulchen_en_el_lote, CORTE, [], 0, True),
    ('dos_lotes_que_no_se_solapan', None, CORTE, [], 0, True, dos_lotes),
    ('observaciones_el_mismo_dia_con_etapas_distintas', etapas_distintas, CORTE, [], 0, True),
    ('bordes_de_la_ventana', bordes_de_la_ventana, CORTE, [], 0, True),
    ('fuera_de_la_ventana', fuera_de_la_ventana, CORTE, [], 0, True),
    ('programa_fuera_de_fae_y_residencia', programa_ajeno, CORTE, [], 0, True),
]


def celda(v):
    if v is None:
        return None
    if isinstance(v, bool):
        return ['b', v]
    if isinstance(v, (int, float)):
        return ['n', v]
    if isinstance(v, (datetime, date)):
        return ['d', v.isoformat()]
    return ['s', str(v)]


def volcar(ruta):
    """Hojas, celdas con su tipo, panel congelado, filtro y anchos de un libro (valores, no estilos)."""
    libro = load_workbook(ruta)
    hojas = []
    for hoja in libro.worksheets:
        anchos = {k: d.width for k, d in sorted(hoja.column_dimensions.items()) if d.customWidth}
        filas = [[celda(c.value) for c in fila] for fila in hoja.iter_rows()]
        while filas and all(v is None for v in filas[-1]):
            filas.pop()
        filas = [f[: max((i + 1 for i, v in enumerate(f) if v is not None), default=0)] for f in filas]
        hojas.append({'titulo': hoja.title, 'congelar': hoja.freeze_panes, 'filtro': hoja.auto_filter.ref,
                      'anchos': anchos, 'celdas': filas})
    libro.close()
    return {'hojas': hojas}


def productos(lotes, bitacoras, corte, tribunales, destino):
    """Los libros y el reporte que escribe `generar` (fecha de emisión = corte, para que no dependan del día)."""
    mb.generar(lotes, destino, corte, emission=corte, bitacoras=bitacoras, tribunales=tribunales or None)
    salida = {p.name: volcar(p) for p in sorted(destino.glob('*.xlsx'))}
    reporte = leer(destino / 'Reporte META4.json')
    return {'libros': salida, 'reporte': reporte}


def capturar(lotes, bitacoras, corte, tribunales, destino):
    """Corre `generar` con `_escribir` sustituido para quedarse con lo que entrega, sin escribir libros."""
    captura = {}

    def escribir_falso(trabajo, corte_, emision, plantilla, lectura, faltan, files, issues, selected, groups, rows,
                       tribunales_, filtros, lotes_):
        captura.update(lectura=lectura, faltan=faltan, rows=rows, filtros=filtros, lotes=lotes_, files=files)

    original = mb._escribir
    mb._escribir = escribir_falso
    try:
        mb.generar(lotes, destino, corte, bitacoras=bitacoras, tribunales=tribunales or None)
    except ValueError as error:
        return {'error': str(error)}
    finally:
        mb._escribir = original
    filas = []
    for lista, grupo in captura['rows'].items():
        for fila in grupo:
            v = fila.values
            filas.append({
                'lista': lista, 'categoria': fila.category, 'tribunal': fila.tribunal,
                'rit': v.get('RIT') or '', 'rut': v.get('RUT') or '', 'nombre': v.get('NOMBRE') or '',
                'derivacion': v.get('DERIVACIÓN') or '',
                'fecha_obs': v['FECHA OBS.'].isoformat() if v.get('FECHA OBS.') else None,
                'etapa': v[mb.EXTRA[0]], 'control': v[mb.EXTRA[1]], 'alerta': v.get('ALERTA') or '',
            })
    lectura = captura['lectura']
    # `_escribir` filtra las bitácoras del tribunal elegido antes de exportarlas: el control de lecturas del producto
    # es el filtrado, no el de la lectura completa.
    elegidos = {mb.meta.tribunal_name(t) for t in (tribunales or [])}
    diarios = [r for r in lectura.registros
               if not elegidos or mb.meta.tribunal_name(r.get('tribunal_nombre')) in elegidos]
    return {
        'filas': filas,
        'lecturas': [{'rit': r.get('rit', ''), 'ingreso': str(r.get('ingreso_id', '')), 'lectura': r.get('estado', ''),
                      'control': r['control']} for r in diarios],
        'avisos_de_lectura': lectura.avisos,
        'filtros': captura['filtros'],
        'lotes': captura['lotes'],
        'sin_bitacoras': [[etiqueta, tribunal] for etiqueta, tribunal in captura['faltan']],
        # Filas que Python deja fuera de Registros por una derivación que no es FAE ni Residencia (Rust las conserva
        # en su libro Registros y las cuenta en «fuera_de_meta4»).
        'fuera_de_alcance': sum(len(f.orphan_rows) for f in captura['files']),
    }


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'meta4.json'
    escenario = next(e for e in leer(RAIZ / 'app' / 'tests' / 'datos' / 'paquete.json')['escenarios']
                     if e['escenario'] == 'contexto')
    casos = []
    for caso in CASOS:
        nombre, variante, corte, tribunales, extra, con_bitacoras = caso[:6]
        construir = caso[6] if len(caso) > 6 else None
        base = Path(tempfile.mkdtemp(prefix='meta4-'))
        try:
            if construir:
                carpetas, bitacoras = construir(base, escenario)
            else:
                lote, bitacoras = preparar(base, escenario)
                if variante:
                    variante(lote, bitacoras)
                carpetas = [lote]
                for n in range(extra):
                    copia = base / f'lote_{n + 2}'
                    shutil.copytree(lote, copia)
                    carpetas.append(copia)
            resultado = capturar(carpetas, [bitacoras] if con_bitacoras else None, corte, tribunales, base / 'Resultado META4')
            if nombre in CON_PRODUCTOS and 'error' not in resultado:
                resultado['productos'] = productos(carpetas, [bitacoras], corte, tribunales, base / 'Productos META4')
            casos.append({
                'caso': nombre, 'corte': corte.isoformat(), 'tribunales': tribunales,
                'lotes': {c.name: volcar_carpeta(c) for c in carpetas},
                'bitacoras': volcar_carpeta(bitacoras, lee_meta4) if con_bitacoras else None,
                'python': resultado,
            })
        finally:
            shutil.rmtree(base, ignore_errors=True)
    poner(destino, json.dumps({'casos': casos, 'regeneraciones': regeneraciones(escenario), 'plantillas': plantillas()},
                              ensure_ascii=False, indent=1) + '\n')
    for c in casos:
        p = c['python']
        print(c['caso'], '->', p.get('error') or [(f['lista'], f['tribunal'], f['fecha_obs'], f['control'][:40], f['alerta'][:8])
                                               for f in p['filas']])


if __name__ == '__main__':
    main()
