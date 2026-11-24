# Puerta de habilitación del registrador real — 24-11-2026

## Alcance

Esta especificación operacional desarrolla P8/P9 del plan integral y la excepción expresa de AGENTS.md de esta rama. **No es un acta, ni un permiso, ni una configuración ejecutable.** CAPABILITIES de contracts.py sigue con cuatro valores false; no se crea adaptador real. No convertir el estado de esta tabla en un interruptor editable por el usuario. El acta de aceptación de versión es la prevista en PLAN_VIGENTE.md; no se inventa una segunda autoridad institucional ni se sustituye la comprobación de permisos aplicables al operador.

### Matriz de requisitos y evidencia mínima

| ID | Condición | Estado a este corte | Evidencia/acción mínima para cerrar |
|---|---|---|---|
| G01 | Contrato de identidad, texto exacto, destino, Estado, versión | Validado ficticio | RegistroFlowV1 + Work/ZIP verificado + revisión exacta vigente; no basta coincidencia de hash externo. |
| G02 | Catálogo y selectores de las cuatro vistas | Observado real, preparación ficticia | Espera 4 Gestión Previa; Cumplimiento 6 Realizada; Informes próximos/vencidos 9 Realizada. Selector, chkBitacora, editor y creación visible identificados en 0.2.1. Revalidar en cada uso, nunca seleccionar alternativa. |
| G03 | Transferencia de aprobación de Work a registrador | Previsualización real con fixture; DPAPI demo validada | Consumidor operativo local aún no existe; debe conservar revisión exacta, fuente, avisos y validar vigencia al recibir y antes de intentar. ConfirmedPreview no autoriza por sí. |
| G04 | Custodia del diario | DPAPI por campos implementada, CI pendiente | Verificar nativamente DB/WAL/SHM, bloqueo de corrupción, clave por diario, reserva duradera. Aún falta adaptar al contrato real y verificar ubicación privada. |
| G05 | Ubicación y acceso local | Pendiente | Ruta exclusiva aprobada, no nube/red/removible sin permiso, ACL comprobada de directorio y archivos, no reparse points ni rutas ambiguas; protección del volumen y copias según política. Verificar con cuenta estándar en Windows institucional. No basta os.chmod de Windows. |
| G06 | Retención y eliminación | Pendiente de decisión | Definir plazo por estado, responsable, respaldo/recuperación y eliminación de DB, WAL/SHM, paquetes cifrados y temporales. D4 (60 días/10 sesiones) de Flow es referencia, no autorización tácita. No purgar evidencia no resuelta automáticamente ni prometer borrado seguro en SSD. |
| G07 | Identidad efectiva en la ventana emergente | Validada ficticia | Comparación local de tribunal/causa/ingreso, etapa/modalidad/tipo con vínculo vigente, no compartir valores; rechazo de duplicados, sesión expirada, otra pantalla, nueva ventana ambigua o DOM cambiado. No se requiere repetir exploración de nombres de controles. |
| G08 | Preparación real sin guardado | Pendiente | Una comprobación autorizada sin submit: verificar texto exacto, Estado, casilla CC, contador/eventos y ausencia de borrador ajeno. No inferir límite servidor por editor oculto ni ejecutar handlers desconocidos como solución. |
| G09 | Autorización humana | Validada ficticia | Operador competente confirma conjunto exacto, identidad, texto, destino, Estado y versión en sesión válida; sin aprobación tácita ni reutilización tras cambio. La aceptación de software no es aprobación de una observación individual. |
| G10 | Lectura viva previa y relectura | Parser existente; comparación ficticia | Snapshot local del ingreso exacto, reciente, completo y con IDs persistentes, autor y fecha/estado/tipo. Si faltan páginas o hay previa equivalente, no intentar; observar evidencia viva durante piloto autorizado, sin compartir contenido. |
| G11 | Diario y ritmo | Validado ficticio | Un intento activo; reserva transaccional persistida antes de cualquier acción, intervalo/cupo respetando límites de servicio; pausa, detención, error crítico. Sin reintentos por HTTP, caída, timeout o ausencia de fila. |
| G12 | Instalación operativa | Pendiente | Chrome/Windows institucional sin administrador ni cambios de política; extensión de demo y pasivo no son escritor. Revisar permisos mínimos exactos. No eludir restricciones, ni automatizar ingreso de credenciales. |
| G13 | Conciliación Excel | Ficticia Linux/Windows | Copia nueva, identidad y recibo exacto, no modificar original; compatibilidad Excel nativo y variantes de formato pendientes. Un error Excel jamás justifica otro intento RUS. |
| G14 | Piloto unitario | Bloqueado | Solo cuando precondiciones G01–G12 estén acreditadas para un entorno autorizado; una observación necesaria y explícitamente aprobada, creación normal visible. No editar/borrar, no POST directo, no registrar datos inventados en RUS real. Verificar después y documentar resultado categórico. |
| G15 | Aceptación y escala | Bloqueado | Acta de usuario de versión, límites institucionales, evidencia del piloto y resolución de incidencias; 400 operaciones solo ficticias hasta decisión expresa de escala. No confundir rendimiento virtual con servicio real. |

## Secuencia de bloqueo

1. **Antes de abrir sesión real:** G03/G04/G05/G06/G12 y autorización aplicable resueltas, política y acta según plan. Si falta una, no habilitar.
2. **Antes de preparar:** G01/G02/G07/G08; revalidar revisión, vista, ingreso y formulario; detectar cambios. La única escritura permitida en eventual piloto es una nueva observación por UI normal, nunca modificación/borrado. Sin acciones invisibles, POST directo ni bypass.
3. **Antes del intento:** G09/G10/G11, verificación actual del formulario y fuente, autorización exacta, reserva persistida, no intento previo ni incierto. Una referencia de formulario obsoleta no puede corregirse automáticamente.
4. **Después:** relectura única y comparación, no inferir éxito por ACK, HTTP 200 o cambio de contador. Si resultado incierto, recuperar solo por lectura; si no hallada/ambigua, detener y escalar a revisión humana. Conciliar Excel solamente con recibo comprobado.
5. **Cierre:** acta y política de retención, sin publicación de datos reales.

## Decisiones no tomadas

- No hay ruta institucional, ACL concreta o plazo de retención acordados para el diario real. El modo de simulación protegido no certifica esos requisitos.
- No hay evidencia de que los eventos ficticios input/change tengan los mismos efectos en RUS; falta observación autorizada sin guardar.
- No se acredita persistencia real, ni resultado en bandeja del tribunal más allá de la regla funcional de la casilla fijada por el usuario.
- No se habilita un interruptor de escritura, ni se solicita un registro de prueba hasta satisfacer precondiciones.
- No se modifica Descargador, motor ni otras ramas; no se amplía el alcance a SATURNO.
