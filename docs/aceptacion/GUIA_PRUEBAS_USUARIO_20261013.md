# Guía de pruebas del usuario

13 de octubre de 2026. Son las pruebas que solo puedes hacer tú: necesitan tu PC autorizado, tu sesión SITFA, tu
Outlook y tus datos reales. Todo lo demás ya está probado de forma automática con datos ficticios.

**Reglas durante las pruebas**
- No envíes ningún correo: el programa solo guarda borradores, y el envío es decisión tuya, fuera de la prueba.
- No copies a ningún registro ni conversación nombres, RUT, RIT, textos de bitácoras, capturas ni archivos. Anota
  solo **conteos, códigos de prueba y PASA/FALLA**.
- Si algo falla, anota el código de la prueba, el paso y el mensaje de la aplicación, sin datos de causas. No borres
  la carpeta de la sesión ni del lote, porque sirven de evidencia local.
- Antes de las pruebas de Outlook (A-08 a A-10), crea una carpeta de Outlook vacía para mover ahí los borradores de
  prueba, o anota cuántos borradores tienes antes de empezar.

## 0. Preparación

| Paso | Operación | Resultado esperado |
| --- | --- | --- |
| 0.1 | Descarga el artefacto `CSMP-Instalador-Windows-0.5.0.dev5` del run de construcción que te indique el programador. Debe ser el último, que incluye la Fase A y EF-04. Anota el **SHA del commit** que muestra la página del run. | Un `.zip` con el instalador |
| 0.2 | Instala con el instalador; si había una versión anterior, acepta reemplazarla. | Accesos directos «Asistente CSMP» y «Descargador SITFA» |
| 0.3 | Carga la extensión de Chrome desde la carpeta que indica el instalador (`chrome://extensions`, «Cargar descomprimida»). Si ya estaba cargada, pulsa «Recargar». | La extensión aparece activa, versión 2.7.6 |
| 0.4 | Copia la plantilla de registro `REGISTRO_PRUEBAS_USUARIO_PLANTILLA.md` a una carpeta tuya, fuera de las carpetas de sesión, y anota el SHA. | Registro listo |

## 1. Instalación y arranque

| Código | Operación | Resultado esperado |
| --- | --- | --- |
| I-01 | Abre cada acceso directo. | Cada uno abre su programa; ninguno abre el otro por sí solo |
| I-02 | En el Asistente, Configuración › Avanzado › autodiagnóstico, si está disponible. | Diagnóstico sin errores |
| I-03 | Cierra y reabre el Asistente. | Abre la última sesión o la pantalla inicial, sin mensajes de error |

## 2. Descargador SITFA

Usa un alcance pequeño: **un tribunal, una modalidad y el período habitual**.

| Código | Operación | Resultado esperado | Qué anotar |
| --- | --- | --- | --- |
| D-01 | Inicia sesión en SITFA en Chrome. En el Descargador, elige el alcance y pulsa **Consultar y crear paquete**. | Avanza por consultas, bitácoras y paquete; la pausa entre consultas se ve; termina con «Abrir carpeta del paquete» y «Copiar ruta» | Duración aproximada; n.º de registros; n.º de bitácoras leídas |
| D-02 | Abre la carpeta del paquete. | En la raíz hay `Captura_CSMP.zip`; los intermedios están en `respaldo/` | PASA/FALLA |
| D-03 | Revisa los avisos del final: SIN_VINCULO, fallidas, cobertura. | Toda fila sin vínculo o fallida aparece con su motivo, sin vacíos silenciosos | Conteos de SIN_VINCULO y FALLIDA |
| D-04 (EF-04) | Con la casilla **Reutilizar bitácoras leídas hoy** marcada, repite D-01 el mismo día y con el mismo alcance. | No abre bitácoras de nuevo (la barra avanza rápido) y el lote termina COMPLETO | Bitácoras abiertas: 0; tiempo comparado con D-01 |
| D-05 (EF-04) | Desmarca la casilla y repite. | Vuelve a abrir todas las bitácoras | N.º de bitácoras abiertas = las de D-01 |
| D-06 | Durante una lectura de bitácoras, pulsa Cancelar. Después usa **Reintentar lecturas fallidas…** o **Retomar** sobre esa carpeta. | Continúa sin repetir lo ya leído | PASA/FALLA |
| D-07 | Mira la pantalla de SITFA durante y después de las pruebas. | El programa nunca guardó, modificó ni envió nada en SITFA | PASA/FALLA |
| D-08 (opcional) | Si tienes lotes o paquetes de más de 60 días: Resultados e historial › **Archivar o borrar lotes antiguos**. Prueba primero **Archivar**. | Solo ofrece lotes antiguos que no están en uso; nada se borra sin confirmar; con «conservar», un paquete mantiene su ZIP | PASA/FALLA |

## 3. Asistente CSMP

