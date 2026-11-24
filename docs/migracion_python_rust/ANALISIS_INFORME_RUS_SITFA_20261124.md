# Análisis del «Informe técnico RUS/SITFA para IA» (24-nov-2026) frente a `app/`

Entrada: `INFORME_TECNICO_RUS_SITFA_PARA_IA_2026-11-24.md`, v1.0, aportado por el usuario. No se copia al repositorio.

Rama contrastada: `estudio/paridad-python-rust-20261119`, punta `a47b1e3`. Método: lectura completa del informe y búsqueda dirigida en `app/src` y en el árbol Python de referencia. No se ejecutaron pruebas: es solo documentación.

Etiquetas:
- **N1**: hecho verificado en código (archivo:línea);
- **N2**: inferencia con su supuesto;
- **N3**: especulación.

## 1. Veredicto

- El informe sirve como **catálogo de límites semánticos**: qué no se puede concluir por fecha, proximidad o ausencia. Lo que describe como «existente» en Python ya está casi todo en `app/`, con paridad (E-Q1 a E-Q7, E-S1, E-A2, E-A5).
- Su aporte nuevo a esta rama:
  1. evidencia contra la asociación por cercanía, que toca tres comportamientos heredados de Python (R-A, R-B y R-D);
  2. dos fuentes visibles en la ventana de causa que ya se descarga (Notificación y Diligencias), coherente con D-5;
  3. una guardia de solo lectura basada en los nombres exactos de los controles de creación.
- La §7 (registrador de observaciones) no se aplica a esta rama (ver C3).

## 2. Conflictos con la rama `[CONFLICTO_ABIERTO]`

| ID | Informe | Rama | Resolución |
|---|---|---|---|
| C1 | Base `feat/registrador-observaciones-rus-v3` @ `9707e72` | El objeto no existe en este clon (`git cat-file` falla). | No se pueden verificar desde aquí sus afirmaciones sobre esa rama: CI 45460844644 y DPAPI. |
| C2 | «Flow Rust congelado por decisión de 19-nov-2026» | D-4 (23-nov) dispone integrar todo; D-5 dice que el Descargador recolecta y Flow procesa. | En esta rama priman D-4 y D-5, posteriores y expresas. El informe está desactualizado respecto de Rust. |
| C3 | §7, P27 y P28: registrador de creación | `AGENTS.md`: «Sin módulo de registro RUS ni SATURNO; si reapareciera, `submit` debe lanzar error.» | Se excluye. El catálogo de §7.1 solo sirve como guardia negativa (M4). |
| C4 | Histórico: «falta prueba real tras corregir el iframe» (V05) | El registro del 24-nov cita el diagnóstico anónimo de un lote del usuario: «114 históricos, todo LEIDA», con el exe Rust. Ver `REGISTRO_IMPLEMENTACION.md`, entrada «Diagnóstico anónimo de calendarios e históricos». | Para Flow Rust, V05 tiene evidencia de uso real (N1, según el registro). El pendiente del informe sigue abierto solo para la rama Python que cita. |

## 3. Contraste dato a dato

### 3.1. Coincide con `app/` (sin acción)

| Elemento del informe | Evidencia en `app/src` (N1) |
|---|---|
| Tope de 5000 aperturas por lote; detener tras 5 bitácoras fallidas seguidas; suspender el histórico tras 5 fallas | `descargador/bitacoras.rs:23`, `:24`, `:49` |
| `ShowObservaciones` con tipos 11/12/13 | `descargador/vinculos.rs:270` |
| Historia: ventana de 1 a 4 meses; una lectura por causa en la misma pasada | `descargador/lecturas.rs:739`; `bitacoras.rs:42` |
| Envoltorio iframe del histórico | `descargador/historico.rs:252` |
| Otro programa vigente: `--` y `SIN INFORMACION` en la misma fila | `captura_trabajo.rs:285` |
| `VENTANA_INFORMES_DIAS = 60`, distinta del filtro de 25 días | `captura_trabajo.rs:18` |
| Días negativos con egreso futuro → aviso, sin afirmar vencimiento (P09) | `motor.rs:744` |
| Entrega anterior sin período acreditado → marca, conservando las acciones | `motor.rs:655` |
| Nomenclatura atribuida con calendario pendiente → revisar asociación (§6.1, punto 5) | `motor/historia/escritos.rs` → `HISTORIA.INFORME_REVISAR_ASOCIACION` |
| Avisos OB-01..OB-05 del Excel de órdenes | `ordenes.rs:1` (E-A5: 21 casos contra Python) |
| Comparador de cortes (P01): no compara lotes parciales ni filtros no verificables, y «Ausente no implica egreso» | `descargador/resultados.rs:563`, `:637` |
| Tablero de avance y cobertura (P12, parcial) | E-Q7, avance por fila; `diagnostico_bitacoras.json` y `diagnostico_calendarios.json` |

