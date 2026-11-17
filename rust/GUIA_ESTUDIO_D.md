# Flow Estudio D · guía de pruebas

Esta versión candidata se entrega para revisión del usuario. No escribe en RUS ni
SATURNO y nunca envía correos. Outlook recibe exclusivamente borradores; se mantiene
el CC fijo ucc_concepcion@pjud.cl. Las pruebas con la sesión real de Outlook y SITFA
requieren al usuario y se registran por separado de las pruebas sintéticas.

## Abrir y orientarse

Descomprime la carpeta completa. `flow.exe` reúne las herramientas. Las copias
Flow_Asistente, Flow_Descargador, Flow_Anonimizador y Flow_Meta4 abren solo su herramienta.
En pantallas pequeñas o con escala alta, el selector Herramienta reemplaza las pestañas.
Estado muestra el mensaje completo y las condiciones de guardado. Ayuda abre con F1.

En Archivo puedes abrir Excel/paquetes y reanudar una sesión local. La recuperación
indica archivo y fecha de corte; queda deshabilitada si no hay sesión. Cambiar Corte o
Etapa no descarta una edición: Más → Recalcular propuestas pide confirmación.

## Trabajo y productos

Trabajo combina tabla, evidencia y observación. Arrastra sus separadores para ajustar
el espacio. Restaurar distribución vuelve al reparto inicial. En tamaño compacto,
Lista, Evidencia y Observación muestran cada zona por separado. Puedes desplazarte
con Tab/Mayús+Tab y activar controles con el teclado.

Seleccionar una fila delimita productos; Revisada registra tu revisión. Son estados
independientes. Edita la observación, copia su texto o restaura la propuesta original.
Ctrl+S guarda el Excel de Trabajo mientras esa herramienta está activa.

En Correos, Preparar este tipo reemplaza la lista por el tipo elegido y Preparar
borradores prepara el conjunto. Revisa el alcance y las modalidades antes de preparar.
Otra modalidad (texto libre) personaliza ese texto y mantiene su alcance tradicional.
Período aparece en Opciones del reporte de proyectos. Excluir este borrador lo omite
del guardado; la búsqueda de la lista no cambia esa exclusión.

Puedes guardar en Outlook cuando esté disponible o como .eml. Si un guardado quedó
incierto, revisa sus recibos y resuelve ese estado antes de intentarlo de nuevo.
Un borrador obsoleto debe prepararse nuevamente. No se envían mensajes.

En Resoluciones prepara, revisa y genera el Word. Plantillas conserva el acceso a
plantillas propias. Los marcadores [COMPLETAR …] requieren revisión del documento.
Resultados permite buscar y abrir los archivos producidos, con mensajes y recibos
separados. Destino de próximos productos cambia la salida posterior; las carpetas
anteriores siguen disponibles durante la sesión.

## Revisión de bitácoras

1. Abre Revisión de bitácoras en Flow o Flow Asistente y elige Abrir paquete.
2. Revisa Desde, Hasta y Corte. El intervalo capturado se propone cuando está
   acreditado; puedes ajustarlo. Los límites son inclusivos y Hasta no supera Corte.
3. Pulsa Analizar bitácoras. En tamaño compacto, Configurar reúne criterios y acciones;
   los criterios también aparecen antes del primer análisis.
4. Revisa las cuatro categorías: Sin observación, Con observación, No verificable y
   Fuera de alcance. Sin observación se refiere al centro y al período elegido.
   Una lectura ausente, parcial o con autoría incierta no demuestra ausencia.
5. Usa Lista para elegir el ingreso y Evidencia para consultar identidad, procedencia,
   cobertura y entradas. Una causa compartida no mezcla ingresos ni tribunales.
6. Aplica el motor a los registros Sin observación. El cálculo usa el contexto completo
   del paquete; solo esos registros reciben propuestas. Esto no genera correos ni Word.
7. Revisa Propuesta, edita si corresponde y marca Revisada. También puedes marcar
   revisadas las otras categorías, aunque no tengan propuesta.
8. Selecciona filas para exportar. Los filtros pueden ocultar filas seleccionadas;
   el contador mantiene su alcance. Seleccionados es la opción inicial. Elige Todos
   los clasificados solo cuando quieras exportar el informe completo.
9. Exportar listado crea juntos un Excel y un JSON en una carpeta nueva. El Excel
   tiene Resumen y las cuatro categorías. El JSON conserva evidencia, procedencia,
   propuesta original, edición y revisión. Abrir resultados lleva a esa carpeta.

Cambiar paquete o período pide confirmación si existen propuestas, ediciones o marcas.
Hasta volver a analizar, el criterio cambiado impide aplicar reglas o exportar.
Cancelar o una operación fallida conserva la revisión anterior. La exportación nunca
reemplaza una carpeta ya existente ni publica solo la mitad del informe.

Los paquetes antiguos siguen siendo legibles. Cuando no conservan las pruebas de
autoría/cobertura requeridas, el resultado es No verificable. Una captura explícita
nueva obtiene evidencia actual sin convertir el archivo antiguo en una lectura completa.
Esta herramienta de revisión no inicia capturas.

## Prueba inicial con datos ficticios

Abre `Paquete_FICTICIO.zip` en Revisión de bitácoras. Con Desde 01/11/2026, Hasta y
Corte 17/11/2026, Analizar debe producir cuatro registros: uno de cada categoría.
Aplica el motor, revisa la propuesta de X-1-2099, edítala y marca Revisada. Selecciona
ese registro y exporta Seleccionados: el Excel/JSON deben incluir solo esa fila, con
los totales del universo explícitos. Exporta Todos para comprobar las cuatro categorías.
Prueba después un cambio de fecha, cancelación, búsqueda y navegación al 200 %.
El paquete y todos sus nombres/personas son ficticios.

## Otras herramientas

Captura separa conexión, configuración y ejecución. Consultar y crear paquete incluye
bitácoras; la descarga de lote conserva la elección de lecturas. Retomar tiene rutas
distintas para paquete y lote. Detener descarga conserva el avance de la consulta;
no es una pausa. Avanzado muestra un resumen de filtros/PDF activos.

Anonimizador: Agregar admite archivos o carpeta; Quitar elimina entradas de la lista,
sin borrar originales. Estricto sigue siendo el modo inicial. Avanzado reúne semilla
y fechas; la semilla permanece en memoria. Revisa la vista previa antes de generar.

Meta 4: distingue Generar y Regenerar. Todos los disponibles y Elegir tribunales
explicitan el alcance; Elegir sin selección impide generar. Usar formato estándar
quita la plantilla propia. Se conserva la producción en carpeta nueva y los originales.

Contactos: busca en la lista y edita la ficha. Importar CSMP/Catastro muestra una
vista previa antes de confirmar. Releer recupera fallos de lectura; Guardar en Flow
incorpora contactos legados que estaban en solo lectura.

## Registrar la prueba del usuario

Anota versión/huella del paquete, Windows y escala, herramienta, pasos, resultado
esperado y observado. No compartas datos de causas/personas en capturas de incidencias.
La aceptación funcional del usuario queda pendiente hasta realizar estas pruebas.

Las huellas de archivos están en SHA256SUMS.txt y la procedencia de compilación en
CANDIDATO.json. La revisión técnica conserva por separado las fallas heredadas del
verificador general; esta entrega no autoriza una integración automática ni una release.

