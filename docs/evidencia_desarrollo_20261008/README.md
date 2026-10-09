# Evidencia técnica local y productos ficticios

Entrega del MVP autorizada por «continua con el plan hasta completar todas las
tareas y productos». El usuario confirmó después que no hay entorno/evidencia
institucional y que SITFA/RUS se usarán después. Esa etapa queda diferida.
Historia/PDF por trámite y reglas CT de producción están excluidas del MVP.

## Continuación: productos desde el ejecutable y actas

- `flow-continuacion-asistente.txt/xml`: 576 aprobadas, 6 omitidas Linux.
- `flow-continuacion-descargador.txt/xml`: 169 aprobadas y 57 subtests.
- `flow-continuacion-golden.txt`: paridad de los 17 productos baseline.
- `verificacion_productos_continuacion.json`: prueba portable del nuevo
  `nurus.personal.capture_selfcheck` y hashes de productos ficticios temporales.

La prueba de productos puede reproducirse con
`python tools/flow_verify_capture_products.py /ruta/nueva`; la misma función se
ejecuta dentro del EXE y el instalador, con prueba Windows registrada por SHA.
Los hashes XLSX/DOCX cambian por metadatos temporales; no son golden semánticos.
`flow_acceptance.py` crea plantillas y valida estructura/coherencia declarada,
sin aceptar gates. Las pruebas del validador usan actas ficticias; no se presentan
como aceptación institucional. No se consultó SITFA/RUS ni se guardó Outlook.

La corrección UTF-8 del CLI está en `flow-continuacion-utf8.txt/xml` y en el
registro `ci_windows_1e50f95_fallo.json`. En `9cd9f50` los tests Windows,
EXE autónomo y paquete conjunto pasaron, pero la comprobación del instalador
quedó bloqueada: se solicitó cancelación, sin declararla aprobada. Se introduce
verificación de procesos con plazo y logs por etapa; su nueva CI debe cerrarse.

## Verificación reproducible

Python 3.12.14 en Linux con Tk/pantalla virtual DISPLAY=:99. Dependencias de
`.[dev,excel-legacy]` y Descargador instaladas. Office/COM no disponible aquí.

```sh
python -m pytest -q --junitxml=resultado_asistente.xml
python -m pytest descargador -q --junitxml=resultado_descargador.xml
python -m compileall -q src tests tools descargador
python tools/flow_plan_progress.py
python tools/flow_baseline_golden.py --output /ruta/nueva/golden --expected docs/evidencia_p0/golden/semantic.json
python tools/flow_capture_pipeline.py /ruta/nueva/productos --rows 4
python tools/flow_capture_pipeline.py /ruta/nueva/benchmark --rows 100 500 2000 --complete
```

Los TXT/XML se conservan sin reformatear. La corrida `flow-mvp-delivery` incluye
543 pruebas aprobadas y 6 omitidas; la última `flow-mvp-final` alcanza 545 aprobadas
y 6 omitidas, añadiendo concatenación desde lote original sin editar el ZIP. Descargador:
169 aprobadas y 57 subtests. Las omisiones y entornos específicos se detallan
en JUnit; no equivalen a aceptación institucional.

La última corrida `flow-mvp-final-build` alcanza **549 aprobadas y 6 omitidas**;
incluye rechazo de metadata de build faltante/incorrecta y exposición del SHA.
`flow-mvp-downloader-final-build` conserva **169 aprobadas y 57 subtests** con
la prueba Tk en proceso separado. Estos son los resultados locales vigentes.

El cierre `flow-mvp-final-rules` alcanza **552 aprobadas y 6 omitidas**. Calendarios
de medida/tipo desconocido no activan reglas ni cruce de Informes; un horario
sin offset no acredita orden aunque tenga una etiqueta de zona. Reimportar
compara versiones y actualizar el motor invalida productos preparados sin tocar
la edición humana. Preview temporal v2 continúa desactivado en producción.
`flow-golden-final-rules.txt` vuelve a confirmar paridad de los 17 productos.
Estos son los resultados finales después de los últimos ajustes funcionales.

`flow-mvp-final-preserved` conserva **552 aprobadas y 6 omitidas**, incluyendo
exclusiones al reanalizar y edición de registros OTRO sin introducir propuestas.
El subconjunto `flow-mvp-human-decisions` registra 43 aprobadas. La intervención
humana sigue prevaleciendo sobre resultados automáticos y cambios de versión.

Golden actual: 17 productos ficticios, paridad de contenidos, observaciones,
reglas y estilos contra P0. El inventario P0 sigue detectando los cambios de
implementación autorizados; no se reemplazan sus hashes de origen.

`matriz_actual.csv/json` conserva los 115 requisitos, estado de partida,
implementación, resultados locales acotados, rutas y aceptación institucional
NO EJECUTADO. `tareas_plan.json` registra los 79 entregables/tareas de P0–P13.
La [matriz legible](../TRAZABILIDAD_EJECUCION_20261008.md) explica las dependencias.

