# Guía de prueba real — historia de la causa (2.8.0)

La hace el usuario. Anotar solo conteos y PASA/FALLA; no pegar datos reales en conversaciones ni en el repositorio.

## Pasos y acta

Alcance pequeño: un tribunal, Cumplimiento y Espera, período habitual. Anotar solo conteos y PASA/FALLA.

| Código | Operación | Esperado |
|---|---|---|
| H-01 | Recargar la extensión 2.8.0 y abrir el Descargador | Casilla de historia habilitada tras «Cargar opciones» (con extensión vieja queda deshabilitada con aviso) |
| H-02 | Casilla desmarcada; «Consultar y crear paquete» | Igual que antes; sin `Historia_SITFA.xlsx` |
| H-03 | Marcar historia; iniciar | Aviso «una historia por causa, hasta 600» |
| H-04 | Dejar terminar | Progreso de historia; resumen con conteos |
| H-05 | Abrir `Historia_SITFA.xlsx` | Hojas Trámites, Causas, Vínculos, Control |
| H-06 | 5 causas (2 de Espera): comparar con la ventana SITFA | Mismos folios, fechas, trámites, descripciones y referencias de los últimos 4 meses |
| H-07 | En esas causas, «Trámites anteriores» | Coincide con lo que la ventana muestra antes de «Desde» |
| H-08 | Causa con varios ingresos del lote | Una fila en Causas, varias en Vínculos, trámites sin repetir |
| H-09 | Cancelar; usar «Reintentar lecturas fallidas…» y elegir el mismo lote; luego iniciar una consulta nueva | El reintento conserva historias verificadas del lote elegido; la consulta nueva vuelve a leer todas sus causas |
| H-10 | Revisar Descargas y carpeta del lote | Cero PDF de trámites |
| H-11 | Importar el ZIP en el Asistente | Importa; mismo trabajo que sin historia |
| H-12 | Cerrar sesión SITFA a mitad (opcional) | Restantes FALLIDA con motivo; nada «vacío» |

Acta `docs/aceptacion/ACTA_HISTORIA_<fecha>.json`:

```json
{"tipo": "ACTA_HISTORIA_CAUSA", "fecha": "", "version_descargador": "2.8.0", "commit": "",
 "alcance": {"tribunales": 0, "modalidades": [], "registros": 0, "causas": 0},
 "resultados": {"H-01": "", "H-02": "", "H-03": "", "H-04": "", "H-05": "", "H-06": "", "H-07": "", "H-08": "", "H-09": "", "H-10": "", "H-11": "", "H-12": ""},
 "conteos": {"LEIDA": 0, "FALLIDA": 0, "PENDIENTE": 0, "tramites_en_ventana": 0},
 "observaciones": "", "firmado_por": "", "decision": ""}
```


## Riesgos que solo se confirman con SITFA real (E3, 13-oct)

Si al probar aparece alguno de estos códigos en la columna «Advertencias / error» de la hoja Causas, anótalo (sin pegar datos) y avisa; no se corrige a ciegas.

| Código | Qué significa | Qué hacer |
|---|---|---|
| `HISTORY_REFLECTED` | La ventana real repite los valores cifrados (`a1`…`e5`), por ejemplo en campos ocultos para cambiar de pestaña. La extensión descarta la ventana para no exponerlos. | Si ocurre en todas las causas: informar. Se evaluará enmascarar esos valores en vez de rechazar. |
| `HISTORIA_TRIBUNAL_VISIBLE` | El nombre del tribunal en la ventana no coincide con el del menú (p. ej. «de» de más o de menos). | Si ocurre en todas las causas: informar ambos nombres tal como se ven (solo el nombre del tribunal). Se evaluará una comparación más tolerante. |
| `HISTORIA_POSIBLE_PAGINACION` | La ventana trae indicios de paginación (clase, campo de página o enlace «Siguiente»). La lectura sigue como LEIDA pero puede estar incompleta. | Comparar el total de trámites con SITFA en esa causa (H-06) y avisar. |
