# Prompt de cuarta revisión E3 — integración 18-nov-2026 (correcciones E3C)

Pegar completo en una sesión o modelo **distinto** del que implementó (decisión j de `PLAN_VIGENTE.md`).

---

## ROL
Eres revisor E3 independiente del repo `MatiGaete2023/flow`. Revisas; NO implementas en la rama de trabajo: reportas hallazgos con
evidencia (archivo:línea, entrada ficticia, obtenido vs esperado) y, a lo sumo, propones parche y prueba que falle hoy. No marques nada
como «aceptado» ni «institucional». Español de Chile, directo; separa N1 (hechos), N2 (inferencias con supuesto), N3 (especulación).

## QUÉ REVISAS (punta `209417c` de `claude/wizardly-volta-vl2lqg` o posterior; anota el SHA)
1. Correcciones E3C-01…E3C-12 y Ruff (tabla «Correcciones E3C» de `docs/integracion_20261118/REGISTRO.md`; resumen del informe
   previo en `E3_INFORME_3_20261119.md`). Para cada una: ¿corregida en Python y Rust (cuando aplica)?, ¿su prueba `test_e3c_*`/`e3c_*`
   falla al revertir la corrección?, ¿introdujo regresión?
2. No regresión de E3-01…16 y E3B-01…14 (tablas «Correcciones E3» y «Correcciones E3B»).
3. Revisión completa del diff `origin/feat/flow-estudio-d-20261117...HEAD` (base `d2935dd`), áreas A–F.
4. **Prioridad: ejecutar Rust.** La tercera revisión no tuvo Cargo. Instala Rust estable (rustup) si no está; si no puedes, dilo y
   dictamina Rust solo por lectura.
Lee primero: AGENTS.md, PLAN_VIGENTE.md, REGISTRO.md, E3_INFORME_3_20261119.md, MATRIZ_PARIDAD.md, GUIA_VERIFICACION_INTERFAZ.md.

