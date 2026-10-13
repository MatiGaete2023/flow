# Análisis consolidado — dos informes y el repositorio

Fuentes: informe de Claude (`INFORME_FACTIBILIDAD_CLAUDE_20261013.md`), informe de la otra IA («Historia de la
causa en el Descargador SITFA», corte `1a2dd0f`, 11 páginas), captura de la ventana entregada por el usuario (solo
rasgos estructurales) y lectura del código en `1a2dd0f`.

Convención: **N1** verificado en código/documento · **N2** inferencia con supuesto · **N3** sin datos.

## 1. Coinciden ambos informes

| Tema | Conclusión común |
|---|---|
| Factibilidad | Viable con la arquitectura local actual; condicionada a caracterizar la ventana real (H0). |
| Arquitectura | Chrome del usuario → extensión → puente `127.0.0.1` → Python → archivos locales. Sin nube, sin IA, sin cookies en Python. |
| Reutilización | Puente, `ChromeTransport`, patrón de `DiaryRunner`, contrato `historia_tramites`, exportador. |
| Identidad | Historia es de alcance **CAUSA**: `scope=CAUSA`, `ingreso_id` vacío; nunca atribuir trámites a un NNA/ingreso; nunca consultar por RIT/nombre. |
| Cobertura | Error/no leído/tabla desconocida ≠ «sin trámites». `VACIA_COMPROBADA` solo con vacío explícito reconocido. |
| PDF | Excluidos; enlaces de documento no se siguen. |
| Asistente | Sin cambios de reglas ni productos; vista de historia en el Asistente sería trabajo aparte. |
| Bloqueo | Sin estructura real no se programa el parser. |
| Espera (C-01) | El defecto abierto no impide diseñar; aconseja cerrarlo antes de una entrega conjunta. |

## 2. Diferencias y resolución

| Tema | Claude | Otra IA | Resolución en este plan |
|---|---|---|---|
| Ventana temporal | Ventana de 4 meses | No heredar en silencio los 4 meses; elegir y declarar | **Decisión H-e del usuario: 4 meses**, declarada en cada producto (criterio, desde, hasta). No es herencia silenciosa. |
| Producto primero | Excel + tabla en ZIP | Visor local primero; Excel complementario; ZIP opcional | **Excel primero** (cubre «ver», menos código); **ZIP sí** porque el usuario quiere análisis posterior (H-d). Visor: pendiente P5. |
| Numeración | WP-100 | Ojo: `tools/estado.py` solo acepta `WP-\d{2}` | **WP-52 a WP-56** (números libres, junto a WP-50/51 «Nuevas funciones»); no se toca `estado.py`. |
| Extractor H0 | Nuevo extractor | `extraer_estructura_har.py` oculta rutas nuevas (`RUTA_NO_RECONOCIDA`); `extraer_estructura_bitacora.py` no sirve | **Nuevo `extraer_estructura_historia.py`** (entregado en esta rama) que conserva rutas `.do/.jsp`, nombres de campos/funciones y esqueletos de funciones, y bloquea la salida si detecta un valor de entrada. |
| Runner | `HistoriaRunner` análogo | `HistoryRunner` independiente, sin duplicar supuestos de bitácoras | **Independiente** (`lectura_historia.py`), reutilizando el recorrido del listado mediante un gancho mínimo en `DiaryRunner` (ver 03, WP-53). |
| Esfuerzo | 5,5–8 días | 9–16 jornadas MVP con visor; 11–19 con ZIP | Sin visor y con escenario A/B: **8–12 jornadas**. Recalcular tras H0. |
| Vínculo de historia | `vinculos.extract` | El extractor actual exige `ShowObservaciones` y descarta filas solo con historia | **Nuevo `extract_historia`** en `vinculos.py` que no dependa de `ShowObservaciones`. |

## 3. Hechos nuevos del repositorio (N1) que fijan el diseño

1. La extensión se inyecta con `world:"MAIN"` (`descargador/extension/conexion.js:44-45`): el código corre en el
   mundo de la página, por lo que técnicamente podría usar funciones de SITFA (relevante para el escenario C).
2. `diary_open` valida la tupla contra el listado vigente (`run.diaryDocument`), que se **reemplaza en cada
   `search`** (`pagina.js:424,459`). La historia debe leerse **mientras su página del listado es la actual**.
3. `PROTOCOLS` se compara por igualdad exacta (`contrato_integral.py`): añadir `historia_lectura` exige cambiar
   extensión y app a la vez y subir versión (2.7.6 → 2.8.0).
4. Contrato: `historia_tramites` ya existe, **sin** columnas para folio, etapa, anexo ni `raw_json`
   (`contract.py:94-107`). El lector acepta columnas adicionales al final (`contract.py:578`:
   `fields[:len(TABLES[table])]`), pero el escritor no (`csv.DictWriter(fieldnames=TABLES[table])`, línea 267).
   → extensión aditiva del contrato en WP-55 (E3).
5. Una consulta con `remote_identity` exige los cuatro campos no vacíos, incluido `ingreso_id`
   (`coverage.py:61-66`). → las consultas de historia **no** llevan `remote_identity`; usan `capture_tables` y una
   clave informativa propia.
6. `source_system` de bitácoras es `'RUS'` (`export_csmp_capture.py:247`). Historia usa el mismo valor.
7. El Asistente indexa eventos solo `VERIFICADA` + `INGRESO` (`capture_input.py:119-122`): filas CAUSA viajan sin
   alterar el trabajo. `record_coverages` mostrará historia como `NO_CONSULTADA` por ingreso (limitación aceptada).
8. `SITFA_Descargador.spec` declara `hiddenimports`; un módulo nuevo de `nurus` debe añadirse ahí.

## 4. Hechos de la captura del usuario (N1, solo estructura)

- Ruta de la ventana: `/SITFAWEB/ConsultaDetalleAtPublicoAccion.do?TipoPopup=3&a1=<opaco>`. Mismo origen
  `familia.pjud.cl` → sin permisos nuevos en el manifiesto.
- `a1` es opaco: **la URL no se arma con tribunal + causa** como la de bitácoras.
- Pestañas: Historia, Notificación, Diligencias, Escritos; botones Litigantes, Materias, Plazos, Remisión.
  Solo **Historia** entra al alcance.
- Columnas: Folio · Doc. · Sel · Anexo · Etapa · Trámite · Desc. Trámite · Referencia · Fecha Tram.
- Orden visible por fecha descendente; dentro de una fecha los folios pueden invertirse; hay folios ausentes
  entre filas mostradas → la continuidad de folios no acredita completitud.
- «Sin referencia.» es un valor literal.
- Encabezado con marca de reserva visible; puede haber causas con acceso restringido (N3).

## 5. Escenarios de acceso a `a1` (lo decide H0)

| Esc. | Cómo se obtiene la ventana | Implementación | Riesgo |
|---|---|---|---|
| A | `a1` ya viene escrito en el listado (atributo/enlace) | `history_open` toma la URL del listado vigente, valida ruta y la pide con GET | Bajo |
| B | Una petición intermedia del mismo origen devuelve `a1` (redirección o HTML) | Dos pasos en lista blanca: intermedia → ventana; `redirect:"manual"` controlado | Medio |
| C | `a1` lo calcula una función JS de la página | Llamar la función de SITFA en el mundo MAIN capturando la URL sin abrir ventana | Medio-alto; **decisión P7 + E3** |
| D | Solo con clic real/ventana con estado | Fuera de esta arquitectura | **Detener y reportar** |
