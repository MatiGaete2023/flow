# Manuales de las herramientas Flow

PDF ilustrados y videos con capturas de las ventanas reales tomadas en Windows, también las de Chrome (workflow
`manuales-windows.yml`, carpeta `capturas_windows/`), y datos ficticios (rama Rediseno, 10 de noviembre de 2026: diseño
con las 32 mejoras de `docs/diseno/MEJORAS_UX_20261109.md`). Cada instalador lleva sus manuales y videos en `manuales/`;
en los programas se abren desde «Más» o con F1.

| Herramienta | Versión | Manual | Videos narrados |
|---|---|---|---|
| Asistente CSMP | 0.5.0.dev11 | [Manual_Asistente_CSMP.pdf](Manual_Asistente_CSMP.pdf) | [1 min 01 s](Flow_Video_Asistente_CSMP.mp4) |
| Descargador SITFA | 2.9.0 | [Manual_Descargador_SITFA.pdf](Manual_Descargador_SITFA.pdf) | [1 min 00 s](Flow_Video_Descargador_SITFA.mp4) · [retomar un paquete, 22 s](Flow_Video_Retomar_Paquete.mp4) |
| SITFA Lite | 1.1.0 | [instalación](SITFA_Lite_Manual_Instalacion.pdf) · [uso](SITFA_Lite_Manual_Uso.pdf) | [instalación y uso, 3 min 54 s](SITFA_Lite_Video_Instalacion_y_Uso.mp4) · [extensión, 1 min 34 s](SITFA_Lite_Video_Instalar_Extension.mp4) · [descargar Seguimiento de un tribunal, 24 s](SITFA_Lite_Video_Descargar_Seguimiento.mp4) |
| Meta 4 | 2.6.0 | [Manual_Meta4.pdf](Manual_Meta4.pdf) | [33 s](Flow_Video_Meta4.mp4) |
| Anonimizador CSMP | 1.1.0 | [Manual_Anonimizador_CSMP.pdf](Manual_Anonimizador_CSMP.pdf) | [29 s](Flow_Video_Anonimizador_CSMP.mp4) |

Las imágenes anotadas están en `img/<herramienta>/`.

## Regenerar

```bash
# 1. Capturas en Windows: workflow «Capturas Windows para manuales y videos» (las guarda en capturas_windows/)
python tools/manuales/generar.py docs/manuales/capturas_windows docs/manuales
python tools/manuales/videos.py docs/manuales/capturas_windows docs/manuales <carpeta_modelo_kokoro> docs/manuales
```

Los videos usan la voz local Kokoro (`kokoro-onnx`, `soundfile`, `ffmpeg`); el modelo se descarga de
github.com/thewh1teagle/kokoro-onnx/releases.

SITFA Lite tiene su propio flujo en `descargador_lite/build/manual_visual/` (capturas de Chrome:
`tools/manuales/capturas_chrome_win.py`, en el mismo workflow).
