# Continuar — 11 de octubre de 2026

Base feat: `286c28f`. WP-63/#34 y WP-80/#35 integrados.
#36–38: CI aprobada; ensayo conjunto en revisión/integracion-36-38-20261011.
No hay revisión E3 de otra raíz; no se ha integrado el ensayo en feat.

WP-84: G-5, hashes de adjuntos compartidos solo durante consultas síncronas de lectura;
identidad de guardado siempre fresca. [Ficha](docs/implementacion/wp/WP-84.md),
[reporte](docs/implementacion/REPORTE_WP84.md). Verificación completa PASA en 98f9cb0: 805/11/6 y 240/62. E3 pendiente; CI en el PR.

WP-17: etiquetas antiguas eliminadas por API y ls-remote sin etiquetas. 607 artefactos
anteriores ya borrados. Referencias de PR/cachés pendientes de Soporte GitHub; no purga completa.

1. E3 independiente; integrar #36–38 y WP-84 aprobados, resolver documentos, generar ESTADO.
2. Verificar y regenerar línea base después de cada integración.
3. Confirmación de Soporte GitHub para WP-17.
4. Excel/Outlook reales y acta WP-02; diez ingresos WP-50; cierre WP-70 tras acta.

Motor/textos/golden17 intactos; seis xfail estrictos. Sin main, release ni force.
No publicar desde clones previos a purga. [Historial](docs/HISTORIAL.md).
