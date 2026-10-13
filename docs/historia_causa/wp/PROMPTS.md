# Prompts para la IA ejecutora (pegar tal cual; uno por WP)

Cada prompt es autosuficiente: la ficha tiene el resto. No añadir contexto de conversaciones anteriores.

## WP-52 (después de recibir `estructura_historia.json`)

> Repositorio `MatiGaete2023/flow`, rama `feat/historia-causa-sitfa`. Ejecuta `docs/historia_causa/wp/WP-52.md`
> pasos 1, 3, 4 y 5. Adjunto `estructura_historia.json` (estructura anónima, sin datos reales). Lee solo lo que
> indica la ficha. Antes de cada push corre `tools/ci_local.sh <pruebas>`. Si el escenario es C o D, detente y
> pregúntame. Responde al final con: escenario, encabezado de la tabla, paginación sí/no y commits creados.

## WP-53

> Repositorio `MatiGaete2023/flow`, rama `feat/historia-causa-sitfa`. Ejecuta `docs/historia_causa/wp/WP-53.md`
> subpaso por subpaso (53.1 → 53.7), prueba primero, un commit por subpaso. Lee solo la ficha, las secciones de
> `02_ESPECIFICACION.md` que cita, `contracts/csmp_capture_v1/HISTORIA_VENTANA.md` y las líneas de código
> citadas. Si algo no calza con la ficha, detente y anótalo en `docs/implementacion/estado/WP-53.json` como
> BLOQUEADO. Al terminar: `tools/ci_local.sh` y `python tools/verificar_wp.py --salida docs/implementacion/verificacion/WP-53.json`.
> Responde solo con la tabla de subpasos (commit, pruebas nuevas, estado) y el resultado de la verificación.

## E3 de WP-53 (otra sesión, modelo capaz)

> Revisa el diff de WP-53 en `feat/historia-causa-sitfa` contra `docs/historia_causa/wp/WP-53.md` y
> `docs/historia_causa/02_ESPECIFICACION.md`. Foco: la extensión solo puede pedir la ruta exacta y no lee
> cookies; ningún error de historia aparece como vacío ni altera bitácoras; filas CAUSA sin `ingreso_id`;
> `a1` nunca persiste; paquete sin historia idéntico; el Asistente importa con historia sin cambiar su trabajo;
> retención incluye `historias.json`. Responde APROBADA o CAMBIOS con lista `archivo:línea → problema`.

## WP-54

> Repositorio `MatiGaete2023/flow`, rama `feat/historia-causa-sitfa`. Ejecuta `docs/historia_causa/wp/WP-54.md`
> pasos 1 a 6. No abras PR ni lances workflows Windows hasta que yo lo pida (paso 7). Antes de cada push corre
> `tools/ci_local.sh`. Responde con commits, pruebas nuevas y resultado de `verificar_wp.py`.
