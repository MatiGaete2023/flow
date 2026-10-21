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
- q) Reglas de historia del Asistente (doc «Reglas del Asistente», 28 reglas): implementadas en `claude/hopeful-hamilton-k2tcqp` (WP-71..WP-79, plan WP-R1..R9), en revisión E3. Textos en el scope `HISTORIA` de `textos_base.json` (34 claves); las redacciones (P) y `P_FINALIZADA` quedaron aprobadas (ver s y t), el resto sigue `confirmado:false`. Cambios deliberados a reglas originales: `espera_dce=0`, `espera_tome=60`, «Audiencia decretada», mayoría de edad sola. Nada está aceptado.
- r) Regla `HISTORIA.P_FINALIZADA` (causa P con Conciliación o Dictación de sentencia sin Egreso de servicio protección especializada y otras redes): constancia de causa finalizada en Espera, Cumplimiento e Informes. Fuente: usuario, 14 de octubre de 2026. Marca de mayoría de edad: solo por la edad del NNA; la mayoría de edad va sobre las observaciones nuevas.
- s) Aprobadas las redacciones (P) del plan de historia (`confirmado:true` en `textos_base.json`). Fuente: usuario, 14 de octubre de 2026: «Apruebo las redacciones».
- t) Aprobado el texto de `HISTORIA.P_FINALIZADA` y confirmado el nombre del egreso («Egreso de servicio protección especializada y otras redes», sin abreviar). Fuente: usuario, 14 de octubre de 2026: «Apruebo P_FINALIZADA, ese es el nombre del egreso, no esta abreviado».
- u) Decisiones sobre la revisión E3 (`docs/implementacion/revision/E3_REGLAS_HISTORIA_20261014.md`). Fuente: usuario, 14 de octubre de 2026: «1. A. 2. B. 3. A. 4. B.».
  1. E10, T5, J2 y R6 son reglas RUS: salen siempre, con Excel o paquete y haya o no historia (cambio deliberado; golden `reglas_rus`).
  2. Informes: un «Ingreso informe de cumplimiento X» solo desmiente el vencimiento si es del día del vencimiento o posterior; la «Prorroga» nunca lo desmiente, queda solo como constancia y se mantienen las acciones.
  3. Regla 6 (duplicidad) es por NNA (RUT): hermanos en la misma causa y programa no son duplicidad; sin RUT no se afirma.
  4. Regla 3: solo cuenta el «Ingreso efectivo» desde el inicio de la espera (corte menos T ESPERA); sin T ESPERA válido no se filtra.

