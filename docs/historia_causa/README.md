# Historia de la causa en el Descargador — rama base

Rama `feat/historia-causa-sitfa` (desde `feat/ejecucion-plan-20261008` @ `1a2dd0f`, 13 de octubre de 2026).
Objetivo: que el Descargador lea, además de las bitácoras, la **tabla de la pestaña Historia** de la ventana
«Información de la Causa», solo en la **ventana de 4 meses**, para revisión en Excel y para que el Asistente la
analice **en otro paso**. Sin PDF. Uso personal en Windows. 100 % local.

## Estado

| Elemento | Estado |
|---|---|
| Factibilidad | Favorable. **H0 cerrado**: acceso (escenario B) y ventana caracterizados en [HISTORIA_VENTANA.md](../../contracts/csmp_capture_v1/HISTORIA_VENTANA.md). |
| Decisiones | [00_DECISIONES.md](00_DECISIONES.md) (tomadas H-a…H-h; pendientes P1…P8). |
| Entregado | `descargador/extraer_estructura_historia.py` (H0) + pruebas + fixtures ficticios; `tools/ci_local.sh`. |
| Bloqueo | Correctivos E3 aplicados sobre `9310d60`; verificación final en curso y nueva revisión independiente pendiente. Código WP-53/WP-54 completo; ver [AVANCE.md](AVANCE.md). |

## Qué leer (según el rol; nada más)

| Rol | Archivos |
|---|---|
| Usuario | Este README · [05_GUIA_USUARIO_H0.md](05_GUIA_USUARIO_H0.md) · [04 §2–3](04_PRUEBAS_Y_ACEPTACION.md) |
| Ejecutor (IA o desarrollador) | **Solo [GUIA_IMPLEMENTACION.md](GUIA_IMPLEMENTACION.md)** (autosuficiente; manda sobre las fichas de `wp/`) |
| Revisor E3 | [GUIA_IMPLEMENTACION.md](GUIA_IMPLEMENTACION.md) §3, §5, §12 · el diff |
| Contexto (opcional) | [referencia/](referencia/ANALISIS_CONSOLIDADO.md): análisis de ambos informes e informe previo |

Paquetes: [WP-52](wp/WP-52.md) H0 y decisión → [WP-53](wp/WP-53.md) núcleo (única revisión E3) →
[WP-54](wp/WP-54.md) interfaz, entrega y acta. Prompts listos: [wp/PROMPTS.md](wp/PROMPTS.md).

## Flujo eficiente (programación, tokens y GitHub Actions)

**Programación**
- Una sola pasada por el listado: la historia se lee junto con las bitácoras; cero consultas extra del listado a SITFA.
- 3 WP en lugar de 5; todo lo riesgoso (extensión, contrato, retención) en WP-53 ⇒ **una** revisión E3.
- Sin visor, sin botones nuevos, sin reutilización entre sesiones; el reintento reutiliza el flujo actual (EF-04).
- Prueba primero con fixtures fieles; nada se programa contra supuestos (H0 antes que código).

**Tokens (agentes)**
- Cada ejecutor lee solo su ficha (1–2 páginas) con anclas `archivo:línea`; no lee `ESTADO.json` entero
  (`grep -n '"WP-5' docs/implementacion/ESTADO.json`) ni los planes antiguos.
- Mientras desarrolla: `tools/ci_local.sh <pruebas del subpaso>` y `pytest -q` (salidas cortas). La suite completa
  y `verificar_wp.py` una vez por WP.
- Modelo capaz para analizar H0 (WP-52) y para la revisión E3; el resto puede hacerlo un modelo menor, porque las
  fichas son cerradas y las pruebas deciden.
- Un mensaje por WP con el prompt de `wp/PROMPTS.md`; no repetir contexto que ya está en las fichas.

**GitHub Actions**
- Trabajar **directo** en `feat/historia-causa-sitfa`, sin PR por WP (decisión n del 13-oct: una rama). Cada push
  corre solo «CI rápida Linux»; las suites Windows corren solo en PR o a pedido.
- `tools/ci_local.sh` antes de cada push (mismos controles que la CI: identidades, estado, enlaces, ruff check,
  **ruff format** y pruebas). Un push rojo cuesta una corrida completa.
- Agrupar commits y hacer un push por cada 2–3 subpasos; commits solo de documentos con `[skip ci]`.
- Windows **una vez**, al final de WP-54: un único PR hacia `feat/ejecucion-plan-20261008` y un único
  `workflow_dispatch` para el instalador.
- La condición de «CI rápida Linux» no se cambia (la revisión E3 del 13-oct mostró que omitir PR `feat→feat` dejaba
  un hueco); el único PR final sí corre la verificación.

## Reglas (mandan `AGENTS.md` y `PLAN_VIGENTE.md`)

- Sin datos reales en repo, CI ni conversaciones: solo `estructura_historia.json` y fixtures ficticios.
- Sin escritura en RUS/SATURNO; sin PDF; sin seguir enlaces de documentos; sin IA en el producto.
- Motor, textos, Outlook, reglas CT y productos del Asistente **intactos**.
- Escenario D en H0 (solo con clic real) ⇒ detener el plan e informar.
