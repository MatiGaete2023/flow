# Prompt · Lote 2 (E-D con interfaz automatizada, y E-I)

Copiar desde la línea siguiente y entregar tal cual a la IA ejecutora.

---

Antes de cualquier otra cosa, ejecuta y pega la salida literal de `git log --oneline -1 && git status -sb`. Si no puedes ejecutar comandos en este repositorio, responde solo «SIN ACCESO» y detente.

Eres una IA ejecutora del proyecto Flow Rust, en Windows (entorno W). En esta sesión ejecutas **E-D y después E-I**. No planificas, no redefines criterios y no tocas otros encargos.

## Contexto mínimo

- Repositorio `MatiGaete2023/flow`, rama `estudio/paridad-python-rust-20261119`, desde el commit de la auditoría del lote 1 o uno posterior. Sin `--force`, sin `main` y sin releases.
- Leer antes:
  - `AGENTS.md`;
  - `docs/migracion_python_rust/ENCARGOS_IA.md` (reglas, E-D, E-I y la plantilla);
  - `docs/migracion_python_rust/PLAN_PRUEBAS.md`, paquetes D y F;
  - la sección «Auditoría · lote 1» de `REGISTRO_IMPLEMENTACION.md`, con el comando oficial de la suite con variables;
  - «Turno 16» del registro, para E-I.
- Hechos (N1):
  - `app/` usa `eframe`/`egui` 0.31, y `ui::App::new(screenshot: Option<PathBuf>)` construye la aplicación;
  - `egui_kittest` 0.31.1 existe en crates.io y es compatible con egui 0.31;
  - no hay pruebas de interfaz automatizadas;
  - las comprobaciones D2 del plan hoy son manuales.
- No puedes hacer clics en la interfaz. Por eso las comprobaciones D2 se automatizan con `egui_kittest`, en pruebas que corren en la CI de Windows. Lo que no se pueda automatizar queda como checklist para el usuario (paquete H); no se informa como PASA.

## E-D · Compilación e interfaz en Windows

1. **Pruebas de interfaz**, en `#[cfg(test)]` (un módulo nuevo, por ejemplo `app/src/ui/pruebas_interfaz.rs` o `app/src/pruebas_interfaz.rs`):
   - agregar `egui_kittest = { version = "=0.31.1", features = ["eframe"] }` en `[dev-dependencies]` y actualizar `Cargo.lock`;
   - cada prueba usa un perfil aislado en una carpeta temporal y nunca el perfil real del usuario. Si `App` no permite elegir la carpeta de perfil, agregar solo un constructor `#[cfg(test)]` o un parámetro de prueba. Cualquier otro cambio de producción es `BLOQUEADO`.
   - Pruebas, con los textos que hoy muestra la interfaz (verificar cada uno con `grep` en `app/src`):
     - `ui_ajustes_muestra_16_umbrales_y_filtra`: Ajustes lista 16 umbrales. «Buscar parámetro…» con «prórroga», «búsqueda» y «ventana_busqueda» filtra la lista (contar filas) y vaciar el campo devuelve 16.
     - `ui_trabajo_consigna_otro_programa_y_conserva`: requiere `FLOW_E3_PAQUETE` (helper `prueba_entorno::requerida` y `#[ignore]`, como E-L). Abre `contexto-e3.zip` con corte 19/10/2026. La fila con otro programa vigente muestra «Otro programa vigente». Consignar retira la frase y aparece «Deshacer». Cerrar y volver a abrir el mismo ZIP con el mismo corte conserva la consignación.
     - `ui_descargador_limita_pausa_y_conserva_preferencias`: la pausa no baja de 0,5 s ni sube de 10 s. Destino, «PDF sin registros» y pausa se conservan al reconstruir `App` sobre el mismo perfil. No hace falta iniciar un lote real: basta un lote sobre el SITFA ficticio si la vista lo permite sin red; si no, solo preferencias, e informarlo.
   - Si `egui_kittest` no encuentra un widget por su texto o rol de AccessKit, informar qué widget es y por qué. No reemplazar la prueba por una llamada directa a la lógica que haga pasar la prueba por un camino distinto al de la interfaz.
