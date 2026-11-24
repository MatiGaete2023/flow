# Instrucciones para agentes

## Excepción exclusiva de la rama feat/registrador-observaciones-rus-v3 (24 de noviembre de 2026)

Decisión expresa del usuario: «esta rama tendra una excepcion, donde se permite escribir en RUS, pero siempre con autorizacion humana». Se resuelve el conflicto de alcance de desarrollo del registrador V3.0 únicamente en esta rama; no se cambia la decisión D3 sobre el módulo anterior ni la política de las ramas productivas. No fusionar esta excepción.

Alcance: nuevo módulo autónomo, escritura mediante la interfaz normal de creación de observaciones, nunca edición/borrado ni POST directo. El escritor se desarrollará en P8, desactivado por defecto. Cada escritura deberá pertenecer a una intención aprobada por un funcionario humano, con identidad, texto, estado y destino congelados, y confirmación explícita del conjunto exacto de operaciones autorizadas antes de ejecutar; no se admiten aprobaciones tácitas, generadas por IA ni autorización general que sobreviva cambios de contenido. P8/P9 deben mantener un solo intento activo, diario persistente antes de cualquier guardado, lectura posterior y bloqueo sin reenvío ante incertidumbre. El permiso para desarrollar no acredita por sí solo permisos institucionales o del operador para ejecutar un piloto en RUS real; esos requisitos y selectores reales deben comprobarse antes de habilitar uso productivo. Las pruebas de desarrollo son ficticias y sin acceso institucional.

Esta excepción prevalece en esta rama solo sobre las prohibiciones de implementar el nuevo registrador y la orden anterior de rama única. El resto de restricciones, incluida privacidad, aislamiento del Descargador y prohibición de eludir controles, sigue vigente.


## Rama única y continuidad (instrucción del usuario, 19-nov-2026)

- **Trabaja solo en la rama `flow-python`.** No crees ramas nuevas; si tu entorno te asigna otra, haz
  `git fetch origin flow-python`, trabaja sobre ella y empuja a `origin flow-python`.
- Antes de empezar lee `docs/python_unico/PLAN.md` y `docs/python_unico/REGISTRO.md` (estado, siguiente paso exacto).
- **Al terminar cada turno** (antes de quedarte sin tokens) actualiza `docs/python_unico/REGISTRO.md`: qué hiciste, cómo
  lo verificaste y el siguiente paso exacto; luego commit y push a `flow-python`. Así otro chat continúa desde ahí.
- Flow Rust está congelado (decisión v de `PLAN_VIGENTE.md`): solo se mejoran las herramientas Python.

Lee, en este orden y nada más salvo que tu paquete lo cite:

1. `PLAN_VIGENTE.md`: esencia, prohibiciones, prioridades y decisiones del usuario.
2. `docs/implementacion/ESTADO.json`: paquetes de trabajo (WP), estado y decisiones D1–D9.
3. `docs/implementacion/PLAN_IMPLEMENTACION.md`: «Reglas inviolables», tu rol y la ficha de tu WP.

Las instrucciones expresas del usuario prevalecen sobre estos documentos.

Trabajo en curso (plan de integración 18-nov-2026: otro programa vigente, informe de avance pendiente, contrato 1.2.0): leer
`docs/integracion_20261118/REGISTRO.md` (estado, decisiones y pendientes) antes de tocar `motor/`, el contrato o Rust.

## Reglas que no se negocian

- Sin escritura automática en RUS ni SATURNO. Sin módulo de registro RUS ni SATURNO; si reapareciera, `submit`
  debe lanzar error.
- Outlook solo borradores (`mail.Save()`); nunca `.Send(`. CC fijo `ucc_concepcion@pjud.cl`.
- Sin credenciales, cookies ni tokens en código, logs, fixtures o conversaciones.
- **Sin HAR, HTML, PDF, Excel ni capturas reales; solo estructura anónima** generada
  localmente (`descargador/extraer_estructura_har.py`, `diagnostico_bitacoras.json`,
  `tools/anonimizar_excel.py`). Si alguien ofrece un archivo real, pedir el extracto anónimo.
- Reglas, umbrales y textos tradicionales intactos (`src/nurus/personal/motor/`,
  `textos_base.json`) salvo decisión expresa del usuario y paridad golden.
- El Asistente no navega SITFA ni inicia el Descargador.
- No marcar nada como «aceptado» o «institucional»: solo el usuario, en un acta.
- Sin `--force`, sin fusionar a `main` y sin releases salvo autorización escrita del usuario.

## Verificación

`python tools/verificar_wp.py --lista-eliminadas docs/implementacion/verificacion/integracion-20261021-eliminadas.txt`
decide PASA/FALLA contra `docs/implementacion/linea_base.json` y valida las bajas históricas ya documentadas.
Desde WP-11, el hook y la CI controlan identidades tras la limpieza de WP-10.
