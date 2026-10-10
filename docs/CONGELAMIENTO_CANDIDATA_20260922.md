# Congelamiento de candidata funcional — 22 de septiembre de 2026

Producto: **CSMP Assistant personal 0.4.0.dev11**.

## Decisión

Dev11 queda congelada como **candidata funcional** después de una prueba real satisfactoria del flujo principal. Desde este punto no se agregan características por conveniencia o hipótesis. Solo se aceptan:

1. correcciones de errores reproducibles;
2. correcciones de seguridad/integridad;
3. fricciones observadas durante uso real que tengan impacto material;
4. ajustes documentales.

No se genera todavía ejecutable `.exe` y no se promueve a `1.0`.

## Evidencia

Automática:
- UX dev11: commit `291c3929fbda63a336a1b04f6d402a7eb84236d0`, run `35733322467`;
- hotfix RES/Excel: commit `b6c9d84b79b416dd7fc5fb9911a9ffe521934ac9`, run `35737408005`;
- hotfix: Windows Python 3.12/3.13/3.14 success; 260 passed, 1 skipped por versión; smoke GUI y construcción Windows en 3.12.

Real:
- procesamiento y modificación del Excel: OK;
- RES categórico: OK;
- creación de resoluciones: OK;
- creación y modificación de borradores de correo: OK;
- modificación de parámetros/configuración: OK.

## Política durante el congelamiento

- mantener `main` como línea vigente;
- usar esta candidata durante uno o dos ciclos normales de trabajo;
- registrar cualquier incidencia con archivo/operación/mensaje/resultado esperado;
- no reabrir arquitectura, motor genérico, dashboards, nube, multiusuario ni empaquetado EXE durante la observación;
- no alterar invariantes: solo borradores Outlook, sin escritura RUS/SATURNO, Excel original intacto y revisión humana.

## Criterio para pasar a la siguiente etapa

Tras uno o dos ciclos normales sin defectos materiales:
1. revisar incidencias y resolver únicamente las reproducibles;
2. cerrar o documentar los pendientes de dominio que corresponda mantener abiertos;
3. decidir promoción a candidata `1.0rc1`;
4. recién entonces preparar la distribución portable `.exe` y probarla en el PC institucional.

La rama estable `candidate-1.0-20260922` conserva este checkpoint documental y funcional.
