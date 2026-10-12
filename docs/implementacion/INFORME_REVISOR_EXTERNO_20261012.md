# Informe de avance y estado para revisión externa

12 de octubre de 2026 · Repositorio `MatiGaete2023/flow` · rama canónica `feat/ejecucion-plan-20261008` en `b63070d`.
Este informe orienta la revisión; las fuentes de verdad son los archivos que cita. Niveles: **N1** = verificable ahora
en el repositorio o en la CI; **N2** = inferencia con el supuesto indicado.

## 1. Qué es el proyecto

Dos aplicaciones de escritorio Windows (Python 3.12+, Tkinter) para un funcionario judicial que revisa causas.

| Producto | Función | No hace (por diseño) |
| --- | --- | --- |
| **Descargador SITFA** (`descargador/`, v2.7.6) | Con la sesión del usuario en Chrome (extensión propia en `descargador/extension/`), consulta SITFA, descarga los Excel originales y las bitácoras, y arma un paquete ZIP (contrato `CSMP_CAPTURE` 1.0.0/1.1.0) | No escribe en SITFA, RUS ni SATURNO |
| **Asistente CSMP** (`src/nurus/`, v0.5.0.dev5) | Importa el paquete o un Excel y aplica las reglas tradicionales (`src/nurus/personal/motor/`). Prepara la copia Excel de trabajo, nóminas, resoluciones Word y borradores de Outlook | No envía correos: solo `mail.Save()`, con CC fijo `ucc_concepcion@pjud.cl`. No registra en RUS, no navega SITFA ni inicia el Descargador |

Flujo de uso: Descargador «Consultar y crear paquete» → Asistente: Trabajo → Correos → Resoluciones → Resultados. Un
lanzador único ofrece los dos modos (`tools/csmp_launcher.py`) y el instalador Inno incluye ambos
(`installer/CSMP.iss`).

## 2. Objetivos y su cumplimiento

Fuente de los objetivos: `PLAN_VIGENTE.md` (esencia, prohibiciones, prioridades, decisiones D1–D9 y a–j) y
`docs/implementacion/PLAN_IMPLEMENTACION.md` («Criterio de término y métricas»).

| Objetivo / métrica | Estado | Evidencia |
| --- | --- | --- |
| Restricciones absolutas: sin escritura RUS/SATURNO, sin `.Send(`, CC fijo, sin credenciales | Cumple (N1) | Control 5 de `tools/verificar_wp.py`; `tests/test_outlook_safety.py` |
| Sin datos reales en el repositorio, historial ni CI | Cumple en árbol e historial (N1); pendiente la limpieza externa de GitHub (WP-17) | `tools/check_real_ids.py` y `--historia`: 0 hallazgos |
| Reglas, umbrales y textos tradicionales intactos | Cumple (N1) | Paridad de 17 productos golden: PASA; `motor/` y `textos_base.json` sin cambios desde `286c28f` |
| Retención local de 60 días y 10 sesiones (D4) | Implementado (N1) | `tests/test_retencion_sesion.py` y WP-13/14 |
| Casos R-T con prueba (32) | 32 de 32; 6 en xfail estricto por un conflicto con la especificación, pendiente de decisión del usuario | `estado/global.json` → `conflictos_abiertos` |
| Módulos no alcanzables y enlaces rotos | 0 y 0 (N1) | Controles 7 y 8 de `verificar_wp.py` |
| Actas de uso real firmadas (puerta única) | **0 de 2 o más: no cumple** | `docs/aceptacion/ACTA_USO_REAL_20261012.json` (plantilla: 40 casos NO_EJECUTADO) |
| Eficiencia y UX (ola 6: UX-01..03 y EF-01..04) | UX y EF-01..03 integrados; EF-04 en rama, pendiente de prueba real | `docs/implementacion/PLAN_EFICIENCIA_UX_20261012.md` |

