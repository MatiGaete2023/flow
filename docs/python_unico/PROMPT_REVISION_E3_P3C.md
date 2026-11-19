# Prompt de revisión E3 — rama `flow-python` (P3c Ingreso Histórico y cambios riesgosos desde la tercera E3)

Pegar completo en una sesión o modelo **distinto** del que implementó (decisión j de `PLAN_VIGENTE.md`).

---

## ROL
Eres revisor E3 independiente del repo `MatiGaete2023/flow`. Revisas; NO implementas en la rama de trabajo: reportas hallazgos
con evidencia (archivo:línea, entrada ficticia, obtenido vs esperado) y, a lo sumo, propones parche y prueba que falle hoy. No
marques nada como «aceptado» ni «institucional». Español de Chile, directo; separa N1 (hechos), N2 (inferencias con supuesto),
N3 (especulación).

## ALCANCE
Rama `flow-python`, punta `c0bb644` o posterior (anota el SHA). Base: `c31d5f2` (última punta con revisión independiente, la
tercera E3). La cuarta E3 (`E3D`) la terminó la sesión implementadora: **no cuenta como independiente** y entra aquí.
Lee primero: `AGENTS.md`, `PLAN_VIGENTE.md` (decisiones v, w, x), `docs/python_unico/PLAN.md`, `docs/python_unico/REGISTRO.md`,
`docs/integracion_20261118/REGISTRO.md` (tablas E3C y E3D), `contracts/csmp_capture_v1/HISTORIA_VENTANA.md`.

### A. P3c — Ingreso Histórico Protección automático (prioridad; toca la extensión)
- `descargador/extension/pagina.js` comando `historico_open`: solo acepta `{tribunal, rut}`; el RUT debe venir de un enlace
  `ShowHistoriaRut(...)` de la tabla del listado vigente; GET a `PopUpHistorial.jsp?tipo_popUp=77&RUT_Litigante=…&RUT_DV=…`;
  luego sigue **un único** iframe del mismo origen con ruta exacta `/SITFAWEB/IrPopUpPublicoAccion.do` y devuelve su contenido.
  Busca: RUT que no esté en el listado y aun así se consulte; parseo de argumentos (comillas, puntos, guion, K, ceros a la
  izquierda); marcos relativos, `&amp;`, `//host/…`, `javascript:`, data URLs; redirecciones; tamaño y tipo de contenido;
  códigos de error; regresiones en `history_open`, `diary_open`, `calendar_open`. Protocolo `historico_lectura` en extensión y
  `contrato_integral.PROTOCOLS` (verificación estricta de igualdad).
- `descargador/lectura_bitacoras.py::_leer_historico`: una vez por persona y lote; pertenencia por el «Rut :» visible
  (`historico_manual.capturar`); falla aislada (bitácora sigue LEIDA); suspensión tras 5 fallas seguidas; tope de lecturas;
  clave `rut-<huella>` (`persona_por_rut`) para filas sin `SITFA_PERSONA_RUS` y su uso en `export_csmp_capture._persona_por_rut`.
  ¿Puede atribuirse el histórico de una persona a otra (hermanos en la misma causa, RUT con/sin ceros, dos tribunales)?
- `extraer_estructura_escritos.estructura_ventana` / `estructura_pestana`: confirma que nunca emiten valores de personas
  (nombres sin dígitos, programas, causas). Intenta romperlo con tablas sin `th`, celdas largas, títulos con datos.

### B. Cambios riesgosos desde `c31d5f2` (clase E3 por la decisión j)
- Motor y captura (E3C/E3D): `capture/escritos.py` (`_compatibles`, `_mejores_consultas`, `_filas_empate`, multiconjunto),
  `motor/reglas_historia.py` (`ESCRITOS_DISCORDANTES`), `motor/evidencia.py` (sujeto con partículas, NFC), `escritos_html.py`
  (titular por rótulo completo), `anonymize_package.py` (catálogo en NFC), `contract.py` (años 1900–2199). Error caro: retirar
  un pide cuenta que corresponde o afirmar «otro programa vigente» falso.
- Paquete: `nurus/bitacora_html.alcance_bitacora` (detector de recorte) y `filters_json.revision_bitacoras` (aditivo);
  `person_id` derivado del RUT en `registros`. ¿Rompe lectores o el anonimizador (`person_id`, `filters_json`)?
- Tope de aperturas 5000 (`MAX_LECTURAS_POR_LOTE`, `MAX_HISTORIAS_POR_LOTE`).

### C. Resto (solo regresiones; clase V)
P1 consignar (`work.py`, `record_view.py`), P2 revisión de bitácoras (`revision_bitacoras.py`, `revision_view.py`), P3a columna
Escritos, P4 biblioteca y plantilla Meta 4, P5 resumen y confirmación, P8 historial de `proceso.json` (¿puede filtrar rutas o
datos personales?).

## PROHIBICIONES
Sin --force, sin merge a main, sin releases, sin PR hacia feat/**. Sin HAR, HTML, PDF, Excel ni capturas reales: solo fixtures
ficticios o extractos anónimos. Sin credenciales, cookies ni tokens. Outlook solo `Save()`, nunca `.Send(`; CC fijo
`ucc_concepcion@pjud.cl`. Sin escritura en RUS/SATURNO. No ejecutes SITFA real.

## PREPARACIÓN
```
git fetch origin flow-python
git checkout -B e3/flow-python-$(date +%Y%m%d) origin/flow-python
pip install pandas openpyxl python-docx lxml docxcompose customtkinter icalendar pytest jsonschema reportlab Pillow ruff
git worktree add /tmp/base c31d5f2
```

## CONTROLES (reporta números)
1. `PYTHONPATH=src python -m pytest -q -p no:cacheprovider tests --continue-on-collection-errors`: compara identificadores de
   fallos con `/tmp/base` en el mismo entorno (sin Tk: `docs/integracion_20261118/base_python_fallos.txt`).
2. Descargador: `cd descargador && python -m pytest -q` (incluye `test_extension_historico.py`, que necesita Node).
3. Meta 4: `cd META4_Bitacoras && python -m pytest -q tests`.
4. `ruff check src descargador tools tests META4_Bitacoras` vs `/tmp/base`: 0 añadidos en archivos tocados.
5. `node --check descargador/extension/pagina.js`; `grep -rn "\.Send(" src descargador`; `python tools/check_real_ids.py`;
   `python tools/check_links.py`.
6. Revierte cada corrección de A y B de a una y confirma que alguna prueba cae; señala las que sobreviven.

## INFORME
`docs/python_unico/E3_INFORME_P3C_<fecha>.md` en tu rama `e3/…`:
1. SHA, entorno (SO, Python, Node, Tk).
2. Veredicto por bloque A, B, C: PASA / PASA CON OBSERVACIONES / NO PASA (nunca «aceptado»).
3. Hallazgos: `E3F-nn | severidad (crítica: atribuye a otra persona, retira pide cuenta, afirma hecho falso, filtra datos o
   bloquea entrega; alta; media; baja) | archivo:línea | entrada → obtenido vs esperado | parche`.
4. Controles 1–6 con números; mutaciones que sobreviven; qué no pudiste verificar (SITFA real, Tk, Windows).