2. **CI**: las pruebas nuevas corren dentro del paso «Probar todas las pruebas con variables» de `flow-rust-local.yml`, ya que usan `--include-ignored`. No agregar pasos de build ni tocar los artefactos.
3. **Corrida de la CI**: el push dispara «Flow Rust local · Windows».
   - Si puedes consultarla (`gh run list --branch estudio/paridad-python-rust-20261119 --workflow "Flow Rust local · Windows"` y `gh run view <id>`), pega el enlace, la conclusión de cada paso y el SHA-256 del `flow_rust.exe` del artefacto `Flow_Rust_Windows_exe`.
   - Si no puedes, escribe `CI NO VERIFICADA POR LA EJECUTORA` y deja en la entrega la lista exacta de lo que el usuario debe mirar: el run del commit, los pasos en verde, el artefacto y su SHA-256.
4. **Local**: `cargo build --manifest-path app/Cargo.toml --locked --release --bin flow_rust` y el SHA-256 del exe local (`Get-FileHash`). Se informa aparte del de la CI: no son el mismo binario.

Criterio de E-D:
- las tres pruebas existen y pasan en W con el comando oficial con variables;
- sin variables, quedan `ignored` las que dependen de una variable;
- clippy no muestra advertencias nuevas en los archivos tocados;
- la corrida de la CI queda citada, o marcada `NO VERIFICADA` con la checklist para el usuario.

## E-I · Lote Integral y Carga en Windows

1. Revisar si las pruebas de biblioteca existentes (`sitfa_ficticio.rs`, turno 16) ya cubren, sobre el SITFA ficticio: Integral de Seguimiento con dos páginas, Carga y consulta vacía con PDF, con nombres y `resumen.json` como en el turno 16. Listar cada una con `grep -n "fn <nombre>"` y su línea `test result` en W.
2. Lo que falte en la biblioteca, agregarlo como prueba en `sitfa_ficticio.rs`, sin red ni Chrome.
3. La ejecución desde el **ejecutable** con la extensión ficticia sobre HTTP local no se puede automatizar sin interfaz: documentarla como checklist del paquete H para el usuario. Incluir el comando para levantar el servidor ficticio si existe en el repositorio (buscarlo; no inventarlo) y los nombres de archivo esperados.

Criterio de E-I:
- cada uno de los tres casos queda cubierto por una prueba de biblioteca que pasa en W, citada por nombre;
- el checklist del ejecutable queda en el registro;
- no se marca como PASA nada que no se haya ejecutado.

## Reglas comunes

- Un encargo, un commit de código y un commit de registro (`## Entrega · E-D`, `## Entrega · E-I`), publicados antes de pasar al siguiente. Cada estado pasa a `ENTREGADO`.
- Si E-D queda `BLOQUEADO` por la construcción de `App`, igual ejecutar E-I, que no depende de la interfaz.
- Sin red real, sin SITFA real, sin Chrome con sesión del usuario, sin Outlook. Datos solo ficticios.
- Comprobar cada prueba citada con `grep -rn "fn <nombre>" app/` y cada commit con `git cat-file -t <hash>` después del push.
- Comandos en W, sin `--features eframe/x11`. Para los ZIP de contexto: `py -3.12 -m venv`, con `descargador/requirements.txt`, `pandas<3` y `pytest`, luego `generar_contexto_e3.py` y `esperado_contexto_e3.py`, como en la CI.

## Respuesta final

Por encargo:
- commits;
- líneas `test result` literales (sin variables y con variables);
- nombres de las pruebas nuevas;
- estado de la CI (enlace o `NO VERIFICADA`);
- SHA-256 local y de la CI;
- checklist para el usuario;
- resultado por criterio (PASA, FALLA o BLOQUEADO);
- pendientes.
