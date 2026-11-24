# Encargos para IA ejecutoras · Flow Rust

Vigente desde la auditoría del turno 17 (23 de noviembre de 2026), sobre `f6a8f28` o un commit posterior de `estudio/paridad-python-rust-20261119`.

## Roles

- **Ejecutora.** Una IA toma **un** encargo, lo implementa o lo prueba y entrega evidencia. No planifica ni redefine criterios. Si un criterio no se puede cumplir, lo informa como `BLOQUEADO`, con el motivo, y no como `PASA`.
- **Planificación y auditoría.** Prioriza encargos, revisa cada entrega contra su criterio y repite las comprobaciones antes de aceptar un resultado. Solo la auditoría cambia el estado de un encargo en este archivo.
- **Usuario.** Toma decisiones de redacción y de reglas y firma el acta (paquete H). Ninguna IA marca algo como «aceptado» o «institucional».

## Reglas de entrega (obligatorias)

1. Leer `AGENTS.md`, este archivo, el encargo y las secciones de [VERIFICACION.md](VERIFICACION.md) que cite.
2. Un `PASA` solo vale con evidencia reproducible:
   - commit probado;
   - entorno (L, W o V de [PLAN_PRUEBAS.md](PLAN_PRUEBAS.md));
   - comando exacto;
   - líneas literales de salida (`test result`, conteos), filtradas con `grep`.
   Sin esa evidencia, el estado es `NO ACREDITADO`.
3. No citar pruebas, funciones o archivos que no estén en el commit publicado. Antes de entregar, comprobar cada nombre con `grep -rn "fn <nombre>" app/`.
4. Una prueba que depende de una variable de entorno (`FLOW_CASOS`, `FLOW_E3_PAQUETE`, `FLOW_REFERENCIA`) y termina con `return` sin ella **no acredita nada**. Indicar la variable usada e incluir en la salida una línea propia del caso, por ejemplo `ORACULO: 174/174`.
5. Un comparador no puede absorber diferencias más amplias que la regla que acredita. Normalizar todo el texto (mayúsculas, tildes o guiones) para clasificar una diferencia acotada es una FALLA del encargo.
6. No cambiar reglas, umbrales ni textos tradicionales (`config.rs`, `textos_base.json`). Si una prueba muestra una diferencia de redacción no aprobada, se registra como decisión del usuario.
7. Datos solo sintéticos. RIT ficticios con año 2099. Antes de publicar, `python tools/check_real_ids.py app/src app/tests` y `python tools/check_contactos.py app/src`.
8. Al terminar:
   - agregar al [registro](REGISTRO_IMPLEMENTACION.md) una sección `Entrega · <encargo>` con la plantilla de abajo;
   - cambiar el estado del encargo a `ENTREGADO`, nunca a `APROBADO`;
   - publicar en la misma rama, sin `--force`, sin fusionar a `main` y sin releases.

### Plantilla de entrega

```
## Entrega · <ID>
- Commit probado: <sha>  · Entorno: <L|W|V> · Herramientas: <rust x.y, python x.y>
- Comandos: <uno por línea>
- Evidencia: <líneas literales de salida>
- Resultado por criterio: <criterio → PASA | FALLA | BLOQUEADO (motivo)>
- Cambios de código: <archivos> · Cambios de lógica de producción: <ninguno | detalle>
- Pendientes: <lista>
```

## Estado de los paquetes de prueba tras la auditoría del turno 17

