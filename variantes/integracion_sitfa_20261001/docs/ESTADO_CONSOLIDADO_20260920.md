# Estado consolidado del proyecto NuRus / CSMP Assistant — 20 de septiembre de 2026

**Repositorio:** `MatiGaete2023/NuRus`  
**Producto vigente:** CSMP Assistant personal  
**Versión vigente:** `0.4.0.dev11` (ver actualización siguiente)
**Checkpoint histórico del cuerpo:** `0.4.0.dev6`
**Rama candidata dev9:** `csmp-ux-20260921` (destinada a fast-forward de `main` y de la rama personal tras el cierre)  
**HEAD al iniciar esta revisión:** `0523f3cb666a7e55b6a4fd64eb92949c89a58e26`  
**Finalidad de este documento:** dejar una representación canónica del estado del proyecto, las decisiones del usuario, los problemas encontrados, cómo se resolvieron, los límites vigentes y el orden recomendado de los siguientes pasos.

Este documento reemplaza como **estado de navegación** a los resúmenes parciales anteriores. Los informes fechados se conservan como evidencia histórica y no deben reinterpretarse como descripción automática de la versión vigente.

## Congelamiento funcional — 22 de septiembre de 2026

Dev11 queda **congelada como candidata funcional** después de prueba real satisfactoria del flujo principal: Excel, RES categórico, resoluciones, borradores de correo y parámetros/configuración. Durante uno o dos ciclos normales solo se corregirán errores reproducibles o fricciones materiales observadas; no se añadirán funciones nuevas ni `.exe`. Referencia: `CONGELAMIENTO_CANDIDATA_20260922.md`.

## Actualización vigente — UX final dev11

Se aplica el pulido final solicitado para uso personal: búsqueda y filtros en Trabajo/Resoluciones; códigos de resolución con descripción legible; color por origen; botón para abrir carpeta de salida; barra de contexto persistente; atajos `Ctrl+O`, `Ctrl+F`, `F5` y `Ctrl+Enter`; y visualización centrada en excepciones. Los filtros son solo visuales y no eliminan ni reducen silenciosamente los registros o proyectos. Se excluyen deliberadamente los contadores laterales y el empaquetado `.exe`. Ver `UX_FINAL_20260922.md`.

## Checkpoint anterior — RES categórico dev10

La revisión final de Resoluciones reemplaza el uso recomendado de números en `RES` por tres valores explícitos: `PC_IE`, `PC_INFO` y `NOMENCL`. Las copias nuevas presentan lista desplegable compatible con Excel; vacío significa sin proyecto. El lector conserva marcas antiguas (`1`, `X`, etc.) para no romper planillas anteriores. Un valor escrito pero desconocido produce advertencia visible y no se adivina. En Resoluciones se diferencia `Definido en RES`, compatibilidad antigua y sugerencia automática; un cambio manual queda marcado como ajuste en la interfaz. Ver `RES_CATEGORICO_20260922.md`.

### Cierre de verificación dev10

Commit `8546dab08ae9976c2e0f26810e2e49d5fee30ced`, GitHub Actions run `35729404913`: **success** en Windows Python 3.12/3.13/3.14, con **256 passed, 1 skipped** en cada versión. Python 3.12 aprobó además el smoke CustomTkinter y construyó la distribución Windows dev10.

## Checkpoint anterior — panel y correos dev9

Se integra CustomTkinter (dependencia nueva en el entorno aislado Windows), navegación lateral y tarjetas de borradores con datos reales. Destinatarios y filtro de tribunal se separan: solo programas, solo tribunales o ambos. `Preparar todos` mantiene la naturaleza de cada comunicación: `programa_*` va a programas; el informativo general y `medidas` van a tribunales. La migración de correos revisión 3 recupera el texto del manual de Informes por vencer sin reemplazar textos personales. No existe botón Enviar; solo guardar uno/todos. Se conserva la limpieza dev8 validada en `bab202e9533e88b5130a277f01b90b816e98940e`, run `35620647820`. El cierre dev9 está validado en `c262321e227302cc9349061a941c06af29879219`, run `35646164874`: Windows 3.12/3.13/3.14 success, 254 passed y 1 skipped por versión, smoke CustomTkinter y distribución dev9 correctos. Los pendientes institucionales continúan.

## Checkpoint anterior — limpieza dev8, 21 de septiembre de 2026

