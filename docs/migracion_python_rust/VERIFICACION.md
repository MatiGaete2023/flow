# Verificación de la aplicación vigente en Windows

Desde la raíz del repositorio, fijar los corpus. En la carpeta local que proviene del ZIP, asignar esas dos variables a las mismas subcarpetas de la copia de Flow Python.

```powershell
$env:FLOW_CASOS = (Resolve-Path rust/casos).Path
$env:FLOW_FIXTURES = (Resolve-Path rust/crates/flow-tabular/tests/datos).Path
$env:FLOW_REFERENCIA = (Get-Location).Path
cargo test --manifest-path app/Cargo.toml --locked --all-targets
cargo run --manifest-path app/Cargo.toml --locked --example verificar_paquete -- contracts/csmp_capture_v1/fixtures/completa.zip
cargo run --manifest-path app/Cargo.toml --locked --example verificar_paquete -- contracts/csmp_capture_v1/fixtures/parcial.zip
cargo build --manifest-path app/Cargo.toml --locked --bin flow_rust
python app/qa/capturar_windows.py --exe app/target/debug/flow_rust.exe --salida app/target/capturas
```

`FLOW_CASOS` y `FLOW_FIXTURES` son necesarios para ejecutar los contrastes. Las pruebas históricas del workspace están en el [commit anterior](https://github.com/MatiGaete2023/flow/blob/ed6a0e43f8efa0856e0294af92ac8e0317c69df7/docs/migracion_python_rust/VERIFICACION.md).

El flujo de GitHub Actions «Flow Rust local · Windows» compila `app/` y genera `Flow_Rust_Local_Capturas_Windows`. Comparar las cinco imágenes con [Diseño](DISENO.md). El modo captura ajusta la escala si la pantalla virtual es menor que la ventana inicial de 1440 × 900 puntos. Mantener fecha y diferencias de escala separadas de cambios en jerarquía y contenido. Registrar el SHA, el entorno y cada vista revisada.

El ejemplo `verificar_paquete` valida la estructura, las tablas y la procedencia. `FLOW_REFERENCIA` habilita las pruebas sintéticas del lector, de apertura desde Trabajo y de exportación de bitácoras. En una carpeta local proveniente de ZIP, apunta esa variable a la raíz de la copia de Flow Python.

Para repetir el contraste V2 con el mismo ZIP P2, usar Python 3.12 y el lock Windows de la referencia.

```powershell
python -m pip install --require-hashes -r requirements/windows-py312.lock
cargo build --manifest-path app/Cargo.toml --locked --example revisar_bitacoras
python app/qa/contrastar_bitacoras.py app/target/debug/examples/revisar_bitacoras.exe
```
