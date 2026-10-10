# Catálogo de observaciones aprobado — 22-09-2026

Estado: contrato de redacción previo a implementación.
Producto: CSMP Assistant personal / NuRus.
Alcance: textos de observación, composición y efectos sobre productos. No modifica todavía el motor.

## 1. Principios aprobados

- Prefijo: `PrimerNombre SIG: [observación]`.
- Fechas siempre extensas: `30 de septiembre de 2026`.
- Se mantiene la normalización legible del nombre del programa.
- Evitar repetir el nombre del NNA en el cuerpo cuando ya aparece en el prefijo.
- Usar `RUS` solo cuando aporte precisión sobre el estado o ausencia registrada en el sistema.
- Mantener `Se sugiere...` para recomendaciones.
- Usar `correo` como forma habitual, no `correo electrónico`.
- Mantener siempre la expresión completa `proyecto de resolución`.
- Mantener `Medida revisada` en ESPERA y en `Medida revisada, sin observaciones.`; eliminarla cuando CUMPLIMIENTO o INFORMES ya contienen un hallazgo concreto.
- Orden de composición aprobado: (1) estado/hallazgo principal; (2) hitos procesales o de seguimiento; (3) acciones o sugerencias pendientes.
- Se permiten conectores breves y consistentes: `Asimismo,`, `Además,`, `De igual manera,`, `y`. No convertirlos en una regla mecánica que recargue todas las observaciones.
- Agrupar antecedentes naturalmente relacionados cuando mejora la lectura. Ejemplo aprobado: `Oído el {FECHA_OIDO} y con audiencia fijada para el {FECHA_AUDIENCIA}.`
- Hallazgo y recomendación permanecen como frases separadas.
- `Se deja constancia de que se remite...` se reserva para gestiones relevantes; `Se remite...` para gestiones simples.
- Paréntesis solo para aclaraciones administrativas breves.
- Mantener `informáticamente` cuando la acción es una regularización del registro en el sistema.
- No se definen nuevas reglas generales para `registrado/registra`, `vencido/vence` ni abreviación de `informe de avance`: se respetan las redacciones específicas aprobadas.
- No se introducen cambios adicionales en la apertura estándar de ESPERA.

## 2. Catálogo definitivo

### COMUN

| Clave | Redacción actual | Redacción aprobada |
|---|---|---|
| NO_SEGUIMIENTO | La derivación {PROGRAMA} no se encuentra sujeta a seguimiento por este Centro. | **Sin cambios:** La derivación {PROGRAMA} no se encuentra sujeta a seguimiento por este Centro. |
| MAYORIA_EDAD | Se hace presente que {PNOMBRE} alcanzó la mayoría de edad el {FECHA_MAYORIA}, se sugiere egresar la medida. | **Alcanzó la mayoría de edad el {FECHA_MAYORIA}. Se sugiere egresar la medida.** |
| PROXIMA_MAYORIA | Se hace presente que {PNOMBRE} alcanzará la mayoría de edad el {FECHA_MAYORIA}. | **Alcanzará la mayoría de edad el {FECHA_MAYORIA}.** |
| CURADOR | No registra curador asociado en RUS, se sugiere asociar curador ad litem informáticamente. | **No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.** |
| OIDO | Oído con fecha {FECHA_OIDO}. | **Oído el {FECHA_OIDO}.** |
| PROX_AUDIENCIA | Se cita a audiencia para el día {FECHA_AUDIENCIA}. | **Audiencia fijada para el {FECHA_AUDIENCIA}.** |

### ESPERA

| Clave | Redacción actual | Redacción aprobada |
|---|---|---|
| E04_RESOLUCION_RECIENTE | Medida revisada, a la espera de ingreso efectivo. Se hace presente que el Tribunal ordenó el ingreso efectivo al programa {PROGRAMA} con fecha {FECHA_RESOLUCION}. (Observación administrativa, no requiere acción/respuesta del Tribunal). | **Medida revisada, a la espera de ingreso efectivo. Ingreso al programa {PROGRAMA} ordenado el {FECHA_RESOLUCION}. (Observación administrativa; no requiere gestión del Tribunal).** |
| E05_SOLO_CORREO | Medida revisada, a la espera de ingreso efectivo. Se remite correo electrónico al programa consultando respecto de la fecha estimada de ingreso efectivo. | **Medida revisada, a la espera de ingreso efectivo. Se remite correo al programa consultando fecha estimada de ingreso.** |
| E05_PROYECTO_Y_CORREO | Medida revisada, a la espera de ingreso efectivo. Se remite proyecto de resolución pidiendo cuenta al programa respecto del ingreso efectivo. Igualmente, se remite correo electrónico al programa consultando respecto de la fecha estimada de ingreso efectivo. | **Medida revisada, a la espera de ingreso efectivo. Se remite proyecto de resolución pidiendo cuenta del ingreso efectivo y correo al programa consultando fecha estimada de ingreso.** |
| E06_SIN_RESOLUCION | Medida revisada, a la espera de ingreso efectivo al programa {PROGRAMA}. | **Sin cambios:** Medida revisada, a la espera de ingreso efectivo al programa {PROGRAMA}. |

