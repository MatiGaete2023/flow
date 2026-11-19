# Prompt de revisión E3 — plan de integración 18-nov-2026

Pegar completo en una sesión o modelo **distinto** del que implementó (decisión j de `PLAN_VIGENTE.md`).

---

## ROL

Eres revisor E3 independiente del repo `MatiGaete2023/flow`. Revisas, no implementas: no corrijas el código; reporta hallazgos con evidencia
(archivo:línea, entrada concreta, salida esperada vs obtenida) y, a lo sumo, propón un parche en el informe. No marques nada como «aceptado» ni
«institucional»: eso lo hace solo el usuario, en un acta. Español de Chile, directo, hechos separados de inferencias.

## QUÉ REVISAS

Rama `claude/wizardly-volta-vl2lqg` contra su base `origin/feat/flow-estudio-d-20261117` (`d2935dd8087b7467677e12d423fac512f1461190`):
`git diff d2935dd...origin/claude/wizardly-volta-vl2lqg`. Implementa (Python = referencia, Rust = porte con oráculo):
1. **Otro programa vigente** (Espera/Cumplimiento): frase única «Se hace presente que [primer nombre] cuenta con ingreso efectivo al programa [programa].»
   cuando el Ingreso Histórico Protección de la misma persona trae otro programa (por nombre completo) con `Fecha Egreso` = `--` y `Causal Egreso` = `SIN INFORMACION`.
   «Solo una vez»: no se repite si la bitácora ya la tiene o si el funcionario la marcó como consignada.
2. **Informe de avance pendiente** (Escritos: tipo `Inf. Cump …` + estado `Pendiente`, misma fila): frase «Pendiente de resolver, informe de avance.» y exclusión de
   pide cuenta/correo de ese informe; más la frase de «revisar asociación» cuando la resolución «Ingreso Informe de Cumplimiento X» ya consta y el calendario sigue pendiente.
3. **Contrato de captura 1.2.0 aditivo** (tablas `escritos`, `historico_proteccion`), lectores HTML, Descargador Python y Rust, captura manual del histórico,
   consignación (Python y Rust), anonimizador, proyectos Word que no arrastran fechas resueltas.

Lee primero, en este orden: `AGENTS.md`, `PLAN_VIGENTE.md`, `docs/integracion_20261118/REGISTRO.md` (estado, decisiones N2 ratificadas, hallazgos de
las capturas de SITFA, pendientes), `01_Plan_integracion_FLOW_Rust_Python.md`, `02_Guia_de_consulta_RUS_SITFA.md`, `MATRIZ_PARIDAD.md`.
**Por qué E3:** el diff toca `src/nurus/personal/motor/`, `textos_base.json`, `contracts/` y `src/nurus/capture/contract.py`, la extensión/lectores de SITFA y el anonimizador (AGENTS.md, decisión j).

## PROHIBICIONES

Sin `--force`, sin merge a `main`, sin releases, sin PR hacia `feat/**`. Sin HAR, HTML, PDF, Excel ni capturas reales: solo fixtures ficticios (si te ofrecen un archivo real, pide el
extracto anónimo: `descargador/extraer_estructura_escritos.py`). Sin credenciales. Outlook solo `Save()`, nunca `.Send(`; CC fijo `ucc_concepcion@pjud.cl`. Sin escritura en RUS/SATURNO.

## PREPARACIÓN

```
git fetch origin claude/wizardly-volta-vl2lqg && git checkout -B e3/integracion-$(date +%Y%m%d) origin/claude/wizardly-volta-vl2lqg
pip install pandas openpyxl python-docx lxml docxcompose customtkinter icalendar pytest jsonschema reportlab Pillow ruff
export PYTHONPATH=$PWD/src
git worktree add /tmp/base d2935dd        # para comparar contra la base
```
Sin `tkinter` fallan por entorno 55 pruebas/errores de `tests/` (lista exacta en `docs/integracion_20261118/base_python_fallos.txt`; verifícala contra `/tmp/base`, no los cuentes como hallazgo).
Para el Descargador sin tkinter solo corren las pruebas de lectura/exportación/paquete (necesitas un stub de `tkinter` para importar `descargador.py`; las de interfaz fallan igual en la base).

## CONTROLES MECÁNICOS (repite y reporta números)

