"""Oráculo del reconocimiento de tribunales de Meta 4 (alias «LA» incluido); no modifica la herramienta Python.

Cada texto ficticio se clasifica con `generate_meta4.tribunal_name`, la misma función que usa Python para una celda.
"""
import json
import random
import sys
from pathlib import Path
RAIZ=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(RAIZ/'META4_Bitacoras/scripts'))
import generate_meta4 as g
TEXTOS=[
    '', '   ', 'LA', 'la', 'La', 'lA', '  la  ', 'JF LA', 'JF La', 'jf la', 'L.A.', 'L. A.', 'l.a.', 'l. a', 'L.A', 'L. A',
    'L.  A.', 'L.\tA.', 'L.A.X', 'XL.A.', '_L.A.', 'Sol.A', 'ÁL.A.', '1L.A.', 'JF L.A. Residencia', 'Familia de L.A.',
    'Juzgado L.A.', 'L. A. FAE', 'FAE ESPERA LA', 'FAE ESPERA LA.xlsx', 'LA FAE', 'LA ESPERA', 'LA CUMPLIMIENTO',
    'LA CUMPLIMEINTO', 'LA INFORME', 'LA JF', 'LA ENERO', 'LA PDF', 'LA 2099', 'LA 12', 'LA2099', 'LA_FAE', 'LA-FAE',
    'LA_ESPERA', 'LA\tFAE', 'LA.pdf', 'LA casa', 'LA Florida', 'La Florida', 'la florida', 'Florida LA', 'DE LA',
    'de la', 'en LA', 'TODA LA', 'SOLO LA', 'Y LA', 'LA ANTE', 'Juzgado de la Familia', 'JF de la Familia LA',
    'Laja', 'LAJA', 'La Laja', 'Laja LA', 'Tome', 'Tomé', 'TOME', 'Tomé LA', 'LA Tomé', 'Tomé / Laja', 'TOMELLOSO',
    'Los Angeles', 'Los Ángeles', 'los angeles', 'LOS ANGELES', 'LA Los Angeles', 'JF Los Ángeles LA',
    'JUZGADO DE FAMILIA DE LOS ANGELES', 'LOSA', 'LGS', 'losa', 'THNO', 'CCP', 'Familia CCP', 'CCP LA', 'LA THNO',
    'Tome CCP', 'LAS', 'LAS FAE', 'LAS ANGELES', 'LA LA', 'JF LA / Tomé', 'LA / FAE', 'Jgdo. L. y G. de Laja',
    'Jgdo. L. y G. de Mulchén', 'Mulchen', 'Santa Bárbara', 'Cañete', 'CAETE', 'Concepcin', '1 LA', 'L A', 'L.A.L.A.',
    'Juzgado de Familia LA', 'FAE RES LA', 'RES LA', 'LA RES', 'LA RESIDENCIA', 'Residencia LA Espera',
]
# Bloque aleatorio con semilla: combina alias, conectores, vecinos, nombres y puntuación para cubrir el contexto.
r=random.Random(9)
PIEZAS=(['LA','la','La','L.A.','L. A.','l.a','LOSA','LGS','THNO','CCP','Tomé','Laja','Florida','Los Ángeles','casa','2099','12']
        +sorted(g.ALIAS_CONNECTORS)+sorted(g.ALIAS_NEIGHBOURS))
SEPARADORES=[' ',' ',' ','  ','/','-','_','.',', ','\t']
for _ in range(400):
    piezas=[r.choice(PIEZAS) for _ in range(r.randrange(1,5))]
    texto=piezas[0]
    for p in piezas[1:]:
        texto+=r.choice(SEPARADORES)+p
    if texto not in TEXTOS:
        TEXTOS.append(texto)
casos=[dict(texto=t,tribunal=g.tribunal_name(t)) for t in TEXTOS]
p=RAIZ/'rust/crates/flow-meta4/tests/datos/tribunales_python.json'
p.write_text(json.dumps(casos,ensure_ascii=False,indent=1)+'\n',encoding='utf-8')
print(f'{len(casos)} textos de tribunal ficticios clasificados con Python.')
