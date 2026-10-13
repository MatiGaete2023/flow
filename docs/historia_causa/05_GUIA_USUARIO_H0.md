# Guía H0 — estructura anónima de la ventana de historia (para el usuario)

Unos 15 minutos, **una vez**, en tu PC autorizado. Resultado: 4 archivos `.har` que **no salen de tu equipo** y un
`estructura_historia.json` que sí compartes, después de revisarlo.

## Qué necesitas
- Chrome con tu sesión SITFA y la carpeta `C:\H0_historia\` (fuera de OneDrive).
- Python con `lxml`: la `.venv` del repositorio (creada por `Iniciar.cmd`) o Python 3.12 +
  `py -3.12 -m pip install lxml==6.1.3`.
- `descargador\extraer_estructura_historia.py` de la rama `feat/historia-causa-sitfa`.

## 1 · Preparar Chrome (antes de buscar)
1. En la pestaña de SITFA pulsa **F12**.
2. ⚙ Configuración → Preferencias → marca **«Abrir automáticamente las Herramientas para desarrolladores en
   ventanas emergentes»**. Cierra la configuración.
3. Pestaña **Red** → marca **«Conservar registro»**.

## 2 · Cumplimiento
1. Seguimiento → Cumplimiento → un tribunal → **Buscar**.
2. Abre la historia de una causa como siempre. La ventana se abre con sus propias herramientas. Si la tabla tiene
   más páginas o «ver más», pásalas una vez.
3. En las herramientas **de la ventana de historia**: Red → clic derecho en la lista → **«Guardar todo como HAR con
   contenido»** → `C:\H0_historia\ventana_cumplimiento.har`. Cierra esa ventana.
4. En las herramientas **de la pestaña SITFA**: Red → clic derecho → «Guardar todo como HAR con contenido» →
   `C:\H0_historia\listado_cumplimiento.har`. Luego pulsa 🚫 (Borrar) para vaciar la lista.

## 3 · Espera
Igual que el paso 2 con Seguimiento → **Espera**: `ventana_espera.har` y `listado_espera.har`.
Si en Espera no encuentras cómo abrir la historia desde la fila, anótalo (es un dato importante).
Opcional y útil: una causa con muchos trámites o sin trámites (`ventana_larga.har`, `ventana_vacia.har`).

## 4 · Generar la estructura anónima
En `cmd`, desde la carpeta del repositorio:

```bat
.venv\Scripts\python descargador\extraer_estructura_historia.py -o C:\H0_historia\estructura_historia.json C:\H0_historia\*.har
```

(Sin `.venv`: reemplaza `.venv\Scripts\python` por `py -3.12`.)
- «Estructura guardada en …» → paso 5.
- «Se detectaron N posibles valores…; no se escribió nada» → la herramienta se protegió; avisa solo ese mensaje.
- Si la estructura sale sin la tabla de historia, guarda además la ventana con **Ctrl+S** («Página web, completa»)
  como `C:\H0_historia\ventana_cumplimiento.html` y repite el comando agregando `C:\H0_historia\*.html`.

## 5 · Revisar y compartir
1. Abre `estructura_historia.json` con el Bloc de notas y busca (**Ctrl+F**) un apellido, un RIT, un RUC, un folio
   y una fecha de esas causas. **No debe aparecer nada.**
2. Deben verse etiquetas («Folio», «Fecha Tram»), rutas `/SITFAWEB/….do`, funciones (`ShowHistoria/3:…`), formas
   (`FECHA_DD/MM/AAAA`, `TEXTO_21-60`) y `VALOR` donde había datos.
3. Comparte **solo** `estructura_historia.json`.

## 6 · Originales
Los `.har` contienen datos reales y tu sesión: no los compartas ni los subas. Guárdalos mientras dure el desarrollo
(sirven para repetir el paso 4) y bórralos al terminar. Cierra la sesión SITFA al terminar la captura.
