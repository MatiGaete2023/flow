# Revisión de eficiencia del proceso y de los productos

12 de octubre de 2026. **Sí, hay una forma más eficiente:** agrupar cambios compatibles, medir una vez la versión integrada y hacer que la interfaz indique una sola acción siguiente. Reducir controles de recuperación para ahorrar escrituras sería una optimización peligrosa.

## Alcance y evidencia

Leí primero `docs/implementacion/EFICIENCIA_PROCESO_20261012.md` y `EFICIENCIA_PRODUCTOS_20261012.md`. Después contrasté plan, manuales, interfaz, pruebas, verificador y construcción. Referencia fija: `adeb6ec`, presente en `origin/consolidacion/20261012` y `origin/feat/ejecucion-plan-20261008` al consultar. La ruta `/tmp/claude-0/.../scratchpad/limpio` indicada no existe en este entorno; utilicé una copia aislada en `/workspace/flow-ux-lectura`, sin editar sus fuentes.

Los cinco cambios comunicados como trabajo de otros agentes se consideran **dependencias en curso**, no funcionalidades comprobadas en esta referencia: exportación automática tras importar, consulta y paquete en un paso, retiro del campo de firmas, carpeta `respaldo/` y lanzador único. Antes de implementar UX debe fijarse el commit donde realmente estén integrados.

No corrí nuevamente las suites para una revisión documental. La línea base registrada corresponde a `7b4feb9`: 858 aprobadas, 11 omitidas y 6 xfail del Asistente; 240 aprobadas y 62 subpruebas del Descargador. Son resultados registrados, no una medición nueva ni aceptación Windows/Office. La CI de mi consolidación anterior `32dd14c` terminó correctamente; otros agentes añadieron WP-86 después. No atribuyo esa CI al nuevo código.

## Evaluación de los informes de la otra IA

| Propuesta | Evaluación y corrección necesaria |
|---|---|
| Una medición completa por ciclo | De acuerdo. `verificar_wp.py --linea-base` ya ejecuta y compara las pruebas antes de publicar la base: evita hacer una ejecución normal inmediatamente antes si la intención es actualizarla. Durante el desarrollo, pruebas focales. |
| Caché de verificación | Útil solo si sigue siendo segura. SHA por sí solo no cubre archivos locales, dependencias, plataforma ni configuración de pruebas. Identificar árbol limpio, entorno, comandos, inventario y versión del verificador; cualquier cambio invalida. Nunca convertir un informe FALLA en PASA ni aceptar artefactos no confiables. |
| Fuente única de estado | De acuerdo. El estado vigente debe ser estructurado; los documentos narrativos enlazan o se generan. Conservar informes históricos con su SHA y fecha: no reemplazarlos por cifras actuales. |
| E3 por riesgo | Ya figura como decisión **j** en `PLAN_VIGENTE.md`; no volver a pedirla. Abarca también anonimización, identidades y extensión SITFA. No basta contar líneas: un cambio de una línea puede ser riesgoso. |
| CI Linux y Windows bajo demanda | Buena dirección. Mantener Windows en la entrega candidata y cuando el diff afecta integración nativa, empaquetado, instalación o interfaz de Windows. Separar esos jobs del control rápido. `[skip ci]` no reemplaza validar enlaces/estado cuando cambian documentos operativos. |
| Hook que instala todo al iniciar | Ajustar: comprobar primero el entorno existente y reutilizarlo. Instalación automática en cada sesión repetiría el mismo desperdicio. Un worktree aislado puede seguir siendo útil; no equivale a un clon completo. |
| Un solo guardado al final de Outlook | **No implementar literalmente.** `outputs.py:create_draft` persiste `saving` antes de COM y `created`/`uncertain` después. Esa secuencia permite recuperar un cierre o resultado incierto sin duplicar borradores. Optimizar almacenamiento y hashing conservando esos puntos de durabilidad. |
| Evaluar solo la fila editada | Perfilar y probar equivalencia antes: fuentes, identidad, conciliación o configuración pueden afectar otras filas. Invalidez explícita por revisión y dependencias; cálculo completo como respaldo. |
| No reparsear una exportación propia | Puede reducir trabajo, pero no elimina comprobar archivo realmente escrito, identidad y fidelidad nativa/portable. Optimizar en la misma transacción; archivos externos siempre se verifican. |
| Reutilizar búsquedas/bitácoras | Buen potencial, sujeto a identidad remota, alcance, fecha y hashes. Mostrar que reutilizar no actualiza SITFA. No saltar consultas cuando el XLS carezca de vínculo verificable. Conservar pausas y validación real. |
| Borrar nóminas previas | No basta que una carpeta sea antigua. Preservar las referenciadas por trabajo, borradores, recibos y sesiones anteriores; limpieza con vista previa y confirmación, conforme D4. |
| Unificar Preparar y retirar productos secundarios | **Descartado por instrucción expresa del usuario.** Mantener ambos alcances y ubicación actual de ZIP, resumen, ICS y recuperación legado. No hay telemetría que demuestre que sobran. |

