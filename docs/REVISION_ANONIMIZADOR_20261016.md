# Revisión del anonimizador (rama `wp/anonimizador-portable-20261015`)

16 de octubre de 2026. Alcance: `src/nurus/personal/anonymize.py`, `anonymize_package.py`, `tools/anonimizar_local.py`,
`portable/`, `.github/workflows/anonimizador-portable.yml`, `docs/ANONIMIZADOR_LOCAL.md`. No es una auditoría del
resto del repositorio. Todas las pruebas usaron datos sintéticos.

Convención: **H** = hecho reproducido en esta revisión (se indica el caso) · **L** = hecho por lectura de código, sin
reproducir · **I** = inferencia · **E** = especulación.

## Resultado

- H: las 50 pruebas existentes pasan (49 aquí; 1 requiere Tkinter/customtkinter y no corre en Linux sin pantalla).
  Pasan porque no cubren ninguno de los casos de abajo.
- H: el anonimizador del Asistente **filtra datos** en 4 casos reproducibles (F1–F4) y **falla** con 4 de los 6 tipos de
  archivo que producen el Descargador/Asistente (F5–F6).
- H: se construyó una herramienta nueva e independiente (`anonimizador/`, 89 pruebas + 1 omitida (requiere pantalla)) que corrige todo lo reproducido.
  El código del Asistente **no se modificó**.

## Filtraciones (prioridad alta)

- **F1 · Hoja de pares «Etiqueta: valor» filtra el nombre** (H). `header_row` toma la primera fila como encabezado y la
  copia sin tocar (`anonymize.py:294-307`, `:622`). Hoja `['Nombre:', 'Ana Díaz']`, `['RUT:', …]`: salida
  `('Nombre:', 'Ana Díaz')`. Además renombra la etiqueta siguiente a «Persona Ficticia 1:».
- **F2 · Apellido tras un rol no se oculta** (H, modo conservador). `_UNKNOWN_AFTER_ROLE` captura una sola palabra
  (`anonymize.py:559-561`): «Madre Marta Rojas» → «Madre [identidad omitida] Rojas». También en paquetes ZIP
  (`anonymize_package.py` reutiliza `scrub_observation`).
- **F3 · Usuario de Windows en rutas** (H). `c:\users\pgonzalez\x.xls` pasa intacto; solo se oculta por casualidad si
  son dos palabras capitalizadas. No hay tratamiento de rutas ni de enlaces.
- **F4 · Paquete ZIP sin revisión final** (L+H). `anonymize_capture` no llama a `_scan` sobre las tablas (solo el consolidado
  pasa por `anonymize`). El caso F2 sale por el ZIP sin que nada falle.
- **F5 · Números de 10–12 dígitos se conservan** (H). `_long_number` solo cubre 7–9 (`anonymize.py:469-472`): `56987654321`
  (teléfono con +56) en una columna «Causa RUS» queda igual.
- **F6 · Columna «RUTA» se trata como RUT** (H). `name.startswith(("rut","run"))`: «Ruta 5 sur km 12» → `################`. Mismo
  patrón (L) con «rit…» (p. ej. «ritmo»).

## Errores funcionales

- **E1 · Fechas en observaciones corrompidas** (H, conservador). `RIT_FREE_RE` interpreta «10-2026» dentro de
  «05-10-2026» como causa: «Audiencia 05-10-2026» → «Audiencia 05-X-3-2026». También «y 10/2026» → «Y-2-2026» (la «y» como
  tipo de causa).
- **E2 · El seudónimo se destruye** (H, conservador). `replacer.text` produce «Persona Ficticia 1» y a continuación
  `_UNKNOWN_FULL_NAME` lo toma por nombre desconocido: «juan perez soto» → «[identidad omitida] 1». Se pierde la
  vinculación entre menciones, que es el objetivo del seudónimo estable.
- **E3 · `--desplazar-fechas` no desplaza fechas en texto** (H). Solo `datetime/date` de celdas. Una fecha de nacimiento
  guardada como texto (`2015-03-04`) queda igual mientras las demás se mueven: incoherente. La GUI no expone la opción; los
  paquetes ZIP tampoco la soportan.
- **E4 · `.xls` que no es BIFF falla** (H). `_xls_sheets` usa xlrd por extensión. El Descargador reconoce cuatro variantes
  de `.xls` (BIFF, OOXML, XML 2003, HTML; `descargador/motor.py:495-552`): las tres últimas dan `XLRDError`.
- **E5 · Rendimiento cuadrático** (H). Un `re.sub` por cada nombre en cada celda: 1 500 filas ≈ 12 s en conservador
  (la herramienta nueva: 40 000 filas ≈ 25 s).
- **E6 · Publicación del ZIP exige enlaces físicos** (L). `write_capture` usa `os.link` sin alternativa
  (`capture/contract.py:666`) mientras el Excel sí la tiene (`anonymize.py:665-683`). En FAT/exFAT o recursos de red falla
  y la GUI solo dice «no se creó una copia válida». `portable/LEEME.md` describe el requisito pero no el motivo.

