# Revisión E3 — rama vigente del Asistente, Descargador y anonimizador (21 de octubre de 2026)

Revisor: Claude, sesión `01UiipQ3fZdQ8wDJSoFcza3h`. Encargo del usuario (21-oct-2026): «Los elementos que todavía no han
pasado la revisión E3, revísalos tú; si pasan, da por cumplida la revisión e intégralos a las ramas pertinentes».

Alcance: `ultima-version/asistente-descargador` (`a41791e`) y `ultima-version/anonimizador` (`08db0b3`) frente a
`feat/ejecucion-plan-20261008` (`0ff63a1`). Datos usados: solo sintéticos.

VEREDICTO: **APRUEBA CON CAMBIOS.** Los tres cambios exigidos (E3-1 a E3-3) quedan aplicados, con pruebas de regresión
que fallan sin la corrección.

## Independencia (decisión j)

- **Independiente** respecto de la sesión `01VB84tpYXBaUxLxFtBYLGQx` (lucid-sagan: WP-55 a WP-57, WP-64 a WP-69,
  WP-71 a WP-79, decisión u), de Codex (WP-58) y de la sesión que escribió exciting-gates (WA-1 a WA-7, CT-05/07/11,
  WD-1) y el anonimizador independiente.
- **No independiente** respecto de la resolución de conflictos de la integración (`85b1ad2`, `c3bd9fb`, `a41791e`) ni
  de la corrección de umbrales (`ddcd243`): las escribió esta sesión. Se revisaron con pruebas ejecutadas y aquí se
  encontró E3-1, pero no reemplazan una E3 de otra raíz. El usuario dio la revisión por cumplida con esta salvedad.
- Las rondas anteriores (`E3_REGLAS_HISTORIA_20261014.md`, `_R2.md`, `E3_WP64_20261014.md`, `E3_WP65_69_20261016.md`)
  se incorporan a esta rama como antecedentes. La ronda 2 y la del 16-oct no eran independientes; esta revisión cubre
  sus diffs.

## Método

Lectura adversarial de los diffs que la decisión j declara de riesgo: `motor/`, `textos_base.json`,
`textos_observaciones.json`, contrato de captura (`contracts/`, `src/nurus/capture/`), retención y borrado
(`descargador/preferencias.py`), extensión SITFA y anonimizador. Luego, pruebas sintéticas de los casos límite donde las
dos líneas se cruzan, y las suites completas.

| Área | Commits revisados | Resultado |
| --- | --- | --- |
| Extensión SITFA (`history_open`, `calendar_open`, captura PDF) | `62f1a6e`, `e59bcd4`, `fecfe92`, `5ca466b` | Sin hallazgos |
| Contrato de captura (columnas aditivas, CT-05/07/11) | `6c256b4`, `4927fa3`, `91a3108` | Sin hallazgos |
| Retención y borrado de lotes | `70a8114`, `ca8bb90` y siguientes | Sin hallazgos |
| Motor: decisiones u y x, WP-55, WP-56, WP-66 | `ecfafac` a `b875ab1` | Sin hallazgos propios de esos commits |
| Motor: WA-1, WA-3, WA-7 y su fusión con la decisión x | `284674f`, `9632a08`, `c3bd9fb` | **E3-1** |
| Interfaz de umbrales con 18 parámetros | `ea057d2`, `91a3108` | **E3-2** |
| Anonimizador independiente | `c416246`, `08db0b3` | **E3-3** |

### Sin hallazgos (verificado)

- **Extensión.** La historia y el calendario solo se abren si el enlace exacto existe en el listado vigente y coincide
  el tribunal seleccionado; si no, el error es explícito. Solo hacen peticiones GET a `familia.pjud.cl`, más la llamada
  de cifrado que hace la propia página. Usan `redirect: "error"`, límites de bytes y tiempo, y no leen
  `document.cookie` ni almacenamiento. Si la respuesta repite un valor cifrado, se descarta (`HISTORY_REFLECTED`). La
  fase `RED` se marca antes de cada petición, para que el error de red se informe como tal.
- **Contrato.** Las columnas de `historia_tramites` son opcionales y van al final, sin cambiar `schema_version`. El
  lector ya aceptaba columnas al final antes de este cambio. CT-05, CT-07 y CT-11 son de «solo mostrar», con
  referencia de aprobación y forma de revertirlas.
- **Retención.** Se rechazan enlaces simbólicos y *junctions* en la raíz, en la selección y en la carpeta de archivo. La
  selección se vuelve a verificar justo antes de borrar. «Conservar» mueve el Excel y el diagnóstico de historia antes
  de limpiar.
