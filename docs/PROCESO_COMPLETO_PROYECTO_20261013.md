# Proceso completo del proyecto — Descargador SITFA y Asistente CSMP

13 de octubre de 2026. Rama única `feat/ejecucion-plan-20261008`, commit `622ed4f`.

Este documento explica qué hace el sistema, con qué materia prima trabaja, cómo la procesa, qué entrega,
cómo se ejecutó el proyecto, qué problemas aparecieron y cómo se resolvieron, y en qué estado está hoy.

Las afirmaciones están marcadas: **N1** hecho verificable en el repositorio o en una medición; **N2** inferencia
con su supuesto; **N3** especulación. Sin marca, es N1.

---

## 1. Qué es el sistema

Dos aplicaciones de escritorio independientes para el trabajo de seguimiento de medidas de protección en el
Juzgado de Laja, Mulchén y Tomé.

| | Descargador SITFA | Asistente CSMP |
| --- | --- | --- |
| Dónde vive | `descargador/` | `src/nurus/` |
| Qué hace | Lee SITFA con la sesión del usuario y arma un paquete | Lee el paquete y prepara los productos de trabajo |
| Interfaz | Tkinter + extensión Chrome 2.7.6 | Tkinter |
| Red | Solo SITFA, con la sesión ya abierta del usuario | Ninguna |
| Escribe en SITFA/RUS | No, nunca | No, nunca |

**Separación deliberada.** El Asistente no navega SITFA ni inicia el Descargador. El Descargador no interpreta
reglas. El único punto de contacto es un archivo: `Captura_CSMP.zip`. Esto permite que el Asistente funcione sin
Chrome, sin extensión y sin sesión institucional, y que un fallo de red nunca contamine los productos.

**Versiones:** Asistente `0.5.0.dev5`, Descargador/extensión `2.7.6`, contrato de intercambio `CSMP_CAPTURE 1.0.0`
(`1.1.0` cuando incluye el XLSX consolidado).

---

## 2. Materia prima: qué nutre los productos

### 2.1 Fuente primaria — SITFA

El Descargador no raspa HTML a ciegas. Usa la **extensión Chrome** sobre la sesión que el usuario ya abrió, y
captura tres cosas distintas:

1. **Excel de consulta.** Lo que SITFA entrega al consultar Seguimiento por tribunal, modalidad, pestaña
   (Espera / Cumplimiento / Informes / Egresados) y período. Es el archivo original, sin tocar.
2. **Vínculos remotos.** De cada fila se extraen los identificadores que SITFA pone en sus acciones
   (`ShowObservaciones`, `ShowHistoria`, `ShowInformesProgramados`): `causa_id`, `ingreso_id`, `etapa`,
   `persona_id`, `centro_id`. Son la identidad remota verdadera, no el RIT ni el nombre.
3. **Bitácoras.** Para cada ingreso con vínculo comprobado, la ventana de observaciones del centro
   (`IrPopUpInformesAccion.do`). Se guarda el HTML íntegro con su SHA-256.

**Regla que define todo el diseño:** el emparejamiento entre una bitácora y un registro se hace **solo** por
`(tribunal, causa, ingreso)`. Nunca por RIT, nombre ni RUT. Un ingreso sin vínculo comprobado queda
`SIN_VINCULO` con diagnóstico, y eso **no** significa «sin actuaciones».

### 2.2 Fuente secundaria — datos locales del usuario

- **Contactos** (`contactos.json` en el directorio del usuario, nunca en el paquete ni en el repositorio).
  Alimenta destinatarios de correo y los destinatarios por tribunal.
- **Configuración** (`configuracion.json`): textos, plazos, alias de programas, CC fijo.
- **Matrices Word** (`plantillas_word/`): 6 plantillas, 2 tribunales × 3 tipos.

### 2.3 Fuente de reglas — el motor tradicional