| Paquete | Estado auditado | Fundamento (N1 salvo indicación) |
|---|---|---|
| A | **PASA, con salvedad** | Las 9 pruebas A1–A8 existen. Biblioteca 40/40 y binario aislado 41 + 1 ignorada en Linux. A7 y A8 no se contrastan con `motor.read_excel` ni con `carga.py` sobre los mismos bytes; usan valores esperados escritos a mano. → E-A78 |
| B | **PASA las cifras, criterio incompleto** (D-1 resuelta: el guion se conserva) | Reproducido: 2.582 casos, 1.997 idénticos, R1 7, R6 577, R1+R6 1 y 0 fallos. El comparador normaliza mayúsculas, tildes y guiones en todo el texto. No verifica R2 ni R4 (solo `R3_MULTI`) ni la conservación de R5. Las 585 diferencias caen dentro del nombre del programa. En 51 de ellas Rust conserva el guion del nombre de origen y Python lo elimina, lo que la R6 aprobada no contempla. → E-B, D-1 |
| C | **NO EJECUTADO** | `app/qa/generar_contexto_e3.py` no cambió: no existe el ZIP de historia que pide el plan, ni contraste con `nurus.personal.work`. Los nombres de prueba del informe no coinciden con los del repositorio. Las cinco pruebas existentes terminan con `return` sin `FLOW_E3_PAQUETE`. → E-C |
| D | **NO ACREDITADO** | Las pruebas `ui_ajustes_muestra_umbrales`, `ui_descargador_muestra_opciones`, `ui_trabajo_muestra_botones` y `ui_revision_muestra_tabla` no existen en ningún commit. No se cita la corrida Windows del commit ni la evidencia de las comprobaciones manuales D2. El SHA-256 informado corresponde a un binario local que no se puede verificar. → E-D |
| E | **NO EJECUTADO** | Se revisaron referencias antiguas, no las capturas de la corrida del commit ni `trabajo-contexto.png`. → E-E |
| F | **PARCIAL** | F1 PASA (biblioteca 40/40). F2 (pausa efectiva medida), F3 (recuperar pendientes en una carpeta nueva sin tocar el lote original) y F4 (favorito tras reinicio) no se ejecutaron. → E-F |
| G | **PASA, con salvedad** | G1: sin `.Send(` y `CC_OBLIGATORIO` fijo en `config.rs`. G2 y G3 sin hallazgos. G4/G5: el script da 58/58 y 16/16, pero solo compara de Rust a Python y omite entradas que no calzan con su expresión regular. G6 no tiene evidencia. → E-G |
| H | Usuario | Sin cambios. |

## Encargos abiertos (en orden de prioridad)

### E-B · Comparador estricto de la redacción mensual — `CERRADO POR AUDITORÍA` (23-nov-2026; R1 y R6 acreditadas, R3 → E-J, R5 → E-K) · [prompt](prompts/E-B.md)

- Objetivo: que `motor::historia::tests::paquete_b_redaccion_mensual` clasifique solo lo que acreditan R1 y R6.
- Pasos:
  1. Clasificar como R6 solo las palabras que difieren dentro del nombre del programa de la fila. Las palabras del nombre salen de la entrada del caso (`programa`, `programa_cruce` o el campo que use `titulo_programa_registro`). Una diferencia fuera de ese tramo es FALLA.
  2. Contar aparte `R6_GUION`: el nombre conserva « - » en Rust y Python lo omite. Es una variante aceptada de R6 (decisión D-1).
  3. Afirmar que no aparecen `R2`, `R3`, `R3_MULTI` ni `R4` en las observaciones.
  4. Comprobar R5 según `INFORME_FALLAS_20261120.md` (tabla de decisiones).
  5. Si falta `FLOW_CASOS`, la prueba falla. No debe terminar con `return`.
- Criterio: salida `PAQUETE_B: … R1 a, R6 b, R6_GUION c, R1+R6 d, fallos 0`, con a + b + c + d = 585 sobre `f6a8f28`. No cambia la lógica de producción.

- Auditoría (23-nov-2026, entorno L, ver registro «Auditoría · E-B»): las cifras de la entrega se reprodujeron en Linux, pero R2, R3 y R5 se informaron como PASA sin casos que las activen y la línea R5 era texto fijo. La auditoría corrigió la prueba: `R6` y `R6_GUION` se cuentan por separado (526 + 51), la suma se afirma, `evaluados` se fija en 2.582, R5 se cuenta (el motor por fila no la emite) y la falta de casos se informa como `SIN_COBERTURA`. La cobertura de R3 queda en `paquete_b_cobertura_r3`, marcada `#[ignore]` hasta cerrar E-J.

