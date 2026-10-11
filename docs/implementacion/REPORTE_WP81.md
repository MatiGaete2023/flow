# Revisión propia y continuación — WP-81

11 de octubre de 2026. Base remota revisada: `286c28f`, después de WP-63 (#34) y WP-80 (#35).
Esta revisión directa del Ejecutor responde a la instrucción del usuario; no constituye E3.

## Estado y razonamiento

El traspaso mezclaba instrucciones anteriores a la integración de #34/#35 con notas posteriores.
Consultar el estado remoto y los archivos fuente evitó volver a integrar paquetes o revertir decisiones.
WP-63/WP-80 y su E3 se conservan. El trabajo nuevo se separó en WP-81 desde la punta integrada.
Las correcciones adicionales subidas a la antigua rama de #34 en `3184a71` son posteriores a su fusión:
solo se trasladaron herramientas y pruebas a la base actual, conservando las mejoras de WP-80.
La línea base integrada se mantiene; no se rebajan métricas para que pase el verificador.

## Hallazgos corregidos

| Hallazgo | Riesgo y reproducción | Implementación |
| --- | --- | --- |
| P1: --salida acepta código/configuración | Una fuente modificada podía excluirse de la procedencia y sobrescribirse con JSON; reproducido con repositorio Git temporal | Resolver/validar ruta antes de ejecutar controles; solo JSON externo o dentro del directorio de evidencias; proteger línea base y rutas Git |
| P2: temporal abierto al fallar serialización | Windows no permite eliminarlo mientras está abierto; podía ocultarse el error original | Una sola secuencia try/finally; cerrar antes de eliminar y conservar el informe anterior |
| P1: nombres no revisados | RUT sintético válido solo en nombre, directorio o miembro ZIP no era detectado | Revisar metadatos antes del contenido, incluso PNG y directorios ZIP; conservar redacción de mensajes |
| O1: ZIP con compresión dañada | zlib.error/LZMAError escapaban con traceback/código 1, confundiendo error de lectura e identidad | Traducir ambos a error de contenedor ilegible: código 2, mensaje genérico sin contenido sensible |

Once regresiones fallaron antes de las primeras tres correcciones y dos antes de O1.
Se añadieron 14 casos de regresión (incluido directorio de un archivo Git); suite focal: 164 PASA.
El test previo de contenido con extensión desconocida sigue probando texto/ZIP detectado por contenido;
ahora distingue metadatos de PNG de sus bytes binarios. Ninguna prueba eliminada ni omitida nueva.

## Verificación

Pendiente de completar en el commit limpio de WP-81: controles generales y CI Windows.
El entorno Linux no acredita Office/SITFA real. Evidencia se registrará por SHA medido.

## Continuación y mejoras del plan

1. WP-81: verificar, entregar PR borrador y revisión/auditoría independiente E3; integrar y entonces
   regenerar línea base. La revisión propia no reabre E3 de paquetes ya integrados.
2. WP-17: residuos de etiquetas y referencias de PR; no reutilizar clones anteriores a la purga.
3. WP-02: ejecutar matriz de uso real local y devolver resultado agregado; Excel nativo y Outlook
   clásico necesitan PC autorizado. CI con pywin32 no acredita aplicaciones Office instaladas.
4. WP-50: revisar solapamiento con app_base.py integrado, contrastar 10 ingresos y esperar WP-02.
5. WP-70: acta final y métricas actuales antes de main; mantener dependencias confirmadas por usuario.

El plan pasa de 35 declarados a 37 paquetes reales (incluye WP-80/WP-81). CONTINUAR.md se
reescribe con un único punto actual; el anterior queda archivado, claramente histórico.
No se alteran decisiones funcionales del cierre. WP-81 debe resolverse antes del cierre.

G-3/G-4/G-5 de WP-80: primero reproducir bloqueo de ventanas secundarias, unificar el criterio de
borrador guardado y medir el coste de huellas; después corregir en paquete propio y verificar Windows.
Duplicación _recalculate/analyze: comparar entradas/salidas y ediciones humanas, extraer lógica común,
y demostrar golden17 antes de integrar. No eliminar motores, plantillas ni código de firmas/bitácoras
por parecer duplicado: comprobar alcanzabilidad, finalidad y decisiones pendientes.

Motor/textos, restricciones de envío/registro, CC obligatorio y seis xfail estrictos se conservan.
