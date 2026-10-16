# Devolución recuperable del descargador — dev17

Esta actualización desarrolla D08/C01/C14. Conserva el flujo de principal completa, informes del mes actual y siguiente y carga de al menos 60 días. No modifica las reglas históricas ni conecta todavía el lector de bitácoras de RUS.

## Inicio

El CSMP instalado permite seleccionar un ejecutable del descargador. Si la configuración anterior apunta a `descargador.py`, solicita elegir `SITFA_Descargador.exe`: no utiliza el propio CSMP como intérprete de Python. La ejecución desde código fuente conserva la opción del script. Una ventana del descargador todavía abierta impide abrir otra desde esa misma instancia de CSMP.

Antes de lanzar el proceso se guarda una solicitud local con modo, carpeta de salida, fecha y archivo único de devolución. Si falla la escritura de la solicitud, no se abre el descargador. Si falla el inicio, se conserva `ERROR_INICIO`; cerrar sin resultado conserva `CERRADA_SIN_RESULTADO`.

## Recuperación

Resultados → **Recuperar descarga** permite elegir la solicitud conservada. No abre RUS ni repite consultas. Con varias solicitudes se elige explícitamente por fecha, modo, estado e identificador; no se toma automáticamente la última fila.

La incorporación comprueba el SHA-256, el modo solicitado y las columnas requeridas antes de sustituir el trabajo. Conserva una sesión del trabajo anterior. Un libro cambiado, incompleto o de otro modo deja la solicitud pendiente y mantiene el trabajo actual.

El análisis completado se archiva antes de marcar la solicitud `INCORPORADA` y antes de exportar con Excel. Si Excel falla, la recuperación utiliza esa sesión; no vuelve a analizar ni a descargar. El guardado habitual conserva también sus ediciones posteriores en el archivo de esta solicitud. Empezar otro trabajo no borra ese archivo.

Un corte entre guardar la sesión y guardar el estado se recupera comprobando la sesión archivada y su huella. La preferencia es el trabajo actual coincidente, la sesión de uso coincidente y luego el archivo propio de la solicitud. Nunca se recupera por RIT/nombre ni por posición de fila. Las rutas del archivo interno admiten rutas largas de Windows.

Las solicitudes antiguas creadas antes de dev17 no tienen este diario. Sus Excel terminados se pueden abrir por el flujo normal de Archivo/opciones; no se inventan solicitudes retroactivas.

## Comprobaciones

Las pruebas cubren selección del EXE en una instalación congelada, ejecución del script desde fuente, disco lleno antes del lanzamiento, inicio fallido, proceso duplicado, respuesta tardía después de reiniciar, modo/huella/columnas incorrectos, error de Excel, corte al guardar el recibo, archivo del trabajo anterior, ediciones posteriores y separación de solicitudes con el mismo RIT. La interfaz se comprueba a 1024×650 en una sola instancia real de CSMP.

El resultado final de regresión y el arranque del paquete Windows deben constar por commit antes de entregar dev17. Las consultas y el flujo reales con Chrome/RUS siguen pendientes.