`src/nurus/personal/motor/` y `textos_base.json`. Codifica los criterios que el usuario ya aplicaba a mano:
reglas de Espera, de Cumplimiento y de Informes, con sus umbrales y sus textos de observación.

**Este módulo es intocable.** Ninguna tarea puede cambiar reglas, umbrales ni textos sin decisión expresa del
usuario, y toda modificación debe conservar la paridad golden (17 productos de referencia byte a byte).

---

## 3. Cómo se procesa: el recorrido completo

```
SITFA ──► Descargador ──► Captura_CSMP.zip ──► Asistente ──► productos
          (3 fases)        (contrato)          (5 etapas)
```

### 3.1 Descargador — tres fases

**Fase 1 · Consultar.** Ejecuta las consultas del alcance elegido. Cada una produce un Excel original, un
recuento de páginas esperadas vs. guardadas, y un estado: `VALIDADA`, `SIN_RESULTADOS`, `INCOMPLETA` o fallida.
Entre consultas hay una pausa de 1 s (`PAUSA_SITFA_S`) para no cargar el servidor.

**Fase 2 · Leer bitácoras.** Para cada vínculo comprobado abre su ventana de observaciones. Controles:

- Tope de **600 aperturas por lote** (`MAX_LECTURAS_POR_LOTE`; eran 300 hasta la decisión m del 13-oct).
- Punto de guardado cada 25 lecturas, para que «Retomar» no repita trabajo.
- **Reutilización** (EF-04, WP-93): si otra carpeta del mismo destino leyó hoy esa misma bitácora, hace menos de
  12 horas, con idéntica identidad y parámetros, y el archivo conserva su SHA-256, se copia en vez de volver a
  abrir SITFA. Queda anotado de qué carpeta vino.
- Cada HTML se verifica contra el ingreso solicitado: si la respuesta pertenece a otro, se descarta.

**Fase 3 · Exportar paquete.** Arma `Captura_CSMP.zip` y deja los intermedios en `respaldo/`.

### 3.2 El contrato de captura — la pieza central

El ZIP no es una carpeta de archivos sueltos. Es un formato con esquema fijo (`src/nurus/capture/contract.py`),
7 tablas CSV y un manifiesto:

| Tabla | Columnas | Qué contiene |
| --- | --- | --- |
| `consultas` | 18 | Una fila por consulta: alcance, cobertura, páginas, identidad remota si aplica |
| `registros` | 24 | Una fila por ingreso: identidad, tipo, JSON normalizado y JSON crudo |
| `bitacoras` | 31 | Una fila por entrada de observación, con fecha, origen, autor y texto |
| `calendarios` | 26 | Informes programados |
| `historia_tramites` | 30 | Trámites (fuera de alcance por ahora) |
| `documentos` | 27 | Documentos asociados |
| `incidencias` | 9 | Todo lo que salió distinto de lo esperado |

Más 19 columnas comunes (`COMMON`) presentes en todas: `capture_id`, `query_id`, identidad remota, procedencia
(archivo, hoja, fila, SHA-256) y cobertura.

**Por qué importa.** Cada dato del Asistente puede rastrearse hasta el archivo original, la fila y el hash que lo
produjo. Y cada ausencia tiene su incidencia: nada desaparece en silencio.

### 3.3 Asistente — cinco etapas

**Etapa 1 · Importar.** Valida el ZIP contra el contrato, calcula la cobertura por tabla y por ingreso, y
construye la sesión de trabajo. Si ya había una captura anterior, **concilia**: compara fila a fila y conserva
las ediciones humanas.

**Etapa 2 · Analizar.** El motor tradicional evalúa cada registro y produce observación, reglas aplicadas y
acciones propuestas. En paralelo, la **cronología solo mostrar** (WP-50, decisión D5) ordena las entradas de
bitácora de los últimos 4 meses y determina la última observación atribuible al centro, las posteriores y los
empates de orden incierto. La cronología **no genera hallazgos ni cambia observaciones**: solo muestra.

