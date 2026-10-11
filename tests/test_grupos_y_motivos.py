"""WP-42: Resoluciones en tres grupos y motivos visibles en los botones de Correos."""

from test_integral_ui import run_gui

COMUN = """
def textos(widget):
    out=[]
    for child in widget.winfo_children():
        try:out.append(child.cget('text'))
        except Exception:pass
        out+=textos(child)
    return [t for t in out if isinstance(t,str)]
"""


def test_resoluciones_tres_grupos_y_mismas_acciones(tmp_path):
    run_gui(
        tmp_path,
        COMUN
        + """
assert list(app.resolution_groups)==['1 Preparar','2 Revisar','3 Generar','Más']
grupo=lambda nombre:set(textos(app.resolution_groups[nombre]))
assert {'Aplicar tipo','Agregar desde Trabajo','Preparar / actualizar proyectos'}<=grupo('1 Preparar')
assert {'Vista PDF','Restaurar automático'}<=grupo('2 Revisar')
assert {'Generar Word','Abrir Word'}<=grupo('3 Generar')
menu=app.resolution_more_menu
assert [menu.entrycget(i,'label') for i in range(menu.index('end')+1)]==['Word manual','Quitar del lote']
antes={'Aplicar tipo','Agregar desde Trabajo','Preparar / actualizar proyectos','Restaurar automático','Generar Word',
       'Abrir Word','Vista PDF del Word generado','Confirmar revisión de cambios','Aplicar datos al proyecto','Abrir matriz en Word'}
assert antes<=set(textos(app.pages['Resoluciones']))
app.tabs.select(app.pages['Resoluciones']);app.update()
for linea in {w.master.master for w in app.resolution_groups.values()}:
    assert linea.winfo_reqwidth()<=app.pages['Resoluciones'].winfo_width()+2, linea.winfo_reqwidth()
""",
    )


def test_correos_muestra_motivo_con_y_sin_excel(tmp_path):
    run_gui(
        tmp_path,
        COMUN
        + """
from nurus.personal.mail_controls import MAIL_ACTIONS, NO_WORK, NO_SELECTED, mail_block, require_mail_action
from nurus.personal.mail_view import SHORT_REASONS
from nurus.personal.outputs import Draft
assert set(app.mail_action_buttons)==set(MAIL_ACTIONS)
app.tabs.select(app.pages['Correos']);app.update()
def revisar():
    for clave,boton in app.mail_action_buttons.items():
        motivo=mail_block(app,clave);etiqueta,tip=app.mail_action_reasons[clave]
        assert (boton.cget('state')=='disabled')==bool(motivo),clave
        assert etiqueta.cget('text')==SHORT_REASONS.get(motivo,motivo) and tip.text==motivo,clave
        assert bool(etiqueta.winfo_manager())==bool(motivo),clave
        if motivo:
            try:require_mail_action(app,clave);raise AssertionError(clave)
            except ValueError as error:assert str(error)==motivo
sin_excel={clave:mail_block(app,clave) for clave in MAIL_ACTIONS}
assert sin_excel['preparar_tipo']==sin_excel['preparar_todos']==sin_excel['guardar_todos']==NO_WORK
assert sin_excel['vista_previa']==NO_SELECTED and all(sin_excel.values())
revisar()
app.work=Work(app.cfg.data).analyze(source(root),'ESPERA',as_of=date(2026,10,2))
app.work.export(root/'copia.xlsx',backend='portable',reduced_fidelity=True);app._show_work();app.update()
assert not mail_block(app,'preparar_tipo') and not mail_block(app,'preparar_todos')
assert mail_block(app,'guardar_todos')
revisar()
app.mail_target.set('todos')
app._display_prepared_drafts([Draft('Uno','Cuerpo',to='a@example.test',program='Programa ficticio')],'{count} preparados');app.update()
assert len(app.drafts)==1 and app.draft_index==0
assert not mail_block(app,'guardar_todos') and not mail_block(app,'vista_previa') and not mail_block(app,'guardar_este')
assert mail_block(app,'editar_adjunto')
revisar()
assert {'Correo libre','Cargar planilla','Adjuntar…','Filtros / opciones'}<=set(textos(app.pages['Correos']))
""",
    )


def test_render_por_trozos_no_deja_botones_con_estado_viejo(tmp_path):
    # Con más de 500 filas la tabla se dibuja en trozos con la interfaz bloqueada;
    # al desbloquear se recalcula la disponibilidad (antes quedaban deshabilitados).
    run_gui(
        tmp_path,
        COMUN
        + """
import time
from openpyxl import Workbook
from nurus.personal.mail_controls import mail_block
from nurus.personal.outputs import Draft
libro=Workbook();hoja=libro.active;hoja.title='ESPERA'
hoja.append(['RIT','TRIBUNAL','NOMBRE','RUT','NOMBRE CENTRO','T ESPERA'])
for i in range(700):hoja.append([f'X-{i+1}-2026','Tribunal ficticio 777',f'Persona {i}','11111111-1',f'PRM Centro {i%7}',40])
libro.save(root/'grande.xlsx')
trabajo=Work(app.cfg.data).analyze(root/'grande.xlsx','ESPERA',as_of=date(2026,10,2))
trabajo.export(root/'copia.xlsx',backend='portable',reduced_fidelity=True)
app.mail_target.set('todos');app.work=trabajo;app._show_work()
app._display_prepared_drafts([Draft('Uno','Cuerpo',to='a@example.test',program='Programa ficticio')],'{count} recuperados')
fin=time.monotonic()+20
while time.monotonic()<fin and getattr(app,'rendering',False):
    app.update();time.sleep(0.02)
app.update()
assert not getattr(app,'rendering',False)
for clave in ('vista_previa','guardar_este','guardar_todos','preparar_todos','preparar_tipo'):
    assert mail_block(app,clave)=='' and app.mail_action_buttons[clave].cget('state')=='normal',clave
""",
    )


def test_paleta_usa_la_misma_condicion_que_el_boton_guardar_borradores(tmp_path):
    run_gui(
        tmp_path,
        """
from nurus.personal.commands import commands
from nurus.personal.mail_controls import mail_block
comando=next(c for c in commands(app) if c.name.startswith('Revisar y guardar borradores'))
app.drafts=[object()];app.work=None
assert mail_block(app,'guardar_todos') and not comando.available()
""",
    )
