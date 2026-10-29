# Ejecución técnica del plan — continuación autorizada

El usuario instruyó: «continua con el plan hasta completar todas las tareas y
productos». Esta instrucción autoriza avanzar desde G0 con la matriz entregada,
sin volver a solicitar aprobación para desarrollo reversible. No sustituye
prueba institucional ni aprobación jurídica de nuevas reglas CT, ni autoriza
merge, release o envío/registro automático.

## Entrega implementada

- **P2:** contrato 1.0.0, JSON Schema, diccionario, siete tablas (calendarios
  explícitos), fixtures completa/parcial y productor/consumidor de referencia.
  Validador ZIP/CSV/JSON con límites, hashes, identidades, claves ajenas y cobertura.
- **P3/P4:** Excel autónomo de bitácoras con copia íntegra, lecturas, consultas,
  incidencias y textos extensos partidos verificablemente. Exportador desde lotes
  originales comprobados, opcionalmente con bitácoras; sin consulta repetida.
  Ventana de resultados ofrece las tres exportaciones independientes.
- **P5:** importador local a Work, cuatro modalidades, registros de varios tipos,
  fuentes raw/normalizadas, fecha de corte explícita, relación por ingreso remoto,
  rechazo antes de cambiar sesión, reimportación y preservación de decisiones.
  Excel de trabajo reconstruido desde CSV, original ZIP intacto; portabilidad sin
  Office. Fuentes/identidad faltantes generan límites visibles.
- **P6:** GUI principal sin lanzamiento de Descargador/Chrome/HAR. Recuperación
  de resultados locales legados conservada. `registration.submit` inhabilitado
  antes de tocar adaptador; lógica de simulación histórica solo en tests.
  Marcas de firmas revisadas no se guardan ni recuperan como autoridad.
- **P7/P8:** cronología por ingreso, procedencia, hallazgos tradicionales y CT
  explicados; contexto real NO_EVALUABLE hasta aceptación. Preview temporal
  exclusivo de fixtures explícitas, preservando empates y orden incierto. Productos
  preparados se invalidan por fuente/reglas/corte. Editor TT/CC/RES, contactos,
  adjuntos, Word y borradores existentes conservados. Solicitudes declarativas
  validadas por Descargador sin ejecutar consultas desde el archivo.
- **P11/P13 técnico:** pruebas locales, pipeline ficticio, benchmarks y verificación
  del contrato incluida en los dos ejecutables; workflows Windows conservados.

## Diferencias autorizadas y límites de migración

El contrato y las entradas nuevas son aditivos. Las salidas Excel/CSV/ICS/ZIP
anteriores del Descargador siguen disponibles; el botón de Excel para CSMP ya no
lanza otro programa. El Asistente conserva páginas y utilidades manuales.

El payload de Work distingue `locally_validated` de `rus_recorded=false`;
la exportación mantiene campos revisados con la primera señal. El adaptador de
exportación sigue interpretando payloads antiguos para compatibilidad técnica,
sin usarlos como nueva evidencia institucional. La base SQLite anterior y los
comprobantes útiles no se borran; no alimentan la última observación del centro.

Las pruebas históricas de `submit` prueban exclusivamente su simulación en tests;
la API distribuida se prueba separadamente para rechazar incluso antes de leer
journal/adaptador. No se elimina la evidencia del protocolo recuperable.

Al reanalizar mismo ZIP con otro corte/configuración se conservan decisiones
humanas y productos, que pueden quedar obsoletos. Al importar otra captura se
archiva el trabajo anterior antes del cambio. Configuración del usuario y
plantillas personales no se reemplazan. Un texto heredado de correo que afirme
registro/envío no acreditado debe corregirse antes de guardar el borrador del
paquete; el programa no reescribe silenciosamente esa plantilla.

## Aceptaciones que no pueden reemplazarse por fixtures

El usuario confirmó que no hay entorno/evidencia y que SITFA/RUS se usarán
después. Se entrega el MVP técnico local y se difiere esa aceptación.

- **G1/P1:** cotejo de SIN_VINCULO, contenido y cobertura con sesión autorizada,
  cuatro modalidades y pestañas, usando EXE/extensión del mismo SHA.
- **P9/G9:** falta contrato remoto comprobado de historia y PDFs. No se implementa
  un extractor especulativo ni se llama a endpoints no auditados. El contrato puede
  transportar esas tablas cuando existan fuentes válidas; título/metadato no prueba
  contenido. Se mantiene ampliación pendiente, como exige el plan.
- **P10/G10:** CT quedan desactivadas para producción; condiciones, textos y
  aplicación jurídica por tribunal requieren aprobación individual. No se altera
  ninguna regla tradicional ni se unifican motores históricos.
- **P12/G12:** aceptación ampliada depende de P9/P10; no se anuncia completada.
- **P13:** EXE Windows, Excel 2010, Outlook 365 institucional, instalación sin admin
  y políticas Chrome requieren entorno real; una compilación CI tampoco certifica
  aceptación institucional. No hay publicación de release ni merge a main.

## Uso

Descargador → Resultados: **Consolidar XLSX**, **Excel bitácoras autónomo** o
**Paquete CSMP v1**. El paquete usa el lote seleccionado y permite seleccionar
un control de bitácoras ya guardado; cancelar esa selección deja NO_SOLICITADA.
Originales excluidos en UI. API ofrece inclusión explícita bajo política local.

Asistente → Trabajo: **Importar paquete**, elegir ZIP y fecha de corte; revisar
**Cobertura y fuentes**, editar y **Exportar copia actual**. Correos/Resoluciones
consumen la copia local y las decisiones actuales. Resultados permite generar
solicitud de antecedentes que se abre manualmente en Descargador.

Excel legado sigue su ruta anterior y se rotula SOLO_PLANILLA en evidencia.
Sin ID remoto no hay unión de bitácoras por nombre/RIT. Calendarios ambiguos se
conservan NO_ASOCIADO. No se suprimen filas repetidas legítimas.

Un calendario independiente solo entra al motor de Informes si su tipo es
INFORME/INFORMES. Medidas y tipos desconocidos se conservan como OTRO sin
propuesta automática; tampoco aportan fecha al cruce C10 de informes.
La reimportación compara versión del motor/mapeo antes de reutilizar resultados
de una sesión anterior, preservando las decisiones humanas al reanalizar.

```sh
python tools/flow_capture_cli.py validate captura.zip
python tools/flow_capture_fixture.py /ruta/nueva/completa.zip
python tools/flow_capture_pipeline.py /ruta/nueva/productos --rows 100 500 2000
```

El pipeline es exclusivamente ficticio, produce Excel revisable, cobertura,
solicitud, sesión y descriptores de borrador. Para 100 filas también Word/nóminas.
No guarda borradores en Outlook ni ejecuta consultas. Sus destinatarios son `.test`.

Los resultados finales y pendientes por tarea se registran en
`docs/evidencia_desarrollo_20261008/` y `CONTINUAR.md`.
