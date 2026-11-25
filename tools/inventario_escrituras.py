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
    'descargador/extension.rs': (3, 'perfil', 'archivos de la extensión Lite/Integral en LOCALAPPDATA/FlowRust/extension-<edición> (carpeta fija, actualizada en el lugar); solo escribe lo que cambió y quita los archivos que ya no son de la extensión'),
    'actualizacion.rs': (10, 'perfil', 'E-I2: reemplazo del propio flow_rust.exe por el que la persona dejó en LOCALAPPDATA/FlowRust/actualizaciones (solo con su botón, tras verificar la huella SHA-256 y guardar la sesión); deja flow_rust.exe.anterior y restaura si algo falla; la actualización aplicada pasa a actualizaciones/aplicadas'),
    'descargador/bitacoras.rs': (8, 'usuario', 'lote de bitácoras (E-Q4): subcarpeta nueva «Bitacoras <fecha>» en el destino elegido (create_dir falla si existe); bitacoras.json y diagnostico_calendarios.json (anónimo, extensión de Rust, mismo patrón temporal+rename) y diagnostico_bitacoras.json (anónimo) por temporal + rename; instantáneas con guardar_exclusivo (nunca sobrescribe)'),
    'descargador/historia.rs': (9, 'usuario', 'historia del lote de bitácoras (E-Q4c): carpeta historia/ del lote; historias.json y diagnostico_historia.json por temporal + rename; estructura_escritos.json y estructura_pestanas.json (E-X2) anónimas, esta por temporal + rename; copia de ventanas íntegras de un lote anterior (solo si el destino no existe); ventanas con guardar_exclusivo'),
    'descargador/historico.rs': (4, 'usuario', 'histórico del lote de bitácoras (E-Q4c): historico.json y estructura_historico.json anónima por temporal + rename dentro de la carpeta del lote; respaldo historico/historico-*.html con guardar_exclusivo'),
    'descargador/excel.rs': (1, 'usuario', 'Excel del lote de bitácoras y de Resultados (E-Q5d/E-S1): Bitacoras_RUS.xlsx e Historia_SITFA.xlsx en la carpeta del lote (nombre con sufijo si existe) o en el destino elegido; crea la carpeta y guarda con guardar_exclusivo (nunca sobrescribe)'),
    'descargador/resultados.rs': (3, 'usuario/temporal', 'productos de Resultados (E-S1): carpeta del lote o la elegida; XLSX con guardar_exclusivo y nombre con marca de microsegundos (publish); metadato JSON al lado del XLSX recién creado; el libro «Para CSMP» del paquete se arma en una carpeta temporal propia que se borra'),
    'descargador/paquete.rs': (2, 'usuario', 'ZIP CSMP_CAPTURE validado y diagnóstico de cobertura anónimo al lado, en el destino elegido, ambos con guardar_exclusivo; si falla el ZIP se retira solo el diagnóstico que se acaba de crear'),
    'descargador/flujo_paquete.rs': (9, 'usuario', 'E-Q9 «Consultar y crear paquete»: carpeta nueva «Paquete CSMP <fecha y hora>» en el destino; respaldo/proceso.json por temporal + rename en cada cambio de fase y al cerrar; el ZIP se arma en una carpeta temporal dentro de respaldo y se renombra al final; si ya había un ZIP se copia a respaldo como «Captura_CSMP anterior …»; nunca sobrescribe el paquete anterior'),
    'descargador/lote.rs': (7, 'usuario', 'carpeta de destino del lote y subcarpeta nueva del lote (create_dir falla si existe); guardar_exclusivo nunca sobrescribe; resumen por temporal + rename; solo borra su propio archivo si la verificación falla'),
    'ui.rs': (5, 'usuario/perfil', 'carpeta de productos de Ajustes; contactos.csv por diálogo Guardar; plantillas, datos y carpeta de actualizaciones (solo crear carpeta) en data_dir() y LOCALAPPDATA/FlowRust'),
    'anon.rs': (1, 'usuario', 'carpeta de salida del anonimizador (solo crear carpeta); las copias las escribe flow-anonimo en una carpeta Anonimizados_* nueva, tras verificarlas'),
    'autodiagnostico.rs': (6, 'temporal', 'carpeta temporal propia flow-autodiagnostico-<pid> (se borra al terminar) e informe --self-test con create_new: nunca reemplaza un archivo'),
    'copia_excel.rs': (7, 'usuario/temporal', 'copia del Excel en la carpeta de productos; ~flow-* y temp_dir() se limpian; rechaza destino existente'),
    'personalizar.rs': (4, 'perfil', 'matrices Word importadas a data_dir()/plantillas (el original se respalda como .bak.docx; escritura por archivo temporal)'),
    'producto_excel.rs': (1, 'usuario', 'carpeta de productos de Ajustes; el libro se guarda en una ruta nueva (nueva_ruta nunca reemplaza un archivo)'),
    'estadisticas.rs': (2, 'usuario/perfil', 'resumen de constancias en la carpeta de productos con nombre nuevo; metricas.json solo con la opción activada, vía config::atomic_write'),
    'manual.rs': (3, 'usuario', 'Word manual en la carpeta de productos con un nombre nuevo (no reemplaza; archivo temporal y renombrado); los favoritos y lo escrito van por config::atomic_write'),
    'outlook.rs': (5, 'temporal', 'borrador.json y la nómina en una carpeta temporal propia que se borra al terminar; Outlook solo recibe Save()'),
    'productos.rs': (6, 'usuario/temporal', 'EML/XLSX/resoluciones en la carpeta de productos (nombres nuevos, carpeta Resoluciones_* nueva); borrador Outlook vía temp_dir() que se limpia'),
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
