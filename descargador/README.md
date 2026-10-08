# Descargador SITFA · prototipo integral 2.7.0

**Entrega 2.7.0, 8 de octubre de 2026.** Incluye la lectura de bitácoras y la
reconstrucción visual del PDF en una misma extensión. El descargador acredita
sus capacidades ante CSMP y comprueba las de la extensión antes de operar.
Preserva pausa, apariencia, favoritos relativos, formato de Excel, calendarios
y ZIP verificable. [Guía de instalación, circuito completo y pruebas](../docs/INTEGRACION_COMPLETA_20261008.md).


**Antecedente 2.5.1:** cuenta también las consultas fallidas, evita anunciar copias cuando no existen y conserva fase, tipo y ubicación del error sin valores privados. La segunda lectura del listado admite el formato HTML aceptado por el descargador normal. [Incidencia y validación pendiente](docs/BITACORAS_CERO_20261008.md).

**8 de octubre de 2026 · lectura de bitácoras.** El botón **Leer bitácoras del lote** recorre el listado y sus páginas para Ambulatorio, FAE, Residencia y DCE. Usa los tribunales y pestañas seleccionados y copia la tabla de observaciones de cada ingreso identificado. Conserva copias HTML, control de lecturas y fallas; **Reintentar lecturas fallidas** vuelve a consultar el listado y reutiliza las copias verificadas. CSMP dev22 recibe el resultado y genera el Excel, o permite abrir después `bitacoras.json` desde Resultados. [Guía de uso y límites](../docs/BITACORAS_LOTES_DEV21.md). Las pruebas locales no equivalen a aceptación con datos reales de las cuatro modalidades. No escribe observaciones en RUS.

**Corrección de retorno a CSMP (6 de octubre).** Descarga conjunta CSMP reúne la principal completa y los informes por vencer del mes actual y siguiente, y entrega el XLSX a CSMP. Se retiró Carga/resoluciones firmadas de ese flujo porque impedía completar la entrega. La actividad queda explícitamente `NO_CONSULTADA`; no acredita ausencia de movimientos. Los flujos anteriores con esas tres fases validadas se pueden recuperar sin repetir descargas. La consulta independiente de Carga se conserva fuera del flujo conjunto, sin atribuirle validación en vivo.

También reconoce el encabezado real `FECHA VENCIMIENTO` del calendario de medidas y el filtro que RUS aplica mediante `cargarOpciones` en Cumplimiento (45 días/vencidas) e Informes. No ejecuta el JavaScript de la respuesta: comprueba el inicializador observado y su valor literal. El período inicial de los calendarios es el mes/año actuales si están disponibles; una elección manual posterior se conserva.

La versión 2.4.3 corrige el falso error de período en Descarga conjunta CSMP y calendarios: reconoce las fechas `dd-mm-aaaa` que entrega RUS. Conserva el rechazo de meses/años ajenos a la consulta. Si una fecha está vacía o es inválida, informa la fila; si pertenece a otro período, informa la fecha y el mes consultado.

La exportación de calendario distingue vencimiento de informe y egreso proyectado, conserva la columna de origen e identifica lotes incompletos. Una fecha proyectada no acredita egreso efectivo. CSV, ZIP, consolidación y comparación siguen en Resultados.

**5 de octubre de 2026.** Esta rama incorpora la descarga conjunta para CSMP: principal completa, informes por vencer del mes actual y siguiente, y carga de al menos 60 días en bloques de hasta 30. Incluye recuperación por consulta, procedencia, vínculos de ingreso y cobertura explícita. La actividad se presenta como parcial hasta confirmar el criterio temporal del sistema. Consulta la [guía de pruebas integrales](docs/GUIA_PRUEBAS_INTEGRALES.md) y el [estado de los 32 elementos](docs/ESTADO_PLAN_INTEGRAL_20261005.md).