**Etapa 3 · Revisar.** El usuario filtra, corrige observaciones y marca decisiones. Desde la Fase A (WP-95..97):

- **Comparación de capturas:** cada fila se clasifica frente a la captura anterior como `NUEVO`,
  `CAMBIO_FUENTE`, `CAMBIO_REGLAS` o `SIN_CAMBIO`; los que salieron quedan como egresados.
- **Filtro «Solo cambios»** y botón **«Ver egresados (N)»**.
- **Bandeja «Qué requiere mi atención (N)»**, ordenada por urgencia.
- **Métrica local opcional** (solo conteos, apagada por defecto).

**Etapa 4 · Producir.** Genera los productos del punto 4.

**Etapa 5 · Resultados.** Consolida productos, incidencias, hechos y fuentes.

---

## 4. Qué entrega: los productos

### 4.1 Del Descargador

| Producto | Qué es |
| --- | --- |
| `Captura_CSMP.zip` | El paquete del contrato. Lo único que necesita el Asistente |
| Excel originales | Tal como los entregó SITFA, en `respaldo/originales/` |
| `Bitacoras_RUS.xlsx` | Copia íntegra de todas las entradas, sin ventana de 4 meses ni truncamiento |
| `bitacoras.json` + HTML | Control de lecturas con SHA-256 de cada copia |
| `diagnostico_bitacoras.json` | Diagnóstico anónimo: conteos, estados, códigos. Sin datos personales |
| XLSX consolidado | Opcional, dentro del ZIP (D6) |
| PDF por consulta | Con sus filtros y fechas; un PDF vacío muestra qué se consultó |
| ICS | Calendario de vencimientos |

### 4.2 Del Asistente

| Producto | Qué es |
| --- | --- |
| **Copia Excel del trabajo** | Se genera automáticamente al importar. Hoja operativa con observaciones del motor |
| **Borradores de correo** | En Outlook, con `mail.Save()`. **Nunca `.Send()`**. CC fijo `ucc_concepcion@pjud.cl` |
| **Nóminas adjuntas** | Un Excel por correo, con los casos de ese programa. Editable antes de guardar |
| **Resoluciones Word** | Desde las 6 matrices, con las variables resueltas |
| **Vista PDF** | De las resoluciones, vía Word |
| **Resumen de resultados** | Productos, incidencias, hechos y fuentes |

### 4.3 Garantías de los productos

- **Sin fórmulas inyectables:** todo valor exportado se fuerza a texto (`_write_text`).
- **Exportación atómica:** si falla el registro posterior, se restaura el estado y se borra el archivo.
- **Diario de recibos** (EF-03, WP-92): antes de guardar borradores se persiste un diario; si el proceso muere a
  mitad del lote, al reabrir se sabe qué quedó guardado y no se duplica.
- **Paridad golden:** 17 productos de referencia se comparan byte a byte en cada verificación.

---

## 5. Cómo se ejecutó el proyecto

### 5.1 Método

El trabajo se dividió en **paquetes de trabajo (WP)** numerados, cada uno con:

- una **ficha** (`docs/implementacion/wp/WP-NN.md`): objetivo, contrato, archivos y pruebas;
- un **estado estructurado** (`docs/implementacion/estado/WP-NN.json`): estado, dependencias, verificación, revisión;
- una **medición** (`docs/implementacion/verificacion/WP-NN.json`).

`tools/estado.py` agrega las fichas en `ESTADO.json` y genera el bloque de estado de `CONTINUAR.md`. Nadie
escribe esas cifras a mano: se generan, y `estado.py validar` rechaza las incoherencias (por ejemplo, un WP
integrado que depende de otro que no lo está).

### 5.2 La puerta de calidad

`tools/verificar_wp.py` decide PASA/FALLA con nueve controles contra `linea_base.json`:

