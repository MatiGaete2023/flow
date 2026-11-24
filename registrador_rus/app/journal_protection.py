"""Almacenamiento de simulación: protege campos antes de SQLite, no cifra toda la DB.

Perfil DPAPI de usuario Windows, sin migración desde texto plano ni fallback.
Los metadatos de control siguen visibles; no es custodia institucional aprobada.
"""
from functools import lru_cache
from hashlib import sha256
import hmac
import os
from pathlib import Path
import stat

from .protected_demo_transfer import _dpapi, UI_FORBIDDEN


SUFFIX = '.protected.sqlite'
PROFILE = 'DPAPI_DEMO_V1'
KEY_DESCRIPTION = 'Flow journal demo key v1'
FIELD_DESCRIPTION = 'Flow journal demo field v1'
KEY_ENTROPY = b'Flow.Journal.Demo.Key.V1'
MAX_FIELD = 32 * 1024 * 1024


class StorageProtectionError(RuntimeError):
    """Error sin datos de operaciones ni detalles de DPAPI."""


class PlainStorage:
    protected = False

    def digest(self, scope, text):
        return sha256(text.encode('utf-8')).hexdigest()

    def seal(self, scope, operation_id, text):
        return text

    def unseal(self, scope, operation_id, value):
        return value

    def entry_key(self, identity_key, entry_id):
        return entry_id

    def require_demo(self, op):
        pass


@lru_cache(maxsize=1)
def _demo_operations():
    from .simulation import demo_operations
    return demo_operations(400)


class ProtectedStorage:
    protected = True

    def __init__(self, crypt, key):
        if type(key) is not bytes or len(key) != 32:
            raise StorageProtectionError('CLAVE_DIARIO_INVALIDA')
        self._crypt = crypt
        self._key = key

    def digest(self, scope, text):
        data = scope.encode('ascii') + b'\0' + text.encode('utf-8')
        return hmac.new(self._key, data, sha256).hexdigest()

    def _entropy(self, scope, operation_id):
        return bytes.fromhex(self.digest('field-scope',scope+'\0'+operation_id))

    def seal(self, scope, operation_id, text):
        try:
            data = text.encode('utf-8')
            if len(data)>MAX_FIELD:
                raise StorageProtectionError('CAMPO_DIARIO_EXCEDE_LIMITE')
            blob = self._crypt.CryptProtectData(data,FIELD_DESCRIPTION,self._entropy(scope,operation_id),
                                                None,None,UI_FORBIDDEN)
            if type(blob) is not bytes or not 0<len(blob)<=MAX_FIELD+65536:
                raise StorageProtectionError('CIFRADO_DIARIO_INVALIDO')
            return blob
        except StorageProtectionError:
            raise
        except Exception:
            raise StorageProtectionError('CIFRADO_DIARIO_FALLIDO') from None

    def unseal(self, scope, operation_id, value):
        if type(value) is not bytes or not 0<len(value)<=MAX_FIELD+65536:
            raise StorageProtectionError('CAMPO_DIARIO_NO_CIFRADO')
        try:
            description,data = self._crypt.CryptUnprotectData(value,self._entropy(scope,operation_id),
                                                             None,None,UI_FORBIDDEN)
            if description!=FIELD_DESCRIPTION or type(data) is not bytes or len(data)>MAX_FIELD:
                raise StorageProtectionError('CAMPO_DIARIO_INVALIDO')
            return data.decode('utf-8',errors='strict')
        except StorageProtectionError:
            raise
        except Exception:
            raise StorageProtectionError('DESCIFRADO_DIARIO_FALLIDO') from None

    def entry_key(self, identity_key, entry_id):
        # Mismo ingreso + misma entrada => misma huella; conserva la restricción UNIQUE.
        return self.digest('receipt-entry',identity_key+'\0'+entry_id)

    def require_demo(self, op):
        # Igualdad completa, no prefijos ni una mera etiqueta DEMO.
        if op not in _demo_operations():
            raise StorageProtectionError('SOLO_DIARIO_FICTICIO_GENERADO')


def check_path(path, protected):
    """No abrir rutas de modo incompatible. No certifica ACL ni ruta institucional."""
    if type(protected) is not bool:
        raise StorageProtectionError('PERFIL_DIARIO_INVALIDO')
    path = Path(path)
    if path.name.endswith(SUFFIX) != protected:
        raise StorageProtectionError('EXTENSION_DIARIO_INCOMPATIBLE')
    if protected:
        # Antes de crear archivos, comprobar que DPAPI existe.
        try:
            crypt = _dpapi()
        except ValueError:
            raise StorageProtectionError('DPAPI_DIARIO_NO_DISPONIBLE') from None
        # Rechazar archivo/enlaces de redirección existentes, también WAL/SHM.
        for item in (path,Path(str(path)+'-wal'),Path(str(path)+'-shm'),Path(str(path)+'-journal')):
            try: info=item.lstat()
            except FileNotFoundError: continue
            if (not stat.S_ISREG(info.st_mode) or
                    getattr(info,'st_file_attributes',0)&getattr(stat,'FILE_ATTRIBUTE_REPARSE_POINT',0x400)):
                raise StorageProtectionError('RUTA_DIARIO_INVALIDA')
        return crypt
    return None


def initialize_storage(db, protected, crypt):
    """Inicialización transaccional antes de crear tablas de operaciones.

    Un diario existente sin perfil no se migra. Si se renombra uno protegido,
    el marcador de base de datos también bloquea el modo anterior.
    """
    db.execute('BEGIN IMMEDIATE')
    try:
        names={r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")}
        if not protected:
            if 'journal_storage' in names:
                raise StorageProtectionError('DIARIO_PROTEGIDO_REQUIERE_MODO_EXPLICITO')
            result=PlainStorage()
        elif 'journal_storage' not in names:
            if names:
                raise StorageProtectionError('MIGRACION_DIARIO_NO_PERMITIDA')
            key=os.urandom(32)
            try:
                blob=crypt.CryptProtectData(key,KEY_DESCRIPTION,KEY_ENTROPY,None,None,UI_FORBIDDEN)
            except Exception:
                raise StorageProtectionError('CLAVE_DIARIO_NO_PROTEGIDA') from None
            if type(blob) is not bytes or not blob:
                raise StorageProtectionError('CLAVE_DIARIO_NO_PROTEGIDA')
            db.execute('CREATE TABLE journal_storage(profile TEXT PRIMARY KEY, wrapped_key BLOB NOT NULL)')
            db.execute('INSERT INTO journal_storage VALUES(?,?)',(PROFILE,blob))
            result=ProtectedStorage(crypt,key)
        else:
            rows=db.execute('SELECT profile,wrapped_key FROM journal_storage').fetchall()
            if len(rows)!=1 or rows[0][0]!=PROFILE or type(rows[0][1]) is not bytes:
                raise StorageProtectionError('PERFIL_DIARIO_INVALIDO')
            try:
                description,key=crypt.CryptUnprotectData(rows[0][1],KEY_ENTROPY,None,None,UI_FORBIDDEN)
            except Exception:
                raise StorageProtectionError('CLAVE_DIARIO_NO_DISPONIBLE') from None
            if description!=KEY_DESCRIPTION:
                raise StorageProtectionError('CLAVE_DIARIO_INVALIDA')
            result=ProtectedStorage(crypt,key)
        db.execute('COMMIT')
        return result
    except BaseException:
        if db.in_transaction:db.execute('ROLLBACK')
        raise
