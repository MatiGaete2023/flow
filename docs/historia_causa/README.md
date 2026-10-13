# Historia de la causa en el Descargador — rama base

Rama: `feat/historia-causa-sitfa`, creada desde `feat/ejecucion-plan-20261008` @ `1a2dd0f` el 13 de octubre de 2026.
Objetivo: que el Descargador lea, además de las bitácoras, la **tabla de trámites de la pestaña «Historia»** de la
ventana «Información de la Causa», solo dentro de la **ventana de 4 meses**, para revisión manual en Excel y para
que el Asistente la analice **en otro paso posterior**. Uso personal, Windows, 100 % local.

## Estado

| Elemento | Estado |
|---|---|
| Factibilidad | Favorable, condicionada a H0 (caracterizar la ventana real). Dos informes concuerdan. |
| Decisiones del usuario | Sin PDF · solo campos visibles · ventana de 4 meses · uso personal Windows. Ver [00_DECISIONES](00_DECISIONES.md). |
| Gobernanza | `PLAN_VIGENTE.md` aún dice «Historia SITFA/PDF… fuera de alcance». Primer paso de WP-52: registrar la decisión. |
| Código entregado en esta rama | `descargador/extraer_estructura_historia.py` (extractor anónimo H0) + pruebas + fixtures ficticios. |
| Bloqueo | Falta que el usuario ejecute H0 en su PC y entregue `estructura_historia.json`. Guía: [05_GUIA_USUARIO_H0](05_GUIA_USUARIO_H0.md). |

## Documentos (leer en este orden)

1. [00_DECISIONES.md](00_DECISIONES.md): qué decidió el usuario y qué falta decidir.
2. [01_ANALISIS_CONSOLIDADO.md](01_ANALISIS_CONSOLIDADO.md): ambos informes + repositorio; acuerdos, discrepancias y cómo se resolvieron.
3. [02_ESPECIFICACION.md](02_ESPECIFICACION.md): especificación funcional y técnica (datos, identidad, estados, productos, contrato).
4. [03_PLAN_PASO_A_PASO.md](03_PLAN_PASO_A_PASO.md): paquetes WP-52 a WP-56, paso a paso, archivos, pruebas y criterios de salida.
5. [04_PRUEBAS_Y_ACEPTACION.md](04_PRUEBAS_Y_ACEPTACION.md): pruebas automáticas, guía de prueba real y plantilla de acta.
6. [05_GUIA_USUARIO_H0.md](05_GUIA_USUARIO_H0.md): cómo obtener la estructura anónima en el PC autorizado.
7. [INFORME_FACTIBILIDAD_CLAUDE_20261013.md](INFORME_FACTIBILIDAD_CLAUDE_20261013.md): informe previo (referencia).

El informe de la otra IA («Historia de la causa en el Descargador SITFA», corte `1a2dd0f`) no está en el repo; su
contenido útil está incorporado en 01 y 02.

## Reglas para quien implemente (resumen; mandan `AGENTS.md` y `PLAN_VIGENTE.md`)

- Sin datos reales en repo, CI ni conversaciones: solo `estructura_historia.json` y fixtures ficticios.
- Sin escritura en RUS/SATURNO; sin PDF; sin seguir enlaces de documentos; sin IA.
- Motor, textos, Outlook y reglas CT del Asistente **intactos**. El Asistente no cambia en este alcance.
- Extensión SITFA y contrato de captura son cambios riesgosos: revisión E3 independiente (decisión j).
- No escribir parser ni comando de extensión **antes** de cerrar H0. Si H0 muestra el escenario D, detenerse.
- Cada WP: rama `wp/WP-5X-<slug>` desde esta rama; `python tools/verificar_wp.py` PASA antes de integrar.
