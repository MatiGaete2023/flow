"""Configuración de dominio por usuario; nunca altera la instalación."""
from copy import deepcopy
from pathlib import Path
import json
import os
import re
import shutil
import tempfile
from string import Formatter

BASE = Path(__file__).parent
CC = 'ucc_concepcion@pjud.cl'
VARIABLES = {'TRIBUNAL','ALCANCE_MODALIDADES','PERIODO','FECHA','PROGRAMA','TABLA_REGISTROS'}
UMBRALES = dict(espera_dce=30, espera_laja=30, espera_mulchen=30, espera_tome=30,
               proyecto_tome=60, mayoria=60, oido=45, resolucion_reciente=30,
               ingreso_reciente=30, medida=45, informe=30, ficha_antigua=180,
               ficha_reciente=30, ficha_fae=120)
PARAMETER_LABELS = {
    'espera_dce':'Espera DCE: consultar ingreso desde',
    'espera_laja':'Espera Laja: proyecto y correo desde',
    'espera_mulchen':'Espera Mulchén: proyecto y correo desde',
    'espera_tome':'Espera Tomé: correo desde',
    'proyecto_tome':'Espera Tomé: proyecto desde',
    'mayoria':'Avisar mayoría de edad dentro de',
    'oido':'Oído: considerar reciente hasta',
    'resolucion_reciente':'Resolución reciente: menos de',
    'ingreso_reciente':'Ingreso reciente: hasta',
    'medida':'Avisar medida por vencer dentro de',
    'informe':'Avisar informe por vencer dentro de',
    'ficha_antigua':'Ficha individual antigua: más de',
    'ficha_reciente':'Ficha individual reciente: hasta',
    'ficha_fae':'Ficha FAE ausente: ingreso hace más de',
}
CORREO_REVISION = 3
LEGACY_OBSERVATION_TEXTS = {
    'COMUN.MAYORIA_EDAD': 'Se hace presente que {PNOMBRE} alcanzó la mayoría de edad el {FECHA_MAYORIA}, se sugiere egresar la medida.',
    'COMUN.PROXIMA_MAYORIA': 'Se hace presente que {PNOMBRE} alcanzará la mayoría de edad el {FECHA_MAYORIA}.',
    'COMUN.CURADOR': 'No registra curador asociado en RUS, se sugiere asociar curador ad litem informáticamente.',
    'COMUN.OIDO': 'Oído con fecha {FECHA_OIDO}.',
    'COMUN.PROX_AUDIENCIA': 'Se cita a audiencia para el día {FECHA_AUDIENCIA}.',
    'ESPERA.E04_RESOLUCION_RECIENTE': 'Medida revisada, a la espera de ingreso efectivo. Se hace presente que el Tribunal ordenó el ingreso efectivo al programa {PROGRAMA} con fecha {FECHA_RESOLUCION}. (Observación administrativa, no requiere acción/respuesta del Tribunal).',
    'ESPERA.E05_SOLO_CORREO': 'Medida revisada, a la espera de ingreso efectivo. Se remite correo electrónico al programa consultando respecto de la fecha estimada de ingreso efectivo.',
    'ESPERA.E05_PROYECTO_Y_CORREO': 'Medida revisada, a la espera de ingreso efectivo. Se remite proyecto de resolución pidiendo cuenta al programa respecto del ingreso efectivo. Igualmente, se remite correo electrónico al programa consultando respecto de la fecha estimada de ingreso efectivo.',
    'CUMPLIMIENTO.C03_INGRESO_RECIENTE': 'Medida revisada. Se hace presente que el ingreso efectivo al programa {PROGRAMA} se registra con fecha {FECHA_INGRESO}.',
    'CUMPLIMIENTO.C04_VENCIDA': 'La medida se visualiza vencida en RUS desde el {FECHA_EGRESO_PROYECTADO}.',
    'CUMPLIMIENTO.C05_VENCE_HOY': 'Se hace presente que la medida se visualiza con vencimiento para el día de hoy, {FECHA_EGRESO_PROYECTADO}.',
    'CUMPLIMIENTO.C05_POR_VENCER': 'Se hace presente que la medida se visualiza próxima a vencer en RUS el {FECHA_EGRESO_PROYECTADO}.',
    'CUMPLIMIENTO.C07_SIN_FICHA': 'No registra ficha individual en RUS, se sugiere confeccionar.',
    'CUMPLIMIENTO.C07_FICHA_ANTIGUA': 'Atendido que la ficha individual registra como última actualización el {FECHA_FICHA_INDIVIDUAL}, superando los 180 días, se sugiere actualizar.',
    'CUMPLIMIENTO.C07_FICHA_RECIENTE': 'Se hace presente que la ficha individual fue actualizada con fecha {FECHA_FICHA_INDIVIDUAL}.',
    'CUMPLIMIENTO.C08_FICHA_FAE': 'Se hace presente que {PNOMBRE} no tiene ficha FAE, se sugiere confeccionar.',
    'CUMPLIMIENTO.C10_HOJA2': 'Medida revisada, se hace presente que el programa {PROGRAMA} deberá remitir informe de avance a más tardar el {FECHA_VENCIMIENTO}.',
    'INFORMES.I01_VENCIDO_DCE': 'Se remite correo electrónico al programa {PROGRAMA} a fin de requerir el informe diagnóstico que se encuentra vencido en RUS desde el {FECHA_VENCIMIENTO}.',
    'INFORMES.I01_VENCIDO_GENERAL': 'Se remite correo electrónico al programa {PROGRAMA} a fin de requerir el informe de avance que se encuentra vencido en RUS desde el {FECHA_VENCIMIENTO}.',
    'INFORMES.I02_POR_VENCER_DCE': 'Se remite correo electrónico al programa {PROGRAMA} a fin de señalar que el informe diagnóstico ordenado en autos debe ser remitido a más tardar el {FECHA_VENCIMIENTO}.',
    'INFORMES.I02_POR_VENCER_GENERAL': 'Se remite correo electrónico al programa {PROGRAMA} a fin de señalar que el próximo informe de avance vence el {FECHA_VENCIMIENTO}.',
}
LEGACY_CORREO_BODIES = {
    'espera': 'Buen día:\n\nJunto con saludar, se informa que en la pestaña Espera del módulo RUS en SITFA se revisaron todos los registros correspondientes a {ALCANCE_MODALIDADES} y se registraron observaciones en bitácora.\n\nAtte. a Ud.,',
    'cumplimiento': 'Buen día:\n\nJunto con saludar, se informa que en la pestaña Cumplimiento del módulo RUS en SITFA se revisaron todos los registros correspondientes a {ALCANCE_MODALIDADES} y se registraron observaciones en bitácora.\n\nAtte. a Ud.,',
    'informes': 'Buen día:\n\nJunto con saludar, se informa que en la pestaña Informes del módulo RUS en SITFA se revisaron todos los registros correspondientes a {ALCANCE_MODALIDADES} y se registraron observaciones en bitácora.\n\nAtte. a Ud.,',
    'medidas': 'Buen día:\n\nJunto con saludar, se remite archivo Excel adjunto con nómina de medidas sin vigencia y por vencer al día {FECHA}, correspondiente a {ALCANCE_MODALIDADES}.\n\nAtte. a Ud.,',
    'programa_por_vencer': 'Buen día:\n\nJunto con saludar, se envía planilla de causas cuyos informes se encuentran próximos a vencer, correspondientes a {PROGRAMA} y {TRIBUNAL}. Se solicita tener presentes las fechas indicadas y acusar recibo de la información.\n\nAtentamente,',
}

