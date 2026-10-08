"""Prueba de ventana real en Windows; no inicia Office ni usa archivos del usuario."""
from tempfile import TemporaryDirectory
from pathlib import Path
import base64
import time
from PIL import Image
import customtkinter as ctk
from nurus.personal.outputs import Draft
from nurus.personal.config import Configuration
from nurus.personal.app import App
from nurus.personal.work import Work, Row

def capture_window(app,name):
    import win32gui,win32ui,win32con
    folder=Path('artifacts');folder.mkdir(exist_ok=True)
    hwnd=win32gui.GetAncestor(app.winfo_id(),2)
    left,top,right,bottom=win32gui.GetWindowRect(hwnd)
    handle=win32gui.GetWindowDC(hwnd)
    source=win32ui.CreateDCFromHandle(handle);target=source.CreateCompatibleDC()
    bitmap=win32ui.CreateBitmap();bitmap.CreateCompatibleBitmap(source,right-left,bottom-top)
    target.SelectObject(bitmap)
    try:
        target.BitBlt((0,0),(right-left,bottom-top),source,(0,0),win32con.SRCCOPY)
        bitmap.SaveBitmapFile(target,str(folder/(name+'.bmp')))
        png=folder/(name+'.png')
        Image.open(folder/(name+'.bmp')).save(png)
        print('CSMP_CAPTURE:'+name+':'+base64.b64encode(png.read_bytes()).decode('ascii'))
    finally:
        target.DeleteDC();source.DeleteDC();win32gui.ReleaseDC(hwnd,handle);win32gui.DeleteObject(bitmap.GetHandle())


