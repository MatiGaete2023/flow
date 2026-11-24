# Registrador RUS — prototipo aislado

El usuario autorizó desarrollar escritura con autorización humana exclusivamente en esta rama (24 de noviembre de 2026); ver excepción en AGENTS y PLAN_VIGENTE. La versión actual no es software productivo ni acredita permiso institucional. No conecta a RUS. No instala extensión, no escribe en la bitácora ni integra Flow.

P1 valida la estructura cerrada de RegistroFlowV1, identidad completa (no acredita identidad remota), texto exacto UTF-8 y SHA-256, destino y estado independientes, fecha UTC y versión positiva. Las capacidades están desactivadas. Comparar la aprobación congelada contra la versión vigente usando require_same_approval; la procedencia y autorización reales no se autentican con el hash. No se impone el contador nominal de 2000 hasta verificar la UI.

Prueba desde la raíz del repositorio (Python 3.12 objetivo; probado en 3.13.5 Linux):
    
    python -m unittest discover -s registrador_rus/tests -p test_contracts.py -v

El esquema JSON describe la estructura; la validación Python es obligatoria para digest, fecha real, Unicode y aprobación vigente.

Continuidad y límites: [CONTINUAR.md](CONTINUAR.md).

Para comprobar también el esquema (solo desarrollo, requiere jsonschema de los extras dev de Flow):

    python -m unittest discover -s registrador_rus/tests -p test_schema.py -v

P2: detector pasivo y fixture ficticio, no extensión instalable. Para ejecutar la prueba DOM se necesita Playwright y Chromium **solo en desarrollo**, no en el PC institucional. Usar `python registrador_rus/tests/run_dom_detection.py` con Chromium de Playwright instalado, o definir REGISTRADOR_CHROMIUM con la ruta de un Chromium local. No requiere sitio RUS; la prueba bloquea solicitudes de red. Resultado esperado: PASS 14. Selectores de producción aún no verificados.

P3: preparación y localización en DOM exclusivamente ficticio. Ejecutar desde raíz `python -m registrador_rus.tests.run_preparation` (mismas dependencias de desarrollo de P2); resultado esperado PASS 35. Falta adaptador real de las vistas y apertura de bitácora; no contiene guardado.

P4: SimulationJournal, diario local WAL de intentos ficticios. Prueba: `python -m unittest discover -s registrador_rus/tests -p test_journal.py -v` (6 métodos). Solo usar datos ficticios hasta resolver protección y retención local, recibos y UI de autorización; la clase no escribe en RUS.

P5: programador local integrado con el diario ficticio. Límite móvil opcional (apagado), intervalos y pausa persistentes. Prueba: `python -m unittest discover -s registrador_rus/tests -p test_scheduler.py -v`. El cierre de intervalos tras recibo se integrará en P6. No controla ni certifica límites de RUS.

P6a: comparación pura de instantáneas con formato del lector existente, sin red. Prueba: `python -m unittest discover -s registrador_rus/tests -p test_readback.py -v`. COINCIDENCIA_UNICA no es todavía un recibo persistido ni acredita un registro real.

P6b: el diario ficticio requiere lectura previa completa y reciente antes de reservar, y persiste recibos solo ante evidencia nueva única. `python -m unittest discover -s registrador_rus/tests -p test_readback_store.py -v`. Todos los recibos son simulados; COMPROBADA no acredita registro institucional. Conciliación Excel pendiente.

P6c: conciliación ficticia por identidad, texto y recibo a una copia XLSX simple. Prueba: `python -m unittest discover -s registrador_rus/tests -p test_reconcile_excel.py -v`. Requiere openpyxl 3.1 (ya dependencia de Flow) y sistema de archivos que admita hardlink local. No sobrescribe; archivos complejos, XLS/XLSM y Excel nativo Windows no verificados. No equivale a certificación de fidelidad universal.

P7a: puente localhost de simulación, sin consumidor real. Prueba: `python -m unittest discover -s registrador_rus/tests -p test_bridge.py -v`. El ack solo confirma recepción categórica; no acredita registro. No admite write_one.

P7b/P8-sim: controlador de simulación sin red y adaptador ficticio cerrado. `python -m unittest discover -s registrador_rus/tests -p test_simulation.py -v`. Todos los guardados y confirmaciones son ficticios; no instalar ni habilitar RUS.

P7c: UI ficticia: `python -m registrador_rus.app.main` desde raíz. Prueba GUI (requiere pantalla Tk): `python -m unittest discover -s registrador_rus/tests -p test_demo_ui.py -v`. Ocho operaciones ficticias y puente opcional. Extensión de demostración en extension/, sin acceso a RUS. `python -m registrador_rus.tests.run_browser_demo` verifica puente y DOM mediante transporte de prueba; carga MV3 nativa pendiente por política del Chromium de desarrollo. El ack nunca autoriza ni registra.

P9-sim: esquemas de configuración y puente, prueba de 400 operaciones ficticias y batería transversal. Ver [Manual de demostración](docs/OPERACION_Y_RECUPERACION.md) y [Evidencia sintética](docs/EVIDENCIA_SINTETICA.md). No hay escritor real ni release productiva.

Revisión de lote: app/batch_review.py comprueba selección inmutable y genera resumen categórico del diario ficticio. Prueba focalizada: `python -m unittest discover -s registrador_rus/tests -p test_batch_review.py -v`. No integrada a UI ni autoridad real.

Detector estructural complementario: extension/structure.js, exclusivamente pasivo, reconstruido a partir de la evidencia sanitaria V1.5/V1.6. Prueba: `python registrador_rus/tests/run_structure.py` (Playwright/Chromium solo desarrollo). Ver [evidencia estructural](docs/EVIDENCIA_ESTRUCTURAL.md). No se carga en la extensión de demostración ni habilita uso en RUS.

Adaptador Work→RegistroFlowV1: app/flow_export.py, solo contrato y prueba con dobles de Work/Row. `python -m unittest discover -s registrador_rus/tests -p test_flow_export.py -v`. No conectado a Flow ni autorizado para RUS.

Revisión persistente del lote: app/batch_store.py (SQLite, ficticia, aún no conectada al diario ni UI). `python -m unittest discover -s registrador_rus/tests -p test_batch_store.py -v`. No autoriza un intento individual.

La demo incluye botón Revisar lote ficticio; el diálogo y su registro SQLite pasaron pruebas independientes (test_batch_dialog.py, requiere pantalla Tk). La conexión con la aplicación completa aún requiere una prueba integrada. La revisión no equivale a autorización individual y la sesión demo se elimina al cerrar.

Puerta Work real (app/flow_work.py) y pruebas de procedencia/lector/diálogo: verificadas en [CI 45426816043](https://github.com/MatiGaete2023/flow/actions/runs/45426816043), 68/68 pruebas Python. No se ha conectado a la UI del Asistente ni a RUS.

Revisión masiva Work y transferencia de previsualización: app/flow_batch.py y app/transfer.py; solo en memoria. Hasta 400 operaciones, una validación de fuente por fase. La recepción no autoriza intentos. Prueba de integración: test_flow_batch.py (fixture de Flow, CI pendiente).
