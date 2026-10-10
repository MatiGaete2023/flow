# Traspaso entre sesiones

Pedido por el usuario el 9 de octubre de 2026: qué se hizo, cómo, para qué y qué falta, para retomar en otra
sesión sin depender de la conversación. Estado fino: `ESTADO.json`; detalle por paquete: `wp/<ID>.md`;
verificación: `verificacion/<ID>.json`. Última actualización: 10 de octubre de 2026.

## Cómo retomar (5 minutos)

1. `git fetch origin ccr-4116db41-nm8g0r && git checkout ccr-4116db41-nm8g0r`.
2. Entorno Linux: `uv python install 3.12` (trae Tk); `$(uv python find 3.12) -m venv .venv && . .venv/bin/activate`;
   `pip install --require-hashes -r requirements/linux-py312.lock`; `pip install --no-deps --no-build-isolation -e .`;
   `pip install ruff`; `git config core.hooksPath tools/hooks`.
3. `python tools/verificar_wp.py` (≈2 min, usa `xvfb-run`): debe dar PASA contra `linea_base.json` (`2765b47`).
4. Leer «Bloqueos» y «Siguiente acción» abajo.

## Cómo se trabajó (desviaciones del plan, declaradas)

- **Rama:** el entorno exige `ccr-4116db41-nm8g0r` (no `wp/<ID>-<slug>`). Parte de `2765b47` (punta de
  `feat/ejecucion-plan-20261008`) y lleva un commit por WP con prefijo `<ID>:`. Integrar por PR hacia
  `feat/ejecucion-plan-20261008` (o cherry-pick por WP). No hay PR abierto todavía.
- **Roles:** un solo agente actuó como Orquestador y Ejecutor. Por la regla «nunca revisa ni audita su propio WP»,
  ningún WP salvo WP-00 está `INTEGRADO`: quedan `EN_REVISION` a la espera de Revisor y Auditor independientes
  (prompts listos en `PLAN_IMPLEMENTACION.md`).
- **Verificación:** `verificar_wp.py` en un worktree aislado de cada commit; todas las verificaciones registradas
  dieron PASA (WP-31 falló una vez y se corrigió; queda constancia en su commit y en `wp/WP-31.md`).
- **Fichas ampliadas por el Orquestador:** WP-16 (lógica a `src/nurus/personal/anonymize.py` para que el botón viaje
  en el EXE), WP-31 (`verify-delivered-installer.yml`), WP-30 → WP-12 (quitar `cc_obligatorio` solo al regenerar
  golden). Dependencias no integradas se saltaron solo cuando el archivo tocado sobrevive al WP previo.

## Hecho

| WP | Qué | Para qué | Estado |
| --- | --- | --- | --- |
| 00 | Kit, `ESTADO.json`, línea base, etiqueta `pre-implementacion-20261009` | Medir todo contra un punto fijo | INTEGRADO |
| 01 | `PLAN_VIGENTE.md`, `AGENTS.md`, `CONTINUAR.md` breve, `docs/HISTORIAL.md`, doc 2.6.0 | Retomar leyendo 3 archivos | EN_REVISION |
| 10 | Identidades reales de NNA → ficticias en pruebas | Restricción 5 | EN_REVISION |
| 11 | `check_real_ids --excluir`, hook pre-commit, paso 1 de CI | Que no vuelva a entrar un RUT real | EN_REVISION |
| 12 | Contactos reales y 14 correos `@pjud.cl` fuera del repo; `contactos.json` local; sesiones sin `config` | Restricción 5 y C2 | EN_REVISION |
| 15 | Extractor HAR anónimo + prueba | Diagnóstico sin HAR real | EN_REVISION |
| 16 | Anonimizador de Excel + botón en Configuración | Compartir casos sin datos reales | EN_REVISION* |
| 24 | Históricos movidos, README breves, índice en tabla | Documentación legible | EN_REVISION |
| 30 | CC único en `policy.MANDATORY_CC` | Evitar CC distintos | EN_REVISION |
| 31 | Versión única (0.5.0.dev5 / 2.7.6) y CI derivada | Saber qué build tiene la oficina | EN_REVISION |
| 32 | Exportación sin residuos ni fórmulas | M3 y M4 | EN_REVISION |
| 35 | Instaladores con Python 3.12 y lock con hashes | Instalar lo que CI probó | EN_REVISION* |
| 40 | Trabajo: orden de columnas, texto completo, editor plegado, excluidos | Legibilidad | EN_REVISION |
| 41 | Barra «Siguiente paso» con conteos | Saber en qué punto va el trabajo | EN_REVISION* |
| 42 | Resoluciones en 3 grupos + «Más»; motivos de botones en Correos | Claridad | EN_REVISION |
| 60 | 32/32 casos R-T con prueba; 4 conflictos abiertos | Probar reglas sin cambiarlas | EN_REVISION |

