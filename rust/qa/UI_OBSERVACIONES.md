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