1. Suite del Asistente — no puede bajar del mínimo
2. Suite del Descargador — igual
3. Inventario de pruebas — ninguna prueba desaparece sin declararlo
4. Identidades reales — **límite 0**
5. Paridad golden — 17 productos intactos
6. Restricciones absolutas — sin `.Send(`, sin escritura RUS, sin credenciales
7. Ruff E9/F63/F7/F82/F401/F841 — 0
8. Módulos no alcanzables — 0
9. Enlaces rotos en documentación — 0

El control 3 es el que más veces evitó un retroceso: impide «arreglar» una prueba borrándola.

### 5.3 Revisión por segunda sesión (E3)

Al principio todo cambio pasaba por una sesión revisora independiente. Resultó caro. Desde la **decisión j**
(12-oct), E3 es obligatorio solo si el diff toca: `motor/`, `textos_base.json`, Outlook/borradores, el contrato de
captura, borrado o retención de datos, el anonimizador / `check_real_ids`, o la extensión SITFA. El resto se
integra con `verificar_wp.py` PASA.

### 5.4 Cifras

| | |
| --- | --- |
| Período | 8 a 13 de octubre de 2026 (6 días) |
| Commits | 458 |
| Líneas de Python | ~39.300 (`src`, `descargador`, `tools`) |
| Archivos de prueba | 92 del Asistente + 32 del Descargador |
| Pruebas | 932 del Asistente (11 omitidas, 6 xfail) + 283 del Descargador (62 subpruebas) |
| Paquetes de trabajo | 55 definidos, 51 integrados |

---

## 6. Problemas y cómo se resolvieron

Cada caso sigue el mismo formato: síntoma, qué descarté, causa raíz, solución, prevención.

---

### P1 · Datos personales reales en el repositorio

**Síntoma.** El repositorio y su historial contenían RUT, nombres y archivos reales (HAR, Excel, capturas).

**Razonamiento.**
1. Primero medir: escribí `tools/check_real_ids.py`, que detecta RUT con dígito verificador válido. No basta con
   buscar un patrón: un número cualquiera con formato de RUT casi siempre falla el dígito verificador, así que
   validarlo reduce los falsos positivos a casi cero.
2. El detector tenía que mirar **dentro** de los contenedores, no solo el texto plano: ZIP, XLSX (que es un ZIP),
   XML interno, y archivos sin extensión conocida. Si un formato no se puede inspeccionar (gzip, UTF-16 sin BOM),
   el control debe **fallar cerrado** y decir «control incompleto», nunca dar por limpio lo que no leyó.
3. Reemplazar los datos reales por sintéticos en todas las pruebas (WP-10).
4. Conectar el detector al hook de pre-commit y a la CI, con límite 0 (WP-11).
5. Reescribir el historial para eliminar lo ya subido (WP-17, decisión D1 del usuario): 35 ramas con
   `force-with-lease` autorizado, 607 artefactos y 2 etiquetas borradas.
6. Para los casos en que igual hace falta compartir estructura, escribir extractores anónimos:
   `extraer_estructura_har.py`, `anonimizar_excel.py`, `diagnostico_bitacoras.json`.

**Estado.** Control en 0 hallazgos, incluido `--historia` sobre todos los blobs alcanzables. **Pendiente:**
referencias de pull requests y cachés antiguas de GitHub, que solo Soporte GitHub puede eliminar (WP-17).

**Prevención.** Regla absoluta: ningún archivo real entra al repositorio ni a una conversación. Si alguien ofrece
uno, se pide el extracto anónimo.

---

### P2 · El trabajo quedó disperso en 45 ramas

**Síntoma.** Paquetes implementados en ramas separadas, algunas con trabajo ya contenido en otras. Imposible
saber qué estaba realmente integrado.

**Razonamiento.**
1. No fusionar a ciegas. Primero inventariar: para cada rama, ¿sus commits ya son ancestros de `feat`?
2. Las que sí: unirlas sin cambiar contenido, solo para preservar el historial.
3. Las que no: fusionar de verdad, resolviendo conflictos uno por uno y midiendo después de cada una.
4. Dejar el inventario con SHA en `ramas_consolidadas_20261011.json`.

