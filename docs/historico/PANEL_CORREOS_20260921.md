> Componente incorporado y vigente en **0.4.0.dev11**. Este documento describe específicamente el panel de Correos introducido en dev9.

# Panel personal y alcance de correos — CSMP Assistant 0.4.0.dev9

## Base recuperada

Se continúa desde `bab202e9533e88b5130a277f01b90b816e98940e`, validado en Windows por el run `35620647820`. Esta base ya contiene la limpieza de interfaz NuRus, la corrección de la prueba de desactivación de advertencias, retiro de dos APIs sin consumidores y carga diferida del backend histórico. No se reinició la migración ni se deshicieron esas correcciones.

## Cambios de uso

- La aplicación abre en modo oscuro con navegación lateral: Trabajo, Correos, Resoluciones, Configuración y Enviados. Formularios, botones y editores usan CustomTkinter; las tablas conservan ttk nativo con tema oscuro para mantener selección múltiple y teclado.
- En Correos, **Solo programas** es el alcance inicial. **Preparar todos** no agrega el informativo al tribunal en ese alcance. Se puede elegir **Solo tribunales** o **Ambos**. Una plantilla individual también puede dirigirse a programas.
- **Filtros / opciones** reúne tribunal de las causas, modalidades, período y selección manual. No seleccionar tribunal incluye todas las causas: este filtro no determina quién recibe el correo.
- Las tarjetas muestran programa, tribunal, cantidad y vencimiento más cercano de los registros, cuando existe. No se inventan comuna, fechas, prioridad ni revisión humana. Seleccionar una tarjeta carga Para, CC, asunto y cuerpo.
- Los adjuntos se muestran por nombre, con una × para quitar cada archivo. Las nóminas siguen usando el nombre del programa. Se conserva adjuntar a todos.
- **Guardar este** y **Guardar todos** crean borradores Outlook. Se mantiene Para vacío permitido, copia institucional obligatoria, prevención de duplicados y continuidad del lote ante un error individual.
- La barra de progreso se anima durante operaciones reales y se detiene al concluir o fallar. No afirma que un correo fue enviado.

## Texto del manual

Fuente: texto literal aportado por el usuario en esta revisión. La plantilla **Informes por vencer · programa** contiene:

> Buen día:
>
> Junto con saludar, se envía planilla con RIT de causas en las cuales, informes de avances se encuentran pronto a vencer. Se ruega acusar recibo de la información.

Se mantiene un cierre editable y la firma configurada. La revisión 3 migra el cuerpo antiguo conocido aunque la configuración tuviera revisión 2. Las modificaciones personales se conservan; las plantillas faltantes se reponen y se deja respaldo `.bak` al guardar.

## Instalación

Extraer en una carpeta nueva y ejecutar `Instalar_CSMP.bat`, después `Abrir_CSMP.bat`. Dependencia nueva: `customtkinter>=5.2.2,<6`, instalada con el resto en `.venv-csmp`, sin administrador. Una instalación offline debe incluirla con sus dependencias en `paquetes/`. Configuración, sesiones y matrices personales siguen en `LOCALAPPDATA/CSMP_Personal`.

## Verificación y límites

Las pruebas verifican alcance por destino, destinatario vacío, nombre de nómina, migración sin pérdida de personalizaciones y selección de causas sin tribunal obligatorio. `Preparar todos` separa semánticamente los tipos automáticos: `programa_*` se dirige a programas; el informativo general y `medidas` se dirigen a tribunales. La ventana real en Windows comprueba navegación, tarjetas, adjuntos, campos editables y geometría.

Cierre validado: commit `c262321e227302cc9349061a941c06af29879219`, GitHub Actions run `35646164874`, **success** en Python 3.12/3.13/3.14 con 254 pruebas aprobadas y 1 omitida por versión. Python 3.12 aprobó además el smoke visual y generó la distribución `CSMP-Windows-dev9`. Los detalles están en `VERIFICACION_PERSONAL.md`.

La documentación primaria usada para los controles es la de [CustomTkinter](https://customtkinter.tomschimansky.com/documentation/), incluidos [CTkTextbox](https://customtkinter.tomschimansky.com/documentation/widgets/textbox/) y [CTkScrollableFrame](https://customtkinter.tomschimansky.com/documentation/widgets/scrollableframe/).

La integración no sustituye la aceptación con Excel 2010 y Outlook institucional. Continúan pendientes DCE, matrices Tomé y medición real del ahorro de trabajo. No se habilita envío automático ni escritura en RUS/SATURNO.
