# Revisión de código de `flow-python`, 24-nov-2026

**Punta revisada:** `afc4d12` (20-nov-2026). Desde entonces no hay commits nuevos.
**Alcance:** los commits posteriores a la revisión E3, es decir `93a3631`, `53ee6a2` y `afc4d12`. También se verificó la rama completa con las suites.
**Método:** solo lectura sobre una copia de la rama, con datos ficticios, Python 3.12, Tk 8.6 y pantalla virtual. No se modificó `flow-python`.

## 1. Resumen

- La verificación oficial (`tools/verificar_wp.py`) da **PASA**. Las cifras coinciden con las que informó la revisión anterior:
  - Asistente: 1405 pruebas aprobadas.
  - Descargador: 422 aprobadas y 62 subpruebas.
  - Meta 4: 133 aprobadas.
- La CI rápida de GitHub pasó en `53ee6a2` y en `afc4d12`.
- La mayoría de las correcciones E3 (E3F-01 a E3F-18) y de la revisión del 20-nov están bien hechas.
- Hallazgos nuevos: **1 Alto**, **5 Medios** y varios Bajos. Ninguno es Crítico. Todos los Altos y Medios se reprodujeron con un script o una prueba ficticia.

## 2. Hallazgos

### A-1 · Alta · El día de lectura se toma en UTC y puede dar un «Sin observación» falso

- **Dónde:**
  - `descargador/export_csmp_capture.py:354` (`'hasta': when[:10]`).
  - `src/nurus/personal/revision_bitacoras.py:68` (`_dia(captured_at)`) y líneas 76 y 157.
- **Problema:** la extensión guarda `captured_at` con `new Date().toISOString()`, que está en UTC. Los 10 primeros caracteres se usan como «día de lectura», pero las fechas de la bitácora están en hora de Chile.
- **Escenario:**
  1. La bitácora se lee el 8-oct a las 21:30 de Santiago. En UTC eso es el 9-oct a las 00:30.
  2. El paquete registra `hasta = 2026-10-09`.
  3. Al revisar el período que termina el 9-oct, el Asistente responde `SIN_OBSERVACION`, aunque la lectura no cubrió el 9-oct local.
- **Comprobado:** con un lote ficticio exportado, importado y clasificado, el resultado fue `SIN_OBSERVACION`.
- **Solución:**
  - Convertir `captured_at` a `America/Santiago` con `zoneinfo` antes de tomar la fecha. Hay que hacerlo en el exportador (`hasta`) y en `revision_bitacoras._dia` (para `captured_at` y `fecha_consulta`).
  - Si el valor no trae zona horaria, usar el criterio conservador: restar un día o marcar `NO_VERIFICABLE`.
  - En Windows, `zoneinfo` necesita el paquete `tzdata`. Hay que agregarlo a los locks de `requirements/windows-*.lock`.
- **Cómo verificar:** agregar a `descargador/test_revision_bitacoras_p2.py` una prueba con `captured_at='2026-10-09T00:30:00Z'` y un período que termina el 09-10. Debe dar `NO_VERIFICABLE`. Otra prueba con `2026-10-09T15:00:00Z` debe seguir dando `SIN_OBSERVACION`.

### M-1 · Media · Si falla el reemplazo del ZIP en Windows se pierde el paquete nuevo

- **Dónde:** `descargador/flujo_paquete.py:207-213`.
- **Problema:** primero se mueve la copia «anterior» a `respaldo/` y después se ejecuta `nuevo.replace(target)`, sin reintentos. En Windows ese reemplazo falla con `PermissionError` si el ZIP anterior está abierto en el Asistente, en la vista previa del Explorador o por el antivirus. Cuando falla:
  - el ZIP nuevo, ya validado, se borra junto con la carpeta temporal;
  - queda una copia «anterior» duplicada en `respaldo/`, una más por cada reintento;
  - el proceso queda `INCOMPLETO`.
