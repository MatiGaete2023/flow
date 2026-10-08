# Recuperación ejecutada de NuRus
Fecha: 8 de septiembre de 2026. Rama: implementacion-plan-2026-09-08.

## Resultado
Se restableció una vía de desarrollo mediante el conector GitHub y las pruebas existentes de GitHub Actions. No se restableció el entorno local ni se recuperó el ZIP. Los cambios siguientes se implementaron de nuevo sobre el contenido remoto exacto, conservando los archivos ajenos en el árbol Git.

## Cambios publicados
| Commit | Problema | Cambio y prueba |
|---|---|---|
| 052ca7826b4e12f387b0fa5015effd05abc756b9 | Faltaba garantizar CC UCC y un guardado podía devolver identificadores vacíos | CC obligatoria en preparación y guardado; conserva otras copias y evita duplicados; acepta Para vacío; rechaza productos no correo; EntryID/StoreID incompletos producen estado incierto |
| 848e84bc88d85207d6eaf6f48f22037a48607f92 | Primera aparición de duplicados aceptable; alias dependiente del formato del ID; importaciones antiguas podían reemplazar correos cambiados | Todas las apariciones duplicadas quedan en conflicto; alias usa ID real; transacción inmediata; comparación de email/CC anteriores; conflictos de alias/identidad revierten la importación |
| dc3d08e898f496f884a1a6c6cf82e86f7e1d07af | Espera e Informes tomaban la primera hoja de libros con varias pestañas | Selección por nombre de modalidad; exige selección explícita si no se identifica en un libro de varias hojas; prohíbe OB y Medidas vencidas como entrada principal; conserva hoja única de nombre genérico |

## Evidencia
- Correo: GitHub Actions 34287246724, Windows y Linux satisfactorios; log Windows: 36 pruebas aprobadas.
- Contactos: GitHub Actions 34287359238, Windows y Linux satisfactorios; log Windows: 40 pruebas aprobadas.
- Lector: GitHub Actions 34287468858. Windows y Linux satisfactorios; 47 pruebas aprobadas en cada entorno.
- Se volvieron a leer los ocho archivos de código/pruebas afectados desde dc3d08e y coinciden exactamente con el contenido publicado.
- Los tests de Outlook usan simulaciones. Los runners Windows de CI no acreditan uso real con Windows 10, Excel 2010 u Outlook institucional.
- No se enviaron correos ni se publicaron planillas, bases de datos o productos con información de causas.

## Procedimiento que evita repetir el bloqueo
1. Leer el fragmento remoto exacto y fijar el commit base.
2. Implementar una corrección pequeña con pruebas de regresión.
3. Crear el commit conservando el árbol remoto restante.
4. Actualizar solo la rama de integración, sin force.
5. Consultar GitHub Actions por SHA, incluidos eventos push; el método limitado a pull_request no muestra todas las ejecuciones.
6. Verificar resultados y leer archivos publicados antes de declarar el cambio comprobado.
7. Mantener el checkpoint actualizado sin depender de un directorio temporal como único respaldo.

## Pendientes materiales
La recuperación operativa no completa NuRus. Falta recuperar o reconstruir y probar:
- snapshots históricos inmutables, bytes fuente y aprobación transaccional con excepción;
- integración final de exportación que conserve todo el libro y marque exclusiones;
- catálogo completo de correos, adjuntos, interfaz y contador;
- matrices y exportación de proyectos, con verificación visual;
- respaldo y aceptación integral de instalación/recuperación en Windows;
- matrices acreditadas faltantes y pruebas reales de Excel/Outlook.
El contenido de la implementación local anterior no debe darse por incorporado. Las 52 pruebas anteriores son evidencia histórica separada de las pruebas remotas de esta recuperación.

## Próxima acción concreta
Revisar Database.approve_batch y WorkController.approve_batch en el HEAD vigente, reproducir el riesgo de aprobación parcial con una prueba de rollback y publicar su corrección por el mismo circuito remoto.

## Si vuelve el acceso local
Inspeccionar el ZIP y el directorio anterior, comparar contra estos commits y recuperar únicamente diferencias necesarias. No sustituir la rama completa con la copia antigua.

