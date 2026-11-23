# Plan de pruebas · Flow Rust (23 de noviembre de 2026)

Plan para que otras IA ejecuten, cada una por separado, las pruebas que quedaron fuera de los turnos 13 y 14. Cada paquete es independiente, salvo que indique un requisito previo. Se ejecuta sobre la rama `estudio/paridad-python-rust-20261119`, desde el commit `5405870` o uno posterior.

## Reglas comunes para todos los paquetes

- Leer primero `AGENTS.md`, el [registro](REGISTRO_IMPLEMENTACION.md) y la [verificación](VERIFICACION.md).
- Usar solo datos sintéticos. No usar HAR, HTML, PDF, Excel ni capturas reales, ni credenciales. No escribir en RUS ni en SATURNO. Outlook solo genera borradores.
- No crear ramas, no fusionar a `main`, no usar `--force` y no crear releases.
- No marcar nada como «aceptado» ni «institucional».
- Si una prueba falla, no corregir el código dentro del paquete salvo que el paquete lo pida. Registrar el caso mínimo que la reproduce.
- Al terminar, agregar al registro una sección `Pruebas · <paquete>` con: commit probado, entorno, comandos, resultado PASA/FALLA por caso y siguiente paso. Publicar en la misma rama.
- Para ahorrar tokens: filtrar las salidas con `grep` (`test result`, `FAILED`, `panicked`), no leer archivos completos y no repetir pruebas que ya pasaron en la CI del mismo commit.

## Entornos

- **L · Linux sin escritorio.** Usar el crate aislado `.aislado/` descrito en [Verificación](VERIFICACION.md). Debe incluir los módulos `anon`, `captura_trabajo`, `config`, `copia_excel`, `demo`, `lectura`, `motor`, `productos`, `trabajo` y `util`. Así corren el motor, Trabajo y la biblioteca, pero no la interfaz egui.
- **W · Windows.** Usar el flujo «Flow Rust local · Windows» de GitHub Actions (push o `workflow_dispatch`) o un equipo Windows con Rust, Python 3.12 y Excel de escritorio cuando el paquete lo pida.
- **V · Revisión visual.** Una sesión que pueda descargar artefactos de GitHub Actions. La sesión en la nube del turno 13 no podía: el proxy bloquea el almacenamiento de artefactos.

## Paquete A · Unitarias del código nuevo (entorno L)

Objetivo: cubrir con pruebas Rust el código de los turnos 13 y 14 que el corpus no ejercita directamente. Este paquete sí agrega pruebas al repositorio, sin cambiar la lógica.

1. **A1 · Trámites de contexto** (`captura_trabajo::tramites_causa`).
   - Un trámite anterior a `desde` llega con `contexto = true`.
   - Una lectura con `desde` posterior al corte no entrega trámites.
   - `evidencia_informe` ignora los trámites de contexto.
   - `historia::tramites` los excluye y `tramites_con_contexto` los incluye.
2. **A2 · Consignación P1** (`Trabajo::consignar` y la sesión).
   - Usar un paquete sintético de `app/qa/generar_contexto_e3.py` con otro programa vigente, o un `ContextoFila` armado a mano.
   - Consignar retira `HISTORIA.OTRO_PROGRAMA` y deja `otros_programas` vacío. Consignar dos veces no cambia nada (idempotencia). Deshacer restituye la frase.
   - La consignación sobrevive a `analizar`, al cambio de corte y a guardar y restaurar la sesión. `reiniciar` la borra.
   - Una sesión antigua sin el campo `consignaciones` se lee sin error.
3. **A3 · Preferencias del Descargador** (`descargador::preferencias`).
   - `limitar_pausa(0.0) = 0.5`, `limitar_pausa(30.0) = 10.0` y `limitar_pausa(NaN) = 1.0`.
   - `cargar` de un JSON inválido o con tipos erróneos devuelve los valores por omisión.
   - `guardar` seguido de `cargar` devuelve los mismos valores. No deben quedar archivos `.tmp`.
4. **A4 · Búsqueda R11** (`historia::busqueda_historia`), con casos sintéticos:
   - una orden de contexto con su término en la ventana se dice completa;
   - con dos trámites posteriores, el primero abre con «Posteriormente» y el segundo con «Luego»;
   - un cierre sin orden se dice como constancia neutra;
   - una orden repetida con el mismo folio se dice una sola vez;
   - sin bitácora rige `ventana_busqueda`;
   - la marca `R11_BUSQUEDA_VIGENTE` aparece solo con una orden sin cierre.
5. **A5 · Proyección de ingreso** (`historia::proyeccion_ingreso`).
   - Probar mes sin año, mes con año, «setiembre», mes igual al del corte (no se copia) y mes pasado.
   - Probar una oración precedida por «Nombre SIGLA: » o por otra oración.
   - Comprobar que con proyección vigente no se emiten E04 ni E05.

Criterio: todas las pruebas nuevas pasan, `oraculo_python` sigue en 174/174 y `oraculo_historia` en 2.582/2.582 (`FLOW_CASOS=$PWD/../rust/casos`). Además, `cargo clippy --tests` no agrega advertencias en los archivos tocados.

## Paquete B · Redacción mensual activa sobre el corpus (entorno L)

Objetivo: los oráculos corren con `redaccion_registro = false`. Hay que comprobar que con la opción activa (el valor por omisión del usuario) las únicas diferencias frente a Python sean R1 y R6, y que R5 se conserve.

