"""WP-40: tabla de Trabajo legible (orden, texto completo, editor plegado y excluidos)."""
from test_integral_ui import run_gui

ESCENARIO_BASE = """
from nurus.personal.work_view import DISPLAY_ORDER, EXCLUDED_COLORS
from reglas_rt_util import analizar, persona
encabezados=['RIT','TRIBUNAL','NOMBRE','RUT','NOMBRE CENTRO','T ESPERA']
work=analizar(root,'ESPERA',encabezados,[persona(1)+[45],persona(2)+[10]])
work.rows[1].excluded=True
app.work=work;app._show_work();app.update()
tree=app.records
"""


def test_orden_columnas_y_excluidos(tmp_path):
    run_gui(tmp_path, ESCENARIO_BASE + """
assert tuple(tree['displaycolumns'])==('Estado','Observación','RIT','Nombre','Programa','Tribunal')
assert set(tree['columns'])==set(DISPLAY_ORDER)
assert str(tree.tag_configure('excluded','foreground'))==EXCLUDED_COLORS['foreground']
assert str(tree.tag_configure('excluded','background'))==EXCLUDED_COLORS['background']
assert 'excluded' in tree.item(work.rows[1].id,'tags')
""")


def test_editor_plegado_hasta_seleccionar(tmp_path):
    run_gui(tmp_path, ESCENARIO_BASE + """
assert app.work_editor_collapsed and not app.record_tabs.winfo_manager()
assert app.work_editor_hint.winfo_manager()
tree.selection_set(work.rows[0].id);app.update()
assert not app.work_editor_collapsed and app.record_tabs.winfo_manager()=='pack'
assert not app.work_editor_hint.winfo_manager()
assert app.observation_id==work.rows[0].id
""")


def test_texto_completo_de_la_observacion_al_pasar_el_cursor(tmp_path):
    run_gui(tmp_path, ESCENARIO_BASE + """
app.deiconify();app.tabs.select(app.pages['Trabajo']);app.update()
iid=work.rows[0].id
tree.see(iid);app.update()
x,y,width,height=tree.bbox(iid,'Observación')
tip=app.work_observation_tip
assert len(tree.set(iid,'Observación'))>40
assert tip.text_at(x+5,y+3)==(iid,tree.set(iid,'Observación'))
rx,ry=tree.bbox(iid,'RIT')[:2]
assert tip.text_at(rx+5,ry+3)[1]==''
tree.event_generate('<Motion>',x=x+5,y=y+3)
assert tip.tip is not None and tip.key==(iid,tree.set(iid,'Observación'))
tree.event_generate('<Leave>');app.update()
assert tip.tip is None
""")