- **Comprobado:** se simuló `Path.replace` con fallo. Resultado: un duplicado, cero ZIP nuevos y estado INCOMPLETO. El ZIP anterior sí se conserva.
- **Solución:**
  1. Reintentar el reemplazo unas 5 veces, con unos 0,5 s entre intentos, ante `PermissionError`.
  2. Guardar la copia «anterior» con su nombre definitivo solo después de un reemplazo exitoso.
  3. Si el reemplazo falla definitivamente, conservar el ZIP nuevo como `respaldo/Captura_CSMP nuevo <fecha>.zip` y avisar al usuario que cierre el ZIP abierto.
- **Cómo verificar:** agregar dos pruebas a `descargador/test_retomar_paquete.py`:
  - fallo transitorio (2 fallos y luego éxito): el ZIP nuevo queda en su ruta y hay una sola copia anterior;
  - fallo permanente: el ZIP nuevo queda conservado en `respaldo/`, no hay duplicados y aparece el aviso.

### M-2 · Media · Un lote sin registros con Histórico activo queda «parcial» con un aviso falso

- **Dónde:** `descargador/export_csmp_capture.py:561` (`if not entradas or pendientes:`).
- **Problema:** con 0 registros, `entradas == []` cuenta como falso. Eso genera la consulta `historico-respaldo` FALLIDA con `HISTORICO_PENDIENTE` y deja el paquete parcial.
- **Comprobado:** con Histórico activo y 0 registros, `partial=True`. Con Histórico apagado, `partial=False`.
- **Solución:** cambiar la condición a `if entradas is None or pendientes:`. Los intentos sin lectura ya los detecta `_historico_pendientes`.
- **Cómo verificar:** con 0 registros e Histórico activo debe dar `partial=False`. Las pruebas de E3F-05 (todo fallido y caso mixto) deben seguir pasando.

### M-3 · Media · E3F-13 sigue incompleto: el detector de paginación no ve imágenes ni otras flechas

- **Dónde:** `src/nurus/bitacora_html.py:115-127` (`alcance_bitacora`).
- **Problema:** el detector solo mira el texto, `aria-label` y `title`. Por eso:
  - no considera el `alt` de una `<img>` dentro de un enlace, ni el de `<input type="image">`;
  - no reconoce flechas como ▶, ► o ❯.

  Una ventana con un paginador de ese tipo queda `COMPLETA` y puede acreditar una ausencia que no está comprobada.
- **Comprobado:** con HTML ficticio, `img alt="Siguiente"`, `input type=image alt=…`, «▶» y «❯» dieron `COMPLETA`.
- **Solución:**
  - Sumar al rótulo el `alt` del nodo y el de sus `img` descendientes.
  - Ampliar el conjunto de flechas a `▶►❯⟩›»⏵`.
  - Agregar una prueba por cada vía: aria-label, title, role=button, alt e imagen. Hoy un único caso cubre varias vías a la vez, así que quitar cualquiera de ellas no hace fallar ninguna prueba.
- **Cómo verificar:** cada variante nueva debe dar `PARCIAL`. Al revertir cada vía por separado, alguna prueba debe fallar.

### M-4 · Media · La corrección E3F-18 en Meta 4 dejó escapar un error de XML

- **Dónde:** `META4_Bitacoras/scripts/meta4_bitacoras.py:975`.
- **Problema:** la nueva lista acotada de excepciones no incluye `lxml.etree.XMLSyntaxError`, que hereda de `SyntaxError`. `Meta4.py:340` solo captura `ValueError`, así que con una plantilla .xlsx de `workbook.xml` truncado el usuario ve un traceback de Tk en lugar del mensaje.
- **Comprobado:** con el `workbook.xml` truncado la excepción salió sin capturar.
- **Solución:** agregar `SyntaxError` a la tupla. Eso cubre `xml.etree.ParseError` y los errores de lxml.
- **Cómo verificar:** agregar a `META4_Bitacoras/tests/test_interfaz.py` una prueba con una plantilla de XML truncado que espere `ValueError`.

### M-5 · Media · La CI no prueba los commits que solo tocan `docs/` y nunca corre Meta 4

