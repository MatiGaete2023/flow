"""Transporte cifrado de demostración, solo Windows y datos inventados.

DPAPI con ámbito de usuario, sin LOCAL_MACHINE ni alternativa en claro.
No autentica al funcionario, no autoriza operaciones ni protege el diario.
No es una puerta de exportación de Work real.
"""
import argparse
import os
from pathlib import Path
import stat
from tempfile import NamedTemporaryFile

from .contracts import ContractError
from .demo_transfer import make_demo_batch, require_demo_batch
from .transfer import MAX_BYTES, PreviewBatch

SUFFIX = '.flow-demo.dpapi'
MAX_CIPHERTEXT = MAX_BYTES + 65536
DESCRIPTION = 'Flow registrador demo protegido v1'
ENTROPY = b'Flow.Registrador.Demo.Transfer.V1'
UI_FORBIDDEN = 1  # CRYPTPROTECT_UI_FORBIDDEN; nunca CRYPTPROTECT_LOCAL_MACHINE.


def _dpapi():
    if os.name != 'nt':
        raise ContractError('DPAPI_REQUIERE_WINDOWS')
    try:
        import win32crypt
    except ImportError:
        raise ContractError('DPAPI_NO_DISPONIBLE') from None
    return win32crypt


def protect_demo(count=8):
    """No acepta un lote externo: el texto e identidad se generan aquí."""
    batch = make_demo_batch(count)
    crypt = _dpapi()
    try:
        result = crypt.CryptProtectData(batch.encode(), DESCRIPTION, ENTROPY, None, None, UI_FORBIDDEN)
    except Exception:
        raise ContractError('CIFRADO_DPAPI_FALLIDO') from None
    if type(result) is not bytes or not 0 < len(result) <= MAX_CIPHERTEXT:
        raise ContractError('CIFRADO_DPAPI_INVALIDO')
    return result


def unprotect_demo(data):
    if type(data) is not bytes or not 0 < len(data) <= MAX_CIPHERTEXT:
        raise ContractError('CIFRADO_DPAPI_INVALIDO')
    crypt = _dpapi()
    try:
        description, plaintext = crypt.CryptUnprotectData(data, ENTROPY, None, None, UI_FORBIDDEN)
    except Exception:
        raise ContractError('DESCIFRADO_DPAPI_FALLIDO') from None
    if description != DESCRIPTION or type(plaintext) is not bytes or len(plaintext) > MAX_BYTES:
        raise ContractError('SOBRE_DPAPI_INVALIDO')
    # DPAPI no prueba aprobación: exige la familia ficticia íntegra, no solo un hash.
    return require_demo_batch(PreviewBatch.decode(plaintext))


def _regular(info):
    return (stat.S_ISREG(info.st_mode) and
            not getattr(info, 'st_file_attributes', 0) & getattr(stat, 'FILE_ATTRIBUTE_REPARSE_POINT', 0x400))


def export_protected_demo(path, count=8):
    """Archivo nuevo, nunca sobrescribe, incluso si falla DPAPI."""
    target = Path(path)
    if not target.name.endswith(SUFFIX):
        raise ContractError('EXTENSION_PROTEGIDA_REQUERIDA')
    data = protect_demo(count)
    temp = None
    try:
        with NamedTemporaryFile(dir=target.parent, prefix='.flow-protected-demo-', delete=False) as stream:
            temp = Path(stream.name)
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.link(temp, target)
    finally:
        if temp is not None:
            temp.unlink(missing_ok=True)
    return target


def import_protected_demo(path):
    source = Path(path)
    if not source.name.endswith(SUFFIX) or not _regular(source.lstat()):
        raise ContractError('ARCHIVO_PROTEGIDO_INVALIDO')
    with source.open('rb') as stream:
        if not _regular(os.fstat(stream.fileno())):
            raise ContractError('ARCHIVO_PROTEGIDO_INVALIDO')
        data = stream.read(MAX_CIPHERTEXT+1)
    return unprotect_demo(data)


def main():
    parser = argparse.ArgumentParser(description='Generar un lote ficticio cifrado por usuario Windows')
    parser.add_argument('output')
    parser.add_argument('--count', type=int, default=8)
    args = parser.parse_args()
    try:
        export_protected_demo(args.output,args.count)
    except (OSError,ValueError):
        parser.exit(2,'No se pudo crear el lote ficticio protegido.\n')


if __name__ == '__main__':
    main()
