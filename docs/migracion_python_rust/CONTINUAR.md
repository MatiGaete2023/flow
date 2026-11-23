# Continuar en Flow Rust

Actualizado: 23 de noviembre de 2026. Rama de trabajo única: `estudio/paridad-python-rust-20261119` de `MatiGaete2023/flow`. No crear ramas, no `--force`, no merge a `main`, no releases.

## Estado (N1)

- Motor de reglas y de historia: paridad con Python en el corpus sintético (174/174 y 2.582/2.582 en `oraculo_python` y `oraculo_historia`).
- CI «Flow Rust local · Windows»: verde en cada push; último código `d2297f0`.
- Descargador: lote Lite e Integral sin Carga, puente local, favoritos, preferencias persistentes, recuperación de pendientes, lector de Excel XML 2003 y contrato de Carga (23 columnas) portados.
- Capturas de la CI sin revisar: el proxy de la sesión bloquea la descarga de artefactos.

## Pendiente (en orden)

1. Paquetes A–H de [PLAN_PRUEBAS.md](PLAN_PRUEBAS.md), cada uno por otra IA; anotar resultados en el [registro](REGISTRO_IMPLEMENTACION.md). El paquete H (acta de aceptación) es solo del usuario.
2. Revisar las capturas de la CI desde una sesión con acceso a artefactos.
3. Habilitar la Carga en `app/src/descargador/lote.rs`. El lote Integral sin Carga quedó habilitado en el turno 15. Faltan las diferencias 3 y 5 del turno 14: perfil de Carga, respuesta web y cotejo con `leer_carga`.
4. Bloques propios del Integral: bitácoras, historia, calendario, histórico y paquete CSMP (unas 3.000 líneas de Python); resto de `preferencias.py`; exportaciones de lote (F22).
5. Prueba de aceptación del usuario con el ejecutable vigente y datos locales.

## Cómo trabajar

1. Leer [diseño](DISENO.md), [registro](REGISTRO_IMPLEMENTACION.md) y [plan](PLAN.md). La compilación parte de `app/Cargo.toml`.
2. Probar sin UI con el crate aislado descrito en [VERIFICACION.md](VERIFICACION.md); la UI se valida en la CI Windows.
3. Reglas: solo datos sintéticos; RUS/SATURNO solo lectura; Outlook solo borradores con CC `ucc_concepcion@pjud.cl`; sin credenciales en código, logs ni fixtures; reglas, umbrales y textos tradicionales intactos salvo decisión del usuario; nada se marca «aceptado» sin acta del usuario.
4. Publicar en la misma rama; commits con las líneas `Co-Authored-By` y `Claude-Session`.