- x) Segunda ronda de reglas de historia, a partir del cruce FAE ficticio de Cumplimiento. Fuente: usuario, 15 de octubre de 2026 (respuestas F-1 a F-6, R-1 a R-6, A a F, 3C, 5C, constancias genéricas y armonización 1–8). Detalle y casos: `docs/implementacion/revision/ANTES_DESPUES_DECISION_X_20261015.md`.
  1. Un dato se dice una vez: si varias reglas llegan a la misma conclusión, sale una sola (la fecha de vencimiento de la medida, el egreso proyectado, los días de espera).
  2. Orden transversal a Espera, Cumplimiento e Informes: constancias, luego acciones del Centro (correos y proyectos de resolución) y al final la gestión del Tribunal (asociar curador, eliminar o verificar designaciones), con la aclaración de causa como cierre. Textos aprobados que mezclaban partes se dividen en dos.
  3. Redacción: «de que» en todos los textos (incluido `P_FINALIZADA`), REITERA abreviada, nomenclatura literal de SITFA («Prorroga», «Ingreso Informe de Cumplimiento X»).
  4. Prórroga (Cumplimiento): «Se hace presente que a folio N, de fecha F, se prorroga la medida hasta el {egreso proyectado}». Con la medida vencida, por vencer, o con el egreso de RUS anterior a la prórroga, no se afirma (prevalece el texto de la medida) y se marca prioritaria si la prórroga tiene hasta `prorroga_reciente` días (90). La regla «No consta Prorroga» no se aplica nunca (se elimina también su marca de nivel 3). Si la bitácora ya mencionó la prórroga, se ignoran frase y marca; el funcionario deja su propia nota. En Informes, Prorroga no dice nada.
  5. Glosario: «Ingreso» es la resolución con que el Tribunal ordena el ingreso (la causa queda en Espera); «Ingreso efectivo» es el trámite con que el programa informa la intervención (pasa a Cumplimiento). «Ing.Serv. protección especializada y otras redes» es la resolución de ingreso (solo Espera): «Tribunal dispone ingreso a {programa}, en resolución de folio N, de fecha F».
  6. Ingreso efectivo: aparece solo en Cumplimiento (un trámite dentro de `ventana_ingreso` días, 30, desde la fecha de RUS confirma la nomenclatura: «La fecha de ingreso efectivo informada al Tribunal corresponde al…»). En Espera rigen las reglas normales y solo se marca nivel 2.
  7. Constancias genéricas: la observación hace constar lo resuelto al aplicar la nomenclatura. Resp. Diagnóstico clínico especializado, Rectificación DCE, Derivación Dirección Reg., Pide cuenta Derivación D.R. y Resp. Derivación dir. regional: sin marca ni reglas. SPE. Resp. Solicitud DCE (escrito): solo Cumplimiento, DCE con medida vencida («Informe clínico diagnóstico ordenado en autos, se encuentra en la causa a folio N, de fecha F, a la espera de lo que se resuelva en la audiencia respectiva»). Resp. continuidad de atención: «consta respuesta sobre la continuidad de atención». Deja sin efecto derivación D.R. y Deja sin efecto medida interconexión M.N.: solo marca nivel 2, sin texto. Búsqueda sin cierre y términos huérfanos: texto neutro «Se hace presente que a folio N, de fecha F, consta el trámite «X»». Ingreso Informe de Cumplimiento X (Informes): «Se hace presente que a folio N, de fecha F, se ingresa el informe de cumplimiento».
  8. Curador: la baja o la designación de la historia se dice solo si RUS no tiene curador, con marca de nivel 2; con curador en RUS se ignora. La aclaración «La historia es de la causa…» es transversal (una vez, al final) cuando la causa tiene varios niños en la nómina.
  9. Fichas: sin texto de la historia; rigen solo las reglas originales (FAE: existencia tras 120 días; individual: sin ficha, más de 180 días y reciente de 0 a 30). Informes solo hace constar lo de informes (vencido, por vencer, ingresado, acumulados J2) más correo y proyecto; la advertencia de posible entrega anterior pasa a marca nivel 2. Causa finalizada se mantiene en las tres etapas. Si el egreso proyectado es posterior a la mayoría de edad, se dice. E10 reemplaza a los días de espera cuando ambas dicen lo mismo.
  10. Umbrales configurables (`UMBRALES`): `prorroga_reciente` = 90, `ventana_ingreso` = 30, `lista_espera_larga` = 100.
  11. `C10_ESCRITO` aprobado por el usuario el 15 de octubre de 2026; Asistente `0.5.0.dev6`.

