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

No habilitar el escritor real. Faltan: comprobación puntual autorizada del DOM (editor, casilla, vínculo, identidad y Estado), evidencia viva de relectura, exportación Flow y aprobación vigente, revisión humana exacta del lote, política de custodia/retención y ruta privada única, instalación Chrome y Windows, conciliación de Excel nativo/compatibilidad, piloto unitario autorizado y verificación posterior. No se fusiona a flow-python ni se publica release productiva. No existe acción de guardado remoto en esta versión.

## Vista previa desde Trabajo del Asistente (código fuente de esta rama)

En un paquete ficticio del Descargador importado al Asistente con copia vigente, selecciona 1–400 filas visibles de una misma vista y abre Trabajo > Más > Vista previa de lote RUS. Escoge origen y destino y un código de Estado explícito (para pruebas ficticias, no se conoce aún el catálogo vivo). Pulsa Preparar revisión, revisa todas las operaciones, marca la confirmación de previsualización y pulsa Confirmar vista previa. El lote se serializa y deserializa únicamente en memoria; no hay archivo de transferencia, importación al registrador ni permiso de guardado. Si cambia selección, texto, destino, estado o fuente, exige nueva revisión. No usa selecciones ocultas. La opción está probada desde código fuente en CI; no se ha validado un ejecutable instalado ni uso real. No introducir datos institucionales para probar este prototipo.
