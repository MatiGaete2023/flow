from copy import deepcopy
from types import SimpleNamespace
from pathlib import Path
import tomllib

from test_personal_flow import exported
from nurus.personal.config import defaults, UMBRALES, PARAMETER_LABELS, Configuration
from nurus.personal.outputs import Draft, create_drafts, prepare_drafts
from nurus.personal.app_base import App


def test_custom_mail_uses_selected_records_without_motor_rule(tmp_path):
    work=exported(tmp_path)
    work.config['correos']['plantillas']['particular_prueba']={
        'nombre':'Mi comunicación','asunto':'Consulta — {TRIBUNAL}',
        'cuerpo':'Registros:\n{TABLA_REGISTROS}', 'adjunto':'opcional','usa_modalidades':False,
    }
    work.rows[0].actions=[]
    drafts=prepare_drafts(work,'particular_prueba',selected=[work.rows[0].id])
    assert len(drafts)==1 and 'Persona Ejemplo' in drafts[0].body
    assert prepare_drafts(work,'particular_prueba',selected=[])==[]
    # No amplía las comunicaciones automáticas a registros sin la acción pertinente.
    assert prepare_drafts(work,'programa_espera')==[]


def test_invalid_address_does_not_abort_remaining_batch(tmp_path,monkeypatch):
    work=exported(tmp_path);saved=[]
    def save(product,**kwargs):
        saved.append(product)
        return SimpleNamespace(entry_id=str(len(saved)),store_id='s')
    monkeypatch.setattr('nurus.personal.outputs.save_draft',save)
    drafts=[Draft('Inválido','Texto',to='sin-direccion'),Draft('Correcto','Texto'),Draft('Otro','Texto')]
    result=create_drafts(work,drafts)
    assert result['created']==2 and len(result['errors'])==1
    assert [p.rendered_subject for p in saved]==['Correcto','Otro']
    again=create_drafts(work,drafts)
    assert again['created']==0 and again['skipped']==2


def test_failed_attachment_does_not_abort_remaining_batch(tmp_path,monkeypatch):
    work=exported(tmp_path)
    monkeypatch.setattr('nurus.personal.outputs.save_draft',lambda *a,**k:SimpleNamespace(entry_id='e',store_id='s'))
    result=create_drafts(work,[Draft('Falta archivo','Texto',attachments=[str(tmp_path/'no.xlsx')]),Draft('Sin adjunto','Texto')])
    assert result['created']==1 and len(result['errors'])==1


class Var:
    def __init__(self,value):self.value=value
    def get(self):return self.value
    def set(self,value):self.value=value


class Editor:
    def __init__(self,text):self.text=text
    def get(self,*args):return self.text


def test_save_text_uses_loaded_rule_not_new_combobox_selection(tmp_path):
    cfg=Configuration(tmp_path/'cfg')
    keys=[s+'.'+k for s,entries in cfg.data['textos'].items() if not s.startswith('_') for k in entries]
    first,second=keys[:2];scope,key=first.split('.')
    text=cfg.data['textos'][scope][key]['texto']+' Texto adicional.'
    app=SimpleNamespace(cfg=cfg,_editing_text_key=first,text_key=Var(second),text_editor=Editor(text))
    App._save_text(app,notify=False)
    assert Configuration(cfg.directory).data['textos'][scope][key]['texto']==text
    assert cfg.data['textos'][second.split('.')[0]][second.split('.')[1]]==defaults()['textos'][second.split('.')[0]][second.split('.')[1]]


def test_invalid_edit_does_not_switch_or_discard_editor(monkeypatch):
    app=SimpleNamespace(_editing_text_key='old');var=Var('new');errors=[]
    def save(**kwargs):raise ValueError('Variable faltante')
    monkeypatch.setattr('nurus.personal.app_base.messagebox.showerror',lambda *args:errors.append(args))
    assert not App._save_before_switch(app,'_editing_text_key',var,save)
    assert var.get()=='old' and errors


def test_generation_reprepares_when_selection_changes():
    calls=[]
    words=SimpleNamespace(selection=lambda:('b|PC_INFO',),get_children=lambda:('a|PC_INFO','b|PC_INFO'))
    app=SimpleNamespace(_require_work=lambda:object(),words=words,projects=[object()],_prepared_selection=('a|PC_INFO',),_prepare_words=lambda **kw:calls.append(kw))
    App._generate_words(app)
    assert calls==[{'then_generate':True}]


def test_labels_cover_operational_thresholds_without_changing_defaults():
    assert set(PARAMETER_LABELS)==set(UMBRALES)
    assert defaults()['umbrales']['espera_laja']==30


def test_legacy_shortcuts_open_only_personal_assistant():
    root=Path(__file__).parents[1]
    for previous,current in [('Abrir_NuRus.bat','Abrir_CSMP.bat'),('Instalar_NuRus.bat','Instalar_CSMP.bat')]:
        text=(root/previous).read_text()
        assert f'call "%~dp0{current}"' in text
        assert 'nurus.app' not in text and '.venv\\' not in text
    scripts=tomllib.loads((root/'pyproject.toml').read_text())['project']['scripts']
    assert scripts['nurus']==scripts['csmp-assistant']=='nurus.personal.app:main'


def test_old_module_entry_opens_personal_assistant(monkeypatch):
    import nurus.app
    calls=[]
    monkeypatch.setattr('nurus.personal.app.main',lambda:calls.append('personal'))
    nurus.app.main()
    assert calls==['personal']


def test_resolution_labels_are_readable_but_codes_stay_stable():
    from nurus.personal.resolutions import KIND_LABELS,kind_code,kind_label
    assert kind_label('PC_IE')==KIND_LABELS['PC_IE']
    assert 'ingreso efectivo' in kind_label('PC_IE').lower()
    assert 'informe' in kind_label('PC_INFO').lower()
    assert 'nomenclatura' in kind_label('NOMENCL').lower()
    assert kind_code(KIND_LABELS['PC_INFO'])=='PC_INFO'


def test_context_bar_describes_shared_active_work():
    context=Var('')
    app=SimpleNamespace(
        work=SimpleNamespace(path='C:/trabajo/entrada.xlsx',mode='ESPERA',rows=[1,2,3],output='C:/salidas/revisable.xlsx'),
        context=context,
    )
    App._update_context(app)
    assert 'ESPERA' in context.get()
    assert 'entrada.xlsx' in context.get()
    assert '3 registros' in context.get()
    assert 'revisable.xlsx' in context.get()


def test_open_output_folder_prefers_current_copy(tmp_path):
    opened=[]
    output=tmp_path/'salidas'/'revisable.xlsx'
    app=SimpleNamespace(work=SimpleNamespace(output=str(output)),folder=Var(str(tmp_path/'otra')),_open=lambda path:opened.append(Path(path)))
    App._open_output_folder(app)
    assert output.parent.is_dir()
    assert opened==[output.parent]
