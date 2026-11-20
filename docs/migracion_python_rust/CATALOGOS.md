# Catálogos comparados en el corte del estudio

Se compararon literalmente el texto Python y las cadenas Rust, agrupadas por clave. La presencia de una cadena no prueba que la regla se active ni que produzca el mismo resultado. La columna de confirmación reproduce el dato del catálogo Python.

| Clave | Texto de referencia Python | Confirmado en Python | Workspace Rust | Prototipo |
|---|---|---|---|---|
| `COMUN.NO_SEGUIMIENTO` | La derivación {PROGRAMA} no se encuentra sujeta a seguimiento por este Centro. | Sí | Igual | Igual |
| `COMUN.MAYORIA_EDAD` | {PNOMBRE} alcanzó la mayoría de edad el {FECHA_MAYORIA}. Se sugiere egresar la medida. | Sí | Igual | Igual |
| `COMUN.PROXIMA_MAYORIA` | {PNOMBRE} alcanzará la mayoría de edad el {FECHA_MAYORIA}. | Sí | Igual | Igual |
| `COMUN.CURADOR` | No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente. | Sí | Igual | Igual |
| `COMUN.OIDO` | Oído el {FECHA_OIDO}. | Sí | Igual | Igual |
| `COMUN.PROX_AUDIENCIA` | Audiencia decretada para el {FECHA_AUDIENCIA}. | Sí | Igual | Igual |
| `COMUN.PROX_AUDIENCIAS` | Audiencias decretadas para {FECHAS_AUDIENCIA}. | Sí | Igual | Igual |
| `COMUN.REITERA_OBSERVACION` | Se reitera observación anterior del {FECHA_OBSERVACION_ANTERIOR}. | Sí | Igual | Igual |
| `COMUN.CURADOR_SIN_DESIGNACION` | No registra curador ad litem en RUS. | Pendiente | Igual | Igual |
| `ESPERA.E04_RESOLUCION_RECIENTE` | Medida revisada, a la espera de ingreso efectivo. Ingreso al programa {PROGRAMA} ordenado el {FECHA_RESOLUCION}. (Observación administrativa; no requiere gestión del Tribunal). | Sí | Igual | Igual |
| `ESPERA.E05_SOLO_CORREO` | Medida revisada, a la espera de ingreso efectivo. Se remite correo al programa consultando fecha estimada de ingreso. | Sí | Igual | Igual |
| `ESPERA.E05_PROYECTO_Y_CORREO` | Medida revisada, a la espera de ingreso efectivo. Se remite proyecto de resolución pidiendo cuenta del ingreso efectivo y correo al programa consultando fecha estimada de ingreso. | Sí | Igual | Igual |
| `ESPERA.E06_SIN_RESOLUCION` | Medida revisada, a la espera de ingreso efectivo al programa {PROGRAMA}. | Sí | Igual | Igual |
| `ESPERA.E07_PROYECCION` | Medida revisada, a la espera de ingreso efectivo. {PROYECCION} | Pendiente | Igual | Igual |
| `CUMPLIMIENTO.C03_INGRESO_RECIENTE` | Ingreso efectivo al programa {PROGRAMA} registrado el {FECHA_INGRESO}. | Sí | Igual | Igual |
| `CUMPLIMIENTO.C04_VENCIDA` | Medida vencida en RUS desde el {FECHA_EGRESO_PROYECTADO}. | Sí | Igual | Igual |
| `CUMPLIMIENTO.C05_VENCE_HOY` | Medida con vencimiento en RUS hoy, {FECHA_EGRESO_PROYECTADO}. | Sí | Igual | Igual |
| `CUMPLIMIENTO.C05_POR_VENCER` | Medida próxima a vencer en RUS el {FECHA_EGRESO_PROYECTADO}. | Sí | Igual | Igual |
| `CUMPLIMIENTO.C07_SIN_FICHA` | No registra ficha individual en RUS. Se sugiere confeccionarla. | Sí | Igual | Igual |
| `CUMPLIMIENTO.C07_FICHA_ANTIGUA` | Ficha individual actualizada por última vez el {FECHA_FICHA_INDIVIDUAL}. Se sugiere actualizarla. | Sí | Igual | Igual |
| `CUMPLIMIENTO.C07_FICHA_RECIENTE` | Ficha individual actualizada el {FECHA_FICHA_INDIVIDUAL}. | Sí | Igual | Igual |
| `CUMPLIMIENTO.C08_FICHA_FAE` | {PNOMBRE} no registra ficha FAE en RUS. Se sugiere confeccionarla. | Sí | Igual | Igual |
| `CUMPLIMIENTO.C09_SIN_OBSERVACIONES` | Medida revisada, sin observaciones. | Sí | Igual | Igual |
| `CUMPLIMIENTO.C09_BASE_BREVE` | Medida revisada. | Sí | Igual | Igual |
| `CUMPLIMIENTO.C10_HOJA2` | Próximo informe de avance de {PROGRAMA} vence el {FECHA_VENCIMIENTO}. | Sí | Igual | Igual |
| `INFORMES.I01_VENCIDO_DCE` | Informe diagnóstico vencido desde el {FECHA_VENCIMIENTO}. Se deja constancia de que se remite proyecto de resolución pidiendo cuenta del informe y, de igual manera, correo al programa {PROGRAMA} requiriendo su envío. | Sí | Igual | Igual |
| `INFORMES.I01_VENCIDO_GENERAL` | Informe de avance vencido desde el {FECHA_VENCIMIENTO}. Se deja constancia de que se remite proyecto de resolución pidiendo cuenta del informe y, de igual manera, correo al programa {PROGRAMA} requiriendo su envío. | Sí | Igual | Igual |
| `INFORMES.I02_POR_VENCER_DCE` | Se remite correo al programa {PROGRAMA} informando que el informe diagnóstico debe ser remitido a más tardar el {FECHA_VENCIMIENTO}. | Sí | Igual | Igual |
| `INFORMES.I02_POR_VENCER_GENERAL` | Se remite correo al programa {PROGRAMA} informando que el próximo informe de avance vence el {FECHA_VENCIMIENTO}. | Sí | Igual | Igual |
| `INFORMES.I02_MARGEN_DCE` | Informe diagnóstico vencido en RUS desde el {FECHA_VENCIMIENTO}. Se remite correo al programa {PROGRAMA} requiriendo su envío a más tardar el {FECHA_LIMITE}. | Pendiente | Igual | Igual |
| `HISTORIA.BASE_TRAMITE` | Se hace presente que en resolución de fecha {FECHA} (folio {FOLIO}), consta el trámite «{DESC}». | Sí | Igual | Ausente |
| `HISTORIA.VARIOS` | Se hace presente que constan {N} trámites «{DESC}»; el último, en resolución de fecha {FECHA} (folio {FOLIO}). | Sí | Igual | Ausente |
| `HISTORIA.MEDIDA_REVISADA` | Medida revisada. | Sí | Igual | Igual |
| `HISTORIA.E04_CON_FOLIO` | Medida revisada, a la espera de ingreso efectivo. Ingreso al programa {PROGRAMA} ordenado en resolución de fecha {FECHA_RESOLUCION} (folio {FOLIO}). (Observación administrativa; no requiere gestión del Tribunal). | Sí | Igual | Ausente |
| `HISTORIA.REITERA` | {MATERIA}; se reitera la sugerencia formulada en la observación del {FECHA_ANTERIOR}. | Pendiente | Igual | Ausente |
| `HISTORIA.MATERIA_CURADOR` | No registra curador ad litem en RUS | Pendiente | Igual | Ausente |
| `HISTORIA.MATERIA_SIN_FICHA` | No registra ficha individual en RUS | Pendiente | Igual | Ausente |
| `HISTORIA.MATERIA_SIN_FAE` | No registra ficha FAE en RUS | Pendiente | Igual | Ausente |
| `HISTORIA.MEDIDA_SIN_VIGENCIA` | Medida sin vigencia en RUS desde el {FECHA_EGRESO_PROYECTADO}. | Sí | Igual | Igual |
| `HISTORIA.EGRESO_PROYECTADO` | RUS registra como fecha de egreso proyectado el {FECHA_EGRESO_PROYECTADO}. | Sí | Igual | Igual |
| `HISTORIA.P_FINALIZADA` | La causa se encuentra finalizada, se deja constancia de que se remite proyecto de resolución que aplica nomenclatura «Egreso de servicio protección especializada y otras redes» a fin de regularizar informáticamente la causa. | Sí | Igual | Ausente |
| `HISTORIA.C10_ESCRITO` | Informe de avance recepcionado en resolución de fecha {FECHA_ESCRITO} (folio {FOLIO_ESCRITO}). Se deja constancia de que se remite proyecto de resolución que aplica la nomenclatura «Ingreso Informe de Cumplimiento X» a fin de regularizar informáticamente la causa. | Sí | Igual | Ausente |
| `HISTORIA.J2_ACUMULADOS` | Registra {N} informes vencidos en RUS; el más antiguo venció el {FECHA}. | Sí | Igual | Igual |
| `HISTORIA.I01_SIN_ENVIO_GENERAL` | Informe de avance vencido desde el {FECHA_VENCIMIENTO}. | Sí | Igual | Ausente |
| `HISTORIA.I01_SIN_ENVIO_DCE` | Informe diagnóstico vencido desde el {FECHA_VENCIMIENTO}. | Sí | Igual | Ausente |
| `HISTORIA.T5_INSTITUCIONES` | Atendido que en sistema de seguimiento RUS se visualizan {N} instituciones distintas como curadores ad litem de {PNOMBRE}, se sugiere eliminar aquellas designaciones que no se encuentren vigentes. | Sí | Igual | Igual |
| `HISTORIA.SERVICIO_INGRESO` | Tribunal dispone ingreso a {PROGRAMA}, en resolución de fecha {FECHA} (folio {FOLIO}). | Sí | Igual | Ausente |
| `HISTORIA.EGRESO_POSTERIOR_MAYORIA` | El egreso proyectado es posterior a la mayoría de edad. | Sí | Igual | Igual |
| `HISTORIA.EGRESO_POSTERIOR_MAYORIA_FECHA` | El egreso proyectado es posterior a la mayoría de edad, que se alcanza el {FECHA_MAYORIA}. | Sí | Igual | Igual |
| `HISTORIA.SOLICITUD_DCE` | Informe clínico diagnóstico ordenado en autos, se encuentra en la causa en resolución de fecha {FECHA} (folio {FOLIO}), a la espera de lo que se resuelva en la audiencia respectiva. | Sí | Igual | Ausente |
| `HISTORIA.CONTINUIDAD_ATENCION` | Se hace presente que en resolución de fecha {FECHA} (folio {FOLIO}), consta respuesta sobre la continuidad de atención. | Sí | Igual | Ausente |
| `HISTORIA.INFORME_INGRESADO` | Se hace presente que en resolución de fecha {FECHA} (folio {FOLIO}), se ingresa el informe de cumplimiento. | Sí | Igual | Ausente |
| `HISTORIA.R11_ORDEN` | Se hace presente que en resolución de fecha {FECHA} (folio {FOLIO}), se decretó orden de búsqueda. | Sí | Igual | Ausente |
| `HISTORIA.R11_PIDE_CUENTA` | Posteriormente, en resolución de fecha {FECHA} (folio {FOLIO}), se pidió cuenta de la búsqueda. | Sí | Igual | Ausente |
| `HISTORIA.R11_SIN_EFECTO` | Luego, en resolución de fecha {FECHA} (folio {FOLIO}), se dejó sin efecto la búsqueda. | Sí | Igual | Ausente |
| `HISTORIA.R11_TERMINO` | Luego, en resolución de fecha {FECHA} (folio {FOLIO}), se dispuso el término de la búsqueda. | Sí | Igual | Ausente |
| `HISTORIA.OTRO_PROGRAMA` | Se hace presente que {PNOMBRE} cuenta con ingreso efectivo al programa {OTRO_PROGRAMA}. | Sí | Igual | Ausente |
| `HISTORIA.INFORME_AVANCE_PENDIENTE` | Pendiente de resolver, informe de avance. | Sí | Igual | Ausente |
| `HISTORIA.INFORME_REVISAR_ASOCIACION` | Se hace presente que el informe de {PROGRAMA} registra ingreso a folio {FOLIO} y figura pendiente en el calendario. Se sugiere revisar su asociación con la fecha proyectada {FECHA_PROYECTADA}. | Sí | Igual | Ausente |

