# Auditoría y limpieza del Asistente — 21 de septiembre de 2026

Versión: **0.4.0.dev8**. Base inspeccionada: `3cbbad450c79ae1991832bf20fadc8c83ad1754e` (dev7).

## Objetivo y alcance

Mantener un único Asistente personal Windows y corregir rutas antiguas o pérdida de ajustes que compliquen su uso. Se inspeccionaron accesos, imports, métodos duplicados, transición entre archivos/productos, configuración, distribución y documentación vigente. No se cambian reglas sustantivas, textos «Se remite…», plantillas judiciales ni la política de solo borradores.

## Hallazgos y acciones

| Hallazgo comprobado en la base | Acción aplicada | Finalidad y evidencia |
|---|---|---|
| `src/nurus/app.py` aún incluía `NuRusApp` completo, pese a redirigir su `main` al Asistente | Sustituido por un pequeño acceso de compatibilidad; eliminado `product_ui.py`, usado únicamente por aquella interfaz | Impedir regreso al flujo de aprobaciones, snapshots y materialización; contrato de entrada única |
| `personal/app_base.py` duplicaba nueve métodos que `app.py` sustituía; su `main` instanciaba la variante base | Retiradas las versiones sustituidas; `main` redirige al Asistente | Evitar recuperar el bloqueo por falta de cruce o las antiguas modalidades al iniciar otro módulo; prueba de ausencia de métodos duplicados |
| Cambiar modalidad conservaba la hoja elegida para el modo anterior | Restablecer selección automática de hoja al cambiar modo | No analizar otra modalidad con una hoja arrastrada; prueba de transición sin descartar el trabajo |
| Actualizar una copia idéntica reconstruía las resoluciones y borraba asignaciones manuales | No reconstruir la vista si no hubo cambios | Conservar trabajo del usuario; prueba de actualización sin cambios |
| Reexportar también reconstruía la lista de proyectos | Actualizar registros sin reconstruir proyectos | Conservar tipos y selecciones manuales; prueba de exportación |
| Localizar una copia modificada no invalidaba los productos preparados de la copia anterior | Limpiar borradores/proyectos cuando se incorporan cambios; preservar todo si falla la lectura | Evitar usar productos anteriores por accidente; pruebas de localización válida e inválida |
| Al cambiar trabajo o preparar cero correos quedaban cuerpos, destinatarios o adjuntos anteriores visibles | Vaciar editores sin producto y referencia al Word anterior | Mostrar únicamente contenido vinculado al trabajo actual; pruebas de limpieza |
| Guardar umbrales sustituía todas las reglas desactivadas por cuatro casillas | Conservar desactivaciones que no administra ese formulario | Respetar configuración externa; prueba de persistencia |
| Portadas e índice aún presentaban dev6 como vigente y describían dos interfaces | Actualizados README, estado, implementación, índice y verificación | Tener una referencia actual y separar evidencia histórica |

## Qué se retira y qué se conserva

Se retiran la interfaz NuRus y sus diálogos, nueve métodos operativos duplicados, `tools/benchmark_pipeline.py` (medía el flujo histórico) y seis pruebas ligadas exclusivamente a la clase de interfaz eliminada. Se añaden diez regresiones del Asistente activo. La cifra de pruebas cambia por el retiro deliberado de una interfaz, no por ocultar fallos.

El historial Git conserva todo el código retirado. Se mantienen el nombre de paquete `nurus`, las columnas técnicas de Excel, la recuperación de sesiones y los accesos BAT anteriores como redirecciones. No se exige reinstalar otra configuración ni volver a cargar archivos por esta limpieza.

El mapa estático de imports no basta para decidir que un servicio puede borrarse: persisten dependencias transitivas entre exportador, modelos, catálogos y almacenamiento. Se mantienen los servicios compartidos y las pruebas de sus contratos; no se ejecuta una eliminación indiscriminada de módulos por tener nombre NuRus.

## Verificación

La compilación y la revisión del diff se ejecutaron localmente. La validación funcional se ejecuta exclusivamente en Windows con Python 3.12, 3.13 y 3.14. El SHA, run y resultados se consignan en `VERIFICACION_PERSONAL.md` al cerrar; no se traslada a dev8 el resultado de dev7.

No se declara aceptación de Excel 2010/Outlook institucional, ni corrección jurídica de los proyectos, ni ahorro de tiempo medido. Continúan pendientes Office institucional, conflicto DCE, matrices Tomé y comparación del ciclo real. No se añadieron aprobaciones individuales, otro sistema operativo ni dependencias de producción.
