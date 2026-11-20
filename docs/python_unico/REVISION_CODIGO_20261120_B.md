# Revisión de código de Flow Python (segunda pasada, rutina programada)

20 de noviembre de 2026 · rama `flow-python` · punta revisada `afc4d12`. Solo datos ficticios. No se modificó código:
este informe propone correcciones y la forma de comprobarlas.

## 1. Estado verificado (reproducido en esta sesión)

Linux, Python 3.12, lock con hashes, Tk 8.6 sobre Xvfb, Node 22, Ruff 0.16.10.

| Control | Resultado |
| --- | --- |
| Asistente `pytest tests` | 1.405 aprobadas, 11 omitidas, 5 xfail |
| Descargador | 422 aprobadas, 62 subpruebas |
| Meta 4 | 133 aprobadas |
| Anonimizador + Descargador Lite | 128 aprobadas, 1 omitida |
| Node (bitácoras, calendario, historia) y `node --check pagina.js` | OK |
| `check_contactos`, `check_real_ids`, `estado.py validar`, `check_links` | OK |
| Ruff `E9,F63,F7,F82,F401,F841` | 0 avisos |
| `ruff format --check` | 27 archivos fuera de formato (deuda heredada, ya informada) |
| `tools/verificar_wp.py --lista-eliminadas …` | **PASA** |
| CI rápida en GitHub para `53ee6a2` y `afc4d12` | success (runs 145 y 146) |

Las cifras coinciden con las de `REVISION_CODIGO_20261120.md`: las correcciones de la revisión anterior no rompen nada.

## 2. Hallazgos nuevos

| ID | Prioridad | Dónde | Problema (comprobado) | Corrección propuesta | Cómo comprobarlo |
| --- | --- | --- | --- | --- | --- |
| R2-01 | **Alta** | `descargador/historico_manual.py:84-88` | En la captura manual (consola, `--url`), si la dirección pide el RUT de la persona, la pertenencia pasa a VERIFICADA **aunque el texto visible muestre otro RUT**. Reproducido: HTML con `Rut : 22.222.222-2`, persona 11.111.111-1, URL con 11111111-1 → `VERIFICADA` con 3 programas, que llegan al paquete como del niño equivocado. Es la misma clase de error que E3F-01, que solo se corrigió en la vía automática. Además, desde el 19-nov se sabe que `PopUpHistorial.jsp` es un envoltorio: la URL no acredita el contenido del iframe. | Dar a la URL solo poder de *desmentir*: `if del_url and del_url != esperado: INCIERTA`. Para verificar, exigir el RUT visible (`rut_visible=persona['rut']` también en modo manual) o, si no hay RUT visible, dejar INCIERTA. Si un RUT visible contradice, INCIERTA siempre. | Nueva prueba en `test_paquete_escritos.py`: HTML con RUT visible ajeno + URL coincidente → `INCIERTA` y 0 programas en Trabajo tras exportar e importar. Ajustar la aserción actual de la línea 150 (`sin_texto` + URL → VERIFICADA) a la decisión que tome el usuario. Script de reproducción en el anexo. |
| R2-02 | Media | `descargador/flujo_paquete.py:208-213` | Por inspección: si `nuevo.replace(target)` falla (en Windows, ZIP abierto por el Explorador, el antivirus o el Asistente), la excepción sale del `with TemporaryDirectory` y **borra el ZIP nuevo ya validado**. El anterior queda en su lugar y su copia en `respaldo/`, así que no hay pérdida de lo antiguo, pero el trabajo del reintento se pierde y el proceso queda INCOMPLETO con `ERROR_ARCHIVO`. | Capturar `OSError` en el reemplazo: mover el ZIP nuevo a `respaldo/Captura_CSMP nuevo <fecha>.zip`, avisar («cierra el ZIP y retoma») y no borrar. Opcional: reintentar 3 veces con pausa corta. | Prueba con `monkeypatch` que haga fallar `Path.replace` solo cuando el destino sea `Captura_CSMP.zip`: el ZIP nuevo debe quedar en `respaldo/` y el anterior intacto. |
| R2-03 | Media | `descargador/test_retomar_paquete.py:80` | La prueba del reintento correcto solo **cuenta** la copia `Captura_CSMP anterior *.zip`; no compara sus bytes. La mutación que reemplaza `copy2` por un archivo vacío **sobrevive**. El informe anterior decía «verificar la copia anterior». | Guardar los bytes del ZIP previo antes de retomar y comparar `read_bytes()` de la copia, o abrirla con `read_capture`. | Repetir la mutación M07 (anexo) y comprobar que la prueba falla. |
| R2-04 | Media | Pruebas del Descargador | Siguen **sin cobertura** mutaciones que la E3 ya marcó (A16, A19, B16): no reiniciar la racha de históricos fallidos tras un éxito (M02), activar «Ingreso Histórico» por defecto (M03) y bajar el tope de 5.000 a 2.000 (M04). También sobreviven quitar `_validar_opciones` (M12) y aceptar RUT de 1 dígito en `_leer_historico` (M13). | Pruebas de frontera: (a) 4 fallas, 1 éxito, 4 fallas → no suspende; (b) `Preferencias()` nueva → `leer_historico is False`; (c) aserción directa `MAX_LECTURAS_POR_LOTE == 5000` y `MAX_HISTORIAS_POR_LOTE == 5000`; (d) `proceso.json` con `opciones.meses = 9` o `historico = "si"` → `PocError`; (e) RUT `1-9` → `HISTORICO_RUT` sin llamada al puente. | Repetir el script de mutaciones del anexo: las cinco deben pasar a DETECTADA. |
| R2-05 | Baja | `src/nurus/personal/revision_bitacoras.py:47-48` | Mutación M09 (quitar la regla «consulta con `warnings` > 0 cuenta como advertida») sobrevive. Hoy no cambia el resultado porque el exportador ya pone `completa=false`, pero la atribución de fechas a «inciertas» queda sin prueba. | Agregar a `test_revision_bitacoras.py` un caso con `revision_bitacoras.warnings = 1` sin incidencia `BITACORA_ADVERTENCIA` y una entrada del centro en el período → `NO_VERIFICABLE`. | M09 debe pasar a DETECTADA. |
| R2-06 | Baja | `flujo_paquete.py:191` | Un corte brusco (corte de luz, cierre forzado) durante la exportación deja `respaldo/.captura-*` con un ZIP a medio escribir. No lo toman los lectores revisados (`preferencias`, `archivo_local`, `pendientes`), pero ocupa espacio y la limpieza por antigüedad lo puede contar. | Al iniciar un reintento, borrar `respaldo/.captura-*` previos (sin seguir enlaces). | Crear la carpeta a mano, retomar y comprobar que desaparece. |
| R2-07 | Baja (documental) | `AGENTS.md`, `CONTINUAR.md`, `linea_base.json` | Los documentos dicen «seis xfail estrictos», pero la suite tiene 5 marcas y el verificador acepta ≤ 6 («xfail 5/6»). | Registrar en `REGISTRO.md` cuándo y por qué quedó en 5, o ajustar el texto. No tocar la línea base sin acta. | `grep -c "pytest.mark.xfail" tests/test_reglas_informes.py` = cantidad documentada. |

