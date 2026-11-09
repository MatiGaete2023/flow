# Auditoría de código — rama Rediseno, 9 de noviembre de 2026

Alcance: los cinco productos (Asistente CSMP, Descargador SITFA, SITFA Lite, Meta 4, Anonimizador), el kit común
`ui/flowkit.py`, las herramientas de manuales y los workflows. Hecha por la misma sesión que escribió el rediseño:
es revisión del Ejecutor, no la E3 independiente. Nada se declara aceptado.

## Método

- Análisis estático: Ruff (F, B, SIM, RUF100, ARG, ERA) y Vulture sobre todo el árbol; cruce de cada función y constante
  de las ventanas contra sus usos en producto, pruebas y herramientas.
- Lectura completa de `ui/flowkit.py`, `descargador_lite/app/sitfa_lite.py`, `META4_Bitacoras/Meta4.py` y
  `anonimizador/anonimizador_csmp/gui.py`; lectura dirigida del resto.
- Pruebas antes y después de cada cambio.

## Hallazgos y corrección

| Id | Dónde | Tipo | Hallazgo | Corrección |
| --- | --- | --- | --- | --- |
| A-1 | SITFA Lite | Error | «Abrir carpeta del último lote» no hacía nada sin lote en la sesión, y fallaba en silencio si la carpeta ya no existía. | Abre la carpeta de descarga; si no existe, avisa. |
| A-2 | SITFA Lite | Error | Si un lote terminaba sin evento final, el pie quedaba en «Deteniendo…». | El pie pasa a «Detenido». |
| A-3 | SITFA Lite | Omisión | «Versión 1.1.0» del menú parecía una acción y no hacía nada. | Queda como texto deshabilitado. |
| A-4 | Meta 4 | Error | Con una generación en curso, el menú «Más» permitía agregar, quitar o vaciar lotes y cambiar bitácoras: la lista dejaba de corresponder a lo que se estaba generando. | Esas acciones se ignoran mientras genera. |
| A-5 | Meta 4 | Error | «Abrir resultados» fallaba en silencio si la carpeta se había movido o borrado. | Avisa «La carpeta de resultados ya no existe». |
| A-6 | Anonimizador | Error | Igual que A-4: agregar o quitar entradas desde «Más» durante el proceso reiniciaba la lista de avance. | Se ignoran mientras procesa. |
| A-7 | Anonimizador | Error | «Abrir carpeta» fallaba en silencio si la carpeta ya no existía. | No intenta abrirla. |
| A-8 | Workflows | Omisión | `meta4-exe.yml` compilaba Meta 4 con cualquier push a Rediseno, aunque no cambiara Meta 4. | Filtro de rutas `META4_Bitacoras/**`. |
| A-9 | Manuales Lite | Omisión | El paso que recaptura las páginas de Chrome en español no estaba en el repositorio; el README describía un flujo que ya no existe. | `capturas_chrome_contenido.js` y README al día. |
| A-10 | Descargador y Lite | Código sin uso | `quick_dates` (los botones de período rápido se retiraron) y, en el Lite, `date_preset`, `change_period`, `screen_radios`, `planned`, `pane`, `range`, `date_widgets`. | Eliminados; la prueba y el capturador usan `refresh` y las fechas directas. |
| A-11 | SITFA Lite `motor.py` | Código sin uso | `PAGE_FIELD`, `TOTAL_FIELD`, `TARGET`, `LAJA_FAE`, `LAJA_ESPERA`: heredados del Descargador, sin uso en el Lite. | Eliminados (en el Descargador los usan sus pruebas: se conservan). |
| A-12 | Manuales Lite | Código sin uso y roto | `video.py`: `main`, `complete` y `encode` (videos sin voz) apuntaban a recuadros que ya no existen; `capturas_programa.py` duplicaba a `tools/manuales/capturas.py lite`. | Eliminados; `video.py` queda como biblioteca de escenas. |
| A-13 | Varios | Código sin uso | `Banner.parent_manager` (kit), expresión `IDENT` (extractor de historia), clave repetida `history_complete`, import y f-string sin uso en dos pruebas. | Eliminados. |

## Revisado y dejado como está

- `from tkinter import messagebox` en `Meta4.py`: la ventana no lo usa, pero las pruebas lo interceptan para comprobar
  que no aparece ningún diálogo.
- `Meta4.App.lots_info`: solo lo usa una prueba.
- 20 avisos B023 (función dentro de un bucle que usa la variable del bucle): en todos la función se ejecuta dentro de la
  misma vuelta; no hay defecto.
- 40 avisos RUF100: son `noqa` de reglas que la configuración no activa; inofensivos.
- Hallazgos de Vulture sobre atributos COM de Office, métodos `do_GET`/`do_POST` y campos de modelos: falsos positivos.
- `src/nurus/personal/motor/` (reglas y umbrales): sin cambios, por regla del proyecto.
- `descargador_lite/docs/EVALUACION.md`: documento de diseño sin enlaces; no va en el instalador.

## Verificación

| Suite | Resultado |
| --- | --- |
| Asistente (`tests/`) | 1.257 aprobadas, 11 omitidas, 5 xfail |
| Descargador | 376 aprobadas |
| SITFA Lite | 20 aprobadas |
| Meta 4 | 131 aprobadas |
| Anonimizador | 100 aprobadas, 1 omitida |
| Ruff (E9, F63, F7, F82, F401, F841), enlaces, copias del kit | PASA |
| Capturador del Lite tras quitar `quick_dates` | genera las 7 capturas |

## Pendiente

- El instalador del Descargador no se ha recompilado desde el cambio de texto «Cargar extensión sin empaquetar».
- E3 independiente y prueba con SITFA real.
- Propuestas de diseño y experiencia: `docs/diseno/MEJORAS_UX_20261109.md`.
