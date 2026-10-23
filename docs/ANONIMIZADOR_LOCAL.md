# Anonimizador local Excel / Captura CSMP

> **Superado por [`anonimizador/`](../anonimizador/README.md)** (herramienta independiente, todos los formatos). Los defectos de este motor están en la [revisión del 16-oct](REVISION_ANONIMIZADOR_20261016.md); evita usarlo con observaciones reales.

## Ejecutar (Windows 10, sin administrador)

En el repositorio `flow`, rama `feat/ejecucion-plan-20261008`:

```powershell
py -3.12 -m pip install --user "openpyxl>=3.1,<4"
py -3.12 tools/anonimizar_local.py
```

La GUI utiliza Tkinter (habitualmente integrado con Python para Windows), openpyxl y
los módulos `src/nurus/personal/anonymize.py` y `src/nurus/capture/contract.py`.
Para libros antiguos `.xls` instalar también `xlrd>=2.0.1` con `--user`.
Si instalaste dependencias de flow, no se necesitan otras. No requiere permisos
de administrador ni conexión a Internet. No incorpora servicio de IA ni OCR.

1. Selecciona un Excel (`.xls`, `.xlsx`, `.xlsm`) o un ZIP `CSMP_CAPTURE_V1`
   del Descargador SITFA.
2. Elige **Conservar observaciones** (con sustitución de identidades detectadas)
   o **Estricto** (omite textos libres enteros).
3. Presiona **Anonimizar**. Se crea `Copia_CSMP_anon_<aleatorio>.xlsx/.zip`
   en la misma carpeta. Ni el nombre ni el contenido del original se muestran
   en la ventana.

## Alcance y garantías técnicas

El Excel se reconstruye solo con valores; mantiene hojas/encabezados estructurales
y celdas numéricas/fechas usadas por reglas, pero elimina comentarios, imágenes,
macros, formatos, propiedades, enlaces, fórmulas y otros objetos. Hojas con títulos
no reconocidos se renombran sin volcar el título original.

El ZIP debe ser **CSMP_CAPTURE_V1**, íntegro y validable por
`nurus.capture.contract.read_capture`. El resultado se vuelve a validar durante
`write_capture`. No extrae ZIP ni lo ejecuta. Se sustituyen identificadores
remotos y locales de manera consistente, manteniendo claves foráneas de consulta,
registro, ingreso, bitácora, historia y documentos, huellas sintéticas de
procedencia y vínculos con el XLSX consolidado opcional. Las huellas de origen
se reemplazan por HMAC con clave efímera de cada ejecución. La captura de salida
mantiene sus tablas y sus estados de cobertura.

La reconstrucción del manifiesto descarta metadatos adicionales y elimina referencias
al productor original. `contains_sensitive_data=true` deliberadamente: **no
certifica que el archivo se haya vuelto inocuo**.

### Rechazos deliberados

Se rechaza el paquete entero **sin crear copia** cuando trae documentos u originales
binarios (PDF, HTML, adjuntos de origen, etc.), extensiones de contrato desconocidas,
columnas técnicas futuras o claves requeridas no reconocidas. No se excluyen
silenciosamente miembros, porque eso podría invalidar relaciones y hashes.

No se procesan ZIP genéricos del Descargador que no implementen `CSMP_CAPTURE_V1`.
Para este caso, debe generarse primero el ZIP de captura interoperable.

### Límite de privacidad

El modo conservador combina sustitución de identidades conocidas y expresiones
regulares para identificadores y nombres no catalogados. No puede garantizar
detección de apodos, errores ortográficos, nombres aislados o descripciones
indirectamente identificables. **Revisión manual de todos los textos conservados
obligatoria antes de compartir archivos.** El modo estricto minimiza ese riesgo,
pero fechas y combinaciones de hechos pueden permitir reidentificación indirecta.
Solo usar información del sistema en el equipo y entorno autorizados.

No hay logs ni carga de red. El original nunca se edita y las copias usan ruta
nueva, sin sobrescribir. La interfaz muestra errores genéricos para evitar
exponer datos sensibles desde mensajes de excepción.

## Pruebas sintéticas

```powershell
py -3.12 -m pytest -q tests/test_anonimizar_local.py tests/test_anonimizar_excel.py tests/test_anonimizar_casos.py
```

Las pruebas construyen originales ficticios, comprueban hashes antes/después,
manifiestos revalidables, referencias consistentes, Excel consolidado y rechazo
de binarios. Se requiere además revisión E3 independiente por tocar el
anonimizador, según `PLAN_VIGENTE.md`. CI no constituye aceptación de uso
institucional, que solo corresponde al usuario.
