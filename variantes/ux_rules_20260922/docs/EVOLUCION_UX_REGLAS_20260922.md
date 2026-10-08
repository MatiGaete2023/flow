# Evolución UX + reglas aprobadas — 22 de septiembre de 2026

Versión de rama: **0.4.0.dev12**. Base: candidata funcional congelada dev11.

## Alcance

Esta evolución aplica únicamente cambios expresamente autorizados sobre la candidata dev11. No reescribe el motor, no introduce una arquitectura nueva y no modifica las invariantes de seguridad: solo borradores Outlook, sin escritura RUS/SATURNO, Excel original intacto y control humano.

## Reglas y redacciones

Se sincronizan `src/nurus/personal/textos_base.json` y `src/nurus/rus/textos_observaciones.json` con las redacciones aprobadas para mayoría de edad, curador, oído, audiencia, Espera, Cumplimiento e Informes.

En Cumplimiento se elimina el prefijo redundante `Medida revisada.` cuando ya existe un hallazgo concreto. `Medida revisada, sin observaciones.` y las fórmulas de Espera se conservan.

Los informes vencidos general y DCE mantienen la asociación estructurada ya existente a `PC_INFO`; los informes por vencer no generan `PC_INFO`. No se modifica el umbral sustantivo DCE que continúa documentado como conflicto abierto.

La configuración pasa a `revision_textos=3`. La migración reemplaza únicamente textos que coinciden exactamente con antiguos valores canónicos; las personalizaciones del usuario se conservan.

## UX acotada

- panel contextual de causa reutilizado en Trabajo, Correos y Resoluciones;
- indicadores compactos de Excel, correo, RES y Word;
- navegación Anterior / Siguiente incidencia usando únicamente warnings existentes;
- comparación motor/final de observaciones y correos solo cuando existe edición;
- aviso de proyecto de resolución modificado respecto de la matriz;
- acción primaria visual en Trabajo, Correos y Resoluciones;
- mensajes de éxito mantienen la barra inferior; errores y decisiones siguen siendo modales;
- Configuración distingue visualmente Básico y Avanzado sin eliminar ni endurecer valores;
- la persistencia ya existente se expone como “Último trabajo”, sin nueva base de datos ni reconstrucción de sesión;
- los códigos RES mantienen sus descripciones legibles existentes.

## Deliberadamente no implementado

No se agregan dashboard, gráficos, contadores laterales, SQLite nuevo, historial exhaustivo por causa, multiusuario, nube, API, IA, plugins, PySide6, Electron, C#, reescritura del motor, EXE, instalador ni diseñador universal de reglas.

## Verificación

CI de la rama candidata en SHA `79cf3b25e525c838510d88ac16bd52220b6f6d07`: GitHub Actions run `35772183073`, **success** en Windows con Python 3.12, 3.13 y 3.14. La suite registró **265 passed, 1 skipped** por versión. Python 3.12 aprobó además el smoke GUI y la construcción de la distribución ZIP dev12.

La primera ejecución de esta evolución detectó dos desalineaciones documentales: ausencia de la fecha literal en README y matriz documental de reglas no sincronizada con los nuevos textos. Ambas fueron corregidas antes de esta ejecución verde.

El resultado automático no acredita funcionamiento real con Excel/Outlook institucional ni la ergonomía final en el flujo cotidiano; esa validación queda a cargo del usuario antes de integrar a `main`.

La candidata dev11 congelada queda preservada como referencia previa.
