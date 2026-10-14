# Plan vigente — Descargador SITFA y Asistente CSMP

Responsable funcional y usuario único: el usuario titular. El agente de IA programa; no decide reglas.

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
- D1 purga de historial: historial reescrito el 11-oct (35 ramas, force-with-lease, autorizado); 607 artefactos y 2 etiquetas remotas borradas; pendiente: refs/pull y cachés vía Soporte GitHub (WP-17).
- D2 casos probados en uso real: «Ninguna todavía»; WP-02 espera resultados del usuario.
- D3 retiro del inspector y diario de registro RUS: «Sí, retirarlos».
- D4 retención local: «60 días y hasta 10 sesiones anteriores».
- D5 cronología CT-01/CT-02: «Sí, horizonte de 4 meses»; código de #33 en feat desde 8c4036d y visible por decisión del usuario (g); aceptación pendiente del acta de 10 ingresos y WP-02.
- D6 XLSX consolidado dentro del ZIP: «Sí, solo el consolidado»; implementado con casilla en el Descargador.
- D7 fusión a main: «Sí, tras el acta final»; no se fusiona ni publica una release antes del acta.
- D8 consultas 2.7.5: «No observé esos fallos»; se descarta la variante reaplicar tras Envio() de WP-33.
- D9 solicitud de antecedentes: «Retirar botón»; retirado.
- Decisiones adicionales del 11 de octubre: quitar apóstrofo visible preservando texto seguro; conservar cc_obligatorio; motor intacto.
- PDF de trámites y reglas CT-03..10: fuera de alcance. Historia de la causa (solo tabla, 4 meses): decisión o, WP-52..WP-54.

## Decisiones del 11 de octubre de 2026
Registradas también en docs/implementacion/estado/global.json → decisiones_adicionales.decisiones_20261011.
- a) «Fusiona todas las ramas»: consolidación WP-85.
- b) WP-63 se integra antes que WP-80.
- c) WP-70 depende funcionalmente de WP-02, WP-17, WP-50 y WP-63.
- d) WP-80 se integra por squash con mensaje formateado (regla 11).
- e) WP-80 sin etiqueta pre-WP-80; rollback con `git revert 5ac7385`.
- f) Excepciones E3 autorizadas para los PR #4/#8/#11/#12–#32, #27 y #36–#39.
- g) La cronología WP-50 queda visible en feat mientras falta el acta de 10 ingresos; la aceptación sigue pendiente.
- h) La plantilla del acta deja vacío el tipo de evidencia; lo elige el usuario.
- i) WP-86 correctivo autorizado («Sí, todo»), con revisión E3 antes de integrar.

## Decisiones del 12 de octubre de 2026
- j) E3 solo para cambios riesgosos: diffs que tocan motor/, textos_base.json, Outlook/borradores, contrato de captura (contracts/, src/nurus/capture/contract.py), borrado o retención de datos, check_real_ids/anonimizador, extensión SITFA. El resto se integra con `verificar_wp.py` PASA, sin
  sesión revisora aparte. Fuente: «Sí, para los cambios riesgosos» (respuesta a EFICIENCIA_PROCESO_20261012.md).
- k) Actualizar los planes y comenzar la ola de eficiencia y UX, conservando los dos botones de preparación y la
  ubicación de los productos secundarios. Fuente: «Actualiza los planes en el repositorio y comienza a implementar».
- l) Completar los cambios anunciados que no estuvieran integrados y continuar con UX; motor, Outlook, contratos,
  retención y extensión intactos. Fuente: «Efectua los cambuos del otro agente que no esten y continua».

## Decisiones del 13 de octubre de 2026
- n) Consolidar todo en una sola rama (`feat/ejecucion-plan-20261008`) y trabajar desde ahí. Fuente: «consolida todo el trabajo en una sola rama, trabajaremnos y solucionaremos problemas desde aca».
- m) Subir `MAX_LECTURAS_POR_LOTE` de 300 a 600: con 344 registros habituales cada corrida exigía «Retomar». La pausa entre lecturas y los checkpoints no cambian. Fuente: «Sube el limite a 600».
- o) Historia de la causa en el Descargador: solo la tabla de la pestaña Historia de la ventana «Información de la Causa» (folio, etapa, trámite, descripción, referencia, fecha y presencia de anexo), solo ventana de 4 meses, sin PDF, uso personal en Windows; el Asistente la analizará en otro paso. Rama `feat/historia-causa-sitfa`; guía `docs/historia_causa/GUIA_IMPLEMENTACION.md`. Fuente: mensajes del usuario del 13-oct («no deseo descargar los pdf», «solo en la ventana de 4 meses», «solo de uso personal en windows»).
- p) Aceptadas las recomendaciones P1–P8 de `docs/historia_causa/00_DECISIONES.md`. Fuente: «Sí a todo lo que me dices».

## Ola de eficiencia y experiencia — 12 de octubre de 2026

Usuario: «Actualiza los planes en el repositorio y comienza a implementar».
- [Plan de la ola](docs/implementacion/PLAN_EFICIENCIA_UX_20261012.md): fuente de alcance y orden.
- WP-87 integró el tema compartido. WP-88 completa los cambios anunciados ausentes y continúa UX-02/03, por instrucción expresa del usuario. Estado vigente en las fichas JSON.
- Conservar separados «Preparar tipo»/«Preparar todos» y conservar la ubicación de ZIP, resumen, ICS y recuperación legado.
- Outlook mantiene persistencia antes/después de cada efecto y el estado incierto. Una caché de presentación no autoriza un guardado externo.
- Una medición completa por versión candidata; pruebas focales durante desarrollo. Estado estructurado y línea base son fuente de resultados vigentes; los informes históricos conservan su fecha/SHA.
- WP-89 implementa CI Linux automática, Windows mínimo en PR de código a feat y construcción única bajo demanda; E4 revisada en el plan general. Mantener verificación Windows completa de entrega candidata. Facturación confirmada mediante jobs ejecutados. Windows completo e instalador PASA en run 38259239456; evidencia y límites en WP-89.

## Documentos
- Plan de implementación: docs/implementacion/PLAN_IMPLEMENTACION.md
- Estado: docs/implementacion/ESTADO.json
- Continuación: CONTINUAR.md; informe de recuperación: docs/implementacion/wp/WP-01.md
- Referencia histórica: docs/plan_20261008/01–06 y docs/HISTORIAL.md (donde difieran, prevalece este archivo).

## Decisiones del 14 de octubre de 2026
- q) Reglas de historia del Asistente (doc «Reglas del Asistente», 28 reglas): implementadas en `claude/hopeful-hamilton-k2tcqp` (WP-71..WP-79, plan WP-R1..R9), en revisión E3. Textos en el scope `HISTORIA` de `textos_base.json` con `confirmado:false`; las redacciones (P) (`R3_MULTI`, `MATERIA_*`, `C1_RUS`, `C3_CON_FOLIO`, `EGRESO_PROYECTADO`, `C4_*`, `C8_NO_USO`, `I1_*`, `T2_MN`) esperan aprobación del usuario. Cambios deliberados a reglas originales: `espera_dce=0`, `espera_tome=60`, «Audiencia decretada», mayoría de edad sola. Nada está aceptado.
