"""Política del operador, contrastada con las cuatro vistas RUS el 24-11-2026."""
from types import MappingProxyType
from .contracts import ContractError

REQUIRED_STATE = MappingProxyType({
    'ESPERA': ('4', 'Gestión Previa'),
    'CUMPLIMIENTO': ('6', 'Realizada'),
    'INFORMES_POR_VENCER': ('9', 'Realizada'),
    'INFORMES_VENCIDOS': ('9', 'Realizada'),
})


def required_state(origin):
    if not isinstance(origin, str) or origin not in REQUIRED_STATE:
        raise ContractError('ORIGEN_INVALIDO')
    return REQUIRED_STATE[origin]


def require_state(origin, code):
    expected, _ = required_state(origin)
    if type(code) is not str or code != expected:
        raise ContractError('ESTADO_NO_PERMITIDO_PARA_ORIGEN')
    return expected