(*) cubiertos por verificaciones acumuladas posteriores (mismo árbol).

Métricas (N1, 10 de octubre de 2026): suite Asistente 667 aprobadas/11 omitidas/5 xfail; Descargador 200/59;
identidades reales 12 (todas en `variantes/`); paridad golden PASA; restricciones sin infracciones; enlaces rotos 13.

## Pendiente fuera de la CI de esta sesión

- CI Windows real (requiere PR o rama `feat/**`): ensayo de WP-11 (RUT válido hace fallar), nombres de artefactos
  (WP-31), `cmd.exe` real del instalador (WP-35). Capturas de interfaz para los PR de WP-40/41/42.

## Bloqueos que necesitan al usuario

1. **Permiso de borrado masivo.** El entorno denegó `git rm -r variantes ...`. Bloquea WP-20 y en cadena WP-21,
   22, 23, 34 (y 17). Opciones: autorizar la acción en la sesión, o ejecutarlo el usuario (desde la punta de la rama):
   ```sh
   git tag pre-WP-20
   git rm -r variantes descargador/.github ORGANIZACION.md docs/ACEPTACION_CSMP_PERSONAL.md docs/IMPLEMENTACION.md \
     docs/VERIFICACION_PERSONAL.md docs/historico/ESTADO_CONSOLIDADO_20260920.md docs/IMPLEMENTACION_PERSONAL.md \
     docs/PLAN_INTEGRAL_20261002.md docs/ESTADO_PLAN_INTEGRAL_20261005.md \
     "docs/plan_20261008/01_ESPECIFICACION_DESCARGADOR_RUS_SITFA (1).md" \
     "docs/plan_20261008/05_PLAN_COMPLETO_EJECUCION_NURUS_CSMP_20261008 (1).md"
   ```
   Después el agente quita `--excluir variantes/` (CI y hook) y actualiza el README del plan y los hashes.
2. **Decisiones D1–D9** (`PLAN_VIGENTE.md`): WP-02 (D2), 13/14 (D4), 17 (D1), 22 (D3), 33 (D8, D4), 45 (D9),
   50 (D5), 51 (D6), 70 (D7).
3. **Conflictos abiertos** (`ESTADO.json → conflictos_abiertos`): C-08 sin columna FAE; R-T19 sin advertencia;
   R-T24–R-T26 fuera de alcance. Ninguna regla se cambió.
4. **Enlaces de copias de procedencia** (`docs/README_NURUS_BASELINE.md`, `docs/README_NURUS_DEV7.md`): moverlas a
   `historico/` o aceptarlas como excepción.

## Siguiente acción

Con permiso de borrado: WP-20 → WP-21 → WP-23 (requiere D3 por WP-22) → WP-34. Sin permiso: abrir PR de
`ccr-4116db41-nm8g0r` hacia `feat/ejecucion-plan-20261008` para que la CI Windows y un Revisor independiente
validen los WP en `EN_REVISION`. WP-61 (formato) va al final.
