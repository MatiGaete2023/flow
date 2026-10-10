# P0.04 / P0.06 / P0.07 — Módulos y migración

El [inventario de 121 módulos](evidencia_p0/inventario/modulos.md) y el
[grafo de imports](evidencia_p0/inventario/grafo_imports.json) se generan con
`tools/flow_baseline_inventory.py`, sin importar la app ni abrir sesiones.
Incluyen imports dentro de funciones; imports calculados y subprocess requieren
lectura manual. No se eliminaron módulos.

| Área | Estado del origen | Destino en el plan | Prueba/condición |
|---|---|---|---|
| `personal.work`, `personal.motor`, textos/configuración | Análisis vigente, propuestas y ediciones | Conservar literalmente; importar paquete mediante adaptador local | Golden fijo `2026-10-08`; pruebas de personal flow y redacción |
| `personal.app_base`, `results_view` | La GUI aún ofrece iniciar descargas/bitácoras RUS | Adaptar P5/P6 únicamente tras G3/G5 | UI con Xvfb pasó; no acredita desacoplamiento final |
| `download_link`, `download_transfer`, `bitacora_link` | Lanzamiento externo y recuperación de solicitudes | Retirar del flujo operativo conservando recuperación legado | `test_download_transfer.py`, `test_bitacora_lote.py` |
| `bitacora_lote`, `bitacora_har`, `bitacoras` | Datos, exportación y análisis mezclados en paquete personal | Extraer exportación común sin mover reglas | Golden Excel bitácoras, parser y tests identidad |
| `nurus.bitacora_html`, `descargador.csmp_shared` | Parser compartido y búsqueda de paquete | Conservar y verificar instalación autónoma | CI origen y pruebas fixtures; P3 pendiente |
| `personal.outputs`, `resolutions`, plantillas | Nóminas, Word y preparación de borradores | Conservar; no enviar ni escribir RUS | Golden y suites Outlook/Word |
| `personal.registration`, `registration_excel` | API genérica y comprobantes de registro RUS | **Retirados en WP-22 (D3)**; sin vía de escritura RUS en `src` (control negativo de `verificar_wp.py`) | Historial Git |
| `personal.review_store`, `rus_activity` citados por 02 | Ausentes en el árbol activo del SHA fijado | No intentar retirarlos otra vez ni copiar versiones antiguas | `test_current_release_contract.py` verifica ausencia de `review_store`; hechos de firmas en `sitfa.py`/`work_tools.py` |
| `manual_products`, `manual_actions` citados por 02 | Ausentes en el árbol activo | Conservar funciones actuales en `manual.py`/`current_tools.py`, no recrear una UI duplicada | `test_current_work_features.py`, `test_personal_cleanup_20260921.py` |
| `nurus.rus.rules`, `services`, `storage` | Motor y flujo alternativos/compatibilidad | **Retirados en WP-21** (se conservan `domain/models.py`, `rus/util.py`, `services/policy.py` y `exports.py`) | Historial Git |
| `tools/rus_inspector_entry.py`, `rus_capture_extension` | Inspector RUS independiente | **Retirado en WP-22 por decisión D3** | Historial Git |
| `variantes/ux_rules_20260922` | Copia histórica versionada (retirada en WP-20; recuperable desde la etiqueta `pre-WP-20` o los commits `39b6c06`/`3708930`) | No importar al camino nuevo | No fusionar automáticamente |

## Configuración, sesiones y rollback

1. `Configuration` conserva textos personalizables y genera respaldo `.bak`.
   Las pruebas `test_config_migration_regressions_20260914.py` verifican reparación
   e idempotencia sin sobrescribir cuerpos personalizados.
2. `Work` conserva sesión v2, ediciones, decisiones, overrides y comprobantes.
   `save/load` y `download_transfer` siguen intactos. No hay migración nueva
   en esta entrega ni acceso a la configuración personal del usuario.
3. `signed_activity` y `activity_reviewed` existen en el modelo actual. La búsqueda
   en `src`/`tests` no encontró productores/consumidores aparte de su inicialización
   en `Work`; `save/load` conservan estos campos con el resto de la sesión. La
   desaparición de `review_store` no prueba que una sesión heredada no contenga
   marcas. P6 debe aislar `activity_reviewed` de decisiones de una nueva captura,
   conservando originales/backup y datos factuales de firmas por separado. No
   borrar ediciones, preferencias, borradores ni comprobantes de productos.
4. Los estados locales de revisión/`rus_recorded` no son evidencia autónoma
   de un registro institucional real. Se conserva baseline; aclaración de
   semántica y redacción queda para aprobación funcional, sin cambio unilateral.
5. Antes de futura migración, copiar configuración/sesión de prueba a carpeta
   nueva, probar ida/vuelta y conservar entradas originales. Nunca convertir
   irreversiblemente trabajo del usuario como parte del arranque.

La recepción del documento 02 cierra su ausencia documental. La correspondencia
con sus rutas antiguas y todos sus requisitos se detalla en
[ANALISIS_ESPECIFICACION_02_20261008.md](ANALISIS_ESPECIFICACION_02_20261008.md).

No hay paridad entre motores certificada por cambiar nombres. La paridad
registrada aquí corresponde al motor personal vigente frente a sí mismo,
con fecha de corte fija y nuevos casos sintéticos repetibles.