**Rama experimental:** las mejoras funcionales y la integración con NuRus están
implementadas aquí. Consulta [GUIA_EXPERIMENTAL.md](GUIA_EXPERIMENTAL.md) para
comparar con `main`, usar consolidación, favoritos, historial, CSV/ICS/ZIP y
exportación para CSMP. La versión original se conserva en `main`; el texto de
funcionamiento básico siguiente sigue describiendo su flujo de descarga.

Herramienta local para Windows que utiliza **tu Chrome habitual** y descarga grupos de consultas de Seguimiento. La versión anterior queda conservada en su carpeta; para esta versión usa el **Iniciar.cmd de esta carpeta**.

## Actualizar desde la versión 2.0.1

Esta actualización añade los calendarios de medidas e informes por vencer, la búsqueda de litigantes y órdenes y la selección de varios tribunales mediante casillas. Conserva la conexión corregida con tu Chrome, las consultas de Seguimiento, las fechas, los nombres de archivos y las descargas secuenciales verificadas.

1. Cierra el descargador y la pestaña de conexión anterior.
2. Reemplaza los archivos de la versión anterior con los de esta versión, **en la misma carpeta cuya extensión cargaste en Chrome**. Si trabajas desde la carpeta entregada directamente, los archivos ya están actualizados.
3. Abre `chrome://extensions`, busca **SITFA · Descargador local** y pulsa **Recargar**. Su versión debe ser **2.2.0**. No necesitas volver a instalarla.
4. Abre **Iniciar.cmd**, pulsa el icono de la extensión desde la pestaña SITFA y conecta con el código nuevo. Luego pulsa **Cargar opciones**.

El código anterior vence al cerrar el descargador. Si tienes varias copias de la herramienta, usa el iniciador y la extensión de la misma carpeta actualizada.

## Preparar Chrome una vez

El botón **Abrir mi Chrome en SITFA** utiliza este acceso:

