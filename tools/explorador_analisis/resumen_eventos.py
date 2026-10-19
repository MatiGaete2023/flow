"""Conteos de control de eventos_sanitizados.jsonl. Uso: python -I resumen_eventos.py eventos.jsonl"""
import json,sys,collections as C
ev=[json.loads(l) for l in open(sys.argv[1])]
print(len(ev),'ids unique',len({e['event_id'] for e in ev}))
ts=[e['observed_at_utc'] for e in ev];print('sorted',ts==sorted(ts),ts[0],ts[-1])
print(C.Counter(e['event_type'] for e in ev))
print(C.Counter(e['source'] for e in ev))
print('ambito',C.Counter(e['ambito_detectado'] for e in ev))
print('origen',C.Counter((e['clasificacion_origen'],e['clasificacion_confianza']) for e in ev))
print('manual',C.Counter(e['ambito_manual_v1'] for e in ev))
print('keys',C.Counter(k for e in ev for k in e))
print('shape',C.Counter((e['payload'].get('route_shape'),e['payload'].get('route_pattern')) for e in ev if 'route_shape' in e['payload']))
print('groups',len({e['payload']['route_group'] for e in ev if 'route_group' in e['payload']}))
print('tabs',len({e['tab_ref'] for e in ev}),'frames',len({(e['tab_ref'],e['frame_ref']) for e in ev}))
print('hint',C.Counter((e['payload'].get('module_hint'),e['payload'].get('hint_basis'),e['event_type']) for e in ev if 'module_hint' in e['payload']))
print('status',C.Counter(e['payload'].get('http_status') for e in ev if e['event_type']=='RESPONSE_META'))
print('reqs',C.Counter((e['payload'].get('http_method'),e['payload'].get('resource_type'),e['payload'].get('route_shape')) for e in ev if e['event_type']=='REQUEST_META'))
print('gaps',[e['payload'] for e in ev if e['event_type'] in('CAPTURE_GAP','REQUEST_FAILED','CLIENT_DISCONNECTED','UNKNOWN_WINDOW_TECHNOLOGY')])
