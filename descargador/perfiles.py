"""Control de una sesion Chrome. Todos los datos de sesion permanecen en memoria."""
from dataclasses import dataclass
from pathlib import Path
import re
import threading
from urllib.parse import urlencode, urlsplit

import motor as m

TABS = {
    "Espera": ("tdEspera", "NUM_PaginaEspera", "NUM_TotalEspera", "tablaEspera"),
    "Cumplimiento": ("tdCumplimiento", "NUM_PaginaCumplimiento", "NUM_TotalCumplimiento", "tablaCumplimiento"),
    "Informes": ("tdInforme", "NUM_PaginaInforme", "NUM_TotalInforme", "tablaInforme"),
    "Egresados": ("tdEgreso", "NUM_PaginaEgreso", "NUM_TotalEgreso", "tablaEgresados"),
}
FORM = 'form[name="InformesPpalForm"]'
SCREENS={'Seguimiento':'seguimiento','Medidas por vencer (calendario)':'calendario_medidas',
         'Carga · resoluciones firmadas':'carga',
         'Informes por vencer (calendario)':'calendario_informes','Seguimiento de litigantes / órdenes':'litigantes'}
MENUS={'seguimiento':'89','litigantes':'94','calendario_informes':'91','calendario_medidas':'90','carga':'29'}


@dataclass(frozen=True)
class Selection:
    tribunal: str
    tab: str
    modality: str
    report: str | None = None
    screen: str = 'seguimiento'
    state: str = '1'
    month: str | None = None
    year: str | None = None
    start: str | None = None
    end: str | None = None


def selection_profile(selection, catalog, template="pendiente"):
    if selection.screen not in MENUS or catalog.get('pantalla','seguimiento')!=selection.screen:
        raise m.PocError('Carga las opciones de la pantalla elegida.')
    if selection.tribunal not in catalog['tribunales']:
        raise m.PocError('Selecciona un tribunal disponible en esta pantalla.')
    tribunal=catalog['tribunales'][selection.tribunal]
    if selection.screen=='carga':
        from carga import parse_day
        a,b=parse_day(selection.start),parse_day(selection.end)
        if a is None or b is None or not 0<=(b-a).days<30:raise m.PocError('Selecciona hasta 30 días inclusivos para carga.')
        return m.QueryProfile(f'T{selection.tribunal}-Carga-{a.isoformat()}-{b.isoformat()}',
            f'{tribunal} / Carga / {selection.start}–{selection.end}','Carga',selection.tribunal,'',template,
            tab='',page_field='',total_field='',table_id='',screen='carga',action='Aud.Carga Func.-',start=selection.start,end=selection.end)
    if selection.screen=='litigantes':
        if selection.state not in catalog.get('estados',{}):raise m.PocError('Selecciona un estado disponible de litigantes u órdenes.')
        label=f'{tribunal} / Litigantes / {catalog["estados"][selection.state]}'
        return m.QueryProfile(f'T{selection.tribunal}-Litigantes-E{selection.state}',label,'Litigantes',selection.tribunal,'2',template,
            tab='',page_field='NUM_PaginaInforme',total_field='NUM_Total',table_id='',screen='litigantes',action='Consulta Informe',state=selection.state)
    if selection.screen.startswith('calendario_'):
        if selection.month not in catalog.get('meses',{}) or selection.year not in catalog.get('anios',{}):
            raise m.PocError('Selecciona un mes y año disponibles en SITFA.')
        kind='Informes' if selection.screen=='calendario_informes' else 'Medidas'
        label=f'{tribunal} / {kind} por vencer / {selection.month}-{selection.year}'
        return m.QueryProfile(f'T{selection.tribunal}-{kind}-{selection.year}-{selection.month}',label,kind,selection.tribunal,'1',template,
            tab='',page_field='',total_field='',table_id='',screen=selection.screen,
            action='Buscar Inf.' if kind=='Informes' else 'Buscar',month=selection.month,year=selection.year)
    if selection.tab not in TABS or selection.tribunal not in catalog["tribunales"] or selection.modality not in catalog["modalidades"]:
        raise m.PocError("Seleccione un tribunal, pestaña y modalidad disponibles en SITFA.")
    if selection.tab == "Informes" and selection.report not in catalog["informes"]:
        raise m.PocError("Seleccione el tipo de informe.")
    if selection.tab == "Cumplimiento" and selection.report is not None and selection.report not in catalog.get("medidas",{}):
        raise m.PocError("Seleccione un filtro de cumplimiento disponible.")
    if selection.tab not in catalog["pestanas"]:
        raise m.PocError("La pestaña seleccionada no esta disponible en este formulario.")
    tab, page, total, table = TABS[selection.tab]
    label = f'{catalog["tribunales"][selection.tribunal]} / {selection.tab} / {catalog["modalidades"][selection.modality]}'
    key = f'T{selection.tribunal}-{selection.tab}-M{selection.modality}'
    prefix = f'T{selection.tribunal}_{selection.tab}_M{selection.modality}'
    report = selection.report if selection.tab in ("Informes","Cumplimiento") else None
    if report:
        options=catalog["informes"] if selection.tab=="Informes" else catalog["medidas"]
        label += f' / {options[report]}'
        key += "-I"+report
        prefix += "_I"+report
    return m.QueryProfile(key, label, prefix, selection.tribunal, selection.modality,
                          template, tab, page, total, table, report)


