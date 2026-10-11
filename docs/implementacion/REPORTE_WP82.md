# Continuación técnica — WP-82

11 de octubre de 2026. Base remota `286c28f`, con WP-63/WP-80 integrados.
WP-81 (#36) tiene CI verde pero no revisión E3 registrada; no se integra su propio trabajo.
Se continúa el pendiente G-4 de WP-80, independiente de WP-81, en rama propia desde feat.

## Hallazgos, corrección y razonamiento

| Hallazgo reproducido | Corrección |
| --- | --- |
| Recibo saving/uncertain desaparecía de Pendientes por existir su clave | Reutilizar check_products; conservar «Comprobar en Outlook antes de repetir» |
| Dirección inválida hacía fallar draft_fingerprint y la lista completa | Mostrar el mismo problema que Resultados sin intentar guardar |
| Contenido vacío y adjunto ausente no tenían su aviso concreto | Mostrar issues de la revisión común; no confundir existencia de recibo con producto utilizable |
| Huella de cada adjunto se calculaba sin haber recibos | Omitir esa búsqueda cuando receipts está vacío, manteniendo la comprobación de existencia |

Pendientes ya no mantiene un segundo criterio de correo guardado. Se conservan los pendientes de
observaciones/Word y las categorías Contacto/Nómina/Correo. El estado confirmed/created sin problemas
no solicita repetir; los recibos inciertos requieren comprobar Outlook, nunca repetición automática.

Diez casos nuevos. Siete regresiones fallaron antes de unificar y la optimización falló antes de
aplicarse. Solo datos ficticios. No se modifica el guardado Outlook, la huella efectiva ni el bloqueo
contra duplicados; no se introduce caché de adjuntos que pueda ocultar cambios de contenido.
La optimización resuelve solo el caso sin recibos de G-5; el resto continúa pendiente.

## Verificación

Pendiente de registrar por SHA limpio: suites completas, nueve controles y CI Windows.
Motor/textos y golden17 se conservan. No se acredita Office/SITFA real ni se crean actas.

## Orden para continuar

1. E3 independiente para #36/WP-81 y este paquete; integrar los aprobados en feat y regenerar
   línea base después de cada integración. Resolver conflictos de documentación y regenerar ESTADO.
2. WP-17: etiquetas antiguas y referencias de PR sensibles; no reutilizar clones anteriores a purga.
3. WP-02: pruebas reales agregadas en PC autorizado y acta del usuario.
4. WP-50 (#33): resolver app_base.py contra feat y verificar; integrar tras WP-02 y 10 ingresos.
5. WP-70: acta final y métricas actuales antes de main. Conserva dependencias funcionales confirmadas.

G-3 (ventanas secundarias editables durante operación) requiere prueba GUI propia; G-5 con recibos
requiere medir coste sin perder comprobación de bytes. Extraer analyze/_recalculate exige paridad
sobre ediciones humanas y golden17, sin alterar motor. No eliminar firmas/módulos de bitácoras
sin resolver los motivos funcionales registrados en WP-80.
