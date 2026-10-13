# Fixtures ficticios de la historia de causa

**Fieles a la estructura observada en H0** (`contracts/csmp_capture_v1/HISTORIA_VENTANA.md`, 13 de octubre de
2026): mismas etiquetas, clases, anidamiento, funciones, aridades, charset y la fila final incompleta. Todos los
**valores** son inventados (canarios: `NOMBRECANARIO`, `C-99999-2099`, `TOKENCANARIO`, `TOKENDOCCANARIO*`, fechas
del año 2099). No contienen datos reales.

- `ventana_historia.html`: ventana «Información de la Causa»: encabezado (`label.form-check-label`), botones,
  pestañas y la única tabla (`table table-sm table-bordered table-hover`) con 3 trámites completos y una última fila
  incompleta de 4 celdas, como la real.
- `listado_seguimiento.html`: fila del listado con `ShowHistoria(num,'RIT',num)` y `ShowObservaciones`, y la función
  `ShowHistoria` con el flujo `Encrypt_AjaxAccion.do` → `ConsultaDetalleAtPublicoAccion.do`.

WP-53 deriva de aquí las variantes (0/1/30 filas, texto largo, fecha ilegible, encabezado distinto, login) en sus
pruebas, modificando copias en memoria.