## Productos revisables

`productos_ficticios/4/` contiene exclusivamente personas e IDs ficticios:

- Captura_CSMP.zip parcial con bitácora no consultada declarada.
- Observaciones.xlsx y Cobertura.xlsx.
- Bitacoras_RUS.xlsx autónomo, generado con otra fixture de lecturas controladas.
- Proyectos_revisables.docx, nóminas en adjuntos y pendientes de proyectos.
- Borradores.json: descriptores con destinatarios `.test`, sin Outlook ni envío.
- solicitud_antecedentes.json y sesión recuperable.

Las plantillas y los productos son demostración técnica para revisión humana.
El generador no valida jurídicamente proyectos ni guarda borradores en Outlook.
Las fixtures completa/parcial de contrato se encuentran en `contracts/`.
`hashes_productos.json` permite comprobar estos archivos sin abrir sus contenidos.

## Rendimiento y límites

Benchmark con fuentes completas ficticias; segundos de cada etapa y memoria
Python (`tracemalloc`) en `benchmark/{100,500,2000}/benchmark.json`.

| Registros | Validar ZIP | Importar/normalizar/analizar | Excel | Pico memoria Python |
|---|---:|---:|---:|---:|
| 100 | 0,035 | 0,484 | 0,682 | 55,1 MiB |
| 500 | 0,120 | 1,696 | 2,573 | 23,3 MiB |
| 2000 | 0,434 | 8,278 | 9,312 | 91,0 MiB |

100 filas incluye además generación Word/nóminas; los demás tamaños miden
Excel y descriptores. Los picos no son directamente comparables ni memoria RSS.
La interfaz institucional y el hardware del operador no fueron medidos.
Los límites ZIP/CSV son conservadores y explícitos en el contrato.

## CI y discrepancias

**CI final 01e013d: SUCCESS en ambos workflows**, 2026-10-08 22:38 UTC.
Asistente: Windows Python 3.12/3.13/3.14 aprobados; 3.12 registró 557 aprobadas
y 1 omitida. Descargador: 169 aprobadas y 57 subtests. Comprobación de contrato,
ejecutables, paquete conjunto, instalación/reinstalación/desinstalación aprobadas.
Los dos EXE del paquete conjunto acreditaron el mismo commit. Las descargas
están en [CONTINUAR](../../CONTINUAR.md) y el JSON contiene IDs/tamaños/URLs.
Los checkpoints posteriores de evidencia no cambian ese código compilado.

Los runs y artefactos Windows se registran en `ci_windows.json`; usar siempre el
SHA indicado. CI demuestra compilación y pruebas ficticias, no SITFA/RUS real,
Windows 10 institucional, Excel 2010 ni Outlook 365 instalado.

El primer run Windows del MVP detectó dos pruebas con codificación implícita
cp1252 (texto emoji y lectura JSON UTF-8). Se corrigieron con `encoding='utf-8'`
en pruebas y generador; no se omitieron los tests. El run histórico anterior
falló al inicializar Tcl tras empaquetado; se conserva como antecedente, no PASÓ.
La corrida Linux inicial detectó una aserción dependiente de imports previos;
se corrigió midiendo los nuevos imports del acto probado.

El fallo Tcl se reprodujo en el run Descargador de d261db5 (job 113562801344).
La prueba afectada crea ahora una ventana real en un proceso separado, manteniendo
las tres aserciones y fallando si el proceso no arranca. No se omite ni se declara
resuelto en Windows por pasar Linux: comprobar el run posterior a esta corrección.

La comprobación del paquete CSMP ahora incluye commit/versión y rechaza un build
con origen ausente o incorrecto. El paquete conjunto rechaza SHA diferentes entre
ambos ejecutables. Los workflows exigen también la verificación del contrato v1.
El wheel instalado fuera del árbol fuente confirmó contrato, catálogo, cinco
patches y seis plantillas; fue necesario usar el entorno de build aislado de pip
porque el venv local no tenía setuptools.

La nueva prueba de concatenación importa el productor del Descargador. El run
Windows 073713d detectó `reportlab` ausente en el entorno de tests del Asistente
(la suite local tenía ambas dependencias instaladas). Se declararon reportlab e
icalendar como dependencias **dev** para reproducir pruebas integrales; no se
añaden al runtime obligatorio del Asistente ni se omite la prueba.

El descriptor de borrador de tribunal de la fixture conserva una glosa heredada
que afirma registro en bitácora. Es material para revisión humana y debe corregirse:
la API de creación de borradores para paquetes rechaza esa afirmación no acreditada.
No se guardó ese descriptor en Outlook ni se cambió la plantilla del usuario.

No hubo consultas institucionales, registros RUS ni correos enviados. La API
distribuida de registro rechaza antes de acceder al adaptador. Las pruebas del
protocolo histórico usan una simulación residente exclusivamente en tests.
