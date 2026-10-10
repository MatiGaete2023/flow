# Continuar

**Estado:** recuperación del plan del 9 de octubre de 2026 desde `2765b47`.
**Rama base:** `feat/ejecucion-plan-20261008`. Cada WP usa una rama propia;
solo el Orquestador integra después de verificación, revisión y auditoría.

## Siguiente acción

Leer `PLAN_VIGENTE.md`, `docs/implementacion/ESTADO.json` y la ficha del primer
WP en estado `LISTO` en `docs/implementacion/PLAN_IMPLEMENTACION.md`.
El informe de recuperación y los pasos pendientes están en
`docs/implementacion/wp/WP-01.md`; el estado compartido es la fuente vigente.
D3 permite retirar inspector y diario RUS; D4 fija 60 días y hasta 10 sesiones
anteriores; D8 no informó fallos. Las demás decisiones siguen pendientes.

## Traspaso de la sesión claude-sesion-012yjVgL (10 de octubre de 2026)

**Qué se hizo:**
- Ejecuté como Ejecutor una pila de PR en borrador. Cada PR apunta a la rama del anterior porque todos editan
  `ESTADO.json`.
- Orden de integración (lo hace solo el Orquestador, con squash):

  | PR | WP | Rama o tema |
  | --- | --- | --- |
  | #12 | WP-20 | `ccr-4116db41-nm8g0r` |
  | #13 | WP-21 | — |
  | #14 | WP-22 | — |
  | #15 | WP-23 | — |
  | #16 | WP-30 | — |
  | #17 | WP-31 | — |
  | #18 | WP-32 | — |
  | #19 | WP-35 | — |
  | #20 | WP-24 | — |
  | #21 | WP-34 | CI |
  | #22 | WP-16 | anonimizador |
  | #23 | WP-40 | — |
  | #24 | WP-41 | — |
  | #25 | WP-42 | — |

**Cómo:**
- Una rama `wp/<ID>-<slug>` por WP.
- `python tools/verificar_wp.py --pruebas-eliminadas-asistente 143`: PASA en cada WP. Las 143 pruebas retiradas
  corresponden a WP-21, WP-22 y WP-23.
- Ficha en `docs/implementacion/wp/WP-XX.md` y verificación en `docs/implementacion/verificacion/WP-XX.json`.
- Todos los WP quedan en `EN_VERIFICACION`: no revisé ni audité mis propios WP.

**Para qué:** ejecutar las olas 2 a 4 del plan sin tocar el motor ni los textos tradicionales.

**Revisión del 10 de octubre (misma sesión).**
- Cinco revisores independientes encontraron errores; todos quedaron corregidos en sus ramas.
- Nuevos PR independientes o declarados:

  | PR | Contenido | Base |
  | --- | --- | --- |
  | #26 | WP-33 | `feat` |
  | #27 | plan, reporte y herramientas | `feat` |
  | #28 | WP-43 | `feat` |
  | #29 | WP-62 | sobre #26 |
  | #30 | WP-44 | sobre #25 |

- Hallazgos, pasos y enmiendas E1–E6:
  [REPORTE_REVISION_20261010.md](docs/implementacion/REPORTE_REVISION_20261010.md) (rama
  `plan/revision-20261010`).
- Desde las 03:24 UTC, Actions no asigna runners: revisar *Settings › Billing › Actions*.

**Falta:**
- Revisión, auditoría e integración de #12 a #25, en ese orden.
- WP-33, WP-43, WP-44 y WP-62: esperan WP-14 (Codex, PR #4).
- Decisiones del usuario: WP-45 (D9), WP-50 (D5), WP-51 (D6), WP-02 (D2), WP-17 (D1) y WP-70 (D7).
- WP-60 y WP-15: Codex.
- WP-61: al final.
- `conflictos_abiertos` de ESTADO: `cc_obligatorio` frente a golden y 4 F401 en el motor.
- Probar el anonimizador con un `.xls` real en el PC autorizado.

## Enlaces

- [Plan vigente](PLAN_VIGENTE.md) · [Instrucciones](AGENTS.md)
- [Plan de implementación](docs/implementacion/PLAN_IMPLEMENTACION.md)
- [Recuperación](docs/implementacion/wp/WP-01.md)
- [Historial anterior](docs/HISTORIAL.md)

## Comandos

```sh
python -m pip install --require-hashes -r requirements/linux-py312.lock
python -m pip install --no-deps --no-build-isolation -e .
python tools/verificar_wp.py
```

## Migración de contactos antes de actualizar

Copia tu `contactos_base.json` de la instalación anterior a
`%LOCALAPPDATA%\CSMP_Personal\contactos.json` antes de reemplazarla.
Si no existe esa copia, el primer inicio abre «Importar contactos» para elegir
el JSON anterior o el catastro Excel; mientras tanto se usa el ejemplo ficticio.

## Control de identidades

Activa el control antes de cada commit con:

```sh
git config core.hooksPath tools/hooks
```

El hook revisa el contenido del índice, incluidos los XML de Excel y Word;
no lee las modificaciones que aún no agregaste. Ambos workflows ejecutan el
control antes de instalar dependencias o producir artefactos, sobre todo el
árbol: WP-20 retiró `variantes/` y las rutas personales (recuperables desde `pre-WP-20`).
