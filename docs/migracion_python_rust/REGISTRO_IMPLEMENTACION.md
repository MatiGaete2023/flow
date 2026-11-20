# Registro de implementación · Flow Rust

Rama `estudio/paridad-python-rust-20261119`. Inicio de la corrección el 20 de noviembre de 2026. El [registro anterior](https://github.com/MatiGaete2023/flow/blob/ed6a0e43f8efa0856e0294af92ac8e0317c69df7/docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md) queda disponible por commit.

## Fuente y decisión

- Diseño y código de producto de la carpeta local `flowrust/flow-flow-python/app/`, publicado como `app/` en la raíz remota.
- Comportamiento de la carpeta `flowpython/flow-flow-python` y la rama `flow-python` vigente. El estudio anterior se hizo sobre `93a3631`. La punta remota consultada para esta corrección fue `afc4d12ae1adabf21049c809c0d0d8f747e2e306`. Comprobado por blob que el `src/nurus/capture/contract.py` local coincide con `flow-python` (`361d0f907100ca4a1a71c790d98f82cbe25f8bb0`). Verificar los archivos de referencia por entrega si difieren las copias.
- El traspaso de ayer terminó en `ed6a0e43`. Sus 355 pruebas Linux, 353 Windows, cuatro clasificaciones V2 y artefacto `Flow_Windows` corresponden a `rust/crates/`. Usar sus casos y resultados como referencia al portar, e identificar la cobertura específica de `app/`.

## Cambios del turno

- Se cambió el destino del plan y la matriz a `app/`. Se conservaron E0–E7 y F01–F28. Se documentaron las vistas y se regeneraron referencias sintéticas de Trabajo, Correos, Resoluciones, Anonimizador y Ajustes oscuro con el ejecutable local preexistente.
- Se respetaron las vistas y su distribución. Se reemplazaron ejemplos de contacto y datos sintéticos en UI, demo y prueba de anonimización por valores reservados. Se corrigió la aserción de esa prueba para que compruebe el mismo RIT ficticio que entrega como entrada.
- Comenzó F07 en `app/src/captura.rs`. Comprobación de rutas, formato ZIP, límites, manifiesto, rangos de versión, entradas requeridas, tamaños y huellas sin extraer archivos. El ejemplo `verificar_paquete` permite repetir la prueba con un archivo local. El flujo de Trabajo sigue abriendo sus formatos anteriores.

## Verificación y pendientes

- Windows, Rust 1.100.0. Cuatro pruebas del módulo de captura aprobadas con sus dependencias fijadas en caché, en un crate de prueba aislado bajo `app/target/`. Los ZIP completo y parcial de la referencia Python también pasaron la comprobación estructural.
- En un segundo crate aislado, se ejecutaron los módulos locales `motor.rs`, `config.rs` y `util.rs` con `FLOW_CASOS` de la referencia Python local. Pasaron las cuatro pruebas del conjunto y coincidieron los 174 casos sin historia ni bitácora.
- Revisadas las capturas de referencia de la carpeta local. El ejecutable preexistente generó cinco PNG desde el modo sintético. Los controles Python de RUT, RIT y contactos pasaron sobre `app/src`.
- La compilación completa local quedó detenida al resolver `home 0.5.12` sin caché y por acceso intermitente a `index.crates.io`. El binario local preexistente solo acredita el diseño de partida. La prueba completa y las capturas generadas por una compilación del commit corresponden al nuevo flujo Windows.
- El cambio de base quedó publicado en `1affc51ccd4a6541734bf23b51e74902540fd465`. Su [ejecución Windows](https://github.com/MatiGaete2023/flow/actions/runs/44747046388) pasó 11 pruebas, con 1 de rendimiento omitida, y comprobó los dos ZIP sintéticos. La revisión de sus cinco capturas identificó recortes a 1024 × 768 por el monitor virtual. Se ajustó el zoom del modo captura para que las próximas imágenes muestren la composición completa. Se retiró el disparador automático de la rama de migración del workflow del workspace histórico.
- **E0 verificado en Windows.** La [corrida final](https://github.com/MatiGaete2023/flow/actions/runs/44748317049) compiló y probó `app/` en `1a839bd1e5d444e8724d6135419d4b11e0039fdb`. Pasaron 11 pruebas, incluida la del oráculo Excel, y los dos ZIP sintéticos. La prueba de rendimiento se mantuvo omitida. Se abrieron y compararon las cinco [capturas del binario](https://github.com/MatiGaete2023/flow/actions/runs/44748317049/artifacts/14634511876) con el diseño local. El modo captura conserva la distribución a escala en 1024 × 768 sin guardar ese ajuste en las preferencias.
- En Windows local, se comprobó además la integridad de `rust/crates/flow-trabajo/tests/datos/paquete_integracion.zip`, con contrato 1.2.0. El lector de referencia Python validó los tres ZIP y registró cuatro filas de `registros` en cada uno.
- Pendientes F07 semántica e integración. El verificador incorporado todavía no valida contenido CSV, conciliación entre consultas, identidad, cobertura, ni estructura interna del XLSX. Sigue después F06 revisión y exportación, y E2 para el Descargador integral y Lite. E3–E7 conservan el orden del plan.

## Siguiente paso exacto

1. En `app/src/captura.rs`, usar los encabezados de `TABLES` y las comprobaciones de `read_capture` y `_validate_rows` de Python. Añadir casos con bytes y huellas correctos pero columnas, filas, estados o identidad inválidos. Mantener evidencia incompleta como tal para V1, V2 y legado.
2. Cuando ese lector pase, integrar apertura por botón existente, Ctrl+O y arrastre en `ui.rs`. Preparar el nuevo análisis antes de cambiar `self.work`. Fijar etapa y corte, y preservar decisiones por identidad de origen. Probar P2 completo con el ZIP sintético que genera `descargador/test_revision_bitacoras_p2.py` y [las cuatro clasificaciones](CASOS_PRIMERA_ENTREGA.json) antes de continuar a exportaciones y E2.
