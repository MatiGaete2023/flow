# Evidencia de revisión de bitácoras — versión 1

Ampliación aditiva autorizada por el usuario el 17 de noviembre de 2026.
No cambia las columnas ni los campos que consume el motor tradicional.

## Fuentes exactas

- Identidad: registros.csv, source_system + court_code + cause_id + ingreso_id,
  con identity_status=VERIFICADA. record_id y origen de fila conservan trazabilidad.
- Asociación de lectura: consultas.csv, screen=bitacoras, filters_json.remote_identity.
- Evidencia nueva: filters_json.revision_bitacoras. El exportador copia el objeto
  lectura.datos.revision_bitacoras; solo lo escribe cuando existe.
- Captura: recoleccion.rs::persistir conserva el mismo límite histórico en entries
  y en los metadatos. Registra warnings, coverage, captured_at, desde y hasta.
- Autoría: lecturas.rs::leer_bitacora valida la tabla y sus HOBS_Centro asociados,
  la identidad del formulario y las entradas ocultas antes de emitir origen=centro.
  El clasificador usa ese campo estructurado; no busca palabras en el texto.
- Los campos origin/origin_verified legados de bitacoras.csv no se usan como prueba
  de ausencia: exportar.rs los marca CENTRO de manera general.

## Objeto adicional

version=1, desde/hasta (fechas civiles ISO inclusivas), captured_at (RFC3339),
coverage, completa (booleano), entries (entradas del intervalo capturado),
warnings, time_zone=FECHA_CIVIL_DE_LA_CAPTURA y contadores de entradas omitidas.
No se vuelve a persistir texto anterior al límite histórico. Una fecha ilegible
deja contador/advertencia de incertidumbre sin retener su texto descartado.

Una lectura nueva solo acredita completitud si coverage es COMPLETA o
VACIA_COMPROBADA, no tiene advertencias y todas sus entradas tienen fecha válida.
El usuario confirmó el 18/11/2026 que la ventana de bitácora de un ingreso muestra
todas sus entradas en una respuesta, sin paginación, carga adicional ni filtros
de fecha, y que la ventana queda vacía cuando no hay observaciones.
Captura registra el método SITFA_TABLA_INFORME_INGRESO_V1 y la huella de la respuesta
solo tras verificar identidad, huella, formulario y estructura reconocida.
La tabla reconocida sin filas se acredita como VACIA_COMPROBADA. Si se detecta
un control que sugiere recorte, la cobertura queda PARCIAL. Una tabla faltante,
estructura inválida o respuesta incompleta tampoco acredita ausencia.
Este alcance rige únicamente para el intervalo conservado por la captura.
El campo coverage tradicional del lote sigue siendo PARCIAL para sus consumidores.
La importación comprueba además fechas, captura, identidad, período y consistencia.
No se asigna completitud a lecturas legadas reutilizadas: sus fechas ilegibles
pueden haber sido descartadas antes. Varias instantáneas diferentes no se combinan
para fabricar cobertura completa.

Las fechas civiles siguen las convenciones de captura existentes. No se convierten
a UTC las horas sin zona. Todo texto no vacío del centro cuenta, en cualquier tipo
o etapa de su bitácora de ingreso. Los marcadores textuales no se interpretan como
ausencia: incluso «sin observación» es texto sustantivo hasta una decisión distinta.
Categorías de registros admitidas: Espera, Cumplimiento e Informes.

## Compatibilidad y contexto

El JSON de filtros admite información adicional y los lectores tradicionales
ignoran esta clave. Una lectura/paquete antiguo sigue abriéndose en Trabajo; en
la herramienta nueva queda No verificable cuando le falta evidencia suficiente.
No cambia la extensión, sus permisos, las consultas ni las reglas del motor.

El motor analiza el conjunto original por la ruta paquete::importar. Solo después
se toman las propuestas de las filas Sin observación. Huella, identidad y versión
de reglas acompañan el informe. La propuesta, edición y evidencia son campos separados.

## Duplicados

Misma identidad compuesta y tipo con datos normalizados iguales: una fila lógica,
con todas sus procedencias. Datos distintos o record_id reutilizado para identidades
distintas: No verificable. Eventos iguales se deduplican; un mismo entry_id con
contenido distinto o instantáneas diferentes queda No verificable.
El cotejo CSV conserva consulta, ID de evento y momento/precisión, además del texto.
Dos eventos con el mismo texto y día pero distintos IDs no se reducen a uno.

## Verificación

tests/revision_bitacoras.rs genera ZIP sintéticos en memoria: identidad, estados,
autoría, fechas, duplicados, paquete legado, alteraciones y paridad del motor con
una fila elegida cuyo cálculo depende de otra no elegida.
recoleccion.rs prueba incertidumbre de fechas ilegibles sin texto retenido y el
recorte histórico. UI prueba captura→ZIP→JSON sin reintroducir texto antiguo.
El flujo sintético Captura → ZIP → Revisión → motor prueba una tabla reconocida
vacía, una con un evento, una con paginador y una sin tabla reconocida.
El JSON exportado acota eventos/entradas al período elegido, informa omisiones y
no muta la evidencia en memoria necesaria para volver a analizar otro intervalo.
