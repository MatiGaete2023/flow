# Continuación técnica — WP-83

11 de octubre de 2026. Base remota `286c28f`, con WP-63/WP-80 integrados.
WP-81/#36 y WP-82/#37 siguen abiertos, con CI aprobada pero sin revisión E3 registrada.
Este paquete continúa G-3 de WP-80 en rama propia desde feat; no integra su propio trabajo.

## Hallazgo y razonamiento

interaction.lock solo recorría app.pages y un conjunto de controles principalmente CustomTkinter.
Los editores manuales, las preferencias y otros diálogos existentes usan Toplevel y controles
Tk/ttk. Durante un worker podían admitir cambios o ejecutar Guardar vista sobre la configuración
que la operación estaba usando. La auditoría anterior clasificó G-3 bajo: no se demostró pérdida
de datos reales. Aquí se reproduce el defecto con ventanas y operaciones ficticias reales del Asistente.

## Implementación

- Recorrer páginas y Toplevel existentes, incluidas ventanas anidadas, sin bloquear la barra Detener.
- Incluir controles editables Tk y ttk; evitar visitas duplicadas.
- En ttk cambiar solo el bit disabled, conservando readonly/selected y los otros bits.
- Al liberar, restaurar disabled previo; en Tk/CustomTkinter restaurar el valor state original.
- Conservar rollback ante error parcial y omitir widgets destruidos; mantener el recálculo de
  acciones de correo existente al desbloquear.

Seis escenarios fallaron antes y pasan con la corrección: editor manual, preferencias con
Guardado durante worker, ventanas Tk/CTk anidadas con estados previos, ventana cerrada y
fallo parcial. Las pruebas verifican el efecto (edición/guardado impedidos y luego permitidos),
no solo una bandera busy. Ninguna prueba retirada ni skip/xfail nuevo.

El bloqueo alcanza controles editables de ventanas existentes al iniciarse la operación. No es
un sistema general para interceptar cualquier ventana creada después ni sustituye los guardas
funcionales/deepcopy de las operaciones. No se cambia la política de cierre de cada ventana; los widgets que ya se destruyeron
se omiten al liberar. No se modifica el comportamiento de cancelación ni las reglas de dominio.

## Verificación

Verificación completa **PASA** en `636af86855df3524c6a0b9fda72819fd0f1cb005`, con árbol limpio y estable antes/después.

- Asistente: 802 aprobadas, 11 omitidas y seis xfail estrictos preexistentes.
- Descargador: 240 aprobadas y 62 subpruebas.
- Nueve controles PASA: golden17, identidades 0, inventario sin bajas, restricciones, lint,
  módulos no alcanzables 0 y enlaces rotos 0. Formato: 275 archivos correctos.
- Seis regresiones GUI PASA con Xvfb. Se reforzó el caso de cierre con un control bloqueado
  realmente destruido; la medición final incluye ese test, con código funcional sin cambios.
- [Evidencia completa](verificacion/WP-83.json). Los commits posteriores solo registran documentación/estado.
- CI Windows: controles por SHA en [PR #38](https://github.com/MatiGaete2023/flow/pull/38).

Estas métricas corresponden a WP-83 sobre feat; no incluyen los paquetes aún no integrados WP-81/WP-82.
Motor/textos intactos; no se acredita Office/SITFA real ni se crean actas.

## Continuación

1. Revisar WP-81/#36, WP-82/#37 y WP-83 por otra raíz E3; integrar aprobados en feat y
   regenerar línea base después de cada integración. Resolver documentos y regenerar ESTADO.
2. G-5 restante: medir lectura de adjuntos con recibos/repintados antes de optimizar; una caché
   por tamaño/fecha puede ocultar cambios de bytes. Duplicación analyze/_recalculate: paquete
   propio, comparar edición humana y paridad golden antes de extraer lógica común.
3. WP-17: etiquetas y referencias de PR sensibles; no usar clones anteriores a purga.
4. WP-02: pruebas reales agregadas y acta del usuario en PC autorizado; Excel/Outlook nativos.
5. WP-50/#33: resolver app_base.py, contrastar 10 ingresos y esperar WP-02 antes de integrar.
6. WP-70: acta final y métricas antes de main; conservar dependencias funcionales confirmadas.

Firmas retiradas/módulos de bitácoras/excel_presentacion y pagina.js mantienen las decisiones
pendientes de WP-80. No se eliminan por parecer duplicados ni se cambian reglas tradicionales.
