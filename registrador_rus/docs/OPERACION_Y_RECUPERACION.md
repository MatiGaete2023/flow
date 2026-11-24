# Manual de la demostración aislada — 0.1.0

**Solo datos ficticios.** No importar expedientes ni usar el prototipo en RUS. No es instalador productivo. La autorización R1 habilita el desarrollo del nuevo módulo, no acredita por sí sola permiso institucional.

## Inicio y requisitos

Desde la raíz del repositorio en la rama feat/registrador-observaciones-rus-v3, ejecutar:

    python -m registrador_rus.app.main

Tkinter y SQLite de la biblioteca estándar; objetivo Windows con Python 3.12 o 3.13, probado solo en Linux 3.13.5 con Tk/Xvfb. No requiere admin ni instalaciones de paquetes adicionales para la UI si Python incluye Tkinter. Para las pruebas de esquema y Excel, jsonschema y openpyxl; para las pruebas del navegador, Playwright y Chromium únicamente en desarrollo. Si la política institucional impide Chrome MV3, detenerse y no intentar modificarla.

## Demostración

1. Elegir una fila. Revisar identidad ficticia, texto, Estado y destino. Administrativa = casilla desmarcada, Al Tribunal = marcada.
2. Preparar, autorizar la simulación individual mediante diálogo y simular intento. El selector de fallos afecta exclusivamente FakeRus en memoria. COMPROBADA en esta interfaz significa recibo ficticio.
3. Cupo vacío significa sin límite horario adicional local; se pueden elegir intervalo fijo o variable. El reloj es virtual: usar el botón de avance para esperar el cupo.
4. Si hay incertidumbre, seleccionar la operación y Recuperar por lectura. No repetir el intento. AMBIGUA exige revisión humana (la resolución manual aún no está implementada). NO_HALLADA no autoriza reenvío. Un bloqueo CRITICAL no se reanuda desde la UI.
5. El diario de esta interfaz es temporal: cerrar elimina la demostración y su FakeRus en memoria. No es la recuperación persistente de una sesión de producción; las pruebas del diario sí reabren el SQLite. No conservar datos judiciales allí.

## Chrome ficticio (opcional, instalación MV3 aún no validada)

Solo en un equipo de pruebas donde la política lo permita, cargar la carpeta registrador_rus/extension como extensión desempaquetada. Abrir su página de demostración desde el icono, copiar el origen de extensión a la app y abrir puente. Copiar puerto, sesión y token solo localmente; no guardarlos ni compartirlos. Elegir fila y Preparar en demo Chrome; pulsar Recibir una orden. Se muestra el formulario ficticio, con el botón de guardado inhabilitado. El ack no constituye confirmación humana ni comprobación RUS. No existen permisos de sitio institucional. Si el transporte pierde un ack, cerrar sesión de demostración y reiniciar; no se reenvía automáticamente.

## Verificación de desarrollo

    python -m unittest discover -s registrador_rus/tests -p "test_*.py" -v
    python registrador_rus/tests/run_dom_detection.py
    python -m registrador_rus.tests.run_preparation
    python -m registrador_rus.tests.run_browser_demo
    python -m registrador_rus.tests.run_batch_demo

La primera instrucción necesita pantalla Tk y dependencias de prueba; en Linux se usó xvfb-run. REGISTRADOR_CHROMIUM permite indicar un Chromium ya instalado. run_browser_demo usa adaptadores de transporte/crypto de prueba, no instala MV3. run_batch_demo solo mide controlador local y reloj virtual, no navegador ni RUS.

## Puerta de uso productivo: no cumplida

No habilitar el escritor real. Ya se observó el DOM de las cuatro vistas con el inspector pasivo, y se validaron revisión local, asignación de Estado y preparación en fixtures. Faltan: identidad efectiva y contexto en uso real, efectos de eventos del formulario, evidencia viva de relectura, transferencia operativa protegida desde Flow y autorización vigente, política institucional de custodia/retención y ruta privada única, instalación productiva Chrome/Windows, conciliación de Excel nativo/compatibilidad, piloto unitario autorizado y verificación posterior. No se fusiona a flow-python ni se publica release productiva. No existe acción de guardado remoto en esta versión.

