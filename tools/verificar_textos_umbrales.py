"""E-G · Compara textos y umbrales de Flow Rust (`app/src/config.rs`) con Flow Python, en ambos sentidos.

- TEXTOS: cada clave de `config.rs::TEXTOS` debe existir en `textos_base.json` con el mismo texto, y cada texto de
  `textos_base.json` debe existir en Rust o estar en EXCLUIDOS_RUST con su motivo.
- UMBRALES: claves, valores y etiquetas iguales a `nurus.personal.config` en ambos sentidos.
- Lectura estricta: los literales se leen con escapes de Rust y se cuentan contra el largo declarado del arreglo
  (`[(&str, &str); N]`). Toda línea del bloque que no se pudo leer se informa.
- Sale con código 1 ante cualquier discrepancia o entrada no leída; 0 si todo coincide.

Uso (desde la raíz): python tools/verificar_textos_umbrales.py
"""

import json
import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[1]
CONFIG_RS = RAIZ / 'app' / 'src' / 'config.rs'
TEXTOS_PY = RAIZ / 'src' / 'nurus' / 'personal' / 'textos_base.json'

# Textos de Python que Flow Rust no usa a propósito. Cada entrada necesita un motivo verificable.
EXCLUIDOS_RUST = {
    'HISTORIA.VARIOS': 'inalcanzable en Python: solo lo emite reglas_historia.constancia() con dos o más trámites, '
    'y la única función que se los pasa, constancias(), no tiene llamadores (rust/oraculo/generar_casos.py, '
    'fuera_del_motor)',
}

LIT = r'"((?:[^"\\]|\\.)*)"'


def desescapar(s):
    def uno(m):
        e = m.group(1)
        if e.startswith('u{'):
            return chr(int(e[2:-1], 16))
        return {'n': '\n', 't': '\t', 'r': '\r', '0': '\0', '\\': '\\', '"': '"', "'": "'"}[e]

    return re.sub(r'\\(u\{[0-9a-fA-F]+\}|[ntr0\\"\'])', uno, s)


def bloque(fuente, nombre):
    m = re.search(rf'pub const {nombre}: \[\(([^\]]+)\); (\d+)\] = \[\n(.*?)\n\];', fuente, re.S)
    if not m:
        raise SystemExit(f'No se encontró el arreglo {nombre} en {CONFIG_RS}')
    return m.group(1), int(m.group(2)), m.group(3)


def leer(fuente, nombre, patron):
    _, declarado, cuerpo = bloque(fuente, nombre)
    leidas, no_leidas = [], []
    for n, linea in enumerate(cuerpo.split('\n'), 1):
        t = linea.strip()
        if not t or t.startswith('//'):
            continue
        m = re.fullmatch(patron, t)
        if m:
            leidas.append(m.groups())
        else:
            no_leidas.append(f'{nombre} línea {n} del bloque: {t[:120]}')
    if len(leidas) != declarado:
        no_leidas.append(f'{nombre}: declarado {declarado}, leídos {len(leidas)}')
    return leidas, no_leidas


def main():
    fuente = CONFIG_RS.read_text(encoding='utf-8')
    errores = []

    textos_rs, no_leidas = leer(fuente, 'TEXTOS', rf'\({LIT},\s*{LIT}\),?')
    errores += no_leidas
    textos_rs = {desescapar(k): desescapar(v) for k, v in textos_rs}
    base = json.loads(TEXTOS_PY.read_text(encoding='utf-8'))
    textos_py = {}
    for sec, entradas in base.items():
        if sec.startswith('_') or not isinstance(entradas, dict):
            continue
        for clave, entrada in entradas.items():
            textos_py[f'{sec}.{clave}'] = entrada['texto'] if isinstance(entrada, dict) else entrada
    for clave, texto in sorted(textos_rs.items()):
        if clave not in textos_py:
            errores.append(f'TEXTOS: {clave} está en Rust y no en textos_base.json')
        elif texto != textos_py[clave]:
            errores.append(f'TEXTOS: {clave} distinto\n    rs={texto!r}\n    py={textos_py[clave]!r}')
    for clave in sorted(set(textos_py) - set(textos_rs)):
        if clave in EXCLUIDOS_RUST:
            print(f'EXCLUIDO {clave}: {EXCLUIDOS_RUST[clave]}')
        else:
            errores.append(f'TEXTOS: {clave} está en textos_base.json y no en Rust (ni en EXCLUIDOS_RUST)')
    for clave in sorted(set(EXCLUIDOS_RUST) - (set(textos_py) - set(textos_rs))):
        errores.append(f'EXCLUIDOS_RUST: {clave} ya no corresponde (no existe en Python o ya está en Rust)')

    umbrales_rs, no_leidas = leer(fuente, 'UMBRALES', rf'\({LIT},\s*(-?\d+),\s*{LIT}\),?')
    errores += no_leidas
    sys.path.insert(0, str(RAIZ / 'src'))
    from nurus.personal.config import PARAMETER_LABELS, UMBRALES

    rs = {desescapar(k): (int(v), desescapar(e)) for k, v, e in umbrales_rs}
    for clave, (valor, etiqueta) in sorted(rs.items()):
        if clave not in UMBRALES:
            errores.append(f'UMBRALES: {clave} está en Rust y no en Python')
            continue
        if UMBRALES[clave] != valor:
            errores.append(f'UMBRALES: {clave} valor rs={valor} py={UMBRALES[clave]}')
        if PARAMETER_LABELS.get(clave) != etiqueta:
            errores.append(f'UMBRALES: {clave} etiqueta\n    rs={etiqueta!r}\n    py={PARAMETER_LABELS.get(clave)!r}')
    for clave in sorted(set(UMBRALES) - set(rs)):
        errores.append(f'UMBRALES: {clave} está en Python y no en Rust')

    print(f'TEXTOS: Rust {len(textos_rs)}, Python {len(textos_py)}, excluidos {len(EXCLUIDOS_RUST)}')
    print(f'UMBRALES: Rust {len(rs)}, Python {len(UMBRALES)}')
    for e in errores:
        print('DISCREPANCIA', e)
    print(f'RESULTADO: {"FALLA" if errores else "PASA"} ({len(errores)} discrepancias)')
    return 1 if errores else 0


if __name__ == '__main__':
    sys.exit(main())
