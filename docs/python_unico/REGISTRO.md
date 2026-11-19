# Registro de trabajo — rama `flow-python`

**Actualizar al terminar CADA turno** (antes de quedarse sin tokens): qué se hizo, verificación, siguiente paso exacto.
Commit y push a `flow-python` en el mismo turno. Plan: [`PLAN.md`](PLAN.md).

## Estado al 19-nov-2026

| Paquete | Estado | Último commit / nota |
|---|---|---|
| P1 consignar (botón) | Pendiente | — |
| P2 revisión de bitácoras | Pendiente | — |
| P3a escritos visibles + diagnóstico | Pendiente | — |
| P3b escritos variante sin tabla | Bloqueado por P3a | — |
| P3c histórico automático | Pendiente de D-H1 (supuesto: sí, casilla desactivada) | — |
| P8 diagnóstico del paquete | Pendiente | Versión en `diagnostico_bitacoras.json` corregida en `99a2c11` |
| P4 resultados / Meta 4 plantilla | Pendiente | — |
| P5 diseño | Pendiente | — |
| P9 notificación y diligencias | Opcional | — |
| P6 `.eml`, P7 consola | Decisión del usuario | — |

## Siguiente paso exacto

P1: en `src/nurus/personal/record_view.py` / `work_view.py`, cuando la observación de la fila tenga la frase de
`HISTORIA.OTRO_PROGRAMA` (lista `current.otros_programas` del contexto, guardada por fila), mostrar un botón
«Marcar «<programa>» como consignada» que llame a `Work.consignar(row_id, programa)` y recalcule; prueba sin Tk sobre la
función que arma las acciones del detalle (patrón de `preparacion_desde_interfaz`, E3D-04).

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
