# Evidencia de estructura recuperada — 24 de noviembre de 2026

Revisión dirigida de los tres ZIP originales conservados localmente y del código de Explorador V1.6. SHA-256 completos de los tres archivos concuerdan con el Plan Maestro. No publicar ZIP, eventos, datos judiciales ni capturas.

| Sesión | Eventos | OBSERVATION_MANUAL_STEP | DOM_STRUCTURE con observation_form |
|---|---:|---:|---:|
| V1.5 e6043fc8… (Espera) | 162 | 20 | 8 |
| V1.5 e90089c9… (otras vistas) | 93 | 18 | 6 |
| V1.6 5f82c777… (Espera) | 87 | 15 | 4 |
| Total | 342 | 53 | 18 |

En los 53 manual_snapshot.controls, los controles visibles presentan esta firma, sin excepciones en la muestra, índices 0–5: select COD_Estado_Detalle_Nuevo, checkbox NO_CATALOGADO, textarea NO_CATALOGADO, text NO_CATALOGADO, button irAccion, button NO_CATALOGADO. Los seis constan habilitados. El editor visible no tiene maxlength exportado; los editores ocultos sí incluyen GLS_Observacion con 2000. V1.6 documenta tres opciones del selector, no sus códigos o rótulos completos.

Código original extension/content_observer.js: safeTechnicalName (80–85) reemplaza nombres no admitidos; isHistoryControl / entryControls / orderedEntryControls (114–126) excluyen HOBS_ y controles históricos ocultos y ordenan hidden al final; manualFormSnapshot (149–188) conserva atributos estructurales, no valores del editor ni IDs DOM. No interpretar NO_CATALOGADO como atributo HTML ausente. El índice solo es parte de una firma, no una autorización de operación.

Nuevo detector pasivo: exige un formulario único, POST, TablaInforme visible, cinco campos de contexto ocultos presentes, firma exacta de controles visibles, Estado con tres opciones, irAccion único visible y ausencia de contradicciones. Devuelve únicamente resultado categórico; no devuelve nodos, no lee valores de campos de identidad, no modifica controles, no genera eventos ni conecta a RUS. Si no existe asociación label/input, devuelve false; si el texto del botón no confirma Nueva obs, también false. No sustituye el detector estricto de preparación ni se carga en la extensión aprobada. Fixture reconstruido con datos inventados, no HTML real.

Verificación: node --check de structure.js y REGISTRADOR_CHROMIUM=/usr/bin/chromium python registrador_rus/tests/run_structure.py -> PASS 12 structural checks. Chromium local Linux; solicitudes abortadas. Se comprobó cero eventos submit/input/change/click durante inspección. No acredita compatibilidad DOM real, acción del formulario, identidad efectiva, ni códigos de Estado. No se ha integrado el adaptador de preparación institucional. Mantener write_enabled:false. No se requiere repetir las cuatro grabaciones; si se necesita evidencia adicional, una inspección pasiva focalizada del formulario deberá resolver solo las dudas que persistan.
