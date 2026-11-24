# Inventario de herramientas: Flow Python frente a Flow Rust (23-nov-2026)

Pregunta: ¿Flow Rust tiene, o tuvo alguna vez, todas las herramientas de Flow Python?

Respuesta corta:
- **No.** Ninguna versión de Rust las cubrió todas.
- `app/`, la versión vigente, cubre bien el **Asistente** (Trabajo, Correos, Resoluciones, Resultados, Contactos, Ajustes) y el **Descargador de listados** (Lite e Integral, con Carga).
- No tiene las lecturas de contexto del Descargador, Meta 4 ni las exportaciones de lote.
- El workspace histórico `rust/` llegó más lejos en Descargador, Meta 4 y Anonimizador, pero nunca se integró en `app/` ni tuvo prueba del usuario.

Fuentes (N1):
- `app/src` en el commit de este informe;
- `rust/crates/*` y `rust/EJECUCION.md`;
- `docs/migracion_python_rust/MATRIZ.md` (20-nov) y la matriz del estudio anterior (`ed6a0e4`);
- los módulos Python citados.

Estados:
- **Sí**: existe y tiene contraste con Python;
- **Parcial**: existe con huecos nombrados;
- **No**: ausente.

## 1. Herramientas de Flow Python y su estado en Rust

| Herramienta Python | Qué hace | `app/` (vigente) | `rust/` (histórico) |
|---|---|---|---|
| Asistente CSMP: Trabajo (`src/nurus/personal/work.py`, motor) | Abre el Excel RUS, aplica reglas y edita observaciones | **Sí**. Motor con paridad golden: 2.585 + 174 casos y cronología 181. | Sí (`flow-motor`, `flow-trabajo`) |
| Importar paquete de contexto (`capture_input.py`) | Abre el ZIP CSMP del Descargador | **Sí**. Paquete C contra Python, con historia, bitácora, escritos, histórico y calendario. | Sí (`flow-trabajo::paquete`) |
| Revisión de bitácoras (`revision_bitacoras.py`) | Período, clasificación y Excel/JSON | **Sí**. V1/V2 contrastadas en la CI. | Sí |
| Correos y nómina (`outputs.py`, `roster.py`) | Borradores por programa con CC fijo | **Parcial**: EML y Outlook presentes; no se puede elegir la cuenta Outlook (N1: sin coincidencias de `SendUsingAccount`). | Sí (`flow-outlook`, COM) |
| Resoluciones Word (`resolutions.py`) | Proyectos PC_IE, PC_INFO y NOMENCL | **Sí** (matrices Laja/Mulchén). Falta el importador de paquetes de plantillas. | Sí |
| Formularios manuales (`manual.py`) | Correo o Word sin abrir un Excel | **No encontrado** (N1: `manual` solo aparece en `motor.rs`). | Por demostrar |
| Resultados y estadísticas (`biblioteca.py`, `statistics.py`, `review_metrics.py`) | Historial, métricas y entrega | **Parcial**: la vista Resultados existe; sin estadísticas ni métricas (N1: `estadist` sin coincidencias). | Parcial |
| Ajustes, contactos y plantillas (`config.py`, `personalization.py`) | Umbrales, textos y contactos | **Sí** en umbrales y textos (verificador bidireccional, E-G). **Parcial** en alias, variables por regla y paquetes de plantillas. | Sí |
| Excel preservado (`services/exports.py`, `sync.py`) | Copia del Excel original con observaciones | **Parcial**: probado con Excel ficticio; faltan reimportación y conciliación. | Parcial |
| Sesión y vigencia (`session.py`, `product_state.py`) | Retomar el trabajo y recibos | **Parcial**: hay sesión por archivo, etapa y corte; faltan recibos y vigencia. | Parcial |
| Descargador: listados (`descargador/lotes.py`, `motor.py`, `perfiles.py`) | Seguimiento, Litigantes, Calendarios y Carga | **Sí** sobre SITFA ficticio: A7/A8 contra Python (E-A78), F2-F4 y D-2. Sin prueba con Chrome real. | Sí |
| Descargador: lecturas de bitácora, historia, calendario e histórico (`lectura_bitacoras.py`, `lectura_historia.py`, `historico_manual.py`) | Arma el contexto de cada causa | **No**: el puente de `app/` rechaza `diary_open`, `history_open` y `calendar_open` con «Acción no permitida.» (prueba en `descargador/puente.rs:440`). Hoy el ZIP de contexto lo sigue produciendo el Descargador Python. | **Sí** (7.4a-c: `flow-captura::{lecturas,vinculos,recoleccion}`, tope de 600) |
| Descargador: lectura de Escritos (`escritos_html.py`) | Escritos de la causa | **No** en el Descargador (Trabajo sí los lee desde el ZIP). | Parcial |
| Descargador: paquete CSMP y modo conjunto (`package_current_lot`, `start_joint`, `export_csmp_capture.py`) | Entrega el ZIP al Asistente | **No** | Parcial (`flow-trabajo::exportar`) |
| Exportaciones de lote (`resultados.py`: libro, CSV, ICS, ZIP, `compare_lots`) | Salidas y comparador de lotes | **No** (F22) | Parcial (`exportar.rs`) |
| Meta 4 (`META4_Bitacoras/Meta4.py`, `scripts/meta4_bitacoras.py`) | Cálculo mensual, cinco productos, plantilla y regeneración | **No**: ninguna coincidencia en `app/src`. | **Sí**: `flow-meta4`, 2.525 líneas; `cargo test -p flow-meta4` da 29 pruebas en verde (N1). No se contrastó con carpetas Python reales (F23). |
| Anonimizador (`anonimizador/anonimizador_csmp`: tablas, texto, JSON, ZIP, verificación posterior) | Anonimiza archivos y paquetes | **Parcial**: tablas, CSV/TSV, texto, JSON e ICS. Omite PDF, imágenes y `.msg`. Sin ZIP ni paquetes de captura. | Más completo (`flow-anonimo`: archivos, documentos, lote y calendario) |
| Intercambio entre herramientas (`download_link.py`, `download_transfer.py`, `capture/temporal.py`) | Solicitud local Asistente→Descargador y comparación temporal | **No** (F27). AGENTS: el Asistente no inicia el Descargador. | No |
| Distribución (`portable/`, `installer/`, `csmp_launcher.py`, self-checks) | Instalación portátil y diagnóstico | **Parcial**: `flow_rust.exe` por CI; sin instalador ni portátil. | Parcial |
| Utilidades de desarrollo (`tools/`, `tools/manuales/`, explorador) | Verificadores, capturas, manuales | **Parcial**: verificadores propios (`verificar_textos_umbrales.py`, `inventario_escrituras.py`, `check_*`); manuales solo en Python | — |

