# Comparar la versión actual y la experimental

## Ramas y configuración

- Original Download: `main`, versión 2.2.0.
- Revisión técnica/documental anterior: `mejoras/revision-simplicidad`.
- Nueva Download: **`experimento/experiencia-csmp-20261001`**.
- Original NuRus: `main`, candidata dev11.
- Nueva NuRus: **`experimento/integracion-sitfa-20261001`**.

Ninguna rama original se sustituye ni se fusiona automáticamente. Descarga cada
rama en una carpeta diferente; sus entornos Python también son independientes.
La herramienta nueva muestra «Experimental» en su título. Download experimental
guarda favoritos/configuración en `LOCALAPPDATA/SITFA_Descargador_Experimental`.
NuRus experimental usa `LOCALAPPDATA/CSMP_Personal_Experimental`; no reutiliza
sesiones ni sobrescribe parámetros de `CSMP_Personal`. Para comparar con tus
ajustes personales puedes copiar voluntariamente tu configuración a la carpeta
experimental antes de abrirla; conserva el original.

Si cargas ambas extensiones Chrome desde carpetas diferentes, conecta cada una
con el descargador de su misma versión. Una extensión queda vinculada al puente
por su identidad y código temporal. La nueva mantiene los permisos anteriores.
La extensión nueva se identifica como «experimental», versión 2.3.1; el flujo
básico 2.2.0 del README corresponde a la versión original.

## Cambios implementados

| Propuesta | Implementación en la rama nueva |
|---|---|
| Consolidar | Resultados → Consolidar XLSX. Una hoja por familia/esquema, resumen, filtros, cabecera fija, procedencia y vencimientos ordenados. Conserva multiplicidad y todos los originales. |
| Integración CSMP | Resultados → Exportar para CSMP. XLSX real con modos Espera/Cumplimiento/Informes; campos comprobados, metadatos `SITFA_*` y mapa de archivos. Rechaza pantallas/esquemas no aptos. |
| Cruce C-10 | Resultados → Añadir informes al cruce. Lotes completos, mismo tribunal/fechas y modos Cumplimiento/Informes. NuRus mantiene el cruce por identidad completa y su regla de última fila futura. |
| Resumen legible | Resultados muestra consultas, estado, registros y compatibilidad CSMP. Doble clic abre el primer archivo de la consulta. |
| Favoritos | Más opciones → Guardar/Elegir favorita. No guarda fechas, RUT, filtros privados ni autenticación; valida IDs contra el catálogo actual. |
| Tribunales reutilizables | Se conserva la selección y se cruza con opciones disponibles. El selector busca ignorando tildes y mantiene las casillas ocultas. «Marcar visibles» se diferencia de «Marcar todos». |
| Fechas rápidas | Este mes, mes anterior y últimos 30 días. Calendarios usan mes/año; otras pantallas activan las fechas visibles. |
| Progreso y aviso | Contexto de consulta, página, archivos y registros; sonido opcional. No inventa un porcentaje global ni estima tiempos sin datos. |
| Historial | Resultados identifica carpetas de lote, completos/vacíos/incompletos; también permite abrir una carpeta externa. |
| Comparar lotes | Resultados → Comparar: filas nuevas/ausentes con multiplicidad; exige alcance estructurado equivalente y rechaza filtros privados no verificables. Ausente no significa causa egresada. |
| Vencimientos/ICS | Hoja Vencimientos ordenada y exportación ICS con eventos de día completo, identificador estable y descripción sin nombres/RUT. Fechas no interpretables se indican; no se recalculan plazos. |
| Pendientes | Resultados → Preparar pendientes. Ejecuta solo las consultas faltantes desde página 1 en un lote nuevo; no reanuda páginas antiguas. Usa el plan estructurado y no recupera filtros privados. |
| ZIP de entrega | Incluye solo originales verificados, resumen, verificaciones y un índice. Un lote parcial se rotula INCOMPLETO. No envía nada. |
| CSV / Power Query / Power BI | CSV UTF-8/BOM por esquema, separador `;`, contenido neutralizado para Excel y procedencia. XLSX es la salida preferida para identificadores y varias hojas. |
| Diagnóstico mínimo | Mensajes por etapa `E_*`; no se guardan cuerpos, RUT ni valores privados en un log de diagnóstico. |
| Límites de recepción/lectura | Chrome corta la recepción al superar 80 MB. Python limita dimensiones/celdas y expansión OOXML; la exportación local limita 200.000 filas. |
| Instalación | Comprobación Python/Tkinter y huella SHA-256 de requirements; el marcador solo se actualiza tras instalar y comprobar dependencias. |
| Ejecutables Windows | Recetas PyInstaller para ambos repositorios y trabajos CI que construyen distribuciones Windows. La extensión sigue instalándose personalmente; Office sigue necesario para la ruta institucional de NuRus. |
| NuRus: comprobar columnas | Botón de compatibilidad, avisos de filas incompletas/descarga antigua y protección antes de PROCESAR si faltan columnas funcionales. |
| NuRus: formato real | Reconoce BIFF/OOXML por firma; un HTML/XML bajo `.xls` explica cómo convertirlo desde Download. |
| NuRus: procedencia | Abrir original SITFA comprueba ruta, tipo y hash. La relación se conserva al recuperar sesión; un archivo movido o alterado da aviso. |
| NuRus: precarga | `--archivo`, `--modo`, `--hoja` precargan controles. No analizan, envían ni guardan productos automáticamente. |
| Pruebas | Regresiones de resultados/UI, fechas, duplicados, fórmulas, rutas, originales alterados, persistencia y una prueba offline entre los dos repositorios. CI Windows conserva las pruebas propias y construye los ejecutables. |

