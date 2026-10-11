# Flow — CSMP Assistant y Descargador SITFA

Herramientas de uso personal para el seguimiento de medidas (CSMP): el **Descargador** lee SITFA con la sesión
del usuario y entrega Excel, bitácoras y un ZIP; el **Asistente** (Windows) lee ese Excel o ZIP, aplica las reglas
tradicionales de Laja, Mulchén y Tomé y prepara Excel, nóminas, Word y borradores de Outlook.
No envía correos, no escribe en RUS/SATURNO y no navega SITFA.

## Instalar

- **Asistente:** `Instalar_CSMP.bat` (Python 3.12 primero; dependencias fijadas con hash) y luego `Abrir_CSMP.bat`,
  o el instalador `Instalar_CSMP_<versión>_Windows.exe` de la CI.
- **Descargador:** `descargador/Iniciar.cmd`, o el EXE `SITFA_Descargador` de la CI; cargar su extensión de la
  misma versión en `chrome://extensions`. Ver [descargador/README.md](descargador/README.md).
- **Desarrollo (Linux):** `pip install --require-hashes -r requirements/linux-py312.lock` y
  `pip install --no-deps --no-build-isolation -e .`; verificación: `python tools/verificar_wp.py`.

## Flujo

1. **Descargador:** consulta, lee bitácoras y crea el paquete ZIP para el Asistente.
2. **Trabajo:** importar el ZIP o el Excel, revisar observaciones (Ctrl+O elige un Excel; Ctrl+K busca acciones).
3. **Correos:** preparar borradores con CC fijo `ucc_concepcion@pjud.cl`; también **Correo libre**.
4. **Resoluciones:** preparar, revisar y generar proyectos Word; también **Word manual**.
5. **Resultados:** productos, incidencias y lo que falta para terminarlos.

Todo se guarda localmente en Windows.

## Documentos

- [PLAN_VIGENTE.md](PLAN_VIGENTE.md): esencia, prohibiciones, prioridades y decisiones del usuario.
- [CONTINUAR.md](CONTINUAR.md) y [estado de la implementación](docs/implementacion/ESTADO.json).
- [Índice de documentación vigente](docs/INDICE_DOCUMENTACION.md) · [historial](docs/HISTORIAL.md).
- [Manual ilustrado del Descargador](docs/manual_descargador_2_7_4/README.md).
