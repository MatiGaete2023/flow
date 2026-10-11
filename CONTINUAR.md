# Continuar — 11 de octubre de 2026

Base remota comprobada: feat/ejecucion-plan-20261008, `286c28f`.
Leer AGENTS.md, PLAN_VIGENTE.md y docs/implementacion/ESTADO.json.

WP-63 (#34) y WP-80 (#35) están integrados con E3 independiente. Línea base: 796/240.
WP-81 (#36): implementación y CI completas; revisión E3 sin registrar, no integrado.
Su informe está en la rama wp/81-revision-propia-20261011.
WP-82 continúa G-4 de WP-80 y evita huellas de adjuntos sin recibos; parte de feat,
independiente de WP-81. Rama wp/82-pendientes-correo-20261011, PR #37 borrador.
Verificación completa PASA en `004dc65` (810/240); controles Windows en el PR.
[Ficha](docs/implementacion/wp/WP-82.md) y [reporte](docs/implementacion/REPORTE_WP82.md).

## Pendientes, en orden

1. Revisar WP-81/WP-82 por otra raíz (E3); integrar los aprobados y regenerar línea base después
   de cada integración. Regenerar ESTADO desde fuentes; conservar ambos paquetes al resolver docs.
2. WP-17: usuario elimina pre-WP-12/pre-implementacion-20261009 y pide limpieza de referencias
   de PR a Soporte GitHub. 607 artefactos antiguos borrados; nunca subir desde clones previos a purga.
3. WP-02/D2: ninguna prueba de uso real aprobada ni acta firmada; resultados agregados locales
   sin datos reales. Probar Excel nativo y Outlook clásico en PC autorizado.
4. WP-50 (#33): contrastar 10 ingresos y esperar WP-02; resolver app_base.py contra feat antes de verificar.
5. WP-70/D7: acta final, métricas y PR #1 hacia main; sin release.

G-3 y resto de G-5, analyze/_recalculate: paquete propio con pruebas y golden.
Firmas retiradas, módulos de bitácoras/excel_presentacion y pagina.js requieren evaluación/decisiones previas.
D3/D9 retirados; D4 60 días/10 sesiones; D5 cuatro meses; D6 solo consolidado; D8 sin fallos observados.
CC fijo, motor/textos intactos y seis xfail R-T estrictos. Actions disponible, presupuesto 5 USD.
Estado fuente en docs/implementacion/estado; tools/estado.py agregar/validar.
[Historial](docs/HISTORIAL.md). La continuación de WP-63 pendiente en notas antiguas quedó superada.
