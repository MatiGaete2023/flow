"""Complete, versioned recovery state; no widget is the sole owner of an edit."""
from .selection import selected_ids, remember, restore_visible
from dataclasses import asdict

from .outputs import Draft
from .resolutions import Project


def archive_bytes(path, content, expected_hash):
    from hashlib import sha256
    from nurus.services.file_output import write_new_file
    if sha256(content).hexdigest()!=expected_hash:
        raise ValueError('El contenido del libro no coincide con su huella; no se guardó la sesión.')
    if path.exists():
        if sha256(path.read_bytes()).hexdigest()!=expected_hash:
            raise ValueError('El archivo de recuperación está dañado: '+path.name)
    else:
        write_new_file(path,lambda target:target.write_bytes(content))


def capture(app):
    app._capture_observation()
    app._capture_mail()
    app._capture_project()
    source = list(getattr(app, '_draft_scope_source', app.drafts))
    for draft in app.drafts:
        if not any(item is draft for item in source):
            source.append(draft)
    app.work.session = dict(
        version=2,
        drafts=[asdict(draft) for draft in source],
        projects=[asdict(project) for project in app.projects],
        prepared_selection=list(getattr(app, '_prepared_selection', None) or []),
        last_word=app.last_word,
        preferences={key: getattr(app, key).get() for key in
                     ('folder', 'period', 'mail_target', 'mail_kind', 'work_search', 'work_filter',
                      'resolution_search','resolution_filter','manual_mail')
                     if hasattr(app, key)},
    )
    prefs=app.work.session['preferences']
    if hasattr(app,'modality_vars'):prefs['modalities']=[key for key,var in app.modality_vars.items() if var.get()]
    if hasattr(app,'_selected_mail_courts'):prefs['courts']=app._selected_mail_courts()
    if hasattr(app,'records'):prefs['records']=list(selected_ids(app,'records'))
    for key in ('work_sash_ratio','resolution_sash_ratio'):
        if hasattr(app,key):prefs[key]=getattr(app,key)


def restore(app):
    saved = getattr(app.work, 'session', {})
    if not saved:
        return
    if saved.get('version') != 2:
        raise ValueError('La versión de la sesión no es compatible.')
    # Parse all models before touching the visible state.
    drafts = [Draft(**data) for data in saved.get('drafts', [])]
    projects = [Project(**data) for data in saved.get('projects', [])]
    for key, value in saved.get('preferences', {}).items():
        if key in ('folder', 'period', 'mail_target', 'mail_kind', 'work_search', 'work_filter',
                   'resolution_search','resolution_filter','manual_mail') and hasattr(app, key):
            if key=='mail_kind' and value not in app.cfg.data['correos']['plantillas']:continue
            if key=='mail_target' and value not in ('todos','programas','tribunales'):continue
            getattr(app, key).set(value)
    prefs=saved.get('preferences',{})
    if 'modalities' in prefs and hasattr(app,'modality_vars'):
        for key,var in app.modality_vars.items():var.set(key in prefs['modalities'])
    if 'courts' in prefs and hasattr(app,'mail_courts'):
        app.mail_courts.selection_clear(0,'end')
        for index,key in enumerate(app.mail_court_keys):
            if key in prefs['courts']:app.mail_courts.selection_set(index)
    if 'records' in prefs and hasattr(app,'records'):
        ids=[rid for rid in prefs['records'] if app.records.exists(rid)]
        remember(app,'records',ids);restore_visible(app,'records')
    for key in ('work_sash_ratio','resolution_sash_ratio'):
        ratio=prefs.get(key)
        if isinstance(ratio,(int,float)) and .1<=ratio<=.9:setattr(app,key,ratio)
    if hasattr(app,'_mail_kind_changed'):app._mail_kind_changed()
    app._display_prepared_drafts(drafts, '{count} borradores recuperados.')
    app.projects = projects
    app.project_index = None
    app._prepared_selection = tuple(saved.get('prepared_selection', []))
    app.last_word = saved.get('last_word', '')
    app.project_list.delete(0, 'end')
    for project in projects:
        app.project_list.insert('end', project.court + ' · ' + project.rit + ' · ' + project.kind)
    if projects:
        app.project_list.selection_set(0)
        app._select_project()


