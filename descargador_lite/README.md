# SITFA Lite · Descargas RUS

**1.1.0 (rama Rediseno):** consultas en la lista lateral, filtros como fichas, opciones que se cargan solas al
conectar, período «Sin filtro / Por fecha» con calendario y avance por consulta. Manuales ilustrados y videos narrados
actualizados a 1.1.

SITFA Lite descarga desde tu Chrome los XLS que produce RUS y guarda un pantallazo PDF cuando una consulta no tiene registros.

- Seguimiento · Informe seguimiento: tribunales, pestañas (Espera, Cumplimiento, Informes, Egresados), modalidades, tipo de informe, filtro de medidas, fechas.
- Seguimiento · Búsqueda litigantes: tribunales, estado, fechas.
- Informes · Informes por vencer / Medidas por vencer: tribunales, mes(es), año.

No escribe en RUS.

## Documentos

- Manuales ilustrados en PDF: [instalación](docs/SITFA_Lite_Manual_Instalacion.pdf) · [uso](docs/SITFA_Lite_Manual_Uso.pdf)
- [Video: instalación y uso completo](docs/SITFA_Lite_Video_Instalacion_y_Uso.mp4) (3 min 39 s, narrado, con subtítulos)
- [Video: instalar la extensión](docs/SITFA_Lite_Video_Instalar_Extension.mp4) (87 s, narrado)
- [Manual de instalación](docs/MANUAL_INSTALACION.md)
- [Manual de uso](docs/MANUAL_USO.md)

## Estructura

```
app/          programa (Tkinter) + motor de validación + puente local
extension/    extensión Chrome MV3 (cargar sin empaquetar)
tests/        pruebas con SITFA ficticio (sin red ni datos reales)
build/        PyInstaller (.spec), Inno Setup (.iss), build.ps1, manuales.py
docs/         manuales, videos y evaluación de diseño
```

## Desarrollo

```bash
python -m pip install -r requirements.txt pytest
```

```bash
python -m pytest -q tests
```

```bash
python app/sitfa_lite.py
```

## Construir el instalador `.exe`

Automático: el workflow `.github/workflows/sitfa-lite.yml` compila en `windows-latest` y publica el artefacto `SITFA_Lite_instalador` (instalador + SHA-256).

Manual en Windows (Python 3.12 x64 + Inno Setup 6):

```powershell
powershell -ExecutionPolicy Bypass -File build\build.ps1
```

Resultado: `dist\Instalar_SITFA_Lite_1.1.0.exe` y `dist\Instalar_SITFA_Lite_1.1.0.sha256`.

La versión vive en `app/version.py` y debe coincidir con `extension/manifest.json` (el build lo comprueba).

## Regenerar manuales ilustrados y video

Todo con datos ficticios. Capturas del programa: en Windows, con el workflow «Capturas Windows para manuales y videos» (`tools/manuales/capturas.py lite`, quedan en `docs/manuales/capturas_windows/lite/`). Capturas de Chrome (Linux + Xvfb + Chromium de Playwright): `build/manual_visual/capturas_chrome.js` (ventana completa) y `capturas_chrome_contenido.js` (página, en español). Luego `anotar.py`, `manuales_pdf.py` + `pdf.js` para los PDF, y `narrado.py` para los videos (voz local Kokoro, `es-419`; modelo en github.com/thewh1teagle/kokoro-onnx/releases).
