# Revisión de código de Flow Python

20 de noviembre de 2026 · rama `flow-python` · base [`93a3631`](https://github.com/MatiGaete2023/flow/commit/93a36312adbd24e2dc96b02dfac45d947193b8b1).

La revisión se concentró en las rutas que cambiaron recientemente y en la verificación de la rama. Incluyó la
recuperación y exportación de paquetes, la revisión de bitácoras, el traspaso de evidencia al Asistente y los controles
automáticos. Los casos se ejecutaron con archivos ficticios y Python 3.12.14.

## Hallazgos y correcciones

| Prioridad | Hallazgo comprobado | Corrección aplicada | Cómo comprobarlo |
| --- | --- | --- | --- |
| Alta | Al retomar un paquete, el alcance se recuperaba desde el control, pero las casillas se tomaban de las preferencias vigentes. Cambiar Histórico, calendarios o el período podía alterar el resultado sin aviso. | Guardar y validar en `proceso.json` las opciones del paquete y la fecha de referencia del calendario. Los paquetes antiguos adoptan las opciones del primer reintento con esta versión. | Iniciar con Histórico, calendarios y un mes. Retomar con las preferencias opuestas y verificar que el paquete usa las casillas y el período iniciales. [Pruebas de recuperación](../../descargador/test_retomar_paquete.py). |
| Alta | Antes de construir el ZIP nuevo, el reintento movía el anterior. Un fallo durante la escritura o validación podía dejar la ubicación habitual vacía u ocupada por un ZIP inválido. | Construir y validar el ZIP en una carpeta temporal del mismo paquete. Luego guardar una copia de la versión anterior en `respaldo/` y sustituir el ZIP en su ruta habitual mediante una operación de reemplazo. | Simular una exportación que alcanza a escribir bytes y falla. Verificar que el ZIP anterior sigue en la ruta habitual con los mismos bytes. En el caso exitoso, abrir el ZIP nuevo y verificar la copia anterior. [Pruebas de recuperación](../../descargador/test_retomar_paquete.py). |
| Alta | La vista «Revisión de bitácoras» mostraba «Con observación» con un texto de una consulta advertida por identidad visible. Tampoco comprobaba el origen del texto y el motivo informaba observaciones al contar fechas distintas. | Exigir origen Centro verificado para la evidencia positiva, excluir las consultas advertidas y mostrar «No verificable» si solo hay evidencia incierta en el período. Informar días con observaciones. Una sesión anterior puede requerir reimportar su ZIP para comprobar el origen. | Importar el ZIP ficticio con un nombre visible discordante y revisar agosto. Debe quedar «No verificable». Cargar dos entradas verificadas del centro en una misma fecha y comprobar que el motivo informa un día. [Prueba del paquete](../../descargador/test_revision_bitacoras_p2.py) y [pruebas del Asistente](../../tests/test_revision_bitacoras.py). |
| Media | El Asistente lanzaba `xdg-open` desde el comando de ayuda. El control del repositorio rechazaba esa importación de `subprocess`. | Manual y video usan la apertura local ya implementada en el Asistente. Windows usa su asociación de archivos. Linux muestra la ruta en el mismo diálogo que para otros productos. | Ejecutar `python tools/verificar_wp.py` y comprobar «Restricciones absolutas»; en Windows, abrir el manual con F1 y el video desde la paleta. [Comandos](../../src/nurus/personal/commands.py). |
| Media | Dos RIT de datos ficticios usaban años distintos del 2099 y el control de privacidad devolvía dos hallazgos. | Cambiar únicamente esos RIT al año reservado para pruebas. | Ejecutar `python tools/check_contactos.py` y `python tools/check_real_ids.py`. |
| Media | La CI rápida excluía `flow-python` del filtro de ramas. La base revisada carecía de ejecuciones asociadas. | Activar la CI rápida en los pushes de `flow-python`. El análisis estático y las suites conservan su carácter obligatorio. La comprobación de formato muestra los incumplimientos heredados sin impedir esas pruebas en esta rama. | Verificar la corrida del commit publicado en [GitHub Actions](https://github.com/MatiGaete2023/flow/actions/workflows/ci-rapida.yml) y el [control de configuración](../../tests/test_audit_tools.py). |

## Verificación reproducible

Con Tk 9.0.4 en Linux, el Asistente registró 1.405 pruebas aprobadas, 11 omitidas y 5 fallos esperados. El Descargador
registró 422 aprobadas y 62 subpruebas aprobadas. Meta 4 registró 133 aprobadas. Anonimizador y Descargador Lite
registraron 128 aprobadas y una omitida en conjunto. Los tres controles Node pasaron.
También pasaron las restricciones, los controles de identificadores, el estado del plan, los enlaces y el análisis
Ruff `E9,F63,F7,F82,F401,F841`.

En Linux, desde la raíz del repositorio, con Python 3.12, Tk y una pantalla virtual. La instalación usa las
dependencias fijadas en el repositorio. Las últimas tres pruebas utilizan Node.

```bash
python -m pip install --require-hashes -r requirements/linux-py312.lock
python -m pip install --no-deps --no-build-isolation .
python -m pip check
python tools/check_contactos.py
python tools/check_real_ids.py
python tools/estado.py validar
python tools/check_links.py
xvfb-run -a python -m pytest -q tests
xvfb-run -a python -m pytest -q descargador
xvfb-run -a python -m pytest -q META4_Bitacoras/tests
xvfb-run -a python -m pytest -q anonimizador/tests descargador_lite/tests
node descargador/test_bitacoras_extension.cjs
node descargador/test_calendario_extension.cjs
node descargador/test_historia_extension.cjs
python tools/verificar_wp.py
```

## Controles que requieren seguimiento

- **Inventario histórico de pruebas.** El verificador ya informaba 38 nodeids ausentes en Asistente y 10 en
  Descargador sobre `93a3631`. Identificar cada prueba renombrada o reemplazada y declarar solo bajas verificadas,
  según el mecanismo de `--lista-eliminadas`. Los nuevos cambios conservan las pruebas preexistentes de la rama.
- **Golden versionado.** Los 17 productos semánticos regenerados dieron el mismo resultado antes y después de esta
  revisión. Ambos difieren del archivo versionado en `config.textos.HISTORIA`. El verificador general sigue en
  «FALLA» por estas dos discrepancias de referencia. Para resolver la de configuración, cruzar exactamente los
  textos distintos con las decisiones del plan antes de actualizar la referencia.
- **Formato.** Quedan 27 archivos fuera del formato Ruff. La CI de esta rama muestra ese resultado. La activación
  del formato como requisito queda para una regularización de esos archivos.
- **Prueba institucional pendiente.** Falta verificar en Windows un lote pequeño con la opción Ingreso Histórico y
  la extensión de la misma entrega. Comprobar por separado una lectura correcta, una lectura fallida y un reintento
  con opciones cambiadas. Las pruebas ficticias y Linux no acreditan la respuesta real de SITFA.
