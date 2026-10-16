"""Historia de la causa (ventana de 4 meses): lectura verificada del respaldo y Excel de revisión."""

import hashlib
import json
import os
import tempfile
from datetime import date, datetime
from pathlib import Path

from nurus.capture.diaries import text_parts
from nurus.historia_html import read_historia, separar_ventana

LEIDAS = ('LEIDA', 'VACIA_COMPROBADA')
LEYENDAS = (
    'Sin PDF: no se abrieron documentos.',
    'La historia es de la causa, no de un ingreso.',
    'Solo trámites entre DESDE y HASTA; no acredita ausencia de trámites fuera de ese rango.',
)
LIMITE_TEXTO = 30000
MAX_BYTES = 8 * 1024 * 1024
HOJAS = {
    'Trámites': (
        'Tribunal',
        'RIT',
        'Folio',
        'Fecha Tram',
        'Etapa',
        'Trámite',
        'Desc. Trámite',
        'Referencia',
        'Anexo visible',
        'Causa ID',
        'Archivo fuente',
        'Fila fuente',
        'SHA-256 fuente',
        'Leída el',
    ),
    'Causas': (
        'Tribunal',
        'RIT',
        'Estado',
        'Trámites en ventana',
        'Trámites anteriores',
        'Filas incompletas',
        'Desde',
        'Hasta',
        'Advertencias / error',
        'RIT visible',
        'RUC',
        'Caratulado',
        'F. Ingreso',
        'Proc.',
        'Forma Inicio',
        'Est. Adm.',
        'Etapa',
        'Estado Proc.',
        'Tribunal visible',
        'Reservada',
        'Causa ID',
        'Archivo fuente',
        'SHA-256',
    ),
    'Vínculos': (
        'Tribunal',
        'RIT',
        'Nombre',
        'RUT',
        'Programa',
        'Pestaña',
        'Modalidad',
        'Ingreso ID',
        'Causa ID',
        'Estado de la historia',
    ),
}


def literal(value):
    if value is None:
        return ''
    if isinstance(value, (date, datetime)):
        return value.isoformat()
    return str(value)


def cargar(manifest_path):
    """(estado, causas). Cada causa leída se relee del HTML respaldado y se verifica su SHA-256."""
    manifest = Path(manifest_path)
    estado = json.loads(manifest.read_text(encoding='utf-8'))
    if estado.get('version') != 1 or estado.get('tipo') != 'HISTORIA_LECTURA':
        raise ValueError('El archivo no es una lectura de historias.')
    causas = []
    for clave, causa in estado['causas'].items():
        item = {'clave': clave, 'causa': causa, 'lectura': None, 'ventana': [], 'anteriores': []}
        if causa['estado'] in LEIDAS:
            ruta = manifest.parent / causa['archivo']
            if not ruta.is_file() or ruta.stat().st_size > MAX_BYTES:
                raise ValueError('Falta el respaldo de una historia leída.')
            data = ruta.read_bytes()
            if hashlib.sha256(data).hexdigest() != causa['sha256']:
                raise ValueError('El respaldo de una historia no coincide con su SHA-256.')
            lectura = read_historia(data)
            leida = datetime.fromisoformat(causa['leida'])
            dentro, anteriores, posteriores, sin_fecha = separar_ventana(lectura['filas'], leida.date())
            if posteriores:
                causa['advertencias'] = list(
                    dict.fromkeys([*causa.get('advertencias', []), 'HISTORIA_FECHA_POSTERIOR'])
                )
            item.update(lectura=lectura, anteriores=anteriores)
            item['ventana'] = sorted(dentro + posteriores + sin_fecha, key=lambda f: f['fila'])
        causas.append(item)
    return estado, causas


def _folio_clave(folio):
    return (0, -int(folio)) if folio.isdigit() else (1, 0)


def _si_no(valor):
    return 'Sí' if valor else 'No'