- y) Ronda del usuario, 16 de octubre de 2026 (detalle en `docs/implementacion/wp/WP-66.md`).
  1. La columna NOMBRE no lleva paréntesis ni lo escrito en ellos (Descargador y Asistente).
  2. Toda constancia de la historia sigue el formato «en resolución de fecha F (folio N)». Una segunda constancia «Se hace presente que» se enlaza con «Asimismo».
  3. Un dato que varias reglas dicen se escribe una sola vez.
  4. Órdenes de búsqueda: primero la orden, luego sus trámites; solo los últimos 2 meses; nada si la bitácora ya lo observó.
  5. Si la observación remite un proyecto y el motor identifica su tipo, se marca en RES (PC_IE, PC_INFO, NOMENCL).
  6. El paquete incluye el calendario de informes por vencer (60 días); Cumplimiento observa los informes que vencen dentro de ese plazo y alimenta el correo preventivo.
  7. El Descargador deja elegir el período a recolectar (1 a 4 meses) y entregar un paquete por hoja de RUS.
  8. Aprobados todos los textos pendientes (16-oct); búsqueda con «Posteriormente»/«Luego»; T5 con «Atendido que». Asistente `0.5.0.dev8`.
  9. Tras la auditoría E3 propia (`docs/implementacion/revision/E3_WP65_69_20261016.md`): el Descargador cambia SITFA al
     calendario de informes y de vuelta a Seguimiento sin aviso; la ventana de 2 meses de las órdenes de búsqueda se mide
     por el último trámite; E-04 con E-05 se deja como está; el caso límite de nombres solo distintos entre paréntesis no se
     corrige. Asistente `0.5.0.dev10`, reglas `historia-7`.
  10. Limpieza de datos privados en el repositorio (WP-56): sin nombres reales en metadatos de plantillas, documentos ni
     capturas del manual; fixtures de anonimización con nombres, RUT y RIT ficticios.
  11. Usuario (16-oct): «Autorizo purga de historial, elimina también [la marca de redacción]». Se retira la marca de las plantillas de
     Laja y se reescribe el historial de todas las ramas (WP-17, segunda purga).

## Fase B de auditoría final — 14 de octubre de 2026

- v) Usuario: «Pasa a la fase B», tras el informe de auditoría de esta sesión. Autoriza correcciones localizadas de
  WP-64 y sus regresiones. No autoriza aceptación, integración de cambios riesgosos sin E3, fusión a main ni release.
- w) Informes ingresados antes del vencimiento sin período acreditado: «Conservar acciones y advertir la posible
  entrega (recomendado)». Se conserva I-01 y se agrega advertencia de revisión; sin ventana de días inventada ni
  modificación de textos institucionales. El mismo límite inferior aprobado para I1 se aplica a I3 al desmentir vencimiento.
- Corrección FAE autorizada: una columna no capturada es NO_EVALUABLE, nunca prueba de inexistencia de ficha.
- Identidad y tiempo: fuentes conservadas, uso sustantivo de historia solo con vínculo VERIFICADA y hasta el corte.

Rama revisable: `wp/WP-64-auditoria-fase-b`, sobre candidato `4f572ed`; feat sigue en `1a2dd0f`.

## Decisiones del 21 de octubre de 2026

- z) Una sola versión vigente: `claude/lucid-sagan-hw9ul4` (dev10, Descargador 2.8.0) y `claude/exciting-gates-5xq24d`
  (WA-1 a WA-7, CT-05/07/11, WD-1) se integran combinando ambas y priorizando lucid-sagan donde se contradicen.
  Fuente: «Combina ambas en A1–A4, prioriza lucid-sagan». A1: «Audiencia decretada» / «Audiencias decretadas». A2:
  mayoría de edad sola, con marca. A3: historia + audiencias múltiples en Espera y Cumplimiento; Informes sin audiencia
  y con margen DCE de 10 días. A4: «Informe diagnóstico vencido desde el …». Las casillas del Descargador «Leer
  calendario de informes» e «Incluir informes por vencer (60 días)» son distintas y se conservan («dejalos como
  estaban»). La auditoría de pruebas WP-65 de la línea feat pasa a WP-58.
- aa) Ramas con nombre claro: `ultima-version/asistente-descargador` y `ultima-version/anonimizador`. Fuente: «Cambia el
  nombre de las ramas con las últimas versiones de cada cosa».
- ab) E3 pendiente hecha por la sesión `01UiipQ3fZdQ8wDJSoFcza3h` y dada por cumplida; integrar a las ramas pertinentes.
  Fuente: «Los elementos que todavía no han pasado la revisión E3, revísalos tú; si pasan, da por cumplida la revisión e
  intégralos». Informe y salvedad de independencia (la resolución de la fusión es de la misma sesión):
  `docs/implementacion/revision/E3_INTEGRACION_20261021.md`. No autoriza aceptación institucional, fusión a `main` ni
  release.