**Estado.** Consolidado en WP-85 (45 ramas, 11-oct) y nuevamente el 13-oct por decisión n. Hoy el remoto tiene
**solo `feat/ejecucion-plan-20261008` y `main`**.

**Lección.** Una rama por paquete tiene sentido cuando hay revisión independiente. Cuando el ciclo es
diagnóstico-corrección rápido con el usuario probando, las ramas solo agregan trabajo de fusión. De ahí la
decisión n.

---

### P3 · Conflictos repetidos en los archivos de estado

**Síntoma.** Cada fusión chocaba en `global.json` (`orden_paquetes`), `ESTADO.json` y `CONTINUAR.md`.

**Razonamiento.** Son archivos **generados**, no fuente. El conflicto es artificial.

**Solución.** Resolver siempre tomando una versión y **regenerar** con `tools/estado.py agregar` +
`continuacion`. En `orden_paquetes`, conservar ambas entradas. El validador confirma el resultado.

---

### P4 · Siete agentes en paralelo agotaron el límite de uso

**Síntoma.** Lancé siete subagentes simultáneos, cada uno en su worktree. Todos cayeron con error 429.

**Razonamiento.** El paralelismo tiene un techo que no depende del diseño del trabajo sino de la cuota. Además,
los paquetes compartían archivos, así que la integración iba a ser conflictiva de todos modos.

**Solución.** Worktrees eliminados; ese trabajo lo hizo otro programador. Para adelante: un agente por paquete,
**sin solapamiento de archivos**, integrando de a uno. Queda escrito en `PROMPT_CONTINUAR_IA.md`.

---

### P5 · Una revisión de código analizó un commit obsoleto

**Síntoma.** Se ejecutó `/code-review` y entregó 20 hallazgos. El commit analizado (`0b3990d`) era de varios días
antes.

**Razonamiento.**
1. Antes de corregir nada, verificar cada hallazgo **contra el código actual**. Corregir algo ya corregido
   introduce riesgo sin beneficio.
2. Resultado: 16 de 20 ya estaban resueltos en `feat`.
3. De los 4 restantes: 2 eran comportamiento por diseño (los contactos viven en el directorio del usuario, WP-12,
   y las pruebas lo aíslan con `LOCALAPPDATA`); 2 eran reales y menores, y se corrigieron.

**Lección.** Una revisión sin SHA es una revisión sin contexto. Toda revisión declara el commit que miró.

---

### P6 · Una prueba de interfaz fallaba de forma intermitente en Windows

**Síntoma.** `test_ui_theme` fallaba en Windows 3.12 y 3.13, y pasaba en Linux y al reintentar.

**Razonamiento.** El foco de ventana en Windows no es inmediato: la prueba verificaba el foco antes de que el
gestor de ventanas lo asignara. No es un defecto del producto.

**Solución.** Bucle de reintento de foco en la prueba (`b63070d`). La reconstrucción pasó.

**Regla que me impuse.** Nunca saltar, desactivar ni poner en cuarentena una prueba para llegar a verde. Si una
prueba es frágil, se hace robusta; si falla de verdad, se arregla el producto.

---

### P7 · «Sin entradas capturadas» en registros que sí tenían observaciones

Este es el problema más instructivo del proyecto, porque **mis dos primeras explicaciones fueron equivocadas** y
el usuario las refutó con datos.

**Síntoma.** El usuario probó la versión real y reportó que muchas filas mostraban «Sin entradas capturadas» en
la columna «Última obs. centro», aunque las bitácoras sí se habían descargado.

**Hipótesis 1 — la ventana de 4 meses.** La cronología solo mira los 4 meses previos a la fecha de corte
(decisión D5). Si todas las entradas fueran anteriores, el estado sería `SIN_ENTRADAS_CAPTURADAS` y el texto
resultaría engañoso.
→ **Corregí el texto** (`8415c0a`): ahora dice «Sin entradas en 4 meses · N anteriores».
→ **El usuario refutó:** «las observaciones son de menos de 4 meses; X-97-2026 tiene una de septiembre».

