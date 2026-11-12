# Guía de pruebas del usuario en Windows

Lo que sigue **no se ha podido probar fuera de Windows real** y solo lo puede confirmar el usuario. Usa primero
datos ficticios (`docs/evidencia_p0/golden/Origen_ESPERA.xlsx`) y después un Excel real, sin compartir nunca el
archivo ni capturas con datos reales.

Anota PASA o FALLA y, si falla, el texto exacto del mensaje.

| # | Prueba | Cómo | Esperado |
| --- | --- | --- | --- |
| 1 | Abre | Doble clic en `flow.exe` | Ventana «Flow» sin consola negra |
| 2 | Lee `.xlsx` | Abrir `Origen_ESPERA.xlsx`, corte 08/10/2026 | 3 registros con observación |
| 3 | Lee `.xls` de SITFA | Abrir un `.xls` recién descargado | Etapa detectada y registros correctos |
| 4 | Observaciones | Comparar 10 filas con lo que propone el Asistente actual | Mismo texto |
| 5 | Edición | Editar una observación, cerrar, abrir y «Reanudar último» | La edición sigue ahí |
| 6 | Excel | Ctrl+S | Libro nuevo con OBSERVACION, FECHA_OBS, RES y REVISAR |
| 7 | Borradores `.eml` | Correos → Preparar → «Guardar como archivos .eml» → abrir uno | Outlook lo abre sin enviar, con CC a ucc y la nómina adjunta |
| 8 | **Outlook** | Correos → «Guardar en Borradores de Outlook» | Aparecen en Borradores; nada en Enviados |
| 9 | CC fijo | Revisar cada borrador | Todos llevan `ucc_concepcion@pjud.cl` |
| 10 | Word | Resoluciones → Preparar → Generar Word | Un documento por tribunal y tipo; un proyecto por página; formato igual al de la plantilla |
| 11 | Tildes y ñ | Nombres, programas y carpetas con tildes | Se ven y se guardan bien |
| 12 | Antivirus | Copiar `flow.exe` al equipo de trabajo | No lo bloquea (si lo bloquea, anotar el aviso) |
| 13 | Conexión Chrome | Instalar la extensión incluida, abrir SITFA, Captura → copiar código → icono Flow → conectar | «Chrome conectado»; las opciones corresponden a la pantalla elegida |
| 14 | Captura de varias páginas | Elegir una consulta conocida con varias páginas | XLS y `resumen.json`, conteos correctos; al terminar restaura los filtros |
| 15 | Cancelar y retomar | Detener entre páginas y retomar la misma carpeta | Conserva lo verificado y completa sin duplicar; rechaza un archivo alterado |
| 16 | Consulta vacía | Activar PDF y consultar una selección conocida sin registros | PDF con selección visible; ningún XLS vacío presentado como descarga válida |
| 17 | Captura → Trabajo | Llevar las descargas de Seguimiento a Trabajo | Etapa y registros correctos; se pueden preparar productos tras revisarlos |

| 18 | Lecturas adicionales | Activar bitácoras, historia y calendario en una consulta pequeña | Tres lecturas por registro cuando existen enlaces comprobables; contextos asociados al ingreso correcto en Trabajo |
| 19 | Reintentar lectura | Detener o interrumpir una lectura y retomar la misma carpeta | Conserva lo verificado; reintenta lo pendiente, muestra errores sin convertirlos en listas vacías |
| 20 | Reutilización del día | Repetir la consulta en otro lote de la misma carpeta padre | Reutiliza lecturas verificadas de hoy; al ampliar el período vuelve a leer bitácora e historia |

| 21 | Cronología | Abrir un ZIP o lote con bitácora y mirar el detalle de una fila | Texto íntegro, respuestas y posteriores; los empates sin hora fiable aparecen como inciertos |
| 22 | Reiteración | Cargar una fila cuya propuesta repita la última observación del centro | Frase de reiteración con fecha; texto previo en Revisar y las mismas acciones; persiste al reabrir la sesión |
| 23 | Avisos de bitácora | Comparar un caso conocido de Al tribunal sin respuesta y un E-05 antiguo | CT-05/CT-11 solo con origen acreditado y el plazo cumplido; no crean acciones nuevas |

