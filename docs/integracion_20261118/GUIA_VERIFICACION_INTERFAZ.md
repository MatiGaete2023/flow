# Guía breve para verificar en la interfaz (plan 18-nov-2026)

No hay interfaz ni Windows en el contenedor de desarrollo: **esto no se ha ejecutado visualmente**. Ejecutarla con datos
ficticios (nunca reales) en Windows. La aceptación institucional la firma solo el usuario, en un acta.

## Preparación
1. Generar paquetes ficticios: `python rust/oraculo/generar_paquete_integracion.py` y `python rust/oraculo/generar_paquete_estructura.py`
   (quedan en `rust/crates/flow-trabajo/tests/datos/paquete_integracion.zip` y `paquete_estructura.zip`).
2. Abrir cada ZIP en el Asistente Python y en Flow (Rust) con corte `2026-10-08`.

## Qué mirar
| Paquete / fila | Debe verse | No debe verse |
|---|---|---|
| `paquete_integracion`, Espera, persona del histórico (`r0`) | «Se hace presente que Persona cuenta con ingreso efectivo al programa FAE Biobío.» una sola vez; marca/gestiones de siempre | `PDE Mulchén` (egresado) ni `PRM Laja` (programa actual) |
| `paquete_integracion`, informe `r2` | «Pendiente de resolver, informe de avance.», sin proyecto de pide cuenta ni correo | Texto «vencido… se remite proyecto pidiendo cuenta» |
| `paquete_integracion`, informe `r3` | Proyecto de nomenclatura (`NOMENCL`) y frase de asociación si corresponde | Pide cuenta |
| `paquete_estructura`, ingreso con 5 fechas | 1.ª: revisar asociación; 2.ª: pendiente; 3.ª: nomenclatura; 4.ª y 5.ª: pide cuenta | Las 3 primeras en el Word/Excel de pide cuenta |
| `paquete_estructura`, dos ingresos de la misma persona/programa | Ingreso de informe citado como constancia de la causa; **acciones conservadas** | Cancelación de productos por una entrega ambigua |

## Pasos
1. En **Trabajo**, abrir la observación de cada fila anterior y comparar con la tabla.
2. En **Correos** preparar «Informes pendientes»: el Excel adjunto solo lista las fechas exigibles.
3. En **Resoluciones** preparar proyectos: un `PC_INFO` por tribunal+causa con solo las filas exigibles; un `NOMENCL` aparte.
4. Guardar y reabrir la sesión; recalcular tras editar una observación a mano: la edición se conserva.
5. Flow (Rust): en el detalle de Trabajo, la fila con «otro programa» muestra «Marcar «X» como consignada»; al pulsarla la constancia desaparece de
   la observación y la sesión la recuerda (se conservan tus ediciones). Python: no hay botón; usar `python tools/consignar.py SESION --listar`.
6. Histórico real: guardar la ventana «Historial Menores» como HTML y correr `flow historico …` (ver `REGISTRO.md`); debe leer la tabla
   «Ingreso Historico Protección» y **no** la «Ingreso Historico(LRPA)».
