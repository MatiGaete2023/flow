"""WP-41: barra «Siguiente paso» con los mismos conteos de cada página."""

from nurus.personal.next_step import counts, text
from test_integral_ui import run_gui


def test_texto_sin_trabajo():
    assert counts(None, [], []) == {'revisar': 0, 'exportar': False, 'correos': 0, 'resoluciones': 0, 'resultados': 0}
    assert text(counts(None, [], [])) == (
        '1 Revisar (0) → 2 Exportar copia → 3 Correos (0) → 4 Resoluciones (0) → 5 Resultados (0)'
    )


def test_conteos_coinciden_con_cada_pagina_y_se_actualizan(tmp_path):
    run_gui(
        tmp_path,
        """
import re,time
from nurus.personal.outputs import Draft
from nurus.personal.resolutions import prepare_projects, automatic_project_selections
bar=app.next_step
assert bar.values['revisar']==0 and not bar.values['exportar']
app.work=Work(app.cfg.data).analyze(source(root),'ESPERA',as_of=date(2026,10,2));app._show_work();app.update()
assert bar.values['revisar']==int(app.incident_text.get().split()[0])==len(app.work.rows)
app.work.export(root/'copia.xlsx',backend='portable',reduced_fidelity=True)
projects,_=prepare_projects(app.work,automatic_project_selections(app.work,'PC_IE'),app.template_dir)
app.projects=projects
app._display_prepared_drafts([Draft('Uno','Cuerpo',to='a@example.test'),Draft('Dos','',to='')],'{count} preparados')
fin=time.monotonic()+5
while time.monotonic()<fin and not bar.values['exportar']:
    app.update();time.sleep(0.05)
assert bar.values['exportar']
app.refresh_pending(capture=False)
total,_,por_corregir=map(int,re.findall(r'(\\d+) ',app.results_summary.get())[:3])
from nurus.personal.product_checks import current_drafts
assert bar.values['correos']==len(current_drafts(app))==2
assert bar.values['resoluciones']==len(app.projects)
assert bar.values['resultados']==por_corregir>=1
assert total>=bar.values['correos']+bar.values['resoluciones']
assert bar.buttons['correos'].cget('text')=='3 Correos (2)'
assert bar.buttons['exportar'].cget('text')=='2 Exportar copia ✓'
for clave,pagina in [('correos','Correos'),('resoluciones','Resoluciones'),('resultados','Resultados'),('revisar','Trabajo')]:
    bar.buttons[clave].invoke();app.update()
    assert app.tabs.select()==str(app.pages[pagina])
assert bar._after is not None
""",
    )


def test_tick_con_error_no_se_repite_ni_detiene_la_barra(tmp_path):
    run_gui(
        tmp_path,
        """
bar=app.next_step
llamadas=[]
def falla(force=False):
    llamadas.append(1);raise OSError('ruta de red no disponible')
bar.refresh=falla
bar._tick();bar._tick()
assert len(llamadas)==2 and isinstance(bar.error,OSError) and bar._after is not None
assert 'No se pudieron actualizar' in app.status.get()
""",
    )


def test_cambios_de_destinatario_y_archivo_actualizan_los_pasos(tmp_path):
    run_gui(
        tmp_path,
        """
from nurus.personal.outputs import Draft
app.work=Work(app.cfg.data).analyze(source(root),'ESPERA',as_of=date(2026,10,2))
archivo=root/'adjunto.txt';archivo.write_text('contenido ficticio')
borrador=Draft('Asunto','Cuerpo',to='correo-invalido',attachments=[str(archivo)])
app.drafts=[borrador];bar=app.next_step
bar.refresh(force=True);assert bar.values['resultados']==1
app.refresh_pending(capture=False)
assert app.results_checks['draft:'+borrador.product_id].state=='Por corregir'
assert 'Dirección inválida' in app.results_checks['draft:'+borrador.product_id].issues[0]
borrador.to='equipo@example.test'
assert bar.refresh() and bar.values['resultados']==0
borrador.cc='cc-invalido'
assert bar.refresh() and bar.values['resultados']==1
borrador.cc='ucc_concepcion@pjud.cl'
assert bar.refresh() and bar.values['resultados']==0
archivo.unlink()
assert bar.refresh() and bar.values['resultados']==1
archivo.write_text('nuevo contenido ficticio')
assert bar.refresh() and bar.values['resultados']==0
from nurus.personal.outputs import draft_fingerprint
clave=draft_fingerprint(borrador)
app.work.receipts[clave]={'state':'created'}
assert bar.refresh() and bar.values['resultados']==0
app.work.receipts[clave]['state']='uncertain'
assert bar.refresh() and bar.values['resultados']==1
assert not bar.refresh()
""",
    )


def test_editar_proyecto_recalcula_campos_pendientes(tmp_path):
    run_gui(
        tmp_path,
        """
from nurus.personal.resolutions import Project
app.work=Work(app.cfg.data).analyze(source(root),'ESPERA',as_of=date(2026,10,2))
proyecto=Project('ficticio','Tribunal ficticio','X-1-2026','PC_IE',[],'',{},'Texto revisado','Texto revisado')
app.projects=[proyecto];bar=app.next_step
bar.refresh(force=True);assert bar.values['resoluciones']==1 and bar.values['resultados']==0
proyecto.text='{{CAMPO_PENDIENTE}}'
assert bar.refresh() and bar.values['resultados']==1
proyecto.text='Texto revisado'
assert bar.refresh() and bar.values['resultados']==0
assert not bar.refresh()
""",
    )


def test_un_error_transitorio_no_guarda_la_firma_sin_recalcular(tmp_path):
    run_gui(
        tmp_path,
        """
from unittest.mock import patch
import nurus.personal.next_step as modulo
bar=app.next_step
app.work=Work(app.cfg.data).analyze(source(root),'ESPERA',as_of=date(2026,10,2))
original=modulo.counts
with patch.object(modulo,'counts',side_effect=OSError('ruta temporalmente inaccesible')):
    bar._tick()
assert isinstance(bar.error,OSError)
with patch.object(modulo,'counts',wraps=original) as recalcular:
    bar._tick()
    assert recalcular.call_count==1
assert bar.error is None and bar.values['revisar']==len(app.work.rows)
""",
    )
