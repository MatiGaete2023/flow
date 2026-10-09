# Continuar

**Estado:** implementación del plan del 9 de octubre de 2026 en curso.
**Rama de trabajo de la sesión:** `ccr-4116db41-nm8g0r` (parte de `2765b47`, punta de
`feat/ejecucion-plan-20261008`). Último código compilado y verificado en Windows: `0795cda`
(Descargador 2.7.5, Asistente 0.5.0.dev4).

## Siguiente acción única

Leer `docs/implementacion/TRASPASO.md` (qué se hizo, cómo, para qué y qué falta) y
continuar con el primer WP en estado `LISTO` de `docs/implementacion/ESTADO.json`.
Las decisiones D1–D9 que bloquean WP están listadas en `PLAN_VIGENTE.md`.

## Migración de contactos (WP-12)

El repositorio ya no trae el directorio real de contactos ni los correos de tribunales. Un perfil existente conserva
los suyos en `configuracion.json`. En un perfil nuevo: Configuración → Contactos → **Importar contactos** y elegir
el `contactos_base.json` de la instalación anterior (o el catastro Excel); queda copia en
`%LOCALAPPDATA%\CSMP_Personal_Prototipo_Integral\contactos.json` (entradas `Tribunal: LAJA` para tribunales).

## Enlaces

- [Plan vigente](PLAN_VIGENTE.md) · [Instrucciones para agentes](AGENTS.md)
- [Plan de implementación](docs/implementacion/PLAN_IMPLEMENTACION.md)
- [Traspaso entre sesiones](docs/implementacion/TRASPASO.md)
- [Historial anterior](docs/HISTORIAL.md) · [Resumen del proyecto](RESUMEN_PROYECTO_NURUS_CSMP_20261009.md)

## Comandos

```sh
python -m pip install --require-hashes -r requirements/linux-py312.lock
python -m pip install --no-deps --no-build-isolation -e .
python tools/verificar_wp.py
git config core.hooksPath tools/hooks   # bloquea commits con RUT reales
```
