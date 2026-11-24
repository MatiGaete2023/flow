# Registrador de observaciones — continuidad V3.0

## P0 — Línea base (24 de noviembre de 2026)

- Rama experimental solicitada expresamente: `feat/registrador-observaciones-rus-v3`, derivada de `flow-python` en `afc4d12ae1adabf21049c809c0d0d8f747e2e306`. No fusionar ni modificar la rama productiva.
- Plan maestro V3.0, 23 de noviembre de 2026, SHA-256 del archivo recibido: `fd4a8c8e8a51e6b1a5c7d9e551d3fab321c9114fb4cd8dfd116b93d8ef0c76a8`. Conservar el documento original localmente; no publicar capturas ni ZIP exploratorios.
- Verificados metadatos del repositorio (público), permiso push y SHA base; leídos AGENTS, PLAN_VIGENTE, continuidad Python, perfiles, vínculos y diseño histórico del diario.
- La decisión v congela Rust. D3 retira el registrador previo. La orden expresa de crear una rama prevalece sobre la regla anterior de rama única, solo para este experimento. No modifica la prohibición de escritura ni autoriza restablecer componentes retirados.
- [CONFLICTO_ABIERTO] PLAN_VIGENTE y AGENTS prohíben escritura automática y módulo de registro productivo. Efecto: módulo autónomo de preparación/simulación, sin escritor ni integración productiva; P8/P9 productivos bloqueados hasta decisión expresa e autorización institucional. No tocar Descargador ni Asistente.
- `descargador/vinculos.py` (blob `bded6257ea0b5c1614908430a2005867d44469ce`): ShowObservaciones tiene seis argumentos literales; causa = 0, tribunal = 1, ingreso = 2, tipo popup = 3, etapa = 4, antiguo = 5; tipos 11/12/13. Identidad exacta, nunca solo RIT.
- `descargador/perfiles.py` (blob `3e97f048cc7b5530319fda0cf059565917ddad22`): menú seguimiento 89; calendario 91 excluido. Informes usa filtro de catálogo, no valores inventados.
- `descargador/extension/pagina.js` (blob `d2ddb915584f91a57b3f3e75bcfdbf8cad7011f3`): escritura_rus:false.
- Fuentes históricas: 342 eventos y nueve envíos según plan recibido, no recibos comprobados; ZIP y selectores reales no disponibles aquí. No afirmar revalidación de esas fuentes. No almacenar sus valores en GitHub.
- Verificación P0 parcial: GitHub API operativa, SHA confirmado. Git directo desde contenedor sin DNS; no se pudo clonar ni ejecutar arranque/importación de Flow. Python local 3.13.5; objetivo 3.12 Windows pendiente. No ejecutar suite global.

## Alcance y orden

- P1: contrato RegistroFlowV1 estricto, cuatro orígenes, identidad sistema/tribunal/causa/ingreso, destino explícito, texto íntegro/hash, aprobación y versión. Sin red.
- P2: detección de formulario nuevo sintético; editor visible, casilla única, Estado independiente, Nueva obs distinto de Grabar. Selectores reales pendientes de inspección pasiva autorizada.
- P3: localización exacta desde cuatro vistas del menú 89 y preparación sin guardado; no ejecutar JS arbitrario ni POST directo.
- P4: diario local recuperable y estados simulados; sin capacidad productiva de intento.
- P5: cuota móvil opcional desactivada, intervalos none/fijo/variable para carga autorizada; bloqueo crítico y una operación activa.
- P6: relectura con cobertura y nueva fila única; conciliación en copia nueva sin reenviar.
- P7: puente cerrado localhost y UI aislada; primera prueba transversal sintética, sin escritura. P8/P9 productivos: bloqueados.
- Solo datos ficticios; no credenciales, cookies, capturas ni texto judicial en repo/CI. Pruebas específicas por paquete, sin suite global hasta integración pertinente.

## Regla funcional

ADMINISTRATIVA = casilla desmarcada; AL_TRIBUNAL = marcada. Estado es independiente. No inferir destino por texto. Nueva obs crea; Grabar edita (fuera de alcance). No truncar texto. El contador nominal de 2000 no acredita validación del servidor.

## Siguiente paso

P1 — implementar contrato y esquema local aislado, pruebas focalizadas. Mantener pendiente P0 (arranque real de la base y fixtures originales en ambiente autorizado). No volver a analizar grabaciones completas salvo necesidad concreta.

## P1 — Contrato local (avance 24 de noviembre de 2026)

- Base del paquete: `6a144c4d691e8f9f31908390455a3230763e8bd5`. Archivos nuevos: app/contracts.py, contracts/registro_flow_v1.schema.json, tests/test_contracts.py, README.md (todo bajo registrador_rus/).
- Decisiones: modelo inmutable, identidad SITFA completa y exacta; no coerción de identificadores; destino explícito separado de Estado; conserva Unicode y espacios del texto, SHA-256 sobre UTF-8; fecha UTC con Z y versión positiva. require_same_approval compara todo el contrato contra la aprobación vigente obtenida por el consumidor local; no acredita identidad del aprobador. El límite de longitud real se verifica en P2/P3, sin truncar.
- Prueba local: 5 métodos unittest, todos OK, incluyendo ocho combinaciones origen/destino y casos inválidos; Python 3.13.5 Linux. No se ejecutó suite global ni pruebas sobre RUS. Blobs esperados del código y pruebas: 0ff9bdb10936c71794f1353bdd6ef9ff3a93a6d4 y 6ce4a210903394d2e265c10c804a2c5dfd7e1454. El esquema estructural se incorpora, falta prueba específica de concordancia JSON Schema. Python 3.12/Windows pendiente.
- Siguiente: cerrar comprobación de esquema P1 y desarrollar P2 con fixture ficticio y detección inequívoca; no inventar selectores reales. P0 arranque/importación de Flow pendiente por entorno sin DNS. P8/P9 productivos bloqueados.

### Cierre de P1

- Comprobación remota: los blobs de contracts.py y test_contracts.py coinciden con los archivos ejecutados localmente. Esquema JSON blob `49f2a3744311d307100217211fff4b460512093d`, verificado localmente con el mismo hash.
- Prueba focalizada adicional: `python -m unittest discover -s registrador_rus/tests -p test_schema.py -v`: 2 OK, Python 3.13.5 Linux, jsonschema (dependencia dev ya declarada en pyproject.toml). Comprueba esquema Draft 2020-12, ejemplos válidos e inválidos, y que un digest formalmente válido no sustituye el cálculo del hash. Blob esperado del test: `432fb66cae9b87892cff5f495e7ab3ef76dd9d40`.
- P1: 7 métodos de prueba OK. P2 es el siguiente paquete. No aceptación institucional ni prueba en Windows; P0 arranque de Flow sigue pendiente.
