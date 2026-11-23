import json
import re
import sys

# 1. Textos
with open('src/nurus/personal/textos_base.json', 'r', encoding='utf-8') as f:
    textos_py = json.load(f)

with open('app/src/config.rs', 'r', encoding='utf-8') as f:
    config_rs = f.read()

textos_rs_matches = re.findall(r'\("([^"]+)",\s*"([^"]+)"\)', config_rs)
print(f'Total TEXTOS in config.rs: {len(textos_rs_matches)}')

discrepancies = []
for full_key, val_rs in textos_rs_matches:
    parts = full_key.split('.', 1)
    if len(parts) != 2:
        continue
    sec, key = parts
    if sec not in textos_py:
        discrepancies.append(f'Section {sec} not in textos_base.json')
        continue
    if key not in textos_py[sec]:
        discrepancies.append(f'Key {key} not in textos_base.json[{sec}]')
        continue
    entry = textos_py[sec][key]
    val_py = entry['texto'] if isinstance(entry, dict) and 'texto' in entry else entry
    if val_rs != val_py:
        discrepancies.append(f'Mismatch for {full_key}:\n  rs={val_rs!r}\n  py={val_py!r}')

print(f'TEXTOS discrepancies count: {len(discrepancies)}')
for d in discrepancies:
    print(' ', d)

# 2. UMBRALES
sys.path.insert(0, 'src')
from nurus.personal.config import UMBRALES as PY_UMBRALES, PARAMETER_LABELS as PY_LABELS

umbrales_rs_matches = re.findall(r'\("([^"]+)",\s*(-?\d+),\s*"([^"]+)"\)', config_rs)
print(f'Total UMBRALES in config.rs: {len(umbrales_rs_matches)}')
print(f'Total UMBRALES in Python: {len(PY_UMBRALES)}')

umbrales_discrepancies = []
rs_keys = set()
for key, val_str, label in umbrales_rs_matches:
    rs_keys.add(key)
    val = int(val_str)
    if key not in PY_UMBRALES:
        umbrales_discrepancies.append(f'Key {key} not in Python UMBRALES')
        continue
    if PY_UMBRALES[key] != val:
        umbrales_discrepancies.append(f'Value mismatch for {key}: rs={val} vs py={PY_UMBRALES[key]}')
    if key not in PY_LABELS:
        umbrales_discrepancies.append(f'Key {key} not in Python PARAMETER_LABELS')
    elif PY_LABELS[key] != label:
        umbrales_discrepancies.append(f'Label mismatch for {key}:\n  rs={label!r}\n  py={PY_LABELS[key]!r}')

py_missing = set(PY_UMBRALES.keys()) - rs_keys
if py_missing:
    umbrales_discrepancies.append(f'Missing keys from Python: {py_missing}')

print(f'UMBRALES discrepancies count: {len(umbrales_discrepancies)}')
for d in umbrales_discrepancies:
    print(' ', d)
