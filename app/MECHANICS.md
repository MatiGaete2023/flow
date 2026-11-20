# Hoja de reglas — Flow (Rust)

La ampliación de estas reglas y herramientas se sigue desde el [plan vigente](../docs/migracion_python_rust/PLAN.md). Esta hoja documenta la base del Asistente que analiza Excel.

Fuente: el código Python del ZIP `flow-flow-python`. **No hay código máquina que descompilar**: el ZIP contiene
444 archivos .py y ningún .exe, .dll, .pyd ni .pyc (`core_target_triage` → ev_66233add…; `native_functions_list`
responde «A ZIP archive is not native machine code»). Por eso las "direcciones" son archivo::función/línea del
fuente original, leído con `core_text_read`:

| Evidencia | Archivo y líneas | Qué fija |
|---|---|---|
| ev_baeaf4c7… | `src/nurus/personal/config.py` 17–36 | UMBRALES (todos los días de abajo) y SESIONES_ANTERIORES = 10 |
| ev_5960f6f7… | `src/nurus/personal/work.py` 47–79 | VENTANA_POR_VENCER_DIAS = 60 y `actions_for` |
| ev_097a02df… | `motor/reglas_informes.py` 9–33 | MARGEN_DCE = 10 y la selección I01/I02/margen |
| ev_78c4c6e4… | `contexto_excel.py` 11–45 | VENTANA_INFORME_AUDIENCIA = 30, avisos N-01 y N-02 |
| ev_0bbf1751… | `motor/reglas_espera.py` 119–148 | E-04, umbral por tribunal (30 si no reconocido), E-05 |

Todo lo que sigue se leyó en el código; lo estimado se marca **(estimado)**.

## Umbrales (días) — `src/nurus/personal/config.py::UMBRALES`

| Clave | Valor | Uso |
|---|---|---|
| espera_dce | 0 | Espera DCE: correo desde T ESPERA ≥ 0 |
| espera_laja | 30 | Espera Laja: proyecto PC_IE + correo |
| espera_mulchen | 30 | Espera Mulchén: proyecto PC_IE + correo |
| espera_tome | 60 | Espera Tomé: correo |
| proyecto_tome | 60 | Espera Tomé: proyecto desde |
| mayoria | 60 | Aviso "alcanzará la mayoría de edad" dentro de |
| oido | 45 | Oído reciente hasta |
| resolucion_reciente | 30 | E-04: resolución de ingreso con menos de |
| ingreso_reciente | 30 | C-03: días de cumplimiento 0..30 |
| medida | 45 | C-05: medida por vencer dentro de |
| informe | 30 | I-02: informe por vencer dentro de |
| ficha_antigua | 180 | C-07 ficha individual antigua: más de |
| ficha_reciente | 30 | C-07 ficha reciente: hasta |
| ficha_fae | 120 | C-08: ingreso hace más de, sin ficha FAE |
| Tribunal no reconocido | 30 | `reglas_espera.py` umbral por defecto (sin observación: el tribunal fuera de Laja/Mulchén/Tomé no se evalúa, `work.py::analyze`) |
| Margen DCE | 10 | `reglas_informes.py::MARGEN_DCE` |
| Ventana correo preventivo C-10 | 60 | `work.py::VENTANA_POR_VENCER_DIAS` |
| Mayoría de edad | 18 años; 29-feb → 28-feb | `motor/utilidades.py::fecha_mayoria` |
| Ventana informe/audiencia (N-02) | 30 | `contexto_excel.py::VENTANA_INFORME_AUDIENCIA` |
| prorroga_reciente | 90 | config.py: solo reglas de historia (no incluidas en esta versión) |
| ventana_busqueda | 60 | config.py: solo reglas de historia (no incluidas en esta versión) |

## Tribunal — `src/nurus/rus/util.py::tribunal`
Texto normalizado contiene MULCHEN → MULCHEN; LAJA → LAJA; TOME → TOME; si no, sin reglas (aviso).

## Composición — `motor/composicion.py::componer`, `motor/textos.py`
- Prefijo `PrimerNombre XXX: ` (primer nombre sin paréntesis, en Title Case; XXX = 3 primeros caracteres del
  programa sin tildes en mayúscula). Sin nombre → `XXX: `.
