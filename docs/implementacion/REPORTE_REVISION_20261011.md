# Revisión del código y del plan — 11 de octubre de 2026

**Resultado:** la revisión se implementa en WP-63, rama `wp/auditoria-refactor-20261011`, sobre
`2cf8ca893e85a0203b1c051026b87f16ff6be964`. Los errores descritos se reprodujeron con datos ficticios y se corrigieron
con regresiones. [PR #34](https://github.com/MatiGaete2023/flow/pull/34), en borrador para revisión E3.
Motor, umbrales, textos tradicionales y golden permanecen intactos. No se declara aceptación de uso real.

## 1. Estado comprobado al retomar

El traspaso del 10 de octubre quedó superado por las integraciones del 11: #4, #8, #11 y #12–#32, estado dividido
por WP y WP-61 ya están en `feat`. Actions funciona: los dos controles de PR #33 están verdes. WP-50 está implementado
en ese PR, pero sigue pendiente del acta de 10 ingresos y WP-02. D2 indica «Ninguna todavía».

El historial de las ramas se reescribió; esta revisión usa un clon nuevo. WP-17 aún requiere limpiar dos etiquetas
antiguas y referencias de PR mediante Soporte de GitHub. Los 607 artefactos anteriores ya fueron borrados por la
sesión anterior; esta revisión no repitió la purga ni recuperó ramas antiguas.

Medición previa reproducida: **Asistente 592 aprobadas, 11 omitidas y 6 xfail; Descargador 231 aprobadas y 62 subpruebas**.
Golden de 17 productos PASA; RUT detectados, lint, módulos no alcanzables y enlaces rotos: 0.
[Informe previo](verificacion/WP-63-base.json). El SHA de `linea_base.json` se corrigió después de la purga;
los umbrales se conservaron y se añadió el inventario exacto de pruebas recolectado en un checkout limpio de `2cf8ca8`.

## 2. Hallazgos e implementación

Alta significa exposición de datos, pérdida de archivos/trabajo o aprobación incorrecta. Media significa flujo bloqueado,
resultado obsoleto o gasto evitable. Cada grupo de errores de código tiene regresiones de comportamiento; los cambios documentales se verifican con estado, enlaces y lectura cruzada.

| Prioridad | Error reproducido | Implementación |
| --- | --- | --- |
| Alta | Se filtraban nombres bajo APELLIDO/APELLIDOS, DEMANDANTE, DEMANDADO, SOLICITANTE e INTERVINIENTE | Ampliar clasificación sensible y reemplazos relacionados; seis alias cubiertos en regresión |
| Alta | Un RIT podía conservarse idéntico al pseudónimo generado | Reservar todos los RIT originales de hojas/títulos antes de asignar reemplazos |
| Alta | Texto literal que comenzaba con `=` se convertía en fórmula al anonimizar | Escribir cadenas como texto explícito; mantener defensa contra fórmulas sin apóstrofo visible |
| Media | Texto inocente que empezaba por `#` se perdía como supuesto error Excel | Usar el tipo real de celda de error, en lugar de inferirlo por caracteres |
| Alta | Fallos al publicar una copia dejaban un archivo parcial o vacío; el hash se calculaba después de publicar | Temporal único, hash antes de publicar, destino exclusivo y limpieza solo del archivo creado por esa operación |
| Alta | Consolidado corrupto o con cabeceras/datos distintos podía usarse como fuente de la copia | Validar CRC, rutas, duplicados, enlaces y compresión interna; comprobar coherencia del contenido antes de elegir copia nativa y caer a portable si no es compatible |
| Media | OBSERVACION revisada conservaba apóstrofo visible | Usar escritura literal segura para propuesta, campos técnicos y revisión humana |
| Alta | Reimportar el mismo ZIP desde otra carpeta conservaba la ruta anterior y perdía su consolidado | Actualizar procedencia a la ruta recién validada, preservando ediciones; rechazar conciliaciones innecesarias |
| Media | Datos guardados se bloqueaba con contenedores JSON parciales o `attachments: null` | Inventario tolerante, sin borrado; excluir enlaces al JSON vigente |
| Alta | Retención podía borrar un Conjunto antiguo con una descarga anidada todavía en uso | Proteger también los contenedores antecesores de todas las fuentes activas |
| Media | Una caché corrupta al retomar permitía superar el tope de aperturas | Comprobar el límite inmediatamente antes de cada apertura real, después de verificar reutilización |
| Media | Un fallo al persistir el cierre dejaba Chrome bloqueado | Liberar el bloqueo en `finally`, conservando el error de persistencia |
| Media | Destinatario/CC inválido rompía Resultados al consultar recibos | Mostrar incidencia «Por corregir» y no calcular huellas para direcciones inválidas |
| Media | Siguiente mantenía conteos antiguos tras editar contactos, proyectos, recibos o perder adjuntos; un error impedía reintentar | Firma con contenido efectivo y metadatos; guardar la firma solo tras completar el cálculo |
| Media | Importar último validaba ZIP en Tk y un manifest comprimido dañado abortaba toda la búsqueda | Búsqueda en el trabajador existente; confirmación/importación en Tk; cancelar antes de importar e ignorar candidatos corruptos |
| Media | El XLSX consolidado se generaba dos veces en ambas rutas GUI | Una sola generación, reutilizando los bytes y conservando la nota de fallback |
| Alta | Pytest/procesos con fallo o salida ilegible podían dar PASA; una medición fallida podía reemplazar la línea base | Exigir código, resumen y datos válidos; timeout/ausencia son FALLA; baseline solo se publica si todo pasa |
| Alta | Bajas numéricas y nodeids inventados daban holgura al verificador | Rechazar bajas numéricas positivas; exigir pertenencia exacta al inventario de la base y ausencia actual con recolección válida |
| Alta | RUT dividido entre runs OOXML o dentro de un XLSX anidado escapaba al scanner; un ZIP corrupto se consideraba limpio | Unir segmentos visibles, recorrer ZIP/OOXML anidados con límites y devolver error si no se puede medir; enmascarar diagnósticos |
| Alta | Informe sin árbol exacto podía referirse a código local distinto; «integrable» podía ignorar controles fallidos | SHA completo, árbol Git, cambios locales e igualdad antes/después; elegibilidad exige procedencia y todos los controles PASA |
| Media | Estado admitía ciclos funcionales o podía regenerarse con fuentes incoherentes | Detectar ciclos y validar antes de agregar; dependencias de archivo mantienen su semántica |
| Media | CI ejecutaba compilaciones/Playwright en PR, invirtiendo E4 | PR con pruebas mínimas 3.12; matriz y pasos pesados al integrar o ejecutar manualmente |
| Media | Plan/manuales mantenían decisiones pendientes o botones/versiones retirados; CONTINUAR mezclaba varios estados | Sincronizar decisiones, manuales y estado; traspaso de 1,7 KB y contenido anterior en HISTORIAL |

La revisión de apoyo del conjunto detectó además la incoherencia entre una celda de T ESPERA del consolidado y el CSV
usado por el motor: se incorpora a la validación de coherencia indicada arriba. Verificar solo identidad/procedencia no
basta para asegurar que el Excel muestre los datos analizados.

## 3. Simplificación y optimizaciones

Se reutiliza el trabajador asíncrono existente, sin crear otro sistema de hilos. La búsqueda ya no bloquea Tk y no importa
si la operación se canceló. La barra revisa tamaño/fecha de modificación de adjuntos, evitando leer su contenido en
cada pulso; la validación efectiva de productos sigue comprobando sus archivos.

Los dos flujos de paquete comparten la generación del consolidado y su nota de resultado. Las copias usan destinos
exclusivos y temporales únicos; no se elimina un archivo que pertenecía a otra operación.

La heurística de bajas de pruebas se elimina, y las pruebas nuevas no ocultan nodos retirados sin declarar: el inventario exacto reemplaza inferencias sobre nombres de funciones,
clases o parámetros. El verificador publica métricas y huellas, sin copiar excepciones potencialmente sensibles.

Se comprobó que Issue/Evaluation y las plantillas JSON sin lectores ya habían sido retirados por la sesión anterior.
No se repitió esa eliminación. La meta aproximada de 600 archivos no justifica borrar pruebas, manuales ni documentos
rectores. Las siete fuentes originales se conservan.

## 4. Cambios implementados en el plan

1. **WP-63 añadido**, con alcance, dependencias y criterios explícitos: auditoría correctiva del conjunto integrado.
   Su ficha y archivo de estado mantienen la trazabilidad; ESTADO sigue siendo una vista generada.
2. **WP-70** depende funcionalmente de WP-02, WP-17, WP-50 y WP-63. El cierre no puede tratar aceptación/purga/revisión
   como simples conflictos de archivos. Las dependencias de formato/documentación restantes se conservan.
3. **Decisiones sincronizadas:** D1 y D3–D9 ya respondidas; D2 no tiene casos probados. Se registran los seis conflictos
   R-T pendientes, con responsable y tratamiento; las dos decisiones ya respondidas quedan identificadas como resueltas.
4. **E2 aplicado:** baseline posterior a la purga identificable, inventario exacto, medición completa y árbol estable.
5. **E4 aplicado a ambos workflows:** compilación e instalador al integrar/manual, evitando ese gasto en cada PR.
6. **E3 se conserva:** revisión de otra raíz antes de integrar. Los subagentes de esta revisión son apoyo;
   no se declara APROBADO/CONFORME formal. La excepción anterior solo cubría los PR ya integrados.

## 5. Validación

Verificación final **PASA**, medida sobre `473175c25b24e1d8f7fb3163ed95b1d3076443e9`, con árbol Git limpio
y sin cambios durante la ejecución. [Evidencia final](verificacion/WP-63.json).

| Medición | Resultado |
| --- | --- |
| Asistente | 767 aprobadas, 11 omitidas, 6 xfail estrictos preexistentes |
| Descargador | 235 aprobadas y 62 subpruebas |
| Inventario de la base | Sin bajas no declaradas; todos los nodeids originales presentes |
| Paridad | 17 productos golden sin cambios |
| Identidades, lint, módulos no alcanzables y enlaces rotos | 0 hallazgos en cada control |
| Restricciones y procedencia | PASA; mismo SHA/árbol limpio antes y después |

Los nueve controles pasan. Formato y estado se comprueban también por separado. La CI Windows se observará en el
PR; su resultado no certifica Excel ni Outlook reales. Los commits posteriores al medido solo completan documentación
y evidencia; el código/configuración/pruebas verificados permanecen idénticos.
Las regresiones focales y la revisión de apoyo sirven de diagnóstico; no reemplazan esa corrida completa ni el acta.
La primera corrida conjunta detectó cinco pruebas antiguas acopladas a la API privada retirada del verificador.
Se actualizaron sus fixtures y aserciones al contrato E2 (inventario medido, código de proceso y JSON de Ruff),
preservando todos sus nodeids y su propósito. El [informe FALLA](verificacion/WP-63-falla-api.json) se conserva para trazabilidad.
Ninguna prueba existente se retira ni se omite para ocultar un fallo. Los seis xfail son los conflictos estrictos
preexistentes de WP-60; no se convierten en aceptación.

## 6. Pasos pendientes, en orden

1. **Revisión técnica:** revisar PR de WP-63 desde otra raíz de agente (E3), integrar en `feat` y regenerar línea base
   después de esa integración. En caso de fallo, corregir y repetir el control afectado antes de integrar.
2. **Privacidad del historial:** borrar las dos etiquetas antiguas desde la cuenta del usuario y pedir a Soporte GitHub
   limpiar referencias de PR. Usar clones nuevos; no hacer push desde los worktrees anteriores a la purga.
3. **Uso real (WP-02/D2):** en el PC autorizado probar descargas, recuperación, retención, importación, copias y
   borradores. Comunicar solo casos/resultados agregados y completar el acta sin identidades ni adjuntos reales.
4. **Windows/Office:** probar salida nativa de WP-51 con Excel y borradores con Outlook clásico. Los tests con mocks
   y la CI Windows no certifican Office ni la compatibilidad del PC autorizado.
5. **WP-50:** contrastar 10 ingresos conocidos según la ficha y registrar acta. Integrar PR #33 tras WP-02;
   resolver el solapamiento en app_base.py con WP-63 y verificar nuevamente golden/cronología/catálogo.
6. **WP-70/D7:** con acta final, actualizar métricas, sacar PR #1 de borrador y fusionar a main, sin release.

## 7. Límites y siguientes mejoras

El anonimizador reconoce identidades por estructura y valores relacionados; un nombre presente únicamente en una
columna de texto libre desconocida requiere revisión local. No se garantiza una anonimización universal por patrones.
El scanner de CI controla RUT en texto y OOXML; sus resultados no prueban ausencia de cualquier tipo de dato personal.

La cancelación de Importar último se comprueba antes/después de la búsqueda; no interrumpe una lectura ZIP en progreso.
Si los lotes reales justifican más optimización, incorporar puntos de cancelación entre candidatos, conservando la
confirmación en Tk y pruebas de no importación tras cancelar.

Como siguiente mejora de instalación, separar locks de ejecución y construcción para reducir descargas sin red.
Debe derivarse del lock probado y verificarse en Windows con inventario/hash; no quitar dependencias a mano ni prometer
instalación offline sin un wheelhouse compatible. Esta propuesta no bloquea las correcciones implementadas.

Los conflictos R-T13, R-T19, R-T24, R-T25, R-T26 y R-T32 esperan una decisión específica por regla. No activar CT-03..10,
historia/PDF ni cambios de motor como consecuencia implícita de esta auditoría.

Las pruebas de fechas seriales ejercitan el lector vigente del motor y emiten una advertencia de deprecación
de NumPy. Se conserva ese código por decisión expresa; revisar compatibilidad al actualizar dependencias,
con paridad y autorización antes de editar motor/. No impide la verificación actual.
