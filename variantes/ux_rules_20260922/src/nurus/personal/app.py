"""Interfaz CSMP personal.

Único flujo operativo del producto; app_base contiene sus controles compartidos.
"""
from copy import deepcopy
from dataclasses import replace
from datetime import datetime
from pathlib import Path
from tkinter import filedialog, messagebox
import customtkinter as ctk
from .ui import Textbox as ScrolledText

from .app_base import App as _BaseApp
from .mail_view import build_mail_page
from .mail_controls import alcance_modalidades, selected_court_record_ids
from .outputs import prepare_drafts, prepare_required_drafts, create_drafts, drafts_for_scope, value
from .resolutions import automatic_project_selections, unique_case_selections, resolution_selection_source, kind_code, kind_label
from .statistics import summarize
from .work import Work
from nurus.rus.columns import normalize


class App(_BaseApp):
    def __init__(self,configuration=None):
        super().__init__(configuration=configuration)
        width=min(1180,self.winfo_screenwidth()-60)
        height=min(820,self.winfo_screenheight()-110)
        self.geometry(f'{width}x{height}')
        self.minsize(min(940,width),min(580,height))

    def _mail_page(self):
        build_mail_page(self)

    def _selected_mail_courts(self):
        return [self.mail_court_keys[i] for i in self.mail_courts.curselection() if i<len(self.mail_court_keys)]

    def _select_all_mail_courts(self):
        if self.mail_courts.size():self.mail_courts.selection_set(0,'end')

    def _select_no_mail_courts(self):
        self.mail_courts.selection_clear(0,'end')

    def _selected_mail_modalities(self):
        return [key for key,var in self.modality_vars.items() if var.get()]

    def _select_all_mail_modalities(self):
        for var in self.modality_vars.values():var.set(True)
        self._mail_modality_changed()

    def _select_no_mail_modalities(self):
        for var in self.modality_vars.values():var.set(False)
        self._mail_modality_changed()

    def _mail_modality_changed(self):
        keys=self._selected_mail_modalities() if hasattr(self,'modality_vars') else []
        phrase=alcance_modalidades(keys)
        self.mail_scope.set('Redacción automática: '+phrase+'.' if phrase else 'Sin modalidades seleccionadas.')

    def _mail_kind_changed(self,event=None):
        kind=self.mail_kind.get();tpl=self.cfg.data['correos']['plantillas'].get(kind,{})
        note=tpl.get('nota','')
        requirement='Adjunto: '+tpl.get('adjunto','opcional')+'.'
        mode_note=' Usa modalidades.' if tpl.get('usa_modalidades') else ' No exige modalidades.'
        self.mail_note.set((note+' '+requirement+mode_note).strip())
        self._mail_modality_changed()

    def _mail_selected_ids(self,work,manual=False):
        courts=self._selected_mail_courts()
        ids=selected_court_record_ids(work,courts) if courts else [row.id for row in work.rows]
        if manual:
            chosen=set(self.records.selection())
            if not chosen:raise ValueError('Selecciona los registros de esta gestión en la pestaña Trabajo.')
            ids=[rid for rid in ids if rid in chosen]
        if not ids:raise ValueError('No hay registros del trabajo que coincidan con los tribunales seleccionados.')
        return ids

    def _sync_mail_config(self,work):
        for key in ('correos','contactos','aliases','cuenta_outlook','firma'):
            work.config[key]=deepcopy(self.cfg.data[key])

    def _mail_target_changed(self,event=None):
        self._capture_mail()
        source=getattr(self,'_draft_scope_source',self.drafts)
        self._display_prepared_drafts(source,'{count} borradores del alcance elegido. Preparar todos incorpora las gestiones faltantes.',keep_source=True)

    def _display_prepared_drafts(self,drafts,message,keep_source=False):
        if not keep_source:self._draft_scope_source=list(drafts)
        target=self.mail_target.get() if hasattr(self,'mail_target') else 'todos'
        drafts=drafts_for_scope(drafts,target)
        originals=getattr(self,'_draft_originals',{})
        for draft in drafts:originals.setdefault(draft.key,draft.body)
        self._draft_originals=originals
        self.drafts=drafts;self.mail_list.delete(0,'end');self.draft_index=None
        self.mail_list.set_drafts(drafts,getattr(getattr(self,'work',None),'receipts',{}))
        if drafts:self.mail_list.selection_set(0);self._select_mail()
        else:self._clear_mail_editor()
        self.status.set(message.format(count=len(drafts)))

    def _prepare_mail(self):
        work=self._require_work();self._sync_mail_config(work)
        kind=self.mail_kind.get();tpl=work.config['correos']['plantillas'][kind]
        target=self.mail_target.get()
        keys=self._selected_mail_modalities()
        if tpl.get('usa_modalidades') and not keys:raise ValueError('Selecciona al menos una modalidad para este tipo de correo.')
        phrase=alcance_modalidades(keys) if tpl.get('usa_modalidades') else ''
        manual=self.manual_mail.get();selected=self._mail_selected_ids(work,manual=manual)
        modality_keys=keys if tpl.get('usa_modalidades') else None
        period=self.period.get()
        def done(drafts):self._display_prepared_drafts(drafts,'{count} borradores preparados para revisión; todavía no se guardaron en Outlook.')
        self._run('Preparando textos y adjuntos…',lambda:prepare_drafts(work,kind,modalities=phrase,period=period,selected=selected,manual_selection=manual,modality_keys=modality_keys,recipient_scope='auto' if target=='todos' else target),done)

    def _prepare_all_mail(self):
        work=self._require_work();self._sync_mail_config(work)
        keys=self._selected_mail_modalities()
        if not keys:raise ValueError('Selecciona al menos una modalidad para preparar el conjunto de correos del trabajo.')
        phrase=alcance_modalidades(keys);selected=self._mail_selected_ids(work,manual=False);period=self.period.get();target=self.mail_target.get()
        def done(drafts):self._display_prepared_drafts(drafts,'{count} correos preparados para el alcance elegido. Todavía no se guardaron en Outlook.')
        self._run('Preparando todos los correos necesarios del trabajo…',lambda:prepare_required_drafts(work,modalities=phrase,period=period,selected=selected,modality_keys=keys,recipient_scope=target),done)

    def _send_all(self):
        work=self._require_work();self._capture_mail()
        if self.drafts:
            drafts=[replace(draft) for draft in self.drafts]
            def done(result):
                self._save_session()
                self.status.set(f"{result['created']} borradores guardados; {result['skipped']} ya procesados; {len(result['errors'])} incidencias.")
                if result['errors']:messagebox.showwarning('Lote guardado con incidencias','\n'.join(result['errors']))
            self._run('Guardando el lote de borradores en Outlook…',lambda:create_drafts(work,drafts),done)
            return
        self._sync_mail_config(work)
        keys=self._selected_mail_modalities()
        if not keys:raise ValueError('Selecciona al menos una modalidad.')
        selected=self._mail_selected_ids(work,manual=False)
        period=self.period.get();phrase=alcance_modalidades(keys);target=self.mail_target.get()
        def action():
            drafts=prepare_required_drafts(work,modalities=phrase,period=period,selected=selected,modality_keys=keys,recipient_scope=target)
            return drafts,create_drafts(work,[replace(draft) for draft in drafts])
        def done(payload):
            drafts,result=payload
            self._display_prepared_drafts(drafts,'{count} borradores preparados y procesados para Outlook.')
            self._save_session()
            self.status.set(f"{result['created']} borradores guardados; {result['skipped']} ya existentes; {len(result['errors'])} incidencias.")
            if result['errors']:messagebox.showwarning('Lote guardado con incidencias','\n'.join(result['errors']))
        self._run('Preparando y guardando todos los borradores en Outlook…',action,done)

    def _attachment_all(self):
        self._capture_mail()
        if not self.drafts:raise ValueError('Primero prepara los borradores.')
        paths=filedialog.askopenfilenames(title='Seleccionar adjuntos para todos los borradores')
        if not paths:return
        for draft in self.drafts:
            for path in paths:
                if path not in draft.attachments:draft.attachments.append(path)
        if self.draft_index is not None:self.attach.set('\n'.join(self.drafts[self.draft_index].attachments))
        self.status.set(f'{len(paths)} adjunto(s) incorporado(s) a {len(self.drafts)} borradores.')

    def _clear_current_attachments(self):
        self.attach.set('');self._capture_mail()

    def _preview_mail(self):
        self._capture_mail()
        if self.draft_index is None:raise ValueError('Primero prepara y selecciona un borrador.')
        draft=self.drafts[self.draft_index]
        win=ctk.CTkToplevel(self);win.title('Vista previa del borrador');win.geometry('820x640');win.transient(self)
        text=ScrolledText(win,wrap='word',font=('Segoe UI',10),padx=12,pady=12);text.pack(fill='both',expand=True)
        text.insert('end','PARA: '+draft.to+'\nCC: '+draft.cc+'\nASUNTO: '+draft.subject+'\n\n'+draft.body)
        if draft.attachments:text.insert('end','\n\nADJUNTOS:\n'+'\n'.join('- '+Path(path).name for path in draft.attachments))
        text.configure(state='disabled')

    def _process(self):
        if self.busy:raise ValueError('Hay una operación en curso.')
        path=self.file.get();mode=self.mode.get();sheet=self.sheet.get() or None
        if not Path(path).is_file():raise ValueError('Selecciona un archivo Excel existente.')
        cfg=self.cfg.data
        def done(work):
            self.work=work;self._clear_drafts();self.observation_id=None;self._show_work();self._export_current()
        self._run('Analizando '+mode+'…',lambda:Work(cfg).analyze(path,mode,sheet=sheet),done)

    def _export_current(self):
        self._capture_observation()
        if not self.work:raise ValueError('Primero analiza el archivo.')
        folder=Path(self.folder.get());folder.mkdir(parents=True,exist_ok=True)
        path=folder/(Path(self.work.path).stem+' - revisable '+datetime.now().strftime('%Y%m%d_%H%M%S_%f')+Path(self.work.path).suffix)
        work=self.work
        def done(result):
            self._show_work(reset_projects=False);self._save_session();self._update_context();self.status.set('Excel generado: '+result+' · Disponible en Correos y Resoluciones.')
        self._run('Exportando copia preservada con Excel…',lambda:work.export(path),done)

    def _show_work(self,reset_projects=True):
        if not self.work:return
        self.mode.set(self.work.mode);self.file.set(self.work.path);self.observation_id=None
        self.observation_editor.delete('1.0','end')
        for iid in list(getattr(self,'_work_all_iids',self.records.get_children())):
            if self.records.exists(iid):self.records.delete(iid)
        if reset_projects:
            for iid in list(getattr(self,'_resolution_all_iids',self.words.get_children())):
                if self.words.exists(iid):self.words.delete(iid)
        fallback_kind=(kind_code(self.manual_word.get()) if hasattr(self,'manual_word') else 'PC_IE') or 'PC_IE'
        selections=unique_case_selections(self.work,automatic_project_selections(self.work,fallback_kind))
        resolution_ids={rid for rid,_ in selections}
        by_id={row.id:row for row in self.work.rows}
        self._work_all_iids=[];self._work_search_text={}
        if reset_projects:
            self._resolution_all_iids=[];self._resolution_search_text={}
        for row in self.work.rows:
            state='Excluido' if row.excluded else 'Revisar aviso' if row.warnings else 'Propuesta'
            tags=[]
            if row.excluded:tags.append('excluded')
            elif row.warnings:tags.append('warning')
            if row.id in resolution_ids:tags.append('resolution')
            self.records.insert('','end',iid=row.id,values=(state,value(self.work,row,'rit'),value(self.work,row,'tribunal'),value(self.work,row,'programa'),row.review.get('OBSERVACION',row.observation)),tags=tuple(tags))
            self._work_all_iids.append(row.id)
            self._work_search_text[row.id]=' '.join(str(value(self.work,row,key) or '') for key in ('nombre','rut','rit','tribunal','programa'))
        for rid,kind in selections if reset_projects else []:
            row=by_id[rid]
            source=resolution_selection_source(self.work,row,kind)
            origin_tag={'Definido en RES':'res_explicit','RES antiguo · tipo inferido':'res_legacy','Sugerencia automática revisable':'res_auto'}.get(source,'res_auto')
            iid=rid+'|'+kind
            self.words.insert('','end',iid=iid,values=(value(self.work,row,'rit'),value(self.work,row,'tribunal'),kind_label(kind),source),tags=(origin_tag,))
            self._resolution_all_iids.append(iid)
            self._resolution_search_text[iid]=' '.join(str(value(self.work,row,key) or '') for key in ('nombre','rut','rit','tribunal','programa'))
        self._apply_work_filter()
        if reset_projects:self._apply_resolution_filter()
        n=len(self.work.rows);exc=sum(r.excluded for r in self.work.rows);obs=sum(bool(r.observation) for r in self.work.rows)
        invalid_res=sum(any(str(w).startswith('RES no reconocido:') for w in r.warnings) for r in self.work.rows)
        projects=len(getattr(self,'_resolution_all_iids',self.words.get_children()))
        self.summary.set(f'{n} registros · {obs} propuestas · {exc} excluidos · {projects} proyectos posibles')
        if invalid_res:self.summary.set(self.summary.get()+f' · {invalid_res} RES por corregir')
        totals=summarize(self.work)
        if totals['constancias']:self.summary.set(self.summary.get()+f" · {totals['constancias']} constancias con fecha · {totals['con_carga']} con carga")
        self.detail.set('\n'.join(dict.fromkeys(self.work.warnings)))
        self._refresh_incident_counter();self._update_context()

    @staticmethod
    def _visible_project_key(values):
        return normalize(values[1]),normalize(values[0]),kind_code(values[2]) or str(values[2]).strip().upper()

    def _existing_project_iid(self,key,exclude=None):
        for iid in getattr(self,'_resolution_all_iids',self.words.get_children()):
            if not self.words.exists(iid):continue
            if iid==exclude:continue
            values=self.words.item(iid,'values')
            if len(values)>=3 and self._visible_project_key(values)==key:return iid
        return None

    def _assign_word_type(self):
        selected=list(self.words.selection())
        if not selected:raise ValueError('Selecciona una o más filas concretas de Resoluciones antes de cambiar su tipo.')
        kind=kind_code(self.manual_word.get()) or 'PC_IE';new=[]
        registry=getattr(self,'_resolution_all_iids',None)
        if registry is None:
            registry=list(self.words.get_children());self._resolution_all_iids=registry
        search_map=getattr(self,'_resolution_search_text',None)
        if search_map is None:
            search_map={};self._resolution_search_text=search_map
        for iid in selected:
            rid=iid.split('|')[0];values=list(self.words.item(iid,'values'));values[2]=kind_label(kind);values[3]='Ajustado manualmente en Resoluciones'
            key=self._visible_project_key(values)
            existing=self._existing_project_iid(key,exclude=iid)
            if iid in registry:registry.remove(iid)
            search_map.pop(iid,None)
            self.words.delete(iid)
            if existing:
                target=existing
            else:
                target=rid+'|'+kind
                if not self.words.exists(target):self.words.insert('','end',iid=target,values=values,tags=('res_manual',))
                if target not in registry:registry.append(target)
                work=getattr(self,'work',None)
                row=next((r for r in work.rows if r.id==rid),None) if work else None
                search_map[target]=' '.join(str(value(work,row,key) or '') for key in ('nombre','rut','rit','tribunal','programa')) if row else ''
            new.append(target)
        if hasattr(self,'_apply_resolution_filter'):self._apply_resolution_filter()
        self.words.selection_set(tuple(iid for iid in dict.fromkeys(new) if self.words.exists(iid)))
        self._clear_projects()
        self.status.set(f'{kind_label(kind)} aplicado solo a {len(selected)} selección(es).')

    def _add_words(self):
        work=self._require_work();kind=kind_code(self.manual_word.get()) or 'PC_IE';selected=list(self.records.selection())
        if not selected:raise ValueError('Selecciona en Trabajo los registros que quieres agregar manualmente como proyecto.')
        self._clear_projects()
        for rid in selected:
            row=next(r for r in work.rows if r.id==rid)
            if row.excluded:continue
            values=(value(work,row,'rit'),value(work,row,'tribunal'),kind_label(kind),'Agregado manualmente')
            key=self._visible_project_key(values)
            if self._existing_project_iid(key):continue
            iid=rid+'|'+kind
            if not self.words.exists(iid):
                self.words.insert('','end',iid=iid,values=values,tags=('res_manual',))
                self._resolution_all_iids.append(iid)
                self._resolution_search_text[iid]=' '.join(str(value(work,row,key) or '') for key in ('nombre','rut','rit','tribunal','programa'))
        self._apply_resolution_filter()


def main():
    App().mainloop()


if __name__=='__main__':main()