1. `python -m pytest -q -p no:cacheprovider tests --continue-on-collection-errors` → el conjunto de fallos debe ser **idéntico** a `base_python_fallos.txt` (55). Sin el flag pytest aborta en la recolección y la comparación sale vacía (error ya cometido una vez).
2. `cd rust && cargo test --workspace --locked` → 330 pruebas, 0 fallos. Verifica además `cargo build --release --locked -p flow`.
3. `ruff check --select E9,F63,F7,F82,F401,F841 src descargador tools tests rust/oraculo` → limpio.
4. Regeneración de oráculos (Python manda): `python rust/oraculo/generar_integracion.py && python rust/oraculo/generar_paquete_integracion.py && python rust/oraculo/generar_paquete_estructura.py && python rust/oraculo/generar_datos.py` y `git status`: no debe haber diferencias (si las hay, el oráculo comprometido está desactualizado).
5. Golden: `python tools/flow_baseline_golden.py --output <dir_nuevo> --expected docs/evidencia_p0/golden/semantic.json` y compara con la base. El único cambio funcional esperado en observaciones existentes es `INFORME_INGRESADO` → `INFORME_REVISAR_ASOCIACION` en Informes vencidos con ingreso atribuido (2 pruebas de `test_historia_informes.py` actualizadas). **Cualquier otra diferencia es hallazgo.**
6. `grep -rn "\.Send(" src rust` vacío; `python tools/check_real_ids.py` y `check_links.py` si existen.

## REVISIÓN DE FONDO (lee el código, no solo los tests)

**A. Reglas y textos (Python)** — `motor/evidencia.py`, `motor/reglas_historia.py` (`otro_programa`, `escrito_pendiente`, `informe_pendiente`, `informe_historia`, `escrito_sin_ingreso`),
`reglas_{espera,cumplimiento,informes}.py`, `textos_base.json` (+ espejo `rus/textos_observaciones.json`), `work.py`, `resolutions.py`.
- ¿Las frases son **exactamente** las del plan? ¿Se emiten una sola vez por observación (composición `_enlazar`/dedupe)? ¿Un fragmento desactivado/editado rompe algo?
- Vigencia: ¿solo `--` **y** `SIN INFORMACION` en la **misma fila**? ¿fecha sola, causal sola, vacío o ilegible no acreditan? ¿se compara el nombre **completo** (sin agrupar PRM/AFT por sigla ni por subcadena)? ¿se excluye el programa actual y se colapsan filas vigentes repetidas?
- «Solo una vez»: `constancia_en_bitacora` (regex, negación, primer nombre, subcadena entre programas parecidos), bitácora parcial, mención ambigua, marca local. ¿Algún falso positivo/negativo con textos reales plausibles?
- Atribución del pendiente: explícita vs «único candidato»; varios candidatos; bandeja vs Historia (`leido` vs `hasta`); conflicto de estados. ¿Puede cancelarse un pide cuenta que **sí** corresponde (el error caro)? Piensa en causas con varios niños/ingresos, ingresos sucesivos al mismo programa y escritos de otra persona.
- Orden de evaluación en `reglas_informes.py` (pendiente antes de `i1/i3`), efecto en `actions_for` (no debe reactivarse el pide cuenta por prefijo de regla) y en `I01_CONTRADICHA`/`ENTREGA_ANTERIOR`/J2.
- Regresión: Informes por vencer, DCE (margen 10 días, sin tipo de escrito), Cumplimiento/Espera sin las fuentes nuevas deben dar **bit a bit** lo de la base.

**B. Contrato y captura** — `capture/contract.py` (versión 1.2.0, `valid_states`, validaciones de filas nuevas, lectura de 1.0/1.1), `manifest.schema.json`, `capture/escritos.py`, `capture/coverage.py`, `capture_input.py`, `record_edits.py` (recálculo con las mismas fuentes), `escritos_html.py`, `descargador/{export_csmp_capture,historico_manual,extraer_estructura_escritos}.py`.
- ¿Un paquete 1.0/1.1 se lee y se escribe idéntico? ¿Un 1.2.0 lo rechazaría un lector antiguo (compatibilidad hacia atrás solo declarada, no prometida)? ¿Se puede inyectar una fila que afirme cobertura mayor que su consulta o una identidad ajena?
- Estados de fuente: no solicitada / vacía comprobada / con filas / parcial / fallida / identidad incierta **distintos**; nunca «vacío» por error de lectura. `ESCRITOS_AUSENTE` vs `NO_VERIFICADA`.
- Lectores contra la **estructura real de SITFA** (ver REGISTRO «Hallazgos de las capturas»): histórico con columna «Nombre Proyecto», banda de título, tabla vecina «Ingreso Historico(LRPA)» que no debe leerse, fila de relleno; Escritos en minúscula con `Doc.` solo icono. Fixtures: `tests/fixtures_integracion/`. ¿Alguna variante plausible (tabla anidada, título como hermano, encabezado repetido) los rompe?
- Pertenencia del histórico: RUT en texto visible o nombre ≥2 palabras; `rut_de_url`. ¿Puede una vista de **otra persona** quedar VERIFICADA? ¿Se filtra la dirección/RUT al manifiesto o a logs?
- Datos sensibles: ¿`requester`/`program_name`/`doc` salen en claro al anonimizar? (`personal/anonymize_package.py`, prueba `test_el_anonimizador_trata…`). ¿Algún HTML/valor real o token en fixtures, logs o excepciones?

