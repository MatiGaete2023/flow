# Matriz de cierre

Ejecutar `py -3.12 -m pytest -q` y repetir manualmente, en datos ficticios, los siguientes casos antes de habilitar cada fase.

| Área | Caso | Resultado exigido |
|---|---|---|
| Destinatarios | contacto exacto, vacío, alias, parecido, ambiguo | solo identidad aceptada completa el destinatario |
| Fechas | vacío, Chile/ISO mezclado, serial Excel 1900/1904, límites | misma fecha y elegibilidad en cada salida |
| Plantillas | variable errónea, cambio no publicado, dos personas | error visible o versión congelada correcta |
| Productos | editar, cancelar, reabrir, generar dos veces | sin pérdida ni duplicación |
| Exportación | fórmula como texto, archivo abierto, disco lleno | texto literal, recuperación y destino íntegro |
| Outlook | no instalado, nuevo/clásico, perfiles, Save incierto | no envío; estado cierto o «por conciliar» |
| Interfaz | 1366×768; 100/125/150 %; teclado | controles accesibles y trabajo preservado |
| Migración | dos ejecuciones, base corrupta, restauración | no duplica ni reemplaza ediciones |

La aprobación exige cero P0 pendiente y que toda excepción tenga producto, fila/origen, estado y evidencia registrable.

