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