- Grupos: C constancia (0), A acción del Centro (1), T gestión del Tribunal (2), F cierre (3). Se ordena por grupo
  y luego por orden de aparición.
- Textos mixtos: E05_SOLO_CORREO y E05_PROYECTO_Y_CORREO = C,A; I01_* = C,A; I02_POR_VENCER_* = A;
  I02_MARGEN_DCE = C,A; CURADOR, C07_SIN_FICHA, C07_FICHA_ANTIGUA, C08_FICHA_FAE, T5 = T; MAYORIA_EDAD = C,T.
- Un dato una vez: C03/C04/C05/MEDIDA_SIN_VIGENCIA/EGRESO_PROYECTADO comparten "vencimiento"/"ingreso".
- Cada fragmento termina en un solo punto; oraciones idénticas no se repiten; desde la segunda
  "Se hace presente que" pasa a "Asimismo, se hace presente que".
- E-04 pierde "(Observación administrativa; …)" si la observación trae una acción A o T.

## Espera — `motor/reglas_espera.py::generar_observacion_espera`
1. Programa sin seguimiento (OPD, DAM, salud privada, hospital, unidad de salud mental, CESFAM, red salud,
   consulta externa, colegio, Chile Crece Contigo al inicio) → solo NO_SEGUIMIENTO; registro excluido.
2. Edad ≥ 18 → MAYORIA_EDAD (marca nivel 3) y nada más.
3. Mayoría en 1..60 días → PROXIMA_MAYORIA.
4. FEC. RESOLUCIÓN hace 0..29 días → E04_RESOLUCION_RECIENTE (principal).
5. T ESPERA ausente/negativo → incidencia.
6. Umbral: DCE → espera_dce; Laja/Mulchén/Tomé → espera_<tribunal>. Si T ESPERA ≥ umbral: DCE → E05_SOLO_CORREO;
   Laja/Mulchén → E05_PROYECTO_Y_CORREO; Tomé → PROYECTO_Y_CORREO si ≥ proyecto_tome, si no SOLO_CORREO.
7. Sin principal → E06_SIN_RESOLUCION.
8. Complementarias: OIDO (0..45 días), audiencias futuras (una: PROX_AUDIENCIA; varias: PROX_AUDIENCIAS),
   curador (columna presente sin RUT ni "Institución:" → CURADOR; ≥2 instituciones → T5_INSTITUCIONES).
9. Duplicidad (mismo RUT y programa en la misma causa) → marca nivel 1 R6.

## Cumplimiento — `motor/reglas_cumplimiento.py::generar_observacion_cumplimiento`
- Vencida = egreso proyectado presente y (días para egresar < 0 o días de cumplimiento < 0). Días negativos con
  egreso futuro → aviso y C04 se anula (`work.py::analyze`).
- Sin seguimiento → NO_SEGUIMIENTO (+ MEDIDA_SIN_VIGENCIA si vencida).
- Edad ≥ 18 → MAYORIA_EDAD + C04 o C05 si corresponde.
- C05: 0 ≤ días para egresar ≤ 45 (0 → VENCE_HOY).
- C03: 0 ≤ días de cumplimiento ≤ 30 con fecha de ingreso; + EGRESO_PROYECTADO si no vencida ni por vencer.
- EGRESO_POSTERIOR_MAYORIA(_FECHA) si el egreso es posterior a la mayoría.
- C10_HOJA2: próximo informe de la hoja de cruce (identidad RIT+RUT+NOMBRE+TRIBUNAL+PROGRAMA, fecha futura, la
  última gana) si la medida no está vencida/por vencer, o con C03, o vence el día del egreso o antes.
- Residencias (programa empieza por rta, rtt, res, rfa, rva) con columna de ficha individual: sin ficha → C07_SIN_FICHA;
  > 180 días → ANTIGUA; 0..30 → RECIENTE.
- FAE/FAS con ingreso hace > 120 días y sin ficha FAE → C08_FICHA_FAE.
- Sin nada → C09_SIN_OBSERVACIONES.

