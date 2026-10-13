"""Historia de la causa en la misma pasada del listado; solo lectura, sin PDF, sin valores cifrados."""

import hashlib
import json
import shutil
from collections import Counter
from datetime import datetime
from pathlib import Path
from uuid import uuid4

import motor as m
from lectura_bitacoras import write
from lotes import ChromeTransport
from vinculos import extract_historia
from nurus.capture.temporal import earliest
from nurus.historia_html import read_historia, separar_ventana

MANIFEST = 'historias.json'
MAX_HISTORIAS_POR_LOTE = 600
MAX_BYTES = 8 * 1024 * 1024
LEIDAS = ('LEIDA', 'VACIA_COMPROBADA')


def _tabla_listado(body, profile):
    """La misma tabla que usa listing_rows."""
    if isinstance(body, bytes):
        try:
            body = body.decode('utf-8-sig')
        except UnicodeDecodeError:
            pass
    tablas = m.root_html(body).xpath(
        '//form[@name=$form]//table[@id=$table]', form=profile.form_name, table=profile.table_id
    )
    if len(tablas) != 1:
        raise m.PocError('No se identifica una tabla única de registros.')
    return tablas[0]


class LectorHistoria:
    def __init__(self, bridge, emit, cancel, pause=1.0, anterior=None):
        self.bridge, self.emit, self.cancel, self.pause = bridge, emit, cancel, max(0.0, float(pause))
        self.anterior = anterior
        self.estado = None
        self.carpeta = None
        self.resultado = None

    def abrir(self, carpeta_lote):
        self.lote = Path(carpeta_lote)
        self.carpeta = self.lote / 'historia'
        self.carpeta.mkdir(parents=True, exist_ok=True)
        self.estado = {
            'version': 1,
            'tipo': 'HISTORIA_LECTURA',
            'estado': 'EN_CURSO',
            'criterio_temporal': 'ventana_4_meses',
            'causas': {},
            'vinculos': [],
            'lecturas': 0,
            'errores_listado': [],
        }
        from contrato_integral import VERSION, capabilities

        info = capabilities()
        self.estado['descargador'] = {'version': info['version'], 'commit': info['commit']}
        self.estado['extension_version'] = VERSION
        self._cargar_anterior()
        self._persistir()

    def _cargar_anterior(self):
        """Conserva solo causas ya leídas cuyo HTML sigue íntegro; FALLIDA/PENDIENTE se vuelven a pedir."""
        if not self.anterior:
            return
        try:
            ruta = Path(self.anterior)
            datos = json.loads(ruta.read_text(encoding='utf-8'))
            if datos.get('tipo') != 'HISTORIA_LECTURA':
                return
            for clave, causa in datos['causas'].items():
                origen = ruta.parent / str(causa.get('archivo', ''))
                if (
                    causa.get('estado') in LEIDAS
                    and origen.is_file()
                    and origen.stat().st_size <= MAX_BYTES
                    and hashlib.sha256(origen.read_bytes()).hexdigest() == causa.get('sha256')
                ):
                    destino = self.carpeta / origen.name
                    if not destino.exists():
                        shutil.copyfile(origen, destino)
                    self.estado['causas'][clave] = {**causa, 'reutilizada': True}
        except (OSError, ValueError, KeyError, AttributeError):
            self.estado['causas'] = {}

    def pagina(self, selection, query, number, body, profile):
        """Una llamada por página del listado vigente; un error aquí nunca detiene las bitácoras."""
        try:
            tabla = _tabla_listado(body, profile)
            for fila in extract_historia(tabla, selection.tribunal):
                clave = f'RUS|{selection.tribunal}|{fila["causa_id"]}'
                self.estado['vinculos'].append(
                    {**fila, 'pestana': selection.tab, 'modalidad': selection.modality, 'clave': clave}
                )
                causa = self.estado['causas'].setdefault(
                    clave,
                    {
                        'tribunal': selection.tribunal,
                        'tribunal_nombre': query.get('tribunal_nombre', ''),
                        'rit': fila['rit'],
                        'causa_id': fila['causa_id'],
                        'estado': 'PENDIENTE',
                    },
                )
                if causa['estado'] in ('PENDIENTE', 'FALLIDA') and not self.cancel.is_set():
                    self._leer(causa)
        except (m.PocError, ValueError, OSError) as exc:
            self.estado['errores_listado'].append({'pagina': number, 'error': getattr(exc, 'code', '') or str(exc)})
        self._persistir()

    def _leer(self, causa):
        if self.estado['lecturas'] >= MAX_HISTORIAS_POR_LOTE:
            causa.update(estado='PENDIENTE', error=f'Tope de {MAX_HISTORIAS_POR_LOTE} historias por lote.')
            self.estado['tope_alcanzado'] = True
            return
        self.estado['lecturas'] += 1
        params = {'tribunal': causa['tribunal'], 'rit': causa['rit'], 'causa': causa['causa_id']}
        try:
            result = self.bridge.call('history_open', {'params': params, 'timeout': 60000})
            data = ChromeTransport.body(result)
            digest = hashlib.sha256(data).hexdigest()
            if len(data) > MAX_BYTES or digest != result.get('digest') or result.get('params') != params:
                raise m.PocError('La respuesta de historia no coincide con la solicitud.')
            leida = datetime.fromisoformat(result['startedAt'].replace('Z', '+00:00')).astimezone()
            lectura = read_historia(data)
            nombre = (
                'historia-' + hashlib.sha256(json.dumps(params).encode()).hexdigest()[:24] + '-' + digest[:16] + '.html'
            )
            ruta = self.carpeta / nombre
            if not ruta.exists():
                m.save_exclusive(ruta, data)
            elif ruta.read_bytes() != data:
                raise m.PocError('El archivo local de historia cambió.')
            dentro, anteriores, posteriores, sin_fecha = separar_ventana(lectura['filas'], leida.date())
            causa.pop('reutilizada', None)
            causa.update(
                estado='VACIA_COMPROBADA' if lectura['vacio_explicito'] else 'LEIDA',
                error='',
                archivo=nombre,
                sha256=digest,
                leida=leida.isoformat(),
                desde=earliest(leida.date()).isoformat(),
                hasta=leida.date().isoformat(),
                en_ventana=len(dentro) + len(posteriores) + len(sin_fecha),
                anteriores=len(anteriores),
                incompletas=sum(a['codigo'] == 'HISTORIA_FILA_INCOMPLETA' for a in lectura['advertencias']),
                advertencias=[a['codigo'] for a in lectura['advertencias']],
            )
        except ValueError as exc:
            causa.update(estado='FALLIDA', error=str(exc))
        except (m.PocError, KeyError, OSError) as exc:
            causa.update(estado='FALLIDA', error=getattr(exc, 'code', '') or str(exc))
        self.emit('history_progress', {'causa': causa['rit'], 'estado': causa['estado']})
        self.cancel.wait(self.pause)

    def cerrar(self, lote_estado='COMPLETA'):
        """Estado final, diagnóstico anónimo y Excel. Nunca lanza: un fallo queda anotado en el manifiesto."""
        if self.estado is None or self.estado['estado'] != 'EN_CURSO':
            return self.resultado
        causas = self.estado['causas'].values()
        conteo = Counter(c['estado'] for c in causas)
        completa = (
            lote_estado == 'COMPLETA'
            and not self.estado['errores_listado']
            and all(c['estado'] in LEIDAS for c in causas)
        )
        self.estado['estado'] = 'CANCELADA' if self.cancel.is_set() else 'COMPLETA' if completa else 'INCOMPLETA'
        self.estado['conteo'] = dict(conteo)
        self._persistir()
        resultado = {
            'estado': self.estado['estado'],
            'leidas': sum(conteo[k] for k in LEIDAS),
            'fallidas': conteo['FALLIDA'],
            'pendientes': conteo['PENDIENTE'],
            'tramites_en_ventana': sum(c.get('en_ventana', 0) for c in self.estado['causas'].values()),
            'manifiesto': str(self.carpeta / MANIFEST),
        }
        try:
            write(
                self.carpeta / 'diagnostico_historia.json',
                {
                    'estado': self.estado['estado'],
                    'causas_por_estado': dict(conteo),
                    'codigos_error': dict(
                        Counter(c.get('error', '') for c in self.estado['causas'].values() if c.get('error'))
                    ),
                    'errores_listado': len(self.estado['errores_listado']),
                    'tope_alcanzado': bool(self.estado.get('tope_alcanzado')),
                },
            )
        except OSError as exc:
            self.estado['diagnostico_error'] = str(exc)
        try:
            from nurus.capture.historias import export_historias

            destino = self.lote / 'Historia_SITFA.xlsx'
            if destino.exists():
                destino = self.lote / ('Historia_SITFA_' + uuid4().hex[:8] + '.xlsx')
            resultado['excel'] = str(export_historias(self.carpeta / MANIFEST, destino))
        except (ValueError, OSError) as exc:
            resultado['excel_error'] = self.estado['excel_error'] = str(exc)
        self._persistir()
        self.resultado = resultado
        self.emit('history_done', resultado)
        return resultado

    def _persistir(self):
        write(self.carpeta / MANIFEST, self.estado)
