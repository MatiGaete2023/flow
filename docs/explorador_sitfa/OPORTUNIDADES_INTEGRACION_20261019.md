# Oportunidades de integración: Descargador y Asistente CSMP

Fecha: 2026-10-19. Base: `feat/ejecucion-plan-20261008` @ `0ff63a1` y la
[auditoría del Explorador](AUDITORIA_V1_V1_1_20261019.md). Documento de diseño: no implementa nada ni modifica el
motor, los textos ni los contratos.

## 0. Reglas de juego (de PLAN_VIGENTE y AGENTS)

- El motor (`src/nurus/personal/motor/`) y `textos_base.json` quedan intactos salvo decisión expresa y paridad golden.
- Las reglas nuevas entran por la capa de contexto (`contracts/csmp_context_v1/catalog.json`,
  `nurus.capture.temporal`). El estado de cada regla avanza así:
  `PENDIENTE_ACEPTACION_FUNCIONAL` → `ACTIVA_SOLO_MOSTRAR` → (acta) → texto en el motor.
  Así se hizo con CT-01/02.
- Identidad VERIFICADA = tribunal + causa + ingreso. Prohibido cruzar por RIT o nombre. Historia tiene alcance CAUSA.
- Prohibido consultar por RUT (`ShowHistoriaRut` y `PopUpHistorial.jsp`).
- Cambios en la extensión o el contrato requieren revisión E3 (decisión j).

## 1. Inventario actual (HECHO)

### 1.1 Motor de observaciones

| Pestaña | Reglas | Campos Excel usados |
|---|---|---|
| Espera | E-01 no seguimiento, E-02/03 mayoría, E-04 resolución reciente, E-05 espera ≥ umbral (correo / proyecto + correo, según tribunal y DCE), E-06 sin resolución | `programa, nacimiento, resolucion, espera` |
| Cumplimiento | C-02 mayoría, C-03 ingreso reciente, C-04 vencida, C-05 por vencer o vence hoy, C-07 ficha individual (RES*), C-08 ficha FAE, C-09 sin observaciones, C-10 próximo informe (hoja 2) | `dias_cumpl, ingreso, dias_egresar, egreso_proy, ficha_ind, ficha_fae` |
| Informes | I-01 vencido, I-02 por vencer (DCE/general) | `vencimiento` |
| Común | T-01 curador, T-02 oído, T-03 audiencia | `curador, oido, prox_aud` |

### 1.2 Capa de contexto

| Regla | Estado |
|---|---|
| CT-01/02 (bitácora: última del centro y novedades) | `ACTIVA_SOLO_MOSTRAR` |
| CT-03..CT-10 | Diseñadas, `PENDIENTE_ACEPTACION_FUNCIONAL` |

`bitacoras.analyze` ya detecta reiteraciones exactas y similares (≥0,92) y el estado de respuesta (`response_state`).

### 1.3 Fuentes

| Fuente | Estado | Identidad |
|---|---|---|
| Listado (4 pestañas) | En feat | VERIFICADA vía `vinculos.py` |
| Bitácoras | En feat | VERIFICADA |
| Calendarios 90/91 | En feat | `NO_ASOCIADO` |
| Carga MAO | En feat | **Desconocida** |
| Litigantes/órdenes 94 | En feat | **Desconocida** |
| Historia | Rama `feat/historia-causa-sitfa`, no integrada | Alcance CAUSA |

## 2. Catálogo de oportunidades

Niveles de datos: **A** = ya se descarga en feat · **B** = existe en la rama de historia ·
**C** = requiere captura nueva o confirmar estructura.
Riesgo = probabilidad de falso positivo. «Reutiliza» = regla o componente existente que se amplía.

### 2.1 Controles de calidad de datos (incidencias, no observaciones)

Generan filas en «Incidencias» (formato `Incidencias.agregar` existente). No alteran el texto propuesto, por lo que su
riesgo funcional es mínimo.

