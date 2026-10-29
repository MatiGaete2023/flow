# WP-84 · G-5 y limpieza restante del historial

Estado actual: #36–39 integrados en feat con excepciones E3 expresas del usuario.
G-5 resuelto; verificación posterior PASA 841/240 en c58de60fc9654a10d027765fa7202865f222c456.
Las evidencias y notas anteriores de preparación son históricas.


11 de octubre de 2026. Base feat `286c28f`. Paquete verificado; E3 independiente pendiente.

## Hallazgos y corrección

1. Consulta con 100 borradores y un adjunto compartido: 100 lecturas antes. Compartir
   únicamente hashes de adjuntos durante la consulta síncrona; descartar al finalizar.
2. Tarjetas: leían bytes incluso sin recibos. Omitir huellas en ese caso conservando
   comprobación de existencia y adjuntos obligatorios.
3. Guardar conserva draft_fingerprint fresco, separado de la caché de presentación;
   una edición con igual tamaño/mtime se detecta en la siguiente consulta y al guardar.
4. Dos etiquetas antiguas eliminadas por API; referencias de PR/cachés requieren Soporte GitHub.

## Verificación

Nueve regresiones focales PASA, incluidas dos ejecuciones GUI reales de tarjetas (con/sin recibos).
Resultados: 100 → 1 lecturas por consulta; segunda consulta vuelve a leer. Contexto aislado del
hilo trabajador; errores limpian el contexto; duplicados siguen bloqueados al guardar.
La primera ejecución focal detectó un error de invocación en la prueba GUI, corregido sin cambiar
código de producción ni omitir casos. Verificación completa PASA en `98f9cb0b8dea3cc7fa93c0b6e9449475ecce0a45`: Asistente 805 aprobadas,
11 omitidas y 6 xfail estrictos; Descargador 240 aprobadas y 62 subpruebas. Nueve controles PASA,
procedencia limpia/estable, identidades 0 y golden17 intacto. Windows: ver controles del PR. Sin pruebas retiradas ni nuevas omisiones; sin modificar motor, textos, golden,
CC, Outlook Save-only ni línea base. #36–38 tienen CI aprobada y ensayo conjunto separado;
no se integran ni se aprueban sin E3 independiente.

## Revisión propia de #36–38

- #36: validación del destino antes de medir; cierre de temporales en Windows; inspección de
  nombres de archivo/miembros ZIP y errores de compresión tratados sin contenido sensible.
- #37: conservar incidencias y recibos inciertos; comparar la huella efectiva tras editar;
  omitir lectura sin recibos. pending() es un ayudante legado, no el lector GUI.
- #38: cubrir controles nativos/ttk y Toplevel; restaurar estados readonly/selected/disabled;
  tolerar ventanas cerradas y errores parciales; conservar Detener disponible.
- Ensayo conjunto: `4a095727a005f57b3d4248dbdcb4f8564cde33e8`, 830/11/6 y 240/62,
  nueve controles PASA. Conflictos exclusivamente en CONTINUAR, plan y ESTADO generado;
  fuentes por WP conservadas. Los tres PR siguen abiertos y sin revisión independiente.

Esta revisión propia no acredita E3. El ensayo está en `revision/integracion-36-38-20261011`;
la línea base integrada permanece en 796/240. Revisión externa antes de merge.

## Alcance de G-5

Los lectores efectivos optimizados son check_products (Resultados/preflight) y DraftCards
(Correos). La tabla Trabajo no calcula huellas de adjuntos al pintar filas; el detalle seleccionado
mantiene lectura fresca. No se introduce caché a través de after(), callbacks ni ediciones.

## Historial comprobado

DELETE de ambas etiquetas exitoso; ls-remote sin etiquetas. Clon nuevo de las ramas:
check_real_ids --historia = 0. Los 96 artefactos actuales corresponden a 19 commits alcanzables
en las ramas saneadas; ninguno quedó fuera de esa comprobación de alcanzabilidad. Esto no
certifica el contenido de cachés ni referencias antiguas de PR. Soporte GitHub sigue pendiente.
Los 607 artefactos antiguos habían sido eliminados por la sesión previa.

## Continuación

E3 de #36–38 y WP-84; integrar aprobados con nueva medición/línea base tras cada integración.
Soporte GitHub: confirmar retiro de referencias y cachés antiguas. WP-02/WP-50/WP-70
siguen sujetos a las pruebas reales y al acta del usuario.

## Ensayo conjunto con G-5

PASA en `eae6b499b16f41960aa570538d857d335cdef9ec`: 839/11/6 y 240/62, nueve
controles y procedencia limpia/estable. La resolución posterior de documentos no modifica
código, pruebas ni configuración. Los cuatro paquetes siguen EN_REVISION, sin E3.

## Verificación después de integrar #36–38

PASA en `9518d9bcc667b915dd46c751561bf740fdceafd5`, contra la línea base integrada
`4470ff32bb9d479f0ad4a6774948038bc6729c02` (830/240). Resultado 839/11/6 y 240/62;
nueve controles y procedencia limpia/estable. Commit posterior: solo documentos/evidencia.
La rama conserva estados de integración, excepción E3 limitada a #36–38 y dependencia
de archivo WP-82 explícita. CI actual en el PR #39; las corridas de preparación son históricas.
No se integra #39 sin revisión E3 propia. No cambios de motor, textos, golden ni línea base.

## Revisión final y corrección de avisos

La ruta sin recibos omitía la validación de direcciones que antes ocurría al calcular
la huella: una tarjeta con destinatario o CC inválido aparecía como Preparado. Dos pruebas
GUI adicionales fallaron antes de corregir; se valida con emails sin leer adjuntos.
Once regresiones focales PASA; se conservan las nueve originales y la reducción 100→1.
Usuario autoriza excepción E3 para #39: «Haz la excepcion y termina tu». No se autoriza main
ni se sustituye el acta real. Verificación completa posterior a integración en curso.

## Cierre técnico — 11 de octubre de 2026

Integración de #39 en c58de60fc9654a10d027765fa7202865f222c456, con excepción E3 explícita del usuario («Haz la excepcion y termina tu»).
Verificación posterior PASA contra base 830/240: 841 aprobadas, 11 omitidas y 6 xfail estrictos en
Asistente; 240 aprobadas y 62 subpruebas en Descargador. Nueve controles PASA, identidad Git
limpia/estable y línea base regenerada en este SHA. Once regresiones focales; dos correcciones GUI
adicionales conservan los avisos sin recibos y sin lectura de bytes. Ninguna prueba retirada.

Código, pruebas y configuración permanecen iguales en el commit posterior de documentos.
Controles Windows, ejecutable e instalador: ver resultados definitivos en el PR #39.
Sin revisión ni auditoría de otra raíz; no presentar esta excepción como aprobación independiente.

Etiquetas antiguas borradas en remoto y copia de trabajo. Checker actualizado --historia = 0
sobre ramas del clon nuevo y copia de trabajo; se conserva pre-WP-63 saneado. Refs de PR/cachés
requieren Soporte GitHub. No purga completa ni aceptación de uso real.

Pendientes del plan general: Soporte GitHub (WP-17), pruebas reales Excel/Outlook y acta (WP-02),
diez ingresos de cronología (WP-50), cierre final WP-70 tras acta. Motor/textos/golden17 intactos.
