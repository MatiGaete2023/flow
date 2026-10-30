# SITFA Lite · Manual de instalación

Tiempo estimado: 5 minutos, una sola vez. No requiere permisos de administrador.

## Requisitos

- Windows 10 u 11 de 64 bits.
- Google Chrome instalado, con tu sesión habitual de SITFA (`familia.pjud.cl`).
- No necesitas Python ni otros programas.

## Paso 1 · Ejecutar el instalador

1. Haz doble clic en `Instalar_SITFA_Lite_1.0.0.exe`.
2. Si Windows muestra «Windows protegió su PC» (SmartScreen): pulsa **Más información** → **Ejecutar de todas formas**. Aparece porque el instalador no tiene firma digital comercial.
3. Lee el aviso inicial y pulsa **Siguiente**.
4. Deja la carpeta propuesta (`C:\Users\<tu usuario>\AppData\Local\Programs\SITFA_Lite`) y marca, si quieres, el acceso directo en el escritorio.
5. Pulsa **Instalar**. Al terminar deja marcadas las dos casillas: se abre este manual y el programa.

El instalador crea en el menú Inicio la carpeta **SITFA Lite** con:

- SITFA Lite (el programa).
- Manual de instalación y Manual de uso.
- Carpeta de la extensión de Chrome.

## Paso 2 · Cargar la extensión en Chrome (una sola vez)

La extensión es la que consulta SITFA con tu sesión. Se instala en modo desarrollador porque es de uso local y no está publicada en Chrome Web Store.

1. Abre SITFA Lite y pulsa **Instalar extensión…**. Esto:
   - copia al portapapeles la ruta de la carpeta `extension`,
   - abre `chrome://extensions` en tu Chrome.
2. En `chrome://extensions`, activa **Modo de desarrollador** (interruptor arriba a la derecha).
3. Pulsa **Cargar descomprimida**.
4. En la ventana de carpetas, pega la ruta (Ctrl+V en la barra de dirección) y elige la carpeta `extension`. La ruta es:
   `C:\Users\<tu usuario>\AppData\Local\Programs\SITFA_Lite\extension`
5. Debe aparecer la tarjeta **SITFA Lite · Descargas RUS 1.0.0**.
6. Pulsa el icono de rompecabezas de Chrome y fija (📌) **SITFA Lite** para tenerlo siempre visible.

> No muevas ni borres la carpeta `extension`: Chrome la usa desde ahí. Si reinstalas en otra carpeta, quita la extensión anterior y vuelve a cargarla.

## Paso 3 · Comprobar la conexión

1. En SITFA Lite pulsa **Abrir SITFA en Chrome** e inicia sesión como siempre.
2. Con la pestaña de SITFA activa, pulsa el icono **SITFA Lite** de Chrome. Se abre la pestaña «Conectar con SITFA Lite».
3. En SITFA Lite pulsa **Copiar código** y pégalo en esa pestaña; pulsa **Conectar**.
4. SITFA Lite muestra **● Conectado** en verde.

Listo. Continúa con el **Manual de uso**.

## Actualizar a una versión nueva

1. Ejecuta el nuevo instalador (reemplaza la versión anterior, conserva tus preferencias).
2. En `chrome://extensions` pulsa el botón de recarga (↻) de la tarjeta SITFA Lite.
3. El programa comprueba que la extensión sea de su misma versión; si no lo es, avisa antes de descargar.

## Desinstalar

1. Windows → Configuración → Aplicaciones → **SITFA Lite** → Desinstalar.
2. En `chrome://extensions` pulsa **Quitar** en la tarjeta SITFA Lite.
3. Tus descargas no se borran (están en la carpeta que elegiste; por defecto `Documentos\SITFA Lite`).

## Problemas frecuentes

| Síntoma | Solución |
| --- | --- |
| «No se encontró Chrome instalado» | Instala Google Chrome o abre SITFA manualmente; el resto funciona igual. |
| No aparece «Cargar descomprimida» | Activa **Modo de desarrollador** en `chrome://extensions`. Si tu equipo lo bloquea por política institucional, pide a soporte que lo habilite para esta extensión. |
| «Chrome usa otra versión de la extensión» | Quita la tarjeta antigua en `chrome://extensions` y carga la carpeta `extension` de esta instalación. |
| «El descargador rechazó el código» | El código cambia cada vez que abres SITFA Lite o pulsas Desconectar. Copia el actual. |
| El antivirus bloquea el instalador | Es un programa empaquetado con PyInstaller sin firma comercial. Verifica el SHA-256 publicado junto al instalador y pide la excepción a soporte. |
