# Checkpoint de recuperación de NuRus
Fecha: 9 de septiembre de 2026.

## [G-OBJETIVO]
Terminar la integración autorizada de Espera, Cumplimiento, Informes, generación de correos/proyectos y contador en NuRus; verificarla y publicar el código en la rama de trabajo.

## [G-REGLAS]
- Repositorio: MatiGaete2023/NuRus. Rama: implementacion-plan-2026-09-08. No integrar en main antes de la aceptación.
- Windows 10, Python 3.12, Excel 2010 y Outlook clásico institucional; instalación aislada sin asumir administrador.
- Nunca enviar correos ni escribir en SATURNO. Borradores con confirmación explícita.
- CC obligatoria: ucc_concepcion@pjud.cl, incluso cuando Para quede vacío.
- Conservar originales y procedencia: archivo, hoja, fila física y SHA-256. Excluidos visibles y coloreados.
- Conservar hojas, fórmulas, estilos, anchos, filtros y demás estructura. OB y Medidas vencidas no se procesan.
- Ausencia de hoja cruzada en Cumplimiento: excepción documentada, no aprobación silenciosa.
- No publicar planillas con datos personales, bases SQLite ni productos de causas en GitHub.

## [G-ESTADO]
**Estado actual:** recuperación operativa mediante GitHub y GitHub Actions. A los cambios remotos de correo, contactos y selección de hojas se añaden aprobación transaccional, snapshots históricos, rechazo de vistas previas antiguas y respaldo de migración (bef83f7), además de protección del archivo original frente a exportación sobre la misma ruta o identidad (1b93196). Se incorpora conservación atómica de los bytes originales por SHA-256, lectura desde la captura y migración con respaldo a v4 (67a23b7). La exportación revisada desde esa copia quedó publicada en e1f7452: modo nativo de Excel para Windows, anotaciones de revisión, trazabilidad oculta y filas excluidas coloreadas. Cumplimiento ahora exige excepción documentada de hoja de cruce, persistida en el snapshot; el instalador detecta Python 3.12 sin depender solo de py (81769e). La implementación local anterior sigue sin recuperarse; no se presenta como publicada.

