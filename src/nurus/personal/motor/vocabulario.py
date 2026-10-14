"""Nombres de nomenclatura (Desc. Trámite) del manual y agrupaciones de las reglas de historia."""

INGRESO_EFECTIVO = 'Ingreso efectivo'
INGRESO = 'Ingreso'
PRORROGA = 'Prorroga'
INGRESO_INFORME_X = 'Ingreso informe de cumplimiento X'
ORDEN_BUSQUEDA = 'Orden de búsqueda'
PIDE_CUENTA_BUSQUEDA = 'Pide cuenta búsqueda'
TERMINO_BUSQUEDA = 'Termino de búsqueda'
SIN_EFECTO_BUSQUEDA = 'Deja sin efecto búsqueda'
SIN_EFECTO_MN = 'Deja sin efecto medida interconexión M.N.'
CONTINUIDAD = 'Resp. continuidad de atención'
DIAGNOSTICO = 'Resp. Diagnóstico clínico especializado.'
DERECHO_OIDO = 'Derecho a ser oído'
FICHA_INDIVIDUAL = 'Ficha individual'
FICHA_FAE = 'Ficha FAE'
SERVICIO = 'Ing.Serv. protección especializada y otras redes'

DCE = (
    'SPE. Resp. Solicitud DCE (escrito)',
    'SPE. Resp. Rectificación DCE (escrito)',
    DIAGNOSTICO,
    CONTINUIDAD,
)
DIRECCION_REGIONAL = (
    'Derivación Dirección Reg.',
    'Pide cuenta Derivación D.R.',
    'Resp. Derivación dir. regional.',
    'Deja sin efecto D.R.',
    SIN_EFECTO_MN,
)
CURADOR = ('Designa abogado', 'Deja sin efecto designación C. Ad-litem')
BUSQUEDA_CITADA = (ORDEN_BUSQUEDA, PIDE_CUENTA_BUSQUEDA)
ANTERIORES_MN = (INGRESO_EFECTIVO, INGRESO, PRORROGA)
ESCRITOS = {
    'AMBULATORIO': 'Inf. Cump Programa Ambulatorio',
    'FAE': 'Inf. Cump FAE/FAS',
    'RESIDENCIAL': 'Inf. Cump Centro Residencial',
}
E10_DIAS = 100
RESIDENCIA_PREFIJOS = ('rta', 'rtt', 'res', 'rfa', 'rva')
