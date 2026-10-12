# Revisión externa independiente — CSMP–SITFA

**NO APTO — N2, criterio de revisión: los defectos N1 de anonimización, recuperación de recibos y retención deben corregirse antes de presentar esta versión para aceptación del usuario.**

Fecha: 12 de octubre de 2026. Este veredicto es una evaluación técnica; no declara aceptación ni condición institucional.

## 1. Alcance y evidencia

- **N1:** revisé `feat/ejecucion-plan-20261008` en `98a9f82344246b0505bf68d6682ae808deffb3de`. Revisé separadamente `wp/93-ef04` en `04095e7d480940b55da31a5d6e799925804642e1`, mediante el diff de tres puntos solicitado. WP-93 no está integrado en la rama canónica.
- **N1:** leí primero `docs/implementacion/INFORME_REVISOR_EXTERNO_20261012.md`; luego las instrucciones, planes, continuación, traspaso, manuales y estados JSON indicados. Contrasté las afirmaciones con código y pruebas.
- **N1:** el árbol de la rama canónica permaneció limpio al inicio y término del verificador. Las sondas, productos ficticios, informe y logs están fuera del proyecto. Las mutaciones se hicieron en otra copia y se revirtieron. No hice commits, push, merge, etiquetas ni releases.
- **N1:** utilicé fixtures del repositorio y simulaciones locales. No consulté SITFA, RUS, SATURNO ni Outlook real. Los accesos remotos fueron a GitHub y a las dependencias del entorno.
- **N1:** entorno local: Linux, CPython 3.12.14, Tcl/Tk 9.0.4, dependencias instaladas con `--require-hashes` desde `requirements/linux-py312.lock`; inventario de versiones conforme. Para GUI se levantó Xvfb y se usó `DISPLAY=:99`, con `PYTHONPATH=/workspace/flow-review/src`. No había `xvfb-run` instalado inicialmente; la primera ejecución sin pantalla se interrumpió y no se usa como evidencia de producto.

En este informe, **N1** significa observado mediante archivo, comando, prueba o CI; **N2**, inferencia con supuesto explícito; **N3**, especulación. No empleo N3 para sustentar el veredicto. Que no haya encontrado otro defecto no demuestra que no exista.

### Resultados de verificación

| Comando / evidencia | Resultado verificado — N1 |
|---|---|
| `python tools/estado.py validar` | PASA; 48 paquetes, 44 integrados, 3 bloqueados por usuario y 1 en verificación en la rama canónica. WP-93 tiene estado en su propia rama. |
| `python tools/check_real_ids.py` | Cero hallazgos del detector. No equivale a certificar ausencia de cualquier dato personal. |
| `python tools/check_real_ids.py --historia` | Cero hallazgos entre los objetos alcanzables por las referencias locales revisadas. No revisa las referencias/cachés externas pendientes de WP-17. |
| `python tools/check_links.py` | Cero enlaces rotos. |
| `python tools/verificar_wp.py --salida /workspace/review-evidence/verificacion-xvfb.json` | **FALLA**: Asistente 901 aprobadas, 11 omitidas, 6 xfail, 1 fallida. Descargador 246 aprobadas y 62 subpruebas. Pasan inventario, identidades, golden, restricciones, Ruff, módulos y enlaces. |
| Repetición diagnóstica: `pytest -q -x -p no:cacheprovider` con el mismo DISPLAY/PYTHONPATH | 768 aprobadas, 7 omitidas, 6 xfail y 1 fallida antes de detenerse. Falla `tests/test_ui_theme.py::test_asistente_conserva_preferencias_y_estilos_al_cambiar_tema`, en la comparación del color de fondo, antes del tramo de foco. |
| `git diff --exit-code 286c28f HEAD -- src/nurus/personal/motor src/nurus/personal/textos_base.json` | Sin diferencias. |
| `node descargador/test_bitacoras_extension.cjs` | PASA; operaciones de lectura e identidad bajo simulación. |
| WP-93: `pytest -q descargador/test_ef04_reutilizacion.py` | 17 aprobadas. La sonda adicional H4 encuentra un caso no cubierto. |
| `python tools/flow_verify_capture_products.py <directorio externo>` | PASA: Excel, 2 nóminas, 1 proyecto Word, 3 descriptores de borrador; edición externa, recuperación, reimportación y rechazo de productos obsoletos comprobados. |
| Sonda continua `e2e.py` | Originales ficticios del Descargador → exportador real → ZIP → Asistente → Excel con 2 hojas, 2 nóminas, 1 Word y 3 guardados simulados de borradores, sin errores. CC obligatorio en las 3 llamadas; sesión recuperada con sus recibos. |

