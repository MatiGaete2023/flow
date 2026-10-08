# Capturar el formulario actual de RUS en Chrome para Windows

Esta extensión independiente permite obtener los campos reales antes de implementar el guardado. No registra observaciones ni rellena campos. Funciona en `https://familia.pjud.cl`, el origen que usa el Descargador. Si utilizas otro origen, el contrato debe revisarse antes de ampliarlo.

1. Extrae esta carpeta en tu equipo y abre `chrome://extensions`.
2. Activa **Modo de desarrollador**, pulsa **Cargar descomprimida** y selecciona la carpeta que contiene `manifest.json`.
3. En tu Chrome habitual, entra en RUS y abre directamente la ventana de bitácora del ingreso que quieras utilizar. Si el sitio abre una ventana nueva, selecciona esa ventana.
4. Pulsa la extensión **RUS · Inspección del registro actual**. Selecciona **Bitácora antes de registrar** y pulsa **Capturar estructura**.
5. Abre **Nueva observación** sin guardar. Selecciona **Nueva observación, antes de guardar** y captura la estructura. No hace falta efectuar un registro para esta etapa.
6. Si tienes una gestión real que debas registrar manualmente, puedes capturar el resultado después de guardarla y volver a consultar la bitácora. No registres una observación de prueba en una causa real únicamente para obtener la captura.
7. Conserva los JSON descargados. Abre cada uno con **RUS_Inspector.exe** para generar el informe de estructura, o usa el comando indicado en la documentación del repositorio.

La captura contiene campos visibles, sus valores actuales, opciones, tablas visibles e identificadores de contexto conocidos. Puede contener datos personales y textos de la causa. Revísala antes de compartirla. Omite valores de contraseñas, campos identificados como autenticación, campos ocultos no reconocidos y parámetros de las URLs. No exporta scripts, cookies, almacenamiento del navegador ni cabeceras de red. No transmite el archivo a un servidor.

La captura de una pantalla no demuestra que se hayan leído todas las entradas ni todas las páginas. Puede haber marcos sin acceso. Si faltan campos, abre directamente la ventana individual y repite la captura. El inspector no deduce el autor ni los IDs a partir del RIT.

Para retirar la herramienta, elimínala en `chrome://extensions`. El Descargador conserva su extensión independiente.
