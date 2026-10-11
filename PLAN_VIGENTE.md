# Plan vigente — Descargador SITFA y Asistente CSMP

Responsable funcional y usuario único: Matías Gaete. El agente de IA programa; no decide reglas.

## Esencia (no se cambia)
- Descargador: lee SITFA con la sesión del usuario; entrega Excel originales, consolidado, bitácoras y ZIP. No escribe en RUS.
- Asistente: lee Excel o ZIP; aplica reglas tradicionales de Laja, Mulchén y Tomé; prepara Excel, nóminas, Word y borradores Outlook (CC ucc_concepcion@pjud.cl). No envía, no registra, no navega.
- Flujo: Trabajo -> Correos -> Resoluciones -> Resultados.

## Prohibiciones absolutas
- Sin escritura RUS/SATURNO, sin envío de correos, sin credenciales.
- Sin datos reales en el repo, en CI ni en conversaciones con agentes: solo estructura anónima (extractor HAR local, diagnostico_bitacoras.json, anonimizar_excel.py).
- Retención local: 60 días para lotes y sesiones; hasta 10 sesiones anteriores (D4).

## Puerta única
Cada versión se acepta con docs/aceptacion/ACTA_USO_REAL_<fecha>.json firmada por el usuario.
No hay otras puertas. Las pruebas automáticas no reemplazan el acta, y el acta no exige operador institucional adicional.

## Prioridades (en orden)
1. Datos sensibles. 2. Simplificar y eliminar. 3. Experiencia de usuario. 4. Nuevas funciones sin cambiar la esencia.

## Decisiones del usuario (estado en docs/implementacion/ESTADO.json → decisiones)
- Motor de reglas vigente: personal/motor (N1: el otro no participa en la app).
- D1 purga de historial: ramas reescritas el 11 de octubre; etiquetas antiguas eliminadas; faltan referencias de PR y cachés mediante Soporte GitHub (WP-17).
- D2 casos probados en uso real: «Ninguna todavía»; WP-02 espera resultados del usuario.
- D3 retiro del inspector y diario de registro RUS: «Sí, retirarlos».
- D4 retención local: «60 días y hasta 10 sesiones anteriores».
- D5 cronología CT-01/CT-02: «Sí, horizonte de 4 meses»; código de #33 consolidado por orden de fusionar todas las ramas; aceptación pendiente del acta de 10 ingresos y WP-02.
- D6 XLSX consolidado dentro del ZIP: «Sí, solo el consolidado»; implementado con casilla en el Descargador.
- D7 fusión a main: «Sí, tras el acta final»; no se fusiona ni publica una release antes del acta.
- D8 consultas 2.7.5: «No observé esos fallos»; se descarta la variante reaplicar tras Envio() de WP-33.
- D9 solicitud de antecedentes: «Retirar botón»; retirado.
- Decisiones adicionales del 11 de octubre: quitar apóstrofo visible preservando texto seguro; conservar cc_obligatorio; motor intacto.
- Historia SITFA/PDF y reglas CT-03..10: fuera de alcance hasta nueva decisión.

## Documentos
- Plan de implementación: docs/implementacion/PLAN_IMPLEMENTACION.md
- Estado: docs/implementacion/ESTADO.json
- Continuación: CONTINUAR.md; informe de recuperación: docs/implementacion/wp/WP-01.md
- Referencia histórica: docs/plan_20261008/01–06 y docs/HISTORIAL.md (donde difieran, prevalece este archivo).
