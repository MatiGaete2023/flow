# Plan vigente — Descargador SITFA y Asistente CSMP

Responsable funcional y usuario único: Matías Gaete. El agente de IA programa; no decide reglas.

## Esencia (no se cambia)
- Descargador: lee SITFA con la sesión del usuario; entrega Excel originales, consolidado, bitácoras y ZIP. No escribe en RUS.
- Asistente: lee Excel o ZIP; aplica reglas tradicionales de Laja, Mulchén y Tomé; prepara Excel, nóminas, Word y borradores Outlook (CC ucc_concepcion@pjud.cl). No envía, no registra, no navega.
- Flujo: Trabajo -> Correos -> Resoluciones -> Resultados.

## Prohibiciones absolutas
- Sin escritura RUS/SATURNO, sin envío de correos, sin credenciales.
- Sin datos reales en el repo, en CI ni en conversaciones con agentes: solo estructura anónima (extractor HAR local, diagnostico_bitacoras.json, anonimizar_excel.py).
- Retención local: lotes y sesiones según D4 (pendiente; propuesta de la auditoría: 30 días y 5 sesiones anteriores).

## Puerta única
Cada versión se acepta con docs/aceptacion/ACTA_USO_REAL_<fecha>.json firmada por el usuario.
No hay otras puertas. Las pruebas automáticas no reemplazan el acta, y el acta no exige operador institucional adicional.

## Prioridades (en orden)
1. Datos sensibles. 2. Simplificar y eliminar. 3. Experiencia de usuario. 4. Nuevas funciones sin cambiar la esencia.

## Decisiones del usuario (estado en docs/implementacion/ESTADO.json → decisiones)
- Motor de reglas vigente: personal/motor (N1: el otro no participa en la app).
- D1 purga de historial: pendiente.
- D2 casos probados en uso real: pendiente.
- D3 retiro del inspector y registro RUS: pendiente.
- D4 retención local (días y sesiones anteriores): pendiente.
- D5 cronología CT-01/CT-02 «solo mostrar» y horizonte: pendiente (propuesta: 4 meses).
- D6 XLSX consolidado dentro del ZIP: pendiente.
- D7 fusión a main al cierre: pendiente.
- D8 reglas V4 del documento 2.6.0 (reaplicar tras Envio(), vacíos sin 0): pendiente.
- D9 solicitud de antecedentes (precargar o retirar): pendiente.
- Historia SITFA/PDF y reglas CT-03..10: fuera de alcance hasta nueva decisión.

## Documentos
- Plan de implementación: docs/implementacion/PLAN_IMPLEMENTACION.md
- Estado: docs/implementacion/ESTADO.json
- Traspaso entre sesiones: docs/implementacion/TRASPASO.md
- Referencia histórica: docs/plan_20261008/01–06 y docs/HISTORIAL.md (donde difieran, prevalece este archivo).