| 24 | Egresados | Importar un ZIP o lote con Egresados e Informes del mismo ingreso | Egresados visible, sin propuesta ni acciones; N-04 si queda informe pendiente; Q-03 ante estados incompatibles |
| 25 | Búsqueda Lit | Importar el XLS del menú 94 o su lote/ZIP | Columnas conservadas, avisos OB solo para órdenes activas; no genera correos ni proyectos |
| 26 | Calendario por ingreso | Comparar una fila con proyecciones y Hoja2 | Fechas y estados visibles; Hoja2 conserva prioridad y CAL-03 avisa diferencias una vez por vista |

La prueba 8 es la más importante: el acceso a Outlook se escribió sin poder ejecutarlo. Si falla, los `.eml` de la
prueba 7 cumplen la misma función.

| 27 | Anonimizador | Agregar dos archivos ficticios que compartan un nombre y crear copias | Mismo seudónimo, nombres de archivo neutros e INFORME.json; originales intactos |
| 28 | Cancelación de anonimización | Cancelar una selección de prueba mientras procesa | Mensaje de cancelación; sin copias parciales publicadas |
| 29 | ZIP anonimizado | Anonimizar un CSMP_CAPTURE ficticio y abrir la copia en Trabajo | Huellas válidas, referencias coherentes; adjuntos no admitidos excluidos e informados |

La anonimización requiere revisar las copias antes de compartirlas, especialmente en modo Conservador.

| 30 | Meta 4 | Capturar Espera y Cumplimiento de FAE o Residencia de un tribunal con bitácoras; Meta 4 → agregar el lote → Generar | Carpeta nueva con cinco archivos; cada registro en su hoja y tribunal |
| 31 | Meta 4: fecha | Comparar FECHA OBS. de 10 filas con la bitácora en SITFA | Es la última observación del centro de Espera o Cumplimiento dentro del mes; las de Informes no cuentan |
| 32 | Meta 4: alerta | Buscar un registro en Cumplimiento cuya última observación sea de Espera | Fila roja con nota; aparece en «Alertas y revisión» |
| 33 | Meta 4 frente al actual | Generar el mismo mes con la herramienta Meta 4 actual y comparar totales por hoja | Mismos totales; anotar cada diferencia |
| 34 | Calendario con «- - -» | Repetir la captura que dio «fecha ilegible en Fecha Informe» | El calendario se lee; ya no aparece el aviso |

Meta 4 se construyó sobre las reglas de la herramienta actual pero **no se comparó archivo contra archivo**, porque
lee otro formato de lote. La prueba 33 es la que lo confirma.


| 35 | Meta 4: regenerar | Genera Meta 4 con datos ficticios; cambia FECHA OBS. y ETAPA ÚLTIMA OBS. en Registros.xlsx y usa «Regenerar desde Registros revisado…» | Una nueva carpeta con cinco archivos, fecha corregida y ALERTA recalculada, aunque se haya escrito a mano una ALERTA incorrecta |
| 36 | Meta 4: conservación y errores | Repite la regeneración con la misma salida; prueba sin Bitacoras_del_mes.xlsx y con FECHA OBS. ilegible | Se rechazan esas entradas, no quedan productos parciales y los originales siguen intactos |

| 37 | Plantilla personalizada | Seleccionar en Meta 4 una plantilla XLSX ficticia con otro color de cabecera y otro ancho; generar y regenerar | Estilos y anchos visibles en las cuatro hojas, fechas día-mes-año, totales correctos; no cambia la plantilla original |
| 38 | Plantilla incompatible | Elegir una plantilla XLSX que no tenga todas las hojas o falte una fila modelo | Mensaje concreto de error, sin carpeta final ni modificación de archivos originales |

Las pruebas automáticas verifican estilos, anchos, alturas, fechas y que el contenido de las
hojas no cambie. **Falta la comprobación funcional en Windows real con SITFA, Outlook y Excel
2010/365**, incluida la comparación Meta 4 frente al producto Python (prueba 33). Ninguna
prueba automática equivale a la aceptación del usuario.
