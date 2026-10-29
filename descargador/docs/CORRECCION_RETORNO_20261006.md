# Descarga conjunta y retorno a CSMP - 2.4.4

El flujo descargaba la principal e informes de ambos meses, pero fallaba al abrir Carga. Como la devolución solo se publica al generar el libro final, CSMP nunca recibía un resultado.

La etapa de Carga se retiró del flujo conjunto por petición del usuario. Ahora incluye únicamente la principal completa y los informes por vencer del mes actual y siguiente. El libro conserva la procedencia y declara `NO_CONSULTADA` en cobertura de firmas, sin inventar una ausencia de movimientos. La tabla de firmas conserva solo su esquema para compatibilidad con CSMP dev18; no contiene resoluciones descargadas.

Los flujos antiguos mantienen su fecha de corte y el alcance. Al recuperar uno con las tres fases válidas, se comprueban los originales y se genera el libro sin consultas al navegador. Si alguna fase principal/de informes está incompleta, se mantienen las restricciones de recuperación. No se borran las fuentes anteriores ni las sesiones de CSMP.

El botón Descarga conjunta CSMP permanece; desaparece Días de firmas. El texto de inicio y resultados describe el flujo actual. No se alteró el protocolo `--csmp-reply`: CSMP dev18 puede recibirlo y analizarlo.

Los calendarios reales incluyen una columna final sin título ni datos. Se omiten solo los títulos vacíos al final; cualquier dato sin cabecera, título vacío interior o columna ambigua se sigue rechazando. Esta corrección permite consolidar octubre y noviembre sin alterar los XLS originales.

Una instalación configurada con un ejecutable anterior debe apuntar al SITFA_Descargador.exe 2.4.4. Extraer el nuevo paquete no modifica una ruta que CSMP ya tenía guardada.

La lectura/registro de bitácoras en vivo y las firmas independientes siguen sin una comprobación nueva en RUS. Esta corrección no las presenta como disponibles dentro del flujo conjunto.