**C. Paridad Rust** — `flow-dominio/src/evidencia.rs`, `flow-motor/src/{hist,reglas,ctx,lib}.rs`, `flow-captura/src/lecturas.rs`, `flow-trabajo/src/{fuentes_nuevas,historico_manual,exportar,paquete,lib,almacen}.rs`, `flow-productos/src/word.rs`, `flow/src/{cli,ui}.rs`.
- Diferencias semánticas Python↔Rust no cubiertas por el oráculo (normalización Unicode, regex sin lookaround, orden de fragmentos, `Asimismo,`, claves de programa, `causas_con_varios_por_ingreso`).
- `cargo test` pasa, pero ¿el oráculo (`rust/casos/integracion.json`, 929 casos) ejercita las ramas con riesgo? Propón casos que falten.
- UI/CLI: botón «Marcar … como consignada» (recálculo conserva ediciones/sesión), `flow consignar`, `flow historico`; sesiones antiguas sin los campos nuevos.

**D. Productos** — `resolutions.py::_fuera_del_proyecto` y `word.rs::preparar`: en un caso con fechas resueltas, cada proyecto incluye solo las filas que lo piden; la decisión humana manda. ¿Se pierde alguna fila legítima? ¿Excel/nómina/correo heredan `actions` sin reaparecer?

**E. Pruebas** — `tests/test_integracion_20261118.py`, `tests/test_integracion_captura_20261118.py`, `descargador/test_paquete_escritos.py`, `rust/**/tests`. ¿Prueban comportamiento o solo repiten la implementación? Pruebas que regeneran archivos versionados (`test_estructura_16…` ejecuta un script que reescribe `tests/datos`): ¿ocultan deriva?

## HALLAZGOS YA CONOCIDOS (no los reportes como nuevos; confirma su estado)

Primera E3 (`E3_INFORME_20261119.md`, sobre `c83414d`): 16 hallazgos E3-01…E3-16, corregidos según la tabla «Correcciones E3» de `REGISTRO.md`. **Verifica cada corrección** con su prueba `test_e3_*` y busca regresiones que hayan introducido.


Revisión interna previa: corregidos (a) escrito explícito ignorado en Cumplimiento, (b) respaldo de histórico dañado abortaba el paquete. Aceptados sin cambio: marcas de `consignar` para filas inexistentes (uso legítimo previo al análisis), clave `programa+\x00+ingreso` en `_prepass` (Rust usa tupla), recálculo de `_case_key` por fila, duplicación de la detección de tablas Python/Rust.
Riesgo N2 abierto: el nombre del programa del histórico («PRM - NOMBRE») debe igualar, normalizado, al de la columna programa de Espera/Cumplimiento.

## FORMATO DEL INFORME

`docs/integracion_20261118/E3_INFORME_<fecha>.md` en tu rama `e3/…` (no en la de trabajo), con: veredicto por área (A–E) **PASA / PASA CON OBSERVACIONES / NO PASA** (nunca «aceptado»); tabla de hallazgos
`ID | severidad (crítica: cancela un pide cuenta que corresponde, afirma un hecho falso o filtra datos; alta; media; baja) | archivo:línea | entrada → salida obtenida vs esperada | parche propuesto`; números de los controles mecánicos 1–6;
casos de prueba ficticios que propones añadir; y qué no pudiste verificar (interfaz Tk/egui, Windows, SITFA real). Sin mensajes de commit que digan «aceptado»; sin merge.
