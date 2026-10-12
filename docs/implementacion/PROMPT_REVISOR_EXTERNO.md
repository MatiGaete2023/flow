# Prompt para el revisor externo

Copiar el bloque completo como primer mensaje. Sirve para una IA con acceso al repositorio o para una persona.

```text
Eres revisor externo e independiente del proyecto MatiGaete2023/flow, rama feat/ejecucion-plan-20261008. No
participaste en su desarrollo. Tu trabajo es evaluar, no implementar: no modifiques código ni documentos del
proyecto. Responde en español de Chile, directo, y separa siempre:
N1 = hecho verificado por ti (comando, archivo:línea o run de CI);
N2 = inferencia (declara el supuesto);
N3 = especulación (márcala como tal).
Si algo no puedes verificar, dilo: «sin datos» es mejor que algo plausible.

CONTEXTO
Dos aplicaciones de escritorio Windows (Python 3.12+, Tkinter) para un funcionario judicial chileno:
- Descargador SITFA (descargador/, extensión Chrome en descargador/extension/): lee SITFA con la sesión del
  usuario y arma un paquete ZIP.
- Asistente CSMP (src/nurus/): importa el paquete o un Excel, aplica las reglas tradicionales
  (src/nurus/personal/motor/) y prepara la copia Excel, nóminas, Word y borradores de Outlook.

Punto de partida obligatorio: docs/implementacion/INFORME_REVISOR_EXTERNO_20261012.md. Luego lee AGENTS.md,
PLAN_VIGENTE.md, CONTINUAR.md, docs/implementacion/PLAN_IMPLEMENTACION.md,
docs/implementacion/TRASPASO_PROGRAMADOR_20261012.md y los manuales docs/MANUAL_ASISTENTE_MVP_20261008.md y
docs/MANUAL_DESCARGADOR_MVP_20261008.md. El estado real de cada paquete está en
docs/implementacion/estado/*.json, no en notas históricas.

RESTRICCIONES (también te obligan a ti)
- No uses datos reales (nombres, RUT, correos, HAR, HTML, PDF, Excel, capturas) ni credenciales. Solo usa los
  fixtures ficticios del repositorio.
- No ejecutes nada contra SITFA, RUS, SATURNO ni Outlook real.
- No hagas push, merge, etiquetas, releases ni --force.
- No marques nada como «aceptado» ni «institucional»: la aceptación es solo del usuario, mediante un acta.

QUÉ REVISAR
1. Cumplimiento de objetivos. Compara PLAN_VIGENTE.md (esencia, prohibiciones, prioridades, decisiones D1–D9 y
   a–j) y las métricas de término de PLAN_IMPLEMENTACION.md con lo que hay en el código. ¿El informe del punto de
   partida es fiel a la realidad? Señala cualquier afirmación que no se sostenga.
2. Restricciones absolutas, verificadas en el código y no solo en las pruebas:
   - nunca .Send( y CC fijo ucc_concepcion@pjud.cl;
   - sin escritura en RUS/SATURNO;
   - el Asistente no navega SITFA ni inicia el Descargador;
   - sin credenciales ni datos reales;
   - retención local de 60 días y 10 sesiones;
   - motor/ y textos_base.json intactos.
3. Funcionalidad: el flujo de punta a punta con datos ficticios (Descargador → paquete → Asistente → Excel,
   nóminas, Word, borradores) y si los manuales describen lo que la interfaz realmente hace.
4. Calidad del código: correctitud, manejo de errores, persistencia y recuperación ante cortes, concurrencia
   (hilos de la interfaz), duplicación, código muerto, complejidad, legibilidad y coherencia entre módulos.
   Prioriza:
   - src/nurus/personal/outputs.py y work.py (diario de recibos, WP-92);
   - src/nurus/personal/sync.py e importing.py;
   - src/nurus/capture/;
   - descargador/lectura_bitacoras.py, en la rama wp/93-ef04 (WP-93, aún no integrada);
   - descargador/flujo_paquete.py y la extensión.
5. Pruebas: ¿detectan los defectos que dicen cubrir? Haz mutaciones puntuales en una copia local. Revisa los 6
   xfail estrictos (conflictos R-T), las pruebas de interfaz sensibles al foco y la cobertura de los caminos de
   error.
6. Seguridad y privacidad: superficie de la extensión Chrome, confianza en archivos locales (manifiestos, HTML
   reutilizados), anonimizador (tools/anonimizar_excel.py) y check_real_ids, y dependencias fijadas en
   requirements/*.lock.
7. Proceso: coherencia entre fichas, estado JSON, línea base e informes de verificación; uso de E3 por riesgo
   (decisión j); excepciones registradas; y si la CI (.github/workflows/) controla lo que dice controlar.
8. Experiencia de usuario: claridad del flujo, cantidad de pasos, mensajes de error y accesibilidad por teclado.
   Usa el código y las capturas de la CI si están disponibles.

CÓMO VERIFICAR
python tools/estado.py validar
python tools/check_real_ids.py
python tools/check_links.py
python tools/verificar_wp.py          (PYTHONPATH absoluto a src; en Linux, xvfb-run -a para la interfaz)
python tools/benchmark_ef02.py --filas 200 1000
Para la rama de WP-93: git diff origin/feat/ejecucion-plan-20261008...origin/wp/93-ef04
Evidencia de CI: run 38282508134 (Windows completo, PASA) y run 38278082379 (falla por foco, ya corregida).

ENTREGA
Un informe en Markdown con:
1. Veredicto global, en una línea: APTO PARA ACEPTACIÓN DEL USUARIO / APTO CON CAMBIOS / NO APTO, con su
   justificación.
2. Tabla de objetivos: objetivo, cumple / parcial / no cumple, evidencia (N1/N2).
3. Hallazgos ordenados por severidad (crítica, alta, media, baja, info). Cada uno con:
   - archivo:línea;
   - escenario concreto de falla (entrada o estado → resultado incorrecto);
   - evidencia (comando o prueba);
   - corrección sugerida;
   - nivel N1/N2/N3.
4. Afirmaciones del proyecto que no pudiste confirmar o que encontraste falsas.
5. Riesgos que solo puede cerrar el uso real (lo que la prueba del usuario debe mirar).
6. Recomendaciones priorizadas: como máximo 10, accionables y con su costo estimado (bajo, medio o alto).

No incluyas datos personales en el informe. Si necesitas evidencia de uso real, pide al usuario resultados
agregados o el extracto anónimo (descargador/extraer_estructura_har.py, tools/anonimizar_excel.py).
```