### E-J · Casos de R3 en el oráculo — `CERRADO POR AUDITORÍA` (23-nov-2026; entrega `2c97a14`/`7fd7f80`) · [prompt](prompts/E-J.md)

- Objetivo: que el corpus `rust/casos` tenga casos sintéticos de Espera con `HISTORIA.E04_CON_FOLIO` y `ESPERA.E05_SOLO_CORREO`, y que `paquete_b_cobertura_r3` pase sin `#[ignore]`.
- Pasos: casos dirigidos en `rust/oraculo/generar_casos.py::dirigidos`, regeneración determinista, actualización de `CASOS_PAQUETE_B` y clasificación estricta de R3 si Python y Rust difieren.
- Criterio: `PAQUETE_B_COBERTURA: R3 n/n` con n ≥ 3, `fallos 0`; los casos previos del corpus no cambian; `oraculo_python` y `oraculo_historia` pasan con los nuevos totales.
- Auditoría (23-nov-2026, entorno L, ver registro «Auditoría · E-J»): la corrida en Linux reproduce las líneas de la entrega (`R3 3/3`, `fallos 0`, 2.585 evaluados). La regeneración con `pandas<3` da `rust/casos/*.json` idénticos a los publicados. Queda cubierto el pendiente de la entrega sobre el entorno L. Salvedad: Python deduplica oraciones idénticas (`composicion.py`), así que la diferencia R3 no puede aparecer en el oráculo. La prueba acredita que Rust produce la misma observación de una sola aparición; no ejercita el recorte del comparador.

### E-K · R5 (reiteración) en `app/` — `CERRADO POR AUDITORÍA` (23-nov-2026; entrega `ed5a6df`/`02e1555`) · [prompt](prompts/E-K.md)

- La reiteración (`COMUN.REITERA_OBSERVACION`) la aplica la cronología, no el motor por fila: `bitacora_avisos.aplicar_reiteracion` en Python y `Cronologia::aplicar_reiteracion` en `app/src/captura_trabajo/cronologia.rs`.
- Su oráculo es `rust/oraculo/generar_cronologia.py`, que escribe `rust/crates/flow-trabajo/tests/datos/cronologia.json`. Ninguna prueba de `app/` lo consume (N1: `grep -rn cronologia.json app/` sin resultados).
- Hacer: una prueba de `app/` que recorra `cronologia.json` y compare observación, reglas y avisos con Python, incluyendo la decisión R5 parcial (no reiterar «Medida revisada, sin observaciones.» ni «La derivación … no se encuentra sujeta a seguimiento por este Centro.»).
- Criterio: `CRONOLOGIA: n/n`, fallos 0, sin cambiar la lógica de producción; si hay diferencias, se informan como hallazgo.
- Auditoría (23-nov-2026, entorno L, ver registro «Auditoría · E-K»): las líneas se reproducen en Linux y dos mutaciones de `aplicar_reiteracion` hacen fallar la prueba. Salvedades: la prueba arma las `Entrada` con el mapeo de `temporal.py` y no pasa por `leer` ni por `Revision::acredita_centro`, así que la lectura desde un ZIP queda para E-C. La segunda excepción de R5 parcial («La derivación…») no tiene casos en el oráculo.

### E-C · Paquete C completo — `CERRADO POR AUDITORÍA` (23-nov-2026; entrega `eabcb99`/`0ca8097`, corregida en la auditoría) · [prompt](prompts/E-C.md)

- Ejecutar el paquete C de [PLAN_PRUEBAS.md](PLAN_PRUEBAS.md) tal como está escrito: `contexto-e3-historia.zip` con sus seis elementos, apertura en las tres etapas, contraste con `nurus.personal.work` y corte antes de la ventana.
- Evidencia: el diff Python↔Rust por fila (vacío, o solo R1/R6) y la línea literal de cada caso con `FLOW_E3_PAQUETE` definido.