1. Ejecutar `oraculo_historia` en una copia local con `redaccion_registro: true` (sin publicar el cambio) y `FLOW_DIFF=1`.
2. Clasificar cada diferencia:
   - R1: con la medida por vencer se omite la oración C10;
   - R6: nombre del programa en tipo título;
   - cualquier otra diferencia es FALLA.
3. Comprobar en las mismas salidas que R2, R3 y R4 no aparecen en la observación.

Criterio: no hay diferencias fuera de R1 y R6. Registrar el conteo por tipo.

## Paquete C · Integración con ZIP sintéticos de Flow Python (entorno W, o L con `FLOW_REFERENCIA`)

Objetivo: probar de punta a punta, desde un ZIP 1.2.0, las reglas que hasta ahora solo se contrastaron con el corpus.

1. Ampliar `app/qa/generar_contexto_e3.py` con un paquete ficticio `contexto-e3-historia.zip`. Debe traer:
   - una orden de búsqueda de contexto y su término;
   - continuidad de atención;
   - una prórroga reciente con la medida por vencer;
   - una designación de curador con RUS sin curador;
   - una causa P con sentencia;
   - una bitácora con proyección de ingreso para un mes futuro.
2. Abrir ese ZIP en Trabajo (prueba `--bin flow_rust` con `FLOW_E3_PAQUETE`) en Espera, Cumplimiento e Informes. Comparar observación, reglas y marcas con las que produce Flow Python sobre el mismo ZIP (`nurus.personal.work`).
3. Repetir con el corte movido antes del inicio de la ventana: no debe haber constancias de historia.

Criterio: las filas coinciden con Python, salvo R1 y R6 si la redacción mensual está activa. Agregar el caso a la corrida Windows si pasa.

## Paquete D · Compilación e interfaz (entorno W)

Requisito previo: corrida Windows verde del commit probado.

1. Confirmar que la corrida del push compiló `ui.rs` y `descargador_ui.rs`, que pasaron las pruebas de la aplicación y que se generaron el ejecutable y las capturas.
2. Con el ejecutable y el ejemplo ficticio, en un perfil aislado:
   - Ajustes muestra 16 umbrales; «Buscar parámetro…» filtra por «prórroga», «búsqueda» y «ventana_busqueda», y vaciar el campo restituye la lista;
   - en Trabajo con `contexto-e3.zip`, la fila con otro programa vigente muestra la sección «Otro programa vigente». Consignar retira la frase y deja el botón «Deshacer». Cerrar y abrir el mismo ZIP con el mismo corte conserva la consignación;
   - en el Descargador, la pausa no baja de 0,5 s ni sube de 10 s. El destino, el PDF sin registros y la pausa se conservan tras iniciar un lote ficticio y reiniciar la aplicación.

Criterio: los tres puntos PASA. Registrar el SHA-256 del ejecutable usado.

## Paquete E · Revisión visual (entorno V)

1. Descargar `Flow_Rust_Local_Capturas_Windows` de la corrida del commit probado.
2. Comparar las cinco vistas y `trabajo-contexto.png` con [Diseño](DISENO.md): jerarquía, navegación lateral, tabla, paneles, tarjetas y acciones.
3. Revisar en especial que la tarjeta de Umbrales de Ajustes no desborde con 16 filas y el buscador, y que la sección «Otro programa vigente» use el tema vigente.

Criterio: sin cambios de jerarquía ni contenido respecto del diseño. Las diferencias de fecha, escala o fuente se anotan aparte.

## Paquete F · Descargador Lite con SITFA ficticio (entorno L para la biblioteca, W para la interfaz)

1. Ejecutar las pruebas de `descargador` del crate aislado (`cargo test --lib`) y confirmar que no hay regresiones por el módulo `preferencias`.
2. Con `sitfa_ficticio`, ejecutar un lote Lite con una pausa de 0 s pedida por la interfaz y comprobar que la ejecución usa 0,5 s. El tiempo entre consultas consecutivas debe ser de al menos 0,5 s.

Criterio: las pruebas existentes pasan y la pausa mínima se respeta.

## Paquete G · Invariantes y seguridad (entorno L)

Auditoría estática, sin ejecutar la aplicación:

1. No hay `.Send(` en `app/` y el CC `ucc_concepcion@pjud.cl` sigue fijo en los borradores.
2. No hay escritura en RUS ni en SATURNO, ni un módulo de registro.
3. No hay credenciales, cookies ni tokens en el código, los fixtures o la documentación nueva.
4. Los textos agregados en `config.rs` (R11, continuidad, solicitud DCE, REITERA, MATERIA_*, P_FINALIZADA) coinciden carácter por carácter con `src/nurus/personal/textos_base.json`.
5. `UMBRALES` coincide en claves y valores con `src/nurus/personal/config.py::UMBRALES` y sus etiquetas con `PARAMETER_LABELS`.
6. Nada se escribe fuera de `%LOCALAPPDATA%\FlowRust` y `%APPDATA%\FlowRust`, salvo los productos que elige el usuario.

Criterio: los seis puntos PASA. Para el punto 4, adjuntar el script usado al registro.

## Paquete H · Aceptación del usuario (requiere al usuario)

Este paquete no lo ejecuta una IA: el acta corresponde al usuario. Una IA solo puede preparar la guía, a partir de `docs/aceptacion/GUIA_PRUEBAS_USUARIO_20261013.md` adaptada a Flow Rust. La guía debe cubrir:

- Excel desde ZIP, marcas, «Instalar extensión…», contactos importados, nómina editable, Word con matriz y redacción R1 y R6;
- consignación de otro programa;
- preferencias del Descargador y un lote Lite real con su sesión.

## Orden sugerido

D (confirma que compila) → A y G en paralelo → B y C → E y F → H.