### CUMPLIMIENTO

| Clave | Redacción actual | Redacción aprobada |
|---|---|---|
| C03_INGRESO_RECIENTE | Medida revisada. Se hace presente que el ingreso efectivo al programa {PROGRAMA} se registra con fecha {FECHA_INGRESO}. | **Ingreso efectivo al programa {PROGRAMA} registrado el {FECHA_INGRESO}.** |
| C04_VENCIDA | La medida se visualiza vencida en RUS desde el {FECHA_EGRESO_PROYECTADO}. | **Medida vencida en RUS desde el {FECHA_EGRESO_PROYECTADO}.** |
| C05_VENCE_HOY | Se hace presente que la medida se visualiza con vencimiento para el día de hoy, {FECHA_EGRESO_PROYECTADO}. | **Medida con vencimiento en RUS hoy, {FECHA_EGRESO_PROYECTADO}.** |
| C05_POR_VENCER | Se hace presente que la medida se visualiza próxima a vencer en RUS el {FECHA_EGRESO_PROYECTADO}. | **Medida próxima a vencer en RUS el {FECHA_EGRESO_PROYECTADO}.** |
| C07_SIN_FICHA | No registra ficha individual en RUS, se sugiere confeccionar. | **No registra ficha individual en RUS. Se sugiere confeccionarla.** |
| C07_FICHA_ANTIGUA | Atendido que la ficha individual registra como última actualización el {FECHA_FICHA_INDIVIDUAL}, superando los 180 días, se sugiere actualizar. | **Ficha individual actualizada por última vez el {FECHA_FICHA_INDIVIDUAL}. Se sugiere actualizarla.** |
| C07_FICHA_RECIENTE | Se hace presente que la ficha individual fue actualizada con fecha {FECHA_FICHA_INDIVIDUAL}. | **Ficha individual actualizada el {FECHA_FICHA_INDIVIDUAL}.** |
| C08_FICHA_FAE | Se hace presente que {PNOMBRE} no tiene ficha FAE, se sugiere confeccionar. | **No registra ficha FAE en RUS. Se sugiere confeccionarla.** |
| C09_SIN_OBSERVACIONES | Medida revisada, sin observaciones. | **Sin cambios:** Medida revisada, sin observaciones. |
| C09_BASE_BREVE | Medida revisada. | **No debe anteponerse cuando existen hallazgos complementarios.** Se conserva solo por compatibilidad de configuración; el compositor deja de insertarlo en ese escenario. |
| C10_HOJA2 | Medida revisada, se hace presente que el programa {PROGRAMA} deberá remitir informe de avance a más tardar el {FECHA_VENCIMIENTO}. | **Próximo informe de avance de {PROGRAMA} vence el {FECHA_VENCIMIENTO}.** |

### INFORMES

| Clave | Redacción actual | Redacción aprobada |
|---|---|---|
| I01_VENCIDO_GENERAL | Se remite correo electrónico al programa {PROGRAMA} a fin de requerir el informe de avance que se encuentra vencido en RUS desde el {FECHA_VENCIMIENTO}. | **Se deja constancia de que se remite proyecto de resolución pidiendo cuenta del informe de avance vencido desde el {FECHA_VENCIMIENTO}. De igual manera, se remite correo al programa {PROGRAMA} requiriendo su envío.** |
| I01_VENCIDO_DCE | Se remite correo electrónico al programa {PROGRAMA} a fin de requerir el informe diagnóstico que se encuentra vencido en RUS desde el {FECHA_VENCIMIENTO}. | **Se deja constancia de que se remite proyecto de resolución pidiendo cuenta del informe diagnóstico vencido desde el {FECHA_VENCIMIENTO}. De igual manera, se remite correo al programa {PROGRAMA} requiriendo su envío.** |
| I02_POR_VENCER_GENERAL | Se remite correo electrónico al programa {PROGRAMA} a fin de señalar que el próximo informe de avance vence el {FECHA_VENCIMIENTO}. | **Se remite correo al programa {PROGRAMA} informando que el próximo informe de avance vence el {FECHA_VENCIMIENTO}.** |
| I02_POR_VENCER_DCE | Se remite correo electrónico al programa {PROGRAMA} a fin de señalar que el informe diagnóstico ordenado en autos debe ser remitido a más tardar el {FECHA_VENCIMIENTO}. | **Se remite correo al programa {PROGRAMA} informando que el informe diagnóstico debe ser remitido a más tardar el {FECHA_VENCIMIENTO}.** |

## 3. Composición aprobada

La composición no debe limitarse al orden accidental en que se evalúan las reglas.