## Vista previa desde Trabajo del Asistente (código fuente de esta rama)

En un paquete ficticio del Descargador importado al Asistente con copia vigente, selecciona 1–400 filas visibles de una misma vista y abre Trabajo > Más > Vista previa de lote RUS. Escoge origen y destino. El Estado se calcula automáticamente: Espera 4 Gestión Previa, Cumplimiento 6 Realizada, Informes por vencer/vencidos 9 Realizada. No se puede editar; su disponibilidad deberá comprobarse en la bitácora. Pulsa Preparar revisión, revisa todas las operaciones, marca la confirmación de previsualización y pulsa Confirmar vista previa. El lote se serializa y deserializa únicamente en memoria; no hay archivo de transferencia, importación al registrador ni permiso de guardado. Si cambia selección, texto, destino, estado o fuente, exige nueva revisión. No usa selecciones ocultas. La opción está probada desde código fuente en CI; no se ha validado un ejecutable instalado ni uso real. No introducir datos institucionales para probar este prototipo.

## Transferencia de un lote de demostración entre procesos

En el Asistente desde fuente, Trabajo > Más > Generar lote ficticio para registrador…; elige cantidad (1–400) y un nombre nuevo terminado en .flow-demo.json. No toma registros del trabajo. Alternativa desde la carpeta raíz: `python -m registrador_rus.app.demo_transfer lote.flow-demo.json --count 400`. Cierra el registrador anterior solo cuando hayas terminado su simulación y abre una sesión nueva con `python -m registrador_rus.app.main --demo-batch lote.flow-demo.json`. Revisa el conjunto, selecciona una fila, prepara y autoriza explícitamente antes de simular. La importación no crea autorizaciones ni intentos. No se admiten archivos de la vista previa de Work ni datos reales. El destino no se sobrescribe. Compatible solo con sistemas de archivos que soporten enlaces duros; CI Windows aprobada con datos ficticios; Excel nativo y equipo institucional pendientes.

## Inspección focalizada opcional — solo con autorización institucional

1. Carga descomprimida en Chrome la carpeta registrador_rus/inspector, como extensión distinta de la demo. Si la política no lo permite, detente; no cambies restricciones.
2. En RUS abre manualmente una bitácora desde Espera, sin editar ni registrar nada. Usa el inspector con la ventana/pestaña de bitácora activa. Si Chrome no permite acceder al icono en esa ventana, detente; no fuerces la ejecución.
3. Pulsa Inspeccionar formulario abierto. CANDIDATE significa coincidencia estructural candidata, no confirmación de operación. Si aparece BLOCKED u otro error, informa únicamente ese código.
4. Si devuelve atributos y catálogo, verifica personalmente que no haya RIT, RUT, nombres, identificadores de expediente/ingreso, textos judiciales ni otros datos reservados. Comparte exclusivamente nombres de atributos técnicos, maxlength, códigos/rótulos generales de Estado y los booleanos de asociación de etiqueta/botón. No compartas capturas del expediente, HTML, tokens ni datos de sesión. No pulsar Nueva obs ni Grabar.

El núcleo pasó 9 pruebas con DOM ficticio; la extensión nativa todavía no fue validada. Un solo formulario es el objetivo inicial, no una nueva exploración general. No es un registrador ni usa la extensión de demo como escritor.

### Si el inspector devuelve FORMULARIO_NO_UNICO

Actualizar el inspector a 0.1.1: descomprimir el ZIP nuevo en una carpeta separada, quitar solo la extensión del inspector anterior e instalar la carpeta inspector nueva, si la política permite. Repetir una sola inspección con la ventana de bitácora activa. Si BLOCKED incluye diagnostic, compartir el JSON de conteos únicamente; no incluye identidad ni contenido. Los conteos distinguen nombre, ID, caja, controles visibles, acción esperada, tabla, selector y marcos visibles (sin entrar en ellos). CANDIDATE: revisar manualmente antes de compartir únicamente atributos técnicos. No editar ni guardar.

