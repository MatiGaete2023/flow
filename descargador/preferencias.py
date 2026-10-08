"""Selecciones locales sin filtros privados ni autenticación."""
import calendar
from dataclasses import asdict
from datetime import date,timedelta
import json
import os
from pathlib import Path

from lotes import Batch,Dates
from motor import PocError


def data_directory():
    root=Path(os.environ.get('LOCALAPPDATA',Path.home()/'AppData'/'Local'))/'SITFA_Descargador_Integral'
    root.mkdir(parents=True,exist_ok=True);return root


def date_preset(name,today=None):
    today=today or date.today()
    if name=='Últimos 30 días':return today-timedelta(days=29),today
    first=today.replace(day=1)
    if name=='Mes anterior':last=first-timedelta(days=1);return last.replace(day=1),last
    if name=='Este mes':return first,today.replace(day=calendar.monthrange(today.year,today.month)[1])
    raise PocError('Período rápido desconocido.')


class Preferences:
    def __init__(self,path=None,memory=False):
        self.path=Path(path) if path else None if memory else data_directory()/'preferencias.json'
        self.data={'favoritos':{},'destino':'','tribunales':None,'aviso_sonoro':False,'csmp':'','dias_firmas':60,'tema':'Sistema'}
        if self.path and self.path.is_file():
            try:
                value=json.loads(self.path.read_text(encoding='utf-8'))
                if isinstance(value,dict):
                    for key in self.data:
                        if key in value and isinstance(value[key],type(self.data[key])):self.data[key]=value[key]
                    if isinstance(value.get('tribunales'),list):self.data['tribunales']=value['tribunales']
            except (OSError,ValueError):pass
    def save(self):
        if not self.path:return
        self.path.parent.mkdir(parents=True,exist_ok=True)
        temp=self.path.with_suffix('.tmp');temp.write_text(json.dumps(self.data,indent=2,ensure_ascii=False),encoding='utf-8');temp.replace(self.path)
    def favorite(self,name,batch,*,calendar_offset=0):
        if not name.strip():raise PocError('Escribe un nombre para el favorito.')
        if batch.keep_filters:raise PocError('Los favoritos no guardan filtros privados preparados en SITFA.')
        value=asdict(batch);value['dates']=None;value['keep_filters']=False
        if calendar_offset not in (0,1):raise PocError('Período relativo inválido.')
        value['calendar_offset']=calendar_offset
        value['date_policy']='Últimos 30 días' if batch.dates else ''
        self.data['favoritos'][name.strip()[:80]]=value;self.save()
    def restore(self,name,catalog,today=None):
        value=dict(self.data['favoritos'][name]);missing=[k for k in value['tribunals'] if k not in catalog['tribunales']]
        value['tribunals']=tuple(k for k in value['tribunals'] if k in catalog['tribunales'])
        for field in ('modalities','tabs'):value[field]=tuple(value[field])
        offset=value.pop('calendar_offset',0);policy=value.pop('date_policy','')
        if value.get('screen','').startswith('calendario_'):
            now=today or date.today();year,month=divmod(now.year*12+now.month-1+offset,12)
            value['month']=str(month+1);value['year']=str(year)
            if value['month'] not in catalog.get('meses',{}) or value['year'] not in catalog.get('anios',{}):raise PocError('El período relativo del perfil no está disponible en SITFA.')
        if policy or value.get('screen')=='carga':
            start,end=date_preset('Últimos 30 días',today)
            value['dates']=Dates(start.strftime('%d/%m/%Y'),end.strftime('%d/%m/%Y'))
        batch=Batch(**value);batch.plan(catalog)
        return batch,missing


def pending_plan(folder):
    from resultados import read_json
    summary=read_json(Path(folder)/'resumen.json')
    if not summary.get('plan'):raise PocError('El lote antiguo no tiene plan estructurado.')
    if summary.get('otros_filtros')=='formulario':raise PocError('Prepara otra vez los filtros privados; no se guardan para recuperación.')
    done=[{k:v for k,v in q.get('seleccion',{}).items() if k not in ('fecha_descarga',)} for q in summary['consultas']]
    return [s for s in summary['plan'] if {k:v for k,v in s.items() if k not in ('fecha_descarga',)} not in done]
