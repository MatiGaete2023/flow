# Análisis e incorporación de la especificación 02

**Registro histórico anterior al desarrollo:** avance vigente en [IMPLEMENTACION_MVP_20261008.md](IMPLEMENTACION_MVP_20261008.md).

Documento recibido: `02_ESPECIFICACION_TECNICA_ASISTENTE_CSMP.md`, SHA-256
`f22ae66bc25edcceeb8416eed54e423a487e0e1d556911995cd823c7e4ad3f55`.
Se conserva íntegro en `docs/plan_20261008/`. La nueva copia del documento 01
tiene el mismo hash que la anterior; se conserva como `(1)` y no sustituye fuentes.

## Conclusión y cambios en esta entrega

El documento 02 completa el diseño del consumidor local y sus 20 casos A-T.
Se actualiza la matriz de **95 a 115 requisitos** con rutas/pruebas verificadas;
la herramienta ahora exige cuatro fuentes canónicas y excluye copias idénticas.
Se ajustan decisiones P2, inventario de módulos, migración y `CONTINUAR.md`.
Se elimina la antigua dependencia «falta recibir 02». G0 sigue pendiente de
aceptación; recibir un documento no equivale a aprobar la matriz ni a implementar
el contrato. Esta actualización no modifica reglas, GUI ni captura institucional.

## Diferencias de versión y correspondencia real

El documento describe la rama experimental `fba9ffea…`, Asistente `0.4.0.dev23`.
El plan maestro conserva como baseline `251d5c4…`, Asistente `0.5.0.dev4` y
Descargador/extensión `2.7.2`. Por tanto, sus requisitos rigen el diseño deseado,
pero las afirmaciones sobre archivos se contrastan con este último SHA.

| Referencia en 02 | Evidencia del baseline actual | Cambio pertinente |
|---|---|---|
| `review_store.py`/`rus_activity.py` almacenan marcas locales | Esos archivos no existen en `src/nurus/personal`; `test_current_release_contract.py` controla ausencia de `review_store` | No retirarlos otra vez ni reincorporarlos desde experimental. Auditar los campos heredados que aún conserva `Work` |
| Eliminar historial longitudinal | `Work` inicializa `activity_reviewed` y `signed_activity`; `save/load` serializan y recuperan sus atributos | A-T16 parcial. P6 aislará marcas de una nueva captura sin destruir sesiones, ediciones o hechos de firmas |
| Infraestructura de registro no habilitada | `registration.py`/`registration_excel.py` existen; `submit(journal,…,adapter)` llama `adapter.save`; no se encontró adaptador institucional ni llamada `submit` desde la GUI actual | A-T15 parcial. No anunciar «sin rutas ejecutables de escritura» en todo el repositorio; aislar API del nuevo flujo y conservar compatibilidad necesaria |
| Recuperación de comprobantes de registro | `app._show_work` carga `Journal` y `attach_receipts` si existe SQLite anterior | Lectura de comprobantes no acredita una nueva actuación. No usar SQLite como última observación del centro |
| `manual_products.py`/`manual_actions.py` | No existen en árbol activo; funciones manuales están en `manual.py`, `current_tools.py` y vistas vigentes | Conservar comportamiento actual; no crear otra capa de interfaz |
| `WorkBatch`/`analyze_package` sugeridos | Ejemplos ilustrativos; `Work`/`Row` actuales analizan Excel y conservan ediciones | P5 ampliará por adaptador. No presentar ejemplos como API ya disponible |
| `SourceReference` | Existe en `nurus.rus.models`, con libro/hash/hoja/fila; no representa aún captura/consulta/identidad del nuevo ZIP | Ampliar procedencia sin confundir ID local con identidad remota |

La búsqueda de imports/calls en `src/nurus` encontró `registration.submit`
solo como definición; las invocaciones con adaptadores ficticios están en tests.
Eso acredita ausencia de una ruta institucional operativa localizada, **no**
imposibilidad general de ejecutar la API con un adaptador futuro.

## Requisitos añadidos al contrato y al consumidor

1. **Cobertura por fuente:** `NO_SOLICITADA` corresponde a fuente fuera del plan;
   `NO_CONSULTADA`, a una solicitada que no se ejecutó. Mantener aparte
   `VACIA_COMPROBADA`, `FALLIDA`, `PARCIAL` y cobertura completa. Cero filas o
   tabla omitida no permiten inferir ausencia de actuaciones.