### 3.2. Ausente en `app/` y en Python

- **Lector de Notificación y Diligencias (§4.11, §4.12, P16, P17).**
  - El extractor anónimo Python ya cuenta las pestañas visibles: `descargador/extraer_estructura_historia.py:507`.
  - El fixture tiene las pestañas, pero no sus tablas: `descargador/fixtures/historia_ficticia/ventana_historia.html:50-51`.
  - `app/src`: sin coincidencias de `Notific` ni `Diligenc` en los lectores.

## 4. Riesgos de error en comportamientos heredados de Python

Los cuatro tienen paridad exacta con Python (N1). `AGENTS.md` prohíbe cambiar reglas y textos sin decisión del usuario y caso golden: se presentan como **decisiones pendientes** (D-10 a D-12).

### R-D · Conciliación de capturas: «egresadas» sin mirar la cobertura (el de mayor impacto práctico)

- **N1, código.**
  - `conciliar.rs:344` agrega a `egresados` toda fila de la captura anterior sin pareja en la nueva. Replica `src/nurus/personal/capture_changes.py:43-56`.
  - `ui.rs:306` y `ui.rs:1047` muestran «… egresadas».
  - No hay control de cobertura ni de alcance de las consultas antes de usar el rótulo.
- **Contraste interno (N1).** `comparar_lotes` (`descargador/resultados.rs:563`) rechaza lotes parciales y advierte: «Ausente no implica egreso ni eliminación». La misma aplicación aplica dos criterios distintos a la misma pregunta.
- **N1, informe.** Base de la objeción en §4.4: «No cerrar registro por desaparición del listado (puede ser filtro, traslado, omisión o nueva consulta parcial)»; §5.4: si es parcial, «no ausencia global».
- **Escenario (N2; supuesto: el usuario abre una captura de un tribunal tras otra de dos).** Todas las filas del tribunal no consultado aparecen como «egresadas». Pasa lo mismo con una página FALLIDA o con otra modalidad.
- **Alcance del daño (N2).** Hoy es un rótulo y una lista: no cambia reglas ni productos. El riesgo es que el usuario tome por egreso lo que es falta de cobertura.

### R-A · Órdenes de búsqueda: cierre por cercanía y sin R2

- **N1, código.** `motor/historia.rs:442` replica `analisis_historia.py:26`: cada cierre cierra la orden anterior más cercana.
- **N1, R2.** `busqueda_historia` no consulta `causa_compartida`. Sí lo hacen `servicio_ingreso` (`:170`), `prorroga` (`:585`) y las reglas de `:298`, `:306` y `:354`.
- **N1, informe.** §4.14, §6 y V10: una persona puede tener varias órdenes, y un «Deja sin efecto búsqueda» genérico no identifica cuál cierra.
- **Escenario (N2; supuesto: hermanos con órdenes propias en la misma causa).**
  - Orden de A en el folio 10 y orden de B en el folio 20.
  - El cierre de A en el folio 25 se asigna a B, la orden más cercana.
  - Queda «vigente» la orden de A, y la frase R11 se redacta en las filas de ambos hermanos.
- **Contradicción (N2).** R2 (20-nov) dice: «la imposibilidad de atribuir un trámite a un ingreso queda como aviso interno». R11 es la regla de historia que no la aplica.

### R-B · C10 (escrito sin ingreso): lo suprime cualquier nomenclatura posterior

