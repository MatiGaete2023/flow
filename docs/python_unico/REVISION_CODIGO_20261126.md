# Revisión de la recuperación de paquetes en Flow Python

26 de noviembre de 2026 · rama `flow-python` · base [`afc4d12`](https://github.com/MatiGaete2023/flow/commit/afc4d12ae1adabf21049c809c0d0d8f747e2e306).

La revisión cubrió «Retomar paquete», desde la comprobación de las descargas en disco hasta la inclusión de los
calendarios en el ZIP. Se utilizaron archivos y respuestas ficticias.

| Prioridad | Hallazgo e impacto | Corrección aplicada | Cómo verificar |
| --- | --- | --- | --- |
| Alta | «Retomar paquete» reutilizaba la descarga principal si su resumen decía `VALIDADA`. Un original dañado o perdido reaparecía en cada intento y la exportación volvía a fallar. | Usar el verificador del lote, que comprueba los manifiestos, los archivos y sus huellas. Repetir la consulta principal cuando la comprobación falla. Los archivos anteriores quedan en su carpeta. | Generar un paquete parcial ficticio, sustituir un Excel de su respaldo y retomar. Comprobar que la descarga nueva usa otra carpeta, que se conserva la anterior y que el ZIP actualizado se abre. |
| Alta | Un error en un mes del calendario de informes cortaba la búsqueda de los meses siguientes. Por ejemplo, si noviembre fallaba, diciembre quedaba sin consultar. | Continuar con los meses posteriores cuando falla uno. El ZIP incorpora los calendarios que se obtuvieron y conserva el aviso de calendario incompleto. | Hacer fallar noviembre en una prueba de octubre a diciembre. Confirmar que diciembre se consulta y que el ZIP queda parcial con el aviso. |
| Media | Retomar volvía a descargar todos los meses, incluidos los ya validados. Una nueva falla podía dejar fuera archivos de un mes que ya estaba disponible. | Verificar cada mes de la ventana original y consultar únicamente los meses faltantes, fallidos o dañados. Si todos se pueden reutilizar, el reintento no abre el calendario. | Retomar después de una falla de noviembre y confirmar que la aplicación solo vuelve a pedir noviembre. Quitar luego un original de octubre y verificar que también se repite ese mes. En un reintento con todos los meses válidos, comprobar que el calendario no se abre. |

## Verificación reproducible

Las pruebas de regresión reproducen el fallo de la descarga principal dañada y la interrupción del calendario
en la versión base. Con la corrección, también se comprueba la reutilización de meses y que cambiar la fecha del
reintento conserva la ventana original de informes.

```bash
python -m pytest -q descargador/test_retomar_paquete.py descargador/test_flujo_paquete.py
python tools/verificar_wp.py --lista-eliminadas docs/implementacion/verificacion/integracion-20261021-eliminadas.txt
```

Pruebas dirigidas en [`test_retomar_paquete.py`](../../descargador/test_retomar_paquete.py). El resultado de esta
revisión se limita al flujo de recuperación con datos ficticios. Sigue pendiente la comprobación de Ingreso Histórico
con SITFA en Windows.
