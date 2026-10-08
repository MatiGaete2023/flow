"""Identidades del ingreso desde enlaces del listado actual. No ejecuta JavaScript."""
import ast
from collections import defaultdict
from html import unescape
import re


def visible_cell_text(cell):
    """Texto del listado sin código embebido de script/style/template."""
    parts = cell.xpath('.//text()[not(ancestor::script or ancestor::style or ancestor::noscript or ancestor::template)]')
    return ' '.join(' '.join(parts).split())


def calls(text,name):
    """Lectura de argumentos literales, respetando comas/paréntesis en textos."""
    result=[]
    for match in re.finditer(r'\b'+re.escape(name)+r'\s*\(',text):
        start=match.end();items=[];token='';quote=None;escaped=False;finished=False
        for char in text[start:]:
            if quote:
                token+=char
                if escaped:escaped=False
                elif char=='\\':escaped=True
                elif char==quote:quote=None
            elif char in ('\x22','\x27'):quote=char;token+=char
            elif char in (',',')'):
                items.append(token.strip());token=''
                if char==')':finished=True;break
            else:token+=char
        if not finished:continue
        values=[]
        for item in items:
            if re.fullmatch(r'\d{1,20}',item):values.append(item)
            elif item[:1] in ('\x22','\x27'):
                try:
                    value=ast.literal_eval(item)
                    if not isinstance(value,str):break
                    values.append(value)
                except (SyntaxError,ValueError):break
            else:break
        if len(values)==len(items):result.append(values)
    return result


def extract(table,tribunal_code):
    import motor as m
    headers=None;result=[]
    for tr in table.xpath('./tr | ./tbody/tr | ./thead/tr'):
        cells=tr.xpath('./th | ./td');values=[visible_cell_text(c) for c in cells]
        normalized=[m.normalized(v) for v in values]
        if 'RIT' in normalized and any(v in ('NOMBRE','NOMBRE MENOR') for v in normalized):
            keys={'rit':['RIT'],'nombre':['NOMBRE','NOMBRE MENOR'],'rut':['RUT','RUT (->RCEI)'],'programa':['DERIVACION','NOMBRE CENTRO','PROGRAMA']}
            found={k:[i for i,v in enumerate(normalized) if v in aliases] for k,aliases in keys.items()}
            if all(len(items)==1 for items in found.values()):headers={k:items[0] for k,items in found.items()}
            continue
        if not headers or len(values)<=max(headers.values()):continue
        if not re.fullmatch(r'[A-Z]+-\d+-\d{4}',m.normalized(values[headers['rit']])):continue
        text=' '.join(n.get(attr,'') for n in tr.xpath('.//*[@onclick or @href]') for attr in ('onclick','href'))
        observation=calls(text,'ShowObservaciones');history=calls(text,'ShowHistoria');calendar=calls(text,'ShowInformesProgramados')
        if len(observation)!=1 or len(history)>1:continue
        obs=observation[0]
        if len(obs)!=6 or obs[1]!=tribunal_code:continue
        if any(not re.fullmatch(r'\d{1,20}',obs[i]) for i in (0,1,2,3,4,5)) or obs[3]!='12':continue
        # En Espera la fila puede no incluir ShowHistoria, pero la acción
        # ShowObservaciones sigue entregando los tres identificadores remotos.
        # Cuando Historia existe, su concordancia continúa siendo obligatoria.
        if history:
            hist=history[0]
            if (len(hist)!=3 or hist[0]!=tribunal_code or obs[0]!=hist[2]
                    or m.normalized(hist[1])!=m.normalized(values[headers['rit']])):continue
        entry={k:values[i] for k,i in headers.items()}
        entry.update(tribunal_codigo=tribunal_code,causa_id=obs[0],ingreso_id=obs[2],etapa=obs[4],antiguo=obs[5])
        if len(calendar)==1 and len(calendar[0])==12:
            cal=calendar[0]
            if cal[0]==obs[0] and cal[2]==tribunal_code and cal[10]==obs[2] and all(re.fullmatch(r'\d{1,20}',cal[i]) for i in (1,3)):
                entry.update(persona_id=cal[1],centro_id=cal[3])
        result.append(entry)
    return result


def identity(rit,rut,nombre,programa):
    import motor as m
    return (m.normalized(rit),re.sub(r'[.\s]','',m.normalized(rut)),m.normalized(nombre),m.normalized(programa))


def match_rows(headers,rows,bindings,tribunal_code):
    from resultados import mapping
    cols=mapping(headers);keys=('rit','rut','nombre','programa')
    titles=('SITFA_CAUSA_RUS','SITFA_INGRESO_RUS','SITFA_ETAPA_RUS','SITFA_PERSONA_RUS','SITFA_CENTRO_RUS','SITFA_ANTIGUO_RUS','SITFA_VINCULO_ESTADO')
    index=defaultdict(list)
    for item in bindings:
        if item.get('tribunal_codigo')==tribunal_code:index[identity(*(item.get(k,'') for k in keys))].append(item)
    output=[]
    for rownum,row in rows:
        candidates=index.get(identity(*(row[cols[k]] if k in cols else '' for k in keys)),[]) if all(k in cols for k in keys) else []
        unique={tuple(item.get(k,'') for k in ('causa_id','ingreso_id','etapa','persona_id','centro_id','antiguo')) for item in candidates}
        if len(unique)==1:values=(*next(iter(unique)),'Vinculado desde respuesta actual')
        else:values=('','','','','','','Identidad ambigua' if unique else 'Sin vínculo comprobado')
        output.append((rownum,(*row,*values)))
    return (*headers,*titles),output
