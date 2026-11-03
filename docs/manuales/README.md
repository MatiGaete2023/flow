# Manuales de las herramientas Flow

PDF ilustrados con capturas de las ventanas reales y datos ficticios (rama Rediseno, 3 de noviembre de 2026).

| Herramienta | Versión | Manual |
|---|---|---|
| Asistente CSMP | 0.5.0.dev11 | [Manual_Asistente_CSMP.pdf](Manual_Asistente_CSMP.pdf) |
| Descargador SITFA | 2.9.0 | [Manual_Descargador_SITFA.pdf](Manual_Descargador_SITFA.pdf) |
| SITFA Lite | 1.1.0 | [instalación](SITFA_Lite_Manual_Instalacion.pdf) · [uso](SITFA_Lite_Manual_Uso.pdf) |
| Meta 4 | 2.6.0 | [Manual_Meta4.pdf](Manual_Meta4.pdf) |
| Anonimizador CSMP | 1.1.0 | [Manual_Anonimizador_CSMP.pdf](Manual_Anonimizador_CSMP.pdf) |

Las imágenes anotadas están en `img/<herramienta>/`.

## Regenerar

```bash
for p in asistente descargador meta4 anonimizador; do xvfb-run -a -s "-screen 0 1400x1000x24" python tools/manuales/capturas.py $p capturas/$p; done
python tools/manuales/generar.py capturas docs/manuales
```

SITFA Lite tiene su propio flujo en `descargador_lite/build/manual_visual/`.
