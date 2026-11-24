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
