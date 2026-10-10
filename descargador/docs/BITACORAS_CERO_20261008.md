# Lote de bitácoras sin lecturas · corrección 2.5.1

**Guía vigente:** [paquete conjunto dev22 / 2.7.0](../../docs/INTEGRACION_COMPLETA_20261008.md). Los números anteriores se conservan como antecedentes.

## Evidencia y alcance

El usuario informó cero lecturas de Residencia/Cumplimiento en Tomé, Laja y
Mulchén. El manifiesto local contiene tres consultas fallidas, cero páginas
enumeradas y cero registros. No contiene copias HTML. El usuario confirmó que
Descargar el lote completo sí entrega Excel con la misma selección.

El estado no significa que las bitácoras estén vacías. Las consultas fallaron
antes de leer una fila. El mensaje genérico anterior descartó el tipo y la
ubicación de la excepción; esa evidencia no permite identificar la causa exacta.
La respuesta concreta de esas tres consultas no fue conservada.

## Cambios

- El resultado diferencia filas fallidas de consultas fallidas. Cero filas
  enumeradas ya no produce un falso contador de cero fallas.
- Si no se guardó ninguna copia, dice «No se guardaron copias de bitácoras».
- Cada consulta fallida incluye fase, clase de excepción y módulos/líneas del
  recorrido. No registra mensajes crudos de excepciones, variables, contenido
  de registros, cookies ni rutas completas del equipo.
- El manifiesto identifica versión y commit del ejecutable que lo generó.
- Se conserva UTF-8 para las respuestas habituales. Si los bytes no son UTF-8,
  la segunda lectura usa el mismo parser HTML que acepta la descarga normal;
  evita fallar antes de enumerar por una conversión UTF-8 obligatoria. Una prueba
  con ISO-8859-1 y tildes reproduce este defecto de compatibilidad. Es una causa
  posible, no una causa demostrada del lote real del usuario.
- La comprobación del paquete ejecuta listado → identidad → apertura ficticia
  → validación → copia → manifiesto. Antes solo comprobaba que el parser común
  estuviera disponible. Las fallas del chequeo generan JSON y salida de error.

## Uso de la actualización

1. Conserva la carpeta completa de Descargador 2.5.1. Es compatible con CSMP
   dev21 y el contrato de bitácoras versión 1.
2. Si lo abres desde CSMP, selecciona el EXE nuevo; comprueba 2.5.1 en su título.
3. Usa la extensión de lectura de bitácoras incluida. El código JavaScript de
   esta actualización conserva el de 2.5.0; una extensión 2.4 anterior carece
   del recorrido de apertura de bitácoras.
4. Conecta Chrome, carga Seguimiento y selecciona tribunales, modalidad y pestaña.
5. Reintentar lecturas fallidas permite elegir la carpeta del lote anterior;
   vuelve a enumerar los registros desde el inicio, sin inventar enlaces.
6. Si sigue fallando, revisa el nuevo `bitacoras.json`: `consultas[].diagnostico`
   indica dónde ocurre. El archivo anterior no puede aportar ese detalle.

## Comprobaciones y límite

Las pruebas incluyen las cuatro modalidades, paginación, cancelación y reintento,
identidades ambiguas, fallas antes de enumerar y mensajes de cero copias. Una
captura local anterior de Residencia permite reconocer tres registros y sus
enlaces. El recorrido ficticio también pasa dentro del EXE de Windows.

Estas pruebas no certifican el lote real que falló. La conexión de Codex a la
pestaña Chrome agotó su tiempo de respuesta; no acredita que RUS esté cerrado.
Es necesaria una nueva ejecución del lote con 2.5.1 para aceptar esta corrección
o conservar el diagnóstico exacto. La escritura de observaciones sigue aplazada.

Las capturas y resultados reales permanecen locales; se publica código, pruebas
ficticias y esta descripción de la incidencia.