### Inspector 0.1.2 — formulario dentro de un marco

Si el diagnóstico anterior devuelve todos los conteos de formulario en cero y visible_frames=1, instalar la versión 0.1.2 y repetir la inspección en la bitácora abierta. Sigue solo un marco visible por nivel si es accesible del mismo origen y no hay marcadores del formulario en el contenedor. No cambia permisos. frame_depth indica niveles recorridos; si MARCO_NO_ACCESIBLE, MARCOS_AMBIGUOS, MARCO_CARGANDO o PROFUNDIDAD_MARCOS_EXCEDIDA, no forzar acceso: informar solo resultado técnico. No guardar.

### Inspector 0.1.3 — diagnóstico simultáneo

Si se obtiene TABLA_NO_UNICA en el marco, instalar 0.1.3. El resultado BLOCKED puede incluir structure (conteos de tabla/contexto/controles) y control_evidence (atributos técnicos si los controles superan sus comprobaciones independientes). Se mantienen las condiciones de bloqueo. Antes de compartir, revisar que solo contenga datos técnicos generales y no valores judiciales. No editar ni guardar; no repetir exploraciones de otras vistas.

### Inspector 0.2.0 — levantamiento integral

Sustituye el inspector previo por 0.2.0. Con la bitácora de Espera ya abierta y solo si está autorizado, pulsa Inspeccionar estructura completa. El nuevo JSON IntegralEvidenceV1 no se detiene ante una tabla o firma divergente; contiene evidencia independiente agrupada por documento y ruta ordinal de marcos. DIAGNOSTIC no significa aptitud para escribir. Revisa los nombres técnicos y opciones generales; omite cualquier información reservada inesperada antes de compartir. Si truncated=true, informa ese indicador. owner=-1 indica ausencia de propietario de formulario encontrado; headers.order usa posiciones 0–8 de los encabezados de bitacora_html.py, -1 desconocido. hidden_prefix_counts sigue el orden HOBS_Centro_, HOBS_Tribunal_, HCOD_Estado_, HFUN_Tribunal_, HFLG_Envio_. No editar ni guardar, no copiar HTML ni tráfico. No se requieren las otras tres vistas para esta etapa.

### Inspector 0.2.1 — bitácora como ventana emergente

Sustituir solo el inspector por 0.2.1. Abrir UNA bitácora desde Espera y regresar al listado sin cerrarla. Abrir icono y pulsar Localizar e inspeccionar bitácora abierta. Chrome puede pedir permiso opcional para familia.pjud.cl (origen completo); aceptar solo si está institucionalmente autorizado. El inspector localiza una candidata por ruta de apertura documentada o ventana popup en destino POST; no inspecciona otras rutas ni abre ventanas. Si hay varias candidatas, cerrar las demás manualmente. Si el diálogo de permiso cierra el popup de extensión, volver a abrirlo. Se intenta retirar permiso al finalizar; comprobar aviso. Si acceso sigue activo, retirarlo desde configuración de la extensión. Si política impide permiso, detenerse. Compartir únicamente IntegralEvidenceV1 revisado o código de bloqueo; nunca guardar una observación para probar.

### Resultado de la inspección 0.2.1

La captura de la ventana emergente real ya identificó los campos de Espera. No repetir inspector para estos atributos. Los estados observados en Espera son 3 Ingreso Efectivo y 4 Gestión Previa; la regla posterior del operador impone 4 automáticamente. Para las otras tres vistas, ver decisión obligatoria al final. La acción irAccion no es única por nombre, distinguir el submit visible Nueva obs del oculto de edición. Preparación/escritura real continúa bloqueada; ver EVIDENCIA_ESTRUCTURAL.md.

