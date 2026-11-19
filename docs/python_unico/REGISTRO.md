# Registro de trabajo — rama `flow-python`

**Actualizar al terminar CADA turno** (antes de quedarse sin tokens): qué se hizo, verificación, siguiente paso exacto.
Commit y push a `flow-python` en el mismo turno. Plan: [`PLAN.md`](PLAN.md).

## Estado al 19-nov-2026

| Paquete | Estado | Último commit / nota |
|---|---|---|
| P1 consignar (botón) | **Hecho** | ver Historial 19-nov (P1) |
| P2 revisión de bitácoras | **Hecho** (sin prueba visual Tk) | ver Historial 19-nov (P2) |
| P3a escritos visibles + diagnóstico | Pendiente | — |
| P3b escritos variante sin tabla | Bloqueado por P3a | — |
| P3c histórico automático | Pendiente; **D-H1 = sí** (casilla desactivada por defecto) | — |
| P8 diagnóstico del paquete | Pendiente | Versión en `diagnostico_bitacoras.json` corregida en `99a2c11` |
| P4 resultados / Meta 4 plantilla | Pendiente | — |
| P5 diseño | Pendiente | — |
| P9 notificación y diligencias | Opcional | — |
| P6 `.eml`, P7 consola | Decisión del usuario | — |

## Siguiente paso exacto

P3a — Escritos visibles y diagnóstico:
1. Descargador (`descargador/lectura_historia.py`): por cada causa leída, contar de forma anónima si la respuesta guardada
   traía la tabla Escritos (usar `nurus.escritos_html.read_escritos` sobre el HTML ya guardado: tabla con filas, tabla vacía,
   pestaña sin tabla = `ESCRITOS_NO_VERIFICADA`, sin pestaña = `ESCRITOS_AUSENTE`) y agregar en `diagnostico_historia.json`
   `escritos: {con_filas, vacia, sin_tabla, ausente, pendientes}` (solo conteos).
2. Asistente: columna «Escritos» en Trabajo (`work_view.py::WORK_COLUMNS` y su render) con «N pendientes», «sin pendientes»,
   «no verificada» o vacío si la fuente no vino; el dato sale de la fila (guardar en `Row` un resumen como en P2:
   `capture_input.py`, `escritos_by_record`).
3. Pruebas con `tests/fixtures_integracion/escritos_ficticio.html` y una variante sin tabla.
Luego P3c (Histórico automático, E3) y P8.

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
