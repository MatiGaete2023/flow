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

La prueba 8 es la más importante: el acceso a Outlook se escribió sin poder ejecutarlo. Si falla, los `.eml` de la
prueba 7 cumplen la misma función.
