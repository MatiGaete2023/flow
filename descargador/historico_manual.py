"""Ingreso Histórico Protección: captura MANUAL de la tabla visible de la persona seleccionada (plan 18-nov-2026).

El funcionario abre la vista en su sesión de SITFA y la entrega a este módulo (HTML guardado desde el navegador o pegado de
la vista abierta). Este módulo no consulta nada, no sigue enlaces ni hace consultas por RUT en segundo plano. Cada captura
guarda el HTML tal cual (con su huella), el resultado del lector y la verificación de pertenencia a la persona de la fila
seleccionada. Si no se prueba la pertenencia, la captura queda INCIERTA y el motor no afirma otro programa; el resto de las
fuentes sigue su curso. El manifiesto `historico.json` lo lee `export_csmp_capture._historico`.
"""

import argparse
import json
import re
import sys
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


def _rut_limpio(texto):
    return re.sub(r'[^0-9kK]', '', str(texto or '')).upper()


def rut_de_url(url):
    """RUT (sin puntos ni guion) que la propia dirección de la vista pide (`RUT_Litigante` + `RUT_DV`), o ''. Es la prueba más
    fuerte de pertenencia: la ventana se abrió para esa persona. No se guarda la dirección (puede traer valores de sesión)."""
    cuerpo = re.search(r'[?&]RUT_Litigante=(\d{1,9})(?:&|$)', str(url or ''))
    dv = re.search(r'[?&]RUT_DV=([0-9kK])(?:&|$)', str(url or ''))
    return (cuerpo.group(1) + dv.group(1).upper()) if cuerpo and dv else ''


def capturar(carpeta, html, persona, *, capturada=None, url=''):
    """Guarda una captura manual. `persona`: dict con `tribunal` (código), `tribunal_nombre`, `person_id` (SITFA_PERSONA_RUS de
    la fila seleccionada), `rut` y `nombre` (solo para comprobar la pertenencia en el texto visible; no se guardan). `url`: la
    dirección de la ventana, solo para leer el RUT que pide: si coincide con el de la persona, la pertenencia queda verificada;
    si pide otro RUT, queda incierta aunque el texto coincida.
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
        del_url, esperado = rut_de_url(url), _rut_limpio(persona.get('rut', ''))
        propiedad = lectura['ownership']
        if del_url and esperado:
            propiedad = 'VERIFICADA' if del_url == esperado else 'INCIERTA'
        entrada.update(
            estado='VACIA_COMPROBADA' if lectura['vacio_explicito'] else 'LEIDA',
            error='',
            ownership=propiedad,
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


def main(argv=None):
    """Captura manual desde un HTML guardado de la ventana «Historial Menores» (sin interfaz):
    python historico_manual.py --carpeta LOTE --html vista.html --tribunal 113 --person-id ID --rut 11.111.111-1
    [--nombre 'Nombre Apellido'] [--tribunal-nombre ...] [--url 'DIRECCION_DE_LA_VENTANA']"""
    ap = argparse.ArgumentParser(description=main.__doc__)
    ap.add_argument('--carpeta', required=True)
    ap.add_argument('--html', required=True)
    ap.add_argument('--tribunal', required=True)
    ap.add_argument('--person-id', required=True)
    ap.add_argument('--rut', default='')
    ap.add_argument('--nombre', default='')
    ap.add_argument('--tribunal-nombre', default='')
    ap.add_argument('--url', default='')
    a = ap.parse_args(argv)
    persona = {'tribunal': a.tribunal, 'tribunal_nombre': a.tribunal_nombre, 'person_id': a.person_id, 'rut': a.rut, 'nombre': a.nombre}
    entrada = capturar(a.carpeta, Path(a.html).read_bytes(), persona, url=a.url)
    print(f"{entrada['estado']} · pertenencia {entrada['ownership']} · {len(entrada['filas'])} programas")
    for f in entrada['filas']:
        print('  ', f['programa'], '|', f['fecha_egreso'], '|', f['causal_egreso'])
    return 0 if entrada['estado'] != 'FALLIDA' else 1


if __name__ == '__main__':
    sys.exit(main())