## Decisión obligatoria del operador — 24/11/2026, 15:16 Chile

El operador dispone que siempre se seleccione Gestión Previa en Espera y Realizado en todas las demás vistas. La denominación exacta observada en RUS es Realizada. Asignación fija, automática y no editable por operación: ESPERA -> código "4" (Gestión Previa); CUMPLIMIENTO -> "6" (Realizada); INFORMES_POR_VENCER -> "9" (Realizada); INFORMES_VENCIDOS -> "9" (Realizada). Esta decisión sustituye las instrucciones anteriores de elegir Estado explícitamente o no asignar un valor por defecto. Los demás estados observados son evidencia del catálogo, no opciones admitidas para registrar con esta herramienta. Mostrar Estado calculado en previsualización y aprobación; comprobar código y rótulo disponibles y únicos en el selector de la bitácora antes de cualquier preparación, bloquear ante discrepancia, jamás elegir una alternativa. No confundir con filtros de consulta ni con destino: chkBitacora desmarcada CC0 / marcada CC1 sigue independiente. La autorización humana de cada registro permanece obligatoria. Regla implementada en ce514a9 y validada por CI; diálogo y detector actualizados, contratos de demo separados. La asignación automática no habilita escritura.

## Verificación integrada exclusivamente ficticia

Desde raíz, con PYTHONPATH=src (en Windows PowerShell: $env:PYTHONPATH='src'), Playwright y Chrome/Chromium de desarrollo disponibles: python registrador_rus/tests/run_observed_journal.py. El ensayo reconstruye dos documentos about:blank, prepara y revalida la sesión, usa diario SQLite temporal y parser existente sobre HTML inventado; no pulsa Nueva obs ni Grabar. No requiere abrir RUS. La CI usa Chrome preinstalado; no instalar herramientas de desarrollo en el equipo institucional para esta prueba. Ninguna transición o recibo de simulación equivale a registro real.

## Transferencia cifrada de demostración (Windows, solo datos inventados)

Desde la raíz del repositorio con Python 3.12 y PyWin32 de las dependencias Windows de Flow, no requiere administrador si ya está instalado:

    python -m registrador_rus.app.protected_demo_transfer lote.flow-demo.dpapi --count 400
    python -m registrador_rus.app.main --protected-demo-batch lote.flow-demo.dpapi

No introducir paquetes reales. Cifra el archivo con DPAPI de usuario de Windows; no hay opción de máquina ni alternativa en claro. No sobrescribe; descifra y comprueba el lote ficticio exacto antes de abrir la UI. La demostración exige autorización individual simulada igual que antes. No copiar a nube, no trasladar a otra cuenta/equipo; DPAPI normalmente depende de credenciales y equipo, con excepciones de dominio. El archivo cifrado no es firma de aprobación y no cifra el diario temporal. El usuario puede borrar el archivo ficticio cuando termina; no hay purga automática ni política de datos judiciales aprobada. En Linux o si falta PyWin32, bloquea. CI nativa pendiente al añadir esta sección.

## Diario protegido de simulación (Windows)

API de desarrollo: SimulationJournal(ruta, protected_demo=True), ruta terminada en .protected.sqlite, nuevo diario o perfil protegido ya existente. Requiere PyWin32; no fallback a texto claro, no migración automática. Solo admite la familia exacta demo_operations, no datos judiciales. Se protegen antes de SQLite texto aprobado/identidad en payload, evidencia previa y actor; se usan huellas con clave para comparar identidad, intención y entrada. SQLite/WAL/SHM conservan metadatos técnicos visibles. Cifrado por campos, no cifrado integral del archivo. No se cambia la reserva persistente WAL/FULL ni recuperación sin reenvío. Ejecutar las pruebas con python -m unittest discover -s registrador_rus/tests -p "test_protected_journal.py" -v en Windows de desarrollo. No es un diario operativo ni acredita ACL o custodia institucional. No se ha acordado retención ni purga del diario real.