## 2. Lectura de la tabla

- **Hechos (N1):**
  - `app/` no contiene Meta 4, lecturas de contexto del Descargador, exportaciones de lote, formularios manuales ni estadísticas;
  - `rust/` tiene Meta 4 y lecturas de contexto con pruebas en verde, pero `app/` no las importa;
  - el flujo completo de uso real todavía necesita el **Descargador Python** para producir el ZIP de contexto, o un ZIP ya existente.
- **Inferencias (N2):**
  - Supuesto: el uso mensual necesita contexto fresco (bitácoras e historia). Mientras F19-F22 no estén en `app/`, Flow Rust no reemplaza a Flow Python, solo convive con él.
  - Supuesto: el código histórico es transferible, como dice `AGENTS.md`. Integrar `flow-captura::lecturas` y `flow-meta4` es portar y contrastar, no escribir desde cero; aun así cada pieza necesita su prueba golden contra Python.
- **Especulación (N3):** el esfuerzo de esa integración no está medido. No hay datos para estimarlo.

## 3. Orden sugerido para cerrar la brecha (decisión del usuario)

1. **F19 + paquete CSMP** (Descargador Integral: bitácoras, historia, calendario, histórico y ZIP). Es lo que hoy obliga a mantener Python.
2. **F23-F24 Meta 4**: integrar `flow-meta4` como vista y contrastarlo con carpetas Python ficticias.
3. **F22** exportaciones de lote y comparador.
4. Brechas menores: cuenta Outlook, formularios manuales, estadísticas, Anonimizador (ZIP, PDF y paquetes) y distribución portátil.

Decisión D-4 del usuario (23-nov-2026): integrar todo. Los encargos E-Q, E-R, E-S y E-T están en `ENCARGOS_IA.md`.
