# Prompt · Lote 1 (E-L → E-C → E-F, en secuencia)

Copiar desde la línea siguiente y entregar tal cual a la IA ejecutora.

---

Antes de cualquier otra cosa, ejecuta y pega la salida literal de `git log --oneline -1 && git status -sb`. Si no puedes ejecutar comandos en este repositorio, responde solo «SIN ACCESO» y detente.

Eres una IA ejecutora del proyecto Flow Rust. En esta sesión ejecutas **tres encargos, uno después del otro y en este orden**: E-L, E-C y E-F. No planificas, no redefines criterios y no tocas otros encargos.

## Reglas del lote

1. Cada encargo se rige por su prompt, que es la fuente de criterios:
   - E-L: `docs/migracion_python_rust/prompts/E-L.md`;
   - E-C: `docs/migracion_python_rust/prompts/E-C.md`;
   - E-F: la sección E-F de `docs/migracion_python_rust/ENCARGOS_IA.md`, con sus cuatro puntos (F2, F3, F4 y D-2).

   Leer el prompt completo de cada uno antes de empezarlo. Ignorar la línea «Antes de cualquier otra cosa…» de cada prompt: ya la cumpliste al inicio.
2. **Un encargo, un commit de código y un commit de registro**, publicados antes de empezar el siguiente. Cada uno lleva su propia sección `## Entrega · E-…` en `REGISTRO_IMPLEMENTACION.md` y su estado pasa a `ENTREGADO` en `ENCARGOS_IA.md`. No mezclar cambios de dos encargos en un commit.
3. **Corte del lote:**
   - Si E-L termina con un criterio `FALLA` o `BLOQUEADO`, detenerse y no empezar E-C ni E-F: sin E-L, las demás pruebas pueden dar verde sin ejecutarse.
   - En E-C, un elemento `BLOQUEADO` del paquete nuevo no detiene el lote. Cualquier `FALLA` de paridad sí lo detiene.
   - E-F se ejecuta solo si E-L y E-C quedaron sin `FALLA`.
4. Desde que E-L esté publicado, todas las corridas de la suite usan el comando oficial con variables que E-L dejó en el registro (con `--include-ignored`). Una prueba `ignored` no cuenta como `PASA`.
5. Rama `estudio/paridad-python-rust-20261119`, desde `HEAD` remoto. Sin `--force`, sin `main`, sin releases.
   - Comprobar cada commit con `git cat-file -t <hash>` después del push.
   - Comprobar cada prueba citada con `grep -rn "fn <nombre>" app/`.
6. **Entorno:**
   - en Linux, usar `--features eframe/x11`;
   - en Windows, omitirlo y declararlo `W` en cada entrega;
   - para el venv de Python en Windows, usar `py -3.12 -m venv`, que trae `tkinter`.
7. No inventar valores, no relajar comparadores, no tocar funciones de producción salvo lo que E-F pide explícitamente (extraer a la biblioteca la regla «solo Seguimiento y Litigantes» de `iniciar`). Esa extracción debe ser un movimiento sin cambio de comportamiento: la prueba D-2 lo demuestra.

## Respuesta final (una sola, al terminar o al cortar)

Por cada encargo ejecutado:
- commits publicados;
- líneas literales de sus criterios:
  - E-L: `test result` sin variables y con variables, y la lista de ignoradas;
  - E-C: `PAQUETE_C`, `PAQUETE_C_CORTE` y la mutación;
  - E-F: el intervalo F2 medido, los pendientes y huellas de F3, el favorito de F4 y los `keep_filters` de D-2;
- resultado por criterio (PASA, FALLA o BLOQUEADO).

Si cortaste el lote, indicar dónde y por qué, y marcar como `NO EJECUTADO` los encargos que no empezaste. Al final, los pendientes.
