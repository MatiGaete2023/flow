# Continuar en Flow Rust

Actualizado: 23 de noviembre de 2026. Rama de trabajo única: `estudio/paridad-python-rust-20261119` de `MatiGaete2023/flow`. No crear ramas, no `--force`, no merge a `main`, no releases.

## Estado (N1, auditado sobre `f6a8f28`)

- Motor de reglas y de historia: paridad con Python en el corpus sintético. Reproducido en Linux: `oraculo_python` 174/174 y `oraculo_historia` 2.582/2.582.
- Biblioteca 40/40 y binario aislado 41 pruebas más 1 ignorada, en Linux.
- Descargador: lote Lite e Integral con Carga sobre SITFA ficticio (turnos 15 y 16). Falta la verificación en Windows.
- Plan de pruebas: A y G pasan con salvedades, B pasa sus cifras con el criterio incompleto, F está parcial, C y E no se ejecutaron y D no está acreditado. El detalle y los encargos están en [ENCARGOS_IA.md](ENCARGOS_IA.md). El turno 17 del registro decía «A–G aprobados íntegramente»; la auditoría lo corrige.

## Pendiente (en orden)

1. Siguiente encargo: E-J (casos de R3 en el oráculo, [prompt](prompts/E-J.md)); E-B quedó cerrado por la auditoría del 23 de noviembre. Luego, los demás encargos de [ENCARGOS_IA.md](ENCARGOS_IA.md), cada uno a cargo de una IA ejecutora. La auditoría revisa cada entrega antes de cambiar su estado.
2. Decisiones D-1 (conservar el guion) y D-2 (`keep_filters` igual que Python) resueltas el 23 de noviembre; ver [ENCARGOS_IA.md](ENCARGOS_IA.md).
3. Paquete H: acta del usuario con su PC autorizado, sesión SITFA y Outlook local ([guía](../aceptacion/GUIA_PRUEBAS_USUARIO_20261013.md)).

## Cómo trabajar

1. Leer [encargos](ENCARGOS_IA.md), [diseño](DISENO.md), [registro](REGISTRO_IMPLEMENTACION.md) y [plan](PLAN.md). La compilación parte de `app/Cargo.toml`.
2. Probar sin UI con el crate aislado descrito en [VERIFICACION.md](VERIFICACION.md); la UI se valida en la CI Windows.
3. Reglas: solo datos sintéticos; RUS/SATURNO solo lectura; Outlook solo borradores con CC `ucc_concepcion@pjud.cl`; sin credenciales en código, logs ni fixtures; reglas, umbrales y textos tradicionales intactos salvo decisión del usuario; nada se marca «aceptado» sin acta del usuario.
4. Publicar en la misma rama; commits con las líneas `Co-Authored-By` y `Claude-Session`.
