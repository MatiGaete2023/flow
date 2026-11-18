"""Ingreso Histórico Protección: captura MANUAL de la tabla visible de la persona seleccionada (plan 18-nov-2026).

El funcionario abre la vista en su sesión de SITFA y la entrega a este módulo (HTML guardado desde el navegador o pegado de
la vista abierta). Este módulo no consulta nada, no sigue enlaces ni hace consultas por RUT en segundo plano. Cada captura
guarda el HTML tal cual (con su huella), el resultado del lector y la verificación de pertenencia a la persona de la fila
seleccionada. Si no se prueba la pertenencia, la captura queda INCIERTA y el motor no afirma otro programa; el resto de las
fuentes sigue su curso. El manifiesto `historico.json` lo lee `export_csmp_capture._historico`.
"""

import json
from datetime import datetime
from hashlib import sha256
from pathlib import Path

import motor as m
from lectura_bitacoras import write
from nurus.escritos_html import mensaje, read_historico

MANIFEST = 'historico.json'
MAX_BYTES = 8 * 1024 * 1024


def _estado(carpeta):
    ruta = Path(carpeta) / MANIFEST
    if ruta.is_file():
        datos = json.loads(ruta.read_text(encoding='utf-8'))
        if datos.get('tipo') == 'HISTORICO_PROTECCION' and datos.get('version') == 1:
            return datos
    return {'version': 1, 'tipo': 'HISTORICO_PROTECCION', 'capturas': {}}


def capturar(carpeta, html, persona, *, capturada=None):
    """Guarda una captura manual. `persona`: dict con `tribunal` (código), `tribunal_nombre`, `person_id` (SITFA_PERSONA_RUS de
    la fila seleccionada), `rut` y `nombre` (solo para comprobar la pertenencia en el texto visible; no se guardan).
    Devuelve la entrada del manifiesto. Vuelve a capturar a la misma persona: reemplaza la anterior (la última manda)."""
    carpeta = Path(carpeta)
    datos = html.encode('utf-8') if isinstance(html, str) else bytes(html)
    if not datos or len(datos) > MAX_BYTES:
        raise m.PocError('La vista del histórico está vacía o supera 8 MiB.')
    for clave in ('tribunal', 'person_id'):
        if not str(persona.get(clave, '')).strip():
            raise m.PocError('Falta ' + clave + ' de la persona seleccionada: no se puede asociar el histórico.')
    capturada = capturada or datetime.now().astimezone().isoformat()
    huella = sha256(datos).hexdigest()
    entrada = {
        'tribunal': str(persona['tribunal']),
        'tribunal_nombre': str(persona.get('tribunal_nombre', '')),
        'person_id': str(persona['person_id']),
        'archivo': 'historico-' + huella[:24] + '.html',
        'sha256': huella,
        'leida': capturada,
    }
    try:
        lectura = read_historico(
            datos, captured_at=capturada, persona_esperada=[persona.get('rut', ''), persona.get('nombre', '')]
        )
    except ValueError as exc:
        entrada.update(estado='FALLIDA', error=str(exc), ownership='INCIERTA', filas=[], advertencias=[mensaje(str(exc))])
    else:
        entrada.update(
            estado='VACIA_COMPROBADA' if lectura['vacio_explicito'] else 'LEIDA',
            error='',
            ownership=lectura['ownership'],
            filas=[
                {'fila': f['fila'], 'programa': f['Programa'], 'fecha_egreso': f['Fecha egreso'], 'causal_egreso': f['Causal egreso']}
                for f in lectura['filas']
            ],
            advertencias=[a['codigo'] for a in lectura['advertencias']],
        )
    ruta = carpeta / 'historico' / entrada['archivo']
    ruta.parent.mkdir(parents=True, exist_ok=True)
    if not ruta.exists():
        m.save_exclusive(ruta, datos)
    estado = _estado(carpeta)
    estado['capturas'][entrada['tribunal'] + '|' + entrada['person_id']] = entrada
    write(carpeta / MANIFEST, estado)
    return entrada


def cargar(carpeta):
    """(manifiesto, [entradas]) con el HTML respaldado verificado contra su huella; una discrepancia es error, no vacío."""
    carpeta = Path(carpeta)
    estado = _estado(carpeta)
    for entrada in estado['capturas'].values():
        ruta = carpeta / 'historico' / entrada['archivo']
        if not ruta.is_file() or sha256(ruta.read_bytes()).hexdigest() != entrada['sha256']:
            raise ValueError('El respaldo de un histórico no coincide con su huella.')
    return estado, list(estado['capturas'].values())
