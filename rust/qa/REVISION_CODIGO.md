# Revisión auxiliar de código y puerta E3

La revisión fue realizada por el subagente revision_p6_p7, dentro de la misma raíz
del ejecutor. Es una comprobación separada útil, pero NO acredita E3 del repositorio.
No hay una aprobación formal externa registrada en el PR59.
El usuario autorizó crear el chat revisor independiente01a1eaf6-0e5e-7842-8fba-0dd991c1e639.
Ese revisor emitió CAMBIOS sobre9f4e01b: cuatro hallazgos propios del candidato.
H1: captura habitual siempre PARCIAL; falta prueba de cobertura de fuente.
H2: metadatos anteriores al recorte reintroducen texto fuera de la política.
H3: cotejo por día/texto puede ocultar un evento distinto faltante.
H4: copiar propuesta obsoleta seguía habilitado. La ejecutora corrige H2-H4;
H1 requiere señal de fuente identificable, sin inventar completitud.

`docs/implementacion/PLAN_IMPLEMENTACION.md`, regla E3, dice: «Revisor y Auditor
pertenecen siempre a una raíz de agente distinta de la del Ejecutor. Los subagentes
de una misma raíz cuentan como el mismo agente». La decisión j acota esa puerta a
los cambios de riesgo enumerados, entre ellos contrato de captura. La ampliación
de evidencia es aditiva y autorizada por el usuario, pero se mantiene visible esta
puerta para el candidato; no se inventa una excepción ni una revisión externa.

| Hallazgo de la revisión auxiliar | Corrección y comprobación |
|---|---|
| Metadata de bitácora discrepante con tabla normalizada | Cotejo bidireccional por identidad/fecha/texto; discrepancias No verificable |
| Eventos iguales con IDs distintos colapsados | Conserva ID y momento; solo deduplica identidad inequívoca |
| Cobertura cambia indebidamente al elegir intervalo anterior | Cobertura capturada conservada independiente del criterio |
| Costo cuadrático de consultas por ingreso | Índice compuesto y fixture de2000ingresos |
| Identidad habitual ausente en Excel | RIT/nombre/RUT/programa y procedencia junto a identidad remota |
| Publicación puede reemplazar destino | renameat2 sin reemplazo Linux, MoveFileExW flags0 Windows y prueba de colisión |
| Faltan pruebas de fallo intermedio | Cancelación después del Excel y fallo real al crear JSON; no carpeta parcial |
| Revisada restringida a filas con propuesta | Marca para cuatro categorías, prueba de criterio obsoleto |
| Primer análisis compacto inaccesible | Criterios visibles sin informe; ZIP real→fechas→clic egui a683×384 |
| CtrlS en Revisión exporta Trabajo anterior | Atajo solo Trabajo; prueba TrabajoA+RevisiónB sin archivo de salida |
| Reuso legado impide lectura nueva explícita | Bitácora sinV1/entries no se reutiliza; pruebas propia/hermana/V1/otroslectores/original intacto |

El revisor auxiliar cerró estáticamente los tres últimos hallazgos en458c58c.
La CI Linux de esa punta pasó las tres pruebas nuevas y la matriz de128capturas.
El diff posterior incorpora escenas sintéticas, guía, matriz de controles y empaquetado;
su validación integrada final se registra en el informe de entrega.

La aprobación del usuario del alcance B1/B2 no se presenta como excepción E3.
Esta revisión tampoco sustituye la aceptación funcional del usuario ni pruebas de
Outlook/SITFA con su sesión. El PR permanece borrador y no se fusiona automáticamente.

