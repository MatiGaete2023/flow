"""Línea de tiempo compacta con alias R01..Rnn (escribe alias.json en cwd). Uso: python -I linea_tiempo.py eventos.jsonl"""

import json, sys

ev = [json.loads(l) for l in open(sys.argv[1])]
alias = {}
n = 0
for e in ev:
    g = e['payload'].get('route_group')
    if g and g not in alias:
        n += 1
        alias[g] = f"R{n:02d}"
json.dump(alias, open('alias.json', 'w'))
tabs = {}
t = 0
from datetime import datetime


def T(s):
    return datetime.fromisoformat(s.replace('Z', '+00:00')).timestamp()


t0 = T(ev[0]['observed_at_utc'])
for i, e in enumerate(ev):
    tab = e['tab_ref']
    if tab not in tabs:
        t += 1
        tabs[tab] = f"t{t}"
    p = e['payload']
    ty = e['event_type']
    if ty in ('RESPONSE_META', 'FRAME_CREATED'):
        continue
    g = alias.get(p.get('route_group'), '')
    extra = ''
    if ty == 'DOM_STRUCTURE':
        extra = f"f{p['form_count']} t{p['table_count']} c{p['column_count']} n{len(p['form_field_names'])} {p.get('module_hint', '')}"
    if ty == 'USER_ACTION':
        extra = p['module_hint']
    if ty == 'POPUP_CREATED':
        extra = 'from ' + str(tabs.get(p.get('source_tab_ref'), p.get('source_tab_ref')))
    if ty == 'REQUEST_META':
        extra = p['http_method'] + ' ' + p['resource_type']
    fr = '' if e['frame_ref'] == 'frame_0' else e['frame_ref']
    print(
        f"{i:4d} {T(e['observed_at_utc']) - t0:7.1f} {tabs[tab]:5s} {fr:11s} {ty[:12]:12s} {g:4s} {extra}  [{e['ambito_detectado']}]"
    )
