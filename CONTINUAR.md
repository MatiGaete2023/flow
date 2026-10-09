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
