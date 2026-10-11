# Continuar

**Estado:** recuperación del plan del 9 de octubre de 2026 desde `2765b47`.
**Rama base:** `feat/ejecucion-plan-20261008`. Cada WP usa una rama propia;
solo el Orquestador integra después de verificación, revisión y auditoría.

## Traspaso del 10 de octubre de 2026 (sesión claude-sesion-012yjVgL)

**Léelo en este orden:** `PLAN_VIGENTE.md` → `AGENTS.md` → este traspaso → `docs/implementacion/REPORTE_REVISION_20261010.md`
(hallazgos, pasos y enmiendas) → la ficha `docs/implementacion/wp/WP-XX.md` del PR que toques.

### Qué se hizo y cómo

Todo el trabajo ejecutable del plan está hecho y abierto como PR borrador. Cada WP tiene su rama `wp/<ID>-<slug>`, su ficha
en `docs/implementacion/wp/` y su verificación en `docs/implementacion/verificacion/`. Esta sesión **no editó**
`ESTADO.json` en las ramas de WP (propuesta E1); el estado propuesto de cada uno está en su ficha.

| PR | WP / contenido | Base |
| --- | --- | --- |
| #12–#25 | WP-20, 21, 22, 23, 30, 31, 32, 35, 24, 34, 16, 40, 41, 42 | pila: cada una sobre la anterior, la primera sobre `feat` |
| #26 | WP-33 ritmo y checkpoint de lectura | `feat` |
| #27 | plan: reporte, `tools/estado.py`, verificador sin holgura, enmiendas E1–E6 aprobadas, D5–D9 registradas | `feat` |
| #28 | WP-43 paquete del lote actual | `feat` |
| #29 | WP-62 dividir `DiaryRunner.run` | #26 |
| #30 | WP-44 importar último paquete | #25 |
| #31 | WP-45 retirar solicitud de antecedentes (D9) | #30 |
| #32 | WP-51 consolidado en el ZIP (D6) | `feat`, sobre #28 |

De Codex: #4 (WP-14, aprobado y conforme), #8 (WP-15), #11 (WP-60). No los modifiqué.

### Decisiones del usuario (respuesta literal en `ESTADO.json`, rama #27)

- D3 retirar inspector y diario RUS · D4 60 días y 10 sesiones · D8 sin fallos · D6 «Sí, solo el consolidado» · D9 «Retirar
  botón» · D5 «Sí, horizonte de 4 meses» · D7 «Sí, tras el acta final».
- D1 (11 de octubre de 2026): «Reescribir historial». WP-17 se ejecuta solo después de integrar WP-10, WP-12 y WP-20, y
  el push forzado necesita además la autorización escrita del usuario en ese momento; NuRus está fuera del alcance de esta sesión.
- **Pendiente:** D2 (lista de casos probados en uso real; el usuario no marcó ninguno).
- **Revisión:** el usuario pidió que esta sesión revise con subagentes; por E3 es apoyo, no aprobación.

### Qué falta (en este orden)

1. **CI caída.** Desde las 03:24 UTC del 10 de octubre Actions no asigna runners (los jobs fallan en 1–2 s, `runner_id` 0,
   sin logs). Causa probable, no verificada: facturación o minutos. El usuario debe revisar *Settings › Billing › Actions*.
   Hasta entonces la única verificación es la local. Si la CI vuelve y falla con logs, corregir en la rama del PR y
   propagar hacia arriba en la pila con merges (nunca rebase ni `--force`).
