# Auditoría integral del repositorio — 16 de septiembre de 2026

Versión resultante: **0.4.0.dev6**. Rama: `csmp-personal-2026-09-13`.

## Alcance

Se revisó la estructura completa del repositorio, entradas ejecutables, aplicación personal, motor RUS, exportación Excel, borradores Outlook, resoluciones Word, pruebas, workflow, recursos empaquetados y documentación vigente/histórica. La auditoría distingue código ejecutable obsoleto de componentes generales o evidencia histórica que siguen teniendo una función distinta.

## Hallazgos corregidos

### 1. Capa transitoria de monkey-patching

Existían cuatro módulos `runtime_fixes_20260916*` cargados por efectos secundarios desde `nurus.personal.__init__`. Funcionaban, pero convertían el orden de importación en parte del comportamiento y mantenían dos implementaciones para varias reglas.

Solución: integrar exportación revisada en `work.py`, identidad de borradores en `outputs.py`, clasificación/deduplicación de proyectos en `resolutions.py`, interacción GUI en `app.py` y estados `REVISADO/PENDIENTE` en `services/exports.py`. Los cuatro módulos transitorios se eliminan y `nurus.personal.__init__` vuelve a ser declarativo.

### 2. Exportación podía perder el sentido de la revisión humana

La ruta histórica exportaba como propuesta aunque hubiera edición en la ventana. Además, una fila editada podía provocar escritura vacía sobre TT/CC/RES existentes no tocados.

Solución: detectar revisión humana por fila, capturar el editor antes de exportar, marcar editadas como `REVISADO`, mantener intactas como `PENDIENTE` y preservar valores fuente no modificados. Se conserva la defensa contra fórmulas de Excel.

### 3. Identidad inestable de borradores

La corrección intermedia calculaba la huella con la ruta absoluta del adjunto. Las nóminas automáticas se crean en carpetas UUID, por lo que dos preparaciones del mismo contenido podían parecer distintas y permitir un borrador duplicado.

Solución: la huella usa destinatarios efectivos, CC, asunto, cuerpo, obligatoriedad y `nombre + SHA-256` de cada adjunto. La ruta temporal queda fuera. La clave se recalcula al guardar para incorporar cualquier edición humana posterior a la vista previa.

### 4. Resoluciones dependían de parche visual

La interfaz podía insertar varias filas de personas del mismo caso y después borrar duplicados visualmente. La selección del tipo también podía depender de la sugerencia original aun cuando la observación humana mostrara otra gestión.

Solución: `automatic_project_selections` aplica precedencia explícita y `unique_case_selections` deduplica por tribunal/RIT/tipo antes de poblar la GUI. La edición manual exige selección concreta y evita crear duplicados del mismo caso/tipo.

### 5. Código sin uso en módulos intervenidos

Se retiraron imports sin uso de `work.py` y `resolutions.py` y la función privada `_excel_column_name` del exportador, que no tenía referencias. El antiguo generador unitario `generate_word` se conserva explícitamente como compatibilidad: no forma parte del flujo visible actual, pero eliminarlo sin contrato de deprecación podría romper consumidores externos.

### 6. Workflow y release desalineados con la revisión actual

La release seguía identificada como dev5 y el workflow usaba revisiones antiguas de acciones que generaban advertencias de runtime.

Solución: versión `0.4.0.dev6`, distribución `CSMP-Windows-dev6`, workflow actualizado y contratos que verifican versión, recursos y ausencia de la capa transitoria.

## Elementos deliberadamente conservados

- El núcleo general `nurus.app`, `domain`, `storage`, `services` y `rus`: mantiene entrada y cobertura de pruebas propias, además de componentes compartidos.
- `Abrir_NuRus.bat` e `Instalar_NuRus.bat`: pertenecen a esa ruta general; no se clasifican como basura de la aplicación personal.
- Documentos históricos fechados: se conservan como trazabilidad. `INDICE_DOCUMENTACION.md` define cuáles son vigentes.
- Campos de sesión `needs_cross`/`exception`: se mantienen solo para leer sesiones antiguas; el flujo vigente no los usa como bloqueo.
- `generate_word`: compatibilidad con el generador unitario previo; el flujo vigente usa proyectos agrupados.

## Contratos agregados o reforzados

La suite verifica exportación de edición humana, preservación de campos no editados, PENDIENTE/REVISADO, estabilidad de huella frente a rutas temporales, cambio de huella por contenido, clasificación de resoluciones, no creación de proyectos por mera mención de informe sin acción, deduplicación por causa/tipo, edición individual y ausencia de archivos transitorios/generados.

## Límites

La auditoría estática y CI no sustituyen una ejecución con Excel 2010 y Outlook clásico institucionales. Tampoco resuelven por inferencia el conflicto sustantivo del umbral DCE señalado en `CAMBIOS_USO_20260914.md` ni crean matrices de Tomé sin fuente.

[G-ESTADO] Repositorio consolidado como **0.4.0.dev6**; código transitorio de la corrección del 16-09 absorbido por los módulos definitivos.
[L-SIGUIENTE] Cerrar la auditoría solo con CI completa del SHA resultante y luego ejecutar aceptación institucional Office.
