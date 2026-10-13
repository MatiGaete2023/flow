# Pruebas y aceptación — historia de la causa

## 1. Pruebas automáticas (todas con datos ficticios)

| Área | Casos mínimos | Dónde |
|---|---|---|
| Extractor H0 | ventana, listado, HAR con redirección; canarios no aparecen; fuga ⇒ no escribe; no sobrescribe | `descargador/test_extraer_estructura_historia.py` (**ya existe**) |
| Parser | 0/1/≥30 filas; texto largo; `=` inicial; «Sin referencia.»; fecha ilegible; anexo; encabezado distinto; login; acceso denegado; vacío explícito; cp1252/utf-8 | `tests/test_historia_html.py` |
| Vínculo | fila solo con historia; dos enlaces; tribunal/RIT discordante; Espera y Cumplimiento | `descargador/test_vinculos.py` |
| Extensión | ruta exacta; payload exacto; host/ruta/parámetro/redirección/tamaño inválidos ⇒ error sin `fetch`; sin `document.cookie`; capacidades 2.8.0 | `descargador/test_historia_extension.cjs` |
| Runner | dedupe por causa; `NO_DISPONIBLE`; `FALLIDA` por login/digest; tope; cancelar; reanudar; ventana; sin `a1` en salidas | `descargador/test_lectura_historia.py` |
| Bitácoras | suite completa sin cambios tras el gancho `procesar_fila` | existentes |
| Excel | hojas/columnas/orden exactos; literal seguro; partición de textos; hash alterado ⇒ error | `tests/test_historia_export.py` |
| Retención | `historias.json` antiguo elegible; reciente protegido | `descargador/test_retencion_lotes.py` |
| Contrato | ida y vuelta con extensión aditiva; validación de `annex_visible`/`raw_json`; fixtures `completa/parcial` válidos | `tests/test_capture_contract.py` |
| Asistente | paquete con historia ⇒ mismo trabajo que sin historia; golden PASA | `tests/test_capture_input.py`, `verificar_wp.py` |
| Flujo | historia apagada ⇒ paquete igual; historia fallida ⇒ paquete creado con estado declarado | `descargador/test_flujo_paquete.py` |
| Identidades | `check_real_ids.py` 0 hallazgos | `verificar_wp.py` |

## 2. Prueba real del usuario (PC autorizado, solo conteos y PASA/FALLA)

Usar un alcance pequeño: un tribunal, Cumplimiento + Espera, período habitual. No copiar nombres, RIT, folios ni textos.

| Código | Operación | Resultado esperado | Anotar |
|---|---|---|---|
| H-01 | Actualizar extensión a 2.8.0 y abrir el Descargador | Casilla de historia habilitada | PASA/FALLA |
| H-02 | Con la casilla **desmarcada**, «Consultar y crear paquete» | Igual que antes: mismo ZIP, sin `Historia_SITFA.xlsx` | PASA/FALLA |
| H-03 | Marcar historia; iniciar | Aviso «N registros → M causas» antes de abrir historias | N, M |
| H-04 | Dejar terminar | Progreso por causa; termina con resumen | Conteos por estado |
| H-05 | Abrir `Historia_SITFA.xlsx` | Hojas Trámites, Causas, Vínculos, Control | PASA/FALLA |
| H-06 | Elegir 5 causas (2 de Espera) y comparar con la ventana SITFA | Mismos folios, fechas, trámites, descripciones y referencias **dentro de los 4 meses**; nada fuera | Coincidencias x/5 |
| H-07 | En esas 5, revisar «Trámites anteriores» | Conteo coherente con lo que se ve antes de la fecha «Desde» | PASA/FALLA |
| H-08 | Causa con varios ingresos del lote | 1 sola causa en «Causas»; varios en «Vínculos»; trámites no repetidos | PASA/FALLA |
| H-09 | Cancelar a mitad y «Reintentar historias fallidas…» | Conserva lo leído; no repite causas leídas | PASA/FALLA |
| H-10 | Confirmar que no hubo descargas de PDF (carpeta Descargas y carpeta del lote) | Cero PDF de trámites | PASA/FALLA |
| H-11 | Importar el ZIP en el Asistente | Importa sin error; observaciones y productos iguales a un paquete sin historia | PASA/FALLA |
| H-12 | Cerrar sesión SITFA a mitad de lectura (opcional) | Causas restantes `FALLIDA` con motivo de sesión; nada aparece «vacío» | PASA/FALLA |

## 3. Plantilla de acta (`docs/aceptacion/ACTA_HISTORIA_<fecha>.json`)

```json
{
  "tipo": "ACTA_HISTORIA_CAUSA",
  "fecha": "",
  "version_descargador": "2.8.0",
  "commit": "",
  "alcance": {"tribunales": 0, "modalidades": [], "registros": 0, "causas": 0},
  "resultados": {"H-01": "", "H-02": "", "H-03": "", "H-04": "", "H-05": "", "H-06": "", "H-07": "", "H-08": "", "H-09": "", "H-10": "", "H-11": "", "H-12": ""},
  "conteos": {"LEIDA": 0, "FALLIDA": 0, "NO_DISPONIBLE": 0, "IDENTIDAD_INCIERTA": 0, "tramites_en_ventana": 0},
  "observaciones": "",
  "firmado_por": "",
  "decision": ""
}
```

Sin datos de causas. El acta la firma solo el usuario; ninguna prueba automática la reemplaza.
