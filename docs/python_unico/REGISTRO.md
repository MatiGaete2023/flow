# Registro de trabajo — rama `flow-python`

**Actualizar al terminar CADA turno** (antes de quedarse sin tokens): qué se hizo, verificación, siguiente paso exacto.
Commit y push a `flow-python` en el mismo turno. Plan: [`PLAN.md`](PLAN.md).

## Estado al 19-nov-2026

| Paquete | Estado | Último commit / nota |
|---|---|---|
| P1 consignar (botón) | **Hecho** | ver Historial 19-nov (P1) |
| P2 revisión de bitácoras | Pendiente | — |
| P3a escritos visibles + diagnóstico | Pendiente | — |
| P3b escritos variante sin tabla | Bloqueado por P3a | — |
| P3c histórico automático | Pendiente; **D-H1 = sí** (casilla desactivada por defecto) | — |
| P8 diagnóstico del paquete | Pendiente | Versión en `diagnostico_bitacoras.json` corregida en `99a2c11` |
| P4 resultados / Meta 4 plantilla | Pendiente | — |
| P5 diseño | Pendiente | — |
| P9 notificación y diligencias | Opcional | — |
| P6 `.eml`, P7 consola | Decisión del usuario | — |

## Siguiente paso exacto

P2 (a) — evidencia de alcance de cada bitácora en el Descargador Python:
1. En `descargador/lectura_bitacoras.py`, donde se valida y guarda una bitácora LEIDA (`leer_bitacora`, ~línea 661), calcular
   `alcance` = `VACIA_COMPROBADA` (tabla reconocida sin filas), `COMPLETA` (tabla reconocida con filas) o `PARCIAL` (algún
   control que sugiera recorte). Portar la detección de `rust/crates/flow-captura/src/lecturas.rs::cobertura_bitacora`
   (nombres `pagina/nropag/numpag/pagenum/fechadesde/fechahasta`, rótulos «siguiente/anterior/proxima/ultima pagina/mostrar
   mas/cargar mas/ver mas», paginadores numéricos o flechas solo en elementos pulsables, iframes/frames/.pagination).
2. Guardarlo en el registro de `bitacoras.json` (`alcance`, `metodo: SITFA_TABLA_INFORME_INGRESO_V1`) y llevarlo al paquete en
   `descargador/export_csmp_capture.py` como `filters_json.revision_bitacoras` de la consulta `diary-*` (ver
   `rust/REVISION_BITACORAS_ESQUEMA.md`; mismo nombre de clave, aditivo).
3. Pruebas: ventana ficticia vacía, con evento, paginada y sin tabla (mismos casos que `rust/crates/flow/src/ui.rs`
   `ventana_completa_de_sitfa_acredita_ausencia_y_activa_el_motor`).
Luego P2 (b): pantalla del Asistente.

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
