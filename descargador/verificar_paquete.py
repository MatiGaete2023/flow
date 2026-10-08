"""Comprobación del ejecutable y su extensión, sin sesión ni datos del usuario."""
from pathlib import Path
from hashlib import sha256
import gc,json,sys,re
import shutil
import tkinter as tk
from uuid import uuid4


def check(destination):
    import descargador,preferencias
    import motor as m
    from collections import Counter
    from dataclasses import replace
    import xlrd,openpyxl,reportlab
    try:from _build_meta import BUILD_COMMIT,BUILD_VERSION
    except ImportError:BUILD_COMMIT=BUILD_VERSION='unknown'
    destination=Path(destination).resolve();destination.parent.mkdir(parents=True,exist_ok=True)
    extension=Path(descargador.__file__).parent/'extension'
    manifest=json.loads((extension/'manifest.json').read_text(encoding='utf-8'))
    from contrato_integral import VERSION
    assert manifest['version']==VERSION
    capability_source=(extension/'pagina.js').read_text(encoding='utf-8')
    assert f"if(command==='capabilities') return {{tipo:'CSMP_RUS_CAPACIDADES',version:'{VERSION}'" in capability_source
    from csmp_shared import parser
    assert callable(parser())
    for name in ('background.js','conexion.js','conexion.html','conexion.css','pagina.js'):
        assert (extension/name).is_file()
    executable=Path(sys.executable).resolve();app_root=executable.parent
    resources={path.relative_to(app_root).as_posix():sha256(path.read_bytes()).hexdigest()
               for path in sorted(extension.rglob('*')) if path.is_file()}
    if not getattr(sys,'frozen',False) or not executable.is_file():
        raise ValueError('La comprobación de distribución requiere el ejecutable congelado.')
    if BUILD_COMMIT=='unknown' or BUILD_VERSION!=manifest['version']:
        raise ValueError('El ejecutable no declara el commit y versión de su código fuente.')
    exe_hash=sha256(executable.read_bytes()).hexdigest()
    # Exercise the frozen calendar validator with SITFA's actual text format.
    for screen,title in [('calendario_informes','FECHA VENCIMIENTO'),
                         ('calendario_medidas','FEC.EGRESO PROYECTADO'),
                         ('calendario_medidas','FECHA VENCIMIENTO')]:
        for month in ('10','11'):
            profile=replace(m.ORIGINAL,screen=screen,month=month,year='2026')
            info=m.ExcelInfo('prueba',Counter({('X-1-2026','PERSONA FICTICIA'):1}),
                             [['RIT','NOMBRE MENOR',title],['X-1-2026','Persona ficticia',f'01-{month}-2026']])
            m.verify_month(info,profile)
            try:m.verify_month(info,replace(profile,month='9'))
            except m.PocError:pass
            else:raise ValueError('El calendario aceptó una fecha de otro mes.')
    from filtros_respuesta import _INITIALIZER
    profile=replace(m.ORIGINAL,report_type='4')
    source='<form><select name="TIP_Informe"><option value="1">Inicial</option></select></form><script>'+_INITIALIZER.replace("'__VALUE__'","'4'")+'</script>'
    form=m.root_html(source).xpath('//form')[0]
    assert m.response_fields(form,profile)['TIP_Informe']=='4'
    from flujo_csmp import plan
    from datetime import date
    phases=plan(('113',),('2',),'Espera',date(2026,10,6))
    assert [batch.screen for _,batch in phases]==['seguimiento','calendario_informes','calendario_informes']
    previous=preferencias.data_directory
    folder=destination.parent/('.descargador-package-check-'+uuid4().hex);folder.mkdir()
    try:
        from prueba_bitacoras_paquete import check as check_diary
        assert check_diary(folder/'bitacoras')
        preferencias.data_directory=lambda:folder
        root=tk.Tk();root.withdraw();app=None
        try:
            app=descargador.Application(root,start_worker=False)
            root.update()
            result={'version':manifest['version'],'source_commit':BUILD_COMMIT,'frozen':True,
                    'executable':executable.name,'executable_sha256':exe_hash,'resource_sha256':resources,
                    'extension_files':len(resources),'joint_button':bool(app.joint_button.winfo_exists()),
                    'bitacora_button_verified':bool(app.diary_button.winfo_exists()),'shared_bitacora_parser_verified':True,
                    'pause_button_verified':bool(app.pause_button.winfo_exists()),'appearance_verified':app.theme.get() in ('Claro','Oscuro','Sistema'),
                    'frozen_bitacora_roundtrip_verified':True,
                    'calendar_dash_dates_verified':True,'measure_calendar_header_verified':True,
                    'dynamic_report_filter_verified':True,'joint_excludes_signatures_verified':True,'ok':True}
        finally:
            if app is not None:app.bridge.close();app.__dict__.clear()
            root.destroy();gc.collect()
            preferencias.data_directory=previous
    finally:shutil.rmtree(folder)
    destination.write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
