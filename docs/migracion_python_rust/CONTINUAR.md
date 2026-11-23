# Continuar en Flow Rust

Actualizado: 23 de noviembre de 2026. Rama de trabajo única: `estudio/paridad-python-rust-20261119` de `MatiGaete2023/flow`. No crear ramas, no `--force`, no merge a `main`, no releases.

## Estado (N1)

- Paquetes A–G de [PLAN_PRUEBAS.md](PLAN_PRUEBAS.md) ejecutados y aprobados íntegramente en Turno 17.
- Motor de reglas y de historia: paridad con Python en el corpus sintético (174/174 y 2.582/2.582 en `oraculo_python` y `oraculo_historia`).
- Suite completa `cargo test --all-targets` pasando: 40 pruebas en biblioteca, 46 pruebas en binario (1 ignorada por diseño), 0 fallos.
- Ejecutable release compilado con éxito: SHA-256 `7528b329863683df5549cfa85f9bea2473c963894a785cfc1f6c906bd8019a46`.
- Controles de seguridad: `check_real_ids.py` (0 RUT reales), `check_contactos.py` (0 contactos no permitidos), invariantes G1-G6 verificadas sin discrepancias.

## Pendiente (en orden)

1. Paquete H: Acta de aceptación y pruebas del usuario con su PC autorizado, sesión SITFA y Outlook local ([guía](docs/aceptacion/GUIA_PRUEBAS_USUARIO_20261013.md)).
2. Revisar las capturas de la CI desde una sesión con acceso a descarga de artefactos.
3. Bloques propios del Integral: bitácoras, historia, calendario, histórico y paquete CSMP (unas 3.000 líneas de Python); resto de `preferencias.py`; exportaciones de lote (F22).

## Cómo trabajar

1. Leer [diseño](DISENO.md), [registro](REGISTRO_IMPLEMENTACION.md) y [plan](PLAN.md). La compilación parte de `app/Cargo.toml`.
2. Probar sin UI con el crate aislado descrito en [VERIFICACION.md](VERIFICACION.md); la UI se valida en la CI Windows.
3. Reglas: solo datos sintéticos; RUS/SATURNO solo lectura; Outlook solo borradores con CC `ucc_concepcion@pjud.cl`; sin credenciales en código, logs ni fixtures; reglas, umbrales y textos tradicionales intactos salvo decisión del usuario; nada se marca «aceptado» sin acta del usuario.
4. Publicar en la misma rama; commits con las líneas `Co-Authored-By` y `Claude-Session`.