**N1 — diagnóstico local:** el traceback de la repetición apunta a la línea 27 del script hijo generado por `run_gui`, que corresponde a `tests/test_ui_theme.py:71`: `style.lookup('Primary.TButton','background')==palette['primary']`. La falla ocurre antes de solicitar foco o generar `<Return>`. No es evidencia de repetición del fallo de foco del run Windows anterior. **N2:** podría depender del entorno Tcl/Tk o de la aplicación; supuesto: la diferencia observada se limita a esta combinación de tema/plataforma. La causa raíz y su reproducción en Windows quedan **sin datos**. El verificador completo descartó el nodeid original, por lo que no puedo afirmar identidad entre ambas fallas; sí identificar la de esta repetición.

El recorrido continuo usa `bound_lot` y `diaries` del repositorio; no prueba un servidor institucional ni COM real. Los descriptores del autocontrol tampoco son correos guardados en Outlook. Esta distinción importa.

**N1 — benchmark solicitado:** medianas de tres repeticiones, en segundos:

| Filas | Importar | Editar 1 registro | Exportar | Guardar sesión |
|---:|---:|---:|---:|---:|
| 200 | 0,7064 | 0,0010 | 0,9421 | 0,3619 |
| 1.000 | 2,2241 | 0,0009 | 3,1102 | 0,9974 |

Hubo otras verificaciones ejecutándose en el entorno. Estos tiempos no acreditan los factores históricos de mejora ni tiempos en el PC del usuario.

### CI consultada

