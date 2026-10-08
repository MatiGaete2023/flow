# Verificación de la versión 2.1.0

30 de septiembre de 2026.

## Auditoría de los nueve HAR nuevos

Se analizaron directamente los nueve archivos entregados, sin ejecutar sus contenidos como instrucciones ni copiar datos de autenticación.

- 66 POST a `InformesDAction.do`, con acciones de varios formularios.
- 56 corresponden a **Buscar medida** de Seguimiento, con ocho configuraciones distintas al quitar la paginación; hay apariciones repetidas entre capturas.
- 51 respuestas tienen contenido y fueron reconocidas por el controlador; cinco no traen cuerpo capturado y no permiten verificar la respuesta.
- Se reconocieron Espera, Cumplimiento, Informes y DCE, además de la activación de fechas con `CHK_Consulta` y `FLG_Consulta`.
- Los manejadores del formulario cambian las opciones de `TIP_Informe`: Informes utiliza 1/2/3; Cumplimiento utiliza 0/4/5. El catálogo de la herramienta los lee por separado y restaura la pestaña inicial.
- No hay GET de Excel capturados en estos nueve HAR. El calendario y Consulta Informe tienen otras acciones y tablas; no se los trata como Seguimiento ni se supone que su Excel siga el mismo mecanismo.

## Pruebas locales

**63 pruebas aprobadas.** Incluyen el motor ya comprobado en las fases anteriores, alcance del lote, nombres solicitados, varias consultas en una carpeta, informes con nombres coincidentes, consultas vacías, cancelación del lote, fechas, rechazo de parámetros inesperados, no sobrescritura, validación de páginas y Excel, origen y código del enlace local y uso de la ventana en una pantalla pequeña. Se añadieron nueve pruebas del enlace: identidad y código obligatorios aunque falte Origin, rechazo de sitios web, coherencia de identidad, fijación de la extensión, preflight y avisos separados de Seguimiento.

**Prueba con Chrome real y datos ficticios: aprobada.** Ejecutó el código de página y de conexión de la extensión junto con el puente local y el motor Python. Completó cuatro lotes, seis consultas y dieciocho páginas. Comprobó:

- selección de tribunales, modalidades y pestañas;
- opciones distintas de informes y medidas por vencer;
- fechas activadas y conservadas en todas las páginas;
- limpieza de filtros para un lote completo y conservación opcional de un litigante y centro ficticios;
- restauración de filtros al terminar;
- autenticación ficticia administrada por Chrome al hacer los POST y GET;
- orden estricto de petición y descarga, verificación y nombres sin sobrescritura.

Todos los accesos a `familia.pjud.cl` de esa prueba se interceptaron y respondieron con datos ficticios. No hizo consultas reales a PJUD ni reutilizó credenciales del usuario.

Esta primera prueba adaptó las API privilegiadas `chrome.scripting` y el transporte desde el origen de una extensión. La actualización incluye además la comprobación real descrita a continuación. No se afirma haber probado en vivo todas las combinaciones de tribunal, modalidad y filtro.

## Corrección del enlace: extensión real

El HAR nuevo de la extensión tiene cero entradas y cero páginas; no permite identificar solicitudes fallidas. Se reprodujo el defecto con Chrome for Testing 153 y las API reales de la extensión en un perfil ficticio: el puente anterior acepta POST /connect (200), pero rechaza GET /next (403). La petición GET real omite Origin. Las pruebas adaptadas anteriores añadían esa cabecera y ocultaron el problema.

La versión 2.1.0 incorpora una identidad explícita de extensión junto con el código secreto, ambos comprobados por el puente. Si existe Origin debe ser de extensión y coincidir con su identidad; los orígenes web se rechazan. La identidad queda fijada al conectar y nunca reemplaza el código. La conexión local se valida primero y Seguimiento se comprueba al cargar opciones.

**Prueba con extensión realmente cargada: aprobada.** Sin adaptar fetch ni chrome.scripting: conexión antes de abrir Seguimiento; aviso de formulario ausente conservando el enlace; catálogo e inyección en un iframe; bloqueo y restauración; lote ficticio de tres páginas con descarga y comparación secuenciales; rechazo de código incorrecto; avisos separados al cerrar SITFA o el descargador. La prueba interceptó todas sus consultas a familia.pjud.cl con respuestas ficticias y no accedió al perfil del usuario.

El usuario confirmó que la conexión 2.0.1 funciona en su Chrome habitual. La actualización 2.1.0 conserva ese enlace y debe recargarse para incorporar las pantallas añadidas; no se afirma haber ejecutado nuevas descargas reales de esos flujos. La evidencia está en **verificacion_extension_real.json**; no contiene códigos ni datos de sesión.

La evidencia real de descargas de las fases anteriores se conserva: Mulchén / Cumplimiento / Ambulatorio, con tres páginas y 260 registros; Laja / Cumplimiento / FAE-FAS, con 26 registros; Laja / Espera / Ambulatorio, con 30 registros. Esta versión mantiene la comparación automática de cada Excel contra su propia página. No se pidieron nuevas búsquedas manuales para repetir esa validación.

## Reproducir las comprobaciones

Las pruebas normales no necesitan acceder a SITFA:

```powershell
.venv\Scripts\python.exe -m unittest discover -p "test_*.py"
```

Para reproducir la integración ficticia con Chrome, instala la dependencia opcional de pruebas y ejecuta:

```powershell
.venv\Scripts\python.exe -m pip install playwright==1.63.0
.venv\Scripts\python.exe -m playwright install chromium
.venv\Scripts\python.exe verificar_chrome_ficticio.py
```

