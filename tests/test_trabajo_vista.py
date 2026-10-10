"""WP-40: Trabajo legible (orden de columnas, observación completa, editor plegado, excluidas)."""
from test_integral_ui import run_gui


def test_columnas_editor_plegado_tooltip_excluidas_y_atajos(tmp_path):
    run_gui(tmp_path, """
from nurus.personal.work_view import WORK_COLUMNS
assert WORK_COLUMNS==('Estado','Observación','RIT','Nombre','Programa','Tribunal')
assert tuple(app.records['columns'])==WORK_COLUMNS
atajos=('<Control-s>','<Control-o>','<Control-f>','<F5>','<Control-k>')
assert all(app.bind_all(sequence) for sequence in atajos)
assert app.observation_editor._textbox.bind('<Control-Return>')
assert not app.work_editor_open()
app.work=Work(app.cfg.data).analyze(source(root),'ESPERA',as_of=date(2026,10,2));app._show_work()
app.tabs.select(app.pages['Trabajo']);app.update()
assert not app.work_editor_open()
fila=app.work.rows[0]
assert app.records.set(fila.id,'Observación')==fila.review.get('OBSERVACION',fila.observation)
assert app.records.set(fila.id,'RIT') and app.records.set(fila.id,'Tribunal')
app.records.selection_set(fila.id);app.update()
assert app.work_editor_open()
app.update_idletasks();app.update()
assert app.observation_editor.winfo_ismapped() and app.observation_editor.winfo_height()>=120
app.update()
caja=app.records.bbox(fila.id,'Observación')
assert caja, 'celda de observación visible'
x,y=caja[0]+caja[2]//2,caja[1]+caja[3]//2
celda,texto=app.observation_tip.text_at(x,y)
assert celda and texto==app.records.set(fila.id,'Observación')
nada,_=app.observation_tip.text_at(sum(app.records.bbox(fila.id,'RIT')[0:3:2])-8,caja[1]+caja[3]//2)
assert nada is None
app.records.configure(displaycolumns=('RIT','Observación'));app.update()
caja=app.records.bbox(fila.id,'Observación')
assert app.observation_tip.text_at(caja[0]+caja[2]//2,caja[1]+caja[3]//2)[1]==texto
assert str(app.records.tag_configure('excluded','foreground'))=='#7a7a7a'
assert str(app.records.tag_configure('excluded','background'))=='#eeeeee'
assert all(app.bind_all(sequence) for sequence in atajos)
""")


def test_preferencia_antigua_respeta_el_orden_nuevo(tmp_path):
    run_gui(tmp_path, """
from nurus.personal.work_view import build
cfg=dict(app.cfg.data);cfg['vista']={'columnas_trabajo':['Estado','RIT','Nombre','Tribunal','Programa','Observación']}
app.cfg.save(cfg)
for child in app.pages['Trabajo'].winfo_children():child.destroy()
build(app);app.update()
assert tuple(app.records['displaycolumns'])==('Estado','Observación','RIT','Nombre','Programa','Tribunal')
from nurus.personal.commands import commands
assert any('Preferencias de vista' in c.name for c in commands(app))
""")
