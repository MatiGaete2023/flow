# UX y observaciones dev12 — 22 de septiembre de 2026

Producto: **CSMP Assistant personal 0.4.0.dev12**. Rama de desarrollo: `ux-observaciones-20260922`.

## Objetivo

Aplicar una mejora UX acotada y las redacciones de observaciones aprobadas sin rediseñar el motor, sin alterar umbrales sustantivos y sin ampliar el producto hacia multiusuario, nube o servicios externos.

## Cambios funcionales

- Panel de detalle reutilizable en Trabajo, Correos y Resoluciones con datos disponibles de la causa y productos relacionados.
- Navegación Anterior/Siguiente incidencia basada exclusivamente en `Row.warnings`.
- Comparación original/final para observación, correo y proyecto Word; se oculta cuando no existe edición.
- Jerarquía de acciones: Actualizar desde Excel, Guardar borradores y Generar Word como acciones primarias; las demás permanecen disponibles.
- Éxitos y operación en curso se informan en la barra de estado; errores, decisiones y riesgos mantienen diálogos.
- Configuración se divide visualmente en Básico y Avanzado sin cambiar el formato persistido.
- Reanudación del último trabajo reutiliza `sesion/trabajo.json`; ya no se activa silenciosamente al iniciar y exige Continuar.
- Tooltips mínimos para `PC_IE`, `PC_INFO` y `NOMENCL`.
- `Enviados` conserva su implementación y se presenta al usuario como Historial.

## Corrección tras prueba visual — 23 de septiembre de 2026

La prueba real de la pantalla Trabajo mostró una fricción material: el panel «Detalle de causa» repetía la observación completa y ocupaba altura que necesitaba el editor. Se corrigió sin alterar dominio ni otras pantallas:

- Trabajo usa un detalle compacto con RIT, NNA, tribunal, programa/modalidad, TT, CC, RES, estado, productos e incidencias cuando existan;
- la observación final deja de repetirse en ese panel porque ya está disponible inmediatamente en el editor;
- el detalle de Trabajo usa mayor ancho de ajuste para evitar saltos de línea innecesarios;
- la distribución inicial del separador vertical cambia de tabla/editor 3:2 a 2:3 para priorizar la edición;
- Correos y Resoluciones conservan el detalle completo.

La regresión `test_work_compact_detail_omits_duplicate_observation_and_keeps_context` protege esta decisión.

## Correcciones tras uso real — 24 de septiembre de 2026

Se corrigieron tres regresiones detectadas durante el uso real de dev12:

- **Correos de programas en ESPERA:** una planilla recargada como entrada externa podía omitir el filtro de acciones por fila y arrastrar registros que no cumplían la gestión de correo. Las comunicaciones automáticas ahora respetan `NURUS_REGLAS`/trazabilidad cuando existe y el subconjunto de filas realmente asociado a `programa_espera`. Una edición humana de la observación final que retire la gestión de correo también impide preparar ese correo automático. La selección manual explícita conserva su carácter de override humano.
- **Adjuntos Excel:** la nómina adjunta se construye desde el mismo subconjunto filtrado del borrador y aplica borde fino a todas las celdas de la tabla, encabezado incluido.
- **Resoluciones por tribunal:** las pruebas cubren conjuntamente `Jgdo. L. y G. de Laja` y `Jgdo. L. y G. de Mulchén`, verificando agrupación y matriz específica por tribunal. La lista de proyectos muestra ahora el tribunal. Antes de generar, se valida además que la ruta de la matriz corresponda al tribunal y tipo del proyecto; una discordancia bloquea la generación en vez de producir silenciosamente un documento incorrecto.
- **Fuentes/estilos Word:** el documento conjunto usa `docxcompose` con preservación de estilos y se eleva la dependencia mínima a `docxcompose>=2.2,<3`. La regresión verifica la fuente efectiva de matrices distintas dentro del mismo Word.

