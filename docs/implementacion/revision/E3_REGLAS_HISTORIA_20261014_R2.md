# Revisión E3, ronda 2 — correcciones de la decisión u (14 de octubre de 2026)

Diff revisado: `35234c0..4f572ed` (`claude/lucid-sagan-hw9ul4`), que aplica la decisión u del usuario («1. A. 2. B. 3. A. 4. B.»)
sobre `feat/reglas-historia-asistente`. Ronda 1: `E3_REGLAS_HISTORIA_20261014.md`.

**Independencia (decisión j): esta ronda NO es independiente.** La misma sesión que hizo la ronda 1 implementó las
correcciones, por orden expresa del usuario, y ahora las revisa. Sirve como control, pero no reemplaza un E3 de otra sesión
o modelo. Nada de esto se declara aceptado.

VEREDICTO: **APRUEBA CON CAMBIOS**

Las cuatro decisiones quedaron implementadas como se pidieron y los controles pasan. Hay 2 hallazgos mayores nuevos,
ambos en Informes. El primero lo introduce la decisión 2B tal como yo la recomendé: un informe ingresado antes del
vencimiento queda ignorado. Requiere una decisión del usuario.

## Controles mecánicos (sobre `4f572ed`)

| Control | Resultado | Número |
| --- | --- | --- |
| `tools/estado.py validar` | PASA | — |
| `tools/check_links.py` | PASA | 0 enlaces rotos |
| `tools/check_real_ids.py` | PASA | 0 RUT reales |
| `ruff check` (alcance CI) + `motor/analisis_historia.py`, `motor/reglas_historia.py` | PASA | 0 |
| `ruff format --check` | PASA | 320 archivos |
| pytest `tests` | igual a la base | 952 pasan (+6 pruebas nuevas), 42 fallan, 8 errores de recolección; los 50 nodeids son idénticos a `f4e3c82` (tkinter) |
| pytest `descargador` | igual a `35234c0` | 296 pasan, 25 fallan, 4 errores; nodeids idénticos |
| golden `--expected` (regenerado) | PASA | «PARIDAD PASÓ»; contra el golden anterior solo cambia la clave nueva `reglas_rus` (PARIDAD FALLIDA: `reglas_rus`), las 3 filas P0 y los 17 productos no cambian |
| `grep -rn "\.Send(" src` | vacío | 0 |

## Verificación de las decisiones (N1, con casos ejecutados)

| Decisión | Caso | Antes (`35234c0`) | Ahora (`4f572ed`) |
| --- | --- | --- | --- |
| 1A J2 | Informes, 2 vencidos mismo NNA, **Excel** | sin J2 | `HISTORIA.J2_ACUMULADOS` (igual que con paquete) |
| 1A R6 | Espera, mismo NNA y programa ×2, **Excel** | sin marca | `R6_DUPLICIDAD` nivel 1 (igual que con paquete) |
| 1A E10/T5 | ya salían sin historia | — | sin cambio; ahora cubiertas por el golden `reglas_rus` |
| 2B I1 | vence 20-sep; ingreso de informe 01-jul | `actions=[]`, I01_CONTRADICHA | `I01_VENCIDO_GENERAL`, `['programa_vencido','PC_INFO']`, sin marca |
| 2B I2 | vence 20-sep; Prórroga 25-sep | `actions=[]`, I01_CONTRADICHA | I-01 + solo constancia, acciones intactas |
| 3A R6 | hermanos (RUT distintos), misma causa y programa | nivel 1 ×2 | sin marca; con historia conservan `R3_MULTI` |
| 3A RUT | `11.111.111-1` vs `11111111-1` | — | se reconocen como el mismo NNA |
| 4B regla 3 | espera 40 días (desde el 04-sep); ingreso efectivo 20-jun | `actions=[]` | E-05 + `['programa_espera','PC_IE']` |
| 4B borde | ingreso efectivo el 04-sep (día de inicio) | — | cuenta (regla 3 y marca nivel 2) |
| 4B sin días | T ESPERA vacío o negativo | — | no se filtra (regla 3 actúa), como se decidió |
| H5 | T2 cita IE f20; regla 3 | f20 citado 2 veces | f20 una vez; la regla 3 cita solo f10 |
| H5 búsqueda | órdenes f10 (01-jul) y «s/f» (01-ago); término 10-ago | emparejaba con f10 | empareja con la «s/f» (más cercana) |