- **N1, código.** `motor/historia/escritos.rs:120-126` replica `reglas_historia.py:478`: basta un «Ingreso Informe de Cumplimiento» con folio mayor, sin vínculo con el escrito.
- **N1, informe.** Evidencia negativa de §6.1: asignar cada escrito al primer folio posterior hizo converger 15 escritos en 6 resoluciones.
- **Escenario (N2; supuesto: resoluciones atrasadas).**
  - Escrito del período 1 en el folio 40 y escrito del período 2 en el folio 50.
  - La resolución del folio 55 corresponde al período 1.
  - El escrito 50 pierde C10, y con él la propuesta `NOMENCL`.
- **Mitigación existente (N1).** Con el informe vencido, `informe_historia` cita el folio 55 con `INFORME_REVISAR_ASOCIACION`. El riesgo es un **falso negativo parcial**, no una afirmación falsa.

### R-C · Elección de trámite por fecha sin folio explícito (sin acción)

- **N1.** `escritos.rs:31-47`: sin asociación explícita, `elegir_tramite` toma el último trámite con fecha igual o posterior al vencimiento.
- **N2.** Es aceptable porque solo cita el trámite o pide revisar su asociación, que equivale al «candidato» del informe.

## 5. Simplificaciones

- **S1 · Una sola tabla de inferencia por cobertura (§5.4 del informe). Recomendada.**
  - `VACIA_COMPROBADA` aparece en 11 archivos de `app/src`: `paquete.rs` (17), `revision.rs` (5), `revision/v1.rs` (5), `contrato.rs` (5), `captura_trabajo.rs` (4), `ui.rs` (3) y otros (N1: `grep -c`).
  - Propuesta: una función `alcance(estado) -> Alcance { Desconocido, SoloPositivas, VaciaAcotada, Completa, Conflicto }` en `captura/contrato.rs`, para los módulos que **interpretan** estados. Los productores no cambian.
  - Mejora concreta (N2; supuesto: hay interpretaciones duplicadas que pueden divergir): una regla única de «qué se puede afirmar». Sirve de base para la corrección de R-D.
  - Condición: refactor sin cambios de salida, con los oráculos existentes en `fallos 0`.
- **S2 · Esquema de resultado de regla de §10.1. Rechazada.** Duplica `Frag`, marcas y avisos, y obliga a cambiar los productos. Sería sobrediseño.
- **S3 · Índices por entidad (§10, P02, P33). Sin acción.** La historia ya se lee una vez por causa (`bitacoras.rs:42`) y el comparador de lotes cuenta la multiplicidad. No hay lectura repetida identificada (N2: no se midió).

## 6. Mejoras propuestas

Orden: primero lo que reduce errores, luego las capacidades nuevas. Las decisiones D-10 a D-12 son del usuario.

