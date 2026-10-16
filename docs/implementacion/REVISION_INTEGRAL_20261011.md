# Revisión integral antes del cierre (11 de octubre de 2026)

Revisor: claude-sesion-012yjVgL, a pedido del usuario: «revisa lo hecho, busca errores, defectos de concordancia y
armonización… necesito una revisión completa antes de seguir». Base: `feat` en `32dd14c`, que contiene WP-81 a WP-85
y la consolidación de 45 ramas. Solo lectura: no se cambió código.

**Método.** Cinco revisiones paralelas:
1. Código WP-81..85.
2. Consolidación.
3. Armonización documental.
4. Verificación completa.
5. Coherencia funcional de punta a punta.

Los hallazgos principales se volvieron a comprobar en el código.

## 1. Estado general: sano

- `verificar_wp.py`: **PASA**.
  - Suites: asistente 848/11/6 xfail; descargador 240/62. Iguales en dos corridas, sin intermitencias.
  - Inventario sin bajas, golden válido, identidades 0, ruff 0, enlaces 0, módulos no alcanzables 0.
- Formato, `estado.py validar`, `check_links` y la prueba de la extensión (node): PASA.
- El autodiagnóstico del EXE da `ok` con todas sus banderas.
- La CI de `32dd14c` está verde (push y PR #1).
- **Reglas inviolables:**
  - `motor/` y `textos_base.json` sin cambios desde `286c28f`.
  - Sin `.Send(`; CC fijo intacto.
  - Sin escritura en RUS/SATURNO, y el Asistente no navega SITFA.
  - Pruebas solo con datos ficticios.
- **Consolidación de 45 ramas: sin pérdidas ni reintroducciones.**
  - Siguen presentes las correcciones de WP-50, WP-63 y WP-80.
  - El código y los archivos retirados no volvieron.
  - Las 45 ramas tienen 0 commits fuera de `feat`.
  - Los seis merges «incorporar historia» dejan el árbol igual.
- **Etiquetas con datos antiguos:** ya no existen en GitHub. En clones anteriores a la purga hay que borrarlas en
  local con `git tag -d pre-WP-12 pre-implementacion-20261009`; hecho en el clon de esta sesión, donde `--historia` da 0.
- **Flujo ficticio Descargador → paquete 1.1.0 → Asistente → Excel:** funciona. La cronología queda `ACREDITADA`.

## 2. Defectos de código y funcionales (verificados)

| # | Sev. | Dónde | Defecto | Arreglo propuesto |
| --- | --- | --- | --- | --- |
| F1 | media | `personal/work.py:170` | Con más de un modo en el paquete (Espera+Cumplimiento, el caso normal) el consolidado se descarta sin avisar y la copia sale de fidelidad reducida. El manual dice que «informa la fidelidad reducida». | Una hoja por modo o, como mínimo, un aviso en `work.warnings` |
| F2 | media | `personal/capture_input.py:236` | «Fuentes faltantes» aparece en toda importación e incluye tablas `NO_SOLICITADA` | Excluir `NO_SOLICITADA` (o listarla aparte) |
| F3 | media | `personal/capture_input.py:117,201` | Las filas EGRESADOS/OTRO pasan por `analyze` como ESPERA y producen el aviso falso «Columnas funcionales ausentes: espera» | No copiar los avisos de esos grupos |
| C1 | media | `personal/ux_support.py:46` (`_mail_state`, WP-82) | Cada selección de fila vuelve a leer y hashear todos los adjuntos de todos los borradores (50 lecturas en 10 selecciones con 5 MB) | `draft_preview_scope()` + `draft_fingerprint_for_display` |
| C2 | media | `capture/temporal.py:118` (WP-50) | Una entrada sin fecha se cuenta como «anterior al período» y las posteriores al corte desaparecen sin aviso | Contar aparte las entradas sin fecha y las posteriores al corte |
| C3 | media | `personal/current_tools.py:12` (WP-82) | `pending()` se reescribió, pero no tiene llamadores: sus 7 pruebas no ejercen nada que vea el usuario | Conectarla a la interfaz o retirarla con sus pruebas (con lista E2) |
| F4 | baja | `temporal.py` / `capture_findings.py` | Sin bitácoras solicitadas, la cronología dice «Sin entradas capturadas», lo mismo que cuando se pidieron y no hay | Estado `NO_SOLICITADA` |
| F5 | baja | `descargador/export_csmp_capture.py:201` | `calendarios.event_at` sale siempre vacío | Llenarlo con la fecha de vencimiento en ISO |
| F6 | baja | `capture_input.py:210` | El aviso «Interpretación temporal no aceptada…» se repite en cada fila | Mostrarlo una sola vez como aviso global |
| F7 | baja | `config.py:302-317` | La migración de textos rev1→rev2 construye el texto antiguo desde la base actual; 6 claves no migran | Construirlo desde `LEGACY_TEXTS_REV2` |
| C4 | baja | `work_view.py:215` | Quien ya guardó sus columnas no ve «Última obs. centro» | Agregar las columnas nuevas |
| C5 | baja | `capture_input.py:47` | `display_version` hace que la primera reimportación tras actualizar pida conciliación una vez | Recalcular solo la vista en la caché |
| — | info | `motor/utilidades.py:304` | `DeprecationWarning` de pandas/NumPy en fechas seriales: en una versión futura será un error | Decisión del usuario (`motor/` protegido) |
| — | info | `contract.py:612`, `correos_base.json` | Mensaje «solo escribe 1.0.0» engañoso (también hay 1.1.0); `cc_obligatorio` sin lector (decisión ya tomada: se conserva) | Corregir el texto |

## 3. Concordancia y armonización documental

| # | Sev. | Defecto | Armonización propuesta |
| --- | --- | --- | --- |
| D1 | alta | **WP-50**: la cronología ya está activa en `feat` (consolidación `8c4036d`). Sin embargo, la ficha exigía el acta de 10 ingresos «antes de integrar», `estado/WP-50.json` dice `BLOQUEADO_USUARIO` con `integrado_en: null`, y no hubo revisión E3 de ese diff. | Decisión del usuario: aceptar que esté visible mientras falta el acta (registrar `integrado_en: 8c4036d` y que la aceptación queda pendiente), o apagarla con `DISPLAY_RULES = ()` hasta el acta |
| D2 | alta | `CIERRE.md` está obsoleto en todo: punta `51b2477`, 592/231, 736 archivos, «etiquetas que debe borrar el usuario» (ya borradas) y acta «en la rama wp/02» | Rehacerlo sobre la punta actual |
| D3 | media | Acta WP-02: hecha para `14f679d` (anterior a WP-63..85 y WP-50) y con `evidence_kind: INSTITUCIONAL` prellenado, cuando AGENTS.md dice «No marcar nada como “institucional”» | Regenerarla con el paquete del SHA final y que el tipo lo elija el usuario |
| D4 | media | Líneas base y CI citadas sin actualizar: `PLAN_IMPLEMENTACION.md:140` dice 841/240 (vigente: 848/240); «CI final en PR #33» (cerrado) | Texto único: «848/11/6 y 240/62; CI verde en `32dd14c`» |
| D5 | media | Decisiones del 11 de octubre sin registro central: sin `pre-WP-80`, squash (regla 11), dependencias funcionales de WP-70, «Fusiona todas las ramas» y excepciones E3 (#4/#8/#11/#12–#32, #27, #36–#39) | Bloque «Decisiones del 11 de octubre» en `PLAN_VIGENTE.md` y en `global.json` |
| D6 | media | D1 tiene dos redacciones distintas (`global.json` frente a `PLAN_VIGENTE.md`) | «Historial reescrito (35 ramas), 607 artefactos y 2 etiquetas borradas; pendiente: refs/pull vía Soporte GitHub» |
| D7 | media | Fichas WP-63 y WP-81..84 con notas superadas («espera E3 antes de integrar», «sigue EN_REVISION») | Sellar «Integrado (sha), excepción E3 expresa» |
| D8 | media | Guía empaquetada (`GUIA_ENTREGA_MVP`) dice dev4/2.7.5; lo vigente es 0.5.0.dev5/2.7.6. No hay entrada 2.7.6 en el CHANGELOG | Actualizar versiones y CHANGELOG |
| D9 | media | Manuales con nombres que no existen en la interfaz: «Abrir Excel legado», «Guardar/recuperar sesión», «Cambiar carpeta», nombre de la casilla del consolidado. Además, el manual no describe la cronología | Alinear con los textos reales |
| D10 | media | `REPORTE_REVISION_20261011.md`, enlazado como «revisión actual», lista pendientes ya resueltos | Apuntar el índice a `REPORTE_CONSOLIDACION` y a esta revisión |
| D11 | media | Conflicto R-T32 redactado como «CT-01/CT-02 desactivadas»; hoy están en solo mostrar | Reformular |
| D12 | baja | `WP-70.json`: `"WP-61"` como texto suelto y faltan WP-80..85 como dependencias de archivo | Normalizar |
| D13 | baja | `PLAN_VIGENTE_propuesto.md` dice 30 días/5 sesiones, frente a D4 (60/10) | Rotular como histórico |
| D14 | baja | `AGENTS.md` cita `registration.submit`, módulo retirado en WP-22 | «Sin módulo de registro RUS; si reaparece, submit debe lanzar error» |

## 4. Pendientes del usuario (fuente única)

1. Decidir D1 (cronología visible o apagada hasta el acta).
2. Pedir a Soporte GitHub que retire `refs/pull` y las cachés (WP-17).
3. Instalar el paquete del SHA final y completar el acta de uso real (WP-02, D2).
4. Contrastar 10 ingresos para la cronología (WP-50).
5. Opcional: decidir regla por regla sobre los 6 R-T en xfail y sobre la advertencia de pandas en `motor/`.
6. Acta final firmada; después, PR #1 a `main` sin `--force` ni release (D7).
7. Opcional: borrar las ramas remotas ya fusionadas (las 45 tienen 0 commits propios).

## 5. Próximo paso propuesto

Un WP correctivo (WP-86):
- F1–F3, C1–C3 y los bajos acotados, cada uno con su prueba;
- armonización documental D2–D14;
- `CIERRE.md` rehecho.

Revisión E3 de otra raíz antes de integrar. D1 y D3 esperan la decisión del usuario.