| ID | Control | Datos | Fuente | Riesgo | Utilidad |
|---|---|---|---|---|---|
| Q-01 | Fechas incoherentes: ingreso < resolución; egreso proyectado < ingreso; ficha, oído o resolución futuras | Excel | A | Bajo | Detecta errores de registro en RUS antes de redactar |
| Q-02 | `dias_cumpl` ≠ (hoy − ingreso) más allá de una tolerancia; `dias_egresar` ≠ (egreso_proy − hoy) | Excel | A | Bajo | Revela campos calculados desactualizados en RUS |
| Q-03 | Mismo ingreso en dos pestañas del mismo lote (p. ej. Espera y Cumplimiento) | listado + `SITFA_INGRESO_RUS` | A | Bajo | Inconsistencia de estado en RUS |
| Q-04 | Vencimiento distinto entre pestaña Informes y hoja 2 / calendario para el mismo ingreso | Excel + calendario | A (identidad parcial) | Medio | Reutiliza CT-04 (discordancia) |
| Q-05 | Guardas de estructura: tabla con encabezado distinto o huella desconocida ⇒ `FALLIDA`, nunca «sin datos» | DOM | A | Bajo | Previene casos como WP-99 |

### 2.2 Observaciones nuevas solo con Excel (nivel A)

Requieren texto nuevo, es decir, decisión y acta. Propuesta: activarlas primero como `ACTIVA_SOLO_MOSTRAR`.

| ID | Regla propuesta | Datos | Riesgo | Texto borrador (no aprobado) |
|---|---|---|---|---|
| N-01 | **Medida excede mayoría**: `egreso_proy` > fecha de mayoría | `nacimiento, egreso_proy` | Bajo | «La medida se proyecta hasta el {EGRESO}, posterior a la mayoría de edad de {PNOMBRE} ({MAYORIA}).» |
| N-02 | **Informe vence después de audiencia**: `prox_aud` < `vencimiento` dentro de N días | `prox_aud, vencimiento` | Bajo | «Audiencia el {AUD}; informe vence el {VENC}. Se sugiere solicitarlo con anticipación.» |
| N-03 | **Ingresos simultáneos de la misma causa**: varias filas con igual tribunal + causa y distinto ingreso | `SITFA_CAUSA_RUS/INGRESO_RUS` | Bajo | Agrupar en un solo correo o proyecto (afecta la **bandeja**, no el texto) |
| N-04 | **Egresado con informe pendiente**: ingreso en Egresos que sigue en Informes por vencer | listado (2 pestañas) | Medio | «Medida egresada con informe pendiente al {VENC}; revisar.» |
| N-05 | **Cumplimiento sin ingreso efectivo prolongado**: `dias_cumpl` alto sin ficha individual ni FAE | ya cubierto por C-07/C-08 | — | No agregar (evitar duplicar) |

### 2.3 Observaciones con bitácora (nivel A, contexto)

| ID | Regla | Datos | Reutiliza | Riesgo | Utilidad |
|---|---|---|---|---|---|
| B-01 | **Reiteración de la propia observación**: el texto que propone el motor es igual o similar (≥0,92) a la última observación del tribunal en la bitácora | bitácora + texto motor | `bitacoras.analyze`, CT-07 | Medio | Evita copiar mes a mes la misma frase |
| B-02 | **Escalamiento E-05**: «solo correo» ya enviado ≥ N días antes (según bitácora) sin ingreso efectivo ⇒ sugerir proyecto | bitácora + `espera` | E-05, CT-02 | Medio | Aplica a DCE y Tomé, donde hoy se repite el correo |
| B-03 | **Observación del centro sin respuesta del tribunal** > N días, con tipo que exige respuesta | `response_state`, `fecha` | CT-05 | Bajo | Lista de pendientes propios |
| B-04 | **Informe posiblemente recibido**: entrada del centro de tipo informe posterior al vencimiento ⇒ no proponer «pidiendo cuenta» sin cotejo | bitácora `tipo` + `vencimiento` | CT-04, CT-09 | Medio (requiere diccionario de `Tipo Observación`) | Evita resoluciones pidiendo un informe ya entregado |
| B-05 | **Novedad del centro tras la última revisión** | CT-02 | Existente | — | Ya activa en «solo mostrar» |

### 2.4 Con historia de causa (nivel B: integrar la rama)