Producto: **CSMP Assistant personal 0.4.0.dev8**. Se retiran la interfaz NuRus, sus diálogos de producto, el benchmark histórico y nueve implementaciones de métodos sustituidas en el Asistente. Se conservan componentes compartidos, formatos y sesiones. Correcciones: hoja al cambiar de modo; resoluciones manuales al actualizar sin cambios y reexportar; limpieza de borradores/proyectos al localizar una copia modificada; editores vacíos al cambiar de trabajo; desactivaciones configuradas fuera de las cuatro casillas.

Referencias: `LIMPIEZA_ASISTENTE_20260921.md` y `VERIFICACION_PERSONAL.md`. Pendientes externos sin cambios: Office institucional, DCE, matrices Tomé y medición del ciclo real. Las secciones anteriores en fecha son historial, no el estado actual del ejecutable.

## Actualización material — 21 de septiembre de 2026

Producto: **CSMP Assistant personal 0.4.0.dev7**. El cuerpo fechado el 20-09 conserva el estado dev6 como referencia. Los cambios posteriores, pruebas y límites están en `REVISION_USABILIDAD_20260921.md`. Las decisiones de dominio permanecen vigentes; se continúa solo la experiencia del Asistente. Los accesos NuRus anteriores pasan a abrirlo. CI de dev7: run `35551027804`, commit `86e8ba8e095db5aa97e9d1a0fae8a51937633c2d`, Windows 3.12/3.13/3.14 exitoso; 239 pruebas aprobadas y 1 omitida por versión. El cierre posterior es solo documental.

---

## 1. Resumen ejecutivo

NuRus comenzó como una unificación técnicamente robusta del motor RUS, revisión, persistencia, correos, resoluciones y trazabilidad. Durante las pruebas reales se comprobó que esa dirección resolvía problemas técnicos, pero también exponía al usuario demasiada arquitectura interna: lotes, estados, aprobaciones, snapshots y productos. El resultado llegó a exigir más pasos que las herramientas originales para algunas tareas.

La decisión de producto se corrigió el 13 de septiembre: **no desechar el trabajo técnico de NuRus, pero dejar de usar dev7 como experiencia de usuario principal**. La aplicación personal adopta la simplicidad del Asistente y del RUS Engine, la ergonomía del Creador de Correos y componentes técnicos seleccionados de NuRus.

El estado vigente es una aplicación Windows con cinco áreas:

1. **Trabajo**: analiza Espera, Cumplimiento e Informes, genera una copia Excel preservada y permite revisión humana.
2. **Correos**: prepara comunicaciones desde el mismo trabajo, permite editarlas y solo guarda borradores Outlook.
3. **Resoluciones**: agrupa proyectos por causa/tipo y genera un único Word con proyectos separados por página.
4. **Configuración**: umbrales, textos, plantillas de correo, contactos, alias, Outlook y matrices Word.
5. **Enviados**: consulta Outlook en modo solo lectura y permite exportar resultados.

El producto es **candidata funcional congelada**, validada en el flujo principal por el usuario. No es todavía una release 1.0 productiva: quedan casos institucionales específicos, el conflicto DCE, matrices Tomé y la fase breve de uso cotidiano.

---

## 2. Invariantes cerrados

Estas reglas prevalecen sobre cualquier refactorización futura:

- **Windows es el único sistema objetivo de CSMP Assistant personal.**
- **Nunca enviar correos.** Outlook solo puede recibir borradores guardados; no existe una ruta autorizada de `Send()`.
- Un borrador puede quedar **sin destinatario** cuando el contacto no es conocido o es ambiguo; el usuario lo completa manualmente.
- La copia institucional obligatoria se mantiene; solo la CC adicional es configurable.
- **No escribir directamente en RUS ni SATURNO.** La aplicación prepara insumos; el registro oficial sigue siendo humano.
- El Excel original debe permanecer intacto.
- Las observaciones, correos y resoluciones son insumos sometidos a revisión humana.
- No inventar matrices judiciales, destinatarios ni reglas faltantes.
- Los cambios funcionales deben reducir tiempo, pasos, errores reales o mejorar la calidad del producto; la complejidad técnica por sí sola no justifica una función.

---

## 3. Topología del repositorio

Al comienzo de esta revisión existían cuatro ramas visibles:

