"""Nombres de nomenclatura (Desc. Trámite) del manual y agrupaciones de las reglas de historia.

«Ingreso» es la resolución con que el Tribunal ordena que un NNA ingrese a un programa (la causa queda en Espera);
«Ingreso efectivo» es el trámite con que el programa informa que el NNA está en intervención (pasa a Cumplimiento).
"""

INGRESO_EFECTIVO = 'Ingreso efectivo'
PRORROGA = 'Prorroga'
INGRESO_INFORME_X = 'Ingreso informe de cumplimiento X'
ORDEN_BUSQUEDA = 'Orden de búsqueda'
PIDE_CUENTA_BUSQUEDA = 'Pide cuenta búsqueda'
TERMINO_BUSQUEDA = 'Termino de búsqueda'
SIN_EFECTO_BUSQUEDA = 'Deja sin efecto búsqueda'
SIN_EFECTO_MN = 'Deja sin efecto medida interconexión M.N.'
SIN_EFECTO_DR = 'Deja sin efecto derivación D.R.'
CONTINUIDAD = 'Resp. continuidad de atención'
SOLICITUD_DCE = 'SPE. Resp. Solicitud DCE (escrito)'
DERECHO_OIDO = 'Derecho a ser oído'
SERVICIO = 'Ing.Serv. protección especializada y otras redes'

CONCILIACION = 'Conciliacion'
DICTACION_SENTENCIA = 'Dictación de sentencia'
EGRESO_SERVICIO = 'Egreso de servicio protección especializada y otras redes'

# Sin regla ni marca (decisión x): Resp. Diagnóstico clínico especializado, Resp. Rectificación DCE (escrito),
# Derivación Dirección Reg., Pide cuenta Derivación D.R. y Resp. Derivación dir. regional.
CURADOR = ('Designa abogado', 'Deja sin efecto designación C. Ad-litem')
BUSQUEDA_SEGUIMIENTO = (PIDE_CUENTA_BUSQUEDA, SIN_EFECTO_BUSQUEDA, TERMINO_BUSQUEDA)
ESCRITOS = {
    'AMBULATORIO': 'Inf. Cump Programa Ambulatorio',
    'FAE': 'Inf. Cump FAE/FAS',
    'RESIDENCIAL': 'Inf. Cump Centro Residencial',
}
RESIDENCIA_PREFIJOS = ('rta', 'rtt', 'res', 'rfa', 'rva')

# Sin regla ni marca por decisión del usuario (decisión x): no cuentan como «trámites sin regla».
IGNORADAS = (
    'Resp. Diagnóstico clínico especializado',
    'Resp. Rectificación DCE (escrito)',
    'SPE. Resp. Rectificación DCE (escrito)',
    'Derivación Dirección Reg.',
    'Pide cuenta Derivación D.R.',
    'Resp. Derivación dir. regional',
)
CONOCIDAS = (
    INGRESO_EFECTIVO,
    PRORROGA,
    INGRESO_INFORME_X,
    ORDEN_BUSQUEDA,
    *BUSQUEDA_SEGUIMIENTO,
    SIN_EFECTO_MN,
    SIN_EFECTO_DR,
    CONTINUIDAD,
    SOLICITUD_DCE,
    DERECHO_OIDO,
    SERVICIO,
    CONCILIACION,
    DICTACION_SENTENCIA,
    EGRESO_SERVICIO,
    *CURADOR,
    *ESCRITOS.values(),
    *IGNORADAS,
)
# Último trámite anterior a la ventana que el Descargador conserva como contexto (11-nov-2026).
CONTEXTO = (
    ORDEN_BUSQUEDA,
    TERMINO_BUSQUEDA,
    SIN_EFECTO_BUSQUEDA,
    CONCILIACION,
    DICTACION_SENTENCIA,
    EGRESO_SERVICIO,
    *CURADOR,
    PRORROGA,
)
