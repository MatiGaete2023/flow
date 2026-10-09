# Plan vigente — Descargador SITFA y Asistente CSMP

Responsable funcional y usuario único: el usuario titular. El agente de IA programa; no decide reglas.

## Esencia (no se cambia)
- Descargador: lee SITFA con la sesión del usuario; entrega Excel originales, consolidado, bitácoras y ZIP. No escribe en RUS.
- Asistente: lee Excel o ZIP; aplica reglas tradicionales de Laja, Mulchén y Tomé; prepara Excel, nóminas, Word y borradores Outlook (CC ucc_concepcion@pjud.cl). No envía, no registra, no navega.
- Flujo: Trabajo -> Correos -> Resoluciones -> Resultados.

## Prohibiciones absolutas
- Sin escritura RUS/SATURNO, sin envío de correos, sin credenciales.
- Sin datos reales en el repo, en CI ni en conversaciones con agentes: solo estructura anónima (extractor HAR local, diagnostico_bitacoras.json, anonimizar_excel.py).
- Retención local: lotes y sesiones según D4 (propuesta: 30 días; máximo 5 sesiones anteriores).

## Puerta única
Cada versión se acepta con docs/aceptacion/ACTA_USO_REAL_<fecha>.json firmada por el usuario.
No hay otras puertas. Las pruebas automáticas no reemplazan el acta, y el acta no exige operador institucional adicional.

## Prioridades (en orden)
1. Datos sensibles. 2. Simplificar y eliminar. 3. Experiencia de usuario. 4. Nuevas funciones sin cambiar la esencia.

## Decisiones (completar con las respuestas del usuario en ESTADO.json)
- Motor de reglas vigente: personal/motor (N1: el otro no participa en la app).
- Inspector/registro RUS: D3 (pendiente).
- Cronología de bitácoras CT-01/CT-02 «solo mostrar»: D5 (pendiente).
- Horizonte de última observación: D5 (propuesta: 4 meses).
- Historia SITFA/PDF y reglas CT-03..10: fuera de alcance hasta nueva decisión.
- Reglas V4 del documento 2.6.0 (reaplicar tras Envio(), vacíos sin 0): D8 (pendiente).

## Documentos
- Plan de implementación: docs/implementacion/PLAN_IMPLEMENTACION.md
- Estado: docs/implementacion/ESTADO.json
- Referencia histórica: docs/plan_20261008/01–06 (donde difieran, prevalece este archivo).
