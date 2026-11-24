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
