# PyInstaller (onedir). La extension se copia aparte, junto al EXE, por el instalador.
from pathlib import Path
from PyInstaller.utils.hooks import collect_data_files

root = Path(SPECPATH).parent
datas = collect_data_files('reportlab') + collect_data_files('PIL') + collect_data_files('tzdata')
a = Analysis(
    [str(root / 'app' / 'sitfa_lite.py')],
    pathex=[str(root / 'app')],
    binaries=[],
    datas=datas,
    hiddenimports=['xlrd', 'openpyxl', 'tzdata', 'lxml.etree', 'lxml.html'],
    hookspath=[],
    runtime_hooks=[],
    excludes=['pytest', 'numpy', 'matplotlib'],
)
pyz = PYZ(a.pure)
exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='SITFA_Lite',
    debug=False,
    strip=False,
    upx=False,
    console=False,
)
coll = COLLECT(exe, a.binaries, a.datas, strip=False, upx=False, name='SITFA_Lite')
