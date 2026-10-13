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
| 53.7 retención + spec | PENDIENTE | | |
| Cierre WP-53 (ci_local, verificar_wp, E3) | PENDIENTE | | E3 la hace otra sesión |
| WP-54 UI/flujo/docs | PENDIENTE | | |

## Notas de desvío respecto de la guía
(ninguna aún)