### E-L · Pruebas condicionadas que pasan sin ejecutarse — `CERRADO POR AUDITORÍA` (23-nov-2026; entrega `690db75`/`ce0c69b`, CI corregida en la auditoría) · [prompt](prompts/E-L.md)

- Hecho (N1, auditoría del 23-nov-2026): las pruebas de `app/src` que leen `FLOW_REFERENCIA`, `FLOW_E3_PAQUETE` o `FLOW_CASOS` terminan con `return` si la variable falta, y se cuentan como `ok`. Ocurre en `trabajo.rs` (7), `captura.rs`, `captura_trabajo.rs`, `revision_ui.rs` y `ui.rs:2241`, y en `oraculo_python` (`motor.rs`). Es el salto silencioso que prohíbe la regla 4 de entrega.
- Hacer: marcar esas pruebas con `#[ignore = "requiere FLOW_…"]` y quitar el `return` (que fallen si se piden sin la variable), o agruparlas en una corrida explícita con `--include-ignored`. Documentar el comando en el registro y en la CI de Windows.
- Criterio: sin variables, la suite informa esas pruebas como `ignored`, no como `ok`. Con variables y `--include-ignored`, pasan. Sin cambios de lógica de producción.

### E-F · Descargador: pausa, pendientes y favoritos — `CERRADO POR AUDITORÍA` (23-nov-2026; entrega `1ee6011`/`af04493`)

- Agregar en `app/src/descargador/sitfa_ficticio.rs`:
  - F2: registrar el `Instant` de cada `search` de un lote de tres consultas con pausa pedida de 0 s; el intervalo debe ser ≥ 0,5 s;
  - F3: cancelar después de la primera consulta; `pendientes` devuelve dos. Se ejecutan en una carpeta nueva y las huellas de todos los archivos del lote original quedan iguales;
  - F4: guardar un favorito, volver a cargarlo con `Preferencias::cargar` desde la misma ruta (simulación de reinicio) y aplicarlo, con alcance y fechas recalculadas;
  - D-2: con `conservar_filtros` pedido, un lote Integral de calendario o de Carga envía `keep_filters: false` a `prepare` y uno de Litigantes envía `true`. Para eso, extraer de `iniciar` (`descargador_ui.rs`) a una función de la biblioteca la regla «solo Seguimiento y Litigantes» y probarla.
- Criterio: las tres pruebas existen, pasan, y biblioteca y clippy `-D warnings` siguen limpios.

### E-A78 · Contraste Python de A7 y A8 — `ENTREGADO` (23-nov-2026, ejecutado por la auditoría; D-3 pendiente)

- Generar con Python, a partir de bytes sintéticos fijos, la salida de `descargador/motor.py::read_excel` (A7) y de `descargador/carga.py` (A8). Guardarla como fixture JSON en `app/tests/datos/` junto a los bytes de entrada.
- Las pruebas Rust comparan contra ese fixture. Agregar un script en `app/qa/` que lo regenere.
- Criterio: filas, conteos y mensajes de error iguales a Python.

### E-G · Auditoría de textos y escritura — `ENTREGADO` (23-nov-2026, ejecutado por la auditoría)

- `tools/verificar_textos_umbrales.py`:
  - comparar en ambos sentidos: cada texto de Python que usa Rust debe existir, y viceversa;
  - informar las entradas de `config.rs` que la expresión regular no leyó;
  - fallar con código distinto de 0 ante cualquier discrepancia.
- G6: listar con `grep` toda escritura a disco de `app/src` (`fs::write`, `File::create`, `OpenOptions`, `rename`) y clasificar cada ruta como perfil local o destino elegido por el usuario.

### E-D · Compilación e interfaz en Windows — `ENTREGADO` (23-nov-2026, por la auditoría en L; falta el run de la CI Windows) · [prompt](prompts/LOTE-2.md)

- Lanzar «Flow Rust local · Windows» sobre el commit vigente y citar el run y su resultado.
- Ejecutar las comprobaciones manuales D2 con el ejecutable del artefacto y el ejemplo ficticio en un perfil aislado. Evidencia: capturas sintéticas y SHA-256 del ejecutable del artefacto.
- Pruebas de interfaz automatizadas: solo si se agregan al repositorio (por ejemplo con `egui_kittest`) y corren en la CI.

