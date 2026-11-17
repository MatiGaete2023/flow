# Flow Estudio D · estado verificable

Base: cfc12ee2af5e4fb5623671fd17ccf7feb9e1cc11. Rama feat/flow-estudio-d-20261117.
PR borrador: https://github.com/MatiGaete2023/flow/pull/59. Fecha: 17/11/2026.
La punta exacta del binario se identifica en CANDIDATO.json de cada entrega.
Aceptación del usuario pendiente. Sin merge, release ni efectos en sesiones reales.

## Decisiones cerradas

B1: intervalo capturado con fechas ajustables y límites inclusivos.
B2: metadatos aditivos con compatibilidad; sin cambios a permisos de la extensión.
La lectura comprueba formulario/ingreso y tabla de autoría antes de acreditar centro.
Los rótulos generalizados del CSV legado no prueban autoría. Sin evidencia suficiente,
No verificable. La revisión no inicia capturas. Una captura explícita no reutiliza
bitácoras legadas sin evidencia V1; preserva el archivo anterior.

## Paquetes implementados y verificación

| Paquete | Implementación | Evidencia |
|---|---|---|
| P0 | Base fijada, instrucciones y contratos leídos | Verificador original ejecutado contra candidato y base; hallazgos heredados idénticos, detalle en qa/CONTROLES_D.md |
| P1 | Tema D común, cuerpo15/controles28, navegación y panel activo compactos, restaurar distribución | Contraste mínimo medido4.8007; capturas egui reales; Tab real Linux y foco observado |
| P2 | Trabajo tabla/evidencia/editor, Archivo, recuperación, etapas con conteos, confirmación recálculo | Sesiones/selección/ediciones automatizadas; CtrlS solo Trabajo, prueba con TrabajoA+RevisiónB |
| P3 | Correos y Resoluciones lista/ficha, modalidades, Período contextual, exclusiones, Plantillas | Controladores y servicios preservados, suite de vigencia/Word/recibos/dobles Outlook |
| P4 | Captura Retomar y Avanzado, Anonimizador modos/entradas, Meta4 alcance explícito | Suites originales más contrato Python y captura MV3 en Chromium ficticio |
| P5 | Biblioteca Resultados con búsqueda y carpetas previas; Contactos lista/ficha/importación confirmada; ayuda | Biblioteca acotada sin symlinks, contactos y recuperación; revisión independiente |
| P6 | Evidencia tipada, cuatro estados, identidad compuesta, duplicados, discrepancias y ZIP seguro | Fixtures de límites/autoría/fechas/identidad y 2000ingresos; revisión independiente cerrada |
| P7 | Motor sobre paquete completo; selección posterior; huella/versión, propuesta y edición separadas | Prueba diferencia al aislar fila y paridad con contexto completo; revisión cerrada |
| P8 | UI independiente, segundo plano/cancelación, filtros/selección, evidencia legible y exportación XLSX+JSON atómica | Criterio obsoleto, cancelación/worker, cuatro Revisada, fallo JSON, colisión, cancelación intermedia y primer análisis compacto |
| P9 | Revisión de contrato, motor, exportación y UI con correcciones | Tres últimos hallazgos cerrados estáticamente en458c58c; pruebas Linux pasan. Falta cierre visual integrado y Windows final |
| P10 | Guía, fixture ficticio, matriz30controles y empaquetado con procedencia/huellas | Entrega final tras confirmar comprobaciones pendientes |

## Pruebas observadas

- Linux y Windows pasaron suites Rust completas en varias puntas. En315b1f8 Windows
  pasó tests, release y arranque CLI; la preparación gráfica del servidor falló.
- En458c58c Linux pasa pruebas nuevas y matriz real de128capturas (16escenarios,
  2tamaños,4escalas). Windows y matriz ampliada final aún deben confirmarse.
- Una prueba Windows detectó que rename podía reemplazar una carpeta vacía. Se
  corrigió con MoveFileExW sin reemplazo y se conserva la prueba.
- Verificador general315b1f8:1283Asistente,382Descargador,62subpruebas y golden pasan.
  FALLA inventario48bajas y restricción Python subprocess, idénticas en basecfc12ee2.
  No se oculta el fallo, no se regenera baseline/golden ni se autoriza integración.
- Capturas: renderizado real con datos ficticios y almacenamiento temporal aislado.
  No leen AppData del usuario ni acceden a Outlook.

## Revisión independiente y correcciones

Se corrigieron discrepancias metadata/CSV, identidad de eventos, conservación de
cobertura al cambiar intervalo, indexado por ingreso, identidad habitual en Excel,
publicación sin reemplazo y pruebas de fallos intermedios. Revisada sirve para las
cuatro categorías. La revisión final encontró primer análisis compacto inaccesible,
CtrlS desde Revisión exportando Trabajo anterior y reuso legado impidiendo evidencia
nueva. Las tres correcciones tienen pruebas y cierre estático del revisor.

Las capturas al200% motivaron paneles activos, scroll, Revisada sobre el editor y
barras más compactas. No se certifica una captura por el solo hecho de existir;
las observaciones y límites están en qa/UI_OBSERVACIONES.md.

## Límites de la entrega

Outlook y captura SITFA reales requieren sesión del usuario y no se han ejecutado.
Los dobles/fixtures no sustituyen esas pruebas. No cambian motor tradicional, textos,
golden, permisos MV3, retención ni originales; la semilla permanece en memoria.
La CI usa Mesa aislada para Windows, sin distribuir DLL ni tocar el sistema.

Reproducción: workflow .github/workflows/rust.yml; pruebas/ui_evidencia.py genera la
matriz; pruebas/verificar_base_plan.py compara el verificador conservando su fallo.
Rollback por revert de commits en la rama, sin reescribir historial ni borrar datos.

