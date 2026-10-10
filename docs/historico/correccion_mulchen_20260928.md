# Correos de Mulchén — 28 de septiembre de 2026

**Actualización del 29-09:** el fallo concreto ya se reprodujo al seleccionar
solo Residencial: faltaba reconocer RTA/RTT/RVA. Ver la
[corrección del filtro de modalidades](correccion_residencial_20260929.md).

## Hallazgo y alcance

El motor ya reconocía `Mulchén`, `MULCHEN`, `MULCHËN`, acentos Unicode
descompuestos y nombres completos o abreviados que contienen la comuna.
Sin embargo, `selected_court_record_ids` normalizaba las filas y no la selección:
seleccionar `Mulchén` o un nombre completo podía devolver cero registros.
La búsqueda posterior de destinatarios también exigía una clave literal
`MULCHEN` en la configuración. Una configuración con clave descriptiva podía
producir un borrador sin destinatarios.

Se incorpora `personal/courts.py` para compartir la identificación del tribunal
entre el filtro, la agrupación y la consulta de destinatarios. Las claves
estables conservan prioridad y se respetan las direcciones personalizadas.
Los alias contradictorios generan un error explícito; un tribunal desconocido
no recibe una dirección deducida.

## Verificación

- Regresiones con siete variantes del nombre, incluido orden distinto de las
  palabras, espacios finales, diéresis y acento descompuesto.
- Lectura de Excel, filtro de tribunal, preparación manual, preparación
  automática y «Preparar todos»: un borrador con los registros correspondientes
  y los destinatarios personalizados; Laja queda fuera de la selección Mulchén.
- Interfaz real de Windows: preparación asíncrona, tarjeta visible y campos
  de destinatarios/asunto cargados, con y sin selección manual de filas.
- Suite completa: **330 pruebas aprobadas, 1 omitida**. La prueba omitida verifica
  el rechazo del backend nativo fuera de Windows.
- No se guardaron ni enviaron correos reales para realizar estas pruebas.

## Límite del diagnóstico

La sesión local guardada también se comprobó en modo lectura antes del cambio:
el motor preparó correctamente un borrador con siete registros de Mulchén y
destinatarios, en modo automático y manual. Por tanto, la inconsistencia
corregida es reproducible, pero aún no permite atribuir a ella el fallo concreto
observado en pantalla por el usuario. Falta conocer el mensaje o el paso exacto
que falla si el problema persiste con esta versión.

Los cambios son de código fuente; no actualizan por sí solos una instalación o
ejecutable que el usuario ya tenga abierto.
