# Integración SITFA en rama experimental

Rama: **`experimento/integracion-sitfa-20261001`**, basada en `main` dev11.
El usuario solicitó implementar las mejoras en ramas separadas para comparar.
La candidata original de `main` permanece congelada; estos cambios no se fusionan
ni promocionan automáticamente.

Cambios:

- Formato Excel por firma: OOXML bajo `.xls` se reconoce; HTML/XML explica que
  debe convertirse con Download → Exportar para CSMP.
- Comprobar columnas muestra compatibilidad y antigüedad; PROCESAR no ejecuta un
  análisis sin columnas funcionales necesarias. Filas incompletas se advierten.
- Procedencia opcional `SITFA_*`: Abrir original comprueba ruta y hash; el mapa
  local se conserva al guardar/recuperar el trabajo.
- Precarga `--archivo`, `--modo`, `--hoja`, sin ejecutar reglas automáticamente.
- Configuración independiente `LOCALAPPDATA/CSMP_Personal_Experimental`.
- Receta PyInstaller/CI para `CSMP_Experimental.exe` y sus recursos completos.

Ejemplo con el entorno de esta rama:

```powershell
.venv-csmp\Scripts\pythonw.exe -m nurus.personal.app --archivo "C:\Resultados\Para CSMP.xlsx" --modo CUMPLIMIENTO --hoja CUMPLIMIENTO
```

No se cambia el motor de observaciones ni las reglas de revisión, RES, borradores
o proyectos. No se envía correo ni se escribe en RUS/SATURNO. La precarga de un
libro nuevo limpia los productos visibles de la sesión anterior, sin borrar su
recuperación de disco; el libro requiere PROCESAR y el flujo habitual de revisión.

Para C-10, Download puede reunir Cumplimiento e Informes completos del mismo
alcance. Se conserva la identidad RIT/RUT/nombre/tribunal/programa y la regla
vigente de última fila futura. No se resuelven diferencias de abreviatura por
suposición y la falta de un cruce utilizable continúa como aviso.

La guía y exportador están en la rama
[`experimento/experiencia-csmp-20261001` de Download](https://github.com/MatiGaete2023/Download/tree/experimento/experiencia-csmp-20261001).
Prueba offline: `pruebas/integracion_exportador_csmp.py` en ese repositorio,
ejecutada con dependencias de ambos en un entorno de desarrollo. Las pruebas
de esta rama incluyen reconocimiento de formato, columnas, fecha antigua,
procedencia, alteración del original, rutas y recuperación de sesión.

La CI Windows conserva el smoke de interfaz y pruebas de dev11 y añade el
ejecutable experimental como artefacto. La revisión Linux no sustituye la
comprobación del ejecutable ni del ciclo institucional con Excel/Outlook.