## Informes — `motor/reglas_informes.py::generar_observacion_informes`
- Sin fecha de vencimiento → incidencia, sin observación.
- DCE vencido hace 1..10 días → I02_MARGEN_DCE (plazo = vencimiento + 10).
- Vencido → I01_VENCIDO_DCE / I01_VENCIDO_GENERAL. 0..30 días → I02_POR_VENCER_*. Más lejos → sin observación.
- J2: más de un vencimiento distinto del mismo tribunal+RIT+RUT+programa → J2_ACUMULADOS (N, el más antiguo).

## Acciones — `work.py::actions_for`
E05 → correo programa_espera; E05_PROYECTO_Y_CORREO → proyecto PC_IE; I01 → correo programa_vencido + PC_INFO;
I02 → programa_por_vencer; C04/C05 → nómina "medidas"; C10 → programa_por_vencer si vence en ≤ 60 días.

## Avisos — `contexto_excel.py::avisos_fila`
N-01 (Cumplimiento): mayoría entre el corte y el egreso proyectado. N-02: audiencia antes del vencimiento del
informe y este vence dentro de 30 días después de la audiencia.

## Correos — `config.py::defaults`, `correos_base.json`, `outputs.py`
CC fijo `ucc_concepcion@pjud.cl` (+ CC adicional). FECHA = hoy dd/mm/aaaa. Alcance: todas → "todas las
modalidades"; una → "la modalidad X"; varias → "las modalidades A, B y C" (`mail_controls.py`). Nóminas:
espera = RIT, TRIBUNAL, NOMBRE, DERIVACION, T ESPERA; informes = RIT, TRIBUNAL, NOMBRE, DERIVACION, F. VENCIMIENTO;
medidas = RIT, TRIBUNAL, RUT, NOMBRE, PROGRAMA, F. EGRESO PROYECTADO, OBSERVACION. Nombres sin paréntesis.
Esta versión guarda en Borradores de Outlook (COM, solo `Save()`, nunca envía) o como `.eml` sin enviar
(cabecera X-Unsent); si Outlook falla queda una copia `.eml` de respaldo. El guardado en Outlook no se probó en vivo.

## Resoluciones — `resolutions.py`, `plantillas_word/`
Tipos PC_IE, PC_INFO, NOMENCL; un proyecto por tribunal + RIT + tipo con todas las personas de la causa; un solo
`Resoluciones_<fecha>.docx`. Faltantes → `[COMPLETAR X]`. Textos de Laja y Mulchén según sus plantillas; Tomé usa
el formato de Mulchén **(estimado: sus plantillas no vienen en el repositorio)**. Varias personas se unen con
", " y " y " **(estimado)**.

## Columnas de RUS — `src/nurus/rus/columns.py`
Alias completos de programa, tribunal, nombre, nacimiento, curador, oído, resolución, audiencias, RIT, RUT,
T ESPERA, días de cumplimiento, días para egresar, ingreso efectivo, egreso proyectado, fichas FAE/individual y
vencimiento; comparación sin tildes ni signos.

## Retención — `config.py`
Sesiones guardadas 60 días, hasta 10 anteriores.

## Verificación
- `cargo test` con `FLOW_CASOS=<rust/casos del ZIP>`: los 174 casos de referencia del motor Python original que usan
  solo datos del Excel (sin historia ni bitácora) coinciden exactamente en observación, reglas, acciones, avisos,
  marcas y exclusión (`motor::tests::oraculo_python`).
- `FLOW_FIXTURES=<rust/crates/flow-tabular/tests/datos>`: los .xls de SITFA en HTML, XML 2003 y BIFF se leen igual.
- Velocidad (release): 20.000 registros leídos y analizados en ~0,4 s; Excel de observaciones en ~0,35 s
  (el original documenta ~72 s para importar 20.000 en `RESUMEN_PROYECTO_NURUS_CSMP_20261009.md` §M01).

## Fuera de esta versión
Paquete ZIP CSMP_CAPTURE_V1 del Descargador y las reglas de historia de la causa (`motor/reglas_historia.py`),
REITERA por bitácora anterior, Descargador SITFA y Meta 4.
