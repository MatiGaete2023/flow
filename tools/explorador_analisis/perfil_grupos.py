"""Perfil por route_group: métodos, recursos, ventanas y huellas DOM. Uso: python -I perfil_grupos.py eventos.jsonl"""

import json, sys, collections as C

ev = [json.loads(l) for l in open(sys.argv[1])]
popups = {e['tab_ref'] for e in ev if e['event_type'] == 'POPUP_CREATED'}
G = C.defaultdict(lambda: C.Counter())
first = {}
# map (tab,frame)->current group from PAGE_NAVIGATED for DOM attribution
cur = {}
dom = C.defaultdict(C.Counter)
for i, e in enumerate(ev):
    p = e['payload']
    g = p.get('route_group')
    k = (e['tab_ref'], e['frame_ref'])
    if e['event_type'] == 'PAGE_NAVIGATED':
        cur[k] = g
    if e['event_type'] == 'DOM_STRUCTURE':
        gg = cur.get(k, '?')
        dom[gg][
            (
                p['form_count'],
                p['table_count'],
                p['column_count'],
                tuple(sorted(p['form_field_names'])),
                p.get('module_hint'),
            )
        ] += 1
    if not g:
        continue
    first.setdefault(g, i)
    G[g][
        (
            e['event_type'],
            p.get('http_method'),
            p.get('resource_type'),
            'POP' if e['tab_ref'] in popups else 'MAIN',
            'F0' if e['frame_ref'] == 'frame_0' else 'SUB',
        )
    ] += 1
    G[g][('shape', p.get('route_shape'), p.get('route_pattern'))] += 1
for g in sorted(G, key=lambda g: first[g]):
    print('\n###', g, 'first_idx', first[g])
    for k, v in sorted(G[g].items(), key=str):
        print('  ', v, k)
    for k, v in dom.get(g, {}).items():
        print('   DOM', v, k)
print('\nDOM sin grupo', dict(dom.get('?', {})))