| Rama | Rol |
|---|---|
| `main` | baseline antiguo; estaba 163 commits detrás de la rama personal |
| `implementacion-plan-2026-09-08` | prototipo NuRus dev7; referencia técnica/histórica |
| `revision-producto-csmp-2026-09-13` | auditoría comparativa independiente; conserva una rama documental divergente |
| `csmp-personal-2026-09-13` | producto vigente y rama de integración |

La rama personal desciende de `implementacion-plan-2026-09-08` y está 93 commits por delante de ella. La rama de revisión de producto conserva una auditoría útil, pero no debe mezclarse mecánicamente con el runtime; sus conclusiones se incorporan aquí.

**Decisión de cierre de esta revisión:** después de comprobar la actualización documental mediante CI, `main` debe avanzar por fast-forward al mismo checkpoint de la rama personal. Las ramas históricas se conservan para trazabilidad y no se borran.

---

## 4. Tamaño e inventario auditado

Árbol del HEAD de runtime auditado `0523f3cb…` (antes de agregar esta consolidación documental):

- 147 archivos versionados.
- 69 archivos bajo `src/`.
- 33 archivos bajo `tests/`.
- 35 archivos bajo `docs/`.
- 87 archivos Python en total.
- Aplicación personal bajo `src/nurus/personal/`.
- Núcleo NuRus general bajo `src/nurus/` conservado como referencia y componentes compartidos.

La auditoría del 16-09 eliminó la capa transitoria `runtime_fixes_20260916*` y trasladó sus correcciones a módulos definitivos. El repositorio incorpora contratos para evitar volver a versionar esa capa, cachés Python y basura común de escritorio.

---

## 5. Evolución y principales observaciones del usuario

### 5.1 Instalación y entorno

**Problema observado:** la primera experiencia exigía comandos manuales y el instalador rechazaba Python 3.14 aunque el paquete podía ejecutarse con él.

**Solución:** se declaró soporte explícito de Python **3.12, 3.13 y 3.14**, se agregó detección de esas versiones y un entorno aislado. Para el producto personal se usan:

- `Instalar_CSMP.bat`
- `Abrir_CSMP.bat`
- `.venv-csmp`

No se requieren privilegios de administrador ni Node. La instalación puede utilizar `paquetes/` como repositorio offline institucional.

### 5.2 Exportación Excel real

**Problema observado:** una versión de NuRus podía analizar el archivo pero Excel/COM fallaba al guardar la copia; además la salida debía ser un producto operativo equivalente o superior al RUS Engine.

**Solución:** se simplificó la automatización nativa de Excel y se reutilizó el exportador preservado. El usuario confirmó posteriormente que la exportación funcionó en su computador. El producto personal conserva hojas, fórmulas, estilos, anchos y filtros mediante la ruta nativa, mantiene los excluidos visibles y no modifica el original.

### 5.3 Aprobación fila por fila

**Queja:** exigir aprobar cientos de registros individualmente convertía la automatización en más trabajo que el proceso manual.

**Primera solución:** dev7 pasó a aprobación masiva de filas limpias y revisión individual solo de excepciones.

**Solución de producto vigente:** el flujo personal elimina por completo esa ceremonia visible. El motor genera propuestas; el usuario revisa en la GUI o en la copia Excel y las salidas usan el mismo trabajo. No hay que aprobar “productos” uno por uno.

### 5.4 El programa no entregaba los productos reales

**Queja:** el prototipo podía preparar un “producto” textual, pero no ofrecía de forma natural el Excel final, Word de resoluciones y correos equivalentes a las herramientas originales.

**Solución:** CSMP Assistant personal se organiza alrededor de los tres productos reales:

- copia Excel revisable;
- proyectos Word;
- borradores Outlook.

Correos y Resoluciones reutilizan automáticamente la copia del trabajo; no se obliga a volver a seleccionar el archivo que la propia aplicación generó.

### 5.5 Sobrediseño para una escala inexistente

**Queja:** NuRus acumuló persistencia, snapshots, estados, aprobaciones y optimizaciones pensadas para miles de registros o escenarios multiusuario, aunque el usuario real es esencialmente uno.

**Decisión:** conservar solo la robustez que no agrega fricción. Hashes, identidad, recuperación y defensas permanecen internamente cuando aportan seguridad; la interfaz ya no obliga a operar conceptos como snapshot, materialización o estado de producto.

### 5.6 Calidad y veracidad de observaciones

Hubo dos necesidades distintas que debían separarse:

1. mejorar redacción y permitir modificar textos sin tocar Python;
2. no registrar en RUS una observación que describa una gestión que todavía no se ha hecho.

