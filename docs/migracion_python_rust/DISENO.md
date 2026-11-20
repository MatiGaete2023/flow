# Diseño de Flow Rust

El punto de partida es la aplicación de la carpeta local `flowrust/flow-flow-python/app/`. Las capturas originales están bajo `app/shots/` en esa carpeta. Las referencias de abajo se volvieron a generar en Windows desde el ejecutable de esa carpeta con el ejemplo ficticio y un perfil aislado. Para reproducirlas, seguir [Verificación](VERIFICACION.md).

| Vista | Referencia local | Qué comprobar |
|---|---|---|
| Trabajo | [Captura](../../app/docs/referencia_visual/trabajo.png) | Barra lateral oscura con Trabajo, Correos, Resoluciones, Resultados y Contactos. Barra superior con apertura, guardado, corte y etapa. Contadores y filtros sobre la tabla, con detalle y edición a la derecha. |
| Correos | [Captura](../../app/docs/referencia_visual/correos.png) | Selector de modalidades y tribunales arriba, tarjetas de borradores a la izquierda y destinatarios, texto, acciones y nómina a la derecha. |
| Resoluciones | [Captura](../../app/docs/referencia_visual/resoluciones.png) | Acciones de preparación y Word arriba, proyectos a la izquierda, campos editables y vista previa a la derecha. |
| Anonimizador y Ajustes | [Anonimizador](../../app/docs/referencia_visual/anonimizador.png) · [Ajustes en oscuro](../../app/docs/referencia_visual/ajustes-oscuro.png) | Misma navegación lateral, tarjetas coherentes, controles de entradas y salida. El cambio claro/oscuro permanece en la barra lateral. |

- `main.rs` configura la ventana de `1440 × 900` puntos con mínimo `1100 × 680`. `ui.rs` mantiene la navegación, diálogos y vistas. `theme.rs` define ambas paletas, tarjetas, chips y acciones principales.
- Integrar ZIP y bitácoras en Trabajo. Los productos y estados del nuevo flujo se presentan en las secciones que ya maneja el Asistente. Las herramientas nuevas toman el mismo tema y sus componentes. Los controles propios del Descargador se alojan en su herramienta.
- Reproducir las capturas con los mismos ejemplos de `demo.rs` y contrastar la jerarquía, navegación, tabla, paneles y formulación de acciones. Fecha, tiempo de análisis y el renderizado de fuentes pueden variar por entorno.
- El artefacto `Flow_Windows` del run [44648737252](https://github.com/MatiGaete2023/flow/actions/runs/44648737252) corresponde al workspace histórico y se conserva como evidencia de comportamiento de esa implementación. El binario de esta aplicación se compila desde `app/Cargo.toml`.
- Las [capturas verificadas de la compilación Windows](https://github.com/MatiGaete2023/flow/actions/runs/44748317049/artifacts/14634511876) muestran las cinco vistas del ejecutable actual. Están escaladas para el monitor virtual de CI; las referencias locales de la tabla se tomaron a 1440 × 900.
