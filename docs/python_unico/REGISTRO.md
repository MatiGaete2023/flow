# Registro de trabajo — rama `flow-python`

**Actualizar al terminar CADA turno** (antes de quedarse sin tokens): qué se hizo, verificación, siguiente paso exacto.
Commit y push a `flow-python` en el mismo turno. Plan: [`PLAN.md`](PLAN.md).

## Estado al 19-nov-2026

| Paquete | Estado | Último commit / nota |
|---|---|---|
| P1 consignar (botón) | **Hecho** | ver Historial 19-nov (P1) |
| P2 revisión de bitácoras | **Hecho** (sin prueba visual Tk) | ver Historial 19-nov (P2) |
| P3a escritos visibles + diagnóstico | **Hecho; confirmado en uso real** (9 causas con escritos, 8 vacías, 14 pendientes) | ver Historial |
| P3b escritos variante sin tabla | **Hecho en lo posible**: detección + extracto anónimo automático; la carga aparte espera ese extracto | ver Historial |
| P3c histórico automático | **Corregido (marco interno) · PRUEBA REAL PENDIENTE (usuario)** · E3 pendiente (extensión) | ver Historial 19-nov |
| P8 diagnóstico del paquete | **Hecho** | ver Historial 19-nov (P8) |
| P4 resultados / Meta 4 plantilla | **Hecho** (sin prueba visual Tk) | ver Historial 19-nov (P4) |
| P5 diseño | Pendiente | — |
| P9 notificación y diligencias | **Postergado por el usuario** | — |
| P6 `.eml`, P7 consola | **Postergados por el usuario** | — |

## Pendientes del usuario (no bloquean el desarrollo)

- **Prueba real de P3c:** ZIP nuevo de `flow-python`, recargar la extensión en `chrome://extensions`, lote chico con
  «Ingreso Histórico (Mejor Niñez)»; enviar solo `historico.json` (o `historico/estructura_historico.json` si falla).
- Prueba visual en Windows de P1, P2, P3a y P4 (el contenedor no tiene Tk).
- Revisión E3 independiente de P3c (toca la extensión).

## Siguiente paso exacto

P5 (diseño, en curso). Ver Historial.

## Base de verificación (contenedor Linux sin Tk)

```bash
python -m pytest -q -p no:cacheprovider tests --continue-on-collection-errors
```

El conjunto de fallos debe ser igual a `docs/integracion_20261118/base_python_fallos.txt` (55, por falta de `tkinter`).
Descargador: `cd descargador && python -m pytest -q …` (las pruebas de interfaz necesitan Tk real).

## Historial

- 19-nov-2026 · Creada la rama `flow-python` desde `claude/wizardly-volta-vl2lqg@d0ee2d3` (todo el trabajo de integración
  18–19 nov, correcciones E3/E3B/E3C/E3D, informe Rust → Python). Plan reformulado con la guía de RUS/SITFA (Escritos en la
  misma respuesta de la causa; Histórico por `PopUpHistorial` tipo 77). Instrucción de rama única en `AGENTS.md`.
- 19-nov-2026 · **D-H1 = sí** (usuario). **P1 hecho:** `Work.desconsignar`, `Work.acciones_consignacion` y
  `Work.aplicar_consignacion` (`src/nurus/personal/work.py`, sin Tk); bloque «Otro programa vigente · constancia única» con
  botones «Marcar «X» como consignada» / «Deshacer consignación…» en la pestaña Gestión del detalle
  (`record_view.py::_consignaciones`, `consignar`); manual del Asistente actualizado. Pruebas:
  `tests/test_integracion_captura_20261118.py::test_p1_*` (consignar, deshacer, persistencia en sesión, transaccional).
  Suite Python = base (55 por falta de Tk); ruff sin avisos nuevos. **No probado con Tk real** (contenedor sin Tk): verificar
  el botón en Windows.
