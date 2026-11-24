"""E-X1: guardia de solo lectura de RUS y SITFA.

Falla (código 1) si el código de producción de `app/src` o las extensiones del Descargador mencionan los controles de
creación o edición de observaciones de la bitácora de RUS, o un envío de Outlook (`.Send(`). Los nombres salen del
inspector integral de la bitácora (informe técnico RUS/SITFA, apéndice B). Flow solo lee y deja borradores.

Uso (desde la raíz): python tools/verificar_solo_lectura.py
"""

import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[1]
CARPETAS = [RAIZ / 'app' / 'src', RAIZ / 'descargador' / 'extension', RAIZ / 'descargador_lite']
SUFIJOS = {'.rs', '.js', '.html', '.json', '.ps1'}
PROHIBIDOS = {
    'control de creación o edición de observaciones de RUS': re.compile(
        r'COD_Estado_Detalle(?:_Nuevo)?|id_Obs_centro|GLS_ObsCentro(?:_Nueva)?|GLS_ObsTribunal|GLS_Observacion|'
        r'\bchkBitacora(?:Edit)?\b|\bTxt_Cont2\b|GLS_ObsNoEdit|GLS_ObsTribNoEdit'
    ),
    'envío de correo (solo se guardan borradores)': re.compile(r'\.Send\s*\('),
}


def sin_pruebas(texto: str) -> str:
    """El código de producción: lo que está antes del primer `#[cfg(test)]` de un archivo Rust."""
    return texto.split('#[cfg(test)]', 1)[0]


def buscar(texto: str) -> list[tuple[int, str, str]]:
    return [
        (texto.count('\n', 0, m.start()) + 1, motivo, m.group(0))
        for motivo, patron in PROHIBIDOS.items()
        for m in patron.finditer(texto)
    ]


def autoprueba() -> int:
    """Permanente: el verificador detecta cada control prohibido y el `.Send(`, e ignora los módulos de prueba."""
    for texto in ('let c = "chkBitacora";', 'x("COD_Estado_Detalle_Nuevo")', 'document.id_Obs_centro', 'mail.Send()', 'a.Send ('):
        assert buscar(texto), texto
    assert not buscar('let c = "chkBitacoraria"; // Save()') and not buscar(sin_pruebas('fn a() {}\n#[cfg(test)]\nconst X: &str = "chkBitacora";'))
    print('AUTOPRUEBA: PASA')
    return 0


def revisar() -> list[str]:
    hallazgos = []
    for carpeta in CARPETAS:
        for ruta in sorted(carpeta.rglob('*')):
            if not ruta.is_file() or ruta.suffix not in SUFIJOS or 'target' in ruta.parts:
                continue
            texto = ruta.read_text(encoding='utf-8', errors='replace')
            if ruta.suffix == '.rs':
                texto = sin_pruebas(texto)
            for linea, motivo, nombre in buscar(texto):
                hallazgos.append(f'{ruta.relative_to(RAIZ)}:{linea}: {motivo}: {nombre}')
    return hallazgos


def main() -> int:
    if '--autoprueba' in sys.argv:
        return autoprueba()
    hallazgos = revisar()
    for h in hallazgos:
        print(h)
    print(f'SOLO_LECTURA: {len(hallazgos)} hallazgos; RESULTADO: {"PASA" if not hallazgos else "FALLA"}')
    return 1 if hallazgos else 0


if __name__ == '__main__':
    sys.exit(main())
