"""E-G · G6: inventario de escrituras a disco del código de producción de `app/src`.

Cuenta por archivo las llamadas que crean, escriben, renombran o borran archivos fuera de los módulos `#[cfg(test)]`
y las compara con la clasificación auditada (CLASIFICACION). Una escritura nueva, o una que desaparece, hace fallar el
script (código 1) hasta que se clasifique aquí: destino de perfil local, temporal o elegido por el usuario.

Uso (desde la raíz): python tools/inventario_escrituras.py
"""

import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[1]
SRC = RAIZ / 'app' / 'src'
PATRON = re.compile(
    r'fs::write|File::create|OpenOptions::new|fs::rename|fs::copy|create_dir_all|fs::create_dir\(|remove_file|'
    r'remove_dir_all|fs::remove_dir\(|\bwb\.save\('
)

# archivo: (cantidad, clase, destino). Auditado el 23-nov-2026.
CLASIFICACION = {
    'config.rs': (3, 'perfil', 'config::atomic_write: config.json y sesiones en data_dir() (APPDATA/FlowRust); temporal + rename'),
    'trabajo.rs': (1, 'perfil', 'limpieza de sesiones antiguas propias en data_dir()/sesiones (RETENCION_DIAS, SESIONES_ANTERIORES)'),
    'descargador/preferencias.rs': (3, 'perfil', 'descargador_preferencias.json en LOCALAPPDATA/FlowRust; temporal + rename'),
    'descargador/extension.rs': (2, 'perfil', 'archivos de la extensión Lite/Integral en LOCALAPPDATA/FlowRust/extension-*'),
    'descargador/lote.rs': (7, 'usuario', 'carpeta de destino del lote y subcarpeta nueva del lote (create_dir falla si existe); guardar_exclusivo nunca sobrescribe; resumen por temporal + rename; solo borra su propio archivo si la verificación falla'),
    'ui.rs': (4, 'usuario/perfil', 'carpeta de productos de Ajustes; contactos.csv por diálogo Guardar; plantillas y datos en data_dir() (solo crear carpeta)'),
    'anon.rs': (3, 'usuario', 'carpeta de salida del anonimizador; nombres nuevos con productos::nueva_ruta'),
    'copia_excel.rs': (7, 'usuario/temporal', 'copia del Excel en la carpeta de productos; ~flow-* y temp_dir() se limpian; rechaza destino existente'),
    'productos.rs': (9, 'usuario/temporal', 'EML/XLSX/resoluciones en la carpeta de productos (nombres nuevos, carpeta Resoluciones_* nueva); borrador Outlook vía temp_dir() que se limpia'),
    'revision_ui.rs': (5, 'usuario', 'carpeta nueva de la revisión de bitácoras (create_dir falla si existe); ante error borra solo lo propio'),
}


def produccion(texto):
    """Líneas fuera del primer módulo `#[cfg(test)] mod …` del archivo."""
    lineas = texto.split('\n')
    for i, linea in enumerate(lineas[:-1]):
        if re.match(r'\s*#\[cfg\(test\)\]', linea) and re.match(r'\s*(pub(\(crate\))?\s+)?mod\s', lineas[i + 1]):
            return lineas[:i]
    return lineas


def main():
    halladas = {}
    detalle = {}
    for ruta in sorted(SRC.rglob('*.rs')):
        rel = ruta.relative_to(SRC).as_posix()
        if rel == 'prueba_entorno.rs':
            continue
        for n, linea in enumerate(produccion(ruta.read_text(encoding='utf-8')), 1):
            if linea.strip().startswith('//') or linea.strip().startswith('use '):
                continue
            if PATRON.search(linea):
                halladas[rel] = halladas.get(rel, 0) + 1
                detalle.setdefault(rel, []).append(f'{n}: {linea.strip()[:110]}')
    errores = []
    for rel in sorted(set(halladas) | set(CLASIFICACION)):
        esperado = CLASIFICACION.get(rel, (0, '', ''))[0]
        if halladas.get(rel, 0) != esperado:
            errores.append(f'{rel}: halladas {halladas.get(rel, 0)}, clasificadas {esperado}')
            for d in detalle.get(rel, []):
                errores.append(f'    {d}')
    for rel, (n, clase, destino) in sorted(CLASIFICACION.items()):
        print(f'{rel}: {halladas.get(rel, 0)}/{n} [{clase}] {destino}')
    for e in errores:
        print('DISCREPANCIA', e)
    print(f'ESCRITURAS: {sum(halladas.values())} en {len(halladas)} archivos; '
          f'RESULTADO: {"FALLA" if errores else "PASA"} ({len(errores)} discrepancias)')
    return 1 if errores else 0


if __name__ == '__main__':
    sys.exit(main())
