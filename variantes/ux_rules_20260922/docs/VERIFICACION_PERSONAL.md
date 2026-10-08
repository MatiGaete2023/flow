# Verificación vigente — CSMP Assistant personal 0.4.0.dev11

Estado: **CANDIDATA FUNCIONAL CONGELADA** para uso cotidiano controlado. No se agregan nuevas funciones durante esta fase; solo se corrigen defectos reproducibles o fricciones observadas en trabajo real.

## Evidencia automatizada

Checkpoint UX dev11: `291c3929fbda63a336a1b04f6d402a7eb84236d0`, GitHub Actions run `35733322467`, Windows Python 3.12/3.13/3.14 en success.

Hotfix Excel RES: `b6c9d84b79b416dd7fc5fb9911a9ffe521934ac9`, GitHub Actions run `35737408005`, Windows Python 3.12/3.13/3.14 en success, **260 passed, 1 skipped** por versión. Python 3.12 aprobó además smoke CustomTkinter y construcción de la distribución Windows dev11.

El hotfix elimina la dependencia de `Application.International` al configurar la lista RES nativa de Excel; usa una hoja técnica oculta y el nombre `NURUS_RES_TIPOS`. La regresión cubre expresamente el escenario COM que produjo `'tuple' object is not callable` en Excel real.

## Evidencia de uso real — 22 de septiembre de 2026

El usuario confirmó funcionamiento satisfactorio de:
- procesamiento y modificación del Excel;
- desplegable y clasificación `RES`;
- generación de proyectos de resolución;
- creación y edición de borradores de correo;
- modificación de parámetros/configuración.

Esta prueba ocurrió después del hotfix RES. Por ello dev11 pasa de prototipo abierto a **candidata funcional congelada**.

## Límites

No se considera todavía versión 1.0 estable. Continúan pendientes los casos específicos no confirmados en la prueba real y registrados en `ACEPTACION_CSMP_PERSONAL.md`, la decisión DCE, matrices Tomé y una breve etapa de uso cotidiano para detectar fricciones. El producto no envía correos y no escribe en RUS/SATURNO.