| ID | Regla | Datos | Reutiliza | Riesgo | Utilidad |
|---|---|---|---|---|---|
| H-01 | **Trámites posteriores a la última revisión** (folio, tipo, fecha) | historia | CT-03 | Medio (CAUSA ≠ ingreso) | Ver qué pasó en la causa sin abrir SITFA |
| H-02 | **Escrito de programa no proveído**: Trámite=Escrito posterior al vencimiento, sin resolución posterior | historia | CT-03/04 | **Alto** (título ≠ contenido) | Detecta informes recibidos pendientes de proveer |
| H-03 | **Curador designado según historia** frente a T-01 «no registra curador» | historia (texto) + `curador` | T-01, CT-04 | Alto | Evita la sugerencia errónea de asociar curador |
| H-04 | **Prórroga resuelta** frente a C-04 vencida | historia + `egreso_proy` | CT-04/08 | Alto | Evita marcar como vencida una medida prorrogada |
| H-05 | **Audiencia fijada en historia** sin `prox_aud` en Excel | historia | T-03 | Medio | Completa un hito |

### 2.5 Requieren captura nueva (nivel C)

| ID | Función | Bloqueo | Resolución |
|---|---|---|---|
| C-01 | **Calendario de informes por ingreso** (ventana `ShowInformesProgramados`, posible R22-c6): calendario con identidad VERIFICADA | Estructura no confirmada | `estructura.json` de una ventana c6 (junto con WP-99) |
| C-02 | **Litigantes/órdenes ↔ espera prolongada**: búsqueda pendiente explica E-05 | Se desconoce si `BusquedaLit` trae causa o ingreso | Encabezados anonimizados de un Excel |
| C-03 | **Resolución firmada (carga) ↔ C-04/I-01** | Identidad de carga desconocida | Encabezados anonimizados de `antr` |
| C-04 | **Subventanas de causa** (Litigantes, Materias, Plazos, Remisión) | Función no identificada | Testimonio del usuario o Explorador V1.2 |
| C-05 | **Pantalla INF de 6 campos** | No identificada | Nombre del menú |

### 2.6 Funciones del Descargador (no son reglas)

| ID | Función | Valor | Requisito |
|---|---|---|---|
| DF-01 | **Captura focalizada** desde solicitudes del Asistente (solo los ingresos que piden bitácora o historia) | Menos tiempo y menos lecturas (tope de 600) | CT-06; ya existe `capture_products` (solicitudes declarativas) |
| DF-02 | **Diccionario de códigos** (`TIP_Informe`, `COD_EstBusqueda`, `TIP_Consulta`) exportado en el paquete | Habilita B-04 y productos legibles | `catalog` ya los lee; contrato aditivo 1.x |
| DF-03 | Integrar `history_open` (rama historia) | Habilita H-01..H-05 | Decisión del usuario + E3 |
| DF-04 | Guardas de estructura por huella de encabezado (Q-05) | Robustez | E3 (extensión) |
| DF-05 | Lectura de informes programados por ingreso (C-01) | Calendario con identidad | Tras confirmar la estructura |

## 3. Priorización propuesta

| Orden | Paquete | Contenido | Por qué |
|---|---|---|---|
| 1 | **Calidad** | Q-01, Q-02, Q-03, Q-05 | Datos ya disponibles; sin texto nuevo; no tocan el motor |
| 2 | **Bitácora inteligente** | B-01, B-02, B-03 | Reutilizan `analyze` + CT; mayor ahorro de redacción |
| 3 | **Excel puro** | N-01, N-02, N-03 | Bajo riesgo; requieren textos (acta) |
| 4 | **Habilitadores** | DF-01, DF-02 | Reducen tiempo de descarga y habilitan B-04 |
| 5 | **Historia** | DF-03 + H-01 | Decisión mayor (rama fuera de alcance) |
| 6 | **Nivel C** | C-01..C-05 | Primero obtener la evidencia mínima |

## 4. Decisiones pendientes del usuario