2. **Importación atómica:** validar ZIP, schema, hashes, tablas, límites e identidad;
   construir un candidato de trabajo y solo entonces reemplazar la sesión activa.
   Un rechazo conserva ediciones, selecciones, borradores y proyectos previos.
3. **Modelo compuesto por ingreso:** raw/normalizado y `SourceReference`, cobertura,
   eventos, varios hallazgos, propuesta y decisión humana separados. Calendario
   sin correspondencia completa queda `NO_ASOCIADO`; no asociar por RIT+nombre.
4. **Determinismo:** fecha `as_of`, versión de reglas y mapeo explícitos; fecha técnica
   de análisis separada. Reimportar captura idéntica conserva decisiones y no
   duplica hallazgos. Las marcas de sesiones no son hechos institucionales.
5. **Excel legado:** conservar lectores, hojas, alias y sincronización; declarar
   `SOLO_PLANILLA`/fuentes faltantes. No enriquecer datos desde otra sesión ni
   afirmar «sin actuaciones» por faltar bitácora/historia.
6. **Productos desde decisiones:** huellas para obsolescencia, TT/CC/RES editables,
   destinatarios comprobados y adjuntos quitados preservados. Propuesta, producto
   preparado, borrador guardado, envío y registro oficial no son el mismo estado.
7. **Interfaces:** conservar páginas y edición masiva; «Importar paquete» como vía
   principal y Excel legado como alternativa. Retirar lanzamientos/HAR de captura
   del camino principal después de G3/G5, conservando recuperación útil.
8. **Seguridad y capacidad:** ZIP traversal/bombs, fórmulas, XXE, límites y hashes;
   1.x solo compatible, mayor desconocida rechazable; 100/500/2000 registros para
   medición por etapa sin fijar tiempos arbitrarios. No se exige Office para análisis
   portable; Excel nativo/Outlook se comprueban en Windows cuando corresponda.

## Límites detectados que siguen pendientes

- A-T01/03/05/19 no implementados: no existe contrato/importador v1 autónomo.
- A-T02/04/07/09/10/14/15/16/20 parciales: existen piezas legadas, pero no
  acreditan el modelo compuesto, el desacoplamiento y toda la semántica requerida.
- A-T06/08/11/13/17 con evidencia de implementación del flujo legado; sus pruebas
  relacionadas son fixtures locales y no aceptación del consumidor nuevo.
- A-T12/18 requieren Outlook/Windows/Office institucional; no sustituirlos por
  dobles de COM, CI o Linux. La matriz conserva sus bloqueos.

`Work.export` actualmente establece `rus_recorded` a partir de revisión local en
su payload legado. No se interpreta ese campo como evidencia de registro oficial.
El contrato/modelo nuevo separará estado humano, preparación y evidencia remota;
su cambio efectivo deberá probar paridad y migración antes de activarse.

No se activan CT nuevas, ni escritor RUS, ni envío Outlook. Las funciones del
baseline permanecen intactas mientras se revisa el resultado P0 completo.

## Verificación de esta actualización

- Matriz regenerada: 115 IDs únicos, 20 A-T y criterios del documento fuente;
  rutas y estados comprobados. Inventario: 674 archivos del baseline conservados,
  121 módulos y 562 definiciones de pruebas.
- Cuatro comprobaciones negativas del generador rechazan documento 02 ausente,
  hash modificado, A-T01 sin clasificar y A-T01 duplicado antes de escribir la
  matriz. [Resultados](evidencia_p0/flow-revision-02-matrix.json).
- Doce suites existentes sobre sesiones, ediciones, productos, configuración,
  Outlook seguro y registro legado: **192 pasaron, 1 omitida** por Windows.
  [Salida](evidencia_p0/flow-revision-02-tests.txt) y
  [JUnit](evidencia_p0/flow-revision-02-tests.xml). No se ejecutó Office real.
- Nueva generación golden: **17 artefactos con paridad** de contenidos,
  observaciones, reglas, productos y estilos.
  [Salida](evidencia_p0/flow-revision-02-golden.txt).
- Compilación de las tres herramientas de auditoría correcta. No se modifican
  archivos de aplicación ni pruebas del baseline para acomodar estos resultados.