[Entrar a SITFA](https://familia.pjud.cl/SITFAWEB/jsp/Login/LoginB4.jsp)

La conexión se realiza mediante la extensión local **SITFA · Descargador local**, incluida en la carpeta `extension`. Las consultas se hacen dentro de tu sesión del navegador; el programa no lee ni copia cookies, contraseñas o perfiles.

Para instalarla personalmente:

1. Abre **Iniciar.cmd** y pulsa **Instalar extensión…**. Se abrirá la administración de extensiones de tu Chrome y se copiará la ruta de la carpeta correspondiente.
2. Activa **Modo de desarrollador** y pulsa **Cargar descomprimida**.
3. Selecciona la carpeta **extension** de esta versión, que contiene `manifest.json`.
4. En el menú de extensiones de Chrome, fija **SITFA · Descargador local** para que su icono quede visible.

No requiere administrador. La extensión solicita acceso únicamente a `familia.pjud.cl` y al enlace local `127.0.0.1`; no solicita permisos de cookies, contraseñas, historial o almacenamiento. Chrome describe el acceso al sitio como lectura y modificación de la página: se utiliza para rellenar los filtros de consulta y bloquear temporalmente la pantalla durante un lote. No hay funciones de edición de causas o medidas.

Se utiliza una extensión porque Chrome restringe el control automatizado de su perfil habitual mediante depuración remota. [Documentación de Chrome](https://developer.chrome.com/blog/remote-debugging-port).

## Conectar cada sesión de trabajo

1. Abre **Iniciar.cmd** y pulsa **Abrir mi Chrome en SITFA**. Inicia sesión personalmente si hace falta y entra a **Seguimiento**. Si usas varios perfiles de Chrome, utiliza el perfil donde está instalada la extensión y donde tienes acceso a SITFA.
2. Desde **esa pestaña de SITFA**, pulsa el icono de la extensión. Se abrirá su pestaña de conexión.
3. En el descargador pulsa **Copiar código**; pégalo en la pestaña de conexión y pulsa **Conectar**.
4. Vuelve al descargador, elige **Consulta** y pulsa **Cargar opciones**. La herramienta abre esa pantalla de consulta en SITFA y carga sus opciones. Para Seguimiento, elige **Seguimiento**.

Mantén abiertas la pestaña de SITFA y la pestaña de conexión. El código solo autoriza el enlace entre la extensión y el programa en este equipo; no contiene credenciales de SITFA y vence al cerrar o desconectar la herramienta. No se guarda en archivos ni en el almacenamiento de la extensión.

## Elegir un lote

Pulsa **Elegir tribunales…** y marca las casillas de los tribunales que quieres incluir; puedes marcar uno, varios o todos. **Marcar todos** y **Desmarcar todos** facilitan preparar la selección. Pulsa **Aplicar** para guardarla; **Cancelar** conserva la anterior. Si no marcas ninguno, la descarga queda deshabilitada. La primera carga marca todos los tribunales disponibles; las siguientes conservan los elegidos que siguen disponibles.

En Seguimiento, selecciona una modalidad o **Todas las modalidades**. Las modalidades son **Residencia, Ambulatorio, FAE y DCE**. Solo se incluyen tribunales y opciones que aparecen en tu formulario actual.

En **Registros**, elige **Espera**, **Cumplimiento**, **Espera y Cumplimiento** o **Informes**. El programa muestra cuántas consultas formarán el lote antes de comenzar.

| Lo que quieres descargar | Tribunal | Modalidad | Registros |
|---|---|---|---|
| Todo Espera y Cumplimiento de Tomé | Tomé | Todas las modalidades | Espera y Cumplimiento |
| Todo Residencia en Cumplimiento de todos los tribunales | Todos los tribunales disponibles | Residencia | Cumplimiento |
| Todo FAE en Espera de todos los tribunales | Todos los tribunales disponibles | FAE | Espera |
| Todo Espera y Cumplimiento del alcance de tu cuenta | Todos los tribunales disponibles | Todas las modalidades | Espera y Cumplimiento |

Para Cumplimiento puedes seleccionar **Todas las medidas**, **Por vencer** o **Vencidas**, según las opciones de SITFA. Cuando eliges Espera y Cumplimiento, este filtro solo se aplica a Cumplimiento. Para Informes, elige **Por vencer**, **Vencidos** o **Recibidos**. La herramienta lee las opciones que entrega cada pestaña; no confunde los plazos de informes con los de medidas.

Por defecto se quitan filtros previos de litigante, causa, centro, plazo o tiempo de espera, para consultar todos los registros del alcance seleccionado. Si necesitas mantenerlos, prepáralos en el formulario de Seguimiento antes de iniciar el lote y marca **Conservar los otros filtros preparados en SITFA**. Si un filtro dependiente no está disponible al cambiar tribunal o modalidad, se detiene la consulta.

Pulsa **Descargar el lote completo**. Las consultas y todas sus páginas se descargan en orden, una después de otra. **Cancelar** detiene el lote antes de pedir la página siguiente; la página en curso puede terminar primero. **Abrir carpeta** muestra los resultados disponibles.

## Pantallas añadidas

En **Consulta** puedes elegir:

- **Medidas por vencer (calendario):** el informe mensual de medidas; elige tribunales, mes y año.
- **Informes por vencer (calendario):** el informe mensual de informes; elige tribunales, mes y año.
- **Seguimiento de litigantes / órdenes:** elige tribunales y estado de órdenes según las opciones de SITFA. Descarga todas sus páginas y permite un rango de fechas.

Después de cambiar Consulta, pulsa **Cargar opciones**. Los controles de modalidad, Espera/Cumplimiento y tipo de informe continúan disponibles para Seguimiento. Los calendarios usan mes y año y entregan un XLS por tribunal. Se consulta separadamente cada tribunal marcado; la opción global de un calendario no se mezcla con tribunales individuales.

Para una persona o causa concreta en Litigantes, carga primero sus opciones, prepara RUT, causa o medida en SITFA y marca **Conservar los otros filtros preparados en SITFA**. Sin esa casilla se consulta el estado elegido sin filtros de persona, causa o medida. Los filtros se conservan durante las páginas y se intenta restaurar el formulario original al terminar; sus valores privados no se guardan en los resúmenes.

Por ejemplo, para descargar Espera de Tomé y Laja, pulsa Elegir tribunales, desmarca todos, marca esos dos, pulsa Aplicar y elige Espera. Los tribunales desmarcados quedan fuera del lote. Esa misma selección por casillas sirve para las tres pantallas añadidas.

## Fechas

Marca **Filtrar por fecha** y elige **Desde** y **Hasta** con los calendarios, o escribe ambas fechas en formato `DD/MM/AAAA`. El inicio debe ser anterior o igual al final. El rango se aplica a todas las consultas del lote mediante los campos nativos de Seguimiento o Litigantes. Los calendarios usan su mes y año.

Cuando la opción está desmarcada, se envía la búsqueda sin filtro de fecha. Tener fechas escritas en los campos de SITFA no basta: la casilla de consulta y su indicador deben activar el rango. La herramienta verifica esos valores antes de enviar la búsqueda.

## Carpetas y nombres

Cada lote crea una subcarpeta como **2026-09-30 15-40-12**. Si ya existe otra del mismo segundo, se añade precisión a la hora.

Ejemplos de Excel dentro de ella:

```
ESP Residencia Jdo Familia Tome.xls
CUMP Ambulatorio Jdo Familia Tome 1.xls
CUMP Ambulatorio Jdo Familia Tome 2.xls
ESP FAE Jdo Letras y Garantia Laja.xls
CUMP DCE Jdo Familia Tome.xls
Informes Jdo Familia Tome 1.xls
Informes Jdo Familia Tome 2.xls
Medidas por vencer Jdo Familia Tome.xls
Litigantes Activa Jdo Familia Tome 1.xls
Litigantes Activa Jdo Familia Tome 2.xls
```

Espera usa **ESP**, Cumplimiento usa **CUMP**, seguidos de modalidad y tribunal. Se normalizan las tildes y abreviaturas del tribunal. Una consulta de una página usa el nombre sin número; varias páginas añaden **1, 2, 3…**.

Informes usa **Informes + tribunal**. Si un lote incluye varias modalidades para el mismo tribunal, los números continúan entre ellas para evitar nombres repetidos. La descripción exacta de cada consulta queda en sus archivos de verificación. No se sobrescriben archivos existentes.

También se guardan **resumen.json** para el lote y **verificacion 001.json**, **002…** para sus consultas. Contienen alcance, fechas elegidas, cantidades, nombres de archivos y comprobaciones; no contienen filtros privados, RIT, nombres de personas ni autenticación. Los Excel conservan los datos originales que entrega SITFA.

- **VALIDADA:** se completaron y verificaron las páginas de la consulta o las consultas del lote.
- **SIN_RESULTADOS:** una consulta no tenía registros; el lote continúa con la siguiente.
- **INCOMPLETA:** el lote se canceló o se detuvo. Se conservan las páginas verificadas. Para completarlo, inicia un lote nuevo.

## Sesión y funcionamiento

La extensión hace las consultas de lectura y los GET de Excel desde tu Chrome, usando la sesión que administra el propio navegador. El programa recibe las respuestas en memoria por un enlace restringido a este equipo, compara los registros y guarda los Excel localmente. Los calendarios reconocen RIT y NOMBRE MENOR; Litigantes reconstruye el RIT desde Tipo Causa, Rol Causa y Era Causa. NOMBRE CENTRO no se confunde con la persona. Los archivos originales conservan todas sus columnas. No envía los datos a servicios externos de procesamiento ni guarda HTML, HAR o datos de autenticación.

La secuencia de cada página es buscar → descargar → comparar RIT y Nombre contra el listado → guardar → comprobar el archivo guardado → continuar. Un Excel anterior, duplicado, no coincidente, un cambio del total o una sesión vencida detienen el lote. No hay reintentos silenciosos.

SITFA reutiliza su Excel temporal. Mientras se descarga, evita hacer otras exportaciones con la misma cuenta, desde esta herramienta o desde otras pestañas. Al finalizar se retira el bloqueo local del formulario y se intenta restaurar los filtros iniciales.

Si no se logra restaurar todos los filtros, el descargador muestra un aviso.
Revisa el formulario de SITFA antes de volver a consultar. Los archivos ya
verificados se conservan y el aviso no cambia el resultado del lote.

**Desconectar** o cerrar la herramienta termina el enlace local y conserva tu Chrome abierto. La sesión de SITFA continúa bajo el control habitual del navegador.

## Requisitos y alcance

Windows, Chrome y Python 3.10 o posterior con Tkinter. El iniciador prepara un entorno privado en esta carpeta e instala las dependencias en su primera ejecución; esa instalación requiere Internet. En este equipo se verificó con Python 3.13 y Chrome 154.

Hasta 500 páginas por consulta y respuestas de hasta 80 MB. Se guardan Excel separados; no hay consolidación, reanudación automática ni integración con CSMP.

Se incorporan los calendarios mensuales de informes y medidas por vencer y Consulta Informe / litigantes, a partir del prototipo entregado y de los HAR. Cada pantalla tiene sus parámetros, acción y formato de Excel propios. Las descargas siguen siendo secuenciales y verificadas. Los calendarios semanal y diario y Egreso quedan fuera del alcance. No se añaden consolidación ni cambios en causas o medidas.

El detalle y las limitaciones de las comprobaciones están en **VERIFICACION.md**.

La revisión de código, las correcciones y las propuestas de mejoras están en
[INFORME_REVISION.md](INFORME_REVISION.md).

Las propuestas de funcionalidades, consolidación e integraciones para facilitar
el trabajo del usuario están en [MEJORAS_USUARIO.md](MEJORAS_USUARIO.md).
Las propuestas se evaluaron en esos documentos; su estado de implementación en
esta rama está en [GUIA_EXPERIMENTAL.md](GUIA_EXPERIMENTAL.md).

La [comparación con NuRus / CSMP Assistant](COMPARACION_NURUS.md) detalla cómo
entregar libros compatibles, aprovechar su análisis, correos y resoluciones,
y qué límites comprobar. Incluye una sonda offline con datos ficticios.

## PDF cuando no hay registros

Cada consulta sin resultados genera un PDF en la carpeta del lote, con el mismo nombre base de sus Excel: `ESP Residencia Jdo Familia Tome.pdf`, `CUMP FAE Jdo Letras y Garantia Laja.pdf`, `Informes Jdo Familia Tome.pdf`, etc. Las consultas de Informes que comparten un nombre se numeran también al guardar PDF; ningún archivo se sobrescribe.

La herramienta vuelve a consultar los mismos filtros mediante un formulario de SITFA en una vista temporal dentro de la pestaña original. Verifica que esa respuesta siga vacía y corresponda al tribunal, modalidad, estado o mes elegido antes de capturarla con Chrome. El PDF contiene únicamente la captura real, sin encabezados, fechas, leyendas ni márgenes añadidos por el descargador. El nombre y la verificación quedan en el archivo y en el manifiesto del lote. No fabrica una tabla ni guarda HTML, capturas PNG intermedias, contraseñas o cookies. La vista se retira al terminar y se restauran los filtros anteriores.

Chrome activa la pestaña SITFA brevemente para la captura. Mantén su ventana abierta; evita cambiar de pestaña o navegar durante ese momento. El permiso `activeTab` se concede al pulsar el icono de la extensión en SITFA: no se agregan permisos para capturar otros sitios. Si la consulta cambia y aparece un registro, se detiene el lote y no se genera un PDF vacío. Si falla la captura, el lote queda incompleto y conserva sus archivos anteriores.

Los PDF pueden contener lo visible en tu sesión de SITFA: se guardan solo en tu carpeta local. El repositorio contiene código, instrucciones y pruebas ficticias; excluye HAR, Excel, PDF y datos descargados.