1. ¿Qué paquetes entran (1–6)?
2. Umbrales nuevos: N días para B-02 y B-03, tolerancia de Q-02 y ventana de N-02.
3. Redacción de N-01, N-02, N-04 y B-02, o mantenerlas en «solo mostrar» sin texto.
4. ¿Integrar la rama de historia (DF-03)?

## 5. Actualización del 19-oct con evidencia del usuario

La evidencia fue una captura de pantalla y un Excel `BusquedaLit`. Solo se extrajo su estructura, sin valores; los
originales no se versionan.

### 5.1 Hipótesis resueltas (HECHO)

| Tema | Resolución |
|---|---|
| Ventana R22-c6 | Es el **Calendario de informes proyectados**: `IrPopUpInformesAccion.do?tipo_popUp=8&ID_Ingreso=…&TIP_Programa=…&CRR_Id…`, abierto desde el ícono «Inf.» del listado |
| Contenido del calendario | Cabecera: RIT, Nombre, Ingreso efectivo, Programa, Plazo remisión (meses), Plazo intervención (meses). Tabla: `Fecha Proyección · Fecha Informe · Estado` (observado: `PENDIENTE`) |
| Órdenes de búsqueda | Son el menú 94, «Seguimiento Búsqueda Lit» (`BusquedaLit`) |
| Litigantes | Botón dentro de «Información de la Causa» |
| `BusquedaLit` | 23 columnas, encabezado en la fila 4 y datos desde la fila 5: `Tipo Causa, Rol Causa, Era Causa, Tribunal, Rut, Nombre, Tipo litigante, Sexo, Tipo O.B., Nomenclatura, Resultado, Estado O.B., Derivado a, Fecha trámite, Duración Medida, Estado envío, Fecha Envío, E-mail Envío, Fec. Nacimiento, Edad, Nacionalidad, Prox.Auds., Curador` |
| Identidad de `BusquedaLit` | **Sin causa_id ni ingreso_id** ⇒ sin cruces con otras fuentes (contrato) |
| Listado de Cumplimiento (pantalla) | Columnas «80 BIS» y «Ult. Act. F.Amb.»; el Asistente no las mapea (`src/nurus/rus/columns.py`) |

### 5.2 Oportunidades nuevas

| ID | Regla | Fuente | Riesgo |
|---|---|---|---|
| DF-05 | Lectura del calendario por ingreso (misma ruta que la bitácora, `tipo_popUp=8`) | Descargador | Medio (E3: extensión) |
| CAL-01 | Informe proyectado con «Fecha Informe» registrada ⇒ no proponer «pidiendo cuenta» (reemplaza a B-04) | calendario + Informes | Bajo |
| CAL-02 | Proyección vencida y PENDIENTE ⇒ confirma I-01 | calendario | Bajo |
| CAL-03 | Vencimiento de la pestaña Informes u hoja 2 distinto del calendario ⇒ incidencia | calendario + Excel | Bajo |
| CAL-04 | Egreso proyectado distinto de ingreso efectivo + plazo de intervención ⇒ incidencia | calendario + Excel | Medio |
| OB-01 | Orden activa sin resultado después de N días desde «Fecha trámite» | BusquedaLit | Bajo |
| OB-02 | Orden activa y persona de 18 años o más (por «Fec. Nacimiento») | BusquedaLit | Bajo |
| OB-03 | «Estado envío» distinto de «Realizada» | BusquedaLit | Bajo |
| OB-04 | Orden activa de un niño o adolescente sin curador | BusquedaLit | Medio |
| OB-05 | Audiencia próxima con orden activa | BusquedaLit | Bajo |

### 5.3 Decisiones del usuario (ronda 1)

- Paquetes 2, 4 y 5, con umbrales B-02 = 30, B-03 = 30, Q-02 = 2 y N-02 = 30 días.
- Observaciones nuevas en «solo mostrar».
- Historia «más adelante»: **[CONFLICTO_ABIERTO]** con haber elegido el paquete 5. La interpretación propuesta es
  diseñarlo ahora e implementarlo después.
- Explorador: sin autorizaciones.

Pendientes:

- Nombres de los parámetros de la dirección del calendario.
- Otros valores de «Estado» del calendario.
- Significado funcional de «80 BIS» y «F.Amb.».