**Hipótesis 2 — casos en espera sin anotaciones.** Supuse que en registros «a la espera de ingreso» el centro no
anota por meses.
→ **El usuario refutó:** «imposible, aunque estén sin ingreso deberían tener una observación de a lo menos
2 meses». Tenía razón: era una suposición mía sobre su dominio, no un dato.

**Cambio de método.** Dejé de proponer explicaciones y pedí evidencia que distinguiera entre causas. Diseñé una
prueba de tres pasos donde **cada resultado posible apuntaba a una causa distinta**: si el HTML no tiene la
observación, el fallo está en la descarga; si la tiene pero el paquete no, está en la exportación; si el paquete
la tiene pero el Asistente no, está en la importación.

El usuario envió una captura de RUS (dos observaciones: 24/09 y 28/09), la pantalla del Asistente (período
13/06–13/10, sin ninguna línea de entradas descartadas) y los CSV del paquete.

**Análisis de los CSV** (solo conteos; no copié su contenido a ningún archivo):

| Dato | Valor |
| --- | --- |
| Registros | 344 (304 Cumplimiento, 40 Espera), todos con identidad VERIFICADA |
| Consultas de bitácora en el paquete | 301 — 300 leídas, 1 FALLIDA |
| Registros **sin ninguna consulta** | 43 |
| Incidencia decisiva | `diary-300`: «se alcanzó el máximo de lecturas del lote» |

**Causa raíz A.** La lectura se detuvo en `MAX_LECTURAS_POR_LOTE = 300`. X-97-2026 era uno de los 43 ingresos
que nunca se consultaron. El diagnóstico del usuario decía 344 leídas porque correspondía a una lectura
posterior, pero el paquete se creó antes de retomarla.

**El defecto real del Asistente.** No era la ventana de 4 meses: era que el Asistente **no distinguía entre
«leímos y no hay nada» y «no alcanzamos a leer»**, y mostraba el mismo texto para ambos. Eso es una afirmación
falsa sobre la realidad, justo lo que el proyecto no puede permitirse.

**Solución** (`0ef6b26`). `attach_displays` ahora lee, de las consultas de bitácora del paquete, cuáles ingresos
tuvieron lectura. Sin consulta o con consulta fallida → estado nuevo `NO_LEIDA` → **«Bitácora no leída»**, con
indicación de usar «Retomar». `DISPLAY_VERSION` subió a `solo-mostrar-3` para que las sesiones ya guardadas se
recalculen. Un paquete antiguo sin esas consultas conserva el comportamiento anterior: no se inventa un dato que
el paquete no trae.

**Causa raíz B — el tope era inadecuado.** Con 344 registros habituales, cada corrida se cortaba.
→ El usuario autorizó subirlo a **600** (`a98971f`, decisión m). La pausa entre lecturas y los puntos de guardado
no cambiaron: el tope protege a SITFA de un lote desbocado, no de un uso normal.

---

### P8 · Bitácoras de Espera leídas pero vacías (abierto)

**Síntoma.** Al separar los datos por tipo apareció un segundo patrón, distinto del anterior: 36 de 40 bitácoras
de Espera se leyeron **correctamente** y vinieron con 0 entradas. Lo mismo en 8 de Cumplimiento.

**Hipótesis 3 — SITFA devuelve la ventana según la consulta activa.** Si las bitácoras de Espera se abren con la
sesión posicionada en Cumplimiento, podrían llegar vacías.
→ **El usuario refutó:** abrió el HTML guardado y sí tiene observaciones.

Tres hipótesis mías, tres refutadas por evidencia del usuario. Eso reorientó el trabajo: si el HTML guardado
tiene las observaciones y el lector devolvió 0 sin error, **el defecto está en el lector**, no en la descarga ni
en SITFA.