## Ruta de prueba recomendada

La revisión 2.3.1 reconoce cabeceras en TD y mensajes explícitos de tablas vacías,
para generar su PDF y continuar el lote. La detección de páginas repetidas compara
las filas completas: compartir RIT/nombre con otra medida o programa no basta
para rechazar una página. El PDF contiene únicamente la captura, sin las leyendas
que antes añadían Chrome y el generador PDF. Actualiza tanto la extensión como
el código o ejecutable; recarga la extensión en `chrome://extensions` y reconecta.

1. Instala la rama nueva de Download en otra carpeta con `Iniciar.cmd`, recarga
   la extensión de esa carpeta y realiza un lote pequeño como acostumbras.
2. Abre Resultados: comprueba resumen y archivos originales. Consolida y compara
   cantidades; el libro derivado no sustituye ni elimina las descargas.
3. Exporta para CSMP. En Más opciones configura `csmp-assistant.exe`,
   `CSMP_Experimental.exe` o el `pythonw.exe` del entorno de la rama nueva de
   NuRus. También puedes abrir el libro manualmente.
4. En NuRus revisa modo/hoja, Comprobar columnas y PROCESAR. Revisa observaciones,
   borradores y proyectos con el flujo habitual; los originales permanecen sin
   cambios. «Abrir original SITFA» comprueba la procedencia de una fila elegida.
5. Para el cruce, selecciona un lote de Cumplimiento y otro de Informes con el
   mismo alcance de tribunales y fechas. C-10 requiere RIT/RUT/nombre/tribunal/
   programa y vencimiento futuro; la falta de cruce sigue siendo advertencia.
6. Para comparar viejo/nuevo, carga los mismos datos en instalaciones separadas.
   No ejecutes dos descargas simultáneas con la misma cuenta: SITFA reutiliza el
   Excel temporal. La compatibilidad de derivados nuevos no se atribuye a NuRus
   original.

Los lotes antiguos se pueden consolidar con la etiqueta y el origen disponibles,
pero no inventan metadatos que no guardaron. La exportación CSMP, comparación y
preparación de pendientes necesitan el contrato estructurado de los lotes nuevos.

## Power Query, Power BI y carpetas compartidas

En Excel: Datos → Obtener datos → Desde archivo → Desde libro; selecciona la hoja
del consolidado. En Power BI Desktop: Obtener datos → Excel; selecciona el mismo
libro. Para CSV elige UTF-8 y delimitador `;`; define identificadores como texto.
Para un informe del estado actual, importa un lote elegido, no todos los lotes
históricos automáticamente. Si haces un histórico, conserva `SITFA_LOTE` y
no confundas registros repetidos entre fechas con registros nuevos.

«Cambiar carpeta» admite una carpeta de trabajo compartida o sincronizada. Las
escrituras se verifican localmente; la sincronización y los accesos los administra
OneDrive o el servicio de carpetas, no el descargador. No se publica un tablero
ni se envían archivos a la nube automáticamente.

ICS genera un archivo local para importación voluntaria. No importa nombres/RUT
al calendario por defecto. El comportamiento de duplicados/actualizaciones
depende del calendario; no representa una notificación judicial oficial.

## Distribución y comprobación

Los workflows Windows generan los artefactos `SITFA-experimental-Windows` y
`CSMP-experimental-Windows`; contienen la carpeta completa del ejecutable,
incluidos sus recursos. No copies solo el `.exe`. La ejecución CI y su artefacto
deben terminar correctamente antes de considerar verificada esa distribución.
El entorno Linux de esta revisión no construye ejecutables Windows.

Las verificaciones offline entre repositorios se ejecutan con un entorno de
desarrollo que tenga instaladas las dependencias de ambos (no es necesario
mezclar sus entornos de uso diario):

```powershell
# Dentro de NuRus experimental, entorno de pruebas
py -3.12 -m venv .venv
.venv\Scripts\python.exe -m pip install -e ".[dev,excel-legacy]"
.venv\Scripts\python.exe -m pip install -r ..\Download\requirements.txt
.venv\Scripts\python.exe ..\Download\pruebas\integracion_exportador_csmp.py .
```

La sonda usa solo Excel ficticios, exportación portable y configuración temporal;
no accede a SITFA ni a Office. Las pruebas institucionales con Excel/Outlook y
tablas representativas siguen siendo necesarias para evaluar la experiencia real.

Verificación local de esta implementación (2.3.1): 99 pruebas de Download aprobadas;
266 pruebas de NuRus aprobadas y una omitida por plataforma; interfaz Tkinter y
CustomTkinter en pantalla virtual; integraciones de extensión real con datos
ficticios (cuatro pantallas, 19 Excel y cinco PDF); exportador CSMP → análisis →
revisable → recuperación de procedencia y cruce C-10 aprobado. Python 3.12/Linux.

## Límites concretos

No hay API documentada de otros sistemas de gestión, datos de prueba equivalentes
ni credenciales/configuración de publicación Power BI Service. Por eso se
implementan intercambios mediante archivos y NuRus, y no se inventan conectores
de escritura. Litigantes y calendario de medidas siguen disponibles para
consolidación/evidencia, pero no se convierten a modos CSMP inexistentes.
No se añaden matrices judiciales faltantes ni reglas para tribunales desconocidos.

El formato CSV no conserva todos los tipos; XLSX es preferible. Cabeceras ambiguas
o filas desconocidas detienen la transformación. Una exportación parcial requiere
la casilla explícita y queda INCOMPLETA; CSMP requiere lotes completos.
Los metadatos y el mapa de rutas son locales: si se mueve la carpeta de origen,
NuRus avisa en lugar de abrir otro archivo por parecido de nombre.