## 3. Análisis de los trabajos de otras IA

**Revisión de código del 20-nov (`REVISION_CODIGO_20261120.md`, commits `53ee6a2` y `afc4d12`).** Sus siete correcciones
se reprodujeron y la CI remota pasó. Lo comprobado:

- Retomar conserva opciones: correcto. Las mutaciones M05 y M06 se detectan.
- ZIP nuevo en carpeta temporal y reemplazo: mejora real. Le faltan el caso R2-02 (fallo del reemplazo) y la prueba de bytes R2-03.
- Origen en la revisión de bitácoras: correcto. M08 y M10 se detectan. M09 no (R2-05).
- `open_doc` usa `app._open`, privacidad de RIT, CI de `flow-python` y golden: correctos. El verificador da PASA.
- Formato: se dejó como no bloqueante en `flow-python`. Es razonable, pero la deuda (27 archivos) debe tener fecha.

**Revisión E3 del 19-nov (`E3_INFORME_P3C_20261119.md`, veredicto NO PASA).** Las correcciones E3F-01 a E3F-18
(`93a3631`) están en la rama y sus pruebas pasan. Pendiente:

- La corrección de E3F-01 cubrió solo la vía automática. La vía manual conserva el riesgo (R2-01).
- De las 16 mutaciones sobrevivientes que informó la E3, al menos A16, A19 y B16 siguen sin prueba (R2-04).
- La propia E3 pide una **nueva revisión independiente** de la implementación corregida. Sigue pendiente, igual que la
  prueba real en Windows y SITFA.

## 4. Orden recomendado

1. R2-01, por el riesgo de atribuir un programa al niño equivocado. Es el mismo criterio que E3F-01.
2. R2-02 y R2-03 (recuperación del ZIP en Windows).
3. R2-04 y R2-05 (pruebas de frontera; no cambian el comportamiento).
4. Prueba real de P3c en Windows (lote chico), luego la revisión E3 independiente pendiente.
5. R2-06, R2-07 y la regularización del formato.

## 5. Cómo repetir esta verificación

Los comandos de la sección «Verificación reproducible» de `REVISION_CODIGO_20261120.md`, más:

```bash
# Reproducción de R2-01 (desde descargador/)
python - <<'PY'
import sys, tempfile; from pathlib import Path
sys.path[:0] = ['.', '../src']
from test_paquete_escritos import HISTORICO_HTML
from historico_manual import capturar
url = 'https://ejemplo.invalido/PopUpHistorial.jsp?tipo_popUp=77&RUT_Litigante=11111111&RUT_DV=1'
base = {'tribunal': '113', 'person_id': 'p1', 'nombre': '', 'rut': '11.111.111-1'}
ajena = HISTORICO_HTML.replace('Rut : 11.111.111-1', 'Rut : 22.222.222-2')
with tempfile.TemporaryDirectory() as d:
    print(capturar(Path(d), ajena, base, url=url)['ownership'])  # hoy VERIFICADA; esperado INCIERTA
PY
```

Mutaciones (en un `git worktree` aparte; cada una se revierte tras ejecutar la suite del Descargador o las pruebas de
revisión). Resultado sobre `afc4d12`:

| Mutación | Resultado |
| --- | --- |
| M01 umbral de suspensión 5 → 6 | Detectada |
| M02 la racha de fallas no se reinicia tras un éxito | **Sobrevive** |
| M03 `leer_historico` activado por defecto | **Sobrevive** |
| M04 tope 5000 → 2000 | **Sobrevive** |
| M05 el flujo ignora la opción Histórico guardada | Detectada |
| M06 el flujo ignora los meses guardados | Detectada |
| M07 copia del ZIP anterior vacía | **Sobrevive** |
| M08 la revisión ignora `origin_verified` | Detectada |
| M09 la revisión ignora `warnings` del alcance | **Sobrevive** |
| M10 acepta una sesión antigua sin comprobación de origen | Detectada |
| M11 la URL manual nunca verifica | Detectada (la prueba fija el comportamiento de R2-01) |
| M12 sin `_validar_opciones` | **Sobrevive** |
| M13 acepta RUT de 1 dígito | **Sobrevive** |