def export_historias(manifest_path, destination):
    from openpyxl import Workbook

    destination = Path(destination)
    if destination.exists():
        raise ValueError('El destino ya existe; elige otra copia.')
    estado, causas = cargar(manifest_path)
    nombres = {c['clave']: c['causa'].get('tribunal_nombre') or c['causa']['tribunal'] for c in causas}
    divididos = []
    book = Workbook()
    book.remove(book.active)
    hojas = {nombre: book.create_sheet(nombre) for nombre in (*HOJAS, 'Control')}
    for nombre, columnas in HOJAS.items():
        hojas[nombre].append(columnas)

    continuaciones = {}

    def agregar(hoja, valores):
        valores = list(valores)
        extra = {}
        for indice, valor in enumerate(valores):
            if not isinstance(valor, str):
                continue
            partes = text_parts(valor, LIMITE_TEXTO)
            valores[indice] = partes[0]
            if len(partes) > 1:
                divididos.append(1)
            for numero, parte in enumerate(partes[1:], 2):
                clave = (hoja.title, indice, numero)
                if clave not in continuaciones:
                    columna = max(len(valores), hoja.max_column) + 1
                    continuaciones[clave] = columna
                    encabezado = hoja.cell(1, indice + 1).value
                    hoja.cell(1, columna, f'{encabezado} (parte {numero})').data_type = 's'
                extra[continuaciones[clave]] = parte
        hoja.append(valores)
        numero = hoja.max_row
        for columna, valor in extra.items():
            hoja.cell(numero, columna, valor).data_type = 's'
        for columna in range(1, len(valores) + 1):
            celda = hoja.cell(numero, columna)
            if isinstance(celda.value, str):
                celda.data_type = 's'

    filas = [(item, fila) for item in causas for fila in item['ventana']]
    filas.sort(key=lambda p: _folio_clave(p[1]['folio']))
    filas.sort(key=lambda p: p[1]['fecha_iso'], reverse=True)
    filas.sort(key=lambda p: (nombres[p[0]['clave']], p[0]['causa']['rit']))
    ordenadas = filas
    for item, fila in ordenadas:
        causa = item['causa']
        agregar(
            hojas['Trámites'],
            [
                nombres[item['clave']],
                causa['rit'],
                fila['folio'],
                fila['fecha_texto'],
                literal(fila['etapa']),
                literal(fila['tramite']),
                literal(fila['descripcion']),
                literal(fila['referencia']),
                _si_no(fila['anexo']),
                causa['causa_id'],
                causa['archivo'],
                fila['fila'],
                causa['sha256'],
                causa['leida'],
            ],
        )
    for item in sorted(causas, key=lambda i: (nombres[i['clave']], i['causa']['rit'])):
        causa, lectura = item['causa'], item['lectura']
        cabecera = lectura['encabezado'] if lectura else {}
        avisos = causa.get('error') or '; '.join(causa.get('advertencias', []))
        agregar(
            hojas['Causas'],
            [
                nombres[item['clave']],
                causa['rit'],
                causa['estado'],
                causa.get('en_ventana', ''),
                causa.get('anteriores', ''),
                causa.get('incompletas', ''),
                causa.get('desde', ''),
                causa.get('hasta', ''),
                avisos,
                *(
                    literal(cabecera.get(k, ''))
                    for k in (
                        'RIT',
                        'RUC',
                        'Caratulado',
                        'F. Ingreso',
                        'Proc.',
                        'Forma Inicio',
                        'Est. Adm.',
                        'Etapa',
                        'Estado Proc.',
                        'Tribunal',
                    )
                ),
                _si_no(lectura['reservada']) if lectura else '',
                causa['causa_id'],
                causa.get('archivo', ''),
                causa.get('sha256', ''),
            ],
        )
    for v in estado['vinculos']:
        causa = estado['causas'][v['clave']]
        agregar(
            hojas['Vínculos'],
            [
                nombres.get(v['clave'], v['tribunal_codigo']),
                v['rit'],
                v['nombre'],
                v['rut'],
                v['programa'],
                v['pestana'],
                v['modalidad'],
                v['ingreso_id'],
                v['causa_id'],
                causa['estado'],
            ],
        )
    conteo = {}
    for item in causas:
        conteo[item['causa']['estado']] = conteo.get(item['causa']['estado'], 0) + 1
    leidas = sorted(c['causa']['leida'][:10] for c in causas if c['causa'].get('leida'))
    desdes = sorted(c['causa']['desde'] for c in causas if c['causa'].get('desde'))
    hastas = sorted(c['causa']['hasta'] for c in causas if c['causa'].get('hasta'))
    control = [
        ('Versión Descargador', estado.get('descargador', {}).get('version', '')),
        ('Versión extensión', estado.get('extension_version', '')),
        ('Commit', estado.get('descargador', {}).get('commit', '')),
        (
            'Fecha de lectura',
            (leidas[0] if leidas[0] == leidas[-1] else leidas[0] + " a " + leidas[-1]) if leidas else "",
        ),
        ('Criterio', estado.get('criterio_temporal', 'ventana_4_meses')),
        ('Desde', desdes[0] if desdes else ''),
        ('Hasta', hastas[-1] if hastas else ''),
        ('Causas por estado', '; '.join(f'{k}: {v}' for k, v in sorted(conteo.items())) or 'ninguna'),
        ('Tope alcanzado', _si_no(estado.get('tope_alcanzado'))),
        ('Textos divididos en columnas de continuación', len(divididos)),
        *(('Leyenda', texto) for texto in LEYENDAS),
    ]
    agregar(hojas['Control'], ('Campo', 'Valor'))
    for fila in control:
        agregar(hojas['Control'], fila)
    for hoja in book:
        hoja.freeze_panes = 'A2'
        hoja.auto_filter.ref = hoja.dimensions
    destination.parent.mkdir(parents=True, exist_ok=True)
    fd, temporal = tempfile.mkstemp(prefix='.historias-', suffix='.xlsx', dir=destination.parent)
    os.close(fd)
    try:
        book.save(temporal)
        if destination.exists():
            raise ValueError('El destino ya existe; elige otra copia.')
        os.replace(temporal, destination)
    finally:
        Path(temporal).unlink(missing_ok=True)
        book.close()
    return destination
