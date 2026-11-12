"""Copia vocabulario y genera oráculo del anonimizador; solo referencias y casos ficticios locales."""
import json
import sys
from pathlib import Path

RAIZ=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(RAIZ/'anonimizador'))
from anonimizador_csmp import vocab,textengine,capture


def main():
    keys=['SENSITIVE','KEEP','FREE_TEXT','ENGINE_HEADERS','SAFE_EXTRA_KEYS','STATE_WORDS','TECHNICAL']
    data={k:sorted(getattr(vocab,k)) if isinstance(getattr(vocab,k),set) else getattr(vocab,k) for k in keys}
    for k in ['INSTITUTIONAL','STOPWORDS','PLACEHOLDERS']:data[k]=sorted(getattr(textengine,k))
    data['SAFE_TITLE']=vocab.SAFE_TITLE.pattern
    (RAIZ/'rust/datos/anonimo_vocab.json').write_text(json.dumps(data,ensure_ascii=False,indent=1)+'\n',encoding='utf-8')
    contrato=dict(tablas=capture.TABLES,control=sorted(capture.CONTROL_FIELDS),libre=sorted(capture.FREE_FIELDS),tiempo=sorted(capture.TIME_FIELDS),filtros=sorted(capture.FILTER_KEEP))
    (RAIZ/'rust/datos/captura_anonima.json').write_text(json.dumps(contrato,ensure_ascii=False,indent=1)+'\n',encoding='utf-8')
    headers=list(dict.fromkeys([*vocab.ENGINE_HEADERS,*vocab.SENSITIVE,*vocab.KEEP,*vocab.FREE_TEXT,*vocab.TECHNICAL,
        'RIT','RUT MENOR','RUN','Rut1','RIT2','CUR. AD LITEM','Correo','Telefono','Domicilio','normalized_json',
        'source_sha256','Cause_ID','Folio','CAUSA RUS','ID Ingreso','categoria libre']))
    casos=[dict(columna=k,tipo=vocab.column_kind(k),protegido=vocab.is_protected(k)) for k in headers]
    destino=RAIZ/'rust/crates/flow-anonimo/tests/datos';destino.mkdir(exist_ok=True)
    (destino/'columnas.json').write_text(json.dumps(casos,ensure_ascii=False,indent=1)+'\n',encoding='utf-8')
    print(f'{len(casos)} clasificaciones de columnas cotejadas con Python.')


if __name__=='__main__':main()
