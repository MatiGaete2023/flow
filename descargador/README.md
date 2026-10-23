# Descargador SITFA · prototipo integral

Lee SITFA desde tu Chrome con tu propia sesión (sin leer cookies ni contraseñas) y entrega los Excel originales,
el consolidado, las bitácoras (`Bitacoras_RUS.xlsx`) y un paquete ZIP para el Asistente CSMP. No escribe en RUS.
La versión vigente está en `contrato_integral.py`; la extensión de `extension/` debe ser de la misma versión.

## Instalar y abrir

1. Ejecuta `Iniciar.cmd` (usa el EXE de `dist/` si existe; si no, crea `.venv` con Python 3.12 y el lock con hashes).
2. En Chrome: `chrome://extensions` → modo desarrollador → «Cargar descomprimida» → carpeta `extension/`.
3. Entra a SITFA → Seguimiento y conecta la extensión con el código que muestra el Descargador.

## Uso

1. Elige tribunales, modalidades, pestañas y fechas; descarga el lote.
2. **Leer bitácoras del lote** (y **Reintentar lecturas fallidas** si hace falta).
3. **Resultados e historial:** revisa lotes anteriores sin Chrome y pulsa **Crear paquete para Asistente (ZIP)**.
4. En el Asistente: Trabajo → **Importar paquete**.

## Diagnóstico sin datos reales

Nunca compartas HAR, HTML, PDF ni capturas reales. Comparte solo `diagnostico_bitacoras.json` o la estructura
anónima de un HAR: `python extraer_estructura_har.py captura.har estructura.json`.

## Documentos

- [Manual ilustrado](../docs/manual_descargador_2_7_4/README.md) y [corrección del ZIP 2.7.5](../docs/CORRECCION_REFERENCIAS_ZIP_20261009.md).
- [Historial de cambios](CHANGELOG.md) · [Plan vigente](../PLAN_VIGENTE.md).
- Pruebas: `python -m pytest descargador -q` y `node descargador/test_bitacoras_extension.cjs`.