2. **Integración (solo el Orquestador, que no es el Ejecutor de esos WP).** Orden: #27 primero; luego `python tools/estado.py
   dividir`; reemplazar `TODAS_LAS_DEMAS` en WP-61 y WP-70 por listas, y marcar como `archivo` WP-33→14, WP-43→33 y
   WP-44→43; `agregar` y `validar`; después #12–#25, #26, #28–#32. Regenerar `linea_base.json` tras cada integración.
   Revisor y auditor deben ser de otra raíz de agente (E3): nadie ha revisado los PR de esta sesión.
3. **Conflictos de texto previstos:** `work.py` entre WP-32 (#18) y WP-51 (#32); `ESTADO.json` entre las ramas que lo
   editan y #27.
4. **WP-02** necesita D2. **WP-50** depende de WP-02 y exige el acta de 10 ingresos contrastados por el usuario.
   **WP-17** necesita D1. **WP-61** (formato) va al final y excluye `src/nurus/personal/motor/`. **WP-70** es el cierre.
5. **Fuera de ficha, a decidir por el Orquestador:** un botón o casilla en `descargador/descargador.py` y
   `ventana_resultados.py` para pasar `consolidated=True` (WP-51 no tiene entrada en la GUI); el manual 2.7.4 aún menciona
   el botón retirado en WP-45.
6. **Sin probar:** la salida nativa con Excel de WP-51 (solo Windows, PC autorizado). La ruta portable sí tiene pruebas.

### Integración de prueba (11 de octubre de 2026)

CI: tras la caída se relanzó cada PR una vez. #12–#30 quedaron en verde; #29 tenía un error real (WP-62 importaba `nurus`
antes de `parser()`), corregido en `c65f9fb`.

Rama `integracion/prueba-20261011` (no es `feat`; nadie la integra): `feat` + #27 + Codex #4, #8 y #11 + cima de la pila
(`wp/WP-45`, que trae #12–#25, #30 y #31) + `wp/WP-62` (#26 y #29) + `wp/WP-51` (#28 y #32). Resultado:

- Conflictos textuales solo en `descargador/descargador.py` (ritmo de WP-33 y comando `package` de WP-43: se conservan
  ambos) y `src/nurus/personal/work.py` (base consolidada de WP-51 dentro del `try` de WP-32, que borra la copia si falla).
- Un choque semántico: WP-34 quitó `from pathlib import Path` de `descargador/export_csmp_capture.py` (F401) y WP-51 lo usa.
  Se restituye al integrar.
- `verificar_wp.py --pruebas-eliminadas-asistente 148`: PASA (Asistente 569/11 y 6 xfail de WP-60; Descargador 227/62;
  identidades 0; módulos no alcanzables 0; enlaces rotos 0). Los 4 F401 del motor quedan excluidos del lint, como en la CI.
- El Orquestador puede usar esa rama como guía de resolución.

### Revisión con subagentes (11 de octubre de 2026; por E3 es apoyo, no aprobación)

Cinco revisores aislados (anonimizador, retiros, interfaz, Descargador y contrato, CI y herramientas). Corregido, cada
arreglo con prueba que falla sin él, y propagado hacia arriba en la pila con merges:

- **WP-16:** nueve filtraciones del anonimizador (RUT con guion largo o espacios o sin dígito, teléfonos 3-3-3, enteros
  de 7 a 9 dígitos, encabezados `CUR. AD LITEM`/`R.U.N.`/`TEL.`/`N° RIT`/`RUN NNA`, teléfonos en columnas del motor,
  curador institucional con abogado, RIT con barra). `_scan` revisa además números, RIT y teléfonos.
- **WP-40:** Ctrl+K borraba texto del campo antes de abrir la paleta; los atajos actuaban con el diálogo de fecha abierto.
- **WP-42:** la paleta mostraba «Revisar y guardar borradores» disponible con el botón deshabilitado.
- **WP-33:** la pausa no separaba una consulta de la siguiente. **WP-62:** al retomar, el manifiesto perdía lo ya
  leído hasta el cierre.
- **WP-51:** el consolidado admitía vínculos externos, conexiones y macros declaradas en `[Content_Types].xml`; el mapeo
  podía fallar en vez de caer a portable.
- **WP-31:** `--sincronizar-extension` ignoraba `--descargador`. **WP-35:** sin `requirements/` se borraba el `.venv` en
  cada arranque.
- **#27:** `verificar_wp.py` daba PASA cuando `check_real_ids`, ruff o pytest no pudieron medir; `--lista-eliminadas`
  aceptaba nodos inventados; `estado.py` sin fuentes daba traceback y trataba como error un conflicto abierto.

**No corregido, decide el usuario o el Orquestador:**

- `exports._annotation_values` deja un apóstrofo visible en `NURUS_PROPUESTA`/`NURUS_OBSERVACION_FINAL` de la copia
  portable; la ficha de WP-32 dice «sin apóstrofo», pero una prueba de la línea base
  (`test_preserved_export_uses_archived_bytes_and_marks_excluded`) exige `'=texto de prueba`. Es una decisión de diseño
  (defensa contra fórmulas).
