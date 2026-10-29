"""Historia de la causa en la misma pasada del listado; solo lectura, sin PDF, sin valores cifrados."""

import hashlib
import json
import shutil
from collections import Counter, defaultdict
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


CODIGOS_ERROR = frozenset(
    {
        'HISTORY_CONTEXT',
        'HISTORY_LINK',
        'HISTORY_IDENTITY',
        'HISTORY_RESPONSE',
        'HISTORY_REFLECTED',
        'HISTORY_TIMEOUT',
        'HISTORY_NETWORK',
        'HISTORIA_IDENTIDAD_VISIBLE',
        'HISTORIA_TRIBUNAL_VISIBLE',
        'HISTORIA_RESPUESTA_INCOMPLETA',
        'HISTORIA_SESION',
        'HISTORIA_TABLA_NO_RECONOCIDA',
        'HISTORIA_ENCABEZADO_DISTINTO',
        'HISTORIA_TABLA_AMBIGUA',
        'HISTORIA_VACIO_NO_COMPROBADO',
        'HISTORIA_PAGINACION',
    }
)


def codigo_error(exc, fallback='HISTORIA_LECTURA_ERROR'):
    if isinstance(exc, OSError):
        return 'HISTORIA_PERSISTENCIA'
    code = getattr(exc, 'code', None)
    if isinstance(code, str) and code in CODIGOS_ERROR:
        return code
    if isinstance(exc, ValueError) and exc.args and isinstance(exc.args[0], str) and exc.args[0] in CODIGOS_ERROR:
        return exc.args[0]
    return fallback


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
    def __init__(self, bridge, emit, cancel, pause=1.0, anterior=None, meses=4):
        self.bridge, self.emit, self.cancel, self.pause = bridge, emit, cancel, max(0.0, float(pause))
        if not isinstance(meses, int) or isinstance(meses, bool) or not 1 <= meses <= 4:
            raise m.PocError('El período de la historia va de 1 a 4 meses.')
        self.meses = meses
        self.anterior = anterior
        self.estado = None
        self.carpeta = None
        self.resultado = None
        self._anteriores = {}
        self._causas_por_rit = defaultdict(set)
        self._rits_por_causa = defaultdict(set)
        self._fallo_persistencia = False

    def abrir(self, carpeta_lote):
        self.lote = Path(carpeta_lote)
        self.carpeta = self.lote / 'historia'
        self.estado = {
            'version': 1,
            'tipo': 'HISTORIA_LECTURA',
            'estado': 'EN_CURSO',
            'criterio_temporal': f'ventana_{self.meses}_meses',
            'meses': self.meses,
            'causas': {},
            'vinculos': [],
            'lecturas': 0,
            'errores_listado': [],
        }
        from contrato_integral import VERSION, capabilities

        info = capabilities()
        self.estado['descargador'] = {'version': info['version'], 'commit': info['commit']}
        self.estado['extension_version'] = VERSION
        self.carpeta.mkdir(parents=True, exist_ok=True)
        self._cargar_anterior()
        self._persistir()

    def _cargar_anterior(self):
        """Conserva solo causas ya leídas cuyo HTML sigue íntegro; FALLIDA/PENDIENTE se vuelven a pedir."""
        if not self.anterior:
            return
        try:
            ruta = Path(self.anterior)
            datos = json.loads(ruta.read_text(encoding='utf-8'))
            # Con otro período (1 a 4 meses) la ventana y los conteos de la lectura anterior no valen: se relee.
            if datos.get('tipo') != 'HISTORIA_LECTURA' or datos.get('meses', 4) != self.meses:
                return
            for clave, causa in datos['causas'].items():
                origen = ruta.parent / str(causa.get('archivo', ''))
                if (
                    causa.get('estado') in LEIDAS
                    and origen.name == causa.get('archivo')
                    and not origen.is_symlink()
                    and origen.is_file()
                    and origen.stat().st_size <= MAX_BYTES
                    and hashlib.sha256(origen.read_bytes()).hexdigest() == causa.get('sha256')
                ):
                    destino = self.carpeta / origen.name
                    if not destino.exists():
                        shutil.copyfile(origen, destino)
                    self._anteriores[clave] = {**causa, 'reutilizada': True}
        except (OSError, ValueError, KeyError, AttributeError):
            self._anteriores = {}

    def pagina(self, selection, query, number, body, profile):
        """Una llamada por página del listado vigente; un error aquí nunca detiene las bitácoras."""
        try:
            tabla = _tabla_listado(body, profile)
            for fila in extract_historia(tabla, selection.tribunal):
                clave = f'RUS|{selection.tribunal}|{fila["causa_id"]}'
                self.estado['vinculos'].append(
                    {**fila, 'pestana': selection.tab, 'modalidad': selection.modality, 'clave': clave}
                )
                if clave not in self.estado['causas'] and clave in self._anteriores:
                    self.estado['causas'][clave] = self._anteriores[clave]
                causa = self.estado['causas'].setdefault(
                    clave,
                    {
                        'tribunal': selection.tribunal,
                        'tribunal_nombre': query.get('tribunal_nombre', ''),
                        'rit': fila['rit'],
                        'rit_enlace': fila.get('rit_enlace', fila['rit']),
                        'causa_id': fila['causa_id'],
                        'estado': 'PENDIENTE',
                    },
                )
                rit = m.normalized(fila['rit'])
                rit_key = (selection.tribunal, rit)
                self._causas_por_rit[rit_key].add(clave)
                self._rits_por_causa[clave].add(rit)
                conflicto = (
                    fila.get('identidad') != 'VERIFICADA'
                    or len(self._causas_por_rit[rit_key]) > 1
                    or len(self._rits_por_causa[clave]) > 1
                    or m.normalized(causa['rit']) != rit
                )
                if conflicto:
                    afectadas = set(self._causas_por_rit[rit_key]) | {clave}
                    for otra in afectadas:
                        incierta = self.estado['causas'][otra]
                        for campo in ('en_ventana', 'anteriores', 'incompletas', 'advertencias', 'desde', 'hasta'):
                            incierta.pop(campo, None)
                        incierta.update(estado='IDENTIDAD_INCIERTA', error='HISTORIA_IDENTIDAD_INCIERTA')
                    for vinculo in self.estado['vinculos']:
                        if vinculo['clave'] in afectadas:
                            vinculo['identidad'] = 'INCIERTA'
                    continue
                # Una FALLIDA de este lote no se repite al reaparecer en otra página; se reintenta en la
                # corrida siguiente (`anterior` solo conserva las leídas).
                if causa['estado'] == 'PENDIENTE' and not self.cancel.is_set() and not self._fallo_persistencia:
                    self._leer(causa)
        except (m.PocError, ValueError, OSError) as exc:
            self.estado['errores_listado'].append(
                {'pagina': number, 'error': codigo_error(exc, 'HISTORIA_LISTADO_ERROR')}
            )
        self._persistir()

    def _leer(self, causa):
        if self.estado['lecturas'] >= MAX_HISTORIAS_POR_LOTE:
            causa.update(estado='PENDIENTE', error='HISTORIA_TOPE')
            self.estado['tope_alcanzado'] = True
            return
        self.estado['lecturas'] += 1
        params = {
            'tribunal': causa['tribunal'],
            'rit': causa.get('rit_enlace') or causa['rit'],
            'causa': causa['causa_id'],
        }
        try:
            result = self.bridge.call('history_open', {'params': params, 'timeout': 60000})
            data = ChromeTransport.body(result)
            digest = hashlib.sha256(data).hexdigest()
            if len(data) > MAX_BYTES or digest != result.get('digest') or result.get('params') != params:
                raise ValueError('HISTORY_RESPONSE')
            leida = datetime.fromisoformat(result['startedAt'].replace('Z', '+00:00')).astimezone()
            lectura = read_historia(data)
            if m.normalized(lectura['encabezado'].get('RIT', '')) != m.normalized(causa['rit']):
                # E3: la ventana no es la causa pedida; no se guarda como suya.
                raise ValueError('HISTORIA_IDENTIDAD_VISIBLE')
            tribunal_visible = lectura['encabezado'].get('Tribunal', '')
            if (
                not tribunal_visible
                or not causa.get('tribunal_nombre')
                or m.normalized(tribunal_visible) != m.normalized(causa['tribunal_nombre'])
            ):
                raise ValueError('HISTORIA_TRIBUNAL_VISIBLE')
            nombre = (
                'historia-' + hashlib.sha256(json.dumps(params).encode()).hexdigest()[:24] + '-' + digest[:16] + '.html'
            )
            ruta = self.carpeta / nombre
            if not ruta.exists():
                m.save_exclusive(ruta, data)
            elif ruta.read_bytes() != data:
                raise m.PocError('El archivo local de historia cambió.')
            dentro, anteriores, posteriores, sin_fecha = separar_ventana(lectura['filas'], leida.date(), self.meses)
            causa.pop('reutilizada', None)
            causa.update(
                estado='VACIA_COMPROBADA' if lectura['vacio_explicito'] else 'LEIDA',
                error='',
                archivo=nombre,
                sha256=digest,
                leida=leida.isoformat(),
                desde=earliest(leida.date(), self.meses).isoformat(),
                hasta=leida.date().isoformat(),
                en_ventana=len(dentro) + len(posteriores) + len(sin_fecha),
                anteriores=len(anteriores),
                incompletas=sum(a['codigo'] == 'HISTORIA_FILA_INCOMPLETA' for a in lectura['advertencias']),
                advertencias=[a['codigo'] for a in lectura['advertencias']]
                + (['HISTORIA_FECHA_POSTERIOR'] if posteriores else []),
            )
        except (ValueError, m.PocError, KeyError, OSError) as exc:
            causa.update(estado='FALLIDA', error=codigo_error(exc))
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
            and not self._fallo_persistencia
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
        except OSError:
            self.estado['diagnostico_error'] = 'HISTORIA_PERSISTENCIA'
            self.estado['estado'] = 'INCOMPLETA'
        try:
            from nurus.capture.historias import export_historias

            destino = self.lote / 'Historia_SITFA.xlsx'
            if destino.exists():
                destino = self.lote / ('Historia_SITFA_' + uuid4().hex[:8] + '.xlsx')
            resultado['excel'] = str(export_historias(self.carpeta / MANIFEST, destino))
        except (ValueError, OSError) as exc:
            resultado['excel_error'] = self.estado['excel_error'] = codigo_error(exc, 'HISTORIA_EXCEL_ERROR')
            self.estado['estado'] = 'INCOMPLETA'
        self._persistir()
        resultado['estado'] = self.estado['estado']
        self.resultado = resultado
        self.emit('history_done', resultado)
        return resultado

    def _persistir(self):
        try:
            write(self.carpeta / MANIFEST, self.estado)
        except OSError:
            self._fallo_persistencia = True
            if self.estado['estado'] != 'EN_CURSO':
                self.estado['estado'] = 'INCOMPLETA'
            self.estado['persistencia_error'] = 'HISTORIA_PERSISTENCIA'
