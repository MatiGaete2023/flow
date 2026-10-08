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
        for group in draft.roster:
            for row in group['rows']:
                previous=prior.get(row['id'])
                if not previous:continue
                row['include']=previous['include']
                for index,cell in enumerate(previous['cells']):
                    base=previous.get('base',previous['cells'])
                    if index<len(row['cells']) and index<len(base) and cell!=base[index]:row['cells'][index]=cell
        # Regenerated automatic attachments replace the old automatic list. Explicit
        # additions/removals survive, without retaining an obsolete generated file.
        originals = old.original.get('attachments', [])
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
