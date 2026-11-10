# Mejoras de diseño y experiencia — propuestas, 9 de noviembre de 2026

**Estado (10 de noviembre de 2026): las 32 propuestas están aplicadas en la rama `Rediseno`** (commit `52391c8` y
siguientes), con pruebas de cada una. Ninguna cambia lo que las herramientas hacen (consultas, reglas, archivos que
generan); solo cómo se ve y cuántos pasos pide. Al final, «Cómo quedó» resume cada una. Base: lectura del código de las ventanas y las
capturas tomadas en Windows (`docs/manuales/capturas_windows/`). No se probó con usuarios ni con SITFA real.

Esfuerzo: **S** = horas · **M** = 1–2 días · **L** = varios días.

## Lo primero (más beneficio por menos trabajo)

| # | Herramienta | Propuesta | Esfuerzo |
| --- | --- | --- | --- |
| 1 | Todas | Quitar el borde punteado de las fichas marcadas | S |
| 2 | Todas | Avisos con cierre «×» y tres tonos (listo, aviso, error) | S |
| 3 | Todas | «Manual» y «Video» en el menú «Más» | S |
| 4 | SITFA Lite | Pantalla de conexión con pasos 1-2-3 e «Instalar extensión» a la vista | M |
| 5 | SITFA Lite | Recordar la última selección de cada consulta | S |
| 6 | Meta 4 | Fecha de corte con calendario y formato DD/MM/AAAA | S |
| 7 | Anonimizador | Quitar una sola entrada y ver las opciones activas en el pie | S |
| 8 | Descargador | Una sola acción principal al pie; el resto a «Más» | L |

## Para todas (kit común)

1. **Fichas sin borde punteado.** En Windows cada ficha marcada muestra un borde punteado de foco (se ve en todas las
   capturas del Lite). Ensucia la fila y parece un error de dibujo. Se quita en el estilo `Chip.Toolbutton`.
2. **Avisos que se pueden cerrar y distinguen el tono.** Hoy el aviso queda hasta que otra acción lo reemplaza, y solo
   hay dos colores. El Anonimizador decide si es error mirando si el texto empieza con un número. Propuesta: «×» para
   cerrar y tres tonos explícitos: listo (verde), aviso (ámbar), error (rojo).
3. **Manual y video desde el programa.** Los PDF y videos ya van en el instalador (Lite) pero solo se abren desde el
   menú Inicio. Dos líneas en «Más»: «Manual de uso» y «Ver video».
4. **Teclado.** Enter = acción principal del pie; Esc = Detener; F1 = manual. Hoy todo es con mouse.
5. **«Marcar todas» descubrible.** Un clic en el nombre de la fila marca o desmarca todas las fichas, pero nada lo
   indica. Agregar una ficha «Todos» al inicio de las filas largas (Tribunales, Mes).
6. **Menú «Más» bloqueado a la vista.** Durante un proceso, las acciones del menú que no aplican hoy se ignoran sin
   decir nada. Mejor mostrarlas deshabilitadas.
7. **Mismo formato de fecha.** El Lite y el Descargador usan DD/MM/AAAA; Meta 4 pide AAAA-MM-DD escrito a mano.
8. **Avance en el título de la ventana.** «3 de 8 · SITFA Lite» se ve en la barra de tareas sin volver a la ventana.

## SITFA Lite

9. **Conexión guiada.** La primera pantalla muestra «Abrir SITFA», un código y «Copiar», sin decir qué hacer con
   ellos. Propuesta: tres pasos numerados (1 Abrir SITFA · 2 Copiar código · 3 Pegarlo en el ícono de la extensión),
   cada uno con su visto al cumplirse, y el botón «Instalar extensión» visible mientras no haya conexión (hoy está en
   «Más»).
10. **Recordar la selección.** Solo se guardan carpeta, pausa y la opción de PDF. Guardar tribunales, pestañas y
    modalidad por consulta evita volver a marcarlos cada día.
11. **Fechas que avisan en el lugar.** Un formato inválido o «Desde» posterior a «Hasta» hoy aparece como texto en el
    pie. Marcar el campo en rojo con el motivo al lado.
