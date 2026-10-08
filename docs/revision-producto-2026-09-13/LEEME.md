# Anexos de auditoría CSMP — 2026-09-13

El informe contiene el dictamen, 16 hallazgos, matrices funcional/de reglas, conflictos, plan y referencias. El manifiesto identifica fuentes y resultados de las suites. El JSON de evidencia contiene las comprobaciones adicionales V03–V08.

Para reproducir las comprobaciones, usar una copia de Asistente v9.1.0 extraída del ZIP identificado, una copia de NuRus del commit indicado en el manifiesto y el Creador adjunto. En un entorno aislado instalar las dependencias declaradas por esos proyectos, `pytest` y `time-machine`; se necesita Tkinter para importar el Creador, pero no una pantalla ni Outlook.

```bash
python verificar_hallazgos.py --asistente /ruta/Asistente --nurus /ruta/NuRus --creador "/ruta/Creador de Correos.py" --salida evidencia_reproducida.json
```

La verificación usa datos sintéticos y congela la fecha en 2026-07-15. No abre Excel, no crea borradores, no envía mensajes y no modifica las fuentes. Los resultados fallarán deliberadamente si cambió el comportamiento auditado: no constituyen una suite de aceptación de la futura versión corregida.

Las suites propias se ejecutan desde la raíz de cada proyecto mediante `python -m pytest -q`; NuRus requiere su directorio `src` en PYTHONPATH o instalación previa en el entorno aislado. Los tiempos del manifiesto pertenecen a esta ejecución y no son una medida del ciclo mensual de trabajo.
