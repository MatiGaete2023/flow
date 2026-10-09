"""WP-42: Resoluciones en tres grupos más «Más»; Correos explica cada botón deshabilitado."""
from test_integral_ui import run_gui

BOTONES = """
def textos(widget):
    out=[]
    for child in widget.winfo_children():
        try:out.append(child.cget('text'))
        except Exception:pass
        out+=textos(child)
    return [t for t in out if t]
"""


def test_resoluciones_en_tres_grupos_y_mas(tmp_path):
    run_gui(tmp_path, BOTONES + """
grupos=app.resolution_groups
assert list(grupos)==['Preparar','Revisar','Generar']
assert {'Agregar desde Trabajo','Preparar / actualizar proyectos'}<=set(textos(grupos['Preparar']))
assert {'Aplicar tipo','Restaurar automático'}<=set(textos(grupos['Revisar']))
assert {'Generar Word','Abrir Word'}<=set(textos(grupos['Generar']))
assert set(textos(app.resolution_more_frame))>={'Word manual','Quitar del lote'}
assert not app.resolution_more_frame.winfo_manager()
app.resolution_more();app.update()
assert app.resolution_more_frame.winfo_manager()=='pack'
visibles=set(textos(app.pages['Resoluciones']))
for accion in ('Aplicar tipo','Agregar desde Trabajo','Quitar del lote','Preparar / actualizar proyectos','Word manual',
               'Restaurar automático','Generar Word','Abrir Word'):
    assert accion in visibles, accion
""")


def test_correos_muestra_motivo_con_y_sin_excel(tmp_path):
    run_gui(tmp_path, """
from nurus.personal.mail_controls import MAIL_BUTTON_REASONS
from nurus.personal.outputs import prepare_drafts
from reglas_rt_util import analizar, persona
app.tabs.select(app.pages['Correos']);app.update()
estado={k:(b.cget('state'),r.get()) for k,(b,r) in app.mail_gates.items()}
assert estado['preparar_tipo']==('disabled','Carga un Excel en Trabajo.'),estado
assert estado['preparar_todos']==('disabled','Carga un Excel en Trabajo.'),estado
assert estado['guardar_borradores']==('disabled','Prepara correos antes de guardarlos.'),estado
work=analizar(root,'ESPERA',['RIT','TRIBUNAL','NOMBRE','RUT','NOMBRE CENTRO','T ESPERA'],[persona(1)+[45]])
app.work=work;app.refresh_mail_buttons()
estado={k:(b.cget('state'),r.get()) for k,(b,r) in app.mail_gates.items()}
assert estado['preparar_tipo']==('normal','') and estado['preparar_todos']==('normal','')
assert estado['guardar_borradores'][0]=='disabled'
app.drafts=prepare_drafts(work,'programa_espera');assert app.drafts
app.refresh_mail_buttons()
assert app.mail_gates['guardar_borradores'][0].cget('state')=='normal'
assert app.mail_gates['guardar_borradores'][1].get()==''
for clave,(boton,motivo) in app.mail_gates.items():
    assert (boton.cget('state')=='disabled')==bool(MAIL_BUTTON_REASONS[clave](app))
""")