- **N1:** el [run 38282508134](https://github.com/MatiGaete2023/flow/actions/runs/38282508134) corresponde a `04095e7`, de WP-93. Los tres jobs Windows —Python 3.12, 3.13 y 3.14— finalizaron correctamente. La construcción, capturas, verificación del instalador y productos adicionales se ejecutaron en **3.12**; esos pasos están omitidos en 3.13 y 3.14. No son tres construcciones completas independientes.
- **N1:** el instalador estaba disponible, con vencimiento informado por GitHub el 11 de noviembre de 2026. Inspeccioné las capturas ficticias del artefacto `CSMP-Windows-interface`, ID `11761953175`; no ejecuté el instalador localmente.
- **N1:** el [run 38278082379](https://github.com/MatiGaete2023/flow/actions/runs/38278082379) falló en 3.12 y 3.13; 3.14 pasó. El log de 3.12 identifica `test_asistente_conserva_preferencias_y_estilos_al_cambiar_tema`. El cambio `b63070d` agrega espera y comprobación explícita del foco antes del evento de teclado.
- **N2:** el foco es una explicación consistente del fallo anterior, suponiendo que la aserción y el cambio representan su causa. Un run posterior correcto no demuestra ausencia futura de intermitencia.

Fuentes de código congeladas: [rama canónica, `98a9f82`](https://github.com/MatiGaete2023/flow/tree/98a9f82344246b0505bf68d6682ae808deffb3de) y [WP-93, `04095e7`](https://github.com/MatiGaete2023/flow/tree/04095e7d480940b55da31a5d6e799925804642e1). Las referencias `archivo:línea` son relativas a esos árboles.

## 2. Tabla de objetivos

Los estados evalúan la evidencia disponible; «parcial» puede significar comportamiento implementado con un límite o verificación insuficiente.

| Objetivo | Estado | Evidencia y nivel |
|---|---|---|
| Descargador produce paquete; Asistente importa y prepara productos | Cumple en recorrido ficticio | **N1:** `e2e.py`, `e2e.json`, autocontrol de productos. Uso real: sin datos propios. |
| Nunca enviar; CC obligatorio | Cumple en código revisado | **N1:** `adapters/outlook.py:150` aplica CC y `:171` llama `Save`; `services/policy.py:5`; control negativo y pruebas del adaptador con dobles. No encontré llamada de envío. |
| Sin escritura RUS/SATURNO | Cumple en código revisado | **N1:** `extension/pagina.js:394` permite apertura de bitácora por GET; `:421` valida consultas; `:427` restringe descargas. No encontré una operación de registro. Un POST de consulta no es por sí mismo escritura. |
| Asistente no navega SITFA ni inicia Descargador | Cumple en código revisado | **N1:** control AST de importaciones, importador local y lanzador externo en `tools/csmp_launcher.py`. |
| Sin credenciales persistidas | Parcial en alcance de comprobación | **N1:** puente usa código efímero; extensión no tiene permiso de cookies ni lectura de `document.cookie`. No encontré credenciales persistidas en los módulos inspeccionados. Ausencia universal en binarios, historial y artefactos: sin datos suficientes. |
| Sin datos reales; D1/purga | Parcial | **N1:** detector sin hallazgos, WP-17 abierto. H1 y H5 impiden usar anonimización/detector como garantía completa. |
| Cero correos institucionales distintos del CC | Parcial en literalidad de la métrica | **N1:** búsqueda textual encontró dos coincidencias distintas del CC en nodeids de `linea_base.json:510`; pertenecen a parametrización de pruebas. No son evidencia de contactos reales, pero el conteo literal no es cero. |
| D4: lotes de 60 días, hasta 10 sesiones anteriores | No cumple para el flujo nuevo completo | **N1:** H3. En Asistente los límites se aplican tras confirmar; sesión vigente y adjuntos externos se conservan, según `session.py` y WP-13. |
| Máximo 2 copias tras 3 cargas normales | Cumple con alcance acotado | **N1:** prueba `test_guardar_sesion_deja_solo_las_dos_copias_recuperables`; `work.py:620`. WP-13 documenta excepción legado de hasta 4 copias recuperables. |
| Motor, umbrales y textos protegidos | Cumple respecto de referencia indicada | **N1:** diff vacío desde `286c28f` y paridad golden. No implica resolver conflictos con otra especificación. |
| 17 productos golden | Cumple | **N1:** control golden PASA en esta revisión. |
| D3: retiro de registro/inspector | Cumple en alcance revisado | **N1:** ausencia de módulos de registro en árbol operativo y restricción negativa PASA. |
| D5/g: cronología visible, 4 meses, solo mostrar | Cumple en implementación; aceptación pendiente | **N1:** `capture/temporal.py:97`, autocontrol y WP-50 bloqueado. No modifica por sí sola las reglas tradicionales. |
| D6: consolidado opcional en ZIP | Cumple en implementación | **N1:** contrato 1.1.0, `contract.py:213`, preferencia y manual. Fidelidad nativa en Office real: sin datos propios. |
| D8: no introducir variante descartada | Cumple en lectura del código | **N1:** `extension/pagina.js:375`; decisiones registradas en plan y JSON. |
| D9: retirar botón de solicitud | Cumple | **N1:** vista vigente y pruebas de retiro; conservar funciones/formato de solicitud para compatibilidad no contradice retirar el botón. |
| D2/D7: actas y cierre por usuario | No cumple la meta de cierre; respeta la espera | **N1:** plantilla sin ejecución, WP-02/WP-70 bloqueados; PR #1 abierto y en borrador al consultar GitHub. |
| Decisiones a–f, h–i: integración y excepciones | Parcialmente verificable | **N1:** constan en plan y estado; fichas conservan excepciones. Autoría y consentimiento original fuera del repositorio: sin datos independientes. |
| j: E3 por riesgo | Cumple documentalmente en WP-92/93 revisados | **N1:** existen informes E3 con CAMBIOS y correcciones posteriores. **N2:** independencia efectiva de las raíces, si los identificadores declarados son fieles; no tengo trazas externas para certificarla. |
| 32 R-T con pruebas ejecutadas | Cumple la métrica de existencia; parcial como protección | **N1:** seis xfail estrictos registrados. H6 demuestra una regresión que un xfail puede absorber. |
| Cero módulos no alcanzables y enlaces rotos | Cumple según herramientas | **N1:** ambos controles en cero. No certifica ausencia de funciones muertas ni duplicación dentro de módulos alcanzables. |
| Aproximadamente 600 archivos | No cumple la cifra orientativa | **N1:** `git ls-files` contiene 821 archivos en el SHA revisado. No encontré una nueva meta cuantitativa que sustituya esa cifra. |
| Aproximadamente 4 acciones de transferencia | Parcial | **N1:** consulta/paquete integrado, importación y autoexportación reducen pasos. **N2:** depende de conexión ya preparada, alcance, diálogo de fecha y confirmaciones. No hay medición de uso real equivalente a “4 clics”. |
| Manuales fieles a interfaz | Mayormente cumple; parcial | **N1:** botones, etapas, autoexportación, recuperación y separación de aplicaciones existen. Retención, comportamiento ante archivos dañados y capturas requieren las correcciones de este informe. |
| CI controla lo anunciado | Parcial | **N1:** locks, privacidad, estado, suites y construcción condicional están implementados. CI rápida no ejecuta los nueve controles completos de `verificar_wp.py`; no debe presentarse como equivalente. |

## 3. Hallazgos por severidad

No confirmé hallazgos críticos. **N2:** el veredicto, las severidades, las correcciones sugeridas y los costos son juicios de revisión. Supuesto: la versión presentada debe preservar privacidad, recibos y gestión de retención en el recorrido recomendado antes de solicitar su aceptación. Los escenarios reproducidos se identifican separadamente como N1.

### H1 — Alta: el anonimizador deja pasar personas presentes solo en texto libre

- **Archivo:** `src/nurus/personal/anonymize.py:470`, `:536`, `:598`; entrada CLI `tools/anonimizar_excel.py`.
- **Escenario:** un nombre de persona está únicamente en OBSERVACION y no aparece en una columna reconocida como persona/curador → no entra al diccionario de reemplazos → queda íntegro en el Excel “anónimo”. La revisión final busca los mismos nombres conocidos y tampoco lo detecta.
- **Evidencia N1:** `probes.py` reutiliza `source()` y un nombre de `PERSONAS` de los fixtures, sin escribir ese nombre en el informe. Resultado: `anon_unknown_person_survives=true`; el XLSX resultante pasa `check_real_ids` con salida 0. Solo datos ficticios.
- **Consecuencia N2:** si el operador comparte una copia con un tercero mencionado solo en narrativa, podría divulgarlo. Supuesto: confiar en la denominación de copia anónima sin detectar manualmente el residuo. No afirmo que haya ocurrido una divulgación real.
- **Corrección sugerida:** para exportación compartible, reemplazar o excluir texto libre no validado por un esquema explícito; conservar únicamente los valores necesarios para reproducir la regla. Agregar prueba con persona exclusiva de narrativa y con hoja sin encabezado reconocido. La instrucción de revisión manual no hace verdadera la promesa “falla cerrado”.

### H2 — Alta: un error leyendo el diario permite repetir un borrador ya guardado

- **Archivo:** `src/nurus/personal/work.py:232`, `:648`; `src/nurus/personal/outputs.py:591`, `:624`.
- **Escenario:** se guarda un borrador y su recibo solo está en el diario; ocurre un corte antes de consolidar la sesión. Al recuperar, `PermissionError` u otro error de lectura del diario se trata igual que su ausencia → se carga `trabajo.json` sin el recibo → el siguiente lote permite otra llamada para el mismo borrador.
- **Evidencia N1:** en `probes.py` simulé el efecto del adaptador, conservé el diario válido y provoqué `PermissionError` solo al leerlo. La carga devolvió cero recibos; repetir el mismo borrador produjo dos efectos simulados totales. El segundo lote informó `created=1`, `errors=[]`. No se invocó Outlook real.
- **Consecuencia N2:** duplicación en Outlook si el primer `Save` real había finalizado. Supuesto explícito: el corte cae entre persistencia de diario y consolidación final, y el error afecta su lectura posterior.
- **Corrección sugerida:** distinguir diario ausente de diario existente ilegible/inválido. En este último caso bloquear nuevos guardados y exigir recuperación o conciliación, preservando evidencia. Mantener el tratamiento separado de diario válidamente obsoleto. Cubrir permisos, JSON truncado y estructura inválida.

### H3 — Alta: la retención no reconoce los paquetes creados por el flujo principal

- **Archivo:** `descargador/flujo_paquete.py:14`; `descargador/preferencias.py:156`, `:163`; `descargador/ventana_resultados.py:275`.
- **Escenario:** “Consultar y crear paquete” genera `Paquete CSMP …/respaldo/…`, con `proceso.json` y controles anidados. Pasan más de 60 días → `old_batches` solo busca `resumen.json`, `bitacoras.json` o `flujo.json` directamente dentro de cada carpeta hija → no ofrece ese paquete para limpieza.
- **Evidencia N1:** `probe_retention.py` ejecuta el pipeline con runners ficticios del repositorio y exportador real. Envejecí a 61 días las marcas de todos los archivos y carpetas. Resultado: ZIP existente, dos controles bajo respaldo y **cero lotes reconocidos** por `old_batches`, aun sin rutas protegidas.
- **Consecuencia N2:** los datos del recorrido recomendado pueden permanecer indefinidamente fuera de la gestión de retención. Supuesto: el usuario usa el nuevo flujo y la limpieza incorporada.
- **Corrección sugerida:** reconocer explícitamente el contenedor de paquete y sus controles, con comprobaciones de pertenencia, enlaces y fuentes en uso. No resolver mediante borrado recursivo indiscriminado. Agregar regresión que cree el paquete por el pipeline y luego lo gestione a los 61 días.
- **Precisión N1:** pedir confirmación en Asistente no es por sí mismo un defecto: lo exige WP-13. El defecto aquí es no encontrar el paquete ni siquiera para ofrecer esa confirmación.

### H4 — Media: WP-93 puede abortar por un manifiesto anterior malformado

- **Rama y archivo:** solo `wp/93-ef04`, `descargador/lectura_bitacoras.py:222`, `:349`.
- **Escenario:** un registro anterior válido en clave, fecha y parámetros contiene `archivo: null` → supera la selección de reutilización → `Path(None)` lanza `TypeError` → aborta el nuevo lote, en lugar de omitir la caché y hacer la lectura normal.
- **Evidencia N1:** `probe_wp93.py` crea el primer lote usando el `Bridge` ficticio, cambia únicamente ese campo del manifiesto y ejecuta otro lote. Resultado: `TypeError`, cero aperturas nuevas. Las 17 pruebas propias de EF-04 pasan sin esa sonda.
- **Corrección sugerida:** validar tipos y estructura completos antes de usar el registro; un archivo previo inutilizable debe quedar fuera de la reutilización. Distinguir fallas leyendo una fuente anterior de fallas escribiendo el destino actual. Agregar `null`, listas, errores de permisos y campos faltantes.

### H5 — Media: el detector omite un Excel contenido como `.bin` dentro de un ZIP

- **Archivo:** `tools/check_real_ids.py:136`, `:155`.
- **Escenario:** un XLSX contiene identificadores sintéticos que el detector reconoce → se incluye exactamente ese XLSX como `sesion.bin` dentro de un ZIP → el recorrido interno filtra por extensión y no inspecciona su contenido → termina sin hallazgos.
- **Evidencia N1:** `probe_scanner.py` usa exclusivamente `sonda()` de `test_anonimizar_excel.py`. Mismos bytes: salida 1 como XLSX directo; salida 0 dentro de `anidado.zip/sesion.bin`.
- **Corrección sugerida:** aplicar detección de contenido también a miembros anidados, conservando presupuestos de tamaño y profundidad; ante formato no inspeccionable, declarar revisión incompleta. Probar contenedores renombrados. No presentar este detector de RUT como detector general de personas, correos o credenciales.

### H6 — Media: los xfail estrictos pueden esconder fallas distintas del conflicto registrado

- **Archivo:** `tests/test_reglas_informes.py:286`, `:307`; patrón equivalente en otros casos xfail con varias aserciones.
- **Escenario:** R-T24 pierde la regla tradicional de informe vencido, antes de llegar a la aserción del hallazgo CT-05 pendiente → falla con `AssertionError` → pytest lo clasifica como el xfail esperado y devuelve éxito.
- **Evidencia N1:** mutación aislada de `work.py` para excluir eventos `INFORMES.I01_`; la prueba R-T24 resultó **1 xfailed, código 0**. Su primera aserción protege precisamente esa regla tradicional. No afirmo que la suite completa deje pasar la misma mutación: otras pruebas pueden detectarla.
- **Corrección sugerida:** separar las invariantes ya implementadas en pruebas ordinarias y acotar el xfail a la discrepancia concreta. Mantener `strict=True` y el conflicto visible hasta decisión del usuario; no cambiar motor ni borrar casos para obtener verde.

### H7 — Baja: las capturas de CI no permiten cerrar legibilidad completa a 1024×650

- **Archivo/evidencia:** `tools/window_capture.py:6`; `tools/smoke_personal_gui.py:99`; `tests/test_audit_gui_20260928.py:127`; artefacto `CSMP-Windows-interface` del run 38282508134.
- **Escenario observado N1:** las imágenes de Trabajo y Correos muestran la franja derecha parcialmente cortada o mal repintada, con controles/textos incompletos. El smoke comprueba geometría de determinados controles, no integridad visual de toda la página.
- **Límite:** **sin datos** para distinguir un defecto real de diseño de un problema de `PrintWindow`/renderizado en ese runner. No atribuyo esas bandas a la aplicación con certeza.
- **Corrección sugerida:** regenerar capturas confiables con fixtures y verificar bordes/recortes de todos los controles esenciales, foco visible y escala de Windows. Confirmar navegación real por Tab; forzar foco y generar `<Return>` no verifica por sí solo todo el recorrido de teclado.

### H8 — Info: verificador y CI tienen alcances distintos

- **Archivo:** `.github/workflows/ci-rapida.yml:31`, `:62`; `.github/workflows/tests.yml:116`; `tools/verificar_wp.py:81`.
- **Escenario N1:** un run verde de CI rápida prueba suites, lint, formato y sus controles previos, pero no ejecuta directamente la comparación completa contra línea base, golden e inventario que hace `verificar_wp.py`. El verificador, a su vez, resume las suites sin conservar en su JSON el nombre de la prueba fallida.
- **Evidencia N1:** YAML leído y resultado local: medición Asistente válida, con una falla; mensaje de control genérico “Medición, línea base o declaración de bajas inválida”. Ese mensaje no identifica la causa real.
- **Corrección sugerida:** declarar con precisión el alcance de cada evidencia y conservar log/nodeids de fallos. Incorporar los controles de candidato faltantes en una ejecución de entrega, sin duplicar builds ni suponer que E3 queda reemplazada por CI.

## 4. Pruebas, calidad y límites de las conclusiones

### Mutaciones ejecutadas en copia separada

| Mutación | Resultado N1 | Interpretación |
|---|---|---|
| Carga ignora diario de recibos | 3 fallan, 9 pasan | Las pruebas de WP-92 detectan perder el diario en la ruta normal. |
| Lote omite guardado completo previo | 3 fallan, 9 pasan | Detectan la precondición corregida después de E3. |
| R-T24: desaparecer regla base de informe vencido | 1 xfailed, salida 0 | El xfail absorbe una falla ajena al conflicto CT-05. |
| WP-93 ignora hash del HTML anterior | 1 falla, 16 pasan | Detectan modificación del archivo anterior. |
| WP-93 ignora condición de mismo día | 1 falla, 16 pasan | Detectan reutilización entre fechas distintas. |

**N1:** los seis conflictos son R-T13, R-T19, R-T24, R-T25, R-T26 y R-T32. Sus marcadores son estrictos y restringidos a `AssertionError`. Eso hace fallar un XPASS, pero no exige que una falla ocurra en la aserción esperada. CT-03..10 siguen fuera del alcance vigente; no recomiendo implementarlas por iniciativa del revisor.

**N1 — aspectos correctos observados:** `sync.py:45` asocia por identidad, rechaza duplicación/faltantes y comprueba los campos de identidad; `:124` trabaja sobre copia de filas y `:179` rechaza conflictos antes de reemplazar el trabajo. `importing.py` lee una instantánea de bytes y resuelve hoja/encabezado antes de devolver registros. El autocontrol recuperó ediciones externas sin perder revisión local. El contrato de captura comprueba rutas, duplicados, tamaños, expansión, hashes y contenido XLSX peligroso; no extrae libremente un ZIP al disco.

**N1 — persistencia y concurrencia:** `config.atomic_json` usa temporal, `flush`, `fsync` y `os.replace`; `Work.save` conserva respaldo y verifica los `.bin`. La GUI entrega resultados por cola, bloquea entradas durante `_run`, evita autosave durante `busy` y rechaza cerrar mientras hay una operación. Esto es una base razonable para errores ordinarios, pero H2 demuestra un vacío concreto en recuperación. No realicé cortes físicos de energía ni validé ejecución simultánea de dos instancias en Windows.

**N1 — superficie de extensión:** Manifest V3 con `activeTab`/`scripting`, acceso al dominio SITFA y loopback; puente enlazado a `127.0.0.1`, comparación del código de conexión y validación del origen/extensión. La sesión institucional se usa mediante peticiones con credenciales del navegador; no se extraen cookies para persistirlas. Las restricciones de destino y acciones están en código, además de las pruebas. **N2:** el hash local detecta alteraciones respecto del manifiesto suponiendo que este sea confiable; no autentica a quien modificara conjuntamente HTML y manifiesto. Esa frontera debe mantenerse explícita.

**N1 — mantenibilidad:** `outputs.py` tiene 789 líneas y `work.py` 679; mezclan varias responsabilidades de productos y persistencia. El recálculo y la conciliación están separados en módulos especializados. No inferí “código muerto cero” desde un grafo de módulos alcanzables. No medí duplicación ni complejidad ciclomática de todo el árbol; sus cifras actuales quedan **sin datos**. No propongo una reescritura general para corregir los hallazgos.

**N1 — dependencias:** lock de Linux instalado con hashes y versiones verificadas; los workflows seleccionan locks Windows por Python. Fijar versiones no demuestra ausencia de vulnerabilidades: no hice un análisis actualizado de CVE de todas las dependencias ni una comparación binaria de builds.

## 5. Afirmaciones no confirmadas, excesivas o contradichas

| Afirmación del proyecto | Evaluación |
|---|---|
| “Ya no queda trabajo de programación que bloquee el cierre”, informe de partida `:144` | **Contradicha por N1:** H1–H3 requieren correcciones antes de esta aceptación técnica. Que el informe la rotule N2 no elimina las contraevidencias. |
| D4 “Implementado”, informe `:30` | **Incompleta por N1:** hay gestión confirmada para sesiones y lotes clásicos, pero falla para contenedores del flujo recomendado, H3. |
| Anonimizador “falla cerrado” si queda un dato original, `anonymize.py:13` | **Falsa en ese alcance general por N1:** H1 produce una copia con el nombre ficticio narrativo intacto. |
| “Sin datos reales en árbol e historial” demostrado por cero hallazgos, informe `:28` | **No demostrado:** el detector tiene alcance limitado y H5 muestra una omisión concreta. No encontré datos reales y no afirmo que los haya. La limpieza externa sigue sin verificar. |
| WP-93: cualquier estructura previa inválida se omite, `lectura_bitacoras.py:223` | **Contradicha por N1:** `archivo: null` provoca `TypeError`. |
| “902/11/6” y “9 controles PASA” | **N1:** son resultados históricos de la línea base; mi corrida obtuvo 901/11/6 y 1 falla. El código de producto no difiere de la base en ese tramo, pero eso no reemplaza medir el entorno actual. |
| “PASA en Python 3.12–3.14: EXE, instalación/desinstalación” | **N1:** pasan suites en las tres versiones; EXE/instalación se verifican solo en el job 3.12. La frase requiere esa precisión. |
| “44 de 48 integrados” | **Confirmada N1** en la rama canónica. WP-93 no se debe sumar como si perteneciera al mismo inventario. |
| “819 archivos” y meta aproximadamente 600 | **N1:** la punta revisada tiene 821. La diferencia con 819 puede corresponder a documentos posteriores; la meta aproximada de 600 sigue sin alcanzarse. |
| E3 independiente y excepciones del usuario | **N1:** hay documentos y decisiones registradas. **Sin datos independientes** para autenticar las conversaciones originales o la independencia de las raíces. No las atribuyo a una revisión mía. |
| Mejoras “245×” y “123,8 → 4,3 s” | **Sin reproducción comparativa propia** de ambas versiones en el mismo equipo. Sí medí los tiempos actuales consignados arriba. |
| Manual ilustrado anterior como prueba del flujo actual | **No suficiente:** está versionado 2.7.4; el manual MVP y código 2.7.6 deben gobernar el flujo vigente. |

## 6. Riesgos que debe cerrar el uso real

Estas son inferencias **N2**, con supuesto común: ejecutar la versión corregida en el PC Windows autorizado, con configuración y sesión reales del usuario. No son resultados observados por este revisor.

1. **Outlook clásico:** ubicación/cuenta de Borradores, firma, adjuntos exactos, políticas COM, errores antes/después de `Save`, cierre de ventanas y conciliación de un estado incierto. Contar borradores antes y después; comprobar que no se envía ninguno.
2. **Excel y Word:** fidelidad de copia nativa, formatos/fechas, archivos abiertos o bloqueados, textos largos, sincronización y conservación de ediciones. Comparar productos con un resultado conocido, sin compartir su contenido.
3. **SITFA y extensión:** sesión vencida, filtros, paginación, vínculos de ingreso, cambios de estructura y respuestas parciales; verificar que los mensajes no equivalgan a ausencia de actuaciones.
4. **EF-04:** contrastar reutilización frente a relectura explícita cuando hubo actuaciones posteriores el mismo día. Menos de 12 horas no garantiza actualidad.
5. **Cronología:** contraste de los diez ingresos previstos, origen del centro, fechas, empates, entradas fuera de horizonte y posterioridad respecto del corte. Mantener “solo mostrar”.
6. **Teclado y pantalla:** Tab/Shift+Tab, foco visible, Ctrl+O/S/K, diálogos, acceso a Pausar/Cancelar, escala 125–150 %, ventana reducida y lector de pantalla si el usuario lo utiliza. Las capturas actuales no cierran este punto.
7. **Recuperación/retención:** reinicio durante cada fase, carpetas movidas, permisos, sincronización local y limpieza de un paquete antiguo con fuentes en uso. No hacer estas pruebas con originales únicos sin respaldo local autorizado.

Solo solicitar resultados agregados, errores sin contenido sensible o extractos anónimos verificados localmente. Dado H1, no asumir que ejecutar `tools/anonimizar_excel.py` basta para compartir un archivo. La aceptación corresponde exclusivamente al usuario mediante el acta de la versión exacta.

## 7. Recomendaciones priorizadas

| Prioridad | Acción verificable | Costo |
|---:|---|---|
| 1 | Corregir H1: exportación compartible con esquema y tratamiento conservador de texto libre; nueva prueba de persona exclusiva de narrativa. | Medio |
| 2 | Corregir H2: bloquear repetición de borradores ante diario existente ilegible; pruebas de recuperación y conciliación. E3 por Outlook. | Medio |
| 3 | Corregir H3: incluir paquetes con `respaldo/` en la gestión de retención, preservando rutas protegidas y confirmación. E3 por borrado. | Medio |
| 4 | Corregir H5: inspección de contenido anidado o rechazo explícito de revisión incompleta; prueba ZIP con `.bin`. E3 por detector. | Medio |
| 5 | Corregir H4 antes de integrar WP-93; probar tipos inválidos y fallas de lectura de caché. | Bajo |
| 6 | Separar invariantes normales de los seis conflictos xfail y ejecutar mutaciones focales. No tocar reglas protegidas sin decisión. | Bajo |
| 7 | Resolver la discrepancia local de color de tema y contrastarla en Windows/Tk soportado; conservar log/nodeids completos del verificador. | Medio |
| 8 | Renovar capturas ficticias confiables y verificar teclado/geometría en Windows con escala del usuario. | Medio |
| 9 | Rectificar informe de avance y metas: alcance de privacidad, retención, CI, archivos y diferencia entre software verificado y aceptación. | Bajo |
| 10 | Tras corregir, fijar SHA/artefacto, ejecutar la verificación candidata y la prueba de usuario; mantener WP-17 y actas pendientes hasta evidencia efectiva. | Medio |

## 8. Evidencia reproducible adjunta

Los archivos están en el mismo directorio de este informe. No contienen datos de uso real.

- `verificacion-xvfb.json` y `.log`: corrida completa, SHA y árbol limpio.
- `asistente-detalle.log`: repetición diagnóstica y traceback de la aserción de tema.
- `estado.log`, `identidades.log`, `historia.log`, `enlaces.log`, `motor.diff`, `dependencies.json`, `extension.log`.
- `benchmark.log`, `products.log`, `e2e.py`, `e2e.json`.
- `probes.py` / `probes.json`: H1 y H2.
- `probe_retention.py` / `retention.json`: H3.
- `probe_wp93.py` / `wp93-malformed.json`: H4; `wp93.diff` y `wp93.log`.
- `probe_scanner.py` / `scanner.json`: H5.
- `mutations.py` / `mutations.json`, `mutate_wp93.py` / `wp93-mutations.json` y logs individuales: cinco mutaciones.
- `ui/`: siete capturas ficticias obtenidas de CI.

Las sondas usan las rutas de las copias locales mencionadas en sus encabezados. Para reproducir en otra máquina, ajustar esas rutas al mismo SHA, instalar el lock de la plataforma y mantener los dobles de Outlook/Bridge; no reemplazarlos por sistemas reales.
