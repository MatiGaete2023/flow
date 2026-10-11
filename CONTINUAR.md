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
control antes de instalar dependencias o producir artefactos. Hasta WP-20,
se excluyen `variantes/` y el documento antiguo del Descargador con rutas
personales también del checkout y del ZIP de fuentes.