**Razonamiento sobre el lector.** `read_snapshot` busca `<table id="TablaInforme">` y lee sus filas. Revisé qué
estructura haría que esa tabla se viera solo con la cabecera: si el HTML cierra la tabla antes de tiempo, el
analizador deja las filas y sus campos ocultos **fuera** del árbol de la tabla. El lector entonces encuentra
cabecera válida, cero filas, y ningún error.

**Lo que hice** (`f363a4d`), sin tener aún el HTML real:

1. **Cerrar el silencio.** Si hay observaciones o campos `HOBS_*` fuera de `TablaInforme`, el lector ahora falla
   con «Hay observaciones fuera de la tabla de bitácora». Esa lectura queda FALLIDA y el Asistente muestra
   «Bitácora no leída». Aunque no sepa aún la causa exacta, **el sistema ya no afirma que no hay observaciones
   cuando no pudo leerlas**. Esa era la parte urgente.
2. **Construir el instrumento.** `descargador/extraer_estructura_bitacora.py` resume una copia HTML en conteos y
   códigos: cuántas tablas, cuántas filas dentro y fuera, cuántos campos ocultos, qué devuelve el lector. Sin
   textos, sin nombres, sin RIT, sin identificadores. Una prueba verifica que ningún dato del fixture aparezca en
   la salida.

**Por qué un instrumento y no pedir el archivo.** La regla del proyecto prohíbe que un HTML real salga del PC del
usuario. En vez de pedir una excepción, construí la herramienta que entrega exactamente lo que necesito para
diagnosticar y nada más.

**Estado: en espera por decisión del usuario.** Falta que entregue `estructura.json` de una bitácora de Espera y
una de Cumplimiento. **N2:** la causa es una tabla cortada en el HTML de Espera; supuesto: el patrón distingue
Espera de Cumplimiento. Sin el archivo de estructura no es confirmable.

---

## 7. Estado actual del proyecto

### 7.1 Repositorio

| | |
| --- | --- |
| Rama única de trabajo | `feat/ejecucion-plan-20261008` |
| Commit | `622ed4f` (13 de octubre de 2026) |
| Ramas en el remoto | Solo `feat` y `main` |
| `main` | Intacta. No se fusiona sin acta final (D7) |

### 7.2 Medición vigente

Línea base sobre `bbf8cdd`:

| Control | Resultado |
| --- | --- |
| Suite Asistente | **932** aprobadas, 11 omitidas, 6 xfail, 0 fallidas |
| Suite Descargador | **283** aprobadas, 62 subpruebas, 0 fallidas |
| Identidades reales | **0** |
| Paridad golden | **PASA** (17 productos) |
| Restricciones absolutas | **PASA** |
| Ruff / módulos no alcanzables / enlaces rotos | **0 / 0 / 0** |
| Veredicto | **PASA** |

### 7.3 Paquetes de trabajo

**51 de 55 integrados.** Los 4 restantes no dependen de programación:

| WP | Estado | Qué falta | De quién depende |
| --- | --- | --- | --- |
| WP-02 | BLOQUEADO_USUARIO | Acta de uso real firmada | Usuario |
| WP-17 | EN_VERIFICACION | Refs de pull requests y cachés antiguas | Soporte GitHub |
| WP-50 | BLOQUEADO_USUARIO | Contraste de 10 ingresos contra SITFA (C-01) | Usuario (bloqueado por P8) |
| WP-70 | BLOQUEADO_USUARIO | Acta final de cierre | Usuario, tras los tres anteriores |

### 7.4 Prueba de usuario del 13 de octubre

Instalador del run 232, commit `14be4a0`. Registro completo en
[`docs/aceptacion/REGISTRO_PRUEBAS_USUARIO_20261013.md`](aceptacion/REGISTRO_PRUEBAS_USUARIO_20261013.md).