## Hallazgos (más graves primero)

### [R2-1] mayor — 2B ignora un informe ingresado antes del vencimiento y pide cuenta de algo que ya consta
`src/nurus/personal/motor/reglas_historia.py` (`informe_historia`): `lista = [t for t in lista if t.fecha >= _d(fecha_vencimiento)]`.

- **Hecho (N1)**: vence el 20-sep-2026; historia `Ingreso informe de cumplimiento X` del 15-sep-2026, folio 40. Resultado:
  `INFORMES.I01_VENCIDO_GENERAL`, `['programa_vencido','PC_INFO']`, sin constancia ni marca. Se manda un proyecto
  pidiendo cuenta de un informe que la historia muestra ingresado 5 días antes del plazo.
- **Inferencia (N2)**: los programas suelen entregar antes del plazo, y RUS puede seguir mostrando «vencido» si no se
  actualizó. Supuesto: el informe del 15-sep corresponde al período que vencía el 20-sep. El filtro «del día del
  vencimiento en adelante» (opción que yo recomendé) resuelve el caso del período anterior, pero rompe este.
- **Opciones** (las decide el usuario):
  - (a) Ventana: cuenta un ingreso desde N días antes del vencimiento. Nuevo umbral configurable `informe_ventana`, por
    ejemplo 30. Costo: un umbral más. Riesgo residual: informes con periodicidad menor que N.
  - (b) Tomar la fecha de inicio del período, si RUS la trae. No tengo datos de que la nómina de Informes la incluya.
  - (c) Quedarse con lo implementado (≥ vencimiento) y aceptar que los ingresos tempranos se ignoren.
- **Recomendación**: (a), con N = 30 hasta tener datos reales. El parche está en el anexo B.

### [R2-2] mayor — I3 (escrito sin ingreso) sigue sin filtro de fecha y sigue quitando PC_INFO
`reglas_historia.py` (`escrito_sin_ingreso`), `reglas_informes.py` (`if vencido and (i1 or i3)`).

- **Hecho (N1)**: vence el 20-sep; historia `Inf. Cump Programa Ambulatorio` del 01-jul, folio 30, sin «Ingreso informe…»
  posterior. Resultado: `HISTORIA.C10_ESCRITO` + `I01_SIN_ENVIO_GENERAL`, `actions=[]`, marca `I01_CONTRADICHA`. Es el
  mismo defecto de H3 (ronda 1), pero en I3, que la decisión 2B no cubrió.
- **Propuesta**: aplicar a I3 el mismo criterio que se fije en R2-1 (anexo B). En Cumplimiento, C10 no cambia, porque allí
  la constancia sirve para regularizar la nomenclatura y no recorta acciones.

### [R2-3] menor — REITERA depende de la bitácora, no de la historia, y no está declarado
`reglas_historia.py` (`reitera`).

- **Hecho (N1)**: con bitácora verificada (paquete) y sin historia, una observación previa «No registra curador ad litem»
  cambia el texto de `COMUN.CURADOR` por «Se reitera observación de fecha…». Conserva el evento original, así que las
  acciones no cambian.
- **Propuesta**: declararlo junto a la decisión u.1 como regla que no exige historia (solo con paquete, porque requiere
  bitácora). Es un caso más de A3 que el usuario no ha resuelto. [CONFLICTO_ABIERTO]

### [R2-4] menor — en la regla 3 el desfase entre la fecha de la nómina y el corte es conservador
`ingreso_efectivo_espera`: `inicio = corte - T ESPERA`.

- **Inferencia (N2)**: T ESPERA se calcula a la fecha de exportación de RUS. Si el corte (`as_of`) es k días posterior, el
  inicio estimado queda k días tarde. Un ingreso efectivo de los primeros k días de espera no desplaza E-05, así que el
  error favorece conservar las acciones. Riesgo bajo; no se propone cambio.

