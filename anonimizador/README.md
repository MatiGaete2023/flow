# Anonimizador CSMP (herramienta independiente)

Anonimiza **localmente** cualquier archivo que entreguen el Descargador SITFA o el Asistente CSMP. No importa
ni necesita ninguno de los dos: solo Python 3.12+ con `openpyxl` (y `xlrd` para `.xls` antiguos). Sin red, sin
servicios de IA, sin registros del contenido y sin escribir jamás sobre el original ni sobre un archivo existente.

## Qué acepta

El formato se detecta por **contenido**, no por extensión (SITFA guarda tablas HTML y XML con extensión `.xls`).

| Entrada | Salida | Tratamiento |
|---|---|---|
| `.xlsx` `.xlsm` `.xls` (BIFF, OOXML, XML 2003, HTML tabular) | `.xlsx` | Se reconstruye solo con valores |
| `.csv` `.tsv` | igual | Mismo delimitador; sin fórmulas activas |
| `.html` `.htm` | `.html` | Esqueleto de tablas y texto; sin scripts, atributos, enlaces, imágenes ni comentarios |
| `.json` `.jsonl` | igual | Recorre toda la estructura (claves, anidados, JSON dentro de celdas) |
| `.ics` | `.ics` | Conserva estructura y fechas; oculta resumen, descripción, asistentes y UID |
| `.txt` `.md` `.log` | igual | Pares «Etiqueta: valor» y texto libre según el modo |
| `.docx` | `.docx` | Texto de cuerpo, encabezados, pies y comentarios; propiedades y vínculos neutralizados |
| `.zip` | `.zip` | Cada miembro por su formato; manifiestos (`MANIFIESTO.json`, `INDICE.txt`) rehechos con tamaños y huellas nuevos |
| ZIP `CSMP_CAPTURE_V1` | `.zip` | Tablas relacionadas con las mismas claves ficticias, consolidado y manifiesto nuevo |
| Carpeta | carpeta `anonimizado_<aleatorio>/` | Recursiva; todos los archivos comparten los mismos seudónimos |

Se rechazan, con mensaje genérico y sin crear copia: PDF, imágenes, ejecutables, Word con imágenes u objetos
incrustados, XML con `DOCTYPE`/`ENTITY`, ZIP cifrados, con rutas inseguras, enlaces o expansión excesiva. Un miembro de
ZIP no admitido **se excluye** (nunca se copia sin tratar) y queda anotado como advertencia; con
`--rechazar-no-admitidos` el ZIP falla completo.

## Uso

Instalación: `python -m pip install -r requirements.txt` (desde esta carpeta).
Ventana (sin argumentos): `python -m anonimizador_csmp`, `Abrir_Anonimizador.bat` o `AnonimizadorCSMP.exe`.

```
python -m anonimizador_csmp ENTRADA [ENTRADA ...] [--modo estricto|conservador] [--salida CARPETA]
                            [--desplazar-fechas DIAS] [--semilla FRASE] [--generalizar-nacimiento]
                            [--rechazar-no-admitidos]
```

Código de salida: `0` todo creado, `1` alguna entrada no se pudo anonimizar, `2` uso incorrecto.

- **estricto** (por defecto): todo texto libre y toda columna desconocida se reemplazan por `[texto omitido]`.
- **conservador**: conserva observaciones sustituyendo identidades conocidas y candidatos a nombre
  (`[identidad omitida]`). Más útil, menos seguro: revisa cada texto conservado.
- **--semilla**: los seudónimos se derivan de la frase con HMAC; así «Persona Ficticia 4812» es la misma persona hoy y
  mañana, entre ejecuciones. Sin semilla, los seudónimos son consecutivos y solo valen dentro de una ejecución.
- **--desplazar-fechas**: suma días a fechas de celdas **y** de textos (ISO y dd-mm-aaaa). Las reglas pueden dar
  otro resultado.
- **--generalizar-nacimiento**: fechas de nacimiento al 1 de enero. Por omisión se conservan porque las lee el
  motor de reglas y combinadas con otros datos pueden identificar a una persona.

## Qué se reemplaza

- RUT por RUT ficticio **con dígito verificador inválido** y el mismo formato; RIT por RIT ficticio que no coincide con
  ninguna causa real del lote; correos, teléfonos, domicilios, enlaces, rutas con nombre de usuario de Windows/Linux.
- Nombres de columnas de identidad (nombre, apellido, NNA, madre, abogado, usuario, defensor, autor, curador…) y de
  pares «Nombre: valor». Cada nombre y cada palabra de un nombre (primer nombre, apellidos) se reemplaza en todo el
  lote, aunque aparezca en otro archivo, con otra mayúscula o sin tildes.
- Identificadores técnicos (causa/ingreso/persona RUS, huellas SHA-256, archivo y hoja de origen) por claves
  estables; cualquier entero de 7 a 12 dígitos por `ID-n`.
- Títulos de hoja, propiedades del documento, nombres de archivo y carpetas con datos: neutralizados.

Se conservan las columnas que lee el motor de reglas (tribunal, programa, fechas, T ESPERA…), números cortos y
estados conocidos (`COMPLETA`, `VALIDADA`…).

## Garantías y límites

La herramienta **falla cerrado ante lo conocido**: antes de publicar cada archivo revisa lo construido y, si queda un
nombre catalogado, un RUT con dígito válido, un RIT real, un correo, un teléfono o una ruta con usuario, no crea la copia
y dice qué categoría (nunca el dato). **No garantiza anonimización absoluta**: no detecta apodos, errores de
ortografía, nombres sueltos de una sola palabra sin rol, descripciones indirectas ni combinaciones de hechos y fechas
que identifiquen a alguien. Revisa a mano todo texto conservado antes de compartir. Una copia anónima no es
aceptación institucional; solo la acredita el usuario.

La copia no conserva formato, fórmulas, macros, imágenes ni metadatos. Las fórmulas se guardan como el valor que
Excel dejó calculado; las que Excel nunca calculó quedan vacías (se avisa).

## Pruebas

```
python -m pip install -r anonimizador/requirements-dev.txt
python -m pytest -q anonimizador/tests
```

Datos solo sintéticos (los RUT con dígito válido se calculan en la prueba, nunca quedan como texto en el
repositorio). `tests/test_engine.py::test_la_herramienta_es_independiente_del_asistente_y_del_descargador` falla si
algún módulo importa `nurus` o el Descargador. La compatibilidad con el lector del Asistente
(`test_compatible_con_el_lector_del_asistente`) se ejecuta solo si el Asistente está instalado.

## Ejecutable portable (Windows 10/11 x64)

`portable/build.ps1` compila `AnonimizadorCSMP.exe` con PyInstaller en un entorno aislado y ejecuta la autoprueba
sintética del propio `.exe` (`AnonimizadorCSMP.exe --self-test informe.json`). Ver `portable/LEEME.md`.
