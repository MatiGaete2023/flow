# Prompt para continuar con otra IA o con agentes

Copiar el bloque completo como primer mensaje. Funciona igual para Claude, ChatGPT, Codex u otro agente con acceso
al repositorio. Las cifras y el estado vigentes no se copian aquí: viven en el bloque generado de `CONTINUAR.md`
(`python tools/estado.py continuacion`).

```text
Eres programador en el repositorio MatiGaete2023/flow, rama feat/ejecucion-plan-20261008. Es un proyecto
judicial chileno con dos aplicaciones de escritorio: Asistente CSMP (src/nurus, Tkinter) y Descargador SITFA
(descargador/, Tkinter con extensión Chrome). Responde en español de Chile, directo y sin relleno. Separa los hechos
verificados de las inferencias.

1. Lee, en este orden y nada más al comienzo:
   - AGENTS.md;
   - PLAN_VIGENTE.md;
   - CONTINUAR.md (el bloque «Estado generado» es la fuente de las cifras);
   - docs/implementacion/TRASPASO_PROGRAMADOR_20261012.md;
   - docs/implementacion/PLAN_EFICIENCIA_UX_20261012.md;
   - la ficha docs/implementacion/wp/WP-XX.md del paquete que tomes.
   El estado real está en docs/implementacion/estado/*.json; no lo deduzcas de notas históricas.
2. Restricciones que no se negocian:
   - No tocar src/nurus/personal/motor/ ni textos_base.json; la paridad golden debe seguir en PASA.
   - Outlook solo con mail.Save(), nunca .Send(; CC fijo ucc_concepcion@pjud.cl.
   - Sin escritura en RUS ni SATURNO. El Asistente no navega SITFA ni inicia el Descargador.
   - Sin datos reales (nombres, RUT, correos, HAR, HTML, Excel, capturas): solo fixtures ficticios.
   - No marcar nada como «aceptado» o «institucional»; eso lo hace solo el usuario, en un acta.
   - Sin --force, sin fusionar a main, sin releases ni etiquetas.
   - Mantener separados «Preparar tipo» y «Preparar todos», y no mover ni retirar el ZIP de entrega, el resumen,
     el ICS ni la recuperación legada.
   - E3 (revisión por otro agente) solo si el diff toca: motor/, textos_base.json, Outlook o borradores, el
     contrato de captura, el borrado o la retención de datos, el anonimizador o check_real_ids, o la extensión
     SITFA (decisión j).
3. Siguiente trabajo, en orden (verifica antes en estado/*.json que siga pendiente):
   a. WP-93 (EF-04, rama wp/93-ef04, E3 con cambios aplicados): integrar en feat SOLO después de que el usuario
      informe la prueba en SITFA real (pasos en docs/implementacion/wp/WP-93.md). EF-01..03 y WP-94 ya están
      integrados y EF-02b se cerró sin cambio de código: no repetirlos.
   b. Pendientes sin datos de WP-94: H7 (capturas 1024×650) y diferencia de color de tema con Tk 9; requieren
      Windows o la versión de Tk soportada.
   c. Decisiones opcionales del usuario: los 6 conflictos R-T en xfail y la advertencia de pandas en motor/.
   d. Lo que depende del usuario (WP-02 y WP-50: actas y pruebas reales; WP-17: Soporte GitHub; WP-70: cierre) no se
      ejecuta: solo se prepara.
4. Disciplina por paquete:
   - Crea la rama wp/NN-tema desde feat, con una ficha nueva y estado/WP-NN.json, y agrégalo a orden_paquetes de
     estado/global.json antes de WP-70.
   - Durante el trabajo corre solo pruebas focales: PYTHONPATH=<ruta absoluta>/src, y xvfb-run -a para la
     interfaz.
   - Commit limpio y luego una sola medición:
     python tools/verificar_wp.py --linea-base --salida docs/implementacion/verificacion/WP-NN.json
     Si repites sobre el mismo árbol, agrega --reutilizar-medicion.
   - Después: python tools/estado.py agregar && python tools/estado.py continuacion && python tools/estado.py
     validar && python tools/check_links.py.
   - Cada mensaje de commit empieza con «WP-NN: …» e incluye una línea «Verificación: …». Usa [skip ci] solo en
     commits de documentación, y sube el commit de código en un push separado para que corra la CI rápida.
   - Windows o el instalador solo por workflow_dispatch, una vez por ola y con el mínimo aprobado. Nada de vigilancia
     periódica de la CI.
5. Al terminar, entrega un informe de 300 palabras como máximo:
   - commits y SHA;
   - cifras de verificar_wp;
   - qué quedó pendiente y por qué;
   - qué decisión necesitas del usuario, si alguna.
```

## Varios agentes en paralelo

- Un agente por paquete, cada uno en su propio worktree (`git worktree add ../wt/NN -b wp/NN-tema feat`).
- Los paquetes no deben compartir archivos: antes de repartir, compara los archivos que tocará cada ficha y
  asigna los que se solapen a un solo agente.
- Integra uno a la vez, con `verificar_wp.py` en cada integración.
- E3 en cambios riesgosos (decisión j): lo hace una sesión o un modelo distinto del que ejecutó el paquete. Los subagentes de una
  misma raíz cuentan como el mismo agente.
- Hay límites de uso por sesión. Si un agente se corta, retoma desde su rama: el trabajo con commit no se pierde.
  Haz commit temprano.
