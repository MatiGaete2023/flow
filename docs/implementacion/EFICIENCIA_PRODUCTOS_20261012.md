# Eficiencia de los productos (12 de octubre de 2026)

Complemento de [EFICIENCIA_PROCESO_20261012.md](EFICIENCIA_PROCESO_20261012.md). Mira la eficiencia del Descargador
SITFA, del Asistente CSMP y de lo que entregan al funcionario.

- **Base:** `feat` en `52ed0d4`.
- **Método:** dos revisiones de solo lectura. Los puntos clave se volvieron a comprobar en el código: archivo Excel
  idéntico, firmas sin uso, bitácoras paralelas, exportación manual y nóminas con `uuid4`.
- **Niveles:** **N1** = verificado en el código; **N2** = inferencia con el supuesto indicado.
- **Fuera de alcance:** `motor/`, `textos_base.json`, las pausas hacia SITFA, la regla «el Asistente no navega SITFA»
  y la regla «Outlook solo borradores».

## 1. Hallazgos

### 1.1 Descargador SITFA

| # | Impacto | Hallazgo | Evidencia | Nivel |
| --- | --- | --- | --- | --- |
| S1 | alto | La lectura de bitácoras vuelve a consultar en SITFA, página por página, los listados que el lote ya descargó como XLS | `lectura_bitacoras.py:332-424` (`enumerar_listado` → `transport.search`) | N1 |
| S2 | medio | Una lectura nueva vuelve a abrir todas las bitácoras aunque existan lotes previos; solo «Retomar» la misma carpeta las reutiliza | `lectura_bitacoras.py:213-225` | N1 |
| S3 | medio | La interfaz pide y valida «días para firmas» (60–730) aunque la consulta de firmas está retirada; además se crea siempre la hoja vacía «Resoluciones firmadas» | `descargador.py:195,498`; `flujo_csmp.py:150-155` | N1 |
| S4 | medio | Por lote se generan unos 10 archivos (XLS, JSON de resumen, verificación y flujo, HTML de bitácoras, `Bitacoras_RUS.xlsx`, XLSX «Flujo CSMP», ZIP); el usuario solo necesita el ZIP | `flujo_csmp.py:92-162`; `lectura_bitacoras.py:608` | N1 |
| S5 | medio | El manual presenta como camino normal la ventana Resultados (8 pasos), aunque existe el atajo «Crear paquete de este lote…» en la ventana principal | `MANUAL_DESCARGADOR_MVP_20261008.md:17-56`; `descargador.py:859-876` | N1 |
| S6 | bajo | Hay 300 lecturas como máximo por lote; si se supera, hay que dividir el trabajo y pulsar «Retomar» a mano | `lectura_bitacoras.py:195,542-546` | N1 |

### 1.2 Asistente CSMP

| # | Impacto | Hallazgo | Evidencia | Nivel |
| --- | --- | --- | --- | --- |
| A1 | alto | Tras importar el paquete el Excel no se exporta solo, y Correos y Resoluciones quedan bloqueados hasta pulsar «Exportar copia actual». «Procesar Excel» sí exporta automáticamente | `app_base.py:322,675`; `app.py:319` | N1 |
| A2 | alto | Guardar N borradores cuesta O(N × tamaño del trabajo) (ver detalle abajo) | `outputs.py:280-288,589,631,656-677`; `sync.py:211` | N1 |
| A3 | medio | Cada edición de un registro recalcula los hallazgos y hace `deepcopy` de todas las filas | `record_edits.py:145`; `capture_findings.py:47-55` | N1 |
| A4 | medio | Exportar vuelve a hashear y a parsear entero el archivo recién escrito; después `_show_work` recalcula selecciones y resumen sin que cambien los datos | `work.py:457`; `sync.py:82-84`; `app.py:388,483` | N1 |
| A5 | medio | `_save_session` guarda dos instantáneas completas cuando la sesión viene de una descarga, y se llama en más de 10 puntos | `app_base.py:298-301`; `download_transfer.py:112` | N1 |
| A6 | medio | Cada preparación de nómina crea `adjuntos/<uuid>/` y no borra las anteriores, así que quedan XLSX huérfanos | `roster.py:61` | N1 |
| A7 | medio | Para preparar correos hay dos botones con alcances que se solapan («Preparar» y «Preparar todos»), y el botón del Excel cambia de nombre según el estado | `app.py:145,191,379`; `work_view.py:132` | N1 (confusión: N2) |
| A8 | bajo | Hay productos secundarios que repiten datos del Excel de trabajo: ZIP de entrega, resumen estadístico, vencimientos ICS, «Recuperar resultado local legado» | `current_tools.py:138`; `results_view.py:82,88` | N2 (supuesto: sin telemetría de uso) |
| A9 | bajo | `inspect_batch` corre dos veces en «Procesar Excel» | `app.py:326`; `work.py:268` | N1 |