### [R2-5] menor — una orden de búsqueda con dos cierres se sigue citando dos veces
`busqueda_historia`. Orden f10, término f20, deja sin efecto f30: las dos frases nombran la orden f10. Puede ser lo
deseado, porque cada cierre necesita su referencia. No se cambió.

## Revisión de código (lectura adversarial del diff)

- `rut_clave`: deja solo dígitos y K (y pasa `k` a mayúscula). Un RUT numérico sin DV (celda int) se compara consigo
  mismo: es coherente dentro de la nómina. Sin RUT no se afirma duplicidad (`bool(rut)`).
- `duplicidades`: `OTRO_REGISTRO` se asigna a todos los registros de causas con más de un registro, y `R3_MULTI` usa
  `duplicidad() is not None`, así que los hermanos conservan la aclaración. `marcar_duplicidad` solo marca `MISMO_PROGRAMA`.
- `Work._prepass`: ya no depende de `historia_by_record`. El modo `EGRESADOS/OTRO` sigue excluido. El camino Excel usa
  `batch.records`, que es el mismo objeto del paquete.
- `reglas_espera`: `espera` se calcula antes de la regla 3. La incidencia G-05 y el umbral E-05 usan la misma variable,
  así que su comportamiento no cambia.
- `constancia`, `prorroga` y `diagnostico_dce` filtran lo ya citado antes de componer. `VARIOS N` cuenta solo trámites no
  citados, coherente con «se cita una vez».
- `informe_historia`: la firma cambió (sin `nombre`). El único llamador es `reglas_informes.py`. Ver R2-1.
- `prorroga_informe`: usa `constancias`, que a su vez usa `_h()`, así que sin historia con datos devuelve `[]`.
- Golden `reglas_rus`: los libros se escriben en `TemporaryDirectory` y no cambian los 17 productos; los RUT son
  ficticios (`check_real_ids` PASA). `avisos` se serializan como listas.
- Sin escritura a RUS/SATURNO, sin red, sin `.Send(`. Los textos (`textos_base.json`) no cambiaron: el texto de la marca R6
  está en el código, no en el catálogo.

## Pendiente de la ronda 1

- H6 (regla 1, folio de cualquier trámite de esa fecha): espera el nombre exacto de la resolución de ingreso.
- `verificar_wp.py` y la suite con tkinter: siguen pendientes de un entorno Windows.
- Paquete real anonimizado para la clave de unión historia↔registro.

## [CONFLICTO_ABIERTO] nuevos

1. Ventana de I1/I3: ¿desde cuántos días antes del vencimiento cuenta un ingreso o escrito de informe? (R2-1, R2-2)
2. REITERA sin historia: ¿se declara como regla de bitácora, igual que las reglas RUS de la decisión u.1? (R2-3)

## Anexo A — código revisado (`git diff 35234c0..4f572ed -- src`)