- `tools/flow_baseline_matrix.py` y `flow_plan_progress.py` reescriben `docs/` aun con `--help` y devuelven enlaces a
  rutas retiradas: no ejecutarlos hasta corregirlos.
- Restos sin uso tras WP-21 (`operational_templates.json`, `resolution_templates.json`, clases `Issue`/`Evaluation` en
  `rus/models.py`), manual 2.7.4 con el botón de WP-45, `os.startfile` no vigilado por `verificar_wp.py`.
- Sin casilla en la GUI para `consolidated=True` (WP-51); búsqueda del último paquete en el hilo de la interfaz (WP-44).

### Conflictos abiertos que decide el usuario (detalle en el reporte, §4)

`cc_obligatorio` contra la prueba golden · 4 avisos F401 en el motor · la ficha de WP-23 contra el retiro de
`download_link.start` · fórmulas de OBSERVACIÓN copiadas como texto · 6 casos R-T de WP-60 · lock de entorno para la
instalación sin red.

### Cómo verificar y trabajar

```sh
python -m venv .venv && . .venv/bin/activate     # o usar el existente
python -m pip install --require-hashes -r requirements/linux-py312.lock
python -m pip install --no-deps --no-build-isolation -e .
xvfb-run -a python tools/verificar_wp.py --salida docs/implementacion/verificacion/WP-XX.json
```

- Desde WP-23 hay que agregar `--pruebas-eliminadas-asistente 148` (pruebas retiradas acumuladas) en las ramas de la pila.
  En #27 la verificación sin holgura usa `--lista-eliminadas` con nodeids.
- Las ramas de la pila se verifican con la cabeza de la pila (`wp/WP-42-…`): 512/11 Asistente y 198 Descargador. #26: 625/11
  y 203/59. #28: 625/11 y 200/59. #29: 625/11 y 203/59. #30: 514/11. #31: 515/11 y 198/59. #32: 637/11 y 204/59. #27: 630/11.
- Pruebas de interfaz en Linux: `xvfb-run`, con stubs de `pywinauto`, `win32gui` y `window_capture` para
  `tools/smoke_personal_gui.py`; la prueba de layout corre `_layout_probe` a 1024×650 y 1180×820 (el editor debe medir ≥ 120 px).
- Un worktree por WP evita pisar ramas: `git worktree add -b wp/<ID>-<slug> <ruta> origin/<base>`.
- El proxy de esta sesión no puede subir etiquetas (`pre-<ID>` las crea el Orquestador).

### Reglas que no se negocian (resumen; el texto vigente está en `AGENTS.md`)

Sin escritura automática en RUS ni SATURNO · Outlook solo borradores, CC fijo `ucc_concepcion@pjud.cl` · sin credenciales ni
datos reales (solo estructura anónima) · reglas, umbrales y textos tradicionales intactos · el Asistente no navega SITFA ni
inicia el Descargador · nada «aceptado» o «institucional» salvo acta del usuario · sin `--force`, sin fusionar a `main` ni
releases sin autorización escrita · cada WP toca solo los archivos de su ficha.

## Traspaso de la sesión claude-sesion-012yjVgL (10 de octubre de 2026)

**Qué se hizo:**
- Ejecuté como Ejecutor una pila de PR en borrador. Cada PR apunta a la rama del anterior porque todos editan
  `ESTADO.json`.
