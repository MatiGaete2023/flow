"""Biblioteca de productos (P4, port de la Biblioteca de Resultados de Flow Rust): archivos creados en la carpeta de salida
y en sus subcarpetas de corridas anteriores, con búsqueda. Las rutas salen de la carpeta de salida conocida, nunca de
mensajes; no sigue enlaces simbólicos ni sale de la carpeta base."""

from __future__ import annotations

import os
import unicodedata
from datetime import datetime
from pathlib import Path

EXTENSIONES = ('.xlsx', '.xls', '.docx', '.eml', '.msg', '.zip', '.json', '.pdf', '.csv', '.ics', '.txt')
LIMITE = 2000


def _clave(texto):
    texto = unicodedata.normalize('NFKD', str(texto)).encode('ascii', 'ignore').decode().lower()
    return ' '.join(texto.split())


def biblioteca(base, limite=LIMITE, profundidad=4):
    """[{'ruta', 'nombre', 'carpeta', 'modificado', 'bytes'}] del más reciente al más antiguo."""
    base = Path(base)
    if not base.is_dir() or base.is_symlink():
        return []
    raiz = base.resolve()
    salida = []
    for actual, carpetas, archivos in os.walk(raiz, followlinks=False):
        nivel = len(Path(actual).relative_to(raiz).parts)
        carpetas[:] = [c for c in carpetas if not (Path(actual) / c).is_symlink() and not c.startswith('.')]
        if nivel >= profundidad:
            carpetas[:] = []
        for nombre in archivos:
            ruta = Path(actual) / nombre
            if ruta.suffix.lower() not in EXTENSIONES or ruta.is_symlink() or nombre.startswith(('~$', '.')):
                continue
            try:
                info = ruta.stat()
            except OSError:
                continue
            salida.append(
                {
                    'ruta': ruta,
                    'nombre': nombre,
                    'carpeta': str(ruta.parent.relative_to(raiz)) if ruta.parent != raiz else '.',
                    'modificado': datetime.fromtimestamp(info.st_mtime).astimezone(),
                    'bytes': info.st_size,
                }
            )
    salida.sort(key=lambda item: item['modificado'], reverse=True)
    return salida[:limite]


def filtrar(items, texto):
    """Búsqueda sin distinguir mayúsculas ni tildes; todas las palabras deben aparecer en el nombre o la carpeta."""
    palabras = _clave(texto).split()
    if not palabras:
        return list(items)
    return [i for i in items if all(p in _clave(i['nombre'] + ' ' + i['carpeta']) for p in palabras)]


def tamano(bytes_):
    for unidad in ('B', 'KB', 'MB', 'GB'):
        if bytes_ < 1024 or unidad == 'GB':
            return f'{bytes_:.0f} {unidad}' if unidad == 'B' else f'{bytes_:.1f} {unidad}'
        bytes_ /= 1024
    return ''