| Código | Operación | Resultado esperado | Qué anotar |
| --- | --- | --- | --- |
| A-01 | Trabajo › **Importar paquete**: elige el ZIP de D-01 y la fecha de corte. | Importa y **exporta la copia Excel automáticamente**; Correos y Resoluciones quedan habilitados | n.º de filas; n.º de avisos |
| A-02 | Abre el Excel exportado. | Se ve bien; las observaciones del motor son las que esperas según tu criterio | Filas revisadas a mano; diferencias con tu criterio (solo conteo por regla) |
| A-03 (Fase A) | Importa **el mismo** ZIP otra vez. | Comparación: 0 nuevas, 0 con cambios; el filtro **Solo cambios** no muestra filas | Conteos del resumen |
| A-04 (Fase A) | Edita la observación de 1 fila. Al día siguiente (o con otro corte), crea un paquete nuevo con el mismo alcance e impórtalo. | El resumen indica nuevas, con cambios de fuente o de reglas y egresadas; **Solo cambios** muestra solo esas filas; **Ver egresados** lista las salidas; tu edición se conserva | Conteos del resumen; filas que tuviste que revisar |
| A-05 (Fase A) | Pulsa **Qué requiere mi atención (N)**. Haz doble clic en un caso. | Lista ordenada por urgencia (incierto, obsoleto, aviso, por preparar, preparado) y luego por vencimiento; el doble clic lleva al caso y a la pestaña que corresponde | N; ¿el orden te sirve? (sí/no) |
| A-06 | En Excel, cambia 1 observación de la copia exportada, guarda, y en el Asistente usa **Actualizar Excel · F5** (pestaña Trabajo). | Incorpora el cambio; si hay conflicto, lo muestra para resolverlo | PASA/FALLA |
| A-07 | Correos: elige un programa y pulsa **Preparar tipo** (y luego **Preparar todos**). | Genera borradores con su nómina adjunta; los contactos desconocidos quedan vacíos con aviso | n.º de borradores preparados |
| A-08 | Revisa 2 borradores (destinatarios, texto, adjunto) y pulsa **Guardar borradores**. | Aparecen en Borradores de Outlook con CC `ucc_concepcion@pjud.cl` y tu firma; **ninguno enviado** | Borradores en Outlook antes y después; enviados: 0 |
| A-09 | Repite A-08 con los mismos borradores. | No se duplican: aparecen como ya guardados u omitidos | Borradores nuevos: 0 |
| A-10 (opcional, recuperación) | Prepara 5 borradores, inicia **Guardar borradores** y cierra el Asistente desde el Administrador de tareas a mitad del lote. Reabre la sesión. | Los ya guardados figuran como guardados o inciertos (revisar Outlook); al repetir, no se duplican | Borradores en Outlook = únicos; duplicados: 0 |
| A-11 | Resoluciones: prepara y genera 1 Word. | El Word se genera con las variables correctas; la bandeja lo marca generado | PASA/FALLA |
| A-12 (Fase A) | Tras A-08 y A-11, abre de nuevo la bandeja. | Los casos guardados ya no aparecen como pendientes de preparar; el detalle dice «Pendiente de actuación: envío o firma (decisión humana)» | PASA/FALLA |
| A-13 | Configuración › Avanzado: activa **Guardar métricas locales de revisión (solo conteos)** y haz 2 o 3 ciclos reales. Luego pulsa **Copiar resumen** y pégalo en tu registro. | El resumen solo trae conteos y fechas | El resumen copiado |

## 4. Cronología de bitácoras (aceptación de WP-50)

| Código | Operación | Resultado esperado | Qué anotar |
| --- | --- | --- | --- |
| C-01 | Elige **10 ingresos** del paquete de A-01. En cada uno, compara la columna **Última obs. centro** y el detalle de cronología con lo que ves en SITFA. | Coincide la fecha de la última observación del centro. Las entradas posteriores, las de orden incierto y las de origen desconocido se muestran como tales | Coincidencias de 10; tipo de discrepancia (fecha, origen, faltante), sin datos |

## 5. Copia anónima (solo si vas a compartir un caso)

| Código | Operación | Resultado esperado |
| --- | --- | --- |
| N-01 | `tools/anonimizar_excel.py original.xlsx copia.xlsx` sobre un Excel tuyo, **en tu PC**. | La copia no tiene nombres ni RUT. Observaciones y glosas dicen «[texto omitido]» y los títulos de hoja no estándar pasan a «HojaN». **Revisa a mano** el encabezado y las columnas que lee el motor antes de compartir |

## 6. Registro y cierre

1. Llena la plantilla con cada código: **PASA**, **FALLA** o **NO_EJECUTADO**, más los conteos pedidos.
2. Envía al programador **solo el registro** (sin datos personales). Con eso:
   - se integra WP-93 si D-04 y D-05 pasan;
   - se completa el acta `ACTA_USO_REAL` de la versión exacta (el SHA de 0.1), con el tipo de evidencia que tú elijas;
   - se registra el contraste de WP-50 (C-01).
3. Tú decides y firmas el acta; nadie más puede marcarla como aceptada.

Si una prueba bloqueante falla (D-07, A-08 con algún correo enviado, A-09/A-10 con duplicados, C-01 con
discrepancias de fecha), detén las pruebas siguientes de ese grupo y envía el registro.