def merge_draft_edits(old_drafts, new_drafts):
    """Preserve explicit edits by logical identity, including across regenerated IDs."""
    if not new_drafts:
        return []
    def key(draft):
        return draft.kind, draft.recipient_type, draft.court, draft.program
    previous = {key(draft): draft for draft in old_drafts}
    for draft in new_drafts:
        old = previous.get(key(draft))
        if old is None:
            continue
        draft.product_id=old.product_id
        for field in ('to', 'cc', 'subject', 'body'):
            if field in old.original and getattr(old, field) != old.original[field]:
                setattr(draft, field, getattr(old, field))
        prior={row['id']:row for group in old.roster for row in group['rows']}
        changed_roster=False
        for group in draft.roster:
            for row in group['rows']:
                previous_row=prior.get(row['id'])
                if not previous_row:continue
                if row['include']!=previous_row['include']:changed_roster=True
                row['include']=previous_row['include']
                for index,cell in enumerate(previous_row['cells']):
                    base=previous_row.get('base',previous_row['cells'])
                    if index<len(row['cells']) and index<len(base) and cell!=base[index]:
                        if row['cells'][index]!=cell:changed_roster=True
                        row['cells'][index]=cell
        if changed_roster:draft.roster_reviewed=False
        # Regenerated automatic attachments replace the old automatic list. Explicit
        # additions/removals survive, without retaining an obsolete generated file.
        originals = list(dict.fromkeys([*old.original.get('attachments', []),
            *(g['generated'] for g in old.roster if g.get('generated'))]))
        additions = [path for path in old.attachments if path not in originals]
        from pathlib import Path
        removed={Path(path).name for path in originals if path not in old.attachments}
        draft.attachments=[path for path in draft.attachments if Path(path).name not in removed]
        if originals and not any(path in old.attachments for path in originals):
            draft.attachments = []
        draft.attachments.extend(path for path in additions if path not in draft.attachments)
    return new_drafts


def merge_project_edits(previous, projects):
    from .resolution_view import rerender
    by_case={(p.court,p.rit,p.kind):p for p in previous}
    for project in projects:
        old=by_case.get((project.court,project.rit,project.kind))
        if old is None:continue
        overrides={key:value for key,value in old.values.items()
                   if key in old.original_values and value!=old.original_values[key]}
        if overrides:
            project.values.update(overrides)
            rerender(project)
        if old.text!=old.original_text:
            project.text=old.text
            project.review_required=old.review_required or project.original_text!=old.original_text


from .config import RETENCION_DIAS, SESIONES_ANTERIORES


def previous_candidates(folder, *, keep=SESIONES_ANTERIORES, max_days=RETENCION_DIAS):
    """Solo carpetas de sesiones reconocidas, sin seguir enlaces simbólicos."""
    import time
    from pathlib import Path
    folder=Path(folder)
    if not folder.is_dir() or folder.is_symlink():return []
    entries=sorted((p for p in folder.iterdir() if not p.is_symlink() and p.is_dir()
                    and not (p/'trabajo.json').is_symlink() and (p/'trabajo.json').is_file()),
                   key=lambda p:((p/'trabajo.json').stat().st_mtime,p.name),reverse=True)
    limit=time.time()-max_days*86400
    return [entry for index,entry in enumerate(entries)
            if (keep is not None and index>=keep) or (entry/'trabajo.json').stat().st_mtime<limit]


def prune_previous(folder, *, keep=SESIONES_ANTERIORES, max_days=RETENCION_DIAS, confirmed=False):
    """La eliminación requiere la confirmación expresa de la interfaz local."""
    import shutil
    if not confirmed:return []
    removed=[]
    for entry in previous_candidates(folder,keep=keep,max_days=max_days):
        shutil.rmtree(entry);removed.append(entry.name)
    return removed


def saved_data_summary(directory):
    """Inventario local; los adjuntos externos se miden, nunca se eliminan."""
    import json
    from pathlib import Path
    directory=Path(directory);session=directory/'sesion'
    result={key:{'count':0,'bytes':0} for key in ('sesiones','copias','adjuntos','recibos')}
    folders=[]
    if (session/'trabajo.json').is_file() and not session.is_symlink():folders.append(session)
    archive=session/'anteriores'
    if archive.is_dir() and not archive.is_symlink():
        folders.extend(p for p in archive.iterdir() if p.is_dir() and not p.is_symlink()
                       and (p/'trabajo.json').is_file() and not (p/'trabajo.json').is_symlink())
    attachments=set()
    for folder in folders:
        result['sesiones']['count']+=1
        files=[p for p in folder.iterdir() if p.is_file() and not p.is_symlink()]
        result['sesiones']['bytes']+=sum(p.stat().st_size for p in files)
        copies=[p for p in files if p.suffix=='.bin']
        result['copias']['count']+=len(copies)
        result['copias']['bytes']+=sum(p.stat().st_size for p in copies)
        try:data=json.loads((folder/'trabajo.json').read_text(encoding='utf-8'))
        except (OSError,ValueError):continue
        if not isinstance(data,dict):continue
        receipts=data.get('receipts',{})
        if isinstance(receipts,dict):
            result['recibos']['count']+=len(receipts)
            result['recibos']['bytes']+=len(json.dumps(receipts,ensure_ascii=False).encode('utf-8')) if receipts else 0
        saved=data.get('session',{})
        if not isinstance(saved,dict):continue
        for draft in saved.get('drafts',[]):
            if isinstance(draft,dict):attachments.update(str(Path(p).resolve()) for p in draft.get('attachments',[]) if isinstance(p,str))
    for path in attachments:
        path=Path(path)
        if path.is_file():
            result['adjuntos']['count']+=1
            result['adjuntos']['bytes']+=path.stat().st_size
    return result
