# SITFA Lite · Manual de uso

SITFA Lite descarga los XLS que produce RUS/SITFA, con todas sus páginas, y los nombra según la consulta. Si una consulta no tiene registros, guarda un pantallazo PDF de SITFA como respaldo.

**Hace:** Seguimiento (Informe seguimiento), Seguimiento (Búsqueda litigantes), Informes por vencer y Medidas por vencer.
**No hace:** paquetes consolidados, lectura de bitácoras, historia de la causa, calendario de informes por ingreso, ni escritura en RUS.

## Uso rápido (4 pasos)

1. **Conectar:** abre SITFA Lite → **Abrir SITFA en Chrome** → inicia sesión → pulsa el icono **SITFA Lite** en Chrome → pega el código (**Copiar código**) → **Conectar**. Debe verse **● Conectado**.
2. **Qué descargar:** marca una de las 4 consultas y pulsa **Cargar opciones**. SITFA abre esa pantalla en tu Chrome y SITFA Lite trae sus tribunales y filtros.
3. **Opciones:** marca tribunales y filtros (ver tabla). Abajo se indica cuántas consultas se harán.
4. **Descargar:** pulsa **⬇ Descargar**. Al terminar, **Abrir carpeta del lote** o doble clic en un archivo de la lista.

Mientras descarga, la pestaña de SITFA queda cubierta con el aviso «Descargando el lote»; no la uses ni la cierres. Deja abierta también la pestaña de conexión.

## Opciones por consulta

| Consulta | Opciones | Una descarga por cada… |
| --- | --- | --- |
| Seguimiento · Informe seguimiento | Tribunales · Pestañas (Espera, Cumplimiento, Informes, Egresados) · Modalidades (Residencia, Ambulatorio, FAE, DCE) · Informe (Por vencer, Vencidos, Recibidos) para la pestaña Informes · Medidas (Todas, Por vencer, Vencidos) para la pestaña Cumplimiento · Fechas (opcional) | tribunal × pestaña × modalidad × informe/medida |
| Seguimiento · Búsqueda litigantes | Tribunales · Estado · Fechas (opcional) | tribunal × estado |
| Informes · Informes por vencer | Tribunales · Mes(es) · Año | tribunal × mes |
| Informes · Medidas por vencer | Tribunales · Mes(es) · Año | tribunal × mes |

Notas:

- Los nombres exactos de informes, medidas, estados, meses y años son los que entrega tu SITFA al pulsar **Cargar opciones**.
- **Filtrar por fecha:** activa la casilla y escribe Desde/Hasta (DD/MM/AAAA), usa 📅 o los atajos *Este mes*, *Mes anterior*, *Últimos 30 días*.
- **Conservar los otros filtros preparados en SITFA:** para buscar a una persona, centro, RUT o causa concreta, prepara esos filtros primero en la pantalla de SITFA y marca esta casilla. Si no la marcas, SITFA Lite limpia esos filtros y trae todos los registros.
- Las consultas de Informes/Medidas por vencer usan mes y año; no admiten rango de fechas.

## Nombres de los archivos

Cada lote se guarda en una subcarpeta con fecha y hora (`2026-10-30 09-15-02`) dentro de la carpeta elegida (por defecto `Documentos\SITFA Lite`). Formato:

`Consulta - [Pestaña - Modalidad - Informe/Medidas] - Tribunal - [Fechas] - [pN de M].xls`

Ejemplos:

- `Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia Mulchen - 01-10-2026 al 30-10-2026 - p2 de 3.xls`
- `Seguimiento - Informes - FAE - Por vencer (a 25 dias) - Jdo Familia Laja.xls`
- `Seguimiento - Espera - Residencia - Jdo Familia Laja - SIN REGISTROS.pdf`
- `Busqueda Lit - Pendientes - Jdo Familia Mulchen - 01-09-2026 al 30-09-2026.xls`
- `Informes por vencer - Noviembre 2026 - Jdo Familia Mulchen.xls`
- `Medidas por vencer - Octubre 2026 - Jdo Familia Laja.xls`

`- pN de M` solo aparece cuando la consulta tiene varias páginas en SITFA (una XLS por página, todas descargadas en orden). `SIN REGISTROS.pdf` es el pantallazo cuando la consulta no devuelve filas: muestra la pantalla de SITFA con la selección aplicada y una franja azul «Consulta solicitada · Tribunal … · Pestaña … · Fechas …».

En la subcarpeta también quedan:

- `resumen.json`: estado del lote (VALIDADA / INCOMPLETA), consultas y archivos.
- `verificacion NNN.json`: por consulta, páginas, registros y huella SHA-256 de cada archivo.

## Controles de calidad automáticos

Cada página se verifica antes de pedir la siguiente: la respuesta de SITFA debe corresponder al tribunal, pestaña, modalidad y filtro pedidos; el XLS debe contener exactamente las mismas causas que la página; no se aceptan dos páginas iguales; el archivo escrito en disco se relee y se compara. Si algo no calza, el lote se detiene y lo dice; los archivos ya verificados se conservan. Nunca sobrescribe un archivo existente.

## Cancelar, errores y sesión

- **Cancelar** detiene el lote al terminar la página en curso. Los archivos ya guardados quedan; repite la consulta en un lote nuevo.
- «Sesión expirada»: vuelve a iniciar sesión en SITFA, abre de nuevo la conexión desde el icono de la extensión y repite.
- «No se pudo capturar la consulta vacía»: deja la ventana de Chrome visible (no minimizada) durante el lote; el pantallazo necesita que Chrome pinte la pestaña.
- Si se pierde la conexión (● Sin conexión): pulsa el icono SITFA Lite en la pestaña de SITFA y pega el código de nuevo.

## Privacidad

- La extensión usa tu sesión de Chrome; SITFA Lite no ve tu contraseña ni tus cookies.
- La comunicación es solo local (`127.0.0.1`) y protegida con el código de conexión.
- Los archivos quedan solo en tu equipo, en la carpeta elegida. Evita carpetas sincronizadas con la nube si tu institución lo prohíbe.
