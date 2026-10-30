# SITFA Lite · Descargas RUS

Versión liviana del descargador SITFA: solo descarga los XLS que produce RUS y guarda un pantallazo PDF cuando una consulta no tiene registros.

- Seguimiento · Informe seguimiento: tribunales, pestañas (Espera, Cumplimiento, Informes, Egresados), modalidades, tipo de informe, filtro de medidas, fechas.
- Seguimiento · Búsqueda litigantes: tribunales, estado, fechas.
- Informes · Informes por vencer / Medidas por vencer: tribunales, mes(es), año.

Sin paquetes consolidados, bitácoras, historia de la causa ni calendario de informes por ingreso. No escribe en RUS.

## Documentos

- [Manual de instalación](docs/MANUAL_INSTALACION.md)
- [Manual de uso](docs/MANUAL_USO.md)
- [Evaluación del integral y decisiones de diseño](docs/EVALUACION.md)

## Estructura

```
app/          programa (Tkinter) + motor de validación + puente local
extension/    extensión Chrome MV3 (cargar descomprimida)
tests/        pruebas con SITFA ficticio (sin red ni datos reales)
build/        PyInstaller (.spec), Inno Setup (.iss), build.ps1, manuales.py
docs/         manuales y evaluación
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

Resultado: `dist\Instalar_SITFA_Lite_1.0.0.exe` y `dist\Instalar_SITFA_Lite_1.0.0.sha256`.

La versión vive en `app/version.py` y debe coincidir con `extension/manifest.json` (el build lo comprueba).
