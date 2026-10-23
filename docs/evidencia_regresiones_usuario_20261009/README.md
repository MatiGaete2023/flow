# Evidencia de las regresiones del usuario — 9 de octubre de 2026

Código funcional: `e3a63691ba75657cef82834406dadb849ba50ee6`.
Asistente 0.5.0.dev4 y Descargador/extensión 2.7.4. Los commits documentales
posteriores no cambian los binarios correspondientes a este SHA.

- `local.json`: entorno, SHA, alcance y resultados de suites completas.
  Suites ejecutadas en `d6ec1b4`; el SHA de entrega solo cambia dos aserciones
  para rutas Windows y lectura UTF-8. Nueve pruebas focalizadas pasan de nuevo
  (`windows_path_encoding_focused.txt`), con código funcional idéntico.
- `pytest_assistant.txt/xml`: 605 aprobadas, 11 omitidas Linux/COM.
- `pytest_downloader.txt/xml`: 192 aprobadas y 59 subpruebas aprobadas.
- `golden.txt` y `golden_semantic.json`: paridad de 17 productos tradicionales.
- `review_probes.txt`: regresiones H01–H08 preservadas.
- `extension_bitacoras.json`: Chromium con extensión instalada y GET ficticios;
  80 lecturas, cuatro modalidades, Espera/Cumplimiento/tres opciones de Informes,
  tipos 11/12/13 y UTF-8/Windows-1252.
- `extension_pdf.json`: captura Chrome nativa; vistas inicialmente vacías,
  opciones ausentes, contexto visible en píxeles y rechazo de contexto alterado.
- `ci_windows.json`: resultados, pasos y digests informados por GitHub.
- `ci_precedentes.json`: ejecuciones sustituidas y correcciones de sus pruebas.

Las fixtures de 800 bitácoras, paquetes separados e interfaz offline están en
`descargador/test_reutilizacion_local.py` y `descargador/test_experiencia.py`.
El caso de 25 correos con edición opcional de uno, conservación de los otros
24, repreparación y guardado individual/lote está en `tests/test_integral_ui.py`.
Las pruebas Office usan un adaptador ficticio: no constituyen aceptación real.

No se publican los HAR, JSON, HTML, capturas o identificadores reales recibidos.
Las cuentas y personas de los tests son ficticias. No se hicieron peticiones
reales a RUS, escrituras institucionales ni envíos de correo.

Comandos principales:

```sh
DISPLAY=:99 python -m pytest -q
DISPLAY=:99 python -m pytest descargador -q
DISPLAY=:99 python descargador/verificar_bitacoras_extension.py
DISPLAY=:99 python descargador/verificar_pdf_extension.py
node descargador/test_bitacoras_extension.cjs
python tools/flow_baseline_golden.py --output /ruta/nueva --expected docs/evidencia_p0/golden/semantic.json
python tools/flow_review_probes.py --verify
```

La compilación usa los locks de Windows por versión de Python y verifica los
EXE, instalador/reinstalación, eliminación de la carpeta instalada y conservación
de datos externos. Los digests de artefactos proceden de la API de GitHub; no se
presentan como hashes recalculados localmente. La aceptación en Windows 10,
Office/SITFA y modalidades reales continúa pendiente.