| ID | Mejora | Cómo | Mejora concreta | Costo (N2) | Requiere |
|---|---|---|---|---|---|
| M1 | **«Ausentes», no «egresadas»** (R-D) | En `conciliar.rs`, clasificar cada fila sin pareja según las `consultas` de ambas capturas: misma consulta (tribunal, pestaña, modalidad y filtros) COMPLETA en la nueva → `AUSENTE_CON_COBERTURA`; si no → `NO_COMPARABLE`. Rótulo en `ui.rs`: «ausentes en la captura nueva (N con cobertura, M sin cobertura)», con el mismo texto de advertencia que `comparar_lotes`. | Evita leer como egreso lo que es un tribunal, una página o una modalidad no consultada. Unifica el criterio con `comparar_lotes`. | Bajo-medio. Casos: mismo alcance, tribunal omitido, consulta PARCIAL. | **D-10**, porque se aparta del rótulo de Python |
| M2 | **R11 con R2** (R-A) | En `busqueda_historia`, con `causa_compartida` y más de una orden: sin frase R11 en la fila y con aviso interno «Orden de búsqueda de la causa no atribuible a este ingreso». `R11_BUSQUEDA_VIGENTE` se conserva. | No se redacta la orden de un hermano en la observación del otro. R11 queda alineada con R2. | Bajo, más casos golden con 2 ingresos × 2 órdenes | **D-11** |
| M3 | **C10 por asociación** (R-B) | Suprimir C10 solo si la nomenclatura posterior tiene asociación explícita (`evidencia.resolucion`) o si el escrito es el único candidato. Si no, emitir C10 junto con `INFORME_REVISAR_ASOCIACION`. | Elimina el falso negativo 40/50/55: se propone `NOMENCL` para el escrito sin resolver. | Medio: redacción y golden de informes | **D-12** |
| M4 | **Guardia de solo lectura** (C3) | Verificador estático `tools/verificar_solo_lectura.py`. Falla si `app/src` (fuera de `#[cfg(test)]`) o las extensiones incluidas mencionan los controles de creación de §7.1: `COD_Estado_Detalle_Nuevo`, `id_Obs_centro`, `GLS_ObsCentro_Nueva`, `chkBitacora`, `chkBitacoraEdit`, `Txt_Cont2`, `GLS_Observacion`, `COD_Estado_Detalle`. Hoy no hay ninguna aparición (N1). | Convierte en control automático la regla «sin escritura en RUS» de `AGENTS.md`, con los nombres exactos de la evidencia. `inventario_escrituras.py` solo revisa escrituras a disco. | Muy bajo | Nada. Encargo E-X1 |
| M5 | **Notificación y Diligencias, paso (a): diagnóstico** | Extender `historia.rs::estructura_pestana` (`:156`) para registrar, sin valores, si la respuesta de la causa ya descargada trae tablas con los encabezados de §4.11 (`Esta.Not.`, `Fec.Not.`, …) y §4.12 (`Folio`, `Est.Dil.`, …), y cuántas filas. Escribirlo en `diagnostico_calendarios.json` o en uno hermano. | Da el dato N1 que falta (V06/V07): si esas fuentes llegan sin navegar más. Coherente con D-5. | Bajo | Nada: es diagnóstico anónimo. Encargo E-X2 |
| M6 | **Paso (b): lector y aviso de notificación fallida o pendiente** | Solo si M5 muestra las tablas en la misma respuesta. Lector con las columnas exactas, alcance CAUSA y sin asociar por fecha. Nueva tabla aditiva en el paquete. Aviso **candidato** «notificación fallida o pendiente» en Trabajo, sin texto en la observación. | Alertas de notificaciones fallidas que hoy no se ven, sin abrir otra pantalla. | Medio-alto | Capacidad nueva fuera de la paridad: decisión, extracto anónimo y oráculo propio sin Python |
| M7 | **Deriva de esquema** (P29) | Comparar los diagnósticos anónimos con los del lote anterior de `carpetas_previas`, solo nombres de campos y encabezados. Avisar si cambian. | Detecta un cambio de HTML de SITFA antes de que todo el lote quede `NO_VERIFICADA`. | Bajo | Decisión, porque es un aviso nuevo |

### No recomendadas en esta rama

- **P27/P28** (registro, y FECHA_OBS/CC tras el recibo): prohibidas por `AGENTS.md`.
- **P30** (PDF y anexos): fuera del alcance acordado.
- **P22/P34** (trayectorias de persona y analítica de permanencia): riesgo de perfilamiento, sin caso de uso.
- **P18–P20**: sin una decisión concreta que los justifique, como pide el propio informe en §9.2.

## 7. Orden sugerido y criterio de aceptación

1. **E-X1 (M4) y E-X2 (M5).** No requieren decisión.
   - E-X1: el verificador falla si se inyecta `chkBitacora` en `descargador/puente.rs`, y pasa en la punta.
   - E-X2: prueba con un fixture ficticio con y sin pestañas pobladas; conteos afirmados.
2. **Decisiones del usuario.** D-10 (M1, la de mayor impacto práctico), D-11 (M2) y D-12 (M3). Cada una se aprueba con casos golden propios. Los textos de `config.rs` cambian solo en lo que se decida.
3. **S1** antes o junto con M1.
4. **M6 y M7**, según lo que muestre E-X2.

## 8. Límites

- **N1.** No se ejecutaron pruebas. Las afirmaciones de código salen de la lectura de `a47b1e3`.
- **N2.** Los escenarios de R-A, R-B y R-D son construidos; ningún caso del corpus los ejercita. Esos son los casos golden que faltan.
- **N3.** No hay datos sobre la frecuencia real de hermanos con órdenes distintas, resoluciones atrasadas o conciliaciones entre capturas de alcance distinto.
