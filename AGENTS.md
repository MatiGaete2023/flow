# Instrucciones para agentes

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

`python tools/verificar_wp.py` decide PASA/FALLA contra `docs/implementacion/linea_base.json`.
Para `flow-python`, pasa ambas listas documentadas:

```bash
python tools/verificar_wp.py \
  --lista-eliminadas docs/implementacion/verificacion/integracion-20261021-eliminadas.txt \
  --lista-eliminadas docs/implementacion/verificacion/flow-python-20261127-eliminadas.txt
```

Desde WP-11, el hook y la CI controlan identidades tras la limpieza de WP-10.
