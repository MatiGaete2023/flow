# Informe de fallas · prueba del usuario con `flow_rust.exe` (20-nov-2026)

Ejecutable: artefacto `Flow_Rust_Windows_exe` de la [corrida 21](https://github.com/MatiGaete2023/flow/actions/runs/44777123749), commit `9e428c8`. El usuario aportó cuatro libros de trabajo (tres reales y uno generado por Rust). **Ninguno se incorpora al repositorio.** El análisis usó solo:

- estructura anónima (hojas, encabezados, tipo y formato por columna, anchos, validaciones, formato condicional), sin valores de celda;
- para el punto 8, una copia de `NOVIEMBRE.xlsx` generada en la sesión con `nurus.personal.anonymize` en modo `conservador` (identidades seudonimizadas y nombres desconocidos omitidos), comparada por plantillas con marcadores `<ID>`, `<FECHA>` y `<N>`.

Niveles: **N1** hecho observado en archivos o código · **N2** inferencia con supuesto explícito · **N3** especulación.

## Resumen

| # | Falla reportada | Causa principal | Nivel | Esfuerzo |
|---|---|---|---|---|
| 1 | No lee los datos nuevos de Mejor Niñez ni los escritos pendientes | Trabajo en Rust solo lee la tabla `registros` del ZIP; historia, bitácoras, escritos e histórico (E3) no están portados | N1 | Alto |
| 2 | Proyectos de resolución distintos, no se pueden cargar, y la fecha cambió de formato | Rust genera un DOCX propio en vez de las plantillas institucionales; las fechas van con números y no en palabras | N1 | Medio |
| 3 | Sin lista de contactos no se pueden generar borradores; la nómina no se puede editar | Rust arranca sin contactos y no migra los de Flow Python; la nómina solo permite incluir o excluir filas | N1/N2 | Medio |
| 4 | No permite desmarcar en Correos y Resoluciones; Trabajo no marca varias ni todas | El área clicable de la tarjeta tapa el checkbox; no hay «marcar todas» en Trabajo | N2/N1 | Bajo |
| 5 | Faltan opciones en Ajustes, Trabajo y el resto del Asistente | La paridad de interfaz no está inventariada; faltan umbrales y acciones | N1 parcial | Alto |
| 6 | No se puede instalar la extensión desde el Descargador | No hay acción «Instalar extensión…» y el artefacto no trae la carpeta de la extensión | N1 | Bajo |
| 7 | El Excel final no tiene el formato de siempre | Con ZIP no se pudo mapear el consolidado y se usó la tabla reducida; el orden de columnas NURUS difiere del registro | N1/N2 | Medio |
| 8 | La redacción sugerida no coincide con la registrada | Ocho patrones de diferencia medidos sobre 1.013 filas de NOVIEMBRE | N1 | Medio (requiere decisión) |

## 1 · Mejor Niñez y escritos pendientes

- **N1.** `app/src/captura_trabajo.rs` construye las hojas de Trabajo solo con la tabla `registros`. No lee `historia_tramites`, `bitacoras`, `calendario_ingreso`, `escritos` ni `historico_proteccion` («Ingreso Histórico (Mejor Niñez)», `docs/python_unico/PLAN.md` P3c). Flow Python (`capture_input.import_capture`) los usa para contexto, avisos y observaciones (`HISTORIA.*`, escritos por resolver, curadores, órdenes de búsqueda).
- **N1.** En la matriz figuran pendientes F04, F05, F19, F20 y F21. En el registro del usuario hay observaciones que dependen de esos datos, por ejemplo «No se remite proyecto de resolución por existir escritos por resolver» e «informe … recepcionado en escrito de fecha …».
- **Solución.** Cumplir E3 en este orden:
  1. Lector de las tablas adicionales del ZIP en `captura.rs`. Ya valida estructura y estados; falta exponer las filas.
  2. Índice por identidad (sistema, tribunal, causa, ingreso), como `capture_input`.
  3. Portar `chronology`, `captured_rule_context`, `historico_para` y `escritos_para`, y las reglas `HISTORIA.*`, `COMUN.CURADOR` y `C10_ESCRITO` del motor.
  4. Contrastar con el corpus `FLOW_CASOS` con historia y bitácora, que hoy se excluye en `motor.rs:743`.

## 2 · Proyectos de resolución

- **N1.** Flow Python (`resolutions.generate_projects`) llena las plantillas institucionales `src/nurus/personal/plantillas_word/{LAJA,MULCHEN}/{PC_IE,PC_INFO,NOMENCL}.docx` con `fill_docx`. Aplica plural y varios programas, une las resoluciones con salto de página y conserva los estilos de cada matriz (`Composer(preserve_styles=True)`).
- **N1.** Rust (`productos::generar_word`) arma un DOCX mínimo con párrafos propios (`plantilla()`, `parrafo()`). Solo usa una plantilla del usuario si existe en `%APPDATA%\FlowRust\plantillas`. Por eso el formato es distinto.
- **N1.** Fechas: Python usa `date_in_words`, que escribe «catorce de septiembre de dos mil veintiséis», para `FECHA` y `FECHA_RESOLUCION`. Rust usa `fecha_es`, que escribe «14 de septiembre de 2026».
- **N2** (supuesto: «no permiten cargarlas» se refiere al sistema donde se suben los proyectos y que exige la matriz institucional). La causa es la misma: el documento no proviene de la plantilla.
- **Solución.** Incrustar las seis plantillas con `include_bytes!` (Tomé usa la de Mulchén, como hoy) y llenar sus `{{VARIABLES}}` reutilizando `llenar_xml`. Portar los reemplazos de plural y `number_in_words`/`date_in_words`. Unir los documentos con salto de página conservando estilos. Verificar comparando el texto y los estilos del DOCX contra el de Python con datos ficticios.

## 3 · Borradores sin contactos y nómina editable

- **N1.** `Config::default()` en Rust parte con `contactos` vacío. Python trae `contactos_ejemplo.json` y guarda `contactos.json` en su carpeta de datos, que Rust no importa. También aprende contactos escritos a mano (`outputs.py`, «Contacto guardado para el próximo lote»).
- **N1.** La nómina adjunta solo permite marcar o desmarcar filas (`ui.rs`, «Nómina adjunta»). No se pueden editar celdas ni agregar o quitar filas.
- **N2** (supuesto: el bloqueo ocurre al guardar o al preparar). En el código, un borrador sin destinatario solo se marca «⚠ Falta destinatario»; no hay una condición que lo impida. Hay que reproducir el caso con `Destino Outlook` activo.
- **Solución.**
  - Al primer inicio, importar los contactos de Flow Python si existen y, si no, cargar `contactos_ejemplo.json`.
  - Permitir guardar en Outlook con `To` vacío, con un aviso visible, y aprender los contactos escritos a mano.
  - Hacer la nómina una grilla editable (texto por celda, quitar y agregar filas) antes de crear el borrador.
  - El CC fijo `ucc_concepcion@pjud.cl` y `mail.Save()` no cambian.

## 4 · Marcar y desmarcar

- **N2** (supuesto: comportamiento de egui, donde el último widget registrado en un mismo rectángulo recibe el clic). En Correos (`ui.rs`, tarjetas de borradores) y Resoluciones (tarjetas de proyectos), después del checkbox se registra `ui.interact(r.response.rect, …, Sense::click())`, que cubre toda la tarjeta. El clic selecciona la tarjeta y no llega al checkbox.
- **N1.** En Trabajo solo existe «Marcar visibles como revisadas». No hay casilla de encabezado para marcar todas, ni para marcar o desmarcar las visibles o un rango con Mayús.
- **Solución.**
  - Registrar el área de la tarjeta antes del contenido, o usar la respuesta del título, para que el checkbox quede encima.
  - Agregar «Marcar visibles», «Desmarcar todas», casilla de encabezado y selección por rango con Mayús+clic.
  - Agregar una prueba de interacción con `egui::Context` simulado.

## 5 · Opciones faltantes

- **N1.** Umbrales de Python ausentes en Rust: `prorroga_reciente` y `ventana_busqueda`, ambos dependen de la historia de E3.
- **N1.** Pendientes de la matriz que el usuario ve como opciones faltantes:
  - F08: edición masiva, deshacer y selecciones ocultas.
  - F09: recibos y vigencia.
  - F11: migración de contactos.
  - F15: formularios manuales.
  - F16: búsqueda, historial y diagnóstico.
  - F26: diagnóstico.
- **Solución.** Hacer un inventario pantalla por pantalla de Flow Python (`app.py`, `app_base.py`, `resolution_view.py`, `revision_view.py`, `current_tools.py`) contra `ui.rs`, con estado por opción en una tabla nueva de la matriz. Implementar por prioridad de uso, a confirmar con el usuario.

## 6 · Instalar la extensión

- **N1.** Python Integral y Lite tienen «Instalar extensión…» (`descargador.py:1059`, `sitfa_lite.py:148`). Copia la ruta de la carpeta `extension` al portapapeles, abre `chrome://extensions/` y explica el paso «Cargar sin empaquetar».
- **N1.** El artefacto `Flow_Rust_Windows_exe` contiene solo `flow_rust.exe` y la vista Descargador de Rust no tiene esa acción.
- **Solución.**
  - Incrustar los archivos de `descargador_lite/extension/` y `descargador/extension/` en el ejecutable.
  - Agregar «Instalar extensión…»: escribe la carpeta en `%LOCALAPPDATA%\FlowRust\extension-<edición>-<versión>` (fuera de Descargas y sin `_internal`), comprueba `manifest.json`, copia la ruta, abre Chrome y muestra los pasos.
  - Incluir también la carpeta en el artefacto.

## 7 · Formato del Excel final

- **N1.** `Observaciones_ESPERA_2026-11-20_2.xlsx` es la tabla de fidelidad reducida de Rust (`productos::excel_observaciones`):
  - Una sola hoja con columnas canónicas más `SITFA_ARCHIVO/HOJA/FILA/SHA256`, `ID_REGISTRO_CAPTURA`, `ESTADO_REVISION`, `REGLAS`, `AVISOS` y `FILA_ORIGEN`.
  - Todo en formato General.
  - Sin `Resumen`, `SITFA_ARCHIVOS` ni `Vencimientos`, y sin columnas como `NACIONALIDAD / PAÍS`, `TIPO`, `SEXO`, `REVISIÓN`, `80 BIS`, `EDAD` o `FEC. ACT. F. …`.
  - Conclusión: la entrada fue un ZIP y `copia_excel::base_paquete` no encontró una base, así que se aplicó el plan B.
- **N1.** Los productos de Python (`Informes - revisable …`, `Amb_laja_mulchen`) y el registro `NOVIEMBRE` comparten la misma estructura:
  - El consolidado del paquete completo, con encabezado en negrita sobre relleno `195F7B`, texto con formato `@` y fechas como texto `dd/mm/aaaa`.
  - Las columnas NURUS al final, en este orden: `NURUS_ID_REGISTRO, NURUS_REGLAS, NURUS_PROPUESTA, FECHA_OBS, OBSERVACION, TT, CC, RES, NURUS_ESTADO_REVISION, NURUS_DECISIONES`.
  - Las hojas ocultas `NURUS_LISTAS` y `NURUS_TRAZABILIDAD`, y `RES` validado con la lista `NURUS_RES_TIPOS`.
- **Orden de columnas (resuelto el 20-nov: orden de NOVIEMBRE).** `flow-python` en `afc4d12` (`exports.py`, `_portable_preserved` y `_native_preserved`) escribe `OBSERVACION` antes de `FECHA_OBS`. Los tres archivos reales tienen `FECHA_OBS` antes de `OBSERVACION`, y Rust copió el orden del código. Se toma como norma el orden de los archivos, porque el producto debe calzar con NOVIEMBRE. Queda pendiente confirmar con el usuario y revisar la versión de Python que produjo esos archivos.
- **N2.** Causas posibles del fallo del mapeo, por verificar con un ZIP sintético equivalente:
  - `SITFA_FILA` en el consolidado es numérica y en `registros` puede tener otra base.
  - El consolidado puede repetir un registro entre hojas, lo que hace que `repetidas` anule la base.
  - Rust tampoco aplica el respaldo de Python cuando el consolidado no sirve: una copia portable de la misma estructura.
- **Solución.**
  1. Registrar el motivo exacto del descarte en el aviso.
  2. Corregir el mapeo con un ZIP sintético de varias hojas generado por `tools/flow_capture_fixture.py`.
  3. Fijar el orden de columnas del registro.
  4. Agregar `NURUS_DECISIONES`.
  5. Si el consolidado no sirve, generar una copia del mismo libro consolidado y no la tabla canónica.
  6. Probar el plan de columnas contra la estructura anónima de NOVIEMBRE.

## 8 · Redacción de observaciones (NOVIEMBRE, sugerida contra registrada)

| Hoja | Filas | Iguales | Solo mayúsculas o puntuación | Distintas |
|---|---|---|---|---|
| ESPERA | 165 | 67 | 0 | 98 |
| CUMPLIMIENTO | 524 | 214 | 35 | 275 |
| INFORMES | 324 | 115 | 60 | 149 |

Patrones observados (N1, salvo que se indique otra cosa). Las plantillas usan marcadores; los conteos son filas.

| ID | Reglas | Sugerida | Registrada | Filas | Propuesta |
|---|---|---|---|---|---|
| R1 | `CUMPLIMIENTO.C05_POR_VENCER` + `C10_HOJA2` | «Medida próxima a vencer en RUS el `<FECHA>`. Próximo informe de avance de `<PROGRAMA>` vence el `<FECHA>`.» | Solo la primera oración | ≈ 125 | Con medida por vencer, omitir la oración C10 |
| R2 | `HISTORIA.R3_MULTI` | «… La historia es de la causa y no permite atribuirlo a un ingreso determinado.» | Sin esa oración | ≈ 25 | Llevarla a aviso interno y sacarla de la observación |
| R3 | `HISTORIA.E04_CON_FOLIO` + `ESPERA.E05_SOLO_CORREO` (DCE) | Repite «Medida revisada, a la espera de ingreso efectivo.» y agrega «Al `<FECHA>`, RUS registra `<N>` días de espera.» | Sin la repetición ni los días | ≈ 15 | Unificar el encabezado y omitir los días cuando hay E04 |
| R4 | `ESPERA.E05_*` + `HISTORIA.SERVICIO_INGRESO` | Incluye «Al `<FECHA>`, RUS registra `<N>` días de espera.» | La quita en ≈ 12 filas y la conserva en ≈ 10 | ≈ 22 | Resuelto: quitar (ver decisiones) |
| R5 | `COMUN.REITERA_OBSERVACION` | «Se reitera observación anterior del `<FECHA>`.» | El texto vigente («Medida revisada, sin observaciones.», «La derivación … no se encuentra sujeta a seguimiento por este Centro.» o la observación completa) | ≈ 15 | No reiterar cuando la regla base tiene un texto propio |
| R6 | `INFORMES.I02`, `CUMPLIMIENTO.C10`, `C03` | Nombre del programa con el artículo en minúscula y comunas o palabras en mayúsculas sostenidas | Artículo inicial con mayúscula, comunas en tipo título, conectores en minúscula, tildes restituidas | ≈ 250 | Normalizar el nombre del programa al estilo del usuario (N2: la anonimización oculta parte del nombre; validar con datos ficticios y confirmar) |
| R7 | `HISTORIA.BASE_TRAMITE` («Prorroga») | «consta el trámite «Prorroga». RUS registra como fecha de egreso proyectado el `<FECHA>`.» | «se prorroga la medida hasta el `<FECHA>`» o «se aplica «Prorroga» por `<N>` meses» | 3 | Redactar la prórroga con su efecto |
| R8 | `HISTORIA.C10_ESCRITO` + nomenclatura | Sugiere el proyecto de nomenclatura de cumplimiento | Se omite el proyecto | 1 | Muestra insuficiente: observar más casos |

Diferencias que no provienen de las reglas: textos que el usuario agrega con información externa, como la proyección de ingreso informada por el programa por correo, sugerencias sobre curadores designados en resoluciones o escritos por resolver. R1, R2, R3, R5 y R6 son consistentes y explican la mayoría de las filas distintas.

**Restricción.** `AGENTS.md` mantiene intactos los textos tradicionales salvo decisión expresa del usuario y paridad golden. Este informe constituye la solicitud. Cada cambio requiere:

1. Confirmación por patrón (en especial R4 y R6).
2. Actualizar la línea golden del motor de Rust con casos ficticios por regla.
3. Decidir si Flow Python también se ajusta.

## Decisiones del usuario · 20-nov-2026

Decisión expresa del usuario sobre textos de observaciones y estructura del producto (`AGENTS.md`: textos tradicionales solo cambian con decisión expresa y paridad golden).

| Tema | Decisión | Aplicación |
|---|---|---|
| R1 | Aprobada | Con `CUMPLIMIENTO.C05_POR_VENCER`, omitir la oración de `C10_HOJA2` («Próximo informe de avance de … vence el …»). |
| R2 | Aprobada | Omitir de la observación «La historia es de la causa y no permite atribuirlo a un ingreso determinado.» (`HISTORIA.R3_MULTI`); queda como aviso interno. |
| R3 | Aprobada | Con `HISTORIA.E04_CON_FOLIO` y `ESPERA.E05_SOLO_CORREO`, un solo «Medida revisada, a la espera de ingreso efectivo.» y sin la oración de días de espera. |
| R4 | **Quitar** | Cuando la observación incluye «Tribunal dispone ingreso a …» (`HISTORIA.SERVICIO_INGRESO`), omitir «Al `<FECHA>`, RUS registra `<N>` días de espera.». |
| R5 | Parcial | No reiterar (usar el texto vigente) solo cuando el texto vigente es «Medida revisada, sin observaciones.» o «La derivación … no se encuentra sujeta a seguimiento por este Centro.». En los demás casos se mantiene «Se reitera observación anterior del `<FECHA>`.». |
| R6 | Aprobada | Nombre del programa en tipo título: artículo inicial con mayúscula, comunas y palabras sin mayúsculas sostenidas, conectores en minúscula y tildes restituidas. El usuario escribió «56»; se interpreta como R6. |
| R7, R8 | Sin decisión | Se mantienen los textos actuales hasta que el usuario decida. |
| Orden de columnas | Confirmado | Columnas NURUS al final del consolidado: `NURUS_ID_REGISTRO, NURUS_REGLAS, NURUS_PROPUESTA, FECHA_OBS, OBSERVACION, TT, CC, RES, NURUS_ESTADO_REVISION, NURUS_DECISIONES`. Cierra el conflicto de la sección 7. |
| Flow Python | Sin decisión | Los cambios de redacción se aplican en Flow Rust. Ajustar Flow Python requiere instrucción aparte. |

Cada cambio de redacción se implementa con casos ficticios por regla en la línea golden de Flow Rust, con la diferencia respecto del texto anterior documentada en el registro.

## Orden propuesto de corrección

1. **7 y 4**: producto Excel conforme al registro y marcas. Es lo que hoy impide el uso diario y tiene bajo riesgo.
2. **6 y 3**: instalar la extensión, contactos y nómina editable.
3. **2**: plantillas institucionales y fechas en palabras.
4. **8**: R1, R2, R3, R4, R5 parcial y R6, según las decisiones del usuario.
5. **1**: E3, con historia, bitácoras, escritos y Mejor Niñez.
6. **5**: inventario de opciones y paridad de interfaz.

Cada corrección se registra en `REGISTRO_IMPLEMENTACION.md` con pruebas sintéticas y su corrida de Windows.