La decisión final del usuario fue que las observaciones de gestión se usan **después de realizar la gestión**, por lo que deben conservar redacción de actuación realizada (por ejemplo, “Se remite…”), no fórmulas del tipo “se sugiere preparar…”. Generar la propuesta en pantalla **no acredita** que la acción se haya realizado: el registro oficial ocurre solo cuando el usuario efectivamente ejecuta la gestión y luego la incorpora en RUS.

Los textos están separados de la lógica y se pueden editar desde Configuración. Modificar la redacción no cambia por sí solo la acción estructurada de la regla.

### 5.7 Reglas, textos y parámetros editables

**Necesidad:** poder ajustar algunas condiciones operativas sin editar código, pero sin crear un diseñador universal de reglas.

**Solución:** Configuración expone umbrales de dominio, activación de determinadas advertencias, textos de observación, plantillas de correo, contactos, alias y parámetros Outlook. La lógica técnica sensible (parseo, identidad, seguridad, no envío, etc.) no se expone como opción.

### 5.8 Correos

El Creador de Correos original demostró una UX mejor que el primer NuRus. Se adoptaron sus principios:

- selección de tipo;
- selección de tribunal(es);
- modalidades;
- período;
- Para, CC, asunto y cuerpo editables;
- adjuntos;
- vista previa;
- guardado de borradores;
- plantillas configurables.

Se agregaron capacidades de NuRus:

- detección automática de comunicaciones específicas desde acciones estructuradas;
- nóminas automáticas por grupo;
- recuperación desde planilla revisada;
- prevención de duplicados;
- guardado masivo.

**Reglas cerradas:**

- nunca enviar;
- destinatario vacío permitido;
- CC institucional obligatoria;
- contactos ambiguos no se adivinan;
- `Guardar TODOS` puede preparar y guardar el lote aunque no exista una vista previa previa;
- la identidad del borrador depende del contenido efectivo y del hash/nombre de los adjuntos, no de la ruta temporal.

### 5.9 Firma Outlook y ventanas de Inspector

Se conserva la firma institucional mediante Outlook. Una corrección del 14-09 cierra el Inspector después de guardar el borrador firmado para evitar dejar una ventana abierta por cada elemento del lote. No se agregó ninguna operación de envío.

### 5.10 Contactos

Se consolidó una fuente configurable de contactos y alias. Puede importarse un catastro Excel. Los conflictos no reemplazan silenciosamente un contacto existente y una coincidencia desconocida/ambigua deja el campo Para vacío.

### 5.11 Contador de correos

El script rígido de “Contador de Correos” no se mantiene como programa separado. Su función útil se integró en **Enviados**:

- rango de fechas;
- filtros;
- consulta solo lectura;
- total;
- exportación.

Un borrador no se cuenta como correo enviado.

### 5.12 Resoluciones

La dirección final se aparta del generador textual genérico de dev7 y de las cadenas embebidas del Asistente antiguo.

Estado vigente:

- tipos `PC_IE`, `PC_INFO` y `NOMENCL`;
- precedencia: `RES` explícito > observación humana revisada > acción del motor;
- una observación aislada no crea proyecto sin `RES` o acción;
- una fila visible por **tribunal + RIT + tipo**;
- varias personas de la misma causa/tipo se integran en un solo proyecto, cada una junto a su cédula;
- un único archivo Word, cada proyecto comienza en página nueva;
- fechas judiciales generadas íntegramente en palabras;
- datos faltantes se marcan para completar, no se inventan;
- el cambio manual de tipo afecta únicamente filas seleccionadas.

### 5.13 Matrices de resolución

Inventario vigente:

- Laja: `NOMENCL`, `PC_IE`, `PC_INFO`.
- Mulchén: `NOMENCL`, `PC_IE`, `PC_INFO`.
- Tomé: sin matrices fuente incorporadas.

Cinco cuerpos recibidos se gestionan mediante payloads verificados por SHA-256; `LAJA/PC_INFO` se incorporó desde el Word antiguo revisado. Las matrices se copian al directorio del usuario y pueden ser perfeccionadas en Word sin cambiar Python.

### 5.14 FAS y modalidades

Se corrigió la clasificación para que FAS se trate como **Familia de acogida**. La selección de modalidades en Correos es explícita y conserva redacción semántica del alcance.

### 5.15 Cumplimiento sin cruce

