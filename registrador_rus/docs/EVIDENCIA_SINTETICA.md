# Evidencia de verificación sintética — 24 de noviembre de 2026

Base de la batería: e2eae73454c400500bb1934b65a328617229f251 más esquemas y pruebas de este paquete. Linux, Python 3.13.5, Chromium local, Tk/Xvfb, openpyxl y jsonschema de desarrollo. Sin sesión ni solicitudes a RUS.

| Prueba | Resultado |
|---|---|
| unittest discover test_*.py | 49 OK, 2,717 s |
| run_dom_detection | PASS 14 |
| run_preparation | PASS 35 |
| run_browser_demo | 8 preparaciones y rechazo de operación ajena; transporte de prueba |
| run_batch_demo | 400 COMPROBADA ficticias, 400 intentos, 400 recibos, integrity_check OK |

Benchmark: cupo simulado 100/h y 2 s de intervalo adicional; pausa y reapertura SQLite a mitad. Una corrida: 3,107 s de ejecución local (tracemalloc activo), 11.098 s de reloj virtual, 399 esperas virtuales, pico de asignaciones Python 871.671 bytes (no RSS ni consumo total). Medianas por fase: preparación 1,129 ms, autorización simulada 0,464 ms, ejecución y relectura 4,022 ms. No mide latencia de RUS, navegador, interacción humana ni red. Los 400 registros se generan localmente; autorizaciones de prueba mediante función, no actos humanos reales. El FakeRus se mantiene en memoria al reabrir el diario.

Los resultados no constituyen aceptación productiva. La instalación MV3 no se pudo comprobar: política del navegador bloquea chrome://extensions/. La prueba de consumidor usa Chromium DOM, HTTP localhost y SHA-256 en Python con adaptadores de prueba; no acredita CORS/WebCrypto nativos. Falta P0 arranque de Flow por bloqueo DNS de Git directo, DOM real, Windows, Excel nativo, prueba de integración con Flow y piloto RUS. No se ha ejecutado escritor real ni se ha creado release.

## CI integrada del registrador — resultado verificado (24 de noviembre de 2026)

- [Corrida 45426816043](https://github.com/MatiGaete2023/flow/actions/runs/45426816043), job 138560955196, SHA probado `8981f2a6daf543efd4f9ce6b3fe0f348b8e6158f`: conclusión success. Python 3.12.16, Ubuntu, Tk/Xvfb, dependencias de requirements/linux-py312.lock. Log: Ran 68 tests in 3.902s; OK. Checkout y dependencias aprobados. No se ejecutaron las pruebas JS/Chromium independientes ni las suites de otros productos en esta corrida.
- Integra diálogo de revisión con el DemoApp completo, persistencia en el diario temporal y bloqueo sin autorización individual; Work/Row reales con paquete creado por el fixture de Flow, verificación de procedencia y bloqueo de cambios en fuente/copia; lector HTML existente con dos destinos, baseline, lectura parcial y recuperación sin reintento; huella única de lote. Se corrigió una expectativa de prueba dependiente del comprobador RFC3339 opcional: el contrato Python continúa rechazando fechas imposibles. Primera CI: 67/68 (corrida 45426483389); segunda CI: 68/68. Sin datos institucionales, acceso RUS ni escritura remota. No equivale a importar un Work mediante la UI real: la prueba instancia Work/Row con el paquete sintético.
- Pendiente conectar la exportación a la UI del Asistente, definir formato de entrega/importación local, obtener catálogo real de filtros de Informes, reducir relecturas del ZIP al procesar lotes, revisar advertencias bloqueantes con la lógica vigente, validar DOM real en modo pasivo y posterior piloto unitario autorizado. El prototipo sigue limitado a simulación; no se requiere repetir las 8 pruebas de operador ni grabaciones anteriores por esta revisión. Este commit es solo documentación, [skip ci] evita una corrida duplicada.
