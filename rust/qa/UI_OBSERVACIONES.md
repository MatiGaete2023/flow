# Revisión visual sintética — Flow Estudio D

No contiene datos de usuario. Capturas generadas por la interfaz real de egui en Xvfb.

## Base observada

- 755e8d44: 88 capturas, Linux y Windows completos PASA.
- a3643cad: 104 capturas, Linux PASA; Windows en ejecución durante esta revisión.
- Tamaño físico 1366×768, escala 200 %: tamaño efectivo medido 683×384 puntos.
- Se observaron Trabajo a 100 %, Trabajo Lista a 200 %, Trabajo Observación a 200 %,
  Correos con lista/editor a 100 % y Contactos Lista a 200 %.

## Hallazgos y correcciones

1. Trabajo a 100 %: tabla, evidencia y editor separados; selección visible y una sola
   marca Revisada. Sin datos de bitácora, no se inventa una cronología.
2. A 200 %, navegación con nueve pestañas ocupaba dos filas: sustituida por selector
   de herramienta en ancho efectivo menor de 1000 puntos. Conserva todas las herramientas.
3. A 200 %, Revisada quedaba debajo del campo de texto y requería desplazamiento:
   acciones movidas antes del editor y campo con tres filas en vista compacta.
4. Contactos en vista compacta conserva Lista/Ficha y desplazamiento. Revalidar espacio
   después de compactar la navegación.
5. Correos a 100 %: lista izquierda y editor derecho, CC fijo visible, recibo contextual.
   El Período pasa a Opciones del reporte solo para Proyectos: correos.json confirma
   que es el único tipo que consume PERIODO. Su valor se conserva en sesión.

## Pendiente para cierre visual

Observar correcciones en capturas nuevas, resto de pantallas/escalas, estados no vacíos
de Revisión de bitácoras, teclado/foco, errores/diálogos, y contraste en sus estados.
No afirmar aceptación ni prueba de Outlook/captura real a partir de esta evidencia.

## Observación de 9f4e01b

Linux pasa303pruebasRust y168capturas reales:20escenarios más teclado,2resoluciones
y4escalas. Se observaron los mosaicos100/200% de ambas resoluciones y ampliaciones
de Revisión inicial, Propuesta, borrador incierto y proyecto obsoleto al200%.
Los mosaicos permiten inspeccionar distribución; no equivalen a leer todos los textos.
Revisión inicial muestra fechas y Analizar; Propuesta deja Revisada y texto legibles.
Proyecto obsoleto conserva su estado y valores con scroll.

La captura de Correos al200% aún dejaba el editor bajo todas las opciones. Corrección:
agrupa preparación/modalidades/alcance en bloque contextual compacto con resumen
siempre visible, y mueve estado/recuperación Outlook antes de los campos del editor.
Se añade prueba egui de los tres controles de incertidumbre visibles a683×384.
Falta observar la captura integrada de este correctivo.

Windows9f pasa tests/release/CLI, pero MesaMSVC26.2.7 no contieneOpenGL. La publicación
del mantenedor confirma el defecto y recomienda MinGWx64; se cambia la dependencia
de pruebas con SHA fijado. Esto no modifica el ejecutable ni distribuye el runtime.

