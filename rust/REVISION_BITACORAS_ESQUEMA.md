# Evidencia de revisión de bitácoras — versión 1

Ampliación aditiva autorizada por el usuario el 17 de noviembre de 2026.
No cambia las columnas ni los campos que consume el motor tradicional.

## Fuentes exactas

- Identidad: registros.csv, source_system + court_code + cause_id + ingreso_id,
  con identity_status=VERIFICADA. record_id y origen de fila conservan trazabilidad.
- Asociación de lectura: consultas.csv, screen=bitacoras, filters_json.remote_identity.
- Evidencia nueva: filters_json.revision_bitacoras. El exportador copia el objeto
  lectura.datos.revision_bitacoras; solo lo escribe cuando existe.
- Captura: recoleccion.rs::persistir llama evidencia_revision antes del recorte
  legado de entradas. Conserva entries, warnings, coverage, captured_at, desde y hasta.
- Autoría: lecturas.rs::leer_bitacora valida la tabla y sus HOBS_Centro asociados,
  la identidad del formulario y las entradas ocultas antes de emitir origen=centro.
  El clasificador usa ese campo estructurado; no busca palabras en el texto.
- Los campos origin/origin_verified legados de bitacoras.csv no se usan como prueba
  de ausencia: exportar.rs los marca CENTRO de manera general.

## Objeto adicional

version=1, desde/hasta (fechas civiles ISO inclusivas), captured_at (RFC3339),
coverage, completa (booleano), entries (entradas originales sin recortar),
warnings y time_zone=FECHA_CIVIL_DE_LA_CAPTURA.

Una lectura nueva solo acredita completitud si coverage es COMPLETA o
VACIA_COMPROBADA, no tiene advertencias y todas sus entradas tienen fecha válida.
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

## Verificación

tests/revision_bitacoras.rs genera ZIP sintéticos en memoria: identidad, estados,
autoría, fechas, duplicados, paquete legado, alteraciones y paridad del motor con
una fila elegida cuyo cálculo depende de otra no elegida.
recoleccion.rs prueba preservación de fechas ilegibles y lectura vacía acreditada.