- Orden de integración (lo hace solo el Orquestador, con squash):

  | PR | WP | Rama o tema |
  | --- | --- | --- |
  | #12 | WP-20 | `ccr-4116db41-nm8g0r` |
  | #13 | WP-21 | — |
  | #14 | WP-22 | — |
  | #15 | WP-23 | — |
  | #16 | WP-30 | — |
  | #17 | WP-31 | — |
  | #18 | WP-32 | — |
  | #19 | WP-35 | — |
  | #20 | WP-24 | — |
  | #21 | WP-34 | CI |
  | #22 | WP-16 | anonimizador |
  | #23 | WP-40 | — |
  | #24 | WP-41 | — |
  | #25 | WP-42 | — |

**Cómo:**
- Una rama `wp/<ID>-<slug>` por WP.
- `python tools/verificar_wp.py --pruebas-eliminadas-asistente 143`: PASA en cada WP. Las 143 pruebas retiradas
  corresponden a WP-21, WP-22 y WP-23.
- Ficha en `docs/implementacion/wp/WP-XX.md` y verificación en `docs/implementacion/verificacion/WP-XX.json`.
- Todos los WP quedan en `EN_VERIFICACION`: no revisé ni audité mis propios WP.

**Para qué:** ejecutar las olas 2 a 4 del plan sin tocar el motor ni los textos tradicionales.

**Revisión del 10 de octubre (misma sesión).**
- Cinco revisores independientes encontraron errores; todos quedaron corregidos en sus ramas.
- Nuevos PR independientes o declarados:

  | PR | Contenido | Base |
  | --- | --- | --- |
  | #26 | WP-33 | `feat` |
  | #27 | plan, reporte y herramientas | `feat` |
  | #28 | WP-43 | `feat` |
  | #29 | WP-62 | sobre #26 |
  | #30 | WP-44 | sobre #25 |

- Hallazgos, pasos y enmiendas E1–E6:
  [REPORTE_REVISION_20261010.md](docs/implementacion/REPORTE_REVISION_20261010.md) (rama
  `plan/revision-20261010`).
- Desde las 03:24 UTC, Actions no asigna runners: revisar *Settings › Billing › Actions*.

**Falta:**
- Revisión, auditoría e integración de #12 a #25, en ese orden.
- WP-33, WP-43, WP-44 y WP-62: esperan WP-14 (Codex, PR #4).
- Decisiones del usuario: WP-45 (D9), WP-50 (D5), WP-51 (D6), WP-02 (D2), WP-17 (D1) y WP-70 (D7).
- WP-60 y WP-15: Codex.
- WP-61: al final.
- `conflictos_abiertos` de ESTADO: `cc_obligatorio` frente a golden y 4 F401 en el motor.
- Probar el anonimizador con un `.xls` real en el PC autorizado.

## Enlaces

- [Plan vigente](PLAN_VIGENTE.md) · [Instrucciones](AGENTS.md)
- [Plan de implementación](docs/implementacion/PLAN_IMPLEMENTACION.md)
- [Recuperación](docs/implementacion/wp/WP-01.md)
- [Historial anterior](docs/HISTORIAL.md)

## Comandos

```sh
python -m pip install --require-hashes -r requirements/linux-py312.lock
python -m pip install --no-deps --no-build-isolation -e .
python tools/verificar_wp.py
```

## Migración de contactos antes de actualizar

Copia tu `contactos_base.json` de la instalación anterior a
`%LOCALAPPDATA%\CSMP_Personal\contactos.json` antes de reemplazarla.
Si no existe esa copia, el primer inicio abre «Importar contactos» para elegir
el JSON anterior o el catastro Excel; mientras tanto se usa el ejemplo ficticio.

## Control de identidades

Activa el control antes de cada commit con:

```sh
git config core.hooksPath tools/hooks
```

El hook revisa el contenido del índice, incluidos los XML de Excel y Word;
no lee las modificaciones que aún no agregaste. Ambos workflows ejecutan el
control antes de instalar dependencias o producir artefactos, sobre todo el
árbol: WP-20 retiró `variantes/` y las rutas personales (recuperables desde `pre-WP-20`).
