# Avance de implementación del plan de ajustes (19-oct-2026)

Rama `claude/exciting-gates-5xq24d`, construida sobre `feat/ejecucion-plan-20261008` @ `0ff63a1`.
`python tools/verificar_wp.py`: **PASA** (Asistente 984, Descargador 283, paridad golden, restricciones,
Ruff, identidades reales 0). Sin fusión a `feat` ni a `main`.

| Paquete | Estado | Qué hace |
|---|---|---|
| WA-1 | Hecho | «Prox. Auds.» con varias fechas (RUS V11): T-03 plural aprobado; fecha ilegible = incidencia T-03 |
| WA-3 | Hecho | Q-03: mismo ingreso en Espera, Cumplimiento o Egresados del mismo lote |
| WA-7 | Hecho | DCE: pide cuenta pasados 10 días del vencimiento (40 desde el ingreso); entretanto I-02 DCE con fecha límite |
| WA-2 | Hecho | Avisos de bitácora CT-07 (repite la última), CT-11 (correo E-05 hace ≥30 días), CT-05 (sin respuesta ≥30 días) |
| WD-4 | Cubierto | El Descargador ya incluye «Búsqueda Lit» en el paquete (registros OTRO, pantalla litigantes) |
| WA-5 | Hecho | Órdenes de búsqueda OB-01 (60 días) a OB-05, solo mostrar, sin cruces |
| WA-4 | Hecho | N-01 mayoría, N-02 informe tras audiencia (30 días), N-03 varios ingresos, N-04 egresado con informe |
| WD-2 | Pendiente de decisión | Usar la solicitud `CSMP_REQUEST` del Asistente para leer bitácoras solo de esos ingresos |
| WD-3 | Propuesto aplazar | El diccionario de códigos perdió su principal uso al retirarse B-04 |
| WD-1 / WA-6 | Bloqueado | Faltan los nombres de los parámetros de la dirección del calendario |
| WA-8 | Después | Historia de causa (H-01, H-02 acotado) |

Cambios que tocan zonas E3 (decisión j): motor (`reglas_*`, `utilidades`), `textos_base.json` y
`textos_observaciones.json` (clave nueva `COMUN.PROX_AUDIENCIAS`), referencia golden (solo esa clave) y el catálogo
de contexto (`CT-05`, `CT-07` activos solo mostrar, `CT-11` nueva). Requieren revisión E3 antes de integrar.

## Evidencia nueva (19-oct, tarde)

**Explorador V1.3** (`ses_d76ba366…`, 71 eventos, hashes íntegros, 0 acciones de escritura): registra rutas conocidas,
códigos de despacho (`TipMenuINF` 89/91), tipos de control, `tablaEgresados` y nombres de función de página. Confirma
(HECHO) que `ShowInformesProgramados` abre `IrPopUpInformesAccion.do` en una ventana reutilizada, con formulario
`InformesPpalForm` y campos ocultos `COD_Tribunal, CRR_IdCausa, ID_Ingreso, TIP_Consulta, COD_Centro, RIT_Causa,
GLS_Nombre, GLS_Centro, FLG_MejorNinez`. `ShowExcel` abre `/sitfa/reportes/*.xls` con MIME Excel.

**Dirección del calendario** (solo nombres de parámetros; los valores reales no se versionan):
`IrPopUpInformesAccion.do?tipo_popUp=8&ID_Ingreso&TIP_Programa&CRR_IdCausa&CRR_IdParte&COD_Tribunal&COD_Centro&
TIP_Consulta&GLS_Rit&FEC_IngEfectivo&GLS_Centro&GLS_Nombre&REM_Informe&PLZ_Intervencion&ID_Ingreso&FLG_MejorNinez`
(`ID_Ingreso` aparece dos veces; `REM_Informe` y `PLZ_Intervencion` llevan texto «N Meses»).

Correspondencia con los 12 argumentos de `ShowInformesProgramados` (`vinculos.py` valida 0, 2 y 10):
- HECHO: 0 = `CRR_IdCausa`, 1 = `CRR_IdParte`, 2 = `COD_Tribunal`, 3 = `COD_Centro`, 10 = `ID_Ingreso`.
- INFERENCIA: 4 = `GLS_Rit`, 5 = `FEC_IngEfectivo`, 6 = `GLS_Centro`, 7 = `GLS_Nombre`.
- NO DETERMINADO: 8, 9 y 11 (¿plazo, remisión, `FLG_MejorNinez`?), y el origen de `TIP_Programa`.

Para WD-1 falta la forma del `onclick` de la lupa «Inf.».
