# Evaluación del descargador integral y diseño de SITFA Lite

Fuente evaluada: rama `ultima-version/asistente-descargador` (commit `1f4e1fd`, 29-oct-2026), carpeta `descargador/` (versión 2.8.1).

## Hechos (N1)

- Tamaño: ~19.000 líneas en `descargador/` (Python + extensión), 104 archivos; GUI `descargador.py` de 1.772 líneas.
- Arquitectura: GUI Tkinter + puente HTTP local (`puente.py`, 127.0.0.1, código secreto) + extensión Chrome MV3 que ejecuta las consultas con la sesión del usuario (`pagina.js`, `conexion.js`).
- Pantallas soportadas: Seguimiento (menú 89), Búsqueda litigantes (94), Informes por vencer (91), Medidas por vencer (90), Carga de resoluciones (29, MAO).
- Funciones además de descargar: lectura de bitácoras (`lectura_bitacoras.py`, `diary_open`), historia de la causa (`lectura_historia.py`, `history_open`), calendario de informes por ingreso (`calendar_open`), descarga conjunta CSMP (`flujo_csmp.py`), paquete ZIP/XLSX consolidado (`flujo_paquete.py`, `export_csmp_capture.py`), reutilización de lecturas, favoritas, retención de lotes.
- Dependencias de la GUI con el monorrepo (`csmp_shared`, `nurus.ui_theme`, `nurus.capture.*`): no se ejecuta aislada.
- El motor (`motor.py`) valida cada página: formulario, tribunal/pestaña/modalidad, paginación, URL XLS, coincidencia exacta de registros HTML↔XLS, duplicados, relectura del archivo escrito. Es la parte más valiosa y estable.
- Evidencia sin registros: `evidence_open` → copia fija sin scripts → `captureVisibleTab` → PDF (`evidencias.py`).
- Nombres de archivo del integral (`lotes.file_base`): `CUMP Ambulatorio Jdo Familia X 2.xls` — no incluyen filtro de informe/medida, fechas, estado ni mes.

## Inferencias (N2)

- Supuesto: el uso diario mayoritario es «bajar los XLS de RUS». Para eso, la ventana del integral tiene ~25 controles que no aplican (paquete, bitácoras, historia, calendario, CSMP, recuperación), lo que agrega fricción y riesgo de error.
- Las funciones de bitácoras/historia abren ventanas emergentes por causa: son las que más tiempo y llamadas a SITFA consumen. Quitarlas reduce la duración del lote al número de páginas XLS.
- Reusar el motor sin cambios de lógica mantiene las garantías ya probadas del integral.

## Decisiones de SITFA Lite

| Tema | Integral 2.8.1 | Lite 1.0.0 |
| --- | --- | --- |
| Consultas | 5 pantallas + bitácoras/historia/calendario/paquetes | 4 pantallas: Seguimiento, Búsqueda Lit, Informes por vencer, Medidas por vencer |
| Carga (MAO) | Sí | No (no solicitada) |
| Comandos del puente | 13 | 10 (`catalog capabilities lock unlock prepare search download evidence_*`); el resto se rechaza en Python y en la extensión |
| Selección | Tribunales y modalidades múltiples; una pestaña/combinación y un filtro por lote | Casillas múltiples en tribunales, pestañas, modalidades, informes, medidas, estados y meses |
| Nombres | `CUMP Ambulatorio Tribunal 2.xls` | `Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Tribunal - fechas - p2 de 3.xls` |
| Sin registros | PDF `base.pdf` | PDF `… - SIN REGISTROS.pdf` |
| Extensión | 639 líneas `pagina.js`, versión 2.8.1 | 434 líneas, versión 1.0.0, tipo `SITFA_LITE_CAPACIDADES` |
| Dependencias de monorrepo | Sí | No (carpeta autónoma) |
| Entrega | Carpeta + `Iniciar.cmd` o instalador conjunto CSMP | `Instalar_SITFA_Lite_1.0.0.exe` (Inno Setup, sin admin) |

Se mantiene sin cambios de lógica: validaciones de `motor.py`, selector dinámico de vencimiento (`filtros_respuesta.py`), PDF de evidencia (`evidencias.py`), seguridad del puente (código de 128 bits, origen `chrome-extension://`, solo 127.0.0.1).

## Especulación (N3) / Riesgos abiertos

- La extensión se adaptó por recorte del integral y se probó con un SITFA ficticio y comprobación de sintaxis; **no se probó contra SITFA real** en esta entrega. Primera ejecución recomendada: un tribunal, una modalidad, una pestaña.
- Si SITFA cambia sus formularios, Lite falla igual que el integral (se detiene y lo informa; no descarga a ciegas).
- Políticas de Chrome institucionales pueden bloquear «Cargar descomprimida».
