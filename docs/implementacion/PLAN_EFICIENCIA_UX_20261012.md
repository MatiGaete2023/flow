# Ola de eficiencia y UX

12 de octubre de 2026. Autorización: «Actualiza los planes en el repositorio y comienza a implementar».

## Fuentes y prioridades

Primero se analizaron [eficiencia del proceso](EFICIENCIA_PROCESO_20261012.md) y [eficiencia de productos](EFICIENCIA_PRODUCTOS_20261012.md). El [contraste de los informes](REPORTE_EFICIENCIA_Y_PRODUCTOS_20261012.md) corrige las propuestas; la [especificación UX](ESPECIFICACION_UX_20261012.md) fija archivos, contratos y aceptación. Estos documentos describen la revisión sobre `adeb6ec`, no resultados posteriores de implementación.

El estado de los WP vive en `estado/*.json`, agregado en [ESTADO.json](ESTADO.json). Cifras vigentes y commit medido: [linea_base.json](linea_base.json). Evitar copiarlos a cada documento. Los informes de verificación e históricos conservan su procedencia.

## Orden implementable

| Fase | Alcance y criterio de término | Estado inicial |
|---|---|---|
| UX-01 / WP-87 | Tokens y estilos comunes claro/oscuro; adaptadores CTk y ttk compatibles con preferencias actuales; pruebas de contraste y de ambas apps. Sin cambios de negocio. | Implementado y verificado; integración en estado WP-87 |
| Flujo integrado / WP-88 | Autoexportar tras importar; consulta y paquete en un paso; preferencia XLSX única; retiro de campo firmas; intermedios en respaldo; lanzador con dos modos. Se comprobó que faltaban en e91e360; el usuario autorizó completarlos en WP-88. | Integrado en WP-88/#42; uso real y Windows pendientes |
| UX-02 | Asistente: extender barra existente a cuatro etapas y siguiente acción; jerarquía contextual; vacíos y resultados persistentes; contratos de widget intactos. | Integrado en WP-88/#42; uso real y Windows pendientes |
| UX-03 | Descargador: grupos principal/lote/avanzado, pausa visible, progreso con ETA honesta y ruta final accesible. | Integrado en WP-88/#42; uso real y Windows pendientes |
| EF-01 | CI rápida separada de construcción, documentos de continuación generados y reutilización segura de mediciones. Validar links/estado en documentación; Windows en candidato y cambios nativos. | CI en WP-89; generación de continuación y reutilización segura de mediciones siguen pendientes |
| EF-02 | Medir edición/exportación con datos sintéticos; reducir duplicación y cálculo conservando dependencias y paridad. | Obtener benchmark antes de optimizar |
| EF-03 | Optimizar borradores y adjuntos conservando persistencia previa/posterior a COM, `uncertain` y referencias de sesiones; E3 por riesgo. | Pendiente; no cabe en una ficha de solo interfaz |
| EF-04 | Reutilización SITFA por identidad/alcance/fecha/hashes, con pausas intactas y prueba real. | Pendiente de evidencia del puesto |

WP-87 queda reservado a UX-01: los dos informes anteriores proponían el mismo número para alcances distintos sin haber creado fichas. WP-88 fue asignado después de comprobar el remoto y recibir «Efectua los cambuos del otro agente que no esten y continua». No duplica trabajo integrado. Cada fase futura obtiene su ficha y alcance en el estado canónico al confirmar base y responsable.

## Límites de esta autorización

- Conservar «Preparar tipo» y «Preparar todos» separados, con sus alcances; no retirar ni mover ZIP de entrega, resumen estadístico, ICS o recuperación legado.
- Motor, umbrales, `textos_base.json`, contratos, CC fijo y prohibición de enviar/registrar no cambian. El Asistente no inicia el Descargador.
- E3 según decisión j: obligatoria en cambios riesgosos; no inventar revisión independiente para interfaces. Outlook, retención, identidades y extensión conservan revisión cruzada cuando corresponda.
- No implementar «guardar Outlook solo al final»: conservar durabilidad de cada efecto y resultado incierto. Caché de presentación no sirve para autorizar un guardado.
- Una rama y PR para el alcance concreto de WP-87. Agrupar commits compatibles dentro de ese alcance; cambios de riesgo tienen ficha separada.
- Durante desarrollo: pruebas focales. Comprobar primero imports y pantalla con rutas absolutas al worktree; revisar el diff y congelar el candidato antes de medir. Sobre candidato limpio: una ejecución completa del verificador que también puede actualizar la línea base. Repetir solo ante cambios de código, fallo o preocupación pendiente.
- La reducción de CI se implementa en WP-89 con E4 revisada: no utilizar `[skip ci]` para esconder cambios de código. No monitorear CI periódicamente.

## Aceptación

Contrastes AA de texto y estados, teclado/modal, preferencias persistidas y contratos existentes comprobados en pruebas sintéticas. Clics, geometría Windows a escala 150 % y Office/SITFA se contrastan en el puesto del usuario; no afirmar que fueron medidos aquí. WP-02, soporte GitHub/WP-17, diez ingresos/WP-50 y cierre/WP-70 siguen pendientes funcionalmente. Sin main/release antes del acta final.