def user_directory():
    return Path(os.environ.get('LOCALAPPDATA') or Path.home()) / 'CSMP_Personal'

def atomic_json(path, data):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(dir=path.parent, suffix='.tmp')
    try:
        with os.fdopen(fd,'w',encoding='utf-8') as stream:
            json.dump(data,stream,ensure_ascii=False,indent=2,default=str)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name,path)
    finally:
        Path(name).unlink(missing_ok=True)

def emails(value):
    result=[]
    for part in re.split(r'[;,\n]+', str(value or '')):
        part=part.strip()
        if part and part.lower() not in {s.lower() for s in result}:
            if not re.fullmatch(r"[^\s@<>]+@[^\s@<>]+\.[^\s@<>]+",part):
                raise ValueError('Dirección inválida: '+part)
            result.append(part)
    return result

def defaults():
    text=json.loads((BASE/'textos_base.json').read_text(encoding='utf-8'))
    mail=json.loads((BASE/'correos_base.json').read_text(encoding='utf-8'))
    mail['tribunales']['LAJA']['nombre']='Jgdo. L. y G. de Laja'
    mail['tribunales']['MULCHEN']['nombre']='Jgdo. L. y G. de Mulchén'
    mail['tribunales']['TOME']['nombre']='Juzgado de Familia de Tomé'
    for kind,title in [('programa_espera','Lista de espera'),('programa_vencido','Informes pendientes'),('programa_por_vencer','Informes por vencer')]:
        body={
            'programa_espera':'Junto con saludar, se acompaña nómina de registros en lista de espera de {PROGRAMA}, correspondientes a {TRIBUNAL}. Se solicita informar la fecha estimada de ingreso efectivo y el estado de las gestiones realizadas.',
            'programa_vencido':'Junto con saludar, se acompaña nómina de informes que figuran pendientes de entrega en RUS, correspondientes a {PROGRAMA} y {TRIBUNAL}. Se solicita informar su estado y remitir los antecedentes que correspondan, o precisar si ya fueron presentados.',
            'programa_por_vencer':'Junto con saludar, se envía planilla con RIT de causas en las cuales, informes de avances se encuentran pronto a vencer. Se ruega acusar recibo de la información.'}[kind]
        closing='Atte. a Ud.,' if kind=='programa_por_vencer' else 'Atentamente,'
        mail['plantillas'][kind]={'nombre':title+' · programa','asunto':title+' — {PROGRAMA} — {TRIBUNAL}', 'cuerpo':'Buen día:\n\n'+body+'\n\n'+closing,'adjunto':'obligatorio','usa_modalidades':False,'fuente':'Manual CSMP: Espera, Informes e instrucciones generales, páginas 7, 13 y 14.'}
    raw=(BASE/'contactos_base.json').read_text(encoding='utf-8').strip()
    records=json.loads(raw if raw.startswith('[') else '['+raw.rstrip(',')+']')
    contacts={};ambiguous=set()
    for record in records:
        name=record['Nombre'].strip();address=record.get('Mail','').strip()
        try:emails(address)
        except ValueError:continue
        if name in contacts and contacts[name]!=address:ambiguous.add(name)
        else:contacts[name]=address
    for name in ambiguous:contacts.pop(name,None)
    aliases={a['alias']:a['nombre_catastro'] for a in json.loads((BASE/'aliases_base.json').read_text(encoding='utf-8'))}
    return {'version':1,'revision_textos':3,'revision_correos':CORREO_REVISION,'perfil':'Asistente v9.1; prioridad confirmada por usuario',
            'umbrales':deepcopy(UMBRALES),'desactivadas':[], 'textos':text,
            'correos':mail,'contactos':contacts,'aliases':aliases,'cuenta_outlook':'','firma':''}