- 19-nov-2026 · **P2 hecho.** (a) Descargador: `nurus/bitacora_html.alcance_bitacora` (port de Rust `cobertura_bitacora`) se
  calcula en `read_snapshot` → toda lectura (nueva, retomada, reutilizada o importada de un lote antiguo) trae `alcance`
  (`COMPLETA`/`VACIA_COMPROBADA`/`PARCIAL`) y `alcance_metodo`; el paquete lo lleva en `filters_json.revision_bitacoras` de cada
  consulta `diary-*` (`export_csmp_capture.py`). Sin falsos positivos en las ventanas ficticias de estructura real.
  (b) Asistente: `personal/revision_bitacoras.py` (sin Tk: datos por fila al importar → `Row.revision_bitacora`, clasificación
  por período, exportación Excel + JSON en carpeta nueva); `personal/revision_view.py` (ventana) abierta desde
  **Trabajo › Más ▾ › Revisión de bitácoras…**: período (por defecto el mes que termina en el corte, como Meta 4), filtro,
  «Seleccionar en Trabajo», «Exportar Excel y JSON…». Manual del Asistente actualizado. Pruebas:
  `tests/test_revision_bitacoras.py` (12) y `descargador/test_revision_bitacoras_p2.py` (2, de punta a punta; fallan con el
  código anterior). Suite Python = base; Descargador = mismos fallos de entorno; ruff sin avisos nuevos.
  **No probado con Tk real**: verificar la ventana en Windows. Paquetes capturados antes de esta versión quedan «No
  verificable» (sin evidencia de alcance) hasta recapturar.
- 19-nov-2026 · **P3a hecho.** Descargador: `lectura_historia.estado_escritos` registra por causa si la respuesta traía la
  tabla Escritos (CON_FILAS / VACIA / SIN_TABLA / AUSENTE / ILEGIBLE, filas, pendientes) y `diagnostico_historia.json` suma
  `escritos` (solo conteos). Asistente: `Row.escritos_resumen` al importar y columna **«Escritos»** en Trabajo («N
  pendientes», «Sin pendientes», «Sin escritos», «No verificada», «Lecturas discordantes»; vacía si la fuente no vino).
  `tests/test_trabajo_vista.py` (Tk) actualizado a la nueva columna. **P3b:** la carga aparte de la pestaña no está en los
  HAR (solo la variante con cuatro tablas); cuando una causa trae la pestaña sin tabla, la Historia deja
  `historia/estructura_escritos.json` ANÓNIMO (`extraer_estructura_escritos.estructura_pestana`) para implementar la carga
  con la forma real. Pruebas `test_p3a_*`, `test_p3b_*`.
- 19-nov-2026 · **P3c hecho (requiere E3: extensión).** Extensión: comando `historico_open` (`pagina.js`) que solo acepta un
  RUT presente en un enlace `ShowHistoriaRut` de la tabla del listado vigente y abre
  `PopUpHistorial.jsp?tipo_popUp=77&RUT_Litigante=…&RUT_DV=…` (GET, html, ≤ 8 MiB); protocolo `historico_lectura:1` en la
  extensión y en `contrato_integral.PROTOCOLS`; permitido en `puente.py`. Descargador: casilla **«Ingreso Histórico (Mejor
  Niñez)»** (preferencia `leer_historico`, desactivada por defecto); `DiaryRunner(historico=True)` lo lee una vez por persona
  y lote (`_leer_historico` → `historico_manual.capturar`, pertenencia por el RUT pedido); una falla deja solo el histórico
  FALLIDO y el paquete PARCIAL. Flujo de paquete y «Crear paquete» pasan `historico=` al exportador. Filas sin
  `SITFA_PERSONA_RUS` (Espera) reciben la clave `rut-<huella>` (`lectura_bitacoras.persona_por_rut`), igual en el registro y en
  el histórico. Decisión w en `PLAN_VIGENTE.md`; nota en `contracts/csmp_capture_v1/HISTORIA_VENTANA.md`. Pruebas
  `test_p3c_*` (puente ficticio y de punta a punta hasta la frase «otro programa vigente»). **Supuesto N2 a confirmar en
  SITFA real:** que `ShowHistoriaRut` corresponde a `tipo_popUp=77` y que su argumento es el RUT de la fila; si no, la lectura
  queda FALLIDA (`HISTORICO_LINK` / `HISTORICO_NO_VERIFICADA`), nunca se atribuye otra persona.
- 19-nov-2026 · **P4 hecho.** Asistente: `personal/biblioteca.py` (lista sin seguir enlaces, búsqueda sin tildes) y ventana
  **Resultados › «Biblioteca de productos…»** (`results_view.open_library`: abrir archivo o su carpeta). Meta 4:
  `meta4_bitacoras.validar_plantilla` (exige las cuatro hojas antes de generar) y menú **«Elegir plantilla Excel…» / «Usar
  diseño estándar»**, usada al generar y al regenerar. Contactos: la importación con conflictos de Python
  (`app_base._import_contacts`) ya equivale a la de Rust; sin cambios. Manuales del Descargador, del Asistente y LEEME de
  Meta 4 actualizados. Pruebas `tests/test_biblioteca_productos.py` y `test_p4_validar_plantilla_*` (Meta 4: mismos 3 fallos
  de entorno que la base, por el lector BIFF).
