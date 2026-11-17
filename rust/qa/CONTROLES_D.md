# Matriz de paridad de controles · Estudio D

Base inspeccionada: cfc12ee2af5e4fb5623671fd17ccf7feb9e1cc11. Inventario obligatorio:
Flow_Auditoria_Controles.html del kit del usuario. Las rutas son relativas a rust/crates/flow/src.
Esta matriz distingue revisión del código y comprobación automática de aceptación del usuario.

| # | Control | Ruta final y comportamiento conservado | Comprobación |
|---|---|---|---|
| 1 | Abrir/Reanudar/Corte/Etapa | ui.rs::barra; Archivo y contexto compacto; Revisión muestra su propio paquete | Revisión de controladores; capturas reales |
| 2 | Volver a analizar | Más → Recalcular propuestas; confirmación antes de descartar | Controlador existente; escena recalculo |
| 3 | Reanudar último | Archivo; nombre/corte y deshabilitado sin sesión | almacen existente; sin-sesion y recuperación automatizada |
| 4 | Ver etapa/cantidad | Selector de tablas con conteos; cada Trabajo conserva su sesión | Pruebas importación multietapa y sesión |
| 5 | Revisada/selección | Una Revisada en editor; selección de tabla acota productos | Pruebas de selección/ediciones; captura editor |
| 6 | Restaurar/Copiar | Junto al editor; Copiar observación y restauración individual | Revisión del controlador y capturas |
| 7 | Preparar tipo/conjunto | Dos acciones en Correos; controladores distintos, con ayuda de reemplazo | Pruebas preparar_tipo y preparar_borradores |
| 8 | Otra modalidad | Etiqueta y ayuda sobre texto libre; claves_modalidades conserva None | Pruebas de alcance/modalidades existentes |
| 9 | Período | Opciones del reporte solo Proyectos; el valor permanece en App | Verificado consumidor PERIODO en datos/correos.json |
| 10 | Outlook/eml/recibo/incierto | Mismos servicios y resolución explícita; sin reintento automático | Dobles de guardado, no duplicación y escena incierto |
| 11 | Excluir borrador | Casilla conserva omitido y conteos incluidos/excluidos | Pruebas de guardado; no depende del filtro visual |
| 12 | Preparar/Generar Word | Acciones separadas; conserva preparación automática obsoleta al generar | Pruebas Word y vigencia; escena proyecto-obsoleto |
| 13 | Plantillas propias | Menú Plantillas en Resoluciones | Mismo destino/controlador; revisión independiente |
| 14 | Abrir/cambiar salida | Resultados separa carpetas producidas de destino próximo | Registro de carpetas previas; pruebas biblioteca |
| 15 | Importar CSMP/catastro | Menú Importar; vista previa y confirmar sin modificación directa | Pruebas catastro y CSMP; revisión independiente |
| 16 | Releer/Guardar Flow | Contextual ante fallo/legado solo lectura | Controladores conservados; pruebas de contactos |
| 17 | Paquete/Descargar | Captura diferencia paquete para Asistente de descarga de lote | Pruebas captura/paquete y fixture Chromium MV3 |
| 18 | Retomar paquete/lote | Menú Retomar con rutas y controladores separados | Recolección/reanudación automatizada |
| 19 | Crear paquete de lote | Etiqueta distingue actual/externo; ruta externa accesible | Exportador y controladores existentes |
| 20 | Leer bitácoras | Paquete indica incluidas; descarga conserva casilla configurable | Pruebas exportación y contrato de evidencia |
| 21 | Filtros/PDF vacío | Avanzado con resumen de valores activos; mismos valores iniciales | Revisión de estado y captura |
| 22 | Conexión/código/cancelar/detener | Conserva copia/cierre; Detener descarga conserva avance | Pruebas de pausa, cancelación y tope en Captura |
| 23 | Agregar archivo/carpeta | Menú Agregar; accesible con foco de teclado | Misma incorporación de entradas; captura y Tab real |
| 24 | Vaciar Anonimizador | Quitar todas las entradas y quitar individual; no borra archivos | Lista modificada en memoria, control de vacío |
| 25 | Conservador | Selector Estricto/Conservador; Estricto inicial | Contrato anonimizador Rust/Python |
| 26 | Semilla/fechas | Avanzado y resumen; semilla solo memoria, política explícita | Contrato de anonimización y revisión de persistencia |
| 27 | Tribunales Meta4 | Todos/Elegir; elegir vacío deshabilita generación | Mismo contrato de subconjunto; revisión y captura |
| 28 | Sin plantilla | Usar formato estándar; mantiene plantilla=None | Mismo controlador; pruebas Meta4 |
| 29 | Generar/Regenerar | Dos rutas, nueva carpeta y acompañantes conservados | Suite Meta4 existente |
| 30 | Quitar/salida/abrir Meta4 | Quitar todos los lotes, quitar individual, salida y abrir resultados | Controladores conservados; captura Meta4 |

## Invariantes y evidencia adicional

- flow-motor, textos tradicionales, golden y permisos MV3 no se modifican.
- La búsqueda/filtrado de listas no cambia selección, destinatarios ni omitido.
- Los servicios Outlook/Word se reutilizan; las pruebas con dobles no acreditan la sesión real.
- La revisión de bitácoras calcula sobre el paquete original completo, después filtra las propuestas.
- La autoría/cobertura se añade al contrato existente. Un paquete legado sigue legible;
  cuando falta prueba suficiente, la clasificación es No verificable.
- La exportación XLSX/JSON se prueba con cancelación intermedia, fallo JSON y colisión de carpeta.
- Restaurar distribución usa identificadores nuevos de panel; no migra ni borra sesiones.

La matriz de capturas generada por pruebas/ui_evidencia.py registra tamaño físico,
tamaño efectivo, escala y foco. Generar una imagen no demuestra por sí solo que todos
los controles funcionan; se combina con las pruebas de controlador y revisión independiente.

## Verificador general: diferencia respecto de la base

Run 44153149631, candidato 315b1f8 y base cfc12ee2, mismo entorno Python 3.12:
ambos FALLA con hallazgos idénticos. Son 38 bajas de inventario Asistente, 10 de
Descargador y `src/nurus/personal/commands.py importa subprocess`. El comparador
conserva el código de error original; no regenera baseline ni golden.

Pasan en ambos las suites (1283 Asistente, 382 Descargador y 62 subpruebas), paridad
golden, identidades reales=0, Ruff=0, módulos no alcanzables=0 y enlaces rotos=0.
Artefacto Flow_D_Verificacion_Plan del run conserva los cinco informes. Esto identifica
deuda heredada y no equivale a autorización para integrar con el verificador fallido.

