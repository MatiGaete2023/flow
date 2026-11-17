# Flow Estudio D — ejecución en curso

Base: cfc12ee2af5e4fb5623671fd17ccf7feb9e1cc11 (coincide con la auditoría).
Rama: feat/flow-estudio-d-20261117. Fecha: 17 de noviembre de 2026.

## Decisiones y contrato

B1 confirmado por el usuario: intervalo capturado con fechas ajustables, límites inclusivos.
B2 autorizado por el usuario: ampliar metadatos conservando compatibilidad.
Advertencia del esquema: exportar.rs::bitacoras escribe origin=CENTRO y
origin_verified=true para todas las entradas. Esos campos legados solos NO bastan
para la herramienta nueva. Conservar autoría comprobada desde lectura original,
fechas inválidas y cobertura por ingreso antes de reducir eventos. No inferir autoría del texto.
Falta acreditar cobertura temporal completa por ingreso antes de conectar conclusiones negativas.
La cronología actual reduce a cuatro meses y no demuestra ausencia; no usarla como adaptador.
Conservar identidad source_system + court_code + cause_id + ingreso_id.

## Estado real

- P0 EN_CURSO: base y documentos revisados; descarga local del repositorio falla por conexión.
  Rust/Cargo no están disponibles en PATH. No existe baseline local de pruebas.
- P1 EN_CURSO: tema compartido índigo/lavanda/menta, cuerpo 15 px, controles 28 px,
  navegación adaptable. Falta verificación visual y paneles ajustables.
- P2 EN_CURSO: recalcular requiere confirmación explícita; copia de observación identificada.
  Distribución preliminar: tabla/evidencia arriba, editor ancho abajo con divisor ajustable.
  Falta restaurar distribución, adaptación a ventana pequeña y paridad de sesión.
- P3 PENDIENTE: solo etiquetas preliminares de correos; falta reorganización.
- P4 EN_CURSO: modos explícitos, quitar individual/todas las entradas,
  etiquetas Meta 4 y selector explícito Todos/Elegir con validación de selección vacía.
  Captura distingue bitácoras de descarga/paquete y etiqueta lote actual/externo.
- P5 EN_CURSO: Importar agrupado, plantillas en menú, destino de próximos productos rotulado.
  Faltan bibliotecas tabulares de Resultados/Contactos y ayuda.
- P6 EN_CURSO: clasificador puro tipado y cinco pruebas de bordes; adaptador y
  evidencia de paquete pendientes. Módulo todavía sin integración con UI/importador.
- P7 PENDIENTE: reutilizar resultados del conjunto completo, sin recalcular agregados filtrados.
- P8 PENDIENTE: vista de bitácoras y exportación XLSX/JSON.
- P9 PENDIENTE: revisión independiente.
- P10 PENDIENTE: candidato, manual, capturas y pruebas Windows.

## Verificación

No afirmar pruebas locales aprobadas. La CI existente de Rust se habilita para esta rama,
sin cambiar sus comandos ni reducir verificaciones. La ejecución remota se registra por SHA.
Primera tanda: job Linux 134143557948 PASÓ, incluidas pruebas del workspace,
contrato de anonimización y captura sintética con extensión. Run 44113050944.
Windows todavía pendiente al registrar este avance. El segundo commit debe validarse aparte.
No se han probado visualmente escalado, Outlook ni captura real.
Sin aceptación, fusión ni release.

## Continuación

Completar adaptador que reutilice abrir() de paquete.rs conservando consultas y eventos
crudos. No agregar campos de captura inventados ni ampliar extensión. Ante falta de
cobertura por ingreso, mantener No verificable. Revisar duplicados contradictorios.
Conectar clasificador únicamente después de fixtures del esquema real.
Implementar P2-P5/P7-P8 y revisión P9 antes de construir el candidato.
