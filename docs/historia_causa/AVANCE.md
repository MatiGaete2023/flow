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

1. Nueva **E3 independiente** del diff corregido. El ejecutor no puede aprobar su propia revisión.
2. Windows candidato e instalador: Git funciona, pero la API de GitHub responde `Forbidden` tanto a GraphQL
   como a REST. No se afirma que ningún workflow remoto se haya ejecutado o aprobado.
3. Prueba real y acta del usuario: [guía](../aceptacion/GUIA_PRUEBAS_HISTORIA_20261013.md).
   No fusionar ni publicar release antes de la autorización correspondiente.
