"""Oráculo E-S3 de historial local y verificación del libro conjunto."""
import hashlib
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(ROOT / 'app' / 'qa'), str(ROOT / 'descargador'), str(ROOT / 'src')]
import generar_lote  # normaliza los saltos de línea
from archivo_local import remember, discover, diary_references
from resultados import inside, digest_file


class Prefs:
    def __init__(self):
        self.data = {'historial_lotes': [], 'historial_bitacoras': []}
    def save(self):
        pass


def run(destino):
    with tempfile.TemporaryDirectory(prefix='historial-local-') as temporal:
        raiz = Path(temporal).resolve()
        casos = []
        def relativo(path):
            return Path(path).resolve().relative_to(raiz).as_posix()
        def crear(nombre, control, datos):
            carpeta = raiz / nombre
            carpeta.mkdir(parents=True, exist_ok=True)
            (carpeta / control).write_text(json.dumps(datos, ensure_ascii=False), encoding='utf-8')
            return carpeta
        lote = crear('lote', 'resumen.json', {'estado': 'VALIDADA', 'consultas_completadas': 3})
        conjunto = crear('conjunto', 'flujo.json', {'estado': 'COMPLETA', 'fases': {'a': {}, 'b': {}}})
        diario = crear('diario', 'bitacoras.json', {'estado': 'COMPLETA', 'registros': [{}, {}]})
        def registrar(nombre, prefs, base='sin-base', ultimo=None, excluidas=()):
            filas = discover(raiz / base, prefs, raiz / ultimo if ultimo else None)
            casos.append({'caso': nombre, 'lotes': [relativo(p) for p in prefs.data['historial_lotes']],
                          'bitacoras': [relativo(p) for p in prefs.data['historial_bitacoras']],
                          'base': base, 'ultimo': ultimo,
                          'archivos': {relativo(f): f.read_text(encoding='utf-8') for f in raiz.rglob('*') if f.is_file()},
                          'excluidas': list(excluidas),
                          'referencias': [relativo(p) for p in diary_references(prefs, [raiz / p for p in excluidas])],
                          'resultado': [[relativo(p), t, e, str(n)] for p, t, e, n in filas]})
        p = Prefs()
        remember(p, lote); registrar('recordar_lote', p)
        remember(p, conjunto); registrar('recordar_conjunto', p)
        remember(p, diario); registrar('recordar_bitacoras', p)
        remember(p, lote); registrar('sin_duplicados', p)
        registrar('descubrir_base', Prefs(), base='.')
        registrar('descubrir_ultimo', Prefs(), ultimo='lote')
        registrar('orden', p, base='.')
        registrar('excluidas', p, excluidas=['diario/bitacoras.json'])
        p.data['historial_lotes'].append(str(raiz / 'lote_ausente'))
        registrar('lote_ausente', p)
        p.data['historial_bitacoras'].append(str(raiz / 'diario_ausente' / 'bitacoras.json'))
        registrar('bitacora_ausente', p)
        (diario / 'bitacoras.json').write_text('{', encoding='utf-8')
        registrar('bitacora_danada', p)
        libro = b'libro sintetico\n'
        (conjunto / 'libro.xlsx').write_bytes(libro)
        huella = hashlib.sha256(libro).hexdigest()
        for nombre, archivo, esperado in [('libro_correcto', 'libro.xlsx', huella), ('libro_alterado', 'libro.xlsx', '0' * 64), ('sin_archivo', None, None)]:
            resultado = None if archivo is None else digest_file(inside(conjunto, archivo)) == esperado
            casos.append({'caso': nombre, 'archivo': archivo, 'sha256': esperado, 'resultado': resultado})
        Path(destino).write_text(json.dumps({'version': 1, 'casos': casos}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    run(sys.argv[1] if len(sys.argv) > 1 else ROOT / 'app' / 'tests' / 'datos' / 'historial_local.json')
