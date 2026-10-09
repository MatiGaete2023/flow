# Traspaso entre sesiones

Pedido por el usuario el 9 de octubre de 2026: qué se hizo, cómo, para qué y qué falta, para retomar en otra
sesión sin depender de la conversación. Se actualiza al cerrar cada WP. Estado fino: `ESTADO.json`; detalle por
paquete: `wp/<ID>.md`; verificación: `verificacion/<ID>.json`.

## Cómo se trabajó (desviaciones del plan, todas declaradas)

- **Rama:** el entorno de la sesión exige trabajar en `ccr-4116db41-nm8g0r` (no `wp/<ID>-<slug>`). Esa rama parte
  de `2765b47` (punta de `feat/ejecucion-plan-20261008`) y lleva **un commit por WP** con el prefijo `<ID>:`.
  Para integrar: PR de `ccr-4116db41-nm8g0r` → `feat/ejecucion-plan-20261008` (o cherry-pick por WP).
- **Roles:** un solo agente actuó como Orquestador y Ejecutor. Por la regla «nunca revisa ni audita un WP que él
  mismo ejecutó», ningún WP figura `INTEGRADO` salvo WP-00; quedan en `EN_REVISION` (o `EN_VERIFICACION`) a la espera
  de Revisor y Auditor independientes (prompts en `PLAN_IMPLEMENTACION.md`).
- **Verificación:** `tools/verificar_wp.py` sobre un worktree aislado del commit, contra `linea_base.json`
  (`2765b47`). Resultado copiado a `verificacion/<ID>.json`.
- **Entorno Linux reproducible:** `uv python install 3.12` (trae Tk), `.venv` con `requirements/linux-py312.lock`,
  `pip install --no-deps --no-build-isolation -e .`, `pip install ruff`, `xvfb-run`.
- **Hook local:** `git config core.hooksPath tools/hooks` (bloquea RUT con DV válido).

## Hecho (9 de octubre de 2026)

| WP | Qué | Para qué | Estado |
| --- | --- | --- | --- |
| 00 | Kit, `ESTADO.json`, línea base (605/11, 198/59, 18 IDs, 12 módulos, 26 enlaces) y etiqueta `pre-implementacion-20261009` | Medir todo contra un punto fijo | INTEGRADO |
| 01 | `PLAN_VIGENTE.md`, `AGENTS.md` (29 líneas), `CONTINUAR.md` (≈1 KB), `docs/HISTORIAL.md`, doc 2.6.0 | Retomar leyendo 3 archivos, no 233 KB | EN_REVISION |
| 10 | Dos identidades reales de NNA → «PERSONA FICTICIA UNO/DOS» | Restricción 5 (datos reales) | EN_REVISION |
| 11 | `check_real_ids --excluir`, hook pre-commit, paso 1 de ambos workflows | Impedir que vuelva a entrar un RUT real | EN_REVISION |
| 15 | Extractor HAR anónimo + prueba | Diagnosticar SITFA sin entregar HAR real | EN_REVISION |
| 16 | Anonimizador de Excel + sonda | Compartir casos de prueba sin datos reales | BLOQUEADO (botón) |
| 30 | CC único en `policy.MANDATORY_CC` + prueba | Evitar CC distintos entre pantalla y borrador | EN_REVISION |
| 31 | Versión única (0.5.0.dev5 / 2.7.6), extensión y CI derivadas | Saber qué build tiene la oficina | EN_VERIFICACION |
| 32 | Exportación sin residuos y sin fórmulas | Cerrar M3 y M4 de la auditoría | EN_VERIFICACION |
| 60 | 20 de 32 casos R-T con prueba (1 conflicto abierto C-08) | Probar reglas sin cambiarlas | TOMADO (WIP) |

## Bloqueos que necesitan al usuario

1. **Permiso de borrado.** El entorno de esta sesión denegó `git rm -r variantes ...` (clasificador de acciones
   destructivas). Bloquea WP-20 y, en cadena, WP-21, 22, 23, 24, 34 y la parte de borrado de WP-12. Opciones:
   autorizar la acción en la sesión, o ejecutar el usuario mismo (desde la punta de la rama):
   ```sh
   git tag pre-WP-20
   git rm -r variantes descargador/.github ORGANIZACION.md docs/ACEPTACION_CSMP_PERSONAL.md docs/IMPLEMENTACION.md \
     docs/VERIFICACION_PERSONAL.md docs/ESTADO_CONSOLIDADO_20260920.md docs/IMPLEMENTACION_PERSONAL.md \
     docs/PLAN_INTEGRAL_20261002.md docs/ESTADO_PLAN_INTEGRAL_20261005.md \
     "docs/plan_20261008/01_ESPECIFICACION_DESCARGADOR_RUS_SITFA (1).md" \
     "docs/plan_20261008/05_PLAN_COMPLETO_EJECUCION_NURUS_CSMP_20261008 (1).md"
   ```
   El resto de WP-20 (rutas `C:/Users/...`, README del plan, quitar `--excluir`) lo hace el agente.
2. **Decisiones D1–D9** (`PLAN_VIGENTE.md`): bloquean WP-02 (D2), 13/14 (D4), 17 (D1), 22 (D3), 33 (D8 y D4),
   45 (D9), 50 (D5), 51 (D6), 70 (D7).
3. **Conflicto abierto C-08** (sin columna FAE se afirma «no registra ficha»): decide el usuario; ver `ESTADO.json`.

## Siguiente acción

Seguir con los WP sin bloqueo en este orden: WP-35 (instaladores con lock), WP-40/41/42 (interfaz), WP-12 (contactos,
salvo el borrado del JSON real), resto de WP-60. Después, si hay permiso de borrado: WP-20 → 21 → 24 → 23 → 34.
