# Especificación implementable de UX — dos apps CSMP

**Base:** `adeb6ec`, 12-oct-2026. La ruta solicitada no está disponible; revisión en copia aislada. Aplicar después de integrar los cambios en curso: autoexportación al importar, «Consultar y crear paquete», preferencia XLSX única, retiro del campo firmas, `respaldo/` y lanzador. No duplicarlos. Solo modificar interfaz; sin motor, textos tradicionales, modelos, contratos, almacenamiento, transporte ni dependencias nuevas.

## 1. Sistema visual común — implementador A

Crear `src/nurus/ui_theme.py`, únicamente stdlib/tkinter/ttk: `tokens(mode)`, `install_ttk(root, mode)`, `button_options(role, mode)`. Este último devuelve datos; CustomTkinter se importa solo desde el adaptador existente `personal/ui.py`. Compartir tokens con `descargador/apariencia.py:apply`. No cambiar `vista.tema` del Asistente ni `prefs.data['tema']` del Descargador; conservar Claro/Oscuro/Sistema y su resolución actual. `appearance_view.py:build` sigue guardando las mismas preferencias.

| Token | Claro | Oscuro |
|---|---|---|
| Fondo / tarjeta | #F3F6FA / #FFFFFF | #15191F / #20262E |
| Texto / secundario | #203248 / #536B7C | #EDF2F7 / #A5B3C2 |
| Primario; texto | #176D8C; #FFFFFF | #277595; #FFFFFF |
| Peligro; texto | #B42318; #FFFFFF | #B42318; #FFFFFF |
| OK fondo/texto | #DCFCE7 / #166534 | #143827 / #BBF7D0 |
| Aviso fondo/texto | #FFF1D4 / #765113 | #653B29 / #FFE2CD |
| Error fondo/texto | #FEE2E2 / #991B1B | #4C2020 / #FECACA |
| Pendiente fondo/texto | #E8F0F5 / #203248 | #2B3541 / #EDF2F7 |

Texto normal ≥4,5:1 en cada combinación; selección usa blanco sobre primario. Foco de 2 px azul sobre claro y amarillo #E7BC55 sobre oscuro, verificable ≥3:1. Tipografía Segoe UI 11 pt, títulos 18 pt, encabezados 12 pt; controles CTk conservan escala propia. Espaciado 4/8/12/16/24 px; controles ≥32 px de alto. Estilos: `Primary.TButton`, `Secondary.TButton`, `Danger.TButton`, `Card.TFrame`, `OK.TLabel`, `Warning.TLabel`, `Error.TLabel`, `Pending.TLabel`; mapear active/focus/disabled, no solo normal. Texto secundario sobre tarjeta; estado siempre con palabra e icono, nunca solo color.

Modificar `personal/ui.py:install_theme`, `Button.__init__`, `PageStack.add` para adaptar roles sin pasar argumentos `style` de ttk a CTk. No parches globales ni reemplazo de widgets. Un propietario del módulo compartido: A; B lo consume. Importación normal empaquetable desde `nurus`, sin alterar sys.path ni specs en esta ola.

## 2. Asistente — implementador A

Modificar `app_base.py:App.__init__`, `_update_context`; `next_step.py:NextStepBar.__init__/refresh`; `work_view.py:build`; `mail_view.py:build_mail_page`; `resolution_view.py:build`; `results_view.py:build`; `commands.py:commands`.

Extender la barra existente con encabezado «1 Importar → 2 Revisar → 3 Correos → 4 Resoluciones» y `Siguiente acción`. Estados: Pendiente, Disponible, Atención, Bloqueado, Completado. Importar completo significa trabajo cargado; exportación fallida mantiene bloqueo de productos. Revisar muestra incidencias, nunca certifica revisión humana por tener cero avisos. Correos/Resoluciones se apoyan en `check_products` y recibos: preparado no equivale a guardado; obsoleto/incierto exige atención. Sin productos aplicables: «No aplica».

El nuevo botón navega a la vista/registro requerido; sin trabajo llama al importador existente. Nunca guarda Outlook ni genera Word automáticamente. Deshabilitado mientras busy/rendering/modal. Registrar la acción en Ctrl+K utilizando las condiciones existentes. Conservar navegación libre de todas las páginas.