## Umbrales por defecto

Valores en días. Para la inclusión del extremo, leer la condición de la regla correspondiente.

| Clave | Python | Workspace Rust | Prototipo |
|---|---:|---:|---:|
| `espera_dce` | 0 | 0 | 0 |
| `espera_laja` | 30 | 30 | 30 |
| `espera_mulchen` | 30 | 30 | 30 |
| `espera_tome` | 60 | 60 | 60 |
| `proyecto_tome` | 60 | 60 | 60 |
| `mayoria` | 60 | 60 | 60 |
| `oido` | 45 | 45 | 45 |
| `resolucion_reciente` | 30 | 30 | 30 |
| `ingreso_reciente` | 30 | 30 | 30 |
| `medida` | 45 | 45 | 45 |
| `informe` | 30 | 30 | 30 |
| `ficha_antigua` | 180 | 180 | 180 |
| `ficha_reciente` | 30 | 30 | 30 |
| `ficha_fae` | 120 | 120 | 120 |
| `prorroga_reciente` | 90 | 90 | Ausente |
| `ventana_busqueda` | 60 | 60 | Ausente |

Fuentes: `src/nurus/personal/{config.py,textos_base.json}`, `rust/datos/{umbrales,textos}.json` y `prototipos/flow-egui/src/config.rs`.
