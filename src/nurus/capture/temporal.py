"""Cronología explicada; no atribuye revisiones locales a actuaciones oficiales."""
from datetime import date,datetime
from calendar import monthrange

RULESET_VERSION='baseline-20261008+context-preview-1'
CONTEXT_RULES={
 'CT-01':('bitacoras','Última observación atribuible al centro'),
 'CT-02':('bitacoras','Entradas posteriores y empates inciertos'),
 'CT-03':('historia_tramites','Trámites posteriores; título no acredita contenido'),
 'CT-04':('historia_tramites','Discordancia temporal requiere cotejo'),
 'CT-05':('bitacoras','Respuesta requiere comprobación de contenido'),
 'CT-06':('consultas','Fuente o identidad necesaria no disponible'),
 'CT-07':('bitacoras','Posible reiteración; no suprimir propuesta'),
 'CT-08':('historia_tramites','Firma requiere cobertura e identidad'),
 'CT-09':('calendarios','Calendario no acredita recepción'),
 'CT-10':('incidencias','Contradicciones requieren examen humano'),
}


def identity(row):
    return tuple(row.get(k,'') for k in ('source_system','court_code','cause_id','ingreso_id'))


def moment(row):
    value=row.get('event_at','')
    if not value:return None
    return datetime.fromisoformat(value.replace('Z','+00:00')) if 'T' in value else date.fromisoformat(value)


def earliest(as_of):
    """Ventana de cuatro meses naturales; no busca antecedentes más antiguos."""
    month=as_of.year*12+as_of.month-1-4
    year,zero_month=divmod(month,12)
    return date(year,zero_month+1,min(as_of.day,monthrange(year,zero_month+1)[1]))


def chronology(capture,record,as_of,*,fixture_preview=False,event_index=None):
    key=identity(record)
    result={'identity':list(key),'last_center':[],'later':[],'uncertain':[],'recent':[],'findings':[],'class':'NO_EVALUABLE','reason':'Interpretación temporal no aceptada institucionalmente; no acredita ausencia de actuaciones.'}
    def finding(code,reason,status='NO_EVALUABLE',sources=()):
        result['findings'].append({'rule_id':code,'rule_version':RULESET_VERSION,'source_ids':list(sources),'record_identity':list(key),'detected_at':as_of.isoformat(),'fact':reason,'condition':CONTEXT_RULES[code][1],'coverage':record['coverage'],'status':status,'reason':reason,'proposed_actions':[],'confidence_class':'INCIERTA'})
    if not all(key) or record.get('identity_status')!='VERIFICADA':
        result['reason']='Identidad de ingreso no verificada; no se relacionan fuentes por RIT o nombre.'
        finding('CT-06',result['reason']);return result
    if event_index is None:
        related=[event for table in ('bitacoras','historia_tramites') for event in capture.rows(table) if identity(event)==key and event.get('identity_status')=='VERIFICADA' and event.get('scope','INGRESO')=='INGRESO']
    else:related=event_index.get(key,())
    start=earliest(as_of)
    result['period_from']=start.isoformat();result['period_to']=as_of.isoformat()
    result['recent']=[e for e in related if e.get('event_at','') and start.isoformat()<=e['event_at'][:10]<=as_of.isoformat()]
    if not fixture_preview or capture.manifest['contains_sensitive_data']:
        finding('CT-06',result['reason'],sources=[e['event_id'] for e in result['recent']]);return result
    # Solo preview ficticio explícito. Garantías del manifiesto no activan reglas.
    related=result['recent']
    center=[e for e in related if e.get('origin')=='CENTRO' and e.get('origin_verified')=='true' and e.get('coverage') in ('COMPLETA','PARCIAL')]
    if not center:
        result['reason']='ULTIMA_OBSERVACION_NO_ACREDITADA dentro del período capturado.';finding('CT-06',result['reason']);return result
    last_day=max(e['event_at'][:10] for e in center);same_day=[e for e in center if e['event_at'][:10]==last_day]
    if any(e.get('time_precision')!='SEGUNDO' or e.get('time_zone') in ('','DESCONOCIDA') for e in same_day):
        last=same_day
    else:
        try:
            maximum=max(moment(e) for e in same_day);last=[e for e in same_day if moment(e)==maximum]
        except TypeError:last=same_day
    result['last_center']=last;last_ids={e['event_id'] for e in last}
    for event in related:
        if event['event_id'] in last_ids:continue
        if event['event_at'][:10]>last_day:result['later'].append(event)
        elif event['event_at'][:10]==last_day:
            certain=all(e.get('time_precision')=='SEGUNDO' and e.get('time_zone') not in ('','DESCONOCIDA') for e in [event,*last])
            if certain:
                try:
                    if moment(event)>max(moment(e) for e in last):result['later'].append(event)
                except TypeError:result['uncertain'].append(event)
            else:result['uncertain'].append(event)
    result['reason']='Preview ficticio acotado a las fuentes capturadas; no acredita historia completa.'
    result['class']='FOCALIZADA' if result['later'] or result['uncertain'] else 'NO_EVALUABLE'
    finding('CT-01',result['reason'],'APLICA',[e['event_id'] for e in last])
    finding('CT-02','Eventos posteriores o de orden incierto; contenido pendiente de cotejar.','APLICA' if result['later'] else 'NO_EVALUABLE',[e['event_id'] for e in [*result['later'],*result['uncertain']]])
    return result