La versión previa podía bloquear o exigir documentar una excepción.

**Solución vigente:** si no existe cruce utilizable, C-10 no se evalúa, se presenta una advertencia y el resto del análisis/exportación continúa. No se inventa C-10 y no se obliga a un diálogo de excepción.

### 5.16 Planilla revisada o externa

El usuario puede cargar una planilla modificada/externa sin volver a ejecutar todo el motor. Se detectan encabezados, se respeta el modo elegido, se recuperan reglas desde `NURUS_REGLAS` o trazabilidad cuando corresponde y se preserva la identidad del registro.

### 5.17 Revisión humana y exportación

Un problema detectado el 16-09 era que una edición visible podía no reflejarse en la exportación si no se pulsaba primero un botón intermedio; además podía borrarse TT/CC/RES no tocado.

**Solución:**

- se captura la edición visible antes de exportar;
- fila modificada: `REVISADO`;
- fila no modificada: `PENDIENTE`;
- valores fuente no editados se preservan;
- observaciones que podrían interpretarse como fórmulas de Excel se neutralizan.

### 5.18 Fechas y ceros en adjuntos

Se corrigió el formato operacional de planillas adjuntas: fechas como `dd/mm/aaaa` sin `00:00:00`, y valores cero sin convertirlos accidentalmente en celdas vacías.

### 5.19 Código transitorio

Durante correcciones rápidas del 16-09 se utilizaron módulos `runtime_fixes_20260916*` mediante monkey-patching. La auditoría posterior concluyó que mantenerlos duplicaba comportamiento y hacía dependiente el orden de importación.

**Solución:** integrar las correcciones en `work.py`, `outputs.py`, `resolutions.py`, `app.py` y `services/exports.py`; eliminar la capa transitoria y agregar regresiones que impiden su reaparición.

---

## 6. Arquitectura vigente

### 6.1 Flujo visible

```text
Excel RUS
   ↓
Trabajo: Espera / Cumplimiento / Informes
   ↓
copia preservada + propuestas
   ↓
revisión humana
   ├── Correos → borradores Outlook
   ├── Resoluciones → Word
   └── Enviados → consulta de solo lectura
```

La sesión compartida conserva la relación entre origen, copia revisable y salidas. La complejidad interna necesaria para evitar asociaciones incorrectas permanece fuera del flujo normal.

### 6.2 Separación regla / texto / acción

La aplicación ya no debe decidir un correo o proyecto mediante búsquedas de frases en la observación.

Conceptualmente cada hallazgo mantiene:

- identificador de regla;
- texto de observación;
- acciones estructuradas;
- advertencias.

Esto permite mejorar el texto sin romper la generación de correos o resoluciones.

### 6.3 Configuración local

La configuración se guarda bajo `LOCALAPPDATA/CSMP_Personal`. Las escrituras son atómicas y se mantiene respaldo cuando corresponde. Una actualización no debe sobrescribir silenciosamente personalizaciones de textos, contactos o matrices.

---

## 7. Estado de pruebas y verificación

### 7.0 CI vigente dev9

GitHub Actions run **35646164874**, commit `c262321e227302cc9349061a941c06af29879219`: **success** en Windows con Python 3.12, 3.13 y 3.14; **254 passed, 1 skipped** en cada versión. Python 3.12 aprobó además el smoke de la interfaz CustomTkinter a 1024×650, capturas de Correos/Configuración y la distribución `CSMP-Windows-dev9`. Esta evidencia automatizada no sustituye la aceptación Office institucional.

### 7.1 CI vigente al inicio de esta revisión

GitHub Actions run **35127052431**, HEAD `0523f3cb666a7e55b6a4fd64eb92949c89a58e26`:

- Windows / Python 3.12: **success**.
- Windows / Python 3.13: **success**.
- Windows / Python 3.14: **success**.
- Python 3.12: **230 passed, 1 skipped**.
- smoke GUI: success.
- wheel: construido y comprobado fuera del árbol fuente.
- distribución `CSMP-Windows-dev6`: construida.
- recursos: cinco payloads de matrices y seis matrices base verificados.
- dependencias, compilación e higiene: success.

Esta evidencia acredita coherencia automática del repositorio, **no** aceptación institucional de Office ni corrección jurídica de cada salida.

### 7.2 Aceptación pendiente

`docs/ACEPTACION_CSMP_PERSONAL.md` contiene doce pruebas ACEP que siguen pendientes de registro formal en el equipo institucional.

