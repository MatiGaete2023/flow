# Verificación de `app/`

La versión anterior, con los comandos de PowerShell paso a paso y el crate `.aislado`, está en [historico/VERIFICACION_20261123.md](historico/VERIFICACION_20261123.md). El crate `.aislado` ya no hace falta.

## Niveles

| Nivel | Cuándo | Comando |
|---|---|---|
| Rápido | Mientras se trabaja un módulo | `tools/probar_app.sh rapido <filtro>` |
| Completo | Antes de publicar un encargo | `tools/probar_app.sh todo` |
| CI | En cada push que toca `app/` | Flujo «Flow Rust local · Windows» |

- El script fija `FLOW_CASOS`, `FLOW_FIXTURES` y `FLOW_REFERENCIA` con los datos del repositorio, agrega `--features eframe/x11` en Linux y muestra solo las líneas de resultado y los fallos.
- `todo` agrega `--include-ignored`: corre los oráculos (`oraculo_python`, `oraculo_historia`, paquete B, cronología) y las pruebas con datos versionados.
- Resultado de referencia en Linux (24-nov-2026): biblioteca `58 passed`, binario `43 passed; 10 filtered out`, en unos 5 s con la compilación en caché.
- Sin el script, en Linux: `cargo test --manifest-path app/Cargo.toml --locked --features eframe/x11`. Sin esa característica la compilación falla con `not supported by winit`.

## Pruebas que necesitan un ZIP generado por Flow Python

Quedan fuera de `todo` si falta la variable. La CI las ejecuta siempre.

```bash
python app/qa/generar_contexto_e3.py <checkout de flow-python> <carpeta>
export FLOW_E3_PAQUETE=<carpeta>/contexto-e3.zip
export FLOW_E3_HISTORIA=<carpeta>/contexto-e3-historia.zip
python app/qa/esperado_contexto_e3.py "$FLOW_E3_HISTORIA" <carpeta>/esperado-historia.json
export FLOW_E3_ESPERADO=<carpeta>/esperado-historia.json
tools/probar_app.sh todo
```

Requiere Python 3.12 con `requirements/linux-py312.lock` o `windows-py312.lock`. `exporta_mismo_lote_de_python` (`FLOW_P2_PAQUETE`) y `velocidad_20000` se omiten por diseño.

## Oráculos grabados

- Datos: `app/tests/datos/{a78,lecturas,vinculos,bitacoras,lote,paquete}.json`. Generadores: `app/qa/generar_<nombre>.py`.
- Regenerar solo si cambian el generador o la referencia Python: `python app/qa/generar_<nombre>.py` y revisar `git diff -- app/tests/datos`.
- Corpus del motor: `rust/casos/*.json`, generado por `rust/oraculo/generar_casos.py`.

## Verificadores estáticos

Solo cuando el cambio toca lo que revisan; la CI los corre siempre.

- Identidades y contactos, antes de publicar datos o fixtures nuevos: `python tools/check_real_ids.py app/src app/tests` y `python tools/check_contactos.py app/src`.
- Textos y umbrales, si cambia `config.rs`: `python tools/verificar_textos_umbrales.py`.
- Escrituras a disco, si se agrega una: `python tools/inventario_escrituras.py`.

## Windows

- El `.exe` sale del artefacto `Flow_Rust_Windows_exe` del flujo, con su `SHA256.txt`. Las capturas salen en `Flow_Rust_Local_Capturas_Windows` y se comparan con [DISENO.md](DISENO.md).
- La sesión en la nube no puede descargar artefactos de Actions: la revisión visual y la prueba con Chrome, SITFA y Outlook reales son del usuario.
- Compilar para Windows desde Linux, sin ejecutar: `rustup target add x86_64-pc-windows-gnu` y `cargo check --manifest-path app/Cargo.toml --locked --target x86_64-pc-windows-gnu --all-targets`.