```diff
diff --git a/src/nurus/personal/motor/analisis_historia.py b/src/nurus/personal/motor/analisis_historia.py
index 76641c1..1493d6f 100644
--- a/src/nurus/personal/motor/analisis_historia.py
+++ b/src/nurus/personal/motor/analisis_historia.py
@@ -32,7 +32,7 @@ def busqueda(historia):
     pares = []
     for cierre in cierres:
         previas = [o for o in ordenes if posterior(cierre, o)]
-        pares.append((cierre, max(previas, key=lambda o: (o.folio, o.fecha)) if previas else None))
+        pares.append((cierre, max(previas, key=lambda o: (o.fecha, o.folio)) if previas else None))
     vigentes = [o for o in ordenes if not any(posterior(c, o) for c in cierres)]
     return pares, vigentes
 
@@ -63,19 +63,26 @@ def primera_observacion(bitacora, materia, antes_de):
     return None
 
 
+def rut_clave(rut):
+    """RUT comparable: solo dígitos y K, sin puntos, guion ni espacios."""
+    return re.sub(r'[^0-9K]', '', str(rut or '').upper())
+
+
 def duplicidades(registros):
-    """registros: iterable de (record_id, tribunal, rit, programa) de UNA etapa.
-    Devuelve {record_id: 'MISMO_PROGRAMA' | 'OTRO_PROGRAMA'} solo para causas con más de un registro."""
+    """registros: iterable de (record_id, tribunal, rit, rut, programa) de UNA etapa.
+    Devuelve {record_id: 'MISMO_PROGRAMA' | 'OTRO_REGISTRO'} solo para causas con más de un registro.
+    MISMO_PROGRAMA: el mismo NNA (RUT) tiene otro registro en el mismo programa (regla 6, por NNA; sin RUT no
+    se afirma). OTRO_REGISTRO: la causa tiene otros registros (otro programa u otro NNA, p. ej. hermanos)."""
     causas = {}
-    for record_id, tribunal, rit, programa in registros:
+    for record_id, tribunal, rit, rut, programa in registros:
         if not clave(rit):
             continue
-        causas.setdefault((clave(tribunal), clave(rit)), []).append((record_id, clave(programa)))
+        causas.setdefault((clave(tribunal), clave(rit)), []).append((record_id, rut_clave(rut), clave(programa)))
     salida = {}
     for items in causas.values():
         if len(items) < 2:
             continue
-        for record_id, programa in items:
-            repetido = sum(1 for _, otro in items if otro == programa) > 1
-            salida[record_id] = 'MISMO_PROGRAMA' if repetido else 'OTRO_PROGRAMA'
+        for record_id, rut, programa in items:
+            repetido = bool(rut) and sum(1 for _, otro_rut, otro in items if otro_rut == rut and otro == programa) > 1
+            salida[record_id] = 'MISMO_PROGRAMA' if repetido else 'OTRO_REGISTRO'
     return salida
diff --git a/src/nurus/personal/motor/reglas_espera.py b/src/nurus/personal/motor/reglas_espera.py
index 0f5e9dc..2f097dd 100644
--- a/src/nurus/personal/motor/reglas_espera.py
+++ b/src/nurus/personal/motor/reglas_espera.py
@@ -81,7 +81,8 @@ def generar_observacion_espera(row, tribunal, cols, incidencias=None, fila_excel
     principal=False
     H.marcar_duplicidad()
     mn=H.deja_sin_efecto_mn()
-    r3=H.ingreso_efectivo_espera(hoy)
+    espera=get_int(row.get(cols.get('espera'))) if cols.get('espera') else None
+    r3=H.ingreso_efectivo_espera(hoy, espera)
     fres=fecha_valida(row.get(cols.get('resolucion'))) if cols.get('resolucion') else None
     if r3 is None and fres:
         d=(hoy-_date(fres)).days
@@ -90,7 +91,6 @@ def generar_observacion_espera(row, tribunal, cols, incidencias=None, fila_excel
             if folio: frags.append(render('HISTORIA','E04_CON_FOLIO', PROGRAMA=titulo_programa(programa), FECHA_RESOLUCION=fecha_es(fres), FOLIO=folio))
             else: frags.append(render('ESPERA','E04_RESOLUCION_RECIENTE', PROGRAMA=titulo_programa(programa), FECHA_RESOLUCION=fecha_es(fres)))
             principal=True
-    espera=get_int(row.get(cols.get('espera'))) if cols.get('espera') else None
     if cols.get('espera') and (espera is None or espera < 0):
         incidencias.agregar(fila_excel, _rit(row, cols), 'E-05/E-06',
                             'tiempo de espera ausente o inválido (G-05)')
diff --git a/src/nurus/personal/motor/reglas_historia.py b/src/nurus/personal/motor/reglas_historia.py
index c0520a7..b9dfd48 100644
--- a/src/nurus/personal/motor/reglas_historia.py
+++ b/src/nurus/personal/motor/reglas_historia.py
@@ -1,5 +1,7 @@
 """Constancias de historia de la causa: cada función devuelve fragmentos de texto ([] si no aplica)."""
 
+from datetime import timedelta
+
 from . import vocabulario as V
 from .analisis_historia import busqueda, duplicidades, instituciones, posterior, primera_observacion  # noqa: F401
 from .contexto import acumulado, bitacora, current, duplicidad, emitidos, evento, historia, marcar
@@ -49,8 +51,9 @@ def tramites(*nombres):
 
 
 def constancia(lista):
-    """Texto base de uno o varios trámites del mismo nombre; '' si todos ya se citaron."""
-    if not lista or all(_id(t) in emitidos() for t in lista):
+    """Texto base de uno o varios trámites del mismo nombre; omite los ya citados ('' si no queda ninguno)."""
+    lista = [t for t in lista if _id(t) not in emitidos()]
+    if not lista:
         return ''
     emitidos().update(_id(t) for t in lista)
     ult = ultimo(lista)
@@ -80,9 +83,14 @@ def folio_resolucion(fecha):
     return next(iter(folios)) if len(folios) == 1 else None
 
 
-def ingreso_efectivo_espera(corte):
-    """Regla 3. Devuelve los fragmentos que reemplazan E-04/E-05/E-06, o None."""
+def ingreso_efectivo_espera(corte, espera=None):
+    """Regla 3. Devuelve los fragmentos que reemplazan E-04/E-05/E-06, o None.
+    Con T ESPERA válido solo cuenta el ingreso efectivo desde el inicio de la espera (corte - días); un ingreso
+    anterior es de otra medida y no desplaza E-05 ni sus acciones (decisión u, 14-oct). Sin T ESPERA no se filtra."""
     lista = tramites(V.INGRESO_EFECTIVO)
+    if espera is not None and espera >= 0:
+        inicio = _d(corte) - timedelta(days=espera)
+        lista = [t for t in lista if t.fecha >= inicio]
     if not lista:
         return None
     marcar(2, 'R3_INGRESO_EFECTIVO', 'Historia con ingreso efectivo y RUS en Espera')
@@ -100,7 +108,7 @@ def lista_espera_larga(programa, espera):
 
 def marcar_duplicidad():
     if duplicidad() == 'MISMO_PROGRAMA':
-        marcar(1, 'R6_DUPLICIDAD', 'Más de un ingreso al mismo programa en la causa')
+        marcar(1, 'R6_DUPLICIDAD', 'Más de un ingreso del mismo NNA al mismo programa')
 
 
 def reitera(materia):
@@ -227,8 +235,8 @@ def ingreso_efectivo_c1(fecha_ingreso):
 
 def prorroga(corte, egreso_proyectado):
     """C2: «Prorroga» presente en la historia junto al egreso proyectado de RUS."""
-    lista = tramites(V.PRORROGA)
-    if not lista or egreso_proyectado is None or all(_id(t) in emitidos() for t in lista):
+    lista = [t for t in tramites(V.PRORROGA) if _id(t) not in emitidos()]
+    if not lista or egreso_proyectado is None:
         return []
     emitidos().update(_id(t) for t in lista)
     ult = ultimo(lista)
@@ -278,8 +286,8 @@ def diagnostico_dce(programa, vencida):
     """C9: informe diagnóstico ordenado en autos con la medida vencida."""
     if not vencida or intervencion(programa) != 'DCE':
         return []
-    lista = tramites(V.DIAGNOSTICO)
-    if not lista or all(_id(t) in emitidos() for t in lista):
+    lista = [t for t in tramites(V.DIAGNOSTICO) if _id(t) not in emitidos()]
+    if not lista:
         return []
     emitidos().update(_id(t) for t in lista)
     ult = ultimo(lista)
@@ -298,9 +306,13 @@ def escrito_sin_ingreso(programa):
     return [render('HISTORIA', 'C10_ESCRITO', FOLIO_ESCRITO=_folio(escrito), FECHA_ESCRITO=fecha_es(escrito.fecha))]
 
 
-def informe_historia(nombre, fecha_vencimiento, vencido):
-    """I1 (Ingreso informe X) e I2 (Prorroga): constancia y estado que RUS registra. [] si no aplica."""
-    lista = tramites(nombre)
+def informe_historia(fecha_vencimiento, vencido):
+    """I1: «Ingreso informe de cumplimiento X» y estado que RUS registra. [] si no aplica.
+    Con el informe vencido solo cuenta un ingreso del día del vencimiento o posterior: uno anterior es de otro
+    período y no desmiente el vencimiento (decisión u, 14-oct)."""
+    lista = tramites(V.INGRESO_INFORME_X)
+    if vencido:
+        lista = [t for t in lista if t.fecha >= _d(fecha_vencimiento)]
     texto = constancia(lista)
     if not texto:
         return []
@@ -308,6 +320,11 @@ def informe_historia(nombre, fecha_vencimiento, vencido):
     return [texto, render('HISTORIA', llave, FECHA_VENCIMIENTO=fecha_es(fecha_vencimiento))]
 
 
+def prorroga_informe():
+    """I2: la Prórroga queda solo como constancia; nunca desmiente el informe vencido (decisión u, 14-oct)."""
+    return constancias(V.PRORROGA)
+
+
 def informes_acumulados():
     """J2: informes vencidos del mismo ingreso según el pre-pase de Work.analyze."""
     dato = acumulado()
diff --git a/src/nurus/personal/motor/reglas_informes.py b/src/nurus/personal/motor/reglas_informes.py
index 66f533e..8bac633 100644
--- a/src/nurus/personal/motor/reglas_informes.py
+++ b/src/nurus/personal/motor/reglas_informes.py
@@ -26,10 +26,10 @@ def generar_observacion_informes(row, tribunal, cols, incidencias=None, fila_exc
     if finalizada: return componer(pfx, finalizada)
     vencido = dias < 0
     mn = H.deja_sin_efecto_mn()
-    i1 = H.informe_historia(H.V.INGRESO_INFORME_X, fv, vencido)
-    i2 = H.informe_historia(H.V.PRORROGA, fv, vencido)
+    i1 = H.informe_historia(fv, vencido)
+    i2 = H.prorroga_informe()
     i3 = H.escrito_sin_ingreso(programa)
-    if vencido and (i1 or i2 or i3):
+    if vencido and (i1 or i3):
         H.marcar(2, 'I01_CONTRADICHA', 'Informe vencido en RUS contradicho por la historia')
         principal = render('HISTORIA', 'I01_SIN_ENVIO_DCE' if es_dce(programa) else 'I01_SIN_ENVIO_GENERAL', FECHA_VENCIMIENTO=fecha_es(fv))
     else:
diff --git a/src/nurus/personal/work.py b/src/nurus/personal/work.py
index 063d8ac..4f927c8 100644
--- a/src/nurus/personal/work.py
+++ b/src/nurus/personal/work.py
@@ -371,7 +371,7 @@ class Work:
             'CUMPLIMIENTO': generar_observacion_cumplimiento,
             'INFORMES': generar_observacion_informes,
         }[self.mode]
-        duplicity, accumulated = self._prepass(batch, cols, historia_by_record is not None)
+        duplicity, accumulated = self._prepass(batch, cols)
         for number, record in enumerate(batch.records):
             if number % 100 == 0:
                 checkpoint()
@@ -455,9 +455,10 @@ class Work:
         progress('Análisis completo', len(batch.records), len(batch.records))
         return self
 
-    def _prepass(self, batch, cols, with_history):
-        """Duplicidad de ingreso (Espera/Cumplimiento) e informes vencidos acumulados (Informes), por causa."""
-        if not with_history or self.mode not in ('ESPERA', 'CUMPLIMIENTO', 'INFORMES'):
+    def _prepass(self, batch, cols):
+        """Duplicidad de ingreso (Espera/Cumplimiento) e informes vencidos acumulados (Informes), por causa.
+        Solo usa datos de RUS: corre igual con Excel o con paquete, haya o no historia (decisión u, 14-oct)."""
+        if self.mode not in ('ESPERA', 'CUMPLIMIENTO', 'INFORMES'):
             return {}, {}
         from .motor.analisis_historia import duplicidades
 
@@ -465,7 +466,10 @@ class Work:
             return str(record.values.get(cols.get(field, ''), '') or '').strip()
 
         if self.mode != 'INFORMES':
-            rows = [(r.record_id, cell(r, 'tribunal'), cell(r, 'rit'), cell(r, 'programa')) for r in batch.records]
+            rows = [
+                (r.record_id, cell(r, 'tribunal'), cell(r, 'rit'), cell(r, 'rut'), cell(r, 'programa'))
+                for r in batch.records
+            ]
             return duplicidades(rows), {}
         today = date.fromisoformat(self.as_of)
         groups = {}
```