La afirmación «más de la mitad del costo» y el ahorro de tokens del 50–70 % no están sustentados por un registro reproducible del conjunto del proyecto. Las duraciones observadas en otra sesión sirven como referencia, no como garantía. En mi trabajo sí hubo repetición evitable: suites tras cambios sucesivos y pushes documentales que reiniciaron una CI todavía activa. La corrección es preparar una sola versión candidata antes de medir y publicar.

## Hallazgos propios sobre la experiencia y los productos finales

1. **El Asistente ya tiene barra de pasos, paleta y estados.** `next_step.py` presenta cinco pasos, incluida exportación manual; `app_base.py:_update_context` la actualiza y `_signature` compara datos periódicamente. Reemplazarla desde cero duplicaría mecanismos. Evolucionarla hacia cuatro etapas, conservando sus contratos y usando el mismo cálculo de productos. No declarar «revisado» porque no hay avisos, ni «guardado» porque hay borradores preparados.
2. **Los dos temas están implementados por separado.** Asistente: `personal/ui.py`, CustomTkinter y ttk; Descargador: `apariencia.py`, Tk/ttk. Un módulo común de tokens y estilos elimina divergencias sin cambiar toolkit. El Descargador ya admite Claro/Oscuro/Sistema; mantener ambas persistencias.
3. **Las actualizaciones genéricas pueden ocultar el resultado relevante.** `app_base.py:_poll` puede escribir «Operación terminada»; Descargador sustituye `status` durante las fases. Separar fase temporal de último resultado persistente, con aviso accionable. La finalización parcial no debe verse como éxito total.
4. **La interfaz principal del Descargador mezcla conexión, consultas y herramientas.** `Application.__init__` acumula acciones y necesita desplazamiento vertical. Agrupar y plegar herramientas poco frecuentes; mantener Pausar/Cancelar siempre visibles durante operación y el resumen del alcance antes de iniciar. El número aproximado de botones no mide por sí solo la fricción.
5. **Los manuales son documentación histórica, no especificación vigente completa.** El manual del Descargador describe el recorrido de ocho etapas por Resultados, aunque existe el paquete del lote actual. `MEJORAS_DISENO_20261007.md` menciona un adaptador RUS que contradice el retiro actual. Actualizar instrucciones del flujo final; marcar explícitamente la referencia histórica. No modificar textos del motor.
6. **El producto es una entrega utilizable, no solo un archivo generado.** Excel debe distinguir fuente original, copia de trabajo y fidelidad; Word debe indicar variables pendientes; Outlook debe diferenciar preparado, guardado e incierto; el ZIP debe tener ruta final y cobertura visible. Mantener fuentes y evidencias en `respaldo/` sin romper sus referencias. No atribuir envío, firma ni registro RUS a la existencia de esos archivos.
7. **Lanzador único no garantiza runtime único.** `build_integral.ps1` construye dos ejecutables PyInstaller. Un selector puede simplificar el acceso conservando ambas distribuciones; reducir tamaño exige otra decisión técnica y pruebas de instalación. El selector no debe hacer que el Asistente navegue SITFA.
8. **La comprobación de borradores sigue necesitando lecturas frescas al guardar.** La caché de WP-84 es deliberadamente de presentación. Reutilizarla en Outlook podría aceptar un adjunto cambiado. Compartir cálculo dentro de una operación es distinto de reutilizar resultados previos a un efecto externo.

