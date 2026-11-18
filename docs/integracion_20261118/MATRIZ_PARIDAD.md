# Matriz de aceptación del plan (paso 9) — Python | Rust

N1: cada fila cita la prueba que la ejerce. Rust se compara literal por literal con el motor Python mediante el oráculo
(`rust/casos/integracion.json`, 929 casos al azar + dirigidos `INT-*-Dnn`) y el paquete ZIP 1.2.0 (`paquete_integracion`).

| # | Escenario del plan | Python | Rust | Estado |
|---|---|---|---|---|
| 1 | AFT/PRM/PDE/PDC/PIE, FAE/FAS y residencial con tipo correcto + Pendiente; tipo/estado distinto no activa | `test_escrito_pendiente_del_tipo_correcto…` (parametrizado), `test_escrito_de_otro_tipo_o_estado…`, `test_tipo_correcto_en_una_fila…` | oráculo `INT-INF-D16…D21` | CUBIERTO |
| 2 | Un ingreso, tres fechas: pendiente / incorporado sin nomenclatura / exigible | `test_ingreso_con_tres_fechas…`, `test_productos_solo_incluyen…`, `test_proyectos_word_no_arrastran…` | `INT-INF-D26…D28`, `paquete_con_fuentes_nuevas_igual_que_python` (r2/r3) | CUBIERTO (asociación explícita sintética) |
| 3 | Cinco fechas, dos acreditadas, fila duplicada | `test_cinco_fechas_dos_acreditadas…`, `test_estructura_16_…` | `paquete_de_estructura_…` | CUBIERTO |
| 4 | Nomenclatura presente y calendario pendiente → revisar asociación | `test_nomenclatura_presente_y_calendario_pendiente…`, `test_i1_contradice…` | `INT-INF-D24` | CUBIERTO |
| 5 | Dos programas/NNA en una causa; dos ingresos sucesivos al mismo programa | `test_estructura_16_vencimientos…` (ingresos 1-2 y 3-4) | `paquete_de_estructura_16_vencimientos_10_ingresos_igual_que_python` | CUBIERTO (corrige atribución por historia entre ingresos sucesivos) |
| 6 | Programa actual A, otro B vigente repetido, C con egreso; fecha sola, causal sola, actual repetido, dos otros | `test_otro_programa_*` (3 pruebas) | `INT-ESP-D01`, `INT-CUM-D02…D05` | CUBIERTO |
| 7 | Ya observado en Espera y revisión en Cumplimiento; borrador descartado; historia parcial | `test_otro_programa_se_consigna_una_vez…`, `test_constancia_previa…`, `test_marca_local_borrador_descartado_y_bitacora_parcial`, `test_consignar_y_reimportar…` | `INT-CUM-D06…D10` | CUBIERTO |
| 8 | Excel antiguo / paquete sin Escritos ni Histórico; vacío acreditado, no disponible, fallo | `test_paquete_sin_las_fuentes_nuevas…`, `test_bandeja_sin_filas_utilizables…`, `test_historico_no_verificable…`, `test_pestana_presente_pero_ilegible…` (Descargador) | `INT-CUM-D11/D12`, `paquete_con_historia_igual_que_python` (legado) | CUBIERTO |
| 9 | Cambio de corte, selección oculta, reintento tras cancelación, producto ya editado | `test_cambio_de_corte_…`, `test_recalcular_tras_una_edicion…` | — | PARCIAL (faltan selección oculta, reintento y producto editado) |
| 10 | Caso real reproducido ficticio: 16 vencimientos, 10 ingresos | `test_estructura_16_vencimientos…` | `paquete_de_estructura_…` | CUBIERTO (inventario: 13 pide cuenta, 1 nomenclatura, 2 resueltas) |

Sin pruebas de interfaz (no hay `tkinter` ni Windows en este contenedor).

**Rust → Python:** `paquete_rust_escritos.zip` (Descargador Rust, 1.2.0) lo lee el lector Python: `test_paquete_rust_con_escritos_lo_lee_el_lector_python`. Lectura de Escritos en HTML: `flow-captura/tests/lecturas.rs::escritos` ↔ `test_integracion_captura_20261118.py` (lectores).
