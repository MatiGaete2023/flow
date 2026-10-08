"""Interfaz personal CSMP: una carga, propuestas y preparación de salidas."""
from .selection import selected_ids, remember, restore_visible
from pathlib import Path
from datetime import date, datetime
from dataclasses import replace
from copy import deepcopy
import os
import queue
import shutil
import threading
import tkinter as tk
from tkinter import filedialog, messagebox, simpledialog
import customtkinter as ctk
from . import ui as ttk
from .ui import Textbox as ScrolledText

from .config import Configuration, PARAMETER_LABELS, VARIABLES
from .widgets import ScrollPane, NamedChoice, Tooltip
from .ux_support import case_detail_text, compact_case_detail_text, grouped_draft_detail, incident_ids, resume_available, resume_description, RES_HELP
from .work import Work
from .outputs import create_draft, import_contacts
from .resolutions import KINDS, kind_code, kind_label, prepare_projects, generate_projects, automatic_project_selections
from .importing import SheetChoice
from nurus.rus.reader import list_workbook_sheets
from nurus.rus.columns import normalize

class App(ctk.CTk):
    def __init__(self,configuration=None):
        self.cfg=configuration or Configuration()
        ctk.set_appearance_mode(ttk.THEMES[self.cfg.data.get('vista',{}).get('tema','Sistema')]);ctk.set_default_color_theme('blue')
        super().__init__()
        self.configure(fg_color=ttk.BG);ttk.install_theme(self)
        from nurus import __version__
        self.title('CSMP Assistant personal · '+__version__);self.geometry('1120x720');self.minsize(820,560)
        self.work=None;self.drafts=[];self.draft_index=None;self.report=None;self.busy=False
        self.projects=[];self.project_index=None;self.last_word='';self.observation_id=None
        self.events=queue.Queue()
        self.template_dir=self.cfg.directory/'plantillas_word'
        self.template_dir.mkdir(parents=True,exist_ok=True)
        self._install_templates()
        self.status=tk.StringVar(value='Selecciona un Excel para comenzar.')
        self.context=tk.StringVar(value='Sin trabajo activo · Ctrl+O para elegir un Excel')
        self.grid_columnconfigure(1,weight=1);self.grid_rowconfigure(0,weight=1)
        sidebar=ctk.CTkFrame(self,width=155,corner_radius=0,fg_color=ttk.SIDEBAR)
        sidebar.grid(row=0,column=0,sticky='nsew');sidebar.grid_propagate(False);sidebar.pack_propagate(False)
        ctk.CTkLabel(sidebar,text='CSMP\nAssistant',font=('Segoe UI',22,'bold'),justify='left',text_color=ttk.TEXT).pack(padx=16,pady=(24,18),anchor='w')
        self.tabs=ttk.PageStack(self,sidebar);self.tabs.grid(row=0,column=1,sticky='nsew',padx=12,pady=12)
        self.pages={}
        for name in ('Trabajo','Correos','Resoluciones','Resultados','Configuración'):
            frame=ttk.Frame(self.tabs,padding=10)
            self.tabs.add(frame,text=name);self.pages[name]=frame
        ctk.CTkLabel(sidebar,text='Uso personal\nSolo borradores',text_color=ttk.MUTED,justify='left').pack(side='bottom',padx=16,pady=18)
        self._work_page();self._mail_page();self._word_page();self._config_page()
        from .results_view import build as build_results
        build_results(self)
        contextbar=ctk.CTkFrame(self,corner_radius=0,fg_color=ttk.PANEL)
        contextbar.grid(row=1,column=0,columnspan=2,sticky='ew')
        ctk.CTkLabel(contextbar,textvariable=self.context,anchor='w',text_color=ttk.MUTED,font=('Segoe UI',11)).pack(fill='x',padx=14,pady=4)
        statusbar=ctk.CTkFrame(self,corner_radius=0,fg_color=ttk.SIDEBAR)
        statusbar.grid(row=2,column=0,columnspan=2,sticky='ew');statusbar.grid_columnconfigure(0,weight=1)
        self.status_label=ctk.CTkLabel(statusbar,textvariable=self.status,anchor='w',wraplength=760)
        self.status_label.grid(row=0,column=0,sticky='ew',padx=14,pady=6)
        self.progress=ctk.CTkProgressBar(statusbar,width=120,height=7,mode='determinate')
        self.progress.grid(row=0,column=1,padx=14);self.progress.set(0)
        self.cancel_button=ttk.Button(statusbar,text='Detener',width=90,height=24,command=self._cancel_operation)
        self.cancel_button.grid(row=0,column=2,padx=(0,12));self.cancel_button.configure(state='disabled')
        self.operation_timings=[]
        self._theme_mode=ctk.get_appearance_mode()
        self.after(700,self._watch_theme)
        self.protocol('WM_DELETE_WINDOW',self._close)
        self._bind_shortcuts()
        self.after(100,self._poll)
        self.after(15000,self._autosave)
        saved=self.cfg.directory/'sesion/trabajo.json'
        if saved.exists():
            try:
                recovered=Work.load(saved.parent)
                if resume_available(recovered):
                    self._resume_work=recovered
                    self._show_resume_offer(saved)
                    self.status.set('Hay un trabajo anterior disponible para continuar.')
                else:
                    self.status.set('La sesión anterior existe, pero su copia Excel ya no está disponible. Elige otro archivo.')
            except Exception as exc:self.status.set('No se pudo recuperar el trabajo anterior: '+str(exc))

    def _install_templates(self):
        source=Path(__file__).parent/'plantillas_word'
        if source.exists():
            for path in source.rglob('*.docx'):
                dst=self.template_dir/path.relative_to(source);dst.parent.mkdir(parents=True,exist_ok=True)
                if not dst.exists():shutil.copyfile(path,dst)

    def _run(self,label,action,done=None):
        if self.busy or getattr(self,'rendering',False):self.status.set('Hay una operación en curso.');return
        from .interaction import lock
        from .operations import Operation,operation_scope
        from time import perf_counter
        lock(self)
        self.busy=True;self.status.set(label);self.progress.configure(mode='indeterminate');self.progress.start()
        self.cancel_button.configure(state='normal')
        self.operation=Operation(self.events.put)
        def worker():
            started=perf_counter()
            try:
                with operation_scope(self.operation):result=action()
                self.events.put((True,result,done))
            except Exception as exc:self.events.put((False,exc,None))
            finally:self.events.put(('timing',label,perf_counter()-started,None))
        threading.Thread(target=worker,daemon=True).start()

    def _cancel_operation(self):
        if self.busy and getattr(self,'operation',None):
            self.operation.cancelled.set()
            self.status.set('Detención solicitada. Se completará la escritura actual antes de detener el resto.')
            self.cancel_button.configure(state='disabled')

    def _watch_theme(self):
        mode=ctk.get_appearance_mode()
        if mode!=self._theme_mode:
            self._theme_mode=mode;ttk.install_theme(self)
        self.after(700,self._watch_theme)

    def _poll(self):
        try:
            while True:
                event=self.events.get_nowait()
                if event[0]=='progress':
                    _,label,completed,total=event
                    self.status.set(label+(f' · {completed} de {total}' if total else ''))
                    if total:
                        self.progress.stop();self.progress.configure(mode='determinate');self.progress.set(completed/total)
                    else:self.progress.configure(mode='indeterminate');self.progress.start()
                    continue
                if event[0]=='timing':
                    self.operation_timings.append({'fase':event[1],'segundos':round(event[2],4)})
                    self.operation_timings=self.operation_timings[-100:]
                    continue
                ok,result,done=event;self.busy=False
                from .interaction import unlock
                unlock(self)
                if hasattr(self,'cancel_button'):self.cancel_button.configure(state='disabled')
                self.progress.stop();self.progress.configure(mode='determinate');self.progress.set(0)
                if ok:
                    self.status.set('Operación terminada.')
                    if done:
                        try:done(result)
                        except Exception as exc:
                            self.status.set(str(exc));messagebox.showerror('No se completó la operación',str(exc))
                else:
                    from .sync import SyncConflict
                    from .operations import OperationCancelled
                    if isinstance(result,OperationCancelled):self.status.set(str(result))
                    elif isinstance(result,SyncConflict):
                        from .sync_view import resolve
                        resolve(self,result)
                    elif isinstance(result,SheetChoice):self._choose_external_sheet(result.names)
                    else:self.status.set(str(result));messagebox.showerror('No se completó la operación',str(result))
                    self._save_session()
                if hasattr(self,'refresh_pending'):self.refresh_pending(capture=False)
        except queue.Empty:pass
        except (ValueError,OSError) as exc:self.status.set('No se pudo guardar la recuperación: '+str(exc))
        finally:self.after(100,self._poll)

    def _autosave(self):
        if self.work and not self.busy:
            try:self._save_session()
            except (ValueError,OSError) as exc:self.status.set('No se pudo guardar la recuperación: '+str(exc))
        self.after(15000,self._autosave)

    def _save_session(self):
        if not self.busy:
            for window,capture_manual in getattr(self,'_manual_captures',{}).values():
                if window.winfo_exists():capture_manual()
        self._capture_observation()
        if self.work and not self.busy:
            try:
                from .session import capture
                capture(self)
                self.work.save(self.cfg.directory/'sesion')
                from .download_transfer import archive_current
                archive_current(self.work,self.cfg.directory)
            except OSError as exc:
                self.status.set('No se pudo guardar la recuperación: '+str(exc))
                raise

    def _close(self):
        if self.busy or getattr(self,'rendering',False):messagebox.showinfo('Operación en curso','Espera a que termine antes de cerrar.');return
        try:
            self._save_text(notify=False);self._save_tpl(notify=False)
            self._save_session()
        except (ValueError,OSError) as exc:
            messagebox.showerror('No se pudo guardar antes de cerrar',str(exc));return
        self.destroy()

    def _require_work(self):
        if self.busy or getattr(self,'rendering',False):raise ValueError('Hay una operación en curso.')
        self._capture_observation()
        if not self.work or not self.work.output:raise ValueError('Procesa primero el Excel; se compartirá automáticamente con esta pestaña.')
        return self.work

    def _guard(self,action):
        try:
            if self.busy or getattr(self,'rendering',False):raise ValueError('Espera a que termine la operación actual.')
            action()
        except Exception as exc:messagebox.showerror('Revisa los datos',str(exc))

    def _open(self,path):
        if not Path(path).exists():raise ValueError('No se encuentra el archivo o carpeta: '+str(path))
        if os.name=='nt':os.startfile(str(path))
        else:messagebox.showinfo('Archivo disponible',str(path))

    def _update_context(self):
        if not self.work:
            self.context.set('Sin trabajo activo · Ctrl+O para elegir un Excel')
            return
        source=Path(getattr(self.work,'path','') or '').name or 'sin archivo'
        mode=str(getattr(self.work,'mode','') or 'TRABAJO')
        count=len(getattr(self.work,'rows',[]) or [])
        output=getattr(self.work,'output','')
        copy=('copia: '+Path(output).name) if output else 'copia aún no generada'
        self.context.set(f'{mode} · {source} · {count} registros · {copy}')

    def _open_output_folder(self):
        output=getattr(getattr(self,'work',None),'output','')
        folder=Path(output).parent if output else Path(self.folder.get())
        folder.mkdir(parents=True,exist_ok=True)
        self._open(folder)

    def _bind_shortcuts(self):
        self.bind_all('<Control-s>',self._shortcut_save)
        self.bind_all('<Control-o>',self._shortcut_open)
        self.bind_all('<Control-f>',self._shortcut_find)
        self.bind_all('<F5>',self._shortcut_refresh)
        from .commands import show_palette
        self.bind_all('<Control-k>',lambda event:(show_palette(self),'break')[1])

    def _shortcut_save(self,event=None):
        self._guard(self._save_session)
        self.status.set('Sesión guardada localmente.')
        return 'break'

    def _shortcut_open(self,event=None):
        self._choose();return 'break'

    def _shortcut_find(self,event=None):
        self.tabs.select(str(self.pages['Trabajo']))
        self.work_search_entry.focus_set()
        try:self.work_search_entry.select_range(0,'end')
        except Exception:pass
        return 'break'

    def _shortcut_refresh(self,event=None):
        if self.work and getattr(self.work,'output',''):self._guard(self._refresh)
        else:self.status.set('No hay una copia de trabajo que actualizar.')
        return 'break'

    @staticmethod
    def _tree_text(tree,iid,extra=''):
        values=tree.item(iid,'values')
        return normalize(' '.join([str(extra or ''),*(str(value or '') for value in values)]))

    @staticmethod
    def _show_tree_item(tree,iid):
        if tree.exists(iid):tree.move(iid,'','end')

    def _apply_work_filter(self,*_):
        if not hasattr(self,'records'):return
        from .table_update import apply_visibility
        remember(self,'records',selected_ids(self,'records'))
        search=normalize(self.work_search.get() if hasattr(self,'work_search') else '')
        mode=self.work_filter.get() if hasattr(self,'work_filter') else 'Todos'
        if not search and mode=='Todos':
            apply_visibility(self.records,self._work_all_iids,set(self._work_all_iids));restore_visible(self,'records');return
        visible=set()
        for iid in getattr(self,'_work_all_iids',[]):
            if not self.records.exists(iid):continue
            tags=set(self.records.item(iid,'tags'))
            haystack=getattr(self,'_work_filter_text',{}).get(iid)
            if haystack is None:haystack=self._tree_text(self.records,iid,getattr(self,'_work_search_text',{}).get(iid,''))
            match=not search or search in haystack
            if mode=='Con aviso':match=match and 'warning' in tags
            elif mode=='Con resolución':match=match and 'resolution' in tags
            elif mode=='Excluidos':match=match and 'excluded' in tags
            elif mode=='Sin incidencias':match=match and not ({'warning','excluded'} & tags)
            if match:visible.add(iid)
        apply_visibility(self.records,self._work_all_iids,visible);restore_visible(self,'records')

    def _apply_resolution_filter(self,*_):
        if not hasattr(self,'words'):return
        from .table_update import apply_visibility
        remember(self,'words',selected_ids(self,'words'))
        search=normalize(self.resolution_search.get() if hasattr(self,'resolution_search') else '')
        mode=self.resolution_filter.get() if hasattr(self,'resolution_filter') else 'Todos'
        visible=set()
        for iid in getattr(self,'_resolution_all_iids',[]):
            if not self.words.exists(iid):continue
            tags=set(self.words.item(iid,'tags'))
            match=not search or search in self._tree_text(self.words,iid,getattr(self,'_resolution_search_text',{}).get(iid,''))
            wanted={'Definido en RES':'res_explicit','Ajustado manualmente':'res_manual','Sugerencia automática':'res_auto','RES antiguo':'res_legacy'}.get(mode)
            if match and (not wanted or wanted in tags):visible.add(iid)
        apply_visibility(self.words,self._resolution_all_iids,visible);restore_visible(self,'words')

    @staticmethod
    def _field(parent,label,variable,row,width=65):
        ttk.Label(parent,text=label).grid(row=row,column=0,sticky='w',pady=3)
        entry=ttk.Entry(parent,textvariable=variable,width=width);entry.grid(row=row,column=1,sticky='ew',padx=6,pady=3)
        parent.columnconfigure(1,weight=1);return entry

    @staticmethod
    def _tree(parent,columns):
        frame=ttk.Frame(parent);frame.pack(fill='both',expand=True,pady=8)
        tree=ttk.Treeview(frame,columns=columns,show='headings',selectmode='extended')
        for col in columns:tree.heading(col,text=col);tree.column(col,width=140,stretch=True)
        y=ttk.Scrollbar(frame,orient='vertical',command=tree.yview);x=ttk.Scrollbar(frame,orient='horizontal',command=tree.xview)
        tree.configure(yscrollcommand=y.set,xscrollcommand=x.set)
        tree.grid(row=0,column=0,sticky='nsew');y.grid(row=0,column=1,sticky='ns');x.grid(row=1,column=0,sticky='ew')
        frame.rowconfigure(0,weight=1);frame.columnconfigure(0,weight=1);return tree

    def _work_page(self):
        from .work_view import build
        build(self)

    def _inspect_input(self):
        from .sitfa import inspect_input
        def done(report):
            message=report['estado']+'\n'+'\n'.join(report['mensajes'])
            self.status.set(message.replace('\n',' · '));messagebox.showinfo('Compatibilidad del libro',message,parent=self)
        path,mode,sheet=self.file.get(),self.mode.get(),self.sheet.get() or None
        self._run('Comprobando el libro…',lambda:inspect_input(path,mode,sheet),done)

    def _open_sitfa_source(self):
        from .sitfa import verified_source
        work=self._require_work();selected=selected_ids(self,'records')
        if len(selected)!=1:raise ValueError('Selecciona una fila para abrir su original.')
        row=next(r for r in work.rows if r.id==selected[0]);self._open(verified_source(work,row))

    def _download_joint(self):
        from .download_link import start
        start(self)

    def _select_folder(self,var):
        path=filedialog.askdirectory()
        if path:var.set(path)

    def _show_resume_offer(self,saved):
        work=getattr(self,'_resume_work',None)
        if not work:return
        self.resume_text.set(resume_description(work,saved))
        self.resume_frame.pack(fill='x',pady=(0,6),before=self.work_top)

    def _continue_resume(self):
        work=getattr(self,'_resume_work',None)
        if not work:raise ValueError('No hay un trabajo recuperable.')
        self.work=work;self._resume_work=None;self.resume_frame.pack_forget()
        self._show_work()
        from .session import restore
        restore(self)
        self.status.set('Trabajo anterior recuperado con sus ediciones y productos pendientes.')

    def _choose_other_resume(self):
        self._resume_work=None;self.resume_frame.pack_forget();self._choose()

    def _next_incident(self,direction=1):
        work=getattr(self,'work',None)
        if not work:return self.status.set('No hay un trabajo activo.')
        ids=incident_ids(work)
        self.incident_text.set(f'{len(ids)} incidencia'+('s' if len(ids)!=1 else ''))
        if not ids:return self.status.set('No hay incidencias reconocidas en los registros.')
        current=selected_ids(self,'records')[0] if selected_ids(self,'records') else None
        try:index=ids.index(current)
        except ValueError:index=-1 if direction>0 else 0
        target=ids[(index+direction)%len(ids)]
        self.work_search.set('');self.work_filter.set('Con aviso');self._apply_work_filter()
        self.records.selection_set(target);self.records.focus(target);self.records.see(target);self._detail()
        self.status.set(f'Incidencia {ids.index(target)+1} de {len(ids)}.')

    def _update_work_case_detail(self,row):
        self.work_case_detail.set_text(compact_case_detail_text(self.work,row,getattr(self,'drafts',[])))

    def _resolution_detail(self,event=None):
        if self.busy:return
        if not getattr(self,'work',None) or not selected_ids(self,'words'):return
        rid=selected_ids(self,'words')[0].split('|',1)[0]
        row=next((r for r in self.work.rows if r.id==rid),None)
        if row:self.resolution_case_detail.set_text(case_detail_text(self.work,row,getattr(self,'drafts',[])))

    def _update_mail_case_detail(self):
        if not hasattr(self,'mail_case_detail'):return
        if self.draft_index is None or self.draft_index>=len(self.drafts):
            self.mail_case_detail.set_text('Selecciona un borrador.')
            return
        self.mail_case_detail.set_text(grouped_draft_detail(self.work,self.drafts[self.draft_index],self.drafts))

    def _mode_changed(self,event=None):
        # Una hoja seleccionada para otro modo no debe anular la detección automática.
        self.sheet.set('')

    def _choose(self):
        if self.busy:return
        path=filedialog.askopenfilename(filetypes=[('Excel','*.xls *.xlsx *.xlsm')])
        if path:
            self.file.set(path);self.sheet.set('')
            self._run('Leyendo nombres de hojas…',lambda:list_workbook_sheets(path),lambda names:self.sheet_box.configure(values=['',*names]))

    def _capture_observation(self):
        if self.busy:return
        if self.observation_id and self.work:
            row=next((r for r in self.work.rows if r.id==self.observation_id),None)
            if row:
                text=self.observation_editor.get('1.0','end-1c')
                if text!=row.review.get('OBSERVACION',row.observation):
                    row.review['OBSERVACION']=text
                    self.work.revision+=1
                    if self.records.exists(row.id):
                        self.records.set(row.id,'Observación',text)
                        if hasattr(self,'_work_filter_text'):self._work_filter_text[row.id]=self._tree_text(self.records,row.id,getattr(self,'_work_search_text',{}).get(row.id,''))
                self._update_work_case_detail(row)
        if hasattr(self,'record_form') and self.work:self.record_form.capture()
    def _apply_observation(self):
        self._capture_observation();self._save_session()
        self.status.set('Edición incorporada a esta sesión y a sus productos.')

    def _detail(self,event=None):
        if self.busy:return
        self._capture_observation()
        if self.work and selected_ids(self,'records'):
            row=next(r for r in self.work.rows if r.id==selected_ids(self,'records')[0])
            self.observation_id=row.id
            self.detail.set('; '.join(row.warnings))
            self.observation_editor.delete('1.0','end');self.observation_editor.insert('1.0',row.review.get('OBSERVACION',row.observation))
            self.observation_editor._textbox.edit_reset()
            if hasattr(self,'record_form'):self.record_form.load(row)
            self._update_work_case_detail(row);
    def _refresh(self):
        self._refresh_resolved()

    def _refresh_resolved(self, resolutions=None, path=None):
        work=self._require_work()
        self._save_session()
        snapshot=deepcopy(work)
        if path:snapshot.output=path
        def action():
            from .sync import SyncConflict
            try:return snapshot.refresh(resolutions),snapshot
            except SyncConflict as exc:
                exc.path=snapshot.output
                raise
        def done(result):
            changed,updated=result
            self.work=updated
            if changed:
                self._show_work()
                from .session import restore
                restore(self)
            self._save_session()
            self._update_context()
            self.status.set('Cambios conciliados. Los productos afectados necesitan actualizarse.' if changed else 'La copia no ha cambiado.')
        self._run('Incorporando cambios de la copia…',action,done)

    def _export_statistics(self):
        from .statistics import export_summary
        work=self._require_work();path=filedialog.asksaveasfilename(defaultextension='.xlsx',initialfile='Resumen_constancias.xlsx')
        if path:self._run('Generando resumen de constancias…',lambda:export_summary(work,path),lambda p:(self._save_session(),self.status.set('Resumen exportado: '+p)))

    def _locate(self):
        self._require_work()
        path=filedialog.askopenfilename(filetypes=[('Excel','*.xls *.xlsx *.xlsm')])
        if path:self._refresh_resolved(path=path)

    def _external(self):
        path=filedialog.askopenfilename(filetypes=[('Excel','*.xlsx *.xls *.xlsm')])
        if path:self._load_external(path)

    def _load_external(self,path,sheet=None):
        self.pending_external=path
        cfg=self.cfg.data;mode=self.mode.get()
        def done(work):
            self.work=work;self._clear_drafts();self.observation_id=None
            self._show_work();self._save_session();self.manual_mail.set(False)
            self.status.set(f'{len(work.rows)} registros incorporados desde {work.sheet}, fila {work.header}. Ya disponibles para todos los productos.')
        self._run('Reconociendo hojas, encabezados y observaciones…',lambda:Work.external(path,cfg,mode,sheet=sheet),done)

    def _choose_external_sheet(self,names):
        window=ctk.CTkToplevel(self);window.title('Elegir tabla del libro');window.transient(self)
        ttk.Label(window,text='Hay varias tablas reconocidas. Elige la que necesitas:').pack(padx=15,pady=10)
        selected=tk.StringVar(value=names[0])
        ttk.Combobox(window,textvariable=selected,values=names,state='readonly',width=45).pack(padx=15)
        def apply():
            name=selected.get();window.destroy();self._load_external(self.pending_external,name)
        ttk.Button(window,text='Usar hoja',command=apply).pack(pady=15)

    def _clear_drafts(self):
        self.drafts=[];self._draft_scope_source=[];self.draft_index=None;self.mail_list.delete(0,'end')
        self._clear_mail_editor();self._clear_projects()
    def _clear_projects(self):
        self.projects=[];self.project_index=None
        self.project_list.delete(0,'end');self.project_editor.delete('1.0','end')
        self._prepared_selection=None
        self.last_word=''

    def _clear_mail_editor(self):
        for variable in (self.to,self.cc,self.subject,self.attach):variable.set('')
        self.body.delete('1.0','end')
        if hasattr(self,'mail_case_detail'):self.mail_case_detail.set_text('Selecciona un borrador.')
        if hasattr(self,'mail_compare'):self.mail_compare.grid_remove()
    def _capture_mail(self):
        if self.draft_index is not None:
            d=self.drafts[self.draft_index];d.to=self.to.get();d.cc=self.cc.get();d.subject=self.subject.get();d.body=self.body.get('1.0','end-1c');d.attachments=[p for p in self.attach.get().split('\n') if p]
            self._update_mail_case_detail();
    def _select_mail(self,event=None):
        if self.busy:return
        self._capture_mail()
        if not self.mail_list.curselection():return
        self.draft_index=self.mail_list.curselection()[0];d=self.drafts[self.draft_index]
        self.to.set(d.to);self.cc.set(d.cc);self.subject.set(d.subject);self.body.delete('1.0','end');self.body.insert('1.0',d.body);self.attach.set('\n'.join(d.attachments))
        self.body._textbox.edit_reset()
        self._update_mail_case_detail();
    def _attachment(self):
        paths=filedialog.askopenfilenames()
        if paths:self.attach.set('\n'.join([p for p in self.attach.get().split('\n') if p]+list(paths)))

    def _send_draft(self):
        work=self._require_work();self._capture_mail()
        if self.draft_index is None:raise ValueError('Primero prepara una vista previa.')
        from .current_tools import confirm_drafts
        if not confirm_drafts(self,[self.drafts[self.draft_index]]):return
        draft=deepcopy(self.drafts[self.draft_index])
        self._run('Guardando únicamente un borrador en Outlook…',lambda:create_draft(work,draft,confirmed=True),lambda r:(self._save_session(),self.status.set('Borrador guardado en Outlook. No se envió ningún correo.')))

    def _word_page(self):
        from .resolution_view import build
        build(self)

    def _capture_project(self):
        if self.project_index is not None and self.project_index<len(self.projects):
            self.projects[self.project_index].text=self.project_editor.get('1.0','end-1c')
    def _select_project(self,event=None):
        if self.busy:return
        self._capture_project()
        if self.project_list.curselection():
            self.project_index=self.project_list.curselection()[0]
            project=self.projects[self.project_index]
            self.project_editor.delete('1.0','end');self.project_editor.insert('1.0',project.text)
            self.project_editor._textbox.edit_reset()
            rid=project.record_ids[0] if project.record_ids else None
            row=next((r for r in self.work.rows if r.id==rid),None) if rid and self.work else None
            if row:self.resolution_case_detail.set_text(case_detail_text(self.work,row,getattr(self,'drafts',[])))
            from .resolution_view import show_values
            show_values(self,project)
    def _prepare_words(self,then_generate=False):
        work=self._require_work()
        from .sync import require_current_copy
        require_current_copy(work)
        self._capture_project()
        previous_projects=deepcopy(self.projects)
        selected=list(selected_ids(self,'words')) or [iid for iid in getattr(self,'_resolution_all_iids',self.words.get_children()) if self.words.exists(iid)]
        if not selected:
            selections=automatic_project_selections(work,kind_code(self.manual_word.get()) or 'PC_IE')
            if not selections:
                raise ValueError('No hay proyectos indicados. Si la planilla usa RES, solo se generan los marcados allí; para agregar otro caso, selecciónalo en Trabajo y usa Agregar desde Trabajo.')
        else:
            selections=[item.split('|') for item in selected]
        def done(result):
            self.projects,errors=result;self.project_index=None;self.project_list.delete(0,'end')
            self.project_editor.delete('1.0','end')
            self._prepared_selection=tuple(selected)
            from .session import merge_project_edits
            merge_project_edits(previous_projects,self.projects)
            if hasattr(self,'generate_word_button'):self.generate_word_button.configure(text=f'Generar {len(self.projects)} proyectos')
            for p in self.projects:self.project_list.insert('end',p.court+' · '+p.rit+' · '+kind_label(p.kind)+' · '+str(len(p.record_ids))+' registros')
            if self.projects:self.project_list.selection_set(0);self._select_project()
            self.status.set(f'{len(self.projects)} proyectos agrupados; {len(errors)} matrices pendientes.')
            if errors:messagebox.showwarning('Matrices pendientes','\n'.join(errors))
            if then_generate and self.projects:self._generate_words()
        self._run('Agrupando proyectos por tribunal, RIT y tipo…',lambda:prepare_projects(work,selections,self.template_dir),done)

    def _generate_words(self):
        work=self._require_work()
        from .product_state import stale
        if any(stale(work,p) for p in self.projects):
            self._prepare_words(then_generate=True);return
        selected=tuple(selected_ids(self,'words') or tuple(iid for iid in getattr(self,'_resolution_all_iids',self.words.get_children()) if self.words.exists(iid)))
        if not self.projects or selected!=getattr(self,'_prepared_selection',None):self._prepare_words(then_generate=True);return
        self._capture_project()
        path=Path(work.output).parent/('Resoluciones_'+datetime.now().strftime('%Y%m%d_%H%M%S_%f')+'.docx')
        projects=deepcopy(self.projects)
        def done(result):
            self.last_word=result;self._save_session();self.status.set('Word conjunto generado: '+result)
        self._run('Generando un Word con los proyectos…',lambda:generate_projects(work,projects,path),done)

    def _config_page(self):
        page=self.pages['Configuración']
        level=ttk.Notebook(page);level.pack(fill='both',expand=True)
        basic=ttk.Frame(level,padding=5);advanced=ttk.Frame(level,padding=5)
        level.add(basic,text='Básico');level.add(advanced,text='Avanzado')
        basic_nb=ttk.Notebook(basic);basic_nb.pack(fill='both',expand=True)
        advanced_nb=ttk.Notebook(advanced);advanced_nb.pack(fill='both',expand=True)

        from .appearance_view import build as build_appearance
        build_appearance(self,basic_nb)
        params_scroll=ScrollPane(basic_nb);basic_nb.add(params_scroll,text='Parámetros');params=params_scroll.body
        self.config_search=tk.StringVar()
        ttk.Label(params,text='Buscar parámetro').grid(row=0,column=0,sticky='w',padx=8,pady=(2,5))
        self.config_search_entry=ttk.Entry(params,textvariable=self.config_search,width=32)
        self.config_search_entry.grid(row=0,column=1,columnspan=3,sticky='ew',padx=8,pady=(2,5))
        self.config_search.trace_add('write',self._filter_config_parameters)
        self.param_vars={};self._config_parameter_widgets=[]
        for i,(key,number) in enumerate(self.cfg.data['umbrales'].items()):
            var=tk.StringVar(value=str(number));self.param_vars[key]=var
            label=ttk.Label(params,text=PARAMETER_LABELS[key]+' (días)');label.grid(row=i//2+1,column=(i%2)*2,sticky='w',padx=8,pady=4)
            entry=ttk.Entry(params,textvariable=var,width=8);entry.grid(row=i//2+1,column=(i%2)*2+1)
            self._config_parameter_widgets.append((key,label,entry))
        ttk.Button(params,text='Guardar umbrales',command=lambda:self._guard(self._save_params)).grid(row=9,column=0,pady=12)
        self.disabled={}
        for i,key in enumerate(['COMUN.CURADOR','COMUN.OIDO','COMUN.PROX_AUDIENCIA','COMUN.PROXIMA_MAYORIA']):
            v=tk.BooleanVar(value=key not in self.cfg.data['desactivadas']);self.disabled[key]=v
            ttk.Checkbutton(params,text='Advertir '+key.split('.')[1].replace('_',' ').lower(),variable=v).grid(row=10+i,column=0,columnspan=4,sticky='w')

        mail=ttk.Frame(basic_nb,padding=8);basic_nb.add(mail,text='Plantillas correo')
        self.tpl_key=tk.StringVar(value='espera');box=NamedChoice(mail,keyvariable=self.tpl_key,names=lambda:{k:v['nombre']+(' · archivada' if v.get('archivada') else '') for k,v in self.cfg.data['correos']['plantillas'].items()},state='readonly',width=35);box.grid(row=0,column=0);box.bind('<<ComboboxSelected>>',self._load_tpl)
        self.tpl_name=tk.StringVar();self.tpl_subject=tk.StringVar();self.tpl_req=tk.BooleanVar();self.tpl_modes=tk.BooleanVar()
        self.tpl_category=tk.StringVar()
        self._field(mail,'Nombre',self.tpl_name,1);self._field(mail,'Asunto',self.tpl_subject,2)
        self.tpl_body=ScrolledText(mail,wrap='word',height=7,undo=True,font=('Segoe UI',11));self.tpl_body.grid(row=3,column=0,columnspan=2,sticky='nsew');mail.rowconfigure(3,weight=1)
        ttk.Checkbutton(mail,text='Adjunto obligatorio',variable=self.tpl_req).grid(row=4,column=0)
        ttk.Checkbutton(mail,text='Usa modalidades',variable=self.tpl_modes).grid(row=4,column=1)
        ttk.Button(mail,text='Guardar plantilla',command=lambda:self._guard(self._save_tpl)).grid(row=5,column=0)
        self.tpl_variable=tk.StringVar(value='{TRIBUNAL}')
        variable_bar=ttk.Frame(mail);variable_bar.grid(row=6,column=0,columnspan=2,sticky='ew',pady=5)
        ttk.Combobox(variable_bar,textvariable=self.tpl_variable,values=['{'+v+'}' for v in sorted(VARIABLES)],state='readonly',width=24).pack(side='left')
        ttk.Button(variable_bar,text='Insertar en cuerpo',command=lambda:self.tpl_body.insert('insert',self.tpl_variable.get())).pack(side='left',padx=4)
        ttk.Label(mail,text='Comportamiento del correo').grid(row=7,column=0,sticky='w')
        from .config import defaults
        NamedChoice(mail,keyvariable=self.tpl_category,names=lambda:{'':'General · selección explícita',
                    **{key:tpl['nombre'] for key,tpl in defaults()['correos']['plantillas'].items()}},
                    state='readonly',width=34).grid(row=7,column=1,sticky='ew')
        ttk.Label(mail,text='Los cambios se guardan también al cambiar de plantilla o cerrar.').grid(row=9,column=0,columnspan=2,sticky='w')
        self.tpl_box=box
        ttk.Button(mail,text='Nueva plantilla',command=lambda:self._guard(self._new_tpl)).grid(row=5,column=1,sticky='w');self._load_tpl()

        from .personalization_view import template_actions,contact_actions
        template_actions(self,mail)
        contacts=ttk.Frame(basic_nb,padding=8);basic_nb.add(contacts,text='Contactos')
        self.contact_name=tk.StringVar();self.contact_mail=tk.StringVar();self.contact_alias=tk.StringVar()
        self._field(contacts,'Programa / tribunal',self.contact_name,0);self._field(contacts,'Correos (; separados)',self.contact_mail,1);self._field(contacts,'Alias (; separados)',self.contact_alias,2)
        ttk.Button(contacts,text='Guardar contacto',command=lambda:self._guard(self._save_contact)).grid(row=3,column=0)
        ttk.Button(contacts,text='Importar catastro Excel',command=lambda:self._guard(self._import_contacts)).grid(row=3,column=1,sticky='w')
        self.contact_list=tk.Listbox(contacts,exportselection=False);self.contact_list.grid(row=4,column=0,columnspan=2,sticky='nsew');contacts.rowconfigure(4,weight=1)
        self.contact_list.bind('<<ListboxSelect>>',self._select_contact)
        ttk.Button(contacts,text='Eliminar contacto seleccionado',command=lambda:self._guard(self._delete_contact)).grid(row=5,column=0)
        contact_actions(self,contacts)
        self._list_contacts()

        office=ttk.Frame(basic_nb,padding=8);basic_nb.add(office,text='Outlook y CC')
        self.account=tk.StringVar(value=self.cfg.data.get('cuenta_outlook',''));self.signature=tk.StringVar(value=self.cfg.data.get('firma',''));self.additional=tk.StringVar(value=self.cfg.data['correos'].get('cc_adicional',''))
        self._field(office,'Cuenta Outlook (vacío: predeterminada)',self.account,0);self._field(office,'Firma de texto adicional',self.signature,1);self._field(office,'CC adicional',self.additional,2)
        ttk.Label(office,text='Copia institucional siempre incluida: ucc_concepcion@pjud.cl').grid(row=3,column=0,columnspan=2,sticky='w',pady=10)
        ttk.Button(office,text='Guardar Outlook y CC',command=lambda:self._guard(self._save_office)).grid(row=4,column=0)

        texts=ttk.Frame(advanced_nb,padding=8);advanced_nb.add(texts,text='Textos de observación')
        keys=[s+'.'+k for s,d in self.cfg.data['textos'].items() if not s.startswith('_') for k in d]
        self.text_key=tk.StringVar(value=keys[0]);box=ttk.Combobox(texts,textvariable=self.text_key,values=keys,state='readonly',width=55);box.pack(anchor='w');box.bind('<<ComboboxSelected>>',self._load_text)
        self.text_variable=tk.StringVar();self.text_variables=ttk.Combobox(texts,textvariable=self.text_variable,state='readonly',width=28);self.text_variables.pack(anchor='w',pady=3)
        ttk.Button(texts,text='Insertar variable en el texto',command=lambda:self.text_editor.insert('insert',self.text_variable.get())).pack(anchor='w')
        self.text_editor=ScrolledText(texts,wrap='word',undo=True,font=('Segoe UI',11));self.text_editor.pack(fill='both',expand=True,pady=6)
        ttk.Button(texts,text='Guardar texto · También se guarda al cambiar de regla',command=lambda:self._guard(self._save_text)).pack(anchor='w');self._load_text()

        matrices=ttk.Frame(advanced_nb,padding=8);advanced_nb.add(matrices,text='Matrices Word')
        ttk.Button(matrices,text='Abrir plantillas Word',command=lambda:self._guard(lambda:self._open(self.template_dir))).grid(row=0,column=0,pady=12,sticky='w')
        self.word_court=tk.StringVar(value='LAJA');self.word_kind=tk.StringVar(value='PC_IE')
        matrix=ttk.Frame(matrices);matrix.grid(row=2,column=0,columnspan=2,sticky='w',pady=5)
        ttk.Combobox(matrix,textvariable=self.word_court,values=['LAJA','MULCHEN','TOME'],state='readonly',width=12).pack(side='left')
        word_kind_box=ttk.Combobox(matrix,textvariable=self.word_kind,values=list(KINDS),state='readonly',width=12);word_kind_box.pack(side='left',padx=4)
        Tooltip(word_kind_box,'PC_IE: '+RES_HELP['PC_IE']+'\nPC_INFO: '+RES_HELP['PC_INFO']+'\nNOMENCL: '+RES_HELP['NOMENCL'])
        ttk.Button(matrix,text='Editar matriz en Word',command=lambda:self._guard(lambda:self._open(self.template_dir/self.word_court.get()/(self.word_kind.get()+'.docx')))).pack(side='left')
        ttk.Button(matrix,text='Reemplazar matriz…',command=lambda:self._guard(self._import_word)).pack(side='left',padx=4)
        ttk.Button(matrices,text='Importar paquete de plantillas ZIP',command=lambda:self._guard(self._import_templates_zip)).grid(row=4,column=0,pady=8,sticky='w')
        ttk.Label(matrices,text='Carpetas LAJA, MULCHEN y TOME; tipos PC_IE.docx, PC_INFO.docx y NOMENCL.docx.',wraplength=640).grid(row=3,column=0,columnspan=2,sticky='w')

    def _filter_config_parameters(self,*_):
        query=normalize(self.config_search.get()) if hasattr(self,'config_search') else ''
        for key,label,entry in getattr(self,'_config_parameter_widgets',[]):
            visible=not query or query in normalize(PARAMETER_LABELS[key]+' '+key)
            if visible:
                label.grid();entry.grid()
            else:
                label.grid_remove();entry.grid_remove()

    def _save_params(self):
        from copy import deepcopy
        cfg=deepcopy(self.cfg.data)
        cfg['umbrales']={k:int(v.get()) for k,v in self.param_vars.items()}
        hidden=[key for key in cfg['desactivadas'] if key not in self.disabled]
        cfg['desactivadas']=hidden+[key for key,var in self.disabled.items() if not var.get()]
        self.cfg.save(cfg)
        self.status.set('Parámetros guardados. Se aplicarán al próximo análisis; el trabajo actual conserva su perfil.')

    def _save_before_switch(self, attribute, variable, save):
        previous=getattr(self,attribute,None)
        if previous and previous!=variable.get():
            try:save(notify=False)
            except ValueError as exc:
                variable.set(previous)
                messagebox.showerror('No se guardó el cambio',str(exc))
                return False
        return True

    def _load_text(self,event=None):
        if not self._save_before_switch('_editing_text_key',self.text_key,self._save_text):return
        self._editing_text_key=self.text_key.get()
        s,k=self._editing_text_key.split('.')
        text=self.cfg.data['textos'][s][k]['texto']
        self.text_editor.delete('1.0','end');self.text_editor.insert('1.0',text);self.text_editor.edit_reset()
        from string import Formatter
        fields=sorted({'{'+f+'}' for _,f,_,_ in Formatter().parse(text) if f})
        self.text_variables.configure(values=fields);self.text_variable.set(fields[0] if fields else '')

    def _save_text(self,notify=True):
        key=getattr(self,'_editing_text_key',None)
        if not key:return
        cfg=deepcopy(self.cfg.data);s,k=key.split('.')
        text=self.text_editor.get('1.0','end-1c')
        if text!=cfg['textos'][s][k]['texto']:
            cfg['textos'][s][k]['texto']=text;self.cfg.save(cfg)
        if notify:self.status.set('Texto guardado para próximos análisis; la revisión actual se conserva.')

    def _load_tpl(self,event=None):
        if not self._save_before_switch('_editing_tpl_key',self.tpl_key,self._save_tpl):return
        self._editing_tpl_key=self.tpl_key.get()
        tpl=self.cfg.data['correos']['plantillas'][self._editing_tpl_key]
        self.tpl_name.set(tpl['nombre']);self.tpl_subject.set(tpl['asunto'])
        self.tpl_body.delete('1.0','end');self.tpl_body.insert('1.0',tpl['cuerpo']);self.tpl_body.edit_reset()
        self.tpl_req.set(tpl['adjunto']=='obligatorio');self.tpl_modes.set(tpl.get('usa_modalidades',False))
        if hasattr(self,'tpl_category'):
            from .mail_category import category,CATEGORIES
            operational=category(tpl,self._editing_tpl_key)
            self.tpl_category.set(operational if operational in CATEGORIES else '')

    def _save_tpl(self,notify=True):
        key=getattr(self,'_editing_tpl_key',None)
        if not key:return
        cfg=deepcopy(self.cfg.data);tpl=cfg['correos']['plantillas'][key]
        tpl.update(nombre=self.tpl_name.get().strip(),asunto=self.tpl_subject.get(),cuerpo=self.tpl_body.get('1.0','end-1c'),adjunto='obligatorio' if self.tpl_req.get() else 'opcional',usa_modalidades=self.tpl_modes.get())
        if hasattr(self,'tpl_category') and ('categoria' in tpl or self.tpl_category.get()!=key):
            tpl['categoria']=self.tpl_category.get()
        if not tpl['nombre']:raise ValueError('La plantilla necesita un nombre.')
        if cfg!=self.cfg.data:
            cfg.setdefault('template_backups',{})[key]=deepcopy(self.cfg.data['correos']['plantillas'][key])
            self.cfg.save(cfg)
        keys=list(cfg['correos']['plantillas'])
        self.tpl_box.configure(values=keys);self.kind_box.configure(values=[k for k in keys if not cfg['correos']['plantillas'][k].get('archivada',False)])
        if notify:self.status.set('Plantilla guardada; se aplica al preparar correos nuevos.')

    def _new_tpl(self):
        from copy import deepcopy
        from uuid import uuid4
        self._save_tpl(notify=False)
        name=simpledialog.askstring('Nueva plantilla','Nombre de la comunicación:')
        if not name:return
        cfg=deepcopy(self.cfg.data);key='particular_'+uuid4().hex[:6]
        cfg['correos']['plantillas'][key]={'nombre':name,'asunto':name+' — {TRIBUNAL}','cuerpo':'Buen día:\n\n\nAtentamente,','adjunto':'opcional','usa_modalidades':False}
        self.cfg.save(cfg);keys=list(cfg['correos']['plantillas']);self.tpl_box.configure(values=keys);self.kind_box.configure(values=keys);self.tpl_key.set(key);self._load_tpl()

    def _list_contacts(self):
        self.contact_list.delete(0,'end')
        query=normalize(self.contact_search.get()) if hasattr(self,'contact_search') else ''
        names=['Tribunal: '+key for key in self.cfg.data['correos']['tribunales']]+sorted(self.cfg.data['contactos'])
        for name in names:
            aliases=' '.join(key for key,target in self.cfg.data['aliases'].items() if target==name)
            if not query or query in normalize(name+' '+aliases):self.contact_list.insert('end',name)

    def _select_contact(self,event=None):
        if self.contact_list.curselection():
            name=self.contact_list.get(self.contact_list.curselection()[0])
            self._editing_contact_name=name;self.contact_name.set(name)
            self.contact_alias.set('; '.join(key for key,target in self.cfg.data['aliases'].items() if target==name))
            self.contact_mail.set('; '.join(self.cfg.data['correos']['tribunales'][name.split(': ',1)[1]]['para']) if name.startswith('Tribunal: ') else self.cfg.data['contactos'][name])

    def _save_contact(self):
        from .personalization import save_contact
        previous=deepcopy(self.cfg.data)
        cfg=save_contact(previous,self.contact_name.get(),self.contact_mail.get(),self.contact_alias.get().split(';'),previous=getattr(self,'_editing_contact_name',None))
        self.cfg.save(cfg);self._contact_undo=previous
        self._editing_contact_name=self.contact_name.get().strip()
        self._list_contacts()
        self.status.set('Contacto y alias guardados. Deshacer está disponible.')

    def _delete_contact(self):
        from copy import deepcopy
        name=self.contact_name.get();cfg=deepcopy(self.cfg.data)
        self._contact_undo=deepcopy(cfg)
        if name.startswith('Tribunal: '):cfg['correos']['tribunales'][name.split(': ',1)[1]]['para']=[]
        else:
            cfg['contactos'].pop(name,None);cfg['aliases']={k:v for k,v in cfg['aliases'].items() if v!=name}
        self.cfg.save(cfg);self._list_contacts()
        self._editing_contact_name=None
        self.status.set('Contacto eliminado. Puedes deshacer esta acción.')

    def _import_contacts(self):
        path=filedialog.askopenfilename(filetypes=[('Excel','*.xlsx *.xls')])
        if not path:return
        from copy import deepcopy
        cfg=deepcopy(self.cfg.data);contacts,conflicts=import_contacts(cfg,path);cfg['contactos']=contacts;self.cfg.save(cfg);self._list_contacts()
        self.status.set(f'✓ Catastro incorporado · {len(contacts)} contactos disponibles.')
        if conflicts:messagebox.showwarning('Contactos con conflicto','No se sustituyeron coincidencias conflictivas:\n'+'\n'.join(conflicts))

    def _save_office(self):
        from copy import deepcopy
        cfg=deepcopy(self.cfg.data);cfg['cuenta_outlook']=self.account.get().strip();cfg['firma']=self.signature.get();cfg['correos']['cc_adicional']=self.additional.get();self.cfg.save(cfg);self.status.set('Configuración de Outlook guardada.')

    def _import_templates_zip(self):
        from .template_package import import_templates
        path=filedialog.askopenfilename(filetypes=[('Paquete Word','*.zip')])
        if not path:return
        result=import_templates(path,self.template_dir)
        self.status.set(f"{len(result['imported'])} matrices importadas; {len(result['unmatched'])} archivos sin identificación automática.")
        if result['unmatched']:messagebox.showinfo('Asignación pendiente','Estos archivos deben incorporarse indicando tribunal y tipo:\n'+'\n'.join(result['unmatched']))

    def _import_word(self):
        court=self.word_court.get();kind=self.word_kind.get()
        if court not in ('LAJA','MULCHEN','TOME') or kind not in ('PC_IE','PC_INFO','NOMENCL'):return
        path=filedialog.askopenfilename(filetypes=[('Word','*.docx')])
        if not path:return
        from docx import Document
        Document(path)  # Verificar antes de reemplazar la matriz vigente.
        target=self.template_dir/court/(kind+'.docx');target.parent.mkdir(parents=True,exist_ok=True)
        if target.exists():shutil.copyfile(target,target.with_suffix('.bak.docx'))
        shutil.copyfile(path,target);self.status.set('Plantilla incorporada: '+str(target))



def main():
    from .app import main as personal_main
    personal_main()

if __name__=='__main__':main()
