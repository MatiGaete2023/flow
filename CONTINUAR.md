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
