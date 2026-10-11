# Continuar — 11 de octubre de 2026

**Repositorio:** MatiGaete2023/flow. **Base:** `feat/ejecucion-plan-20261008`, `2cf8ca8`.
Leer `AGENTS.md`, `PLAN_VIGENTE.md` y `docs/implementacion/ESTADO.json`.

El historial fue purgado: volver a clonar; nunca subir ramas desde copias anteriores.
#4, #8, #11 y #12–#32 ya están integrados en `feat`, junto con WP-61.
CI Windows y compilación aprobadas en `14f679d`; Actions vuelve a funcionar.

## Revisión actual

WP-63 corrige la versión integrada: [PR #34](https://github.com/MatiGaete2023/flow/pull/34),
rama `wp/auditoria-refactor-20261011`.
Reporte: [REPORTE_REVISION_20261011.md](docs/implementacion/REPORTE_REVISION_20261011.md).
PASA en `d25fe2f`: Linux 780/11/6 xfail, Descargador 235/62, golden17; Windows 790/1/6 y CI verde.
Los commits posteriores solo registran documentación/evidencia; consultar controles de PR #34 por SHA.
La ficha registra evidencia y siguiente acción; no equivale a aceptación de uso real.
El estado se edita en `docs/implementacion/estado/` y se regenera con `tools/estado.py agregar`.

- **Integrado en `feat`** (excepción a E3 autorizada por el usuario): #27, Codex #4, #8 y #11, #12–#32, más WP-61
  (formato). `ESTADO.json` es una vista generada desde `docs/implementacion/estado/` (`tools/estado.py`, PASA). La
  línea base se regeneró: asistente 592/11/6 xfail, descargador 231/62, identidades 0, enlaces rotos 0.
- **Decisiones:** D1 (purga hecha), D3–D9 respondidas, D2 «Ninguna todavía».
- **Pendientes del usuario:**
  1. Borrar las etiquetas `pre-WP-12` y `pre-implementacion-20261009` (conservan el historial viejo; `check_real_ids
     --historia` encuentra en ellas 12 RUT con dígito verificador válido).
  2. Pedir a GitHub la eliminación de datos sensibles en las referencias de PR.
  3. ~~Borrar los artefactos de Actions previos~~: hecho (607 borrados por la API, 11 de octubre de 2026).
  4. Billing: GitHub Pro y presupuesto de Actions de 5 USD (11 de octubre de 2026); CI verde en `14f679d`.
- **Lo que falta del plan:**
  - WP-02 (D2: casos probados en uso real).
  - WP-50: implementado en `wp/50-cronologia-solo-mostrar` (PR #33 borrador, CI verde, `verificar_wp` PASA). Se integra
    tras el acta de 10 ingresos del usuario y WP-02.
  - WP-70: cierre. Paquete compilado en CI (`14f679d`, run 38136720831); esperar el acta final y, con D7, sacar el PR #1
    de borrador y fusionarlo a `main`, sin release.
- **Auditorías integradas (11 de octubre de 2026, tarde)**, por decisión escrita del usuario y con revisión E3 de otra raíz:
  - **WP-63** (PR #34): integrado. Línea base regenerada en `65fc543`.
  - **WP-80** (PR #35): integrado con squash en `5ac7385`.
    - E3: revisión APROBADO; alineación CONFORME. Informes en la rama `e3/wp-80-revision` (`docs/implementacion/auditoria/`).
    - Sin etiqueta `pre-WP-80`, por decisión del usuario: el rollback es `git revert 5ac7385`.
  - Línea base actual: asistente 796, descargador 240.
  - Dependencias de WP-70: funcionales sobre WP-02, WP-17, WP-50 y WP-63 (confirmadas por el usuario).
  - Pendientes bajos: G-3/G-4/G-5 (GUI), O1 de WP-63 y `_recalculate` duplicado frente a `analyze` (WP propio con
    paridad golden). Detalle en `docs/implementacion/estado/WP-80.json`.
  - Pendientes que requieren decisión del usuario:
    - código de firmas retirado en `carga.py`;
    - módulos `bitacoras`/`bitacora_har`/`bitacora_lote` y `excel_presentacion` duplicados;
    - control de controles restituidos en `pagina.js`, que requiere probar en SITFA real.
- **Conflictos respondidos** (11 de octubre de 2026): `cc_obligatorio` se conserva; motor/ queda intacto (4 F401
  excluidos en CI; los 6 R-T de WP-60 siguen xfail estricto hasta una decisión por regla).

## Pendientes, en orden

1. Revisar WP-63 y sus verificaciones antes de integrar; E3 exige revisión de otra raíz.
2. WP-17: el usuario borra las dos etiquetas antiguas y solicita a Soporte de GitHub
   limpiar referencias de PR. Los 607 artefactos anteriores ya fueron borrados.
3. WP-02/D2: resultados de pruebas en el PC autorizado, sin compartir datos reales.
   No hay casos aprobados todavía ni acta firmada.
4. WP-50: PR #33 preparado y CI verde; integrar tras WP-02 y acta de 10 ingresos.
   Después de WP-63 resolver el solapamiento en `app_base.py` antes de verificar.
5. Probar exportación nativa con Excel y borradores con Outlook clásico en Windows.
6. WP-70/D7: acta final, actualizar métricas y fusionar PR #1 a `main`, sin release.

Motor y textos tradicionales intactos; seis conflictos R-T siguen en xfail estricto.
Historial de traspasos: [docs/HISTORIAL.md](docs/HISTORIAL.md).