- **Dónde:** `.github/workflows/ci-rapida.yml:52` y la lista de suites que ejecuta.
- **Problema:**
  - Un commit cuyos archivos están todos en `docs/` o son `.md` se trata como «sin código» y salta todas las suites. Las pruebas, sin embargo, leen datos de `docs/`: `MATRIZ_REGLAS_PERSONAL.json`, `evidencia_p0/requisitos.json` y la referencia golden.
    - Ejemplo: `afc4d12` cambió `docs/evidencia_p0/golden/semantic.json` y su CI duró 20 segundos sin ejecutar pruebas.
  - La CI de `flow-python` no ejecuta `META4_Bitacoras/tests` ni las de Anonimizador o Descargador Lite. Los cambios a Meta 4 de `93a3631` (E3F-18) no quedaron controlados.
- **Solución:**
  - Saltar las suites solo cuando todos los cambios sean `*.md`. Alternativa: tratar `docs/**/*.json|csv|html|xlsx` como código.
  - Agregar `xvfb-run -a python -m pytest -q META4_Bitacoras/tests anonimizador/tests descargador_lite/tests` cuando cambien esas carpetas.
- **Cómo verificar:** un commit que solo cambie un JSON de `docs/` debe ejecutar las suites en Actions, y un cambio en Meta 4 debe mostrar su suite en el registro.

### Bajos

| ID | Dónde | Problema | Solución y verificación |
|---|---|---|---|
| B-1 | `descargador/lectura_bitacoras.py:888-898` | Un timeout del puente queda como `HISTORICO_RESPALDO_INVALIDO` y suma para la suspensión del lote. La cancelación durante la pausa se pierde y queda como FALLIDA, porque `detener_si_cancelada()` quedó dentro del `try`. Comprobado con prueba ficticia. | Sacar la espera del `try` o volver a lanzar la excepción si `cancel.is_set()`. Usar el código `HISTORICO_FALLIDO` para los errores sin código. Verificar: el timeout da el código correcto y la cancelación detiene el lote. |
| B-2 | `descargador/extraer_estructura_escritos.py:67,107` | Quedó un resto de E3F-04: el nombre de una etiqueta arbitraria con `onclick` sale literal en el JSON «anónimo». Comprobado. | Usar un catálogo cerrado de etiquetas y escribir «otra» para el resto. Verificar que el literal no aparezca en el JSON de las tres funciones. |
| B-3 | `descargador/historico_manual.py:87` | En el modo manual, la pertenencia todavía se acepta por nombre aunque la ventana muestre otro RUT. Es un resto de E3F-01, visto por inspección. | Aplicar también aquí la comprobación `rut_visible`. Verificar: nombre coincidente con RUT distinto debe dar `INCIERTA`. |
| B-4 | `descargador/flujo_paquete.py:189` | Si el proceso se corta a mitad de la exportación, queda una carpeta `respaldo/.captura-*` huérfana. | Limpiar esas carpetas al iniciar o al retomar. |
| B-5 | `descargador/test_paquete_lote_actual.py` | E3F-17 solo cubre el caso sin Histórico. | Agregar la variante con `leer_historico=True`. |
| B-6 | `src/nurus/capture/escritos.py:246`, `_contenida`, `_instante` | Siguen vivas las mutaciones B02, B13 y B20 de la E3, más varias del código nuevo de `revision_bitacoras.py` (líneas 47-52 y 77). Comprobado. | Agregar las pruebas que propuso la E3: un folio exclusivo de la lectura descartada, los años 1899 y 2200 por la vía textual, y una lectura completa con 1 copia frente a una parcial con 2 copias. |
| B-7 | `src/nurus/personal/revision_bitacoras.py:160` | En una sesión anterior al cambio, el mensaje dice «El período comienza antes…» cuando lo que corresponde es pedir que se reimporte el paquete. El resultado es conservador; solo el texto confunde. | Detectar que falta `desde` y mostrar «Reimporte el paquete». |
| B-8 | `descargador_lite/tests/test_lite.py:26` | Dos pruebas fallan si el repositorio está en una ruta larga, porque el límite de 260 caracteres recorta los nombres esperados. Es un problema del entorno de prueba, no del producto. | Usar una carpeta temporal corta, o calcular el nombre esperado con la misma función de `app/lotes.py`. |
| B-9 | 27 archivos | Siguen fuera del formato Ruff. La CI lo muestra sin bloquear. | Regularizarlos en un commit aparte y después volver obligatorio el control. |

