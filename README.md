# Flow Rust · Asistente CSMP y Descargador

La aplicación vigente de esta rama es el escritorio [`app/`](app/), que compila `flow_rust.exe`.

```powershell
cargo run --manifest-path app/Cargo.toml --locked
```

- [Estado y siguiente paso](docs/migracion_python_rust/ESTADO.md)
- [Plan y decisiones](docs/migracion_python_rust/PLAN.md)
- [Verificación](docs/migracion_python_rust/VERIFICACION.md)
- [Diseño](docs/migracion_python_rust/DISENO.md)
- [Índice de la migración](MIGRACION_RUST.md)

No envía correos (solo borradores con CC fijo), no escribe en RUS ni en SATURNO y solo usa datos sintéticos en el repositorio.

## Referencia funcional Python

El código Python de esta rama (`src/`, `descargador/`, `descargador_lite/`, `anonimizador/`, `META4_Bitacoras/`, `tests/`, `tools/`) se conserva porque los oráculos de `app/qa/` y `rust/oraculo/` lo ejecutan. Su documentación, manuales, plan y pruebas completas están en la rama [`flow-python`](https://github.com/MatiGaete2023/flow/tree/flow-python); aquí se retiraron el 24 de noviembre de 2026. Las pruebas Python que leen esos documentos se ejecutan en `flow-python`, no en esta rama.