### E-E · Revisión visual de la corrida — `ABIERTO` (entorno V)

- El paquete E de [PLAN_PRUEBAS.md](PLAN_PRUEBAS.md) sobre el artefacto de la corrida de E-D.

### E-I · Lote Integral y Carga en Windows — `ENTREGADO` en la biblioteca (23-nov-2026); ejecución desde el exe → paquete H · [prompt](prompts/LOTE-2.md)

- Con la extensión ficticia sobre HTTP local, ejecutar desde el ejecutable un lote Integral de Seguimiento con dos páginas, otro de Carga y una consulta vacía con PDF. Verificar los nombres y `resumen.json` según el turno 16 del registro.
- La prueba con Chrome real y sesión SITFA corresponde al usuario (paquete H).

### E-P · Descargador Integral restante — `EN ESPERA` (tras E-F e E-I)

- Lecturas de bitácora, historia, calendario e histórico (F19), paquete CSMP y exportaciones de lote (F22). La auditoría lo especificará en encargos más chicos antes de asignarlo.

## Integración completa (D-4): encargos E-Q, E-R, E-S y E-T

Base común: portar desde `rust/crates/*`, que tiene pruebas en verde en L (23-nov-2026): `flow-captura` 81, `flow-trabajo` 78, `flow-anonimo` 38, `flow-productos` 60 y `flow-meta4` 29. Se reutilizan sus oráculos de `rust/oraculo/`. Cada encargo:
- se integra en `app/` (biblioteca o binario), sin cambiar reglas ni textos;
- trae su prueba golden contra Python sobre datos ficticios y su mutación de control;
- deja la suite y la CI en verde.

Restricciones vigentes:
- SITFA solo se lee con la sesión del usuario a través de la extensión;
- sin escritura en RUS ni SATURNO;
- el Asistente no inicia el Descargador.

En orden de dependencia:

### E-Q1 · Lectores locales de contexto — `ENTREGADO` (23-nov-2026, `8cfd5b6`, por la auditoría) · [prompt](prompts/LOTE-3.md)

- Portar `flow-captura::lecturas` a `app/src/descargador/lecturas.rs`: `leer_bitacora`, `leer_historia`, `leer_calendario`, `leer_historico`, `leer_escritos_html` y `separar_ventana`.
- Oráculo: `rust/oraculo/generar_lecturas.py` (`nurus.bitacora_html`, `historia_html` y `calendario_html`). Su fixture se copia o se regenera en `app/tests/datos/`.
- Criterio: `LECTURAS: n/n, fallos 0`, con todas las ventanas del fixture y sus pruebas negativas; ventana de 1 a 4 meses.

### E-Q2 · Vínculos del listado — `ENTREGADO` (23-nov-2026, `702ccad`, por la auditoría) · [prompt](prompts/LOTE-3.md)

- Portar `flow-captura::vinculos` (extraer, `descartar_ambiguos` e identidad) con el oráculo `generar_vinculos.py` (`descargador/vinculos.py::extract` y `extract_historia`).
- Criterio: `VINCULOS: n/n, fallos 0`.

### E-Q3 · Puente y comandos de lectura — `ENTREGADO` (23-nov-2026, `a161241`, por la auditoría)

- Permitir en el puente `diary_open`, `history_open` y `calendar_open` **solo en Integral**, que ya declara `bitacoras_lectura`, `historia_lectura`, `calendario_lectura` e `historico_lectura`. Lite sigue rechazándolos.
- `abrir_bitacora`, `abrir_historia` y `abrir_calendario` sobre el SITFA ficticio.
- La prueba de `puente.rs:440` pasa a exigir el rechazo en Lite y la aceptación en Integral. `navigate` y `carga` siguen rechazados.

### E-Q4 · Lote de bitácoras (`DiaryRunner`) — `ENTREGADO` salvo los Excel (23-nov-2026, `3012cdd` … `c17caa3`, por la auditoría); los Excel pasan a E-Q5

