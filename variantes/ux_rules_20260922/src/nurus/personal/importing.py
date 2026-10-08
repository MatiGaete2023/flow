"""Lectura de planillas revisadas con encabezados desplazados y alias RUS."""
from io import BytesIO
from pathlib import Path
from hashlib import sha256
import json
from nurus.rus.columns import map_columns, normalize, ColumnMappingError

class SheetChoice(ValueError):
    def __init__(self,names):
        self.names=list(names)
        super().__init__('El libro contiene varias tablas: '+', '.join(self.names))

def read_external(path,mode='ESPERA',sheet=None):
    import pandas as pd
    path=Path(path).resolve()
    content=path.read_bytes()
    requested=str(mode).upper()
    distinctive={'ESPERA':'espera','CUMPLIMIENTO':'dias_cumpl','INFORMES':'vencimiento'}
    candidates=[]
    with pd.ExcelFile(BytesIO(content),engine='xlrd' if path.suffix.lower()=='.xls' else 'openpyxl') as book:
        for name in book.sheet_names:
            if sheet and name!=sheet:continue
            preview=pd.read_excel(book,sheet_name=name,header=None,nrows=60,dtype=object,keep_default_na=False)
            for i,values in enumerate(preview.itertuples(index=False,name=None)):
                headers=[str(v).strip() if str(v).strip() else '__col_'+str(j) for j,v in enumerate(values)]
                if len({normalize(h) for h in headers})!=len(headers):continue
                try:requested_mapping=map_columns(headers,requested)
                except ColumnMappingError:continue
                if not all(requested_mapping.get(k) for k in ('rit','tribunal','nombre')):continue
                detected=requested
                mapping=requested_mapping
                # La modalidad elegida por el usuario manda. Solo inferimos otra cuando
                # el libro carece de la columna distintiva solicitada y hay una única
                # modalidad inequívoca en los encabezados.
                if distinctive[requested] not in requested_mapping:
                    inferred=[]
                    for candidate in ('ESPERA','CUMPLIMIENTO','INFORMES'):
                        try:candidate_mapping=map_columns(headers,candidate)
                        except ColumnMappingError:continue
                        if distinctive[candidate] in candidate_mapping:
                            inferred.append((candidate,candidate_mapping))
                    if len(inferred)==1:
                        detected,mapping=inferred[0]
                # Los campos humanos usan el mismo nombre interno cualquiera sea su alias.
                human={}
                for key,aliases in {'OBSERVACION':('observacion','observaciones'),'FECHA_OBS':('fecha_obs','fecha obs','fecha observacion'),'TT':('tt',),'CC':('cc',),'RES':('res','resolucion generada','resolucion','generar resolucion','proyecto resolucion','proyecto de resolucion')}.items():
                    found=[h for h in headers if normalize(h) in {normalize(a) for a in aliases}]
                    if len(found)==1:human[key]=found[0]
                rules_col=next((h for h in headers if normalize(h)==normalize('NURUS_REGLAS')),None)
                candidates.append((name,i+1,headers,mapping,detected,human,rules_col))
                break
        if not candidates:raise ValueError('No se encontró una tabla con RIT, TRIBUNAL y NOMBRE en las primeras 60 filas. Revisa la hoja y sus encabezados.')
        if len(candidates)>1:
            preferred=[c for c in candidates if normalize(c[0])==normalize(requested)]
            if len(preferred)!=1:raise SheetChoice(c[0] for c in candidates)
            chosen=preferred[0]
        else:chosen=candidates[0]
        name,header,headers,mapping,detected,human,rules_col=chosen
        trace_rules={}
        for trace_name in book.sheet_names:
            if not normalize(trace_name).startswith(normalize('NURUS_TRAZABILIDAD')):continue
            trace=pd.read_excel(book,sheet_name=trace_name,header=None,dtype=object,keep_default_na=False)
            header_index=None;trace_headers=None
            for idx,values in enumerate(trace.itertuples(index=False,name=None)):
                normalized=[normalize(v) for v in values]
                if all(normalize(key) in normalized for key in ('HOJA','FILA','REGLAS')):
                    header_index=idx;trace_headers=list(values);break
            if header_index is None:continue
            positions={normalize(value):pos for pos,value in enumerate(trace_headers)}
            for values in list(trace.itertuples(index=False,name=None))[header_index+1:]:
                try:
                    source_sheet=str(values[positions[normalize('HOJA')]] or '').strip()
                    source_row=int(float(values[positions[normalize('FILA')]]))
                    raw=str(values[positions[normalize('REGLAS')]] or '').strip()
                    parsed=json.loads(raw) if raw else []
                except (ValueError,TypeError,json.JSONDecodeError,IndexError):
                    continue
                if source_sheet and isinstance(parsed,list):
                    trace_rules[(normalize(source_sheet),source_row)]=[str(item) for item in parsed if str(item).strip()]
        frame=pd.read_excel(book,sheet_name=name,header=None,skiprows=header,dtype=object,keep_default_na=False)
        records=[]
        for offset,vals in enumerate(frame.itertuples(index=False,name=None),header+1):
            row={h:vals[j] if j<len(vals) else '' for j,h in enumerate(headers)}
            if not str(row.get(mapping['rit'],'')).strip() or not str(row.get(mapping['nombre'],'')).strip():continue
            review={key:row.get(col,'') for key,col in human.items()}
            rules=[]
            if rules_col:
                raw=str(row.get(rules_col,'') or '').strip()
                if raw:
                    try:
                        parsed=json.loads(raw)
                        if isinstance(parsed,list):rules=[str(item) for item in parsed if str(item).strip()]
                    except (json.JSONDecodeError,TypeError):
                        pass
            if not rules:rules=list(trace_rules.get((normalize(name),offset),[]))
            records.append((offset,row,review,rules))
    return dict(path=str(path),content=content,digest=sha256(content).hexdigest(),sheet=name,header=header,mapping=mapping,mode=detected,records=records)