**Conclusión (N2, supuesto: las métricas del plan reflejan el objetivo).** El software está funcionalmente completo y
verificado con datos ficticios. El proyecto **no está cerrado**: le falta la aceptación con uso real, que solo puede
dar el usuario.

## 3. Estado de los paquetes de trabajo (N1)

`docs/implementacion/estado/*.json` (vista generada: `ESTADO.json`; resumen generado en el bloque de `CONTINUAR.md`):
48 paquetes, de los cuales **44 están INTEGRADOS**.

| WP | Estado | Bloqueo |
| --- | --- | --- |
| WP-02 Acta de uso real | BLOQUEADO_USUARIO | Pruebas reales con Excel, Outlook y SITFA en el PC autorizado |
| WP-17 Purga de historial | EN_VERIFICACION | Soporte GitHub debe retirar `refs/pull` y las cachés |
| WP-50 Cronología de bitácoras | BLOQUEADO_USUARIO | Código integrado y visible; falta el contraste de 10 ingresos |
| WP-70 Cierre | BLOQUEADO_USUARIO | Depende de WP-02, WP-17, WP-50 y WP-63, más el acta final |
| WP-93 EF-04 (rama `wp/93-ef04`) | BLOQUEADO_USUARIO | E3 con CAMBIOS, aplicados; falta la prueba en SITFA real |

Integraciones recientes (12 de octubre):
- WP-86: correcciones de la revisión integral.
- WP-87/88: tema común, flujo guiado, lanzador y paquete en un paso.
- WP-89: CI eficiente.
- WP-90: continuación generada y reutilización de mediciones.
- WP-91: benchmark y edición 245× más rápida.
- WP-92: lotes de Outlook de 123,8 s a 4,3 s, tras una revisión E3 con CAMBIOS que se aplicaron.

## 4. Verificación vigente (N1)

