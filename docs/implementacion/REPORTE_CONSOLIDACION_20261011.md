# Reporte de consolidación — 11 de octubre de 2026

## Resultado consolidado

Rama canónica: **feat/ejecucion-plan-20261008**. Todas las ramas inventariadas tienen
sus SHA incorporados como ancestros. Main conserva su estado previo conforme a D7.


45/45 ramas remotas incorporadas al historial de feat/ejecucion-plan-20261008.
Inventario completo con SHA en ramas_consolidadas_20261011.json. Se conserva una rama
canónica de trabajo; las referencias antiguas permanecen como respaldo, no como versiones activas.
Usuario ordenó fusionar todas las ramas. No se fusiona a main ni se declara aceptación real.

## Hallazgos y resolución

1. 37 ramas ya eran ancestros. Ocho encabezados aún no lo eran: seis ramas históricas
   con cambios ya aplicados y dos pendientes (WP-02 y WP-50).
2. Se incorporó la historia de las seis ramas antiguas conservando el árbol vigente:
   evita restaurar versiones antiguas del verificador, correos, bloqueo de editores y línea base.
   Los informes E3 de WP-63/WP-80 se recuperaron. No se recuperaron logs o código retirados.
3. WP-02 aportaba una plantilla con 40 registros NO_EJECUTADO; se recupera como plantilla,
   sin firma, resultados ni aceptación inventados. Su estado sigue BLOQUEADO_USUARIO.
4. WP-50 incorpora columna «Última obs. centro» y panel de novedades de cuatro meses,
   solo mostrar. Catálogo CT-01/CT-02 con enabled=false; CT-03..10 y motor intactos.
   La fusión automática conserva los arreglos de reimportación y notas humanas de WP-80.
   El código queda consolidado; aceptación de diez ingresos y WP-02 sigue pendiente.
5. Conflictos únicamente de CONTINUAR y ESTADO generado: se conserva el traspaso vigente
   y se regenera la vista desde fuentes. Decisiones D1–D9 y dependencias WP-70 no se alteran.

## Verificación

29 focales PASA: cronología, interfaz de captura/Trabajo, G-5 y bloqueo de ventanas.
45/45 encabezados son ancestros; motor/textos y línea base previa sin cambios en preparación.
Verificación completa PASA en `4dc54e9b52bbca441ca4d43cd577e5c1cbcda288`, contra base 841/240: Asistente 848 aprobadas,
11 omitidas y 6 xfail estrictos; Descargador 240 aprobadas y 62 subpruebas. Nueve controles
PASA y procedencia limpia/estable; línea base regenerada. Historia Git: 0 hallazgos de RUT.
Ninguna prueba retirada ni nueva omisión.
La orden de consolidar es expresa; no hay revisión de otra raíz ni acta funcional.

## Pasos pendientes

1. Resultado publicado en la rama canónica; controles CI finales en PR #33.
2. Soporte GitHub retira referencias antiguas de PR/cachés (WP-17).
3. Pruebas reales de Excel/Outlook y diez ingresos; completar actas WP-02/WP-50 con resultados
   agregados, sin datos reales. Luego acta final WP-70 y main según D7.

## Recomendaciones

Usar únicamente feat/ejecucion-plan-20261008 para continuar. No desarrollar sobre ramas históricas.
Antes de eliminar referencias de respaldo, usar el inventario de SHA; el historial ya está conservado
íntegramente en la rama canónica. No repetir las correcciones ni reducir el umbral de pruebas.

## Límites y trazabilidad

Las referencias históricas no se borraron: el usuario pidió fusionar/consolidar, y sus
SHA/archivos están preservados en el historial. No hay cambios pendientes exclusivos
de las ramas inventariadas. No se declara revisión independiente ni aceptación real.
El commit posterior solo modifica documentación/evidencia; el código coincide con el SHA medido.
CI final, ejecutable e instalador: ver resultado observado en PR #33.

El inventario excluye el alias simbólico origin/HEAD: no es una rama adicional.
El conteo verificado de ramas reales es 45/45.
