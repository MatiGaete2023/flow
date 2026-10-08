"""Incremental Treeview updates. All widget work stays in the Tk thread."""


def upsert(tree, iid, values, tags=()):
    if tree.exists(iid):
        # Tcl stringifies values; avoid needless redraws for unchanged rows.
        if tuple(map(str, tree.item(iid, 'values'))) != tuple(map(str, values)) or tuple(tree.item(iid, 'tags')) != tuple(tags):
            tree.item(iid, values=values, tags=tags)
    else:
        tree.insert('', 'end', iid=iid, values=values, tags=tags)


def apply_visibility(tree, ids, visible):
    """Detach only changed visibility; preserve scroll position on no-op filters."""
    current = tuple(tree.get_children())
    wanted = tuple(iid for iid in ids if iid in visible)
    if current == wanted:
        return
    wanted_set = set(wanted)
    for iid in current:
        if iid not in wanted_set:
            tree.detach(iid)
    for index, iid in enumerate(wanted):
        tree.move(iid, '', index)


def prune(tree, old_ids, new_ids):
    wanted = set(new_ids)
    for iid in old_ids:
        if iid not in wanted and tree.exists(iid):
            tree.delete(iid)


def render_rows(app, jobs, finished, chunk_size=150):
    """Large redraws yield between bounded chunks instead of blocking Tk."""
    if len(jobs)<=500:
        for tree,iid,values,tags in jobs:upsert(tree,iid,values,tags)
        finished();return
    from .interaction import lock,unlock
    app.rendering=True;lock(app)
    cursor=0
    def chunk():
        nonlocal cursor
        try:
            for tree,iid,values,tags in jobs[cursor:cursor+chunk_size]:upsert(tree,iid,values,tags)
            cursor+=chunk_size
            if cursor<len(jobs):app.after(1,chunk)
            else:
                app.rendering=False;unlock(app);finished()
        except Exception:
            app.rendering=False;unlock(app)
            raise
    app.after(0,chunk)