La experiencia anterior del usuario sí aporta evidencia práctica de que Python 3.14 y la corrección de exportación Excel/COM del prototipo funcionan en su equipo, pero esa prueba no sustituye la aceptación de la release personal dev6 completa.

---


### 7.3 Cierre remoto de esta consolidación

El contrato documental se verificó en el commit `574955b5e63608ff7bcf16db29ee36d859a59385`, GitHub Actions run `35535124753`: **success** en Windows con Python 3.12, 3.13 y 3.14. En Python 3.12 se obtuvieron **230 passed, 1 skipped**, smoke GUI correcto y distribución `CSMP-Windows-dev6` generada. Los commits posteriores de este cierre solo actualizan documentación y se marcan `[skip ci]`; no alteran runtime ni suite.


### 7.4 Higiene adicional del árbol actual

Se recorrieron los **87 archivos Python** versionados del árbol final buscando rutas de riesgo o deuda evidente: llamadas `.Send(`, marcadores `TODO`/`FIXME`, rutas personales rígidas `C:\\Users\\...`, `shell=True`, `eval(`/`exec(` y restos ejecutables `runtime_fixes_`. No se encontraron esas rutas en código productivo. Las únicas referencias a `runtime_fixes_` permanecen en el contrato de release que comprueba que dichos módulos no existan. Esta revisión es una comprobación de higiene dirigida; no sustituye análisis estático especializado ni aceptación funcional.

---

## 8. Conflictos y límites abiertos

### C01 — Regla DCE en Informes

El Manual de Funciones CSMP 2025 disponible indica pedir cuenta respecto de informes DCE pendientes de entrega de **más de 40 días**. El motor vigente clasifica `I01_VENCIDO_DCE` cuando el informe DCE está vencido.

No debe modificarse por inferencia. Se requiere cerrar cuál es la instrucción operativa vigente y después actualizar regla, matriz documental y regresiones en conjunto.

### C02 — Matrices de Tomé

No existen matrices fuente incorporadas para Tomé. El sistema no debe reciclar automáticamente una plantilla de Laja o Mulchén. Se necesitan los documentos efectivos o una decisión explícita de que determinada gestión no requiere matriz en el alcance actual.

### C03 — Aprobación sustantiva de matrices

La integridad técnica y formato pueden verificarse automáticamente, pero el contenido de un proyecto concreto requiere revisión humana/jurídica.

### L01 — Office institucional

CI usa runners Windows modernos. Queda pendiente ejecutar y registrar Excel 2010/Outlook clásico, cuenta, firma, rutas y restricciones del PC institucional.

### L02 — Beneficio real de tiempo

Todavía falta medir el mismo ciclo con:

- herramientas anteriores;
- CSMP Assistant dev9;

registrando tiempo, clics, recargas y correcciones. Esta comparación es el criterio final para decidir adopción.

---

## 9. Qué no conviene volver a hacer

- No reconstruir otra interfaz desde cero.
- No volver a exponer snapshots, materialización o estados técnicos como pasos obligatorios.
- No optimizar por volumen masivo sin un caso real.
- No duplicar contactos, textos o matrices como varias fuentes de verdad.
- No inferir reglas operativas a partir de una redacción.
- No reabrir decisiones cerradas sin nueva evidencia.
- No declarar productiva una función porque sus tests unitarios pasan.
- No fusionar una matriz judicial de otro tribunal para “completar” un faltante.

---

## 10. Próximos pasos recomendados

### P0 — Aceptación institucional

Ejecutar ACEP-01 a ACEP-12 en Windows/Excel/Outlook institucional con copias autorizadas y registrar resultados directamente en `ACEPTACION_CSMP_PERSONAL.md`.

Criterio de cierre: ningún fallo material sin explicación/reproducción.

### P0 — Resolver DCE

Confirmar la regla operativa vigente de informes DCE (>40 días vs vencimiento inmediato). Una vez resuelta:

1. modificar código;
2. actualizar `REFERENCIA_CATALOGO_ASISTENTE.md`;
3. actualizar `MATRIZ_REGLAS_PERSONAL.json`;
4. agregar/ajustar pruebas;
5. documentar origen de la decisión.

### P0 — Matrices Tomé

Obtener las matrices vigentes o cerrar expresamente qué productos no se generarán para Tomé. No crear sustitutos automáticos.

### P1 — Medición comparada de usabilidad