def validate(cfg):
    if cfg.get('version')!=1:raise ValueError('Versión de configuración no admitida.')
    for key in UMBRALES:
        v=cfg['umbrales'].get(key)
        if type(v) is not int or not 0<=v<=730:raise ValueError('Umbral inválido: '+key)
    if cfg['umbrales']['proyecto_tome']<cfg['umbrales']['espera_tome']:
        raise ValueError('El umbral de proyecto Tomé no puede anteceder al correo.')
    base=defaults()['textos']
    for scope,entries in base.items():
        if scope.startswith('_'):continue
        for key,value in entries.items():
            actual=cfg['textos'][scope][key]['texto']
            old_fields={f for _,f,_,_ in Formatter().parse(value['texto']) if f}
            new_fields={f for _,f,_,_ in Formatter().parse(actual) if f}
            if new_fields != old_fields or not actual.strip():raise ValueError('Variables de observación inválidas: '+scope+'.'+key)
    allowed={'COMUN.CURADOR','COMUN.OIDO','COMUN.PROX_AUDIENCIA','COMUN.PROXIMA_MAYORIA'}
    if not set(cfg['desactivadas'])<=allowed:raise ValueError('Solo pueden desactivarse advertencias complementarias.')
    for tpl in cfg['correos']['plantillas'].values():
        for key in ('asunto','cuerpo'):
            fields={f for _,f,_,_ in Formatter().parse(tpl[key]) if f}
            if not fields<=VARIABLES:raise ValueError('Variable de correo desconocida: '+str(fields-VARIABLES))
        if tpl['adjunto'] not in ('obligatorio','opcional'):raise ValueError('Requisito de adjunto inválido.')
    for contact in cfg['contactos'].values():emails(contact)
    for court in cfg['correos']['tribunales'].values():emails(';'.join(court['para']))
    emails(cfg['correos'].get('cc_adicional',''))