## Plan general corregido: orden de ejecución

Los nombres siguientes son frentes de trabajo, **no nuevos WP asignados**: los informes proponen WP-87 con alcances diferentes. Resolver esa colisión en el estado canónico antes de crear fichas.

| Orden | Trabajo | Pasos concretos y salida comprobable |
|---|---|---|
| 1 | Estabilizar la ola en curso | Inventariar commits de los cinco cambios anunciados; integrar una vez; actualizar manuales y recoger una medición de esa versión. Evitar reimplementarlos en las dos apps. |
| 2 | Eficiencia del proceso | Separar CI rápida/empaquetado; generar continuación desde estado; reutilizar medición solo con procedencia comprobada. Registrar segundos, número de ejecuciones completas y minutos de Actions. |
| 3 | UX común y por app | Aplicar la especificación adjunta: primero tokens por implementador A; luego dos commits de interfaz independientes; revisión de diff y pruebas focales; un único control completo del conjunto. |
| 4 | Rendimiento local | Benchmark sintético de edición/exportación con 1.000 y 10.000 filas; unificar Excel compartido y evaluar invalidez por fila. Mantener paridad de datos y golden. |
| 5 | Outlook y retención | Perfilar hashes/escrituras para 30 borradores; conservar recuperación `saving/uncertain`; limpiar únicamente adjuntos sin referencias. Revisión E3 por riesgo. |
| 6 | SITFA y aceptación | Validar vínculos y reutilización en puesto real; completar WP-02 y diez ingresos de WP-50; solicitar tratamiento de refs/cachés pendientes a Soporte GitHub; cerrar WP-70 y fusionar a main después del acta. |

No hay base para estimar una fecha de término o porcentaje funcional: falta uso real. El código consolidado y las pruebas automáticas no resuelven esa dependencia. `main` sigue siendo la rama predeterminada; el intento anterior de cambiarla fue rechazado por permisos de la integración (HTTP 403).

## Cómo medir el resultado sin inventar ahorros

- **Proceso:** por ola, una medición completa local de la versión candidata; documentar otra solo si hubo cambios posteriores o fallo. Cero builds Windows por cambios exclusivamente narrativos. Mantener control completo para código candidato y controles focales de documentación.
- **Uso:** contar pulsaciones activadoras, selecciones y confirmaciones por separado; las esperas y los cambios de ventana se registran aparte. Comparar tarea equivalente con el mismo conjunto sintético.
- **Archivos:** registrar cantidad visible en la carpeta principal, rutas rotas, integridad del ZIP, tamaño instalado y número de accesos necesarios para localizar el producto. Mover intermedios no elimina su obligación de conservación.
- **Rendimiento:** medir mediana/p95 de edición, exportación y guardado; contar bytes hasheados y escrituras. Primero obtener línea base. Las estimaciones de la otra IA no se presentan como mediciones propias.
- **Aceptación:** probar pantalla pequeña y escala Windows, teclado, tema oscuro, detención y resultado parcial; Office/SITFA requieren prueba real y acta del usuario.

La [especificación de interfaz](ESPECIFICACION_UX_20261012.md) contiene archivos, funciones, contratos de pruebas y objetivos de clics (971 palabras). Comprobé por fórmula WCAG las 15 parejas de texto/fondo propuestas: todas superan 4,5:1, con mínimo 5,17:1; [resultados reproducibles](CONTRASTES_UX_20261012.json). Los estados hover/foco/disabled y el renderizado final todavía requieren prueba de interfaz. Las recomendaciones de proceso y rendimiento permanecen como propuesta; esta revisión no modifica apps, reglas ni workflows.
