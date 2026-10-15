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
BUSQUEDA_CITADA = (ORDEN_BUSQUEDA, PIDE_CUENTA_BUSQUEDA)
ESCRITOS = {
    'AMBULATORIO': 'Inf. Cump Programa Ambulatorio',
    'FAE': 'Inf. Cump FAE/FAS',
    'RESIDENCIAL': 'Inf. Cump Centro Residencial',
}
RESIDENCIA_PREFIJOS = ('rta', 'rtt', 'res', 'rfa', 'rva')
