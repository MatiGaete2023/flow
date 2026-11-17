# Auditoría de la rama completa · 17/11/2026

Rama `auditor/estudio-d-20261117`, sobre `feat/flow-estudio-d-20261117` (cbf645b).
No se tocaron motor, `textos_base.json`, Outlook, contratos, retención, extensión, golden ni línea base.

## Mediciones (Linux, Python 3.12, datos ficticios)

| Suite | Resultado |
|---|---|
| Rust `cargo test --workspace` | 315 aprobadas, 0 fallidas |
| `verificar_wp.py` con la lista integracion-20261021 | Asistente 1285, Descargador 382 (62 subpruebas), inventario, restricciones, ruff, identidades, módulos y enlaces PASA |
| anonimizador, META4_Bitacoras, descargador_lite (pytest) | 103, 132 y 26 aprobadas |
| Pruebas Node de la extensión (descargador y rust/pruebas) | 6 PASA |
| `rust/pruebas/anonimo_contrato.py` | PASA |
| Matriz de capturas `ui_evidencia.py --completa` | 176 PNG, código 0 |

## Corregido

| Hallazgo | Corrección y prueba |
|---|---|
| Paquete legado con bitácoras rotulado «evidencia contradictoria» | Cotejo solo para consultas V1; prueba `paquete_legado_con_bitacoras_explica_falta_de_lectura_y_no_contradiccion` |
| Reanalizar con el mismo criterio borraba selección y registro activo | `nueva` solo si cambia el criterio; prueba de UI |
| Restricción absoluta: `commands.py` importaba `subprocess` (control 5 FALLA) | Fuera de Windows se indica la ruta del manual; `tests/test_open_doc_sin_procesos.py` |
| `verificar_base_plan.py` informaba 48 bajas ya declaradas | Pasa `--lista-eliminadas integracion-20261021-eliminadas.txt` (48 nodeids exactos) |
| Filtros de Trabajo apilados verticalmente: 2-3 filas visibles a 1366×768 | Fila única `horizontal_wrapped`; captura comparada |
| Campos de texto indistinguibles del panel | Fondo propio y borde con contraste ≥ 3:1; prueba de contraste |
| Acciones Restaurar/Copiar ocultas bajo el texto en Revisión | Se muestran antes del texto; editor 150 px |
| «1 registros», «1 visibles», «1 contactos», búsquedas truncadas | Concordancia de número y anchos |

## Pendiente documentado (no corregido)

- Paridad golden FALLA solo en este entorno: zlib-ng 1.3.1 comprime distinto. El contenido descomprimido de
  `Origen_*.xlsx` es idéntico byte a byte; cambian `source_sha256` y los `record_id` derivados. No se regenera golden.
- H1: la captura habitual sigue siempre PARCIAL (`lecturas.rs` fija la cobertura). Requiere una señal de la fuente.
- La biblioteca de Resultados vuelve a explorar carpetas cada 2 s en el hilo de la interfaz; posible pausa en carpetas de red.
- El período sugerido al abrir un paquete une los intervalos de todos los ingresos.
- Ruff B023 (20 avisos): revisados; las clausuras se usan dentro de la misma iteración, sin defecto.
- Windows, Outlook y SITFA reales no se ejecutaron aquí.
