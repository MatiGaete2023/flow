# Flow Estudio D — ejecución en curso

Base: cfc12ee2af5e4fb5623671fd17ccf7feb9e1cc11.
Rama: feat/flow-estudio-d-20261117. PR borrador: #59.
Fecha: 17 de noviembre de 2026. No es todavía la entrega final para pruebas de usuario.

## Decisiones confirmadas

- B1: intervalo capturado con fechas ajustables, límites inclusivos.
- B2: ampliar metadatos conservando compatibilidad. La extensión y sus permisos no cambian.
- Autoría: el lector valida tabla/HOBS_Centro y formulario del ingreso antes de
  emitir origen=centro. Los campos generalizados del CSV legado solos no acreditan ausencia.
- Lecturas antiguas reutilizadas sin evidencia original no reciben completitud nueva.

## Estado por paquete

| Paquete | Estado | Evidencia / trabajo restante |
|---|---|---|
| P0 | EN_CURSO | Base e instrucciones revisadas; sin Cargo local. Descarga directa falla. Desarrollo y CI mediante GitHub. Falta baseline completo de verificar_wp.py. |
| P1 | EN_CURSO | Tema D compartido, controles 28 px, cuerpo 15 px, navegación adaptable. Faltan contraste medido, foco, panel activo en ventanas pequeñas y capturas de escalado. |
| P2 | EN_CURSO | Editor ancho inferior, tabla/evidencia superiores, confirmación de recálculo. Faltan restauración de distribución, agrupación de etapas y apertura/recuperación contextual. |
| P3 | EN_CURSO | Etiquetas de correos y menú Plantillas. Faltan reorganización completa de Correos y Resoluciones y sus verificaciones visuales. |
| P4 | EN_CURSO | Anonimizador con modos y quitar individual/todas; Meta 4 con Todos/Elegir; Captura con etiquetas explícitas. Faltan opciones avanzadas, agrupación Retomar y diseño final. |
| P5 | EN_CURSO | Importar agrupado y destino rotulado. Faltan bibliotecas tabulares de Resultados/Contactos y ayuda final. |
| P6 | EN_CURSO | Evidencia aditiva, clasificador, identidad compuesta, cotejo metadata/CSV, duplicados y ZIP acotado. Pruebas Linux del correctivo pasaron; falta confirmar Windows de la punta. |
| P7 | EN_CURSO | Ruta del motor completo y selección posterior, huella/versión, ediciones separadas. Prueba demuestra que aislar la fila cambia el resultado. Revisión independiente estática cerró cuatro hallazgos. Falta verificación final de punta. |
| P8 | EN_CURSO | Vista conectada en Asistente/combinado; filtros, evidencia, propuestas, XLSX+JSON todos/seleccionados. Exportación probada en Linux con cancelación intermedia/fallo JSON/colisión. Vista recién enviada a CI; falta QA visual, revisión independiente y Windows final. |
| P9 | PENDIENTE | Revisión P6/P7 realizada; falta cierre P8 y revisión del candidato completo. |
| P10 | PENDIENTE | Binario candidato final, manual, matriz de paridad, capturas, inventario/huellas y paquete de entrega. |

## Verificación ya observada

- Punta inicial 1ba7ffe: Linux y Windows pasaron completos (run 44113902091).
- b192b88: Linux pasó; siguientes commits iniciaron verificación nueva.
- 26d8b817: Linux pasó con fixtures de los cuatro hallazgos P6/P7.
- da317bf8: Linux pasó con publicación no-clobber y pruebas de fallo/cancelación.
- f0508363: Linux pasó con la vista integrada; Windows seguía en pruebas al escribir este estado.
- Motor, textos tradicionales y golden existentes permanecen intactos.
- No se ha probado visualmente escalado, Outlook real ni captura real; no marcar esos casos como pasados.

## Revisión independiente y cierre

P6/P7 encontró y corrigió:
1. Discrepancia entre metadata y tabla de bitácoras: cotejo bidireccional.
2. Notas iguales con IDs distintos: identidad y momento de evento conservados.
3. Cambio a intervalo anterior de una captura: cobertura real conservada.
4. Costo cuadrático de consultas: índice por ingreso, fixture de 2000 ingresos.

P8 encontró y corrigió identidad habitual ausente del Excel, carrera de publicación
POSIX y falta de pruebas intermedias; la revisión independiente cerró esos tres puntos.
La revisión de UI encontró la marca Revisada restringida a filas con propuesta: se
separó del editor y se añadió prueba para las cuatro categorías. La evidencia en la
pantalla se presenta en lenguaje legible, sin exponer JSON como interfaz principal.

## Continuación inmediata

1. Comprobar y corregir CI de f0508363, especialmente préstamos de la nueva UI.
2. Completar P1–P5 contra la matriz obligatoria de 30 controles del kit.
3. QA sintética real de todas las pantallas y escalas; revisión P8/P9 independiente.
4. Construir y descargar candidato Windows final con manual, fixtures y huellas.
5. Auditar cada requisito P0–P10 antes de declarar el producto listo para pruebas de usuario.

No fusionar, publicar release ni afirmar aceptación. No usar datos reales ni ejecutar efectos externos.