Usar un lote real representativo de 100–500 registros y comparar con las herramientas anteriores:

- tiempo hasta Excel;
- tiempo hasta Word;
- tiempo hasta borradores;
- cantidad de selecciones de archivo;
- cantidad de correcciones manuales;
- errores/incidencias.

El producto solo debe adoptarse si reduce materialmente fricción manteniendo o mejorando calidad.

### P1 — Pulido de observaciones con resultados reales

Reunir ejemplos finales ya corregidos por el usuario y comparar propuesta del motor vs texto finalmente registrado. Ajustar únicamente patrones recurrentes, manteniendo las reglas separadas del texto.

### P1 — Revisión de plantillas y contactos

- validar matrices Word con los tribunales/formatos reales;
- completar contactos faltantes;
- revisar alias y destinatarios obsoletos;
- mantener una sola fuente configurable por usuario.

### P2 — Release candidata

Después de aceptación:

- decidir si `0.4.0.dev9` avanza a `0.4.0rc1` o versión estable;
- crear tag/release;
- conservar distribución Windows reproducible;
- congelar documentación de la release;
- no eliminar ramas históricas hasta confirmar que ya no se necesitan para recuperación.

### P2 — Simplificación posterior

Solo después de aceptación y uso real:

- evaluar qué componentes del núcleo NuRus histórico pueden marcarse como deprecados;
- reducir duplicación documental;
- eventualmente retirar entradas antiguas, pero únicamente con contrato de migración y sin perder trazabilidad.

---

## 11. Estado de ramas al cierre previsto

La política recomendada es:

- `main`: rama por defecto; al cierre se sincroniza por fast-forward con el checkpoint documental de `csmp-personal-2026-09-13`.
- `csmp-personal-2026-09-13`: rama de desarrollo que conserva el historial del producto personal.
- `implementacion-plan-2026-09-08`: referencia del prototipo dev7.
- `revision-producto-csmp-2026-09-13`: referencia de la auditoría que motivó la migración selectiva.

No borrar las ramas históricas en esta fase.

---

## 12. Estado operativo comprimido

```text
[G-OBJETIVO]
Una herramienta personal CSMP que sea más rápida y de mayor calidad que
RUS Engine + scripts separados, sin perder revisión humana.

[G-REGLAS / INVARIANTES]
- Windows.
- No enviar correos; solo borradores.
- Para vacío permitido.
- CC institucional obligatoria.
- No escribir en RUS/SATURNO.
- Original Excel intacto.
- No inventar reglas, contactos o matrices.

[G-DECISIONES]
D01. Producto principal: CSMP Assistant personal.
D02. NuRus dev7: referencia técnica, no UX final.
D03. Una carga de trabajo compartida por Excel/Correos/Resoluciones.
D04. Regla, texto y acción desacoplados.
D05. Configuración de dominio editable; lógica técnica no.
D06. Resoluciones agrupadas por tribunal/RIT/tipo en un Word.
D07. Revisión humana por excepción, sin aprobación fila por fila.

[G-ESTADO]
Versión: 0.4.0.dev11.
CI vigente: run 35729404913, Windows 3.12/3.13/3.14 verde;
256 passed, 1 skipped por versión; smoke CustomTkinter y construcción de distribución Windows OK en 3.12.
Código transitorio e interfaz NuRus paralela eliminados.
Excel preservado, correos con alcance programa/tribunal, Word, configuración y Enviados integrados.

[L-PENDIENTE]
- aceptación Office institucional;
- regla DCE;
- matrices Tomé;
- medición comparada del ciclo;
- aprobación sustantiva de matrices.

[L-SIGUIENTE]
Ejecutar ACEP-01 a ACEP-12 y resolver C01 antes de declarar release candidata.
```

---

## 13. Criterio final de éxito

El proyecto no se considera terminado por cantidad de tests, módulos o trazabilidad.

Se considera exitoso cuando, para un ciclo real:

1. requiere menos pasos que las herramientas anteriores;
2. reduce el tiempo total;
3. no exige volver a seleccionar archivos generados por la propia aplicación;
4. produce observaciones que requieren menos corrección;
5. produce proyectos Word de calidad al menos equivalente;
6. produce todos los borradores necesarios sin enviar ninguno;
7. permite mantener textos, umbrales razonables, contactos y plantillas sin modificar Python;
8. conserva evidencia suficiente para detectar errores sin convertir esa evidencia en trabajo adicional para el usuario.