**Verificación remota vigente:** 80 pruebas aprobadas en Linux y 79 aprobadas más 1 omitida en Windows, sobre 81769e. Evidencia: [Actions 34369824759](https://github.com/MatiGaete2023/NuRus/actions/runs/34369824759). La omisión exige Excel de escritorio real. Ver INSTALACION_Y_EXCEPCION_CUMPLIMIENTO_20260909.md para recuperación de instalación, excepción y aceptación. La aceptación con Excel 2010 y Outlook clásico continúa pendiente.

**Base remota de la recuperación inicial:** ad5c7c4 y checkpoint abfa54a. Los avances remotos posteriores se documentan en RECUPERACION_EJECUTADA.md. Las pruebas de GitHub Actions corresponden a esos commits; no al código local inaccesible.

**Comprobado en la ejecución anterior:** 52 pruebas aprobadas en 13,50 segundos. Evidencia: salida de pytest de la sesión 19350, conservada en la conversación. Esa ejecución precede a las últimas modificaciones del contador, respaldo e importación de contactos. No equivale a validar el estado final ni el código remoto.

**Última ejecución pendiente:** sesión 14420, iniciada después de esas modificaciones; su resultado no pudo recuperarse al quedar indisponible el entorno. No atribuirle un resultado positivo.

**Limitaciones:** sin acceso actual al filesystem ni a ejecución Python; tampoco Excel/Outlook reales en Windows. No se comprobó pérdida de los archivos locales: están inaccesibles. No reconstruir archivos completos a partir de este resumen.

## Avance local que debe recuperarse y contrastarse
Directorio anterior: /workspace/scratch/4cec07edc980/nurus-implementation
No era un checkout Git inicializado. El directorio /workspace/scratch/4cec07edc980/NuRus es otra copia; no asumir que contiene estos cambios.

| Componente | Ubicación local relativa | Estado antes de la interrupción |
|---|---|---|
| Origen en bytes, snapshots inmutables, aprobación transaccional y excepción | src/nurus/storage/database.py; rus/models.py; services/workflow.py | Cubierto por la ejecución de 52 pruebas, salvo respaldo posterior |
| Selección de hoja por modalidad y prioridad de ingreso efectivo en Informes | src/nurus/rus/reader.py; columns.py | Pruebas previas aprobadas |
| Exportación sobre copia, trazabilidad y exclusiones coloreadas | src/nurus/services/workbook_export.py | Alternativa portátil probada con referencias; backend Excel nativo sin aceptación Windows |
| Catálogo de nueve correos y CC obligatoria | services/operational_templates.json; communications.py; policy.py | Núcleo probado; revisión visual pendiente |
| Adjuntos limitados por selección y hash | services/communications.py; products.py | Pruebas de subconjunto y alteración aprobadas |
| Borradores con comprobante y bloqueo de duplicación incierta | adapters/outlook.py; services/delivery.py | Pruebas con simulaciones aprobadas, no Outlook real |
| Cinco matrices históricas editables y exportación DOCX | services/resolution_templates.json; resolutions.py | Pruebas de texto/estructura aprobadas; aceptación visual y operativa pendiente |
| Interfaz de productos, contactos, políticas e historial | src/nurus/app.py; product_ui.py | Implementada localmente; no declarar validación visual |
| Contador de Enviados por período, solo lectura | adapters/sent_mail.py; app.py; tests/test_sent_mail.py | Añadido después de las 52 pruebas; resultado final pendiente |
| Respaldo SQLite antes de migración y rechazo de degradación | storage/database.py; tests/test_integrity.py | Última ejecución pendiente |
| Duplicados de contactos, UUID real para alias y rechazo de vista previa antigua | services/contacts.py; tests/test_contacts_import.py | Última ejecución pendiente |

## Límites de compatibilidad que no deben omitirse
1. La alternativa portátil de Excel requiere consentimiento explícito por fidelidad reducida. Haber comprobado fórmulas, valores, anchos y hojas de las muestras no acredita preservación universal de todos los objetos Excel. La interfaz usa el backend nativo.
2. La salida conservadora .xls no estaba implementada; no declarar compatibilidad completa BIFF.
3. Faltan matrices acreditadas de Tomé y de pide-cuenta de informes de Laja. No sustituirlas por textos de otro tribunal.
4. Los proyectos son borradores no firmados y requieren revisión de procedencia. Las pruebas de texto no acreditan vigencia jurídica ni equivalencia visual con un Word final.
5. Los informes privados usados en pruebas son pruebas técnicas, no productos operativos aprobados para causas.
6. La documentación general del repositorio todavía debe actualizarse: puede describir pendientes ya implementados localmente.

## [L-SIGUIENTE]
Ejecutar la aceptación institucional en Windows 10 con Excel 2010 y Outlook clásico sobre copias, documentando resultados y cualquier diferencia. La excepción de Cumplimiento y la exportación desde bytes conservados ya están implementadas; no repetirlas salvo nueva evidencia.

## Recuperación del material local cuando vuelva a estar disponible
Esta secuencia es complementaria: no impide continuar cambios independientes y verificables mediante GitHub Actions.

## Secuencia mínima de continuación
1. Comprobar existencia y contenido actual del directorio anterior. No borrar ni sustituirlo.
2. Recuperar el resultado pendiente o ejecutar desde ese directorio:
   PYTHONPATH=src:../testdeps NURUS_REFERENCE_DIR=../upload python -m pytest -q
   Esta orden corresponde al entorno Linux de trabajo anterior, no a Windows.
3. Corregir solo fallos reproducibles. Verificar en particular importación de contactos existentes con alias, transacción y respaldo; contador con fechas límite y consulta parcial.
4. Compilar src y tests; comprobar dependencias/paquete. La instalación destinada al usuario sigue siendo Python 3.12 en un entorno virtual.
5. Comparar con el HEAD remoto vigente antes de publicar. Conservar modificaciones ajenas y archivos remotos no descargados, incluidas matrices binarias. Publicar un commit de código en la rama de integración, sin forzar ni modificar main.
6. Verificar el commit remoto y sus pruebas CI. Actualizar README, IMPLEMENTACION y PRUEBAS con resultados reales y límites.
7. Ejecutar aceptación en Windows con copias: conservación Excel; apertura de Word; cuenta y carpeta Outlook; CC; adjuntos; ausencia de envío; resultado incierto y duplicados; contador de solo lectura.
8. Solo tras resolver las pruebas y las matrices faltantes, evaluar la declaración de compatibilidad productiva.

Este documento es memoria de navegación. La fuente para editar sigue siendo el código recuperado y la fuente para acreditar pruebas sigue siendo su resultado.