## Omisiones

- **O1** (H). No procesa HTML, JSON, CSV, ICS, TXT ni Word; los ZIP deben ser `CSMP_CAPTURE_V1` (la entrega
  `INDICE.txt`+`MANIFIESTO.json` de ambos programas da «Sin manifiesto»); los ZIP con originales se rechazan completos.
- **O2** (H). Cada archivo se anonimiza aislado: «Persona Ficticia 1» es otra persona en el Excel y en las bitácoras. No hay
  sesión por lote ni semilla opcional.
- **O3** (L). Un nombre solo es conocido si aparece en una columna de identidad: en otro archivo del mismo lote no se ve.
- **O4** (L). La GUI: un archivo por vez, sin carpeta de salida, sin días a desplazar, todos los errores con el mismo
  mensaje, modo conservador por defecto (el menos seguro), geometría fija 535×315 que recorta avisos.
- **O5** (L). Fechas de nacimiento se conservan sin opción de generalizarlas (documentado solo de forma indirecta).
- **O6** (L). Hojas sin encabezado reconocible: toda cadena queda `[texto omitido]` (seguro, pero también pierde estados y
  fechas). JSON dentro de celdas («JSON de textos extensos» de `Bitacoras_RUS.xlsx`) tampoco se interpreta.

## Calidad y proceso

- **Q1** (L). `portable/` y el workflow dependen del Asistente: `python -m pip install -e '.[excel-legacy]'` instala pandas,
  python-docx, customtkinter…; `package.py` copia `nurus.capture`, `nurus.rus`; `selftest.py` manipula `App.source`.
  No es independiente aunque el `.exe` lo parezca.
- **Q2** (L). `package.py` fija `UPSTREAM='9726448…'` y `LEEME.md` cita la rama `feat/ejecucion-plan-20261008`: la
  procedencia declarada no es la de la rama que se compila.
- **Q3** (L). Sin CI Linux del anonimizador; el workflow corre solo en Windows y solo al hacer push a una rama.
- **Q4** (I). Autoprueba débil: `B2 != 'Persona Ficticia Uno'` casi no puede fallar; no busca fugas de RUT, teléfono ni nombre en observaciones.
- **Q5** (I). Las advertencias son fijas («revisa manualmente…»): no distinguen qué categoría quedó pendiente, y el éxito
  siempre trae las mismas, lo que invita a ignorarlas.

## Fuera de alcance / sin verificar

- `tools/verificar_wp.py` da FALLA en esta máquina por entorno (Tkinter ausente, árbol modificado), no por esta
  revisión. Con los cambios: `check_real_ids` 0, `check_links` 0, `estado.py validar` PASA, ruff del CI 0.
- E: el costo de una filtración por nombres de una sola palabra sin rol («Ramírez no asistió») o apodos no se puede
  eliminar con reglas; solo reducir con modo estricto y revisión humana. Sin cambio de diseño posible aquí.
- No se probó en Windows ni con Tkinter real (la ventana se probó con un Tkinter simulado); no se compiló el `.exe`.
- No se evaluó el código del Asistente fuera del anonimizador.

## Qué cambia con la herramienta nueva

Ver `anonimizador/README.md`. Cada F/E/O arriba tiene prueba en `anonimizador/tests/` (F1 `test_pares_etiqueta_valor_sin_encabezado`,
F2 `test_apellido_tras_rol_se_oculta_completo`, F3 `test_rutas_con_usuario_y_enlaces`, F4 `test_captura_coherente_y_sin_datos`,
F5 `test_numeros_largos_y_ids_se_vinculan_entre_hojas`, F6 `test_json_anidado_claves_rutas_e_ids`, E1 `test_fechas_en_texto…`,
E2 `test_pseudonimo_no_se_destruye…`, E3 `test_desplazar_fechas_en_celdas_y_textos`, E4 `test_html_con_extension_excel…` y
`test_xml_spreadsheet_2003`, E5 benchmark, E6 `test_sin_enlaces_fisicos…`, O1 `test_archive.py`/`test_formats.py`, O2
`test_sesion_compartida_entre_archivos`, O3 `test_nombre_de_una_persona_en_otro_archivo_se_detecta`).

Defectos propios encontrados y corregidos mientras se construía: la misma cabecera de «pares» filtraba el nombre en la
versión nueva (se corrigió con `looks_like_person`), `_publish` podía borrar un archivo preexistente ante colisión, los RIT
ficticios de cinco dígitos se confundían con teléfonos en la revisión final y `column_kind('ruta')` devolvía RUT.

## Recomendación

1. Usar `anonimizador/` para todo lo que salga del Descargador/Asistente.
2. Decidir (usuario) si el botón «Configuración» del Asistente se mantiene con el motor viejo, se redirige o se retira;
   mientras tanto no usar `nurus.personal.anonymize` con observaciones reales: F1–F4 aplican.
3. Revisión E3 independiente del nuevo motor antes de cualquier uso institucional (exigida por `PLAN_VIGENTE.md` al tocar el
   anonimizador). Esta revisión no la reemplaza ni acepta nada.