Detalle de A2. Por cada borrador, el Asistente:
- vuelve a hashear el Excel de salida;
- recalcula la huella de dependencias;
- hashea los adjuntos dos veces, aunque `create_drafts` ya los había hasheado;
- guarda la sesión completa 2 o 3 veces, y cada guardado reescribe el `.bak`.

### 1.3 Código y entrega compartidos

| # | Impacto | Hallazgo | Evidencia | Nivel |
| --- | --- | --- | --- | --- |
| E1 | alto | Hay dos programas PyInstaller, cada uno con su propio runtime de Python. El instalador solo incluye el Asistente; el Descargador y la extensión se copian sueltos, y la extensión se recarga a mano en Chrome con cada versión. Además hay 5 o más scripts `.bat`/`.cmd` de arranque | `tools/build_integral.ps1:39-51`; `installer/CSMP.iss:29-34` | N1 |
| E2 | medio | `descargador/excel_presentacion.py` y `src/nurus/personal/excel_presentation.py` son byte a byte idénticos | `cmp` sin diferencias | N1 |
| E3 | medio | `personal/bitacoras.py`, `bitacora_har.py` y `bitacora_lote.py` solo se usan en pruebas y herramientas, en paralelo a `lectura_bitacoras.py` + `nurus.bitacora_html` | `grep` de importaciones | N1 |
| E4 | medio | La selección de la función del motor y el manejo de incidencias están duplicados en `record_edits.py:179-240` y `work.py:290-330`, con riesgo de que se desalineen | — | N1 |
| E5 | bajo | `normalize` está copiada 3–4 veces y los formatos de fecha `%d/%m/%Y` aparecen 23 veces | `bitacora_html.py:11` = `rus/columns.py:19` | N1 |

## 2. Propuestas, por impacto y costo

Riesgo E3 según la decisión j del 12 de octubre de 2026: «Sí» = el cambio toca una zona riesgosa y requiere revisión
cruzada.

| Prioridad | Propuesta | Hallazgos | Riesgo E3 | Requiere decisión |
| --- | --- | --- | --- | --- |
| 1 | Exportar el Excel automáticamente al importar el paquete, como ya hace «Procesar Excel» | A1 | No | No |
| 2 | Al guardar el lote en Outlook: un hash por archivo, una huella por lote y un solo `save` al final | A2 | **Sí** (Outlook) | No |
| 3 | Evaluar los hallazgos solo de la fila editada y no reparsear la exportación propia | A3, A4, A9 | No | No |
| 4 | Retirar el campo «días para firmas» y la hoja vacía «Resoluciones firmadas»; el código de firmas sigue en el historial | S3 | No | **Sí**: la decisión pendiente desde WP-80 sobre «Firmas en `carga.py`» |
| 5 | Limpiar las nóminas: reutilizar la carpeta por programa o borrar las anteriores al preparar | A6 | **Sí** (borrado de datos) | No |
| 6 | Unificar en un solo paso la consulta y la creación del paquete, llevar los intermedios a una subcarpeta `respaldo/` y reescribir el manual con el atajo | S4, S5 | No | No |
| 7 | Tomar los vínculos de bitácora de los XLS ya descargados, sin repetir la búsqueda en SITFA, y reutilizar las lecturas de lotes previos con la misma clave | S1, S2 | **Sí** (extensión SITFA) | **Sí**: solo puede validarse contra SITFA real en tu PC |
| 8 | Un solo programa con dos modos (Asistente / Descargador) elegidos por el usuario, con el instalador incluyendo la extensión | E1 | No | **Sí**: confirmar que no hay requisito institucional de separarlos |
| 9 | Unificar el código duplicado: Excel compartido en `nurus`, evaluador común del motor sin tocar `motor/` (con paridad golden), retiro de las bitácoras paralelas | E2–E5 | Sí para E4 (camino de reglas) | No |
| 10 | Un solo botón «Preparar correos» con filtro, y retirar o mover a «Avanzado» los productos secundarios sin uso | A7, A8 | No | **Sí**: necesito saber cuáles usas |

N2 (supuesto: una sesión típica de 300 bitácoras y 30 borradores):
- La propuesta 7 ahorra una pasada completa de búsquedas en SITFA por lote.
- La propuesta 2 baja el guardado de borradores de unas 60–90 escrituras de sesión a 1.
- Las propuestas 1 y 6 quitan entre 3 y 5 pasos manuales por ciclo.

No hay medición de tiempos reales: no se pueden tomar sin SITFA ni Outlook.

## 3. Siguiente paso propuesto

- **WP-87 (sin decisión del usuario):** propuestas 1, 3 y 6, más E2, E3 y E5 de la propuesta 9. Integra con
  `verificar_wp.py` PASA.
- **WP-88 (riesgoso, con E3):** propuestas 2 y 5.
- **Esperan una decisión tuya:** propuestas 4, 7, 8 y 10.