Compatibilidad: mantener `counts`, `text`, claves de `buttons` y etiquetas probadas. El stepper nuevo es el encabezado; los controles actuales permanecen como detalle plegable «Estado de productos», con el mismo temporizador. No crear una segunda comprobación periódica. «Siguiente acción» es secundario cuando la vista ya tiene primario.

Un primario visible por vista, según contexto: importar/actualizar en Trabajo; preparar o guardar borradores en Correos; preparar/generar en Resoluciones. Mantener ambos botones separados y sus alcances: el código llama **«Preparar tipo»** al solicitado «Preparar». Resultados conserva acciones secundarias en su ubicación actual.

Vacíos: instrucción de una frase y acción existente, p. ej. «Importa un paquete para revisar registros». Reutilizar `status` para progreso y añadir banner persistente de resultado/error hasta nueva operación o descarte explícito. No cambiar `_poll` de negocio: observar estados desde interfaz. Foco tras importación en tabla, tras preparación en primer producto; Tab/Shift+Tab, Enter/Espacio, Escape en diálogos; conservar Ctrl+O/S/F/K y F5, sin activaciones dentro de modal.

## 3. Descargador — implementador B

Modificar `descargador.py:Application.__init__/refresh/process_event/apply_theme`, `ventana_flujo.py:JointWindow.__init__`, `ventana_resultados.py:ResultsWindow.__init__/show_result`, `apariencia.py:apply`. Los cambios de `process_event` se limitan a presentación.

Agrupar: **Conexión y alcance**; **Acción principal** con el botón único en curso; **Lote** con Pausar/Cancelar/Abrir carpeta/Resultados; **Avanzado**, plegable con descargas separadas, lectura/reintento manual, favoritas y configuración. Crear widgets en sus contenedores desde el inicio: Tk no permite reparentar. Conservar atributos, comandos y habilitación originales; paquete manual continúa disponible para recuperación. No mover productos secundarios de sus ubicaciones actuales.

Panel fijo: fase, consulta n/total, página n/total, archivos verificados y tiempo transcurrido (`time.monotonic`). ETA solo con total conocido y avance suficiente de esa misma fase; en otro caso «Estimando…». Congelar cálculo durante pausa; distinguir «Pausa solicitada», «Pausado por usuario» y «Espera SITFA». No inventar countdown si el evento no proporciona duración; no modificar pausas reales.

Final: ruta del ZIP seleccionado por el usuario, cobertura/pendientes y botones «Abrir carpeta del paquete»/«Copiar ruta», habilitados tras archivo existente. Copiar solo ruta local, nunca código de conexión. Parcial/cancelado/error no usa badge OK. Mantener resultado al quedar idle. Scroll no debe ocultar Pausar/Cancelar; teclado y foco visible en principal y desplegable.

## 4. Contratos y aceptación

No renombrar atributos `pages`, `tabs`, `records`, `work_primary_button`, `mail_action_buttons`, `mail_action_reasons`, `save_all_button`, `results_download_button`, `generate_word_button`, `next_step.buttons/values`; Descargador `download_button`, `package_button`, `pause_button`, `cancel_button`, `package_consolidated`. Conservar «Aceptar» en diálogo de fecha, «3 Correos (2)», «2 Exportar copia ✓» y salida exacta de `text(counts(None,[],[]))`: pruebas `test_integral_ui`, `test_siguiente_paso`. `test_paquete_lote_actual` exige habilitación y mensaje «Paquete creado:». No controles ocultos creados únicamente para satisfacer tests.

Pruebas focales con `xvfb-run -a python -m pytest tests/test_integral_ui.py tests/test_siguiente_paso.py tests/test_trabajo_vista.py tests/test_resolution_case_view_20260916.py`; Descargador, mismo comando desde `descargador/` con `test_paquete_lote_actual.py test_lotes.py test_flujos.py test_ritmo_checkpoint.py`. Añadir casos UX de foco, tema, modal, parcial y banner. Verificación completa una vez al integrar.

Objetivos de clics, **conteo estático, no medición de usuario**: importación→copia actual, con selectores idénticos, elimina 1 activación Exportar; lote conectado/configurado→ZIP elimina 2 activaciones separadas (bitácoras/paquete), dependiendo del cambio en curso; localizar siguiente tarea ≤1 activación; copiar ruta final 1 frente a recorrido manual no medido. Mantener confirmaciones. Adaptar encabezado a dos filas si falta ancho. Validar 1024×650 y Windows al 150 %, cero controles críticos tapados, cuatro etapas visibles, teclado completo y contrastes AA.
