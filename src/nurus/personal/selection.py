"""Selecciones lógicas independientes de las filas visibles de Treeview."""


def selected_ids(app,name):
    tree=getattr(app,name)
    remembered=getattr(app,'_logical_selections',{}).get(name,[])
    if not remembered:return list(tree.selection())
    visible=set(tree.get_children())
    hidden=[iid for iid in remembered if iid not in visible and tree.exists(iid)]
    return list(dict.fromkeys([*hidden,*tree.selection()]))


def remember(app,name,ids):
    if not hasattr(app,'_logical_selections'):app._logical_selections={}
    app._logical_selections[name]=list(ids)


def restore_visible(app,name):
    tree=getattr(app,name);visible=set(tree.get_children())
    tree.selection_set([iid for iid in app._logical_selections.get(name,[]) if iid in visible])