12. **Carpeta de descarga visible.** Solo se ve y cambia en «Más». Mostrarla abreviada en el pie, con clic para cambiarla.
13. **Resultados más directos.** El doble clic abre el archivo, pero no se indica. Agregar la pista «doble clic para
    abrir» y, si hubo consultas detenidas o con error, el botón «Reintentar las que faltan».
14. **Detener no es un error.** Al detener, el aviso «Lote cancelado…» sale en rojo. Debe ser tono neutro.

## Descargador SITFA

15. **Un solo estilo de filtros.** Seguimiento mezcla botones «Elegir tribunales…» (abren diálogo), listas
    desplegables, casilla «Filtrar por fecha» y fichas. Llevarlo al patrón del Lite: fichas en la misma pantalla y
    «Período: Sin filtro / Por fecha».
16. **Una acción principal.** Al pie hay tres filas de botones (crear paquete, XLSX, reutilizar bitácoras, calendario,
    historia, cancelar, pausar, abrir carpeta, copiar ruta, resultados). Propuesta: pie con «Consultar y crear
    paquete» y «Detener»; las opciones del paquete como fichas en una fila «Incluir»; lo demás en «Más».
17. **Retomar sin buscar.** Si al abrir hay un paquete incompleto, ofrecer «Retomar el paquete de ayer (faltan 3
    consultas)» en un aviso con botón, en vez de dejarlo solo en el menú.
18. **Misma conexión guiada que el Lite** (punto 9).

## Meta 4

19. **Fecha de corte con calendario** (el mismo del Lite) y validación al escribir; hoy el error aparece recién al
    pulsar «Generar».
20. **«Usar el último lote».** Proponer la última carpeta de descargas usada, en vez de navegar cada vez.
21. **Avisos accionables.** Cada aviso con «Abrir» hacia el archivo que lo explica (por ejemplo ERRORES_Y_ADVERTENCIAS).
22. **Estado vacío.** Sin lotes, la lista es un recuadro en blanco. Mostrar «Agrega el lote que dejó el Descargador»
    con el botón dentro.

## Anonimizador

23. **Quitar una entrada.** Hoy solo existe «Quitar todo».
24. **Saber cuál es cuál.** La lista dice «Entrada 1 · .xlsx» para no mostrar nombres. Mostrar el nombre al pasar el
    mouse (no queda en capturas ni registros).
25. **Opciones activas a la vista.** Semilla, desplazamiento de fechas y nacimiento se eligen en «Más» y luego no se
    ven. Resumirlas en el pie: «fechas +7 días · semilla definida».
26. **Modo explicado.** Una línea bajo «Estricto / Conservador» con lo que hace el modo marcado (hoy solo en «Acerca de»).

## Asistente CSMP

27. **Pasos con contador.** Trabajo → Correos → Resoluciones → Resultados ya es el flujo; numerarlos y mostrar cuántos
    quedan en cada uno («Correos · 12 por preparar»).
28. **Barra superior más corta.** «Archivo / opciones», «Importar paquete», «Último», «Actualizar Excel» y los filtros
    comparten una línea. Dejar «Importar paquete» como acción principal y mover el resto a un menú.
29. **Estado vacío.** Sin paquete, Trabajo muestra una tabla vacía. Poner al centro «Importa el paquete del Descargador».
30. **Un solo nombre.** La ventana dice «CSMP Assistant»; manuales e instalador dicen «Asistente CSMP».

## Manuales y videos

31. **Chrome capturado en Windows.** Las pantallas del programa ya son de Windows; las de Chrome siguen siendo de un
    Chromium de prueba en Linux.
32. **Video corto por tarea** (menos de 60 s): «Descargar Seguimiento de un tribunal», «Retomar un paquete», en vez de
    un solo video largo.

## Qué no propongo

- Tema oscuro, rediseño de íconos o animaciones: costo alto, beneficio bajo para el uso diario.
- Cambios en reglas, textos tradicionales o formatos de salida: fuera de alcance por regla del proyecto.

