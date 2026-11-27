# Revisión del perfil de Tomé para otros tribunales

27 de noviembre de 2026 · rama `flow-python` · base [`9988d61`](https://github.com/MatiGaete2023/flow/commit/9988d612cf5b9ff1f0e3427ff881d56ac55e6a05).

Revisé cómo se aplican los cambios de análisis del 26 de noviembre cuando se reimporta una captura y se preparan
proyectos Word. También revisé la trazabilidad de la prueba anterior que cambió con la decisión.

| Prioridad | Hallazgo comprobado | Corrección | Cómo verificar |
| --- | --- | --- | --- |
| Alta | La versión de reglas seguía siendo la de antes del perfil de Tomé. Reimportar el mismo ZIP con el mismo corte y configuración podía devolver la sesión anterior con propuestas antiguas. Los productos preparados tampoco reconocían el cambio de versión. | Actualizar la versión de reglas usada por la importación y el control de vigencia de productos. El recálculo reutiliza la conciliación existente para recuperar las decisiones humanas. | Recuperar una sesión ficticia de otro tribunal analizada con la versión anterior y reimportar el mismo ZIP con el mismo corte. Verificar que aparece la propuesta de Espera, se conserva la edición y el producto anterior queda obsoleto. [Prueba de reimportación](../../tests/test_capture_input.py). |
| Media | El Asistente ofrecía un proyecto Word para un tribunal con reglas de Tomé, pero descartaba la selección al preparar el documento sin informar la falta de matriz. | Mostrar un aviso por causa y tipo de proyecto con el nombre original del tribunal. El Asistente utiliza matrices del tribunal correspondiente. | Procesar una fila ficticia de otro tribunal, seleccionar `PC_IE` y comprobar que se informa qué matriz falta. [Prueba de preparación](../../tests/test_reglas_informes.py). |
| Media | La prueba histórica de exclusión de tribunales se sustituyó el 26 de noviembre. Su identificador seguía en la línea base y faltaba declararlo para que el verificador reconociera el reemplazo. | Registrar el identificador exacto y su reemplazo en la lista de bajas de esta rama. | Ejecutar el verificador con las dos listas y comprobar el inventario y la paridad de los 17 productos golden. [Registro del reemplazo](../implementacion/verificacion/flow-python-20261127-eliminadas.txt). |

## Verificación reproducible

La prueba de Historia comprueba la versión publicada por el código al importar. La prueba de regresión de reimportación
mantiene explícita la versión que produjo la sesión anterior.

```bash
python -m pytest -q tests/test_capture_input.py tests/test_reglas_informes.py \
  tests/test_capture_temporal_products.py tests/test_historia_paquete.py
python tools/verificar_wp.py \
  --lista-eliminadas docs/implementacion/verificacion/integracion-20261021-eliminadas.txt \
  --lista-eliminadas docs/implementacion/verificacion/flow-python-20261127-eliminadas.txt
```

Las pruebas usan archivos ficticios. Para comprobar el flujo en Windows, reimporta un paquete del tribunal original
al mismo corte y revisa en Trabajo la propuesta, la edición humana y el nombre del tribunal. Luego revisa la vigencia
de los productos que habías preparado.
