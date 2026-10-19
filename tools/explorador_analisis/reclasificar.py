"""Reclasificación estructural V1.1 basada en rutas FLOW. Uso: python -I reclas.py eventos.jsonl"""

import json, sys, collections as C

OP = {  # alias -> (operacion propuesta, ambito, nivel)
    'R01': ('MENU_INF InformesViewAccion.do', 'MENU', 'N2'),
    'R02': ('XHR_LISTADO (carga/cambio pestaña)', 'LISTADO', 'N3'),
    'R03': ('InformesDAction.do', 'LISTADO', 'N1'),
    'R04': ('Encrypt_AjaxAccion.do', 'A05/A06', 'N2'),
    'R05': ('ConsultaDetalleAtPublicoAccion.do (Información de la Causa)', 'A05/A06', 'N2'),
    'R06': ('SUBVENTANA_CAUSA_1 (JSP)', 'A05/A06-sub', 'N2'),
    'R07': ('SUBVENTANA_CAUSA_1 contenido', 'A05/A06-sub', 'N2'),
    'R08': ('SUBVENTANA_CAUSA_2 (JSP)', 'A05/A06-sub', 'N2'),
    'R09': ('SUBVENTANA_CAUSA_2 contenido', 'A05/A06-sub', 'N2'),
    'R10': ('DOCUMENTO desde causa (PDF probable)', 'A05/A06-doc', 'N3'),
    'R11': ('SUBVENTANA_CAUSA_3 (JSP)', 'A05/A06-sub', 'N2'),
    'R12': ('CONTENIDO_COMPARTIDO R11/R16', '?', 'N2'),
    'R13': ('SUBVENTANA_CAUSA_4 (JSP)', 'A05/A06-sub', 'N2'),
    'R14': ('SUBVENTANA_CAUSA_4 contenido', 'A05/A06-sub', 'N2'),
    'R16': ('POPUP_JSP_LISTADO (candidato PopUpHistorial.jsp)', 'POR_RUT?', 'N3'),
    'R18': ('SUBVENTANA_CAUSA_5 (JSP)', 'A05/A06-sub', 'N2'),
    'R19': ('SUBVENTANA_CAUSA_5 contenido', 'A05/A06-sub', 'N2'),
    'R20': ('XHR en R19', 'A05/A06-sub', 'N2'),
    'R21': ('XHR en R19', 'A05/A06-sub', 'N2'),
    'R22': ('IrPopUpInformesAccion.do (bitácora c9 / programados c6)', 'A07', 'N2'),
    'R28': ('POPUP_JSP_LISTADO_2 (envuelve R22)', 'A07?', 'N3'),
    'R29': ('MAOViewAccion.do', 'CARGA', 'N2'),
    'R30': ('MaoDAction.do', 'CARGA', 'N1'),
}
for r in ('R15', 'R17', 'R23', 'R24', 'R25', 'R26', 'R27'):
    OP[r] = ('DESCARGA_EXCEL /sitfa/reportes/*.xls', 'DESCARGA', 'N2')
alias = {}
ev = [json.loads(l) for l in open(sys.argv[1])]
for e in ev:
    g = e['payload'].get('route_group')
    if g and g not in alias:
        alias[g] = f"R{len(alias) + 1:02d}"
cur = {}
tabroot = {}
out = C.Counter()
amb = C.Counter()
for e in ev:
    p = e['payload']
    k = (e['tab_ref'], e['frame_ref'])
    g = alias.get(p.get('route_group'))
    if e['event_type'] == 'POPUP_CREATED':
        tabroot[e['tab_ref']] = g
    if e['event_type'] == 'PAGE_NAVIGATED':
        cur[k] = g
    gg = g or cur.get(k) or tabroot.get(e['tab_ref'])
    op = OP.get(gg, ('SIN_RUTA', None, None))
    if e['event_type'] in ('PAGE_CLOSED', 'USER_ACTION'):
        op = (e['event_type'], None, 'N1')
    out[(op[0], op[2])] += 1
    amb[op[1]] += 1
for k, v in out.most_common():
    print(v, k)
print(amb)
print('sin asignar', out[('SIN_RUTA', None)])
