# Paridad funcional en la aplicación vigente

Estado de `app/` al reiniciar el trabajo el 20 de noviembre de 2026. «Presente» significa que existe un flujo local. El cierre exige comparar con Python mediante la prueba indicada. La [matriz del estudio anterior](https://github.com/MatiGaete2023/flow/blob/ed6a0e43f8efa0856e0294af92ac8e0317c69df7/docs/migracion_python_rust/MATRIZ.md) conserva su inventario detallado.

| ID | Capacidad | Estado y comprobación en `app/` |
|---|---|---|
| F01 | Lectura de hojas | Presente en `lectura.rs`. Comparar selección, tipos, fila física y CSV con comillas. |
| F02 | Trabajo y cruces | Tres etapas, Hoja 2, calendario 91 y proyección por ingreso. CAL-03 contrasta vencimientos de Cumplimiento e Informes. Pendientes Egresados, OB e identidad multifila. |
| F03 | Reglas de Excel | Presente en `motor.rs`. Revalidar el subconjunto sin historia y reproducir fronteras contra Python actual. |
| F04 | Contexto de causa | Trabajo muestra Escritos, Mejor Niñez, historia, calendario y cronología de bitácoras al corte. El motor incorpora todas las reglas de historia y bitácora de Flow Python. `oraculo_historia` coincide en los 2.582 casos del corpus con historia o bitácora, y `oraculo_python` en los 174 restantes (23-nov-2026). La consignación manual de otro programa está en el detalle de Trabajo y se guarda en la sesión. |
| F05 | Otro programa, reiteración y Escritos | Reiteración con R5, programas vigentes y atribución de Escritos e Historia por ingreso y vencimiento. El contraste de 16 filas verifica textos, reglas, marcas y acciones. Pendientes consignar y deshacer constancias de otro programa. |
| F06 | Revisión de bitácoras | Evidencia V1 y V2, período, filtros, detalle, selección y Excel/JSON. El informe aplica la misma copia preservada o tabla de respaldo que **Guardar observaciones** y añade clasificación y motivo por ID de registro. |
| F07 | Importación de paquetes | Validación estructural y semántica, consolidado y apertura tabular integrados en Trabajo. Pendientes conciliación entre capturas, calendario independiente y paridad de contextos. |
| F08 | Edición y revisión | Editor, filtros, selección y marcas presentes. Comparar edición masiva, deshacer y selecciones ocultas. |
| F09 | Sesión y vigencia | Estado local por archivo, etapa y corte presente. Pendientes recibos, vigencia, compatibilidad y recuperación. |
| F10 | Excel preservado | Probado con Excel de escritorio sobre originales y consolidados ficticios. Conserva RIT, columnas NURUS, decisiones, lista RES y trazabilidad. Pendientes reimportación, conciliación y aceptación con archivos del usuario. |
| F11 | Configuración y contactos | Ajustes, plantillas e importación presentes. Cotejar contactos duplicados, variables y migración de formatos. |
| F12 | Correos y nómina | Preparación, nómina editable por celda, contactos importados de Flow Python y aviso sin destinatario. Cotejar agrupación, firma y alcance. |
| F13 | Borradores Outlook | EML y guardado Outlook presentes. Simular fallos antes y después de guardar y validar recibos. |
| F14 | Proyectos Word | Matrices institucionales de Laja y Mulchén incluidas, fechas en palabras, plural y sin cursivas. Los escritos incorporados sin ingreso posterior proponen `NOMENCL` con la nomenclatura correspondiente. Pendiente confirmar la entrega de un .docx por proyecto. |
| F15 | Formularios manuales | Cotejar gestión particular en blanco. Completar Word manual y reutilizar los productos existentes. |
| F16 | Resultados y estadísticas | Vista y reporte presentes. Cotejar búsqueda, historial, diagnóstico, calidad y composición de entrega. |
| F17 | Descargador integral y Lite | Perfiles, catálogos desde JSON o Chrome, instalación de las extensiones incluidas y ejecución de lotes Lite contra SITFA ficticio. Pendientes ejecución Integral, Carga y prueba con Chrome real. |
| F18 | Gestión de lotes | Planificación, ejecución Lite, pausa, cancelación, manifiestos, `resumen.json` y PDF de consulta vacía. Preferencias persistentes de destino, PDF sin registros y pausa, con el mínimo de 0,5 s y el máximo de 10 s de Python. Favoritos (alcance sin filtros privados, fechas de los últimos 30 días y mes relativo en calendarios) y recuperación de consultas pendientes de un lote incompleto desde `resumen.json`. Los topes de tabla de `motor.table_grid` (200.000 filas, 200 columnas y 2.000.000 celdas) ya estaban en `xls.rs` y `sitfa/registros.rs`. Pendiente el resto de `preferencias.py`: historial, paquete y lecturas opcionales. |
| F19 | Bitácora, historia y calendario | Pendiente en el Descargador vigente. Probar identidad, ventanas, procedencia y respaldo alterado. |
| F20 | Escritos | Pendiente. Diferenciar tabla vacía, ausente, variante con pestaña y lecturas parciales. |
| F21 | Histórico de Protección | Pendiente. Evaluar titular visible, reutilización, marco compatible, tope y suspensión por fallas. |
| F22 | Exportaciones de lote | Pendiente. Comparar CSV, XLSX, ICS, ZIP, consolidado y comparador de lotes en ambos sentidos. |
| F23 | Cálculo Meta 4 | Pendiente. Cotejar mes, fechas, transiciones, procedencia y carpetas Python históricas. |
| F24 | Plantilla y regeneración Meta 4 | Pendiente. Elegir plantilla y reproducir productos tras correcciones de registros. |
| F25 | Anonimizador | Presente para tablas y texto. Completar archivos, carpetas, paquetes y verificación posterior de XLSX y otros formatos. |
| F26 | Distribución y diagnóstico | Existe un binario de escritorio. Pendientes empaquetado, instalación portátil y verificación de herramientas finales. |
| F27 | Intercambio entre herramientas | Pendiente. Probar solicitud local, devolución, procedencia, cancelación y comparación temporal. |
| F28 | Utilidades y catálogos | Inventario previo disponible. Registrar equivalencia o reemplazo de cada herramienta de operación y conservar las de desarrollo aplicables. |

## Inventario de Ajustes

Comparación de `src/nurus/personal/app_base.py::_config_page` y `config.py` de Flow Python con `app/src/ui.rs` y `config.rs` de Flow Rust. Indica también las opciones ubicadas en otras vistas de Flow Rust.

| Opción en Flow Python | En Flow Rust | Falta para la paridad |
|---|---|---|
| Parámetros y búsqueda de umbrales | Ajustes muestra los 16 umbrales de Flow Python, incluidos `prorroga_reciente` (90 días) y `ventana_busqueda` (60 días), con restablecimiento individual y aplicación a la nómina. | Búsqueda de umbrales presente, como `_filter_config_parameters`. |
| Cinco interruptores de avisos y reiteración | Ajustes permite habilitar o deshabilitar la regla desde el selector de textos, incluidos Curador, Oído, Próxima audiencia, Próxima mayoría y Reiteración. | Cotejar persistencia y trazabilidad al cambiar de etapa y reabrir. |
| Textos de observación y variables | Ajustes permite editar textos, restituir el original y activar la regla. La redacción mensual tiene interruptor propio. | Agregar el selector de variables y validación de campos por regla. |
| Plantillas de correo, variables, comportamiento y archivo | Correos genera, edita y guarda borradores con las plantillas configuradas. | Incorporar edición persistente, duplicación, archivado, categoría, variables y condiciones de adjuntos. |
| Contactos, importación, alias y respaldo | Contactos permite editar programas y tribunales, importar el listado de Flow Python y configurar CC adicional y firma. | Agregar edición de alias y cotejar respaldo, borrado y contactos duplicados. |
| Cuenta Outlook, firma y CC | Correos ofrece borradores Outlook o archivos EML. Contactos conserva firma y CC adicional. | Agregar elección de cuenta Outlook y comprobar el destino en un perfil con varias cuentas. |
| Matrices Word y paquetes de plantillas | Ajustes abre la carpeta de matrices propias. Resoluciones usa las matrices incluidas o el archivo propio correspondiente. | Integrar editor/importador por tribunal y tipo y la importación de paquetes ZIP. |
| Datos guardados, exportación de caso anónimo y métricas | Ajustes abre la carpeta de datos locales. Anonimizador tiene su propia vista. | Cotejar gestión de sesiones, exportación anónima para reproducir casos y métricas voluntarias sin información personal. |
