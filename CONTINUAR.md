# Continuar — 11 de octubre de 2026

Repositorio: MatiGaete2023/flow. Leer AGENTS.md, PLAN_VIGENTE.md y ESTADO.json.
Base comprobada: `feat/ejecucion-plan-20261008`, `286c28f`.

## Punto exacto

#4, #8, #11, #12–#32 y WP-61 están integrados. WP-63 (#34) y WP-80 (#35)
también están integrados, con E3 independiente registrado en sus fichas de estado.
WP-80 se integró con squash `5ac7385`; la línea base se regeneró después:
Asistente 796 aprobadas, Descargador 240. Actions funciona; presupuesto 5 USD.
No volver a tratar #34/#35 como pendientes ni reemplazar sus decisiones por notas antiguas.

WP-81 continúa la revisión propia pedida por el usuario desde esa base, en
`wp/81-revision-propia-20261011`. Corrige cuatro fallos adicionales de herramientas,
incluido O1 de WP-63, y ordena el traspaso. Su ficha registra la verificación y el PR.
Reporte: [REPORTE_WP81.md](docs/implementacion/REPORTE_WP81.md).
La revisión propia no sustituye la revisión/auditoría E3 independiente de este paquete.

## Continuación, en orden

1. Verificar y revisar WP-81 por otra raíz; integrar en feat y regenerar la línea base solo después.
2. WP-17: quedan dos etiquetas antiguas (`pre-WP-12`, `pre-implementacion-20261009`)
   y referencias de PR sensibles. El usuario elimina las etiquetas y pide limpieza a Soporte
   de GitHub. Ya se borraron 607 artefactos de Actions. Nunca subir desde clones anteriores a la purga.
3. WP-02/D2: no hay casos reales aprobados ni acta firmada. Recibir resultados agregados
   del PC autorizado, sin archivos reales. Probar también Excel nativo y Outlook clásico.
4. WP-50: PR #33 borrador preparado; contrastar 10 ingresos, esperar WP-02 y resolver
   el solapamiento en app_base.py contra la integración actual antes de verificar e integrar.
5. WP-70/D7: acta final del usuario y métricas actualizadas; después PR #1 hacia main,
   sin release. Conserva dependencias funcionales confirmadas sobre WP-02/17/50/63.

## Mejoras separadas

G-3/G-4/G-5 de WP-80 y duplicación `_recalculate`/`analyze`: paquete propio con pruebas
que reproduzcan el fallo y paridad golden. Firmas retiradas en carga.py, módulos de bitácoras,
excel_presentacion y controles de pagina.js: evaluar alcance y decisiones pendientes antes de eliminar.
O1 queda corregido en WP-81; su integración sigue pendiente.

D3: inspector/diario RUS retirados. D4: 60 días y 10 sesiones anteriores. D5: cuatro meses.
D6: solo consolidado en ZIP. D8: sin fallos observados. D9: botón retirado.
CC obligatorio conservado; motor/textos intactos, seis R-T siguen xfail estricto.
Estado fuente en docs/implementacion/estado; generar ESTADO.json con tools/estado.py agregar.
Historial anterior: [docs/HISTORIAL.md](docs/HISTORIAL.md).
