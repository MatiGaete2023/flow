# AVANCE — traspaso entre IAs (WP-53/WP-54)

Fuente del procedimiento: [GUIA_IMPLEMENTACION.md](GUIA_IMPLEMENTACION.md).
Rama: `feat/historia-causa-sitfa`; correctivos publicados en `2af78a5`. PR existente: https://github.com/MatiGaete2023/flow/pull/45.

## Correcciones E3 — 13 de octubre de 2026

Veredicto recibido: **CAMBIOS**, sobre `9310d60`, contra `origin/feat/ejecucion-plan-20261008`.
El directorio `/tmp/e3-review` del revisor no existe en esta máquina. Las regresiones añadidas al repositorio
usan exclusivamente datos ficticios y comprueban los escenarios del informe.

| Hallazgo | Corrección y regresión |
|---|---|
| Privacidad del extractor | Listas cerradas de nombres/rutas/tipos; léxico compartido; excluye strings, backticks, comentarios y regex; sin exención por forma de código. |
| Cifrado reflejado | La extensión rechaza valores cifrados reflejados, incluidos valores ocultos y variantes URL/HTML, antes de devolver `body`. |
| Reutilización ajena al lote | Consulta nueva siempre fresca; solo el reintento explícito del lote elegido permite conservar causas verificadas que reaparecen. |
| Historia afecta bitácoras | Fronteras abrir/página/cerrar aisladas, incluyendo persistencia y error del propio aviso. |
| Cobertura del ZIP | Consultas/incidencias de enumeración fallida participan en el agregado, también sin causas leídas. |
| Identidad insuficiente | Llamadas reales; índices bidireccionales globales por tribunal; cotejo de RIT y tribunal visibles antes de guardar. |
| Retención mediante enlaces | Rechaza enlaces y junctions en todos los componentes; revalida cada lote antes de actuar. |
| Diagnóstico no anónimo | Códigos cerrados y mensajes constantes; sin texto de excepciones ni rutas de usuario. |
| Límites y CI | Esquema JSON exacto; streaming de 4096 bytes y 8 MiB; `test_historia_extension.cjs` en CI rápida y CI local. |
| Excel y fechas | Textos íntegros en columnas de continuación, conservando cuatro hojas; incidencia `HISTORIA_FECHA_POSTERIOR`. |

WP-53.1–53.7 y WP-54 interfaz/flujo/documentación están implementados; la nueva E3 y la aceptación siguen pendientes.
Las reglas del motor, textos y efectos Outlook permanecen fuera del cambio.

## Segunda E3 (APROBADA, sin bloqueantes) y ajustes — 13 de octubre de 2026

Revisó `ea3005e`. Riesgos mayores (solo confirmables con SITFA real) y menores; estado de cada uno:

| # | Hallazgo | Estado |
|---|---|---|
| 1 | Valores cifrados reflejados en la ventana real → rechazo | Código propio `HISTORY_REFLECTED` (extensión, puente, conexion.js, lector) y documentado en la guía de prueba. Enmascarar en vez de rechazar: **solo si la prueba real lo confirma**. |
| 2 | Tribunal visible ≠ nombre del menú → `HISTORIA_TRIBUNAL_VISIBLE` en todas | Documentado en la guía de prueba. Comparación más tolerante: **solo si la prueba real lo confirma**. |
| 3 | Error inesperado → sin Excel/diagnóstico/manifiesto en curso | **No reproducible**: el `finally` ya cierra historia y bitácoras. Se agregó prueba de regresión (`test_e3_error_inesperado_en_el_lote_...`). |
| 4 | Reintento desde Avanzado abre historias con la opción apagada | Corregido (`descargador.py`, exige `leer_historia`) + prueba. |
| 5 | «Conservar» no protegía Excel/diagnóstico de historia | Corregido (`preferencias._subir_historia`, `_es_preservable`) + prueba. |
| 6 | Causa incierta conserva conteos | Corregido: se borran conteos/fechas/advertencias al pasar a IDENTIDAD_INCIERTA + prueba. |
| 7 | Se envía el RIT de la celda, no el del enlace | Corregido: `rit_enlace` desde `vinculos.extract_historia`; la extensión admite letras en minúscula + pruebas Python y `.cjs`. |
| 8 | Paginación no detectada | Advertencia `HISTORIA_POSIBLE_PAGINACION` (clase/campo/enlace «Siguiente», fuera de scripts) visible en Causas e incidencias; la causa sigue LEIDA. Documentada en la guía. |
| 9 | `descargador-integral.yml` no corre `test_historia_extension.cjs` en Windows | **PENDIENTE: toca un workflow; requiere autorización del usuario.** |

Verificación local tras estos cambios: `tools/ci_local.sh` completo PASA (963 + 342 pruebas, ambas `.cjs`).

## Verificación

- Regresiones focales y `tools/ci_local.sh` focal: PASA; ambas pruebas `.cjs`: PASA.
- Retención gráfica: 20 PASA con Xvfb; ambos ZIP de contrato válidos.
- Primera medición completa: 958 PASA, 11 omitidas, 6 xfail y 2 fallos en Asistente; 11 fallos en Descargador.
  Los 13 fallos fueron por intentar guardar preferencias fuera del área escribible. Los 13 pasan con
  `LOCALAPPDATA=/workspace/cloud-tools/appdata`. Medición inicial no certificada por árbol modificado.
- Repetición final de `verificar_wp.py` sobre commit limpio `2af78a5`: **PASA**.
  Asistente: **960 PASA**, 11 omitidas, 6 xfail conocidos; Descargador: **338 PASA** y 62 subpruebas;
  **cero fallos**. Inventario sin bajas, identidades cero, paridad golden, restricciones, Ruff, módulos y enlaces PASA.
  Evidencia: [WP-53.json](../implementacion/verificacion/WP-53.json).
- CI local focal final: **89 PASA** y controles PASA; ambas pruebas `.cjs` PASA.

Entorno de esta máquina: activar `/workspace/flow-venv/bin` y `/workspace/cloud-tools/root/usr/bin` en `PATH`;
usar `LOCALAPPDATA=/workspace/cloud-tools/appdata`. `tools/ci_local.sh` elige `xvfb-run` automáticamente sin pantalla.
No crear worktrees: cada tarea de nube ya está aislada.

## Pendiente

1. ~~E3~~ APROBADA (sin bloqueantes) sobre `ea3005e`; los ajustes posteriores (tabla anterior) no tienen E3 propia, son menores y con prueba.
2. ~~Windows~~ HECHO el 13-oct: «CSMP Assistant Windows» por `workflow_dispatch` sobre `ea3005e` (run 38476339683,
   https://github.com/MatiGaete2023/flow/actions/runs/38476339683): **éxito** en windows-latest con Python 3.12, 3.13 y 3.14.
   «CI rápida Linux» sobre `2af78a5` (código): éxito. Este run no incluye el instalador firmado ni lo prueba el usuario.
3. Prueba real y acta del usuario: [guía](../aceptacion/GUIA_PRUEBAS_HISTORIA_20261013.md).
   No fusionar ni publicar release antes de la autorización correspondiente.
