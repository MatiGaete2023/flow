# AVANCE — traspaso entre IAs (WP-53/WP-54)

Fuente de verdad del procedimiento: [GUIA_IMPLEMENTACION.md](GUIA_IMPLEMENTACION.md). Este archivo solo dice **dónde quedó el trabajo**.
Retomar: `git log --oneline -8` + este archivo; siguiente paso = primera fila no `HECHO`.
Entorno: `.venv` (ver guía paso 0); `tools/ci_local.sh <pruebas>` antes de cada push. Commits locales por paso; push cada 2–3 pasos.

| Paso | Estado | Commit | Notas |
|---|---|---|---|
| 53.1 lector | HECHO | previo | — |
| 53.2 vínculo | HECHO | previo | — |
| 53.3 extensión `history_open` | HECHO | ver git log | prueba `test_historia_extension.cjs` pasa; descargador 293 OK |
| 53.4 `LectorHistoria` + gancho | HECHO | ver git log | `cerrar(lote_estado)` recibe estado del lote (desvío menor) |
| 53.5 Excel `historias.py` | HECHO | ver git log | `cargar()` reutilizable por 53.6; `literal` local (no importa de descargador); textos >30000 se recortan con aviso |
| 53.6 contrato + paquete | HECHO | ver git log | `historia=` en export_capture y package_current_lot; EXTENSIONS en contract.py; DICCIONARIO actualizado |
| 53.7 retención + spec | HECHO | ver git log | CONTROLES en preferencias.py incluye historia/historias.json |
| Cierre WP-53 (ci_local, verificar_wp, E3) | PASA; E3 PENDIENTE | ver git log | verificar_wp PASA; E3 la hace otra sesión (guía §12) |
| WP-54 UI/flujo/docs | HECHO (falta 54.7 y acta) | ver git log | casillas, flujo, manual, CHANGELOG, guía de prueba. Desvío: sin deshabilitar casillas por capacidades (la UI no recibe el handshake; `verify_extension` falla con aviso claro) |

## Notas de desvío respecto de la guía
- 53.4: `cerrar(lote_estado)` recibe el estado del lote (si no es COMPLETA, la historia queda INCOMPLETA).
- 53.5: Excel sin `excel_presentacion` (src no puede importar descargador): freeze_panes + autofiltro como `export_diaries`; `literal` vive en `nurus/capture/historias.py` y `export_csmp_capture` lo importa; textos >30000 se recortan con aviso y se cuentan en Control.
- 53.7: `tools/write_build_meta.py` VERSION_NAME cambió a «bitácoras e historia» (lo exige tests/test_versiones_coherentes).
- 53.6: `original_file` de las filas es `respaldo/<lote>/historia/<archivo>` (ruta real; la guía decía `respaldo/historia/`).
- WP-54: sin deshabilitado dinámico de casillas por versión de extensión.

## Pendiente (en orden)
0. Autorrevisión adversarial hecha (13-oct): sin `.Send(`/cookies/almacenes en el diff; corregida la ruta de procedencia. No sustituye la E3.
1. **E3** de WP-53 por otra sesión/persona (guía §12; foco del diff `git diff feat/ejecucion-plan-20261008...HEAD`). Si APROBADA: poner `revision: APROBADO` en `estado/WP-53.json`, estado `INTEGRADO` solo cuando el usuario integre.
2. **Prueba real y acta** del usuario (`docs/aceptacion/GUIA_PRUEBAS_HISTORIA_20261013.md`); solo el usuario firma.
3. **WP-54.7** solo si el usuario lo pide: PR `feat/historia-causa-sitfa → feat/ejecucion-plan-20261008` + `workflow_dispatch` «CSMP Assistant Windows». No fusionar sin autorización escrita.
4. Opcional: deshabilitar casillas si la extensión no declara `historia_lectura` (requiere exponer capabilities a la UI).
