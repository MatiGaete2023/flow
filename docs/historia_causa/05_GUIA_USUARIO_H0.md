# Guía H0 — estructura anónima de la ventana de historia (para el usuario)

Tiempo estimado: 20–30 minutos. Se hace **una vez**, en tu PC autorizado, con tu sesión SITFA. Los archivos
originales **no salen de tu equipo**; solo compartes `estructura_historia.json`, después de revisarlo.

## Qué necesitas

- Chrome con tu sesión SITFA.
- Una carpeta de trabajo **fuera** de OneDrive y de las carpetas de sesión, por ejemplo `C:\H0_historia\`.
- Python con `lxml`. Si tienes la carpeta del repositorio con `.venv` (creada por `Iniciar.cmd`), úsala.
  Si no: instala Python 3.12 de python.org y ejecuta `py -3.12 -m pip install lxml==6.1.3`.
- El archivo `descargador\extraer_estructura_historia.py` de la rama `feat/historia-causa-sitfa`.

## Paso 1 · Preparar Chrome para capturar la ventana emergente

1. En SITFA, pulsa **F12** (Herramientas para desarrolladores).
2. Engranaje ⚙ (Configuración) → **Preferencias** → marca **«Abrir automáticamente las Herramientas para
   desarrolladores en ventanas emergentes»** (Auto-open DevTools for popups). Cierra la configuración.
3. Pestaña **Red / Network** → marca **«Conservar registro» (Preserve log)**.

## Paso 2 · Listado de Cumplimiento

1. Seguimiento → Cumplimiento → un tribunal → Buscar (como siempre).
2. Con la página de resultados a la vista: **Ctrl+S** → tipo **«Página web, completa»** → guarda como
   `C:\H0_historia\listado_cumplimiento.html` (Chrome crea también la carpeta `listado_cumplimiento_files`).

## Paso 3 · Ventana de historia (Cumplimiento)

1. En una fila, abre la historia de la causa como lo haces normalmente. Se abre la ventana «Información de la
   Causa» **con sus propias herramientas de desarrollador**.
2. Verifica que esté en la pestaña **Historia**. Si la tabla tiene más páginas o un «ver más», pásalas una vez.
3. En la ventana emergente: **Ctrl+S** → «Página web, completa» → `C:\H0_historia\ventana_cumplimiento.html`.
4. En las herramientas **de la ventana emergente**: pestaña Red → clic derecho en la lista →
   **«Guardar todo como HAR con contenido»** → `C:\H0_historia\ventana_cumplimiento.har`.
5. Vuelve a la pestaña del listado. En sus herramientas: Red → clic derecho →
   «Guardar todo como HAR con contenido» → `C:\H0_historia\listado_cumplimiento.har`.
   (Aquí queda la petición que abrió la ventana; es la parte más importante.)

## Paso 4 · Repetir en Espera

Igual que los pasos 2 y 3, con Seguimiento → **Espera**: `listado_espera.html`, `ventana_espera.html`,
`ventana_espera.har`, `listado_espera.har`. Si en Espera no encuentras cómo abrir la historia desde la fila,
anótalo (es un dato importante) y sigue.

## Paso 5 (opcional, muy útil)

- Una causa con **muchos** trámites (tabla larga o con varias páginas): `ventana_larga.html` + `.har`.
- Una causa **sin** trámites o que muestre un aviso/restricción: `ventana_vacia.html` + `.har`.

## Paso 6 · Generar la estructura anónima

En una consola (`cmd`), desde la carpeta del repositorio:

```bat
.venv\Scripts\python descargador\extraer_estructura_historia.py -o C:\H0_historia\estructura_historia.json ^
  C:\H0_historia\listado_cumplimiento.html C:\H0_historia\listado_cumplimiento_files ^
  C:\H0_historia\ventana_cumplimiento.html C:\H0_historia\ventana_cumplimiento.har C:\H0_historia\listado_cumplimiento.har ^
  C:\H0_historia\listado_espera.html C:\H0_historia\listado_espera_files ^
  C:\H0_historia\ventana_espera.html C:\H0_historia\ventana_espera.har C:\H0_historia\listado_espera.har
```

(Si no usas `.venv`, reemplaza `.venv\Scripts\python` por `py -3.12`. Añade los archivos del paso 5 si los tienes.)

Resultados posibles:
- «Estructura guardada en …»: sigue al paso 7.
- «Se detectaron N posibles valores de entrada en la salida; no se escribió nada.»: la herramienta se protegió.
  Avisa el mensaje (sin datos); el programador ajusta la herramienta.

## Paso 7 · Revisar antes de compartir

1. Abre `estructura_historia.json` con el Bloc de notas.
2. Con **Ctrl+F**, busca: un apellido de alguna persona de esas causas, el número de un RIT, un RUC, un folio,
   una fecha de trámite. **No debe aparecer nada.**
3. Debe verse: etiquetas como «Folio», «Fecha Tram», rutas como `/SITFAWEB/…​.do`, nombres de funciones
   (`ShowHistoria/3:…`), formas como `FECHA_DD/MM/AAAA`, `TEXTO_21-60`, y la palabra `VALOR` donde había datos.
4. Si todo está bien, comparte **solo** `estructura_historia.json`.

## Paso 8 · Cuidado de los originales

Los `.html`, carpetas `_files` y `.har` contienen datos reales y la sesión: **no** los compartas ni los subas.
Consérvalos en tu equipo mientras dure el desarrollo (sirven para repetir el paso 6) y bórralos al terminar.
Cierra la sesión SITFA al terminar la captura.
