# Instrucciones para agentes

Lee, en este orden y nada más salvo que tu paquete lo cite:

1. `PLAN_VIGENTE.md`: esencia, prohibiciones, prioridades y decisiones del usuario.
2. `docs/implementacion/ESTADO.json`: paquetes de trabajo (WP), estado y decisiones D1–D9.
3. `docs/implementacion/PLAN_IMPLEMENTACION.md`: «Reglas inviolables», tu rol y la ficha de tu WP.

Las instrucciones expresas del usuario prevalecen sobre estos documentos.

## Reglas que no se negocian

- Sin escritura automática en RUS ni SATURNO; `registration.submit` siempre lanza error.
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

`python tools/verificar_wp.py` decide PASA/FALLA contra `docs/implementacion/linea_base.json`.
Desde WP-11, el hook y la CI controlan identidades tras la limpieza de WP-10.
Hasta WP-20 se excluye `variantes/`; después se retira esa exclusión.