| Evidencia | Resultado |
| --- | --- |
| `verificar_wp.py --linea-base` sobre `c752c6b` (`linea_base.json`) | Asistente 902 aprobadas, 11 omitidas y 6 xfail; Descargador 246 aprobadas y 62 subpruebas; 9 controles PASA |
| `verificar_wp.py` sobre la rama WP-93 (`3a421f4`) | Asistente 902/11/6; Descargador 263/62 |
| CI Windows completa, `workflow_dispatch`, run [38282508134](https://github.com/MatiGaete2023/flow/actions/runs/38282508134), sobre `04095e7` (= `feat` + WP-93) | **PASA** en Python 3.12, 3.13 y 3.14: EXE de ambos productos, instalación/desinstalación e instalador `CSMP-Instalador-Windows-0.5.0.dev5` (expira el 11-nov-2026) |
| Run anterior 38278082379 | FALLA por una prueba de foco intermitente (`test_ui_theme`), ajena a WP-92/93; se estabilizó en `b63070d` |

Con `[skip ci]` en commits solo documentales, la CI rápida de Linux no corrió sobre algunos commits de código
(WP-90). Su evidencia es la medición local completa.

## 5. Calidad del código: puntos a revisar

Tamaño (N1): `src/nurus` tiene 101 módulos y unas 18 600 líneas; `descargador/`, 65 módulos y unas 14 500;
`tests/`, 89 archivos; `tools/`, 29 scripts; 819 archivos versionados en total.

Riesgos y deuda conocidos:

1. **Reglas en conflicto:** 6 casos R-T en xfail estricto (`estado/global.json` → `conflictos_abiertos`). Cuál
   documento manda lo decide el usuario.
2. **Advertencia de pandas/NumPy** en `motor/utilidades.py:304`, sobre timedelta genérico. En una versión futura
   será un error. `motor/` está protegido, por decisión del usuario.
3. **Relectura en la exportación:** cerca del 40 % del tiempo de exportar se conserva a propósito, porque garantiza
   la sincronía con Excel (EF-02b, ficha WP-91).
4. **Diario de recibos (WP-92):** cambió la persistencia de Outlook. La revisó E3 y está cubierto por 12 pruebas,
   pero sin prueba real con Outlook.
5. **EF-04 (WP-93):** reutiliza bitácoras del mismo día y de menos de 12 horas, con hash comprobado. El riesgo
   residual es que no aparezcan actuaciones registradas después de esa lectura; se mitiga con la casilla que obliga a
   volver a leer.
6. **Pruebas de interfaz con foco:** dependían del gestor de ventanas; hubo una intermitencia corregida en
   `b63070d`. Puede haber otras parecidas.
7. **Documentación extensa con historia:** muchos informes fechados. Los vigentes los fija el índice
   `docs/INDICE_DOCUMENTACION.md`, y el estado vive solo en los JSON.

## 6. Proceso y gobernanza (N1)

- Paquetes de trabajo con ficha (`docs/implementacion/wp/`), estado JSON y verificación estándar (`verificar_wp.py`:
  suites, inventario sin bajas, identidades, golden, restricciones, ruff, módulos y enlaces).
- E3, revisión por otra raíz de agente: obligatoria solo en zonas riesgosas desde la decisión j (`PLAN_VIGENTE.md`).
  Los informes están en `docs/implementacion/auditoria/`.
- Sin `--force`, sin `main` y sin releases hasta el acta final (D7). `main` no recibió fusiones; el PR #1 sigue en
  borrador.
- Excepciones del proceso, registradas en `estado/global.json` y `PLAN_VIGENTE.md`:
  - E3 exceptuada por el usuario en varios PR del 11 de octubre;
  - consolidación de 45 ramas;
  - integraciones por fast-forward sin PR (WP-86, WP-90, WP-91 y WP-92).

## 7. Cómo revisar

```
git clone --no-tags --single-branch --branch feat/ejecucion-plan-20261008 https://github.com/MatiGaete2023/flow.git
python tools/estado.py validar && python tools/check_real_ids.py && python tools/check_links.py
python tools/verificar_wp.py          # suites completas, ~3 min; GUI con xvfb-run en Linux
python tools/benchmark_ef02.py --filas 200 1000   # rendimiento con datos ficticios
```

Orden de lectura sugerido:
1. `AGENTS.md`;
2. `PLAN_VIGENTE.md`;
3. `CONTINUAR.md`;
4. este informe;
5. `docs/implementacion/TRASPASO_PROGRAMADOR_20261012.md`;
6. las fichas WP-86 a WP-93 y sus auditorías;
7. los manuales `docs/MANUAL_ASISTENTE_MVP_20261008.md` y `docs/MANUAL_DESCARGADOR_MVP_20261008.md`.

El entorno se instala con `requirements/*.lock` (`pip install --require-hashes`).

**Restricción para el revisor:** no usar datos reales (HAR, HTML, PDF, Excel ni capturas) ni credenciales. Para
evidencia de funcionamiento real, pedir al usuario resultados agregados o el extracto anónimo
(`descargador/extraer_estructura_har.py`, `tools/anonimizar_excel.py`).

## 8. Qué falta para cerrar

| # | Tarea | Responsable |
| --- | --- | --- |
| 1 | Prueba de EF-04 en SITFA real (ficha WP-93) y luego integrar WP-93 | Usuario, luego programador |
| 2 | Uso real de Excel, Outlook y SITFA con el instalador de la tabla 4; completar el acta (WP-02, D2) | Usuario |
| 3 | Contraste de 10 ingresos de la cronología (WP-50) | Usuario |
| 4 | Retiro de `refs/pull` y cachés por Soporte GitHub (WP-17) | Titular de la cuenta |
| 5 | Decisión regla por regla sobre los 6 R-T y la advertencia de `motor/` (opcional) | Usuario |
| 6 | Acta final (WP-70) y después PR #1 a `main`, sin release automática | Usuario y orquestador |

N2: ya no queda trabajo de programación que bloquee el cierre. El plazo depende de cuándo se hagan las pruebas
reales y de Soporte GitHub; no hay datos para estimarlo.