- **Hallazgo (N1, 23-nov-2026):** en Flow Python las lecturas no corren dentro del lote de listados. Son un flujo aparte del Descargador (`start_diary`, `descargador/lectura_bitacoras.py::DiaryRunner`, 1.068 líneas, más `lectura_historia.py`, con 351, y `historico_manual.py`, con 182), con su propio formato en disco:
  - `bitacoras/bitacoras.json` (manifiesto) y una instantánea por registro;
  - `historia/historias.json`;
  - `historico/`;
  - el calendario;
  - `diagnostico_bitacoras.json` anónimo;
  - `Bitacoras_RUS.xlsx`.

  `flow-captura::recoleccion` (777 líneas) tiene otro modelo de `resumen` y no se puede copiar sin más.
- Hacer: `app/src/descargador/bitacoras.rs` con el comportamiento de `DiaryRunner`:
  - enumerar el listado y abrir por fila bitácora, historia, calendario e histórico, con los lectores de E-Q1 y el puente de E-Q3;
  - pausa, tope de lecturas, `MAX_FALLAS_SEGUIDAS` y cancelación;
  - reanudación;
  - reutilización de lecturas de las últimas horas (`lecturas_reutilizables`, `carpetas_previas`);
  - escribir **el mismo formato en disco que Python**.
- **Oráculo:** ejecutar `DiaryRunner` de Python y el de Rust sobre el mismo SITFA ficticio (`descargador/test_lotes.py::CATALOG`, `test_lectura_historia.FakeBridge` y `bitacora_fixtures`) y comparar los manifiestos, salvo marcas de tiempo declaradas. Además, `package_current_lot` de Python debe aceptar la carpeta producida por Rust.

### E-Q5 · Paquete CSMP desde el lote — `ENTREGADO` salvo los Excel y el XLSX consolidado (24-nov-2026, `8814173`, `8655610`, `44ebe91`, por la auditoría); los Excel quedan en E-Q5d

- Incluye también los Excel de E-Q4: `Bitacoras_RUS.xlsx` (`nurus.capture.diaries.export_diaries`) e `Historia_SITFA.xlsx` (`nurus.capture.historias.export_historias`). Los dos dependen de `import_lotes`, el mismo análisis que usa el paquete. Se comparan las celdas con openpyxl y calamine.

- Python: `descargador.py::package_current_lot` → `export_csmp_capture.py::export_capture_with_note` (806 líneas), con `load_sources` y `match_diaries`.
- Hacer: el mismo ZIP `CSMP_CAPTURE` 1.2.0 desde la carpeta del lote de listados más la de bitácoras.
- **Oráculo:** sobre las mismas carpetas ficticias, Python y Rust dan las mismas tablas y el mismo manifiesto (salvo la procedencia no determinista, como en E-C). `nurus.capture.read_capture` y `import_capture` de Python abren el ZIP de Rust, y el paquete C de `app/` sigue en `fallos 0` con él.

### E-Q5d · Excel del lote de bitácoras y XLSX consolidado — `ABIERTO` (L)

- `Bitacoras_RUS.xlsx` (`nurus.capture.diaries.export_diaries`, con las hojas extra de `import_lote`), `Historia_SITFA.xlsx` (`nurus.capture.historias.export_historias`) y el XLSX consolidado del paquete (`consolidated_workbook`, `resultados.export_book(csmp=True)`).
- Oráculo: celdas comparadas con openpyxl y calamine sobre las carpetas de `app/tests/datos/paquete.json` (escenario «contexto»).

### E-Q6 · Interfaz del Descargador para las lecturas y el paquete — `ENTREGADO` salvo el avance por fila (24-nov-2026, por la auditoría)

- Opciones de lectura (bitácoras, historia, calendario, histórico y escritos), progreso, tope y reanudación, y botón «Crear paquete».
- Pruebas `egui_kittest` en un perfil aislado.
- Pendiente: avance por fila de la lectura (hoy, indicador de actividad y resumen al terminar) y prueba de la lectura real con la extensión en Windows (W).

