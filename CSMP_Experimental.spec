from pathlib import Path
import sys
from PyInstaller.utils.hooks import collect_data_files,collect_submodules
source=Path(SPECPATH)/'src'
sys.path.insert(0,str(source))
datas=collect_data_files('nurus')+collect_data_files('customtkinter')+collect_data_files('docx')+collect_data_files('docxcompose')
a=Analysis(['tools/csmp_launcher.py'],pathex=['src'],binaries=[],datas=datas,
           hiddenimports=collect_submodules('nurus')+['win32com.client','pythoncom','pywintypes','xlrd'],hookspath=[],runtime_hooks=[],excludes=[])
pyz=PYZ(a.pure)
exe=EXE(pyz,a.scripts,[],exclude_binaries=True,name='CSMP_Integral',debug=False,strip=False,upx=False,console=False)
coll=COLLECT(exe,a.binaries,a.datas,strip=False,upx=False,name='CSMP_Integral')