def response_profile(data, pairs, selection, catalog):
    """La plantilla se obtiene de la respuesta actual, sin inferir prefijos de modalidad."""
    profile = selection_profile(selection, catalog)
    m.query_pairs(urlencode(pairs), profile)
    root = m.root_html(data)
    if m.has_login(root):
        raise m.PocError("La sesion expiro. Cierre la sesion y vuelva a ingresar en Chrome.")
    forms = root.xpath('//form[@name=$name]',name=profile.form_name)
    if (len(forms) != 1 or forms[0].get("action") not in (urlsplit(profile.endpoint).path, profile.endpoint)
            or forms[0].get("method", "").lower() != "post"):
        raise m.PocError("SITFA no devolvio el formulario esperado.")
    fields = m.response_fields(forms[0],profile)
    if any(fields.get(k) != v for k,v in profile.target.items() if k != "COD_Lengueta" and not (profile.screen=='litigantes' and k=='TIP_Consulta')):
        raise m.PocError("La respuesta no corresponde a la seleccion realizada.")
    if profile.screen=='seguimiento' and fields.get("COD_Lengueta") not in ("", profile.tab):
        raise m.PocError("La respuesta pertenece a otra pestaña.")
    if any(re.search(r"csrf|token|nonce", key, re.I) for key in fields):
        raise m.PocError("Aparecio un campo dinamico no auditado; requiere revision.")
    records=m.result_records(forms[0],profile)
    if not records:
        # Las cabeceras/mensajes vacíos de pestañas inactivas no son registros.
        # Si SITFA no identifica la pestaña activa, seguir rechazando datos de otra.
        other_rows = profile.screen=='seguimiento' and fields.get('COD_Lengueta','')=='' and any(
            m.records_in_grid(m.table_grid(table))
            for cfg in TABS.values() if cfg[3] != profile.table_id
            for table in forms[0].xpath('.//table[@id=$id]', id=cfg[3]))
        if other_rows or fields.get(profile.total_field, "") not in ("", "0", "1"):
            raise m.PocError("La respuesta vacia no coincide con la pestaña seleccionada.")
        return profile, None
    u = urlsplit(m.export_url(root))
    if (u.scheme != "https" or u.netloc != "familia.pjud.cl" or u.query or u.fragment
            or not re.fullmatch(r"/sitfa/reportes/[A-Za-z0-9_-]+\.xls", u.path)):
        raise m.PocError("El enlace Excel esta fuera del sitio permitido.")
    template = u.path.rsplit("/",1)[-1].rsplit("_",1)[-1][:-4]
    if not re.fullmatch(r"[A-Za-z0-9-]*", template) or (not template and (selection.screen!='seguimiento' or selection.modality!='4')):
        raise m.PocError("El nombre Excel no se puede reconocer.")
    expected={'litigantes':'BusquedaLit','calendario_informes':'InfoPorVencer','calendario_medidas':'MedPorVencer','carga':'antr'}
    if selection.screen in expected and template!=expected[selection.screen]:raise m.PocError('El Excel corresponde a otra pantalla.')
    profile = selection_profile(selection, catalog, template)
    return profile, m.search_page(data, pairs, profile=profile)