### E-R1 · Meta 4: cálculo — `ABIERTO` (L)

- Portar `flow-meta4` (cálculo y tribunales) a `app/`.
- Oráculos: `generar_calculo_meta4.py` (450 casos de `META4_Bitacoras/scripts/meta4_bitacoras.py`) y `generar_tribunales_meta4.py`.
- Agregar un adaptador para carpetas de lote producidas por Python (F23).

### E-R2 · Meta 4: plantilla, regeneración y vista — `ABIERTO` (L/W), tras E-R1

- Portar `flow-meta4::{plantilla,regenerar,salida}` y agregar la vista «Meta 4» en la navegación de `ui.rs`.
- Contrastar los cinco productos con Python sobre un lote ficticio y probar la vista con `egui_kittest`.

### E-S1 · Exportaciones de lote y comparador — `ABIERTO` (L)

- `descargador/resultados.py`: `export_book`, `export_csv`, `export_ics`, `export_zip` y `compare_lots`, con un oráculo nuevo en `app/qa/` sobre lotes ficticios.
- Base en `flow-trabajo::exportar`.

### E-T · Brechas menores — `ABIERTO`

- **E-T1:** elegir la cuenta Outlook (solo borradores, `mail.Save()`, CC fijo).
- **E-T2:** formularios manuales (`manual.py`), sin abrir un Excel.
- **E-T3:** estadísticas y métricas de Resultados (`statistics.py`, `review_metrics.py`).
- **E-T4:** Anonimizador con ZIP, paquetes de captura y verificación posterior de XLSX (portar `flow-anonimo`, con el oráculo `generar_anonimo.py`).
- **E-T5:** distribución portátil y autodiagnóstico (`portable/`, `package_check.py`).

## Decisiones del usuario

- **D-3 · Mensaje ante un XML 2003 con entidad externa — RESUELTA, opción a** (23-nov-2026, usuario): se conserva el mensaje de Rust. Antecedente (E-A78): Ni Python ni Rust resuelven la entidad y ambos rechazan el archivo, pero con mensajes distintos: Python dice «No se reconoce la estructura RIT/Nombre del listado; descarga detenida.» y Rust «No se pudo abrir/parsear el archivo Excel; requiere revision local.». La prueba `a78_igual_a_python_sobre_bytes_fijos` la cuenta como `DIFERENCIA_ACEPTADA D-3` y falla ante cualquier otra.
- **D-4 · Alcance de la migración — RESUELTA: integrar todo** (23-nov-2026, usuario). Encargos E-Q, E-R, E-S y E-T. Ver [INVENTARIO_HERRAMIENTAS_20261123.md](INVENTARIO_HERRAMIENTAS_20261123.md): `app/` no tiene las lecturas de contexto del Descargador, Meta 4, las exportaciones de lote, los formularios manuales ni las estadísticas. Decidir si se integran (desde `rust/crates/flow-captura` y `flow-meta4`) o si Flow Rust convive con el Descargador Python.

- **D-1 · Guion en el nombre del programa (R6): conservar** (23 de noviembre de 2026). «PRM CIUDAD DEL NIÑO - MULCHÉN» se redacta «Ciudad del Niño - Mulchén», como hace hoy Flow Rust. R6 incluye conservar el guion del nombre de origen. No requiere cambios de código. E-B sigue contando `R6_GUION` por separado, como variante aceptada de R6: no es FALLA.
- **D-2 · `keep_filters` en Integral: igual que Python** (23 de noviembre de 2026). En Python, `lotes.py::BatchRunner` lo envía sin restringir, pero `descargador.py::batch` (líneas 1117 y 1139) lo fija en `False` fuera de Seguimiento y Litigantes. De punta a punta, Python solo conserva filtros en esas dos pantallas, y Flow Rust hace lo mismo (`descargador_ui.rs`, `iniciar`). No requiere cambios de código. La duda planteada en la auditoría del turno 17 consideraba solo `lotes.py`. Se cubre con el punto agregado a E-F.
