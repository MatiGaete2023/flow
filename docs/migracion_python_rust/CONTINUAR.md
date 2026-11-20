# Encargo para la siguiente IA

Trabaja en `estudio/paridad-python-rust-20261119` cuando el usuario te encargue implementar. El original Python está fijado en `93a36312adbd24e2dc96b02dfac45d947193b8b1`. El destino recomendado es `rust/`. La copia `prototipos/flow-egui/` sirve de referencia de diseño y de contraste. Mantén Python como oráculo independiente.

**Primera sesión:** lee [el plan](PLAN.md) E0/E1 y [la verificación](VERIFICACION.md). Revisa `descargador/test_revision_bitacoras_p2.py`, `src/nurus/personal/revision_bitacoras.py` y `rust/crates/flow-trabajo/src/paquete.rs`. Genera el ZIP ficticio desde Python y reproduce en el lector Rust los cuatro resultados de [la primera entrega](CASOS_PRIMERA_ENTREGA.json). Deja primero evidencia de qué caso falla. Implementa la lectura V2 manteniendo V1 nativo y desconocido/legado. Prueba por ingreso, por intervalo, tras guardar sesión y en Excel/JSON.

**Luego:** continúa por dependencia en [PLAN.md](PLAN.md). En [MATRIZ.md](MATRIZ.md) identifica ID, fuente Python, ruta Rust y criterio de cierre. Usa [INVENTARIO.tsv](INVENTARIO.tsv) para localizar funciones. Si una capacidad ya existe, demuéstrala desde el punto de entrada y registra evidencia. Si falta, incorpora la pieza más pequeña que complete el recorrido. Si depende del PC institucional, deja el resultado sintético y el caso exacto que deberá probar el titular.

**Condiciones de producto:** aplicar textos y umbrales del original, conservar cambios humanos y archivos existentes, trabajar solo con fuentes sintéticas en el repo. Las lecturas de SITFA pertenecen al Descargador. Outlook solo guarda borradores con CC institucional. Las observaciones son propuestas para revisión humana. Las redacciones pendientes y la aceptación real pertenecen al usuario.

**Estado inicial:** documentación y prototipo de referencia entregados. El cambio de compatibilidad V2 y las otras entregas del plan están pendientes. Al terminar una sesión deja el commit, matrices afectadas, qué se ejecutó y la siguiente acción concreta. Evita reabrir una decisión resuelta por evidencia salvo que cambie el código de referencia.