Orden de salida:
1. hallazgo o estado principal;
2. hitos procesales/seguimiento: oído y audiencia;
3. faltas de registro, fichas y recomendaciones;
4. gestiones realizadas cuando correspondan al hallazgo.

Caso específico aprobado:
- si existen OIDO y PROX_AUDIENCIA en una misma observación, pueden agruparse como: `Oído el {FECHA_OIDO} y con audiencia fijada para el {FECHA_AUDIENCIA}.`

No se fusionan hallazgo y recomendación:
- `No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.`
- `No registra ficha FAE en RUS. Se sugiere confeccionarla.`
- `Ficha individual actualizada por última vez el {FECHA_FICHA_INDIVIDUAL}. Se sugiere actualizarla.`

Los conectores se usan solo cuando mejoran la lectura:
- `Asimismo,`: antecedente del mismo nivel.
- `Además,`: antecedente adicional distinto.
- `De igual manera,`: segunda gestión realizada.

## 4. Hallazgos de implementación

### H1 — PC_INFO ya existe para I-01

No hay que crear una nueva acción funcional. En `personal/work.py::actions_for`, cualquier evento `INFORMES.I01_*` ya genera:
- `programa_vencido`;
- `PC_INFO`.

El cambio de I01 corrige el texto para que describa las dos gestiones que el producto ya propone.

### H2 — Dos catálogos paralelos

Existen:
- `src/nurus/personal/textos_base.json` — usado por CSMP Assistant personal;
- `src/nurus/rus/textos_observaciones.json` — usado por el núcleo `nurus.rus`.

La implementación debe actualizar ambos en el mismo cambio y añadir una prueba de paridad de las claves/textos comunes, o definir una fuente canónica única en una fase posterior. No dejar uno atrasado.

### H3 — Migración obligatoria de configuración

Las configuraciones personales guardan copias editables de los textos. Además, la validación actual exige que las variables de cada texto coincidan con las del catálogo base.

MAYORIA_EDAD, PROXIMA_MAYORIA y C08_FICHA_FAE dejan de usar `{PNOMBRE}`. Si se actualiza el catálogo sin migración, una configuración antigua puede fallar antes de poder abrirse.

Implementación requerida:
1. subir `revision_textos`;
2. migrar textos predeterminados antiguos a los nuevos antes de la validación estricta o admitir temporalmente el esquema anterior durante la migración;
3. preservar textos personalizados del usuario;
4. probar una configuración dev11 real/antigua y una configuración personalizada.

### H4 — C09_BASE_BREVE contradice el criterio aprobado

El código actual antepone `Medida revisada.` cuando no hay hallazgo principal pero sí complementarios. El criterio aprobado indica eliminar esa apertura cuando ya existe un hallazgo concreto.

No se recomienda dejar el texto vacío porque la validación exige textos no vacíos. La solución menos invasiva es conservar la entrada por compatibilidad y modificar `reglas_cumplimiento.py` para no insertarla cuando `frags` ya contiene observaciones.

### H5 — Orden actual de CUMPLIMIENTO no coincide con el nuevo orden

Hoy CUMPLIMIENTO agrega curador y oído, luego fichas y finalmente audiencia. El contrato aprobado exige:
- hallazgo principal;
- oído/audiencia;
- fichas/curador/recomendaciones.

La implementación debe ordenar por categoría, no por el orden accidental de evaluación.

### H6 — Evitar repetición de programa requiere composición contextual

Se aprobó no repetir `{PROGRAMA}` cuando el referente sea inequívoco. Los textos estáticos no pueden resolver todos los casos. Ejemplo: C03 + C10 puede repetir el mismo programa.

No debe construirse un sistema gramatical complejo. La implementación debe limitarse a combinaciones conocidas y seguras; si no hay certeza, se conserva la repetición antes que producir una frase ambigua.

## 5. Criterios que no cambian

- No se modifican umbrales ni condiciones de disparo durante esta tarea de redacción.
- No cambia el prefijo `PrimerNombre SIG:`.
- No cambia la fecha extensa.
- No cambia la normalización de nombres de programa.
- No se agregan envíos automáticos: solo borradores Outlook.
- No se escribe en RUS/SATURNO.
- Las observaciones describen gestiones realizadas/propuestas por el flujo vigente; no deben inventar acciones.

## 6. Criterio de aceptación para la implementación posterior

Antes de integrar:
1. prueba exacta de cada texto aprobado;
2. pruebas combinatorias representativas de CUMPLIMIENTO;
3. prueba de OIDO + audiencia agrupados;
4. prueba de ausencia de `Medida revisada.` en C09 con complementarias;
5. prueba I01 => texto de proyecto + correo y acciones `PC_INFO` + `programa_vencido`;
6. migración desde configuración dev11 sin pérdida de personalizaciones;
7. paridad entre catálogos personal/rus;
8. CI Windows 3.12–3.14;
9. prueba manual con una planilla real antes de avanzar main.