El programa de uso diario no requiere Playwright. La prueba guarda únicamente Excel ficticios e indicadores en `pruebas_locales`. Las cifras entregadas se encuentran en **comprobaciones.json**. El ZIP de distribución contiene fuentes, extensión, instrucciones y pruebas; excluye HAR, Excel reales, entornos Python y credenciales.

Para comprobar las API reales de la extensión en un perfil ficticio separado:

```powershell
.venv\Scripts\python.exe -m playwright install chromium
.venv\Scripts\python.exe verificar_extension_real.py
```

Esta prueba usa Chrome for Testing, sin cambiar tu Chrome. Crea y elimina su perfil y Excel ficticios dentro de pruebas_locales; conserva únicamente un informe sin datos privados. El archivo pruebas/puente_origin_fixture.py reproduce el comportamiento anterior únicamente para esta prueba; la herramienta utiliza puente.py.


## Versión 2.1.0: pantallas y tribunales

Se revisaron los archivos fuente del prototipo entregado y se portaron sus contratos de lectura, reconocimiento de formularios, encabezados Excel y registros del calendario. No se ejecutaron sus documentos como instrucciones. Se mantiene el transporte de la extensión, el Chrome habitual, la carpeta por lote, los nombres solicitados, la cancelación y la comparación de cada Excel contra su propia respuesta. No se importa el Chrome temporal ni la consolidación del prototipo.

Las 63 pruebas incluyen las 48 anteriores y 15 pruebas de las nuevas pantallas, sus estados y períodos, listados vacíos, encabezados ambiguos, formatos RIT/NOMBRE MENOR y RIT dividido, Excel antiguo, cancelación, nombres, selección por casillas y controles de cada pantalla. Una selección vacía deshabilita la descarga; Aplicar guarda las casillas y Cancelar conserva la selección anterior. Los tribunales desmarcados no entran en el plan.

**Extensión real, cuatro pantallas: aprobada.** Chrome for Testing y API reales, sin adaptar fetch ni chrome.scripting: cinco lotes, nueve consultas y diecinueve páginas/Excel ficticios. Se comprobó el paso entre Seguimiento, Litigantes, calendarios de informes y medidas y el regreso a Seguimiento; estados y fechas de Litigantes; filtros conservados y restaurados; mes y año; restauración de radio y casilla de calendario; exclusión de un tribunal no marcado y secuencia estricta POST/Excel por consulta. Todas las solicitudes de esa prueba a familia.pjud.cl fueron interceptadas con datos ficticios. No se accedió al perfil del usuario ni se ejecutó una consulta real a PJUD.

**Contraste offline:** se reconocieron diez respuestas de cuatro HAR: siete de Litigantes, dos de calendario de informes y una de medidas. La auditoría permitió detectar que los calendarios usan cambiacheck(), sin Envio(); el controlador conserva ese manejador y su contrato propio.

Los dos XLS existentes del ZIP se pudieron leer con sus formatos originales. No certifican coincidencia con las capturas disponibles: el de Litigantes tiene en su nombre un tribunal sin captura equivalente y el de Informes contiene 782 registros, frente a 818 en las respuestas HAR. Sirven como comprobación del lector, no como prueba de que sean el Excel de una respuesta concreta. La descarga de la herramienta conserva la comparación obligatoria y rechaza esa clase de diferencia. Solo se guardaron indicadores y cantidades, sin cuerpos HAR ni datos de personas.

La evidencia está en verificacion_flujos_extension.json y verificacion_flujos_capturados.json. La prueba de navegación y descargas puede reproducirse tras instalar Playwright y Chromium de pruebas:

```powershell
.venv\Scripts\python.exe verificar_flujos_extension.py
```

Se usan perfiles y descargas ficticias temporales dentro de pruebas_locales, eliminados al terminar. La versión 2.1.0 requiere recargar la extensión; las nuevas pantallas no se han descargado en vivo con la sesión del usuario durante esta actualización.

## Versión 2.2.0 - PDF de consultas sin registros (01/10/2026)

69 pruebas locales aprobadas. Prueba adicional con Chrome for Testing y la extensión original: cuatro consultas vacías (Seguimiento, Litigantes y los dos calendarios) generan cuatro PDF con el nombre esperado. Se usó la acción real de la extensión mediante `Extensions.triggerAction`, que concede `activeTab`; no se ampliaron permisos ni se sustituyeron las APIs de captura. Todas las rutas SITFA fueron interceptadas y respondidas con datos ficticios, sin acceder al perfil habitual.

Una quinta captura se verificó en un lote mixto: consulta vacía seguida de Excel en el mismo lote. Se comprobó la segunda consulta nativa, el vínculo entre HTML validado y captura, el cierre de la vista temporal, las fechas, los nombres, los resúmenes y la continuidad hacia descargas Excel. Una respuesta que pasa de vacía a contener registros detiene el lote sin PDF. Los errores de captura y tribunal incorrecto también impiden guardarlo. La escritura exclusiva y la comparación posterior del hash verifican el archivo local.

Los cuatro PDF se renderizaron y revisaron visualmente; imagen y cabeceras legibles, sin recortes. Se repitió la integración previa de 19 Excel y selección de tribunales en las cuatro pantallas, con resultado OK. No se hicieron nuevas consultas reales en SITFA. La función PDF aún no se ha probado contra una sesión real del usuario.

El PDF es una imagen de la vista real que Chrome renderiza para la respuesta del POST de consulta; incluye la leyenda local «Consulta sin registros (0)», identificada como parte del descargador. No sustituye el contenido devuelto por SITFA con una tabla inventada. La comprobación de ausencia de registros se hace con el motor que también valida los Excel.