- 19-nov-2026 · **Resultado real del usuario** (solo diagnósticos anónimos; los JSON con nombres no se guardan): lote de 4
  tribunales, Cumplimiento modalidad 1, 18 registros: bitácoras 18/18 LEIDA; historia 17/17 causas; **Escritos funciona**
  (`CON_FILAS` 9, `VACIA` 8, 14 pendientes; sin `SIN_TABLA`); **Histórico 18/18 FALLIDA `HISTORICO_NO_VERIFICADA`**: la
  ventana `PopUpHistorial.jsp?tipo_popUp=77` respondió HTML pero sin la tabla «Ingreso Historico Protección» → el supuesto
  N2 sobre esa ventana no se cumple. Cambios: (1) tope de aperturas por lote 2000 → **5000** (bitácoras, calendarios,
  históricos e historias; pedido del usuario: «lo máximo razonablemente posible»); (2) **5 históricos seguidos sin la tabla
  suspenden el histórico** en el resto del lote (aviso; las bitácoras siguen); (3) la primera ventana no reconocida deja
  `historico/estructura_historico.json` anónimo (`extraer_estructura_escritos.estructura_ventana`; también por consola con
  `--ventana`). Pruebas `test_p3c_historico_sin_tabla_*`, `test_p3c_estructura_de_ventana_*` (sin datos de personas).
- 19-nov-2026 · El usuario envió una **captura de pantalla** de la ventana real (no la salida del extractor; contenía datos de
  una persona: no se guardó ni se transcribe). Hechos estructurales (N1): la URL `PopUpHistorial.jsp?tipo_popUp=77&RUT_Litigante=<sin
  puntos>&RUT_DV=<dv>` es la misma que arma `historico_open`; la ventana muestra bloque «Historial Menores» con rótulos, tabla
  LRPA de 25 columnas y tabla Protección de 16 columnas (`CausalIngreso1..3`, `EntidadAsigna1..3`, `Con Quien Egresa`), con
  barra de desplazamiento horizontal. Con esa forma visible, en ISO-8859-1, `read_historico` lee bien (prueba
  `test_historico_con_la_forma_visible_de_la_ventana_real_19nov`). N2: el HTML que recibe el Descargador difiere de lo
  renderizado (tabla cargada por script, encabezado y cuerpo en tablas separadas o similar). **Falta** `estructura_historico.json`.
- 19-nov-2026 · **Estructura anónima del usuario** (`--ventana` sobre un `historico-*.html` real): `PopUpHistorial.jsp` es un
  envoltorio de 675 bytes, título «Despliegue de Pop Up», sin tablas, con **un iframe a `/SITFAWEB/IrPopUpPublicoAccion.do`**.
  Causa de las 18 FALLIDA: se leía el envoltorio. Corrección: `historico_open` sigue ese único marco (mismo origen, ruta exacta,
  sin fragmento; dos marcos o ruta distinta = `HISTORICO_RESPONSE`) y devuelve su contenido; la pertenencia la decide el RUT
  rotulado visible («Rut : …»), ya no la dirección. Prueba Node ampliada (`test_extension_historico.py`).
- 19-nov-2026 · El usuario deja **pendiente la prueba real de P3c** y posterga P9, P6 y P7. Plan actualizado (§3b de
  `PLAN.md`). Se continúa con P8 y P5.
- 19-nov-2026 · **P8 hecho.** `flujo_paquete.anotar`: historial anónimo (últimos 200) en `respaldo/proceso.json` con inicio o
  reintento, fases, consultas reutilizadas, resumen de bitácoras e históricos, bitácoras no leídas por tribunal, error literal
  y estado final; rutas reemplazadas por «<ruta>». `lectura_bitacoras.faltantes_por_tribunal` / `aviso_faltantes`: el
  resultado del lote trae los faltantes por tribunal y el flujo los avisa al terminar. Manual del Descargador actualizado.
  Pruebas `test_p8_*` en `descargador/test_flujo_paquete.py`.
