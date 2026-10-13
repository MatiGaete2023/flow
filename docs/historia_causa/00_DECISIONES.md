# Decisiones — historia de la causa

## Tomadas por el usuario (13 de octubre de 2026)

| ID | Decisión | Fuente (palabras del usuario) | Efecto |
|---|---|---|---|
| H-a | Leer la historia de la causa desde el Descargador, además de bitácoras | «que en el descargador, además de poder leer las bitácoras, también se pueda leer la historia de la causa» | Nueva fuente `historia_tramites` en el Descargador |
| H-b | **Sin PDF** | «no deseo descargar los pdf de los trámites de la causa» | No se abre, descarga, lee ni convierte ningún documento; la tabla `documentos` queda `NO_SOLICITADA` |
| H-c | Solo los datos visibles de la ventana | «solo ver la descripción del trámite y el trámite, su fecha, folio, los datos que están en esa ventana» | Columnas Folio, Etapa, Trámite, Desc. Trámite, Referencia, Fecha Tram (+ presencia de anexo) |
| H-d | Fin: verlos y que el Asistente los analice **en otro paso**; nada más | «a fin de, en otro paso, puedan ser analizados por el asistente, pero nada más» | Este alcance no cambia el Asistente ni sus reglas; solo deja los datos en el paquete |
| H-e | **Solo ventana de 4 meses** | «y solo en la ventana de 4 meses» | Se conservan como datos solo los trámites con fecha dentro de los 4 meses naturales previos a la fecha de lectura |
| H-f | Uso personal en Windows | «Este producto será solo de uso personal en windows» | Entrega en el instalador Windows existente; sin variantes multiusuario |
| H-g | Ventana igual en Espera y Cumplimiento | «la ventana de trámites es igual para todos los registros» | Un solo parser; el acceso desde el listado puede variar por pestaña (H0 lo confirma) |
| H-h | Rama propia como base | «Deja el plan y todo lo necesario en otra RAMA» | `feat/historia-causa-sitfa` |

## A registrar en el gobierno del repositorio (primer paso de WP-52)

`PLAN_VIGENTE.md` dice: «Historia SITFA/PDF y reglas CT-03..10: fuera de alcance hasta nueva decisión.» Hay que
añadir la decisión del 13 de octubre (H-a…H-f) en `PLAN_VIGENTE.md` y en `docs/implementacion/estado/global.json`
→ `decisiones_adicionales`, dejando explícito que **PDF y CT-03..10 siguen fuera de alcance**.

## Pendientes (con recomendación; el usuario elige)

| ID | Pregunta | Recomendación | Por qué |
|---|---|---|---|
| P1 | ¿Incluir en el Excel el encabezado de la causa (RIT, RUC, Caratulado, F. Ingreso, Proc., Forma Inicio, Est. Adm., Etapa, Estado Proc., Tribunal, marca de reserva)? | Sí, en la hoja «Causas» | Está en la ventana (H-c) y sirve para cotejar identidad; es producto operativo local |
| P2 | ¿Opción «Leer historia» activada por defecto? | No: casilla desmarcada, recordada entre sesiones | Menos carga a SITFA y menos datos cuando no se necesita |
| P3 | Tope de historias por lote | 600, separado del tope de bitácoras | Igual que la decisión m; un tope no consume al otro |
| P4 | ¿Reutilizar historias leídas hoy (como EF-04)? | No en la primera entrega | Una actuación nueva no aparecería; simplicidad |
| P5 | ¿Visor dentro del Descargador además del Excel? | No en la primera entrega; Excel con filtros | Prioridad 2 del plan (simplificar); el Excel cubre «ver» |
| P6 | ¿Conservar el HTML íntegro de cada historia en `respaldo/historia/`? | Sí (local, 60 días) | Permite reprocesar y auditar sin volver a SITFA; nunca va en el ZIP |
| P7 | Si H0 muestra que `a1` solo se obtiene ejecutando la función de la página (escenario C) | Decidir tras ver el esqueleto; requiere E3 | Implica ejecutar código de SITFA en la página del usuario |
| P8 | ¿Trabajar directo en esta rama, sin PR por WP? | Sí; un solo PR al final | Menos corridas de Actions (Windows solo una vez) y menos sesiones; coherente con la decisión n |

Fecha de corte de la ventana: la fecha local de la lectura (no la fecha de corte del Asistente). Queda escrita en
cada producto.