| Área | Resultado |
| --- | --- |
| Instalación y arranque (I-01..03) | PASA |
| Descargador: extensión, consultas, PDF, bitácoras, paquetes (D-01, D-02) | PASA |
| Reutilización de bitácoras (D-04, D-05) | PASA → habilitó integrar WP-93 |
| Importación y exportación Excel (A-01) | PASA |
| Comparación, «Solo cambios», bandeja (A-03..A-05) | PASA |
| Correos, adjuntos editables, borradores en Outlook (A-07, A-08) | PASA |
| Sin duplicados al repetir (A-09) | PASA |
| Resoluciones Word (A-11) | PASA |
| **Cronología (C-01)** | **FALLA** → P7 corregido, P8 en espera |

### 7.5 Correcciones del 13 de octubre

| Commit | Qué cambió |
| --- | --- |
| `8415c0a` | Texto de la columna distingue entradas fuera de la ventana de 4 meses |
| `0ef6b26` | Estado `NO_LEIDA` → «Bitácora no leída» cuando el ingreso no se leyó |
| `a98971f` | `MAX_LECTURAS_POR_LOTE` 300 → 600 (decisión m) |
| `f363a4d` | El lector falla si hay observaciones fuera de la tabla; extractor de estructura anónima |
| `622ed4f` | Consolidación en rama única; ramas auxiliares retiradas |

Motor, textos, productos golden, Outlook y contrato de captura: **sin cambios**.

### 7.6 Pendientes técnicos sin datos

- **H7:** capturas de pantalla a 1024×650 — requiere Windows.
- **Color de tema con Tk 9** — requiere esa versión de Tk.
- **6 conflictos R-T en xfail** y advertencia de pandas en `motor/` — esperan decisión del usuario, porque tocar
  el motor exige su autorización expresa.

### 7.7 Camino al cierre

```
1. estructura.json del usuario  ──►  corregir el lector (P8)
2. repetir C-01 con 10 ingresos ──►  WP-50 aceptado
3. acta de uso real firmada     ──►  WP-02 cerrado
4. Soporte GitHub               ──►  WP-17 cerrado
5. acta final                   ──►  WP-70 cerrado
6. autorización escrita         ──►  fusión a main (D7)
```

**El código del plan está completo.** Los cinco pasos que faltan son actos del usuario, de Soporte GitHub o
consecuencias de ellos. Ninguno es programación pendiente, salvo la corrección del lector de bitácoras, que
espera el archivo de estructura.

---

## 8. Principios que guiaron las decisiones

Lo que este proyecto hizo distinto, y por qué:

**No afirmar lo que no se verificó.** El sistema nunca dice «no hay actuaciones». Dice «no hay entradas
capturadas en el período», «bitácora no leída», «origen desconocido». P7 fue exactamente eso: el defecto no era
que faltara un dato, sino que el programa afirmaba algo falso sobre un dato que no tenía.

**Fallar cerrado.** Si un control no puede inspeccionar un archivo, dice «control incompleto», no «limpio». Si un
respaldo está dañado, se conservan las copias en vez de borrarlas. Si una lectura no se completó, se marca como
no leída.

**Los datos del usuario no salen de su PC.** Toda la cadena de diagnóstico se construyó alrededor de esa regla:
extractores anónimos, diagnósticos con conteos, análisis local de CSV sin copiarlos a ningún archivo.

**La evidencia del usuario manda sobre mi hipótesis.** Tres veces propuse una explicación y tres veces fue
refutada. Cada refutación acortó el camino a la causa real. Cuando el usuario dijo «imposible», tenía razón: yo
estaba suponiendo sobre un dominio que él conoce y yo no.

**Nada se acepta solo porque las pruebas pasan.** 932 pruebas verdes no acreditan que el sistema sirva. Eso lo
decide el usuario, en un acta, después de usarlo con datos reales. Las pruebas automáticas evitan retrocesos; no
sustituyen el juicio.