## 3. Revisión de lo hecho por otras IA

Se analizaron los dos informes de la rama: `E3_INFORME_P3C_20261119.md` (veredicto NO PASA, 18 hallazgos) y `REVISION_CODIGO_20261120.md` (7 hallazgos ya corregidos).

**Bien corregido, según la verificación de esta revisión:**

- E3F-01 en modo automático, E3F-02, E3F-03, E3F-06 a E3F-12, E3F-14, E3F-15 y E3F-16.
- La prueba del «pide cuenta» que propuso la E3 se incorporó como `test_e3f_parciales_distintas_aun_con_completa_conservan_pide_cuenta` y pasa.
- Las 7 correcciones del informe del 20-nov:
  - al retomar un paquete se conservan las opciones con las que se creó;
  - el reemplazo del ZIP es atómico ante fallos de la exportación, con la excepción de M-1;
  - el origen del texto se exige como CENTRO verificado;
  - el comando de ayuda ya no usa `subprocess`;
  - los RIT ficticios usan 2099;
  - la CI rápida se activa en `flow-python`;
  - `semantic.json` coincide con `textos_base.json` y el motor no cambió.

**Incompleto o con efectos nuevos:**

- E3F-01 en modo manual (B-3).
- E3F-04 (B-2).
- E3F-05: corrige el problema, pero ahora marca parcial de más (M-2).
- E3F-13 (M-3).
- E3F-17 (B-5).
- E3F-18 introdujo una regresión (M-4).
- Las mutaciones B02, B13 y B20 que la E3 dejó señaladas siguen sin pruebas (B-6).

**Lo que los informes anteriores no detectaron:** A-1 (zona horaria), M-1 (bloqueo del ZIP en Windows) y M-5 (alcance de la CI). La afirmación de que «la CI rápida verifica la rama» es cierta solo para los commits con código; `afc4d12` no ejecutó pruebas.

**Pendientes que siguen abiertos de esos informes:**

- La prueba real de P3c en Windows con SITFA.
- La revisión independiente específica de la extensión.

## 4. Orden sugerido de reparación

1. A-1: zona horaria. Es lo único que puede producir un resultado jurídico incorrecto.
2. M-1 y M-2: paquete y Windows. Conviene resolverlos antes de la prueba real de P3c.
3. M-3 y M-4.
4. M-5: CI. Es barato y evita regresiones futuras.
5. Bajos, con las pruebas de mutación.

## 5. Cómo reproducir esta revisión

```bash
git fetch origin flow-python && git checkout --detach origin/flow-python
python3.12 -m venv .venv && . .venv/bin/activate
pip install --require-hashes -r requirements/linux-py312.lock && pip install --no-deps --no-build-isolation .
export PYTHONPATH=$PWD/src
python tools/check_real_ids.py && python tools/check_contactos.py && python tools/estado.py validar && python tools/check_links.py
xvfb-run -a python -m pytest -q tests          # 1405 passed, 11 skipped, 5 xfailed
xvfb-run -a python -m pytest -q descargador    # 422 passed, 62 subtests
xvfb-run -a python -m pytest -q META4_Bitacoras/tests   # 133 passed
xvfb-run -a python -m pytest -q anonimizador/tests descargador_lite/tests  # 2 fallos solo con rutas largas (B-8)
for f in bitacoras calendario historia; do node descargador/test_${f}_extension.cjs; done
python tools/verificar_wp.py --lista-eliminadas docs/implementacion/verificacion/integracion-20261021-eliminadas.txt  # PASA
```

Después de cada corrección, agregar la prueba indicada en el hallazgo. Comprobar que falla con el código actual y pasa con la corrección, y volver a ejecutar `verificar_wp.py`.