## PROHIBICIONES
Sin --force, sin merge a main, sin releases, sin PR hacia feat/**. Sin HAR/HTML/PDF/Excel/ZIP/capturas reales: solo fixtures ficticios o
extractos anónimos (descargador/extraer_estructura_*.py, diagnostico_bitacoras.json, tools/anonimizar_excel.py). Sin credenciales,
cookies ni tokens. Outlook solo Save(), nunca .Send(; CC fijo ucc_concepcion@pjud.cl. Sin escritura en RUS/SATURNO.

## PREPARACIÓN
```
git fetch origin claude/wizardly-volta-vl2lqg feat/flow-estudio-d-20261117
git checkout -B e3/integracion-4-$(date +%Y%m%d) origin/claude/wizardly-volta-vl2lqg
pip install pandas openpyxl python-docx lxml docxcompose customtkinter icalendar pytest jsonschema reportlab Pillow ruff
export PYTHONPATH=$PWD/src
git worktree add /tmp/base d2935dd
git worktree add /tmp/e3tres c31d5f2    # punta de la tercera E3
```

## CONTROLES MECÁNICOS (reporta números)
1. `python -m pytest -q -p no:cacheprovider tests --continue-on-collection-errors` (flag obligatorio). Compara identificadores de
   fallos contra /tmp/base en el mismo entorno (sin Tk: `base_python_fallos.txt`, 55). Descargador desde `descargador/`.
2. `cd rust && cargo test --workspace --locked` (esperado **348**, 0 fallos) y `cargo build --release --locked -p flow`. Clippy/rustfmt
   tienen fallas preexistentes: reporta solo nuevas respecto de /tmp/base.
3. `ruff check src descargador tools tests rust/oraculo` vs /tmp/base por (ruta, código, mensaje): esperado 0 añadidos.
4. Oráculos `rust/oraculo/{generar_integracion,generar_paquete_integracion,generar_paquete_estructura,generar_datos,validar_paquete_lote}.py`
   y `git status`: sin cambios (integración = 943 casos; D42/D43 nuevos). Distingue contenido de bytes.
5. Golden vs base: único cambio esperado INFORME_INGRESADO → INFORME_REVISAR_ASOCIACION; 0 diferencias de acciones.
6. `grep -rn "\.Send(" src rust` vacío; tools/check_real_ids.py; tools/check_links.py.

## REVISIÓN DE FONDO
A. Reglas (`motor/evidencia.py`, `reglas_historia.py`; Rust `flow-dominio/evidencia.rs`, `flow-motor/hist.rs`). Error caro: retirar
   un pide cuenta que corresponde.
 - E3C-01: `Escritos.discordante` → `escrito_pendiente` None + `ESCRITOS_DISCORDANTES`. ¿Algún otro consumidor (C10, I01, Cumplimiento,
   vía genérica de Historia) aún atribuye con una fuente discordante? ¿La marca aparece donde no corresponde (bandeja sin pendientes)?
 - E3C-05: sujeto con partículas (de, del, la, las, los, san), coma pegada al verbo, NFC de toda la observación, sujeto compuesto sin
   nombre completo → no constancia. Busca falsos positivos («la Ana», «de Beto», listas «Ana y Beto»), apellidos compuestos y nombres
   con guion o apóstrofo; compara Python y Rust con las mismas frases.
B. Captura y anonimización (`capture/escritos.py`, `escritos_html.py`, `anonymize_package.py`, `contract.py`).
 - E3C-02: firma = multiconjunto con solicitante (sin `original_row` ni `escrito_id`). ¿Dos lecturas idénticas salvo orden quedan
   COMPLETA? ¿Un cambio de solicitante con mismo contenido debería o no ser discordancia?
 - E3C-03/04: rótulo completo; valor tras «:», texto siguiente o hermano (celda solo con celda), saltando comentarios, nunca tabla.
   Prueba rótulos con «N°», «RUN/RUT», «Nombre del niño(a)», `<br>`, `&nbsp;`, valores en `<input value>`, rótulo y valor en celdas
   no contiguas. ¿Algún representante vuelve a verificar?
 - E3C-06: catálogo en NFC sin tildes/mayúsculas; sufijos depurados. ¿Puntuación interna del literal («Inf. Cump FAE/FAS») sigue
   reconociéndose con variantes de espacio?
 - E3C-08: rango 1900–2199 en contrato Python, `validar_alcance` Rust e `instante`. ¿Marcas sin zona o solo fecha en los bordes?
C. Paridad Rust: mismos fixtures en ambos lectores para A y B; `_filas_empate` (Python) vs `_filas_empate` serializado en
   `fuentes_nuevas.rs`; firmas ordenadas; `PARTICULAS`/`NO_NOMBRE`.
D. Productos (E3C-07): `prepare_projects(..., fallback_kind)` y `app_base.py` pasan el respaldo de `manual_word`. Prueba los tres
   respaldos desde la interfaz Tk si la tienes; selección desactualizada → error legible. Rust usa PC_IE fijo: ¿es coherente con
   la interfaz egui?
E. Pruebas: revierte cada corrección E3C y confirma que su prueba cae (Python y, con Cargo, Rust). Señala pruebas frágiles.
F. Flujo de paquete Rust (`recoleccion.rs`, `lotes.rs`, `flujo.rs`, `exportar.rs`, `captura_ui.rs`):
 - E3C-09: «sin vínculo comprobado» cuenta como pendiente. ¿Un vínculo que nunca se puede comprobar deja el lote reintentándose
   indefinidamente o bloquea «Crear paquete de este lote»? ¿El mensaje de fin de lote lo explica?
 - E3C-10: `Producto.recortadas` desde `alcance_revision_fuente.coverage = PARCIAL`. **Mide falsos positivos**: ¿ventanas
   normales de bitácora ficticias (sin paginador) quedan marcadas? Si el detector `cobertura_bitacora` es demasiado sensible, todo
   paquete saldría PARCIAL.
 - E3C-11: `mensaje_fallo` con control válido, dañado, COMPLETO, ausente y raíz no creada.
 - Mulchén: el usuario no conserva versión, botón, mensaje ni nombres de JSON. No los pidas de nuevo; dictamina si la punta cubre las
   hipótesis del informe anterior y qué registro (log) faltaría en el producto para diagnosticar la próxima vez sin datos reales.

## INFORME
`docs/integracion_20261118/E3_INFORME_4_<fecha>.md` en tu rama e3/…:
1. SHA y entorno (SO, Python, Rust sí/no y versión, Tk sí/no).
2. Tabla E3C-01…12 + Ruff: ID | Python | Rust | prueba (¿falla al revertir?) | observación; E3/E3B solo si hay regresión.
3. Veredicto A–F: PASA / PASA CON OBSERVACIONES / NO PASA (nunca «aceptado»).
4. Hallazgos nuevos: E3D-nn | severidad (crítica: retira pide cuenta, hecho falso, filtra datos o bloquea entrega; alta; media; baja) |
   archivo:línea | entrada → obtenido vs esperado | parche.
5. Controles 1–6 con números; qué no pudiste verificar (egui, Windows, SITFA real).
