# Distribución Windows sin Python instalado; la extensión mantiene instalación manual.
from PyInstaller.utils.hooks import collect_data_files
from pathlib import Path
root=Path(SPECPATH)
shared=next(p for p in (root.parent/'csmp'/'src',root.parent/'src') if (p/'nurus').is_dir())
datas=collect_data_files('reportlab')+collect_data_files('PIL')
datas+=[('extension','extension')]
a=Analysis(['descargador.py'],pathex=[str(shared)],binaries=[],datas=datas,hiddenimports=['xlrd','openpyxl','tzdata','nurus.bitacora_html'],hookspath=[],runtime_hooks=[],excludes=[])
pyz=PYZ(a.pure)
exe=EXE(pyz,a.scripts,[],exclude_binaries=True,name='SITFA_Descargador',debug=False,strip=False,upx=False,console=False)
coll=COLLECT(exe,a.binaries,a.datas,strip=False,upx=False,name='SITFA_Descargador')
