# Auditoría completa de CSMP Assistant y descargador SITFA

Fecha de revisión: 5 de octubre de 2026  
Alcance: `nurus`/CSMP Assistant 0.4.0.dev17 y descargador SITFA 2.4.0 para Windows.

## Resultado ejecutivo

El flujo local está listo para pruebas con Excel, Word, borradores editables y devolución recuperable entre CSMP y el descargador. La rama publicada originalmente como `experimento/plan-integral-csmp-20261005-dev17` tuvo una corrupción de contenido en `app_base.py`; el commit corregido es `c3feedd357d4ddf88c5caa946b5d33bd80413fe4` y está disponible en `experimento/plan-integral-csmp-20261005-dev17-fix`.

El alcance que todavía requiere validación con una sesión autenticada es el acceso real a RUS. El prototipo prepara propuestas y contratos verificables, pero no debe presentarse como un sistema que ya lee o escribe bitácoras judiciales. Tampoco se acredita un borrador real en Outlook mediante las pruebas simuladas.

## Evidencia revisada

- Código fuente de `work/prototypes/csmp` y `work/prototypes/descargador`.
- Pruebas de CSMP: 459 aprobadas y una omitida en la regresión del runtime dev17.
- CI Windows 3.12, 3.13 y 3.14: ejecución 37352808040, exitosa.
- Pruebas del descargador: 111 casos previos a la comprobación del ejecutable.
- Pruebas de Excel de escritorio: preservación del original, hojas, fórmulas, bordes, formatos y columnas `FECHA_OBS`, `TT` y `CC`.
- Capturas de interfaz `trabajo-1024.png`, `correos-1024-edited.png` y `resoluciones-1180.png`.
- Documentos de flujo integrado, registro recuperable, carga de tribunal y bitácoras.

## Hallazgos

### 1. Corrupción de publicación — corregido, prioridad crítica

La copia publicada de `src/nurus/personal/app_base.py` contenía una línea imposible de compilar (`e,None);`) y otra línea truncada en la pantalla Historial. El error se manifestaba después de instalar el paquete, por eso el traceback apuntaba a `.venv-csmp/Lib/site-packages`.

Se restauró el archivo desde el fuente válido, se verificó con `py_compile` y `compileall`, se creó el commit `c3feedd…` y se recompiló el ejecutable Windows. El proceso de publicación debe verificar el SHA del blob remoto y volver a compilar el archivo descargado antes de mover una rama.

### 2. Bitácoras RUS — pendiente real, prioridad crítica

Los servicios de análisis, clasificación, deduplicación y recibos están probados con adaptadores ficticios. Falta el adaptador autenticado que abra la bitácora del ingreso correcto, lea el texto completo, identifique la última observación de los cuatro meses solicitados y registre la observación del Excel con fecha, `CC` y texto comprobables.

Implementación recomendada: mantener separado el motor puro de clasificación y agregar un adaptador de navegador con estados `LOCALIZADA`, `ABIERTA`, `LEÍDA`, `REGISTRADA`, `CONFIRMADA` y `ERROR`. Cada escritura debe exigir coincidencia inequívoca de tribunal, RIT, ingreso y persona; guardar un recibo local y volver a leer los campos antes de marcarla como confirmada.

### 3. Cobertura de movimientos — parcial, prioridad alta

La carga de 60 días se consulta en bloques de hasta 30 días. La muestra de 169 filas demostró que una consulta puede incluir firmas posteriores al día elegido. Por eso el sistema debe resaltar actividad encontrada, conservar la fecha real de cada resolución firmada y rotular la cobertura como parcial cuando el portal no permita demostrar ausencia.

La comparación debe considerar solamente resoluciones firmadas para la decisión operativa del centro, según el criterio indicado por el usuario. Las etapas de pregrabado, envío a despacho o devolución del juez quedan como trazabilidad secundaria.

### 4. Reglas de negocio acotadas — intencional, prioridad media

Las reglas históricas de correos y matrices Word están limitadas a Laja, Mulchén y Tomé. La selección, carga, alertas y auditoría pueden cubrir otros tribunales, pero el asistente no debe inventar una matriz ni un destinatario para un tribunal que no tenga configuración. El manual debe distinguir esta frontera para evitar que un resultado vacío parezca un error.

