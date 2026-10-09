# CSMP_CAPTURE_V1 — contrato local 1.0.0

Implementación compartida: `nurus.capture.contract`, compatible con Python 3.10
productor y Python 3.12–3.14 consumidor. No importa UI, Office ni módulos personal.
El ZIP transporta capturas ya obtenidas; no ejecuta instrucciones ni accede a red.

## Estructura y compatibilidad

`manifest.json` y CSV en `data/`: consultas, registros e incidencias obligatorios;
bitacoras, calendarios, historia_tramites y documentos opcionales con estado
explícito. Una tabla disponible/vacía comprobada debe existir, aunque sin filas.
NO_SOLICITADA difiere de NO_CONSULTADA. FALLIDA/PARCIAL nunca equivale a vacío.

CSV UTF-8, coma y comillas RFC 4180; las columnas y su orden están en
[DICCIONARIO.md](DICCIONARIO.md). Números de identidad son texto. `raw_json`
conserva campos de captura; `normalized_json` conserva mapeo sin sustituir raw.
`calendarios.csv` cierra la omisión de calendario en el diseño inicial.

[manifest.schema.json](manifest.schema.json) describe la estructura; el validador
además comprueba claves ajenas, identidad, conteos, cobertura y hashes. Versiones
1.x admiten metadatos y columnas opcionales adicionales al final; `required_features` no soportadas y
versiones mayores distintas se rechazan. Cambios semánticos o columnas obligatorias requieren negociación
explícita; columnas opcionales desconocidas se conservan sin interpretarlas como hechos institucionales.

## Integridad y límites

No se extraen archivos del ZIP. Se rechazan rutas relativas inseguras, absolutas,
reservadas Windows, colisiones por mayúsculas, enlaces, cifrado, métodos de
compresión inesperados, archivos no declarados y hashes/conteos distintos.
Presupuesto conservador inicial: 80 MiB de archivo y suma descomprimida, 2048
entradas, 200.000 filas agregadas, 8 MiB por campo, expansión máxima 200:1.
Estos límites conservan máximos previos y se documentan con benchmark; no son
promesas de rendimiento institucional.

Originales excluidos por defecto. Solo `originals_authorized=true` admite inclusión
explícita; nunca se publica un original real en GitHub. XML/PDF incluidos se
verifican como bytes sin interpretarlos, sin XXE, macros ni contenido activo.
CSV técnico conserva literal completo, incluidas cadenas que comienzan por `=`;
no debe abrirse directamente como hoja. Excel derivado escribe esas cadenas como
texto. Texto extenso se conserva en CSV y las salidas Excel usan partición/sidecar.

## Identidad, tiempo y aceptación

IDs locales identifican consultas/filas, no reemplazan identidad remota.
VERIFICADA exige tribunal+causa+ingreso. No hay cruce por RIT/nombre. Historia de
alcance CAUSA permanece sin ingreso; calendario sin identidad es NO_ASOCIADO.
Hora/zona ausentes se conservan desconocidas. Fecha de captura no sustituye fecha
de evento; `as_of` y versión de reglas se fijan al analizar.

La publicación valida un temporal y crea destino nuevo atómicamente, sin
sobrescribir. Una importación inválida no altera el trabajo activo. Reimportar
mismo hash/corte/configuración conserva decisiones del trabajo actual.

Los campos `institutional_verified`/`history_complete` son declaraciones de origen;
no certifican por sí solos G1/G9. El Asistente no los acepta como autorización de
reglas nuevas. Las reglas CT quedan desactivadas; interpretación temporal de datos
reales depende de aceptación institucional y funcional externa.

## Uso reproducible

```sh
python tools/flow_capture_fixture.py /ruta/nueva/completa.zip
python tools/flow_capture_fixture.py /ruta/nueva/parcial.zip --partial
python tools/flow_capture_cli.py validate /ruta/completa.zip
```

[Fixture completa](fixtures/completa.zip) y [parcial](fixtures/parcial.zip) son
exclusivamente sintéticas. El generador permite tamaños 100/500/2000 para medición.

## Cobertura por identidad y consultas vacías

El agregado de una tabla no puede declarar mayor cobertura que sus consultas o
filas. Una consulta parcial nunca respalda una fila COMPLETA; VACIA_COMPROBADA
no admite filas. Los calendarios agregan también consultas pendientes sin filas.

`filters_json` admite dos claves declarativas opcionales: `capture_tables`, lista
no vacía de tablas fuente, y `remote_identity`, objeto con `source_system`,
`court_code`, `cause_id`, `ingreso_id`, todos textos completos de fuente.
La identidad del alcance debe coincidir con sistema/tribunal de la consulta y
con sus filas. Esto permite acreditar un resultado vacío para un ingreso concreto.
No son comandos ni disparan consultas. Los productores antiguos siguen siendo
legibles; un vacío global sin alcance por ingreso no prueba vacío de ese ingreso.

El Asistente calcula cobertura por fuente/ingreso y conserva incidencias del ZIP.
La tabla global sirve de resumen; las solicitudes evitan repetir fuentes completas
y piden las insuficientes. La captura v1 mantiene el límite agregado de 200.000
filas y 80 MiB. El análisis de trabajo limita a 20.000 registros incluidos los
calendarios independientes; dividir lotes mayores. Ese techo se ensayó con datos
sintéticos Linux y no certifica rendimiento ni Office institucional.