## Cómo quedó (10 de noviembre de 2026)

| # | Aplicado | Dónde |
| --- | --- | --- |
| 1 | Fichas y lista lateral sin relieve ni anillo de foco | `ui/flowkit.py` (`_flat_toolbutton`) |
| 2 | Aviso con «×» y tonos info, listo, aviso y error | `Banner.show(kind=…)` |
| 3 | Manual y video en «Más»; los instaladores los copian a `manuales/` | `Window.add_docs`, workflows y `CSMP.iss` |
| 4 | Enter, Esc y F1 (en el Asistente, F1 y la paleta Ctrl+K) | `Window`, Descargador, `app_base.py` |
| 5 | Ficha «Todos» en tribunales, estado y mes | `Chips(all_label=…)` |
| 6 | Acciones del menú deshabilitadas a la vista mientras trabaja | `TopBar.add(busy_lock=True)` |
| 7 | DD/MM/AAAA en Lite, Descargador y Meta 4 (Meta 4 acepta también AAAA-MM-DD) | `parse_date`, `DateField` |
| 8 | «3 de 8 · SITFA Lite» en la barra de tareas (Lite, Descargador, Anonimizador) | `Window.set_progress` |
| 9, 18 | Conexión en tres pasos con ✓ e «Instalar extensión…» visible | Lite y Descargador |
| 10 | Selección recordada por consulta, solo con opciones que SITFA siga ofreciendo | Lite (`selecciones`) |
| 11, 19 | Fecha mal escrita, invertida o futura marcada en rojo en el lugar | Lite, Descargador, Meta 4 |
| 12 | Carpeta de descarga en el pie, con clic para cambiarla | Lite y Descargador |
| 13 | Pista «doble clic para abrir» y «Reintentar las que faltan (n)» | Lite |
| 14 | Detener se informa en tono neutro | Lite |
| 15 | Filtros del Descargador como fichas en la misma pantalla, con «Período» | Descargador |
| 16 | Una acción principal (crear paquete o descargar); Pausar y Detener solo mientras trabaja; el resto en «Más» | Descargador |
| 17 | Aviso «Retomar» al abrir si un paquete reciente quedó EN_CURSO o INCOMPLETO | `flujo_paquete.pendientes` |
| 20 | «Usar el último lote» | Meta 4 (`preferencias.json` local) |
| 21 | Doble clic en un aviso abre Registros.xlsx (hoja ERRORES_Y_ADVERTENCIAS) | Meta 4 |
| 22 | Estado vacío con el botón dentro | Meta 4 |
| 23 | «Quitar entrada elegida» y tecla Supr | Anonimizador |
| 24 | Nombre de la entrada solo al pasar el mouse | Anonimizador (`Tooltip`) |
| 25 | Opciones activas en el pie | Anonimizador |
| 26 | Línea que explica el modo elegido | Anonimizador |
| 27 | «1 Trabajo · 4 avisos», «2 Correos · 3»… en la barra lateral | Asistente (`sidebar_counts`) |
| 28 | Barra superior: modo, «Importar paquete» y «Más ▾» | Asistente |
| 29 | Estado vacío en Trabajo con «Importar paquete» | Asistente |
| 30 | «Asistente CSMP» en la ventana y la barra lateral | Asistente |
| 31 | Capturas de Chrome tomadas en Windows | `tools/manuales/capturas_chrome_win.py` |
| 32 | Videos cortos: «Descargar Seguimiento de un tribunal» (Lite) y «Retomar un paquete» (Descargador) | `narrado.py`, `videos.py` |

Límites conocidos:

- La barra lateral del Asistente cuenta avisos, correos, resoluciones y productos por revisar con los mismos números de
  la barra «Siguiente paso»; no distingue «por preparar» de «preparados».
- El Descargador conserva sin mostrar los controles anteriores (botones y listas) porque el resto del código y sus
  pruebas leen de ellos el estado; las fichas los mantienen sincronizados.
- Las capturas del rompecabezas y del alfiler de Chrome dependen de la posición de la barra de Chrome en el runner de
  Windows; si no calzan, se conservan las anteriores.