- **Motor.** Están bien aplicados:
  - I1 por fecha, la prórroga solo como constancia, la duplicidad por RUT y la regla 3 desde el inicio de la espera
    (decisión u);
  - el orden por grupos y la regla de decir cada dato una vez;
  - la ventana de la búsqueda medida por su último trámite;
  - la mención en bitácora solo con folio o fecha completa.

  El texto plural de audiencias y la reiteración de CT-07 usan el grupo «constancia», igual que el singular.

## Hallazgos

### [E3-1] mayor — CORREGIDO — el margen DCE de 10 días impedía contrastar la historia

Lo introdujo la resolución de la fusión (`c3bd9fb`), no lucid-sagan ni exciting-gates por separado. Con un informe DCE
vencido en RUS hace 1 a 10 días (margen WA-7), `vencido` se calculaba desde la clave de la frase y no desde la fecha de
RUS. Efectos, con corte el 14-oct-2026 y vencimiento el 9-oct:

- Si la historia trae «Ingreso informe de cumplimiento X» del 11-oct, igual salía el correo pidiendo el informe
  (`programa_por_vencer`), sin la marca `I01_CONTRADICHA`.
- Si trae un ingreso del 1-jul (de otro período), se citaba como entrega vigente y no salía el aviso `ENTREGA_ANTERIOR`.

Corrección: el contraste con la historia usa `vencido = dias < 0` (decisión u). El margen de WA-7 solo cambia la frase y
el plazo cuando la historia no desmiente el vencimiento. Pruebas:
`test_dce_en_el_margen_con_informe_ingresado_despues_del_vencimiento` y
`test_dce_en_el_margen_con_informe_anterior_al_vencimiento_conserva_el_aviso` (`tests/test_historia_informes.py`).

### [E3-2] menor — CORREGIDO — el último umbral compartía celda con «Guardar umbrales»

Con los 18 umbrales de dev10, la fila 9 de parámetros ocupaba la misma celda (9,0) que el botón, fijo en esa fila.
Comprobado con `grid_slaves()` bajo Xvfb. Corrección: el botón y las casillas se ubican después del último umbral.
Prueba: `test_ningun_umbral_queda_bajo_el_boton_de_guardar`.

### [E3-3] mayor (privacidad, modo conservador) — CORREGIDO — apellido suelto junto a un nombre conocido

En modo conservador, una palabra con mayúscula pegada a un nombre ya reemplazado quedaba visible: «Josefa Muñoz
**Tapia**», «abog. Juan Pérez **Cotapos**», «hermano Tomás Araya **Lillo**». Las reglas de nombre desconocido exigen dos
palabras y el marcador del nombre conocido cortaba la secuencia. El modo estricto (el que viene por defecto) no
filtraba nada. Corrección: `scrub_unknown` oculta la palabra de nombre que sigue a un nombre conocido, y la que antecede
a un apellido conocido, salvo términos institucionales. Pruebas: tres nuevas en `anonimizador/tests/test_textengine.py`.
Sonda sintética posterior: sin fugas en ninguno de los dos modos; «PRM Quillón» es un programa y se conserva a propósito.

## Observaciones (N2, no exigen cambio)

- O-1: los riesgos 1 y 2 de la segunda E3 de WP-53 (valores cifrados reflejados y tribunal visible distinto del menú)
  solo se confirman con SITFA real. El código falla cerrado; queda para la prueba real del usuario.
- O-2: la reiteración de CT-07 (bitácora) y `HISTORIA.REITERA` (historia) conviven. CT-07 reemplaza la observación
  completa cuando repite a la última del centro (≥ 0,92), así que no duplica frases. Sin comprobación con datos reales.
- O-3: los avisos CT-05, CT-07 y CT-11 dependen de una cronología `ACREDITADA`; sin paquete no aparecen.

## Controles

- Asistente: 1240 aprobadas, 11 omitidas, 5 xfail. Descargador: 352 aprobadas, 62 subpruebas. Node 3/3. Ruff y formato.
- Paridad golden de 17 productos: PASA, sin cambiar referencias en esta ronda.
- Anonimizador: 93 aprobadas, 2 omitidas (requieren pantalla).
- `verificar_wp.py --lista-eliminadas docs/implementacion/verificacion/integracion-20261021-eliminadas.txt`: PASA.

## No verificado

Windows, Office, Outlook y SITFA reales; aceptación institucional. Esta revisión no acepta ni declara nada institucional:
eso solo lo hace el usuario en un acta.