La causa exacta del caso real en que un documento de Mulchén apareció como Laja no puede acreditarse solo desde el repositorio: la clasificación actual reconoce el nombre institucional de Mulchén y las matrices empaquetadas están separadas por tribunal. Las matrices locales personalizadas en `%LOCALAPPDATA%/CSMP_Personal/plantillas_word` se conservan deliberadamente entre actualizaciones; si el caso reaparece, deben revisarse el Excel concreto, el Word generado y la matriz local implicada.

Checkpoint de código de estas correcciones: `31f2fefee70641c2e6f7311a2e2c8d01a6561f5c`. GitHub Actions run `36009322188`: **success** en Windows/Python 3.12–3.14, con **278 passed, 1 skipped** por versión. Python 3.12 aprobó también smoke GUI y construcción de la distribución ZIP.

## Hotfix de identidad de copia revisada — 25 de septiembre de 2026

Correos y Resoluciones dejaron de depender rígidamente de que la copia revisada conserve `NURUS_ID_REGISTRO`. Cuando esa columna existe sigue siendo la identidad autoritativa. Si falta, el sistema intenta asociar la revisión por la identidad compuesta disponible (RIT, RUT, NNA, tribunal y programa), pero solo acepta el fallback si cada identidad es única y el conjunto coincide exactamente. No se usa el número de fila como sustituto.

Esto evita el bloqueo observado en uso real sin relajar la protección contra aplicar observaciones o productos a otra persona. Duplicados o cambios de identidad continúan deteniendo la operación.

## Observaciones y RES

Los catálogos activos `personal/textos_base.json` y `rus/textos_observaciones.json` quedan sincronizados con las redacciones aprobadas. La migración `revision_textos=3` reemplaza solamente textos que aún coinciden exactamente con el valor predeterminado anterior y conserva personalizaciones.

En Cumplimiento se elimina el prefijo genérico «Medida revisada.» cuando existe un hallazgo concreto. La composición prioriza estado principal, luego hitos procesales (oído/audiencia) y finalmente sugerencias como curador/fichas.

`INFORMES.I01_VENCIDO_GENERAL` e `I01_VENCIDO_DCE` mantienen la asociación estructurada a `PC_INFO`. `I02_*` continúa generando solo correo preventivo. No se deduce `PC_INFO` desde palabras de la observación.

## Lo que no cambia

No cambian los umbrales, modalidades, criterios de mayoría de edad, reglas de fichas, criterios de espera ni reglas de cruce. Se conservan Excel original intacto, copia revisable, RES categórico y legado, validación nativa Excel, Word, borradores Outlook, revisión humana, Windows y Python 3.12–3.14.

No se agrega SQLite, dashboard, gráficos, usuarios/roles, nube, API externa, IA, plugins, PySide6, Electron, C#, `.exe`, instalador ni actualización automática.

## Conflicto abierto

El conflicto histórico DCE/Informes permanece sin modificación: el manual documentado exige más de 40 días para pedir cuenta, mientras el motor vigente clasifica el informe como vencido desde que pasa la fecha. Dev12 no cambia ese umbral porque esta evolución autorizó redacción y `PC_INFO`, no reglas temporales sustantivas.

## Pruebas

Se agregan regresiones para textos exactos, sincronización de catálogos, migración sin sobrescribir personalizaciones, `PC_INFO` de vencidos, ausencia de `PC_INFO` por vencer, orden de composición, detalle de causa, incidencias, comparación, productos, reanudación y jerarquía UX.

Checkpoint corregido `dfa7809505da47f13e2dbdf799ab820de05719a9`, GitHub Actions run `35867829076` (attempt 2): **success** en Windows/Python 3.12–3.14, con **273 passed, 1 skipped** por versión. Python 3.12 aprobó además el smoke GUI y la distribución ZIP dev12. La primera ejecución de Python 3.12 agotó el límite de 8 minutos sin fallo de aserción mientras coexistían varias matrices disparadas por commits consecutivos; la repetición aislada terminó correctamente.

## Validación real pendiente

La prueba visual ya detectó y permitió corregir el exceso de altura del detalle en Trabajo. Aún debe comprobarse visualmente la corrección en el PC del usuario y continúan pendientes la navegación de incidencias, comparaciones, recuperación de sesión y el flujo habitual completo con Excel 2010/Outlook institucional.