with TemporaryDirectory() as directory:
    app=App(Configuration(Path(directory)))
    try:
        app.update_idletasks();app.update()
        assert isinstance(app,ctk.CTk)
        assert ctk.get_appearance_mode()=='Dark'
        assert len(app.tabs.tabs())==5
        for tab in app.tabs.tabs():
            app.tabs.select(tab);app.update_idletasks();app.update()
            assert app.nametowidget(tab).winfo_ismapped()
        assert app.template_dir.is_dir()
        assert len(list(app.template_dir.rglob('*.docx')))>=5
        assert len(app.modality_vars)==4
        assert all(v.get() for v in app.modality_vars.values())
        assert app.project_editor.winfo_exists()
        assert app.observation_editor.winfo_exists()
        app.geometry('1024x650');app.update_idletasks();app.update()
        # Las acciones deben estar dentro de la ventana a tamaño de PC institucional.
        app.tabs.select(app.pages['Correos']);app.update_idletasks();app.update()
        app._display_prepared_drafts([Draft('Informes por vencer','Buen día:\n\nTexto de prueba editable.',program='PROGRAMA DE PRUEBA',court='Jgdo. L. y G. de Laja',due='2026-10-15',record_ids=['a']),Draft('Otro borrador','Segundo cuerpo',program='SEGUNDO PROGRAMA')],'{count} borradores de prueba')
        app.mail_list._choose(1);app.update()
        assert app.subject.get()=='Otro borrador' and app.body.get('1.0','end-1c')=='Segundo cuerpo'
        app.mail_list._choose(0);app.update()
        app.body.insert('end',' Edición conservada.')
        app.mail_target.set('tribunales');app._mail_target_changed();app.update()
        assert not app.drafts
        app.mail_target.set('programas');app._mail_target_changed();app.update()
        assert len(app.drafts)==2 and app.body.get('1.0','end-1c').endswith('Edición conservada.')
        app.attach.set('Programa de prueba.xlsx\nSegundo programa.xlsx');app.attachment_chips._remove(0)
        assert app.attach.get()=='Segundo programa.xlsx'
        assert app.mail_target.get()=='programas'
        app.update_idletasks();app.update()
        capture_window(app,'correos-1024x650')
        button=app.save_all_button
        assert button.winfo_ismapped()
        assert button.winfo_rootx()+button.winfo_width() <= app.winfo_rootx()+app.winfo_width()
        assert button.winfo_rooty()+button.winfo_height() <= app.winfo_rooty()+app.winfo_height()
        body_height=app.body.winfo_height()
        assert body_height >= 100, f'Editor de correo demasiado bajo a 1024x650: {body_height}px'
        # Nombres legibles, identidad interna estable y guardado al cambiar de plantilla.
        before=app.tpl_key.get();app.tpl_body.insert('end','\nPrueba de edición conservada.')
        other=next(k for k in app.cfg.data['correos']['plantillas'] if k!=before)
        app.tpl_key.set(other);app._load_tpl()
        assert app.cfg.data['correos']['plantillas'][before]['cuerpo'].endswith('Prueba de edición conservada.')
        app.tpl_key.set(before);app._load_tpl()
        assert app.tpl_body.get('1.0','end-1c').endswith('Prueba de edición conservada.')
        assert app.kind_box.get()==app.cfg.data['correos']['plantillas'][app.mail_kind.get()]['nombre']
        app.tabs.select(app.pages['Configuración']);app.update_idletasks();app.update()
        capture_window(app,'configuracion-1024x650')
        # Trabajo y Resoluciones: contexto persistente, búsqueda, revisión por excepción y tipos legibles.
        work=Work(app.cfg.data);work.mode='ESPERA';work.path='prueba.xlsx'
        work.mapping={'rit':'RIT','tribunal':'TRIBUNAL','nombre':'NOMBRE','rut':'RUT','programa':'DERIVACION'}
        work.rows=[
            Row('a',2,{'RIT':'X-1','TRIBUNAL':'Jgdo. L. y G. de Laja','NOMBRE':'NNA UNO','RUT':'11111111-1','DERIVACION':'AFT PRUEBA'},'Ingreso efectivo',[],['PC_IE'],[],review={'RES':'PC_IE'}),
            Row('b',3,{'RIT':'X-2','TRIBUNAL':'Jgdo. L. y G. de Laja','NOMBRE':'NNA DOS','RUT':'22222222-2','DERIVACION':'AFT PRUEBA'},'Revisión manual',[],[],['Dato a revisar']),
        ]
        app.work=work;app._show_work()
        assert '2 registros' in app.context.get()
        assert 'Pide cuenta ingreso efectivo' in app.words.item('a|PC_IE','values')[2]
        assert 'Definido en RES'==app.words.item('a|PC_IE','values')[3]
        app.work_search.set('NNA UNO');app.update()
        assert app.records.get_children()==('a',)
        app.work_search.set('');app.work_filter.set('Con aviso');app._apply_work_filter();app.update()
        assert app.records.get_children()==('b',)
        app.work_filter.set('Todos');app._apply_work_filter()
        app.resolution_filter.set('Definido en RES');app._apply_resolution_filter();app.update()
        assert app.words.get_children()==('a|PC_IE',)
        app.resolution_filter.set('Todos');app._apply_resolution_filter()
        app.tabs.select(app.pages['Trabajo']);app.update_idletasks();app.update();capture_window(app,'trabajo-1024x650')
        app.tabs.select(app.pages['Resoluciones']);app.update_idletasks();app.update();capture_window(app,'resoluciones-1024x650')
        # La vista real conserva un tipo elegido manualmente al reexportar.
        app.words.selection_set('a|PC_IE')
        app.manual_word.set('NOMENCL');app._assign_word_type()
        app._show_work(reset_projects=False)
        assert app.words.get_children()==('a|NOMENCL',)
        assert app.words.selection()==('a|NOMENCL',)
        assert 'Ajustado manualmente' in app.words.item('a|NOMENCL','values')[3]
        app.to.set('anterior@example.cl');app.body.insert('end','Correo anterior')
        app.project_editor.insert('end','Proyecto anterior');app._clear_drafts()
        assert not app.to.get() and not app.body.get('1.0','end-1c')
        assert not app.project_editor.get('1.0','end-1c')
        app._run('Prueba de progreso real',lambda:True)
        deadline=time.monotonic()+5
        while app.busy and time.monotonic()<deadline:
            app.update();time.sleep(.02)
        assert not app.busy and app.progress.cget('mode')=='determinate' and app.progress.get()==0
        print('Cinco áreas CTk oscuras; búsqueda y filtros; contexto persistente; revisión por excepción; tipos legibles; tarjetas y adjuntos; botones visibles a 1024x650; plantillas y resoluciones conservadas.')
    finally:app.destroy()
