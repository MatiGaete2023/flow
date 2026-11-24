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


CON_PRODUCTOS = ('identidad_visible_distinta', 'observacion_en_el_mes', 'cumplimiento_con_espera_alerta',
                 'consulta_sin_alcance_de_bitacoras', 'tribunal_elegido_con_registros')

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
        captura.update(lectura=lectura, faltan=faltan, rows=rows, filtros=filtros, lotes=lotes_)

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
    return {
        'filas': filas,
        'lecturas': [{'rit': r.get('rit', ''), 'ingreso': str(r.get('ingreso_id', '')), 'lectura': r.get('estado', ''),
                      'control': r['control']} for r in lectura.registros],
        'avisos_de_lectura': lectura.avisos,
        'filtros': captura['filtros'],
        'lotes': captura['lotes'],
        'sin_bitacoras': [[etiqueta, tribunal] for etiqueta, tribunal in captura['faltan']],
    }


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'meta4.json'
    escenario = next(e for e in leer(RAIZ / 'app' / 'tests' / 'datos' / 'paquete.json')['escenarios']
                     if e['escenario'] == 'contexto')
    casos = []
    for nombre, variante, corte, tribunales, extra, con_bitacoras in CASOS:
        base = Path(tempfile.mkdtemp(prefix='meta4-'))
        try:
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
    poner(destino, json.dumps({'casos': casos}, ensure_ascii=False, indent=1) + '\n')
    for c in casos:
        p = c['python']
        print(c['caso'], '->', p.get('error') or [(f['lista'], f['tribunal'], f['fecha_obs'], f['control'][:40], f['alerta'][:8])
                                               for f in p['filas']])


if __name__ == '__main__':
    main()
