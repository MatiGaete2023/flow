# Estado de Flow Rust

Actualizado: 24 de noviembre de 2026, sobre `751634b`. Es la única fuente del estado: se edita en el mismo commit que cierra cada encargo.

- Aplicación: `app/` (`flow_rust.exe`). Referencia funcional: rama `flow-python`.
- Rama única: `estudio/paridad-python-rust-20261119`.
- Estados: **Sí** (existe y tiene contraste con Python), **Parcial** (falta lo que se indica), **No** (ausente). Ninguno significa «aceptado»: eso solo lo declara el usuario en un acta.
- Origen: registro y encargos hasta el 24 de noviembre. En esta fecha solo se repitió la suite en Linux (`tools/probar_app.sh todo`: 58 y 43 pruebas, 0 fallos).

## Siguiente paso

1. **Referencia Python (E-V1 de [PLAN.md](PLAN.md)).** Los oráculos usan el Python de esta rama, que difiere de `flow-python` en dos módulos de producción. Hacerlo antes de portar más.
2. **Usuario:** prueba corta del tramo 1 (ver «Pendiente del usuario»).
3. **E-Q5d**, y luego el resto del tramo 1 de [PLAN.md](PLAN.md).

## Pendiente del usuario

- **D-8:** decidir si se retiran de esta rama la documentación y el árbol Python duplicados, que siguen en `flow-python`.
- **Prueba corta del tramo 1** con el `.exe` del último run de «Flow Rust local · Windows»: un lote Integral de Seguimiento con Chrome y la sesión SITFA propia, «Leer bitácoras del lote», «Crear paquete de este lote…», abrir el ZIP en Trabajo y guardar el Excel. Anotar lo que falle; no hace falta acta.
- Revisión visual de las capturas del run (antes E-E) y paquete H: acta de uso real.

## Capacidades

### Asistente

- **F01 Lectura de hojas — Sí.** `lectura.rs`.
- **F02 Trabajo y cruces — Parcial.** Tres etapas, Hoja 2, calendario 91 y CAL-03. Faltan Egresados, OB e identidad multifila.
- **F03 Reglas de Excel — Sí.** `oraculo_python`, 174 casos.
- **F04 Contexto de causa — Sí.** `oraculo_historia`, 2.585 casos; paquete C con ZIP de Flow Python.
- **F05 Otro programa, reiteración y Escritos — Sí.** Cronología con 181 casos, R5 parcial y consignación con deshacer.
- **F06 Revisión de bitácoras — Sí.** V1 y V2 contrastadas en la CI.
- **F07 Importación de paquetes — Parcial.** Falta la conciliación entre capturas.
- **F08 Edición y revisión — Parcial.** Sin cotejo de edición masiva, deshacer ni selecciones ocultas.
- **F09 Sesión y vigencia — Parcial.** Faltan recibos, vigencia y recuperación.
- **F10 Excel preservado — Parcial.** Faltan la reimportación y la conciliación desde el Excel revisado.
- **F11 Configuración y contactos — Parcial.** 16 umbrales y textos con verificador. Faltan alias, selector de variables, plantillas de correo persistentes e importador de matrices y paquetes.
- **F12 Correos y nómina — Parcial.** Sin cotejo de agrupación, firma y alcance.
- **F13 Borradores Outlook — Parcial.** EML y Outlook. Falta elegir la cuenta y simular fallos. Sin prueba con Outlook real.
- **F14 Proyectos Word — Sí.** Matrices de Laja y Mulchén. Por confirmar con el usuario: un `.docx` por proyecto.
- **F15 Formularios manuales — No.**
- **F16 Resultados y estadísticas — Parcial.** Vista y reporte; sin estadísticas ni métricas.

### Descargador

- **F17 Integral y Lite — Sí** sobre SITFA ficticio. Sin prueba con Chrome real.
- **F18 Gestión de lotes — Sí.** Pausa, cancelación, pendientes, favoritos y preferencias.
- **F19 Bitácora, historia y calendario — Sí** sobre SITFA ficticio (E-Q1 a E-Q4, 16 escenarios). Falta el avance por fila en la interfaz.
- **F20 Escritos — Sí** dentro de la lectura de historia.
- **F21 Histórico de Protección — Sí**, con suspensión por fallas.
- **F22 Exportaciones de lote — Parcial.** Paquete `CSMP_CAPTURE` 1.2.0 desde el lote (5 escenarios). Faltan los Excel de bitácoras e historia, el XLSX consolidado, `include_originals`, CSV, ICS, ZIP y el comparador de lotes.
- **F27 Intercambio entre herramientas — No.** Sigue en el alcance (D-6).

### Otras herramientas

- **F23 y F24 Meta 4 — No** en `app/`. Existe en `rust/crates/flow-meta4` con 29 pruebas.
- **F25 Anonimizador — Parcial.** Tablas, texto, JSON e ICS. Faltan ZIP, paquetes y verificación de XLSX; `rust/crates/flow-anonimo` los tiene, con 38 pruebas.
- **F26 Distribución — Parcial.** Solo el `.exe` de la CI.
- **F28 Utilidades y ayuda — Parcial.** Verificadores propios; sin ayuda ni manuales.

## Diferencias declaradas frente a Python

- R1 a R6 y el orden de columnas NURUS: decisiones del usuario (ver [PLAN.md](PLAN.md)).
- D-3: mensaje propio de Rust ante un XML 2003 con entidad externa.
- Carga en XLS BIFF: Python escribe `5.0` y Rust `5`. En HTML y XLSX coinciden.
- Historia: Rust la lee siempre que se elige «Historia de la causa»; Python solo al retomar.
- Lecturas: el puente de Rust no trae el `code` de la extensión, y las fallas quedan con el código genérico.

## Documentos

- Lectura al iniciar: `AGENTS.md`, este archivo y [PLAN.md](PLAN.md).
- Según necesidad: [VERIFICACION.md](VERIFICACION.md), [DISENO.md](DISENO.md), [REGISTRO_IMPLEMENTACION.md](REGISTRO_IMPLEMENTACION.md) y [REVISION_20261124.md](REVISION_20261124.md).
- [historico/](historico/) guarda el registro, los encargos, la matriz y los informes anteriores al 24 de noviembre. No se lee al iniciar.
