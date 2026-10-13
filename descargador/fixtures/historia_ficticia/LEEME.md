# Fixtures ficticios de la historia de causa

**Provisionales.** Reproducen solo lo visible en la ventana «Información de la Causa» (pestaña Historia) descrita por
el usuario el 13 de octubre de 2026. Todos los valores son inventados (canarios: `NOMBRECANARIO`, `C-99999-2099`,
`TOKENCANARIO`, fechas del año 2099). Los ids, clases y la forma exacta del HTML **no** son los de SITFA: se
reemplazan por la estructura real anónima que entregue la fase H0 (`docs/historia_causa/wp/WP-52.md`).

- `ventana_historia.html`: encabezado de causa, botones, pestañas y tabla de trámites.
- `listado_seguimiento.html`: fila de listado con `ShowObservaciones` y `ShowHistoria`, y una función
  `ShowHistoria` que arma una URL con parámetro opaco `a1` (supuesto a confirmar en H0).