## Anexo B — parche propuesto para R2-1 y R2-2 (no aplicado; requiere decidir N)

`src/nurus/personal/config.py`, dentro de `UMBRALES` (más su etiqueta en `PARAMETER_LABELS`):

```python
    informe_ventana=30,
```

`src/nurus/personal/motor/reglas_historia.py`:

```python
def _desde_ventana(fecha_vencimiento):
    """Primer día en que un ingreso o escrito de informe cuenta para el vencimiento (umbral informe_ventana)."""
    return _d(fecha_vencimiento) - timedelta(days=parametro('informe_ventana'))


def informe_historia(fecha_vencimiento, vencido):
    """I1: «Ingreso informe de cumplimiento X» y estado que RUS registra. [] si no aplica.
    Con el informe vencido solo cuenta un ingreso desde informe_ventana días antes del vencimiento."""
    lista = tramites(V.INGRESO_INFORME_X)
    if vencido:
        desde = _desde_ventana(fecha_vencimiento)
        lista = [t for t in lista if t.fecha >= desde]
    texto = constancia(lista)
    if not texto:
        return []
    llave = 'I1_VENCIDO' if vencido else 'I1_POR_VENCER'
    return [texto, render('HISTORIA', llave, FECHA_VENCIMIENTO=fecha_es(fecha_vencimiento))]


def escrito_sin_ingreso(programa, fecha_vencimiento=None):
    """C10/I3: último escrito de informe sin «Ingreso informe de cumplimiento X» posterior (por folio).
    En Informes con el informe vencido solo cuenta un escrito dentro de la ventana."""
    nombre = V.ESCRITOS.get(intervencion(programa))
    lista = tramites(nombre) if nombre else []
    if fecha_vencimiento is not None:
        desde = _desde_ventana(fecha_vencimiento)
        lista = [t for t in lista if t.fecha >= desde]
    escrito = ultimo(lista)
    if escrito is None or _id(escrito) in emitidos():
        return []
    if any(posterior(i, escrito) for i in tramites(V.INGRESO_INFORME_X)):
        return []
    emitidos().add(_id(escrito))
    return [render('HISTORIA', 'C10_ESCRITO', FOLIO_ESCRITO=_folio(escrito), FECHA_ESCRITO=fecha_es(escrito.fecha))]
```

(requiere `from .contexto import parametro` en el import de `contexto`)

`src/nurus/personal/motor/reglas_informes.py`:

```python
    i3 = H.escrito_sin_ingreso(programa, fv if vencido else None)
```

**Cuidado con las configuraciones antiguas (N1):** `validate` (`config.py:230-233`) exige todas las claves de `UMBRALES`
y se ejecuta en `Configuration.__init__` **antes** de cualquier migración. Si solo se agrega `informe_ventana` a
`UMBRALES`, una configuración guardada sin esa clave queda «Configuración no utilizable». Hay que completar los umbrales
faltantes antes de validar, junto a `complete_texts`:

```python
            self.data = json.loads(self.path.read_text(encoding='utf-8'))
            added = complete_texts(self.data)
            for key, value in UMBRALES.items():
                if key not in self.data.setdefault('umbrales', {}):
                    self.data['umbrales'][key] = value
                    added.append('umbrales.' + key)
            validate(self.data)
```

Pruebas a agregar: ingreso a 5 días del vencimiento (cuenta), a 31 días (no cuenta), escrito a 5 y a 31 días, y una
configuración rev4 sin `informe_ventana` que carga y queda con 30.