### 5. Manejo de excepciones silenciosas — mejora de mantenibilidad

Se encontraron varios `pass` en adaptadores de Outlook, lector RUS, catálogo, exportadores y servicios de contactos. Algunos son tolerancia deliberada frente a APIs opcionales, pero otros pueden ocultar una causa real de datos incompletos. Conviene sustituirlos gradualmente por resultados tipados, registro local de diagnóstico y mensajes accionables. No se debe mostrar un stack trace de Office al usuario, pero sí indicar qué registro o archivo quedó sin comprobar.

### 6. Publicación y metadatos — corregir en el próximo release

Los documentos históricos aún mencionan dev14, ramas antiguas y la ausencia de algunas mejoras que sí están presentes en dev17. El README y el manual de esta rama deben enlazar siempre la rama `-fix`, el commit y la versión del paquete. La publicación debe ser atómica: crear blobs, verificar tamaños/SHA remotos, compilar y recién después mover la rama.

### 7. Exportación Excel — funcional con regresiones recomendadas

La devolución preserva el libro original, aplica bordes por celda y muestra `Sin dato` cuando falta vencimiento, espera o egreso. Se recomienda agregar una prueba parametrizada para Espera, Cumplimiento e Informes y para las tres extensiones XLS/XLSX/XLSM que compruebe borde, fecha vacía y fecha corregida en la misma fila.

### 8. Outlook — preparado, pero no acreditado en cuenta real

La interfaz permite editar Para, CC, asunto, cuerpo y adjuntos, y las pruebas verifican que no se invoque el adaptador con datos inválidos. El resultado demuestra preparación segura de borradores; no demuestra que una cuenta Outlook concreta haya creado el borrador. La aceptación final debe registrar el identificador local, asunto, destinatarios y archivo adjunto después de la creación.

## Fidelidad frente a lo solicitado

| Requisito | Estado | Evidencia o límite |
|---|---|---|
| Reglas de Espera/Cumplimiento/Informes para Laja, Mulchén y Tomé | Implementado | Motor, plantillas, matrices y pruebas dev17 |
| Normalización de Mulchén y residenciales RTA/RTT/RVA | Implementado | Pruebas de clasificación y tarjetas de correo |
| Bordes y fechas de vencimiento/espera/egreso | Implementado | Exportación nativa y comprobaciones Excel |
| Descarga conjunta y recuperación tras reinicio | Implementado localmente | Diario, solicitudes y recuperación; falta prueba autenticada |
| Consulta de actividad de 60 días y alertas | Parcial | Contrato y cruce local; cobertura real del portal pendiente |
| Revisión de bitácoras de cuatro meses | Motor preparado | Adaptador RUS real pendiente |
| Registro automático de observaciones en RUS | Pendiente | No se debe simular como completado |
| Informe de última observación, repetidas, contestadas y cargas | Motor preparado | Falta alimentar con bitácoras reales |
| Manual de usuario interactivo | Incorporado en esta rama | `docs/manual_usuario_csmp.html` |

## Plan de cierre

1. Probar con una sesión RUS autenticada una sola causa de cada modalidad y guardar los recibos de lectura sin escribir.
2. Validar el registro de una observación de prueba autorizada, reabrir la bitácora y comparar texto, fecha y `CC`.
3. Ejecutar el lote de 60 días para un tribunal, cruzar únicamente resoluciones firmadas y revisar las alertas en Excel.
4. Repetir la aceptación para Laja, Mulchén y Tomé; luego comprobar que un tribunal sin matriz queda claramente identificado como pendiente de configuración.
5. Publicar el commit final solo después de verificar que el blob remoto compila y que el paquete arranca desde una carpeta extraída nueva.

## Conclusión operativa

El prototipo es apto para pruebas de escritorio y para validar el flujo de descarga, análisis, edición y preparación. El acceso real a bitácoras y el registro judicial siguen siendo el límite principal y deben cerrarse con una sesión RUS controlada antes de llamar a la solución “automática”.

