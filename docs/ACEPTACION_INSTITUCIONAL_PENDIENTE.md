# Procedimiento pendiente de aceptación en el entorno autorizado

El usuario confirmó el 2026-10-08: «no hay, usaremos SITFA y RUS después».
Estas pruebas quedan diferidas por su decisión. El MVP técnico local se entrega
con evidencia ficticia; no se solicita acceso institucional en esta etapa.

Esta acta describe trabajo aún no ejecutado. No contiene datos personales,
credenciales, pantallas institucionales ni comprobantes ficticios.

| Gate | Prueba de operador | Registro admisible en flow |
|---|---|---|
| G1 | EXE/extensión mismo SHA; revisar casos SIN_VINCULO y modalidades/pestañas; cotejar identidad, entradas, autor, fechas, texto íntegro y cobertura | SHA/versiones, códigos, conteos y PASÓ/FALLÓ; evidencia personal permanece local |
| G9 | Caracterizar historia SITFA real: identidad CAUSA/INGRESO, columnas, paginación, origen y referencias; documentos solo selección autorizada | Diccionario anonimizado, variantes del contrato y resultado; no HTML/HAR/PDF real |
| G10 | Aprobar cada CT, umbral/texto/acción y tribunal aplicable con comparación baseline | Referencia de aprobación y catálogo versionado; ninguna activación global implícita |
| G11/G8 | Importar completo/parcial/corrupto en instalaciones separadas; conservar edición, adjuntos, Word; cotejar Excel 2010 y borradores Outlook 365 | Resultado por A-T/INT y versión; ningún envío ni registro automático |
| G12 | Ejecutar ampliación historia/contexto únicamente aceptada | Casos aprobados y límites, sin anunciar capacidades no probadas |
| G13 | EXE abre sin admin, recursos/plantillas completos, versión/SHA/extensión, políticas Chrome y recuperación | Manifiesto/hashes del artefacto; autorización de distribución separada |

Nunca compartir cookies, tokens, usuarios/passwords ni datos reales de NNA en PR.
Las pruebas se realizan bajo sesión institucional legítima del operador.

Formato mínimo de acta anónima por caso:

```json
{"gate":"G1","case":"D-T10","artifact_commit":"SHA_REAL",
 "downloader_version":"VERSION_REAL","extension_version":"VERSION_REAL",
 "modality_code":"CODIGO","tab":"PESTANA","executed_at":"FECHA_REAL",
 "result":"NO_EJECUTADO","counts":{},"limitations":[],
 "evidence_retained_locally":true,"operator_reference":"REFERENCIA_LOCAL"}
```

Los placeholders son una plantilla, no evidencia. Historial incompleto/fallo de
consulta se conserva explícito. No puede convertirse en «sin actuaciones».

Historia/PDF no se desarrollan contra endpoints supuestos: primero contrato real,
luego adaptador de lectura y pruebas. La aceptación CT no cambia automáticamente
las observaciones editadas del trabajo actual.

## Preparar y validar el acta sin dar por hechas las pruebas

`tools/flow_acceptance.py` funciona localmente con Python 3.10 o posterior.
No necesita Chrome, Office, sesión institucional ni credenciales. La plantilla
incluye los 115 IDs de las especificaciones, todos `NO_EJECUTADO`. No ejecutar
`init` sobre un acta existente: la herramienta conserva el archivo y rechaza
sobrescribirlo. Guardar actas nuevas para cada artefacto/alcance.

Ejemplo PowerShell desde el repositorio, leyendo el SHA del EXE comprobado:

```powershell
$build = Get-Content 'C:\Paquete\VERIFICACION_CSMP.json' -Raw | ConvertFrom-Json
python tools/flow_acceptance.py init 'C:\Trabajo\acta.json' --commit $build.source_commit --assistant-version $build.source_version --downloader-version 2.7.2 --extension-version 2.7.2
python tools/flow_acceptance.py validate 'C:\Trabajo\acta.json'
```

Verificar las versiones del Descargador/extensión en su JSON antes de usar los
valores del ejemplo. `--case D-T10 --gate G1` crea una plantilla focalizada;
`--kind SINTETICA_WINDOWS` permite registrar pruebas ficticias de CI separadas.
El paquete conjunto incluye una plantilla institucional del SHA que se compiló.

Cada resultado ejecutado `PASÓ`/`FALLÓ` exige gate, fecha con zona, entorno,
custodia local, al menos una huella SHA256 de evidencia y referencia opaca de
operador `OP-` seguida de doce dígitos hexadecimales mayúsculos. La correspondencia
del código con el funcionario y las evidencias se conserva institucionalmente.
Un hash identifica un archivo; no acredita por sí solo su contenido ni su origen.

Campos admisibles:

| Campo | Valores |
|---|---|
| `evidence_kind` | `SINTETICA_LOCAL`, `SINTETICA_WINDOWS`, `INSTITUCIONAL` |
| `environment` | Local: `LINUX`, `WINDOWS`, `MACOS`; CI: `WINDOWS_CI`; institucional: `WINDOWS_10_INSTITUCIONAL`, `WINDOWS_INSTITUCIONAL` |
| `result` | `PASÓ`, `FALLÓ`, `NO_EJECUTADO`, `NO_APLICA` |
| `counts` | Solo `expected`, `observed`, `matched`, `failed`, `unknown`, enteros no negativos |
| `limitations` | `FALTA_ENTORNO`, `FALTA_EVIDENCIA`, `FALTA_OFFICE`, `FALTA_CONTRATO_REMOTO`, `FALTA_APROBACION_FUNCIONAL`, `COBERTURA_PARCIAL`, `IDENTIDAD_INCIERTA`, `FUERA_DEL_ALCANCE`, `FALLO_TECNICO`, `DIFERENCIA_BASELINE`, `NO_APLICA_AL_ARTEFACTO` |

Los casos no ejecutados tienen fecha/entorno/referencia vacíos, sin comprobantes.
`NO_APLICA` exige `NO_APLICA_AL_ARTEFACTO` y tampoco se cuenta como ejecución.
No introducir nombres, RUT, causas, ingresos, rutas de evidencia, URLs, glosas
ni texto libre. El validador rechaza campos adicionales, claves JSON duplicadas,
documentos mayores de 1 MiB, casos repetidos y un `PASÓ` con fallos declarados.
Su salida muestra únicamente SHA y conteos separados por tipo de evidencia.

Los conteos describen una misma unidad: `expected` es el alcance previsto,
`observed` lo cotejado, y `matched`/`failed`/`unknown` clasifican lo cotejado.
Son opcionales; si se declaran las tres categorías, suman `observed`. Ninguna
clasificación puede superar lo observado. PASÓ requiere cero fallos/desconocidos
y, cuando ambos se declaran, `expected == observed`. Un resultado sin conteos
no convierte esa ausencia en evidencia; conserva las exigencias de custodia.

La validación comprueba estructura y coherencia declarada. **No verifica la
autenticidad de evidencias, no ejecuta pruebas, no aprueba gates y no activa CT.**
El responsable debe cotejar evidencia, alcance y criterios de las especificaciones.
