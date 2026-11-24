"""Perfil de carpeta privada para ensayos DPAPI Windows, sin cambiar ACL existentes.

No acredita política institucional, cifrado de volumen ni ausencia de sincronización.
La comprobación por ruta no resiste a otro proceso de la misma cuenta/administrador.
"""
import os
from pathlib import Path
import re
import stat

from .journal_protection import StorageProtectionError

# SECURITY_DESCRIPTOR_CONTROL / ACE flags, valores Win32 documentados.
DACL_PROTECTED = 0x1000
OI_CI = 0x03
INHERITED = 0x10
FILE_ALL_ACCESS = 0x1F01FF
SYSTEM_SID = 'S-1-5-18'
ADMIN_SID = 'S-1-5-32-544'


def _api():
    if os.name != 'nt':
        raise StorageProtectionError('CARPETA_PRIVADA_REQUIERE_WINDOWS')
    try:
        import win32api
        import win32file
        import win32security
        return win32api,win32file,win32security
    except ImportError:
        raise StorageProtectionError('ACL_WINDOWS_NO_DISPONIBLE') from None


def _current(a,s):
    token=s.OpenProcessToken(a.GetCurrentProcess(),8)  # TOKEN_QUERY
    try:
        user=s.ConvertSidToStringSid(s.GetTokenInformation(token,s.TokenUser)[0])
        owner=s.ConvertSidToStringSid(s.GetTokenInformation(token,s.TokenOwner))
    finally:
        token.Close()
    return user,owner


def _safe_location(path,a,f):
    p=Path(path)
    if not p.is_absolute() or not re.fullmatch(r'[A-Za-z]:\\',p.anchor) or len(p.parts)<2:
        raise StorageProtectionError('UBICACION_LOCAL_ABSOLUTA_REQUERIDA')
    if any(x in ('.','..') or x.endswith((' ','.')) or ':' in x or
           Path(x).is_reserved() for x in p.parts[1:]):
        raise StorageProtectionError('RUTA_DIARIO_INVALIDA')
    if f.GetDriveType(p.anchor)!=3 or a.GetVolumeInformation(p.anchor)[4].upper()!='NTFS':
        raise StorageProtectionError('VOLUMEN_NTFS_FIJO_REQUERIDO')
    for part in reversed(p.parents):
        info=part.lstat()
        if not stat.S_ISDIR(info.st_mode) or getattr(info,'st_file_attributes',0)&0x400:
            raise StorageProtectionError('ANCESTRO_DIARIO_INSEGURO')
    return p


def _acl(path,a,s,*,directory):
    user,default_owner=_current(a,s)
    sd=s.GetNamedSecurityInfo(str(path),s.SE_FILE_OBJECT,
                              s.OWNER_SECURITY_INFORMATION|s.DACL_SECURITY_INFORMATION)
    owner=s.ConvertSidToStringSid(sd.GetSecurityDescriptorOwner())
    allowed_owners={user}
    # SQLite crea sus auxiliares con el propietario por defecto del token.
    # Una cuenta elevada puede usar Administradores como propietario; no
    # admitir otros grupos, ni permitirles una ACE en la DACL.
    if not directory and default_owner==ADMIN_SID:
        allowed_owners.add(ADMIN_SID)
    if owner not in allowed_owners:
        raise StorageProtectionError('PROPIETARIO_DIARIO_INESPERADO')
    if directory and not sd.GetSecurityDescriptorControl()[0]&DACL_PROTECTED:
        raise StorageProtectionError('HERENCIA_CARPETA_NO_PROTEGIDA')
    dacl=sd.GetSecurityDescriptorDacl()
    if dacl is None or dacl.GetAceCount()!=2:
        raise StorageProtectionError('DACL_DIARIO_INSEGURA')
    trustees=set()
    for i in range(dacl.GetAceCount()):
        ace=dacl.GetAce(i)
        if len(ace)!=3 or ace[0][0]!=0 or ace[1]!=FILE_ALL_ACCESS:
            raise StorageProtectionError('ACE_DIARIO_INSEGURA')
        flags=ace[0][1]
        if (directory and flags!=OI_CI) or (not directory and flags not in (0,INHERITED)):
            raise StorageProtectionError('HERENCIA_DIARIO_INSEGURA')
        trustees.add(s.ConvertSidToStringSid(ace[2]))
    if trustees!={user,SYSTEM_SID}:
        raise StorageProtectionError('DACL_DIARIO_INSEGURA')


def _directory(path,a,f,s):
    p=_safe_location(path,a,f)
    info=p.lstat()
    if not stat.S_ISDIR(info.st_mode) or getattr(info,'st_file_attributes',0)&0x400:
        raise StorageProtectionError('CARPETA_DIARIO_INSEGURA')
    _acl(p,a,s,directory=True)
    return p


def create_private_demo_directory(path):
    """Crea UNA carpeta nueva con DACL explícita desde su creación, no repara otras."""
    a,f,s=_api()
    try:
        p=_safe_location(path,a,f)
        user,_=_current(a,s)
        sd=s.ConvertStringSecurityDescriptorToSecurityDescriptor(
            f'O:{user}D:P(A;OICI;FA;;;{user})(A;OICI;FA;;;SY)',1)
        sa=s.SECURITY_ATTRIBUTES()
        sa.SECURITY_DESCRIPTOR=sd
        f.CreateDirectory(str(p),sa)
        return _directory(p,a,f,s)
    except StorageProtectionError:
        raise
    except Exception:
        raise StorageProtectionError('CREACION_CARPETA_PRIVADA_FALLIDA') from None


def validate_private_demo(path):
    """Solo lectura de permisos; no seguir enlaces ni corregir automáticamente."""
    a,f,s=_api()
    try:
        p=_safe_location(path,a,f)
        _directory(p.parent,a,f,s)
        for item in (p,*(Path(str(p)+suffix) for suffix in ('-wal','-shm','-journal'))):
            try: info=item.lstat()
            except FileNotFoundError: continue
            if (not stat.S_ISREG(info.st_mode) or getattr(info,'st_file_attributes',0)&0x400
                    or info.st_nlink!=1):
                raise StorageProtectionError('ARCHIVO_DIARIO_INSEGURO')
            _acl(item,a,s,directory=False)
    except StorageProtectionError:
        raise
    except Exception:
        raise StorageProtectionError('ACL_DIARIO_NO_VERIFICABLE') from None
