# Registrador RUS — prototipo aislado

No es software productivo ni permiso institucional. No conecta a RUS. No instala extensión, no escribe en la bitácora ni integra Flow.

P1 valida la estructura cerrada de RegistroFlowV1, identidad completa (no acredita identidad remota), texto exacto UTF-8 y SHA-256, destino y estado independientes, fecha UTC y versión positiva. Las capacidades están desactivadas. Comparar la aprobación congelada contra la versión vigente usando require_same_approval; la procedencia y autorización reales no se autentican con el hash. No se impone el contador nominal de 2000 hasta verificar la UI.

Prueba desde la raíz del repositorio (Python 3.12 objetivo; probado en 3.13.5 Linux):
    
    python -m unittest discover -s registrador_rus/tests -p test_contracts.py -v

El esquema JSON describe la estructura; la validación Python es obligatoria para digest, fecha real, Unicode y aprobación vigente.

Continuidad y límites: [CONTINUAR.md](CONTINUAR.md).
