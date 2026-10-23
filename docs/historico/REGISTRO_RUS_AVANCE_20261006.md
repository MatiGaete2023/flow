# Avance hacia el registro real en RUS — 6 de octubre de 2026

El registro institucional sigue pendiente de conexión y validación. Este cambio prepara la evidencia necesaria con una extensión independiente para el Chrome autenticado del usuario y un inspector separado para Windows. No se efectuaron lecturas ni escrituras en una sesión institucional desde este entorno.

## Entrega implementada

- `tools/rus_capture_extension`: captura por acción explícita de la ventana actual, con formularios, valores vivos, opciones, límites declarados, campos conocidos de identidad y tablas visibles. Conserva el origen institucional del Descargador. No modifica el DOM ni realiza consultas de red. No extrae cookies ni contraseñas.
- `nurus.personal.rus_capture`: valida formato, origen y huella del contenido; genera inventario de campos y requisitos pendientes. No duplica el texto de observación en su informe.
- `nurus.personal.rus_capture_app`: inspector gráfico separado, sin cargar un trabajo CSMP, disponible como `RUS_Inspector.exe` en el artefacto de Windows. Guarda un informe nuevo sin reemplazar originales.
- `registration.FormEvidence`: el protocolo de guardado exige campos efectivos coincidentes, evidencia reciente y límite observado. Se retiró el supuesto institucional de 2.000 caracteres. Un `maxlength` declarado se valida en unidades UTF-16; `None` representa ausencia observada de ese atributo, no conocimiento de los límites adicionales del servidor.
- El diario conserva la huella, fecha y límite del formulario antes de intentar guardar. Una consulta que envejece durante la preparación bloquea el intento. Los recibos y la recuperación del intento actual se mantienen para evitar duplicados; no se reintroducen históricos ni comparación de planillas.

## Uso

La instalación y las tres pantallas útiles se explican en [LEEME de la extensión](../tools/rus_capture_extension/LEEME.md). La primera captura puede hacerse sin registrar nada. Las capturas contienen información de la causa; revisarlas antes de compartirlas.

Con Python y el paquete CSMP instalado, el inspector gráfico se inicia así en PowerShell:

```powershell
python -m nurus.personal.rus_capture_app
```

También puede generar el informe por consola:

```powershell
python -m nurus.personal.rus_capture RUS_formulario.json --salida RUS_formulario_inspeccion.json
```

La huella detecta modificaciones accidentales, pero no autentica la sesión ni convierte el JSON en un recibo de registro. Ninguna captura importada habilita por sí sola `submit`.

## Validación y próximo paso

Las pruebas del protocolo incluyen formulario incompatible, evidencia ausente o antigua, límites distintos, caracteres no BMP, procedencia conservada antes del guardado y consulta vencida durante la preparación. Las pruebas del inspector verifican integridad, origen y no reemplazo de informes. `tools/smoke_rus_capture.py` prueba el capturador con Chromium real y un sitio ficticio: texto editado, opciones seleccionadas, destino, identidad conocida, omisión de valores sensibles, ausencia de solicitudes y ausencia de modificaciones.

Estas pruebas no validan permisos de la extensión en el perfil real del usuario, autenticación institucional, paginación RUS ni guardado real. La siguiente etapa requiere las capturas actuales para definir nombres y opciones reales, vínculo de ingreso, usuario efectivo, navegación, paginación y entrada persistida con su fecha de servidor. Después se implementa el lector y se conecta el escritor al protocolo recuperable; la aceptación necesita un registro real autorizado y su relectura.

El inspector es una herramienta de integración del trabajo actual. No ofrece informes de historial ni comparación entre libros.