class Configuration:
    def __init__(self,directory=None):
        self.directory=Path(directory) if directory else user_directory()
        self.path=self.directory/'configuracion.json'
        if not self.path.exists():self.save(defaults())
        try:self.data=json.loads(self.path.read_text(encoding='utf-8'));validate(self.data)
        except Exception as exc:raise ValueError(f'Configuración no utilizable: {self.path}. Conserva el archivo y recupera su copia .bak. Detalle: {exc}') from exc

        if self.data.get('revision_textos',1)<2:
            updated=deepcopy(self.data)
            base=defaults()['textos']
            for scope,entries in base.items():
                if scope.startswith('_'):continue
                for key,entry in entries.items():
                    original=entry['texto']
                    previous=original.replace('Se remite correo electrónico','Se sugiere remitir correo electrónico').replace('se remite correo electrónico','se sugiere remitir correo electrónico').replace('Se remite proyecto de resolución','Se sugiere preparar proyecto de resolución')
                    if updated['textos'][scope][key]['texto']==previous:
                        updated['textos'][scope][key]['texto']=original
            updated['revision_textos']=2
            self.save(updated)

        if self.data.get('revision_textos',1)<3:
            updated=deepcopy(self.data)
            canonical=defaults()['textos']
            for compound,previous in LEGACY_OBSERVATION_TEXTS.items():
                scope,key=compound.split('.',1)
                current=updated.get('textos',{}).get(scope,{}).get(key,{}).get('texto')
                if current==previous:
                    updated['textos'][scope][key]['texto']=canonical[scope][key]['texto']
            updated['revision_textos']=3
            self.save(updated)

        if self.data.get('revision_correos',0)<CORREO_REVISION:
            updated=deepcopy(self.data)
            canonical=defaults()['correos']['plantillas']
            templates=updated['correos']['plantillas']
            # Completa únicamente plantillas ausentes. Las existentes se respetan,
            # salvo cuerpos predeterminados antiguos conocidos que sí deben migrar.
            for key,canonical_template in canonical.items():
                if key not in templates:
                    templates[key]=deepcopy(canonical_template)
                    continue
                previous_body=LEGACY_CORREO_BODIES.get(key)
                if previous_body and templates[key].get('cuerpo')==previous_body:
                    templates[key]['cuerpo']=canonical_template['cuerpo']
            updated['revision_correos']=CORREO_REVISION
            self.save(updated)

        # Actualiza matrices empaquetadas una sola vez por revisión. Si existían
        # versiones distintas, conserva respaldo antes de instalar la nueva.
        from .template_package import install_bundled_templates
        install_bundled_templates(BASE/'plantillas_word',self.directory/'plantillas_word')

    def save(self,data):
        validate(data)
        if self.path.exists():shutil.copyfile(self.path,self.path.with_suffix('.bak'))
        atomic_json(self.path,data)
        self.data=deepcopy(data)
