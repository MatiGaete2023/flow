"""Genera seguimiento acumulativo sin modificar fuentes ni evidencia P0.

La matriz distingue implementación, pruebas locales y aceptación institucional.
No declara gates aprobados a partir de que un archivo de test exista.
"""
import csv
from hashlib import sha256
import json
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'docs/evidencia_desarrollo_20261008'
PLAN=ROOT/'docs/plan_20261008/05_PLAN_COMPLETO_EJECUCION_NURUS_CSMP_20261008.md'
STATES={'PENDIENTE','EN_CURSO','BLOQUEADA_POR_EVIDENCIA','FALLÓ_PRUEBA',
        'IMPLEMENTADA_PENDIENTE_ACEPTACION','ACEPTADA'}
LOCAL='IMPLEMENTADA_PENDIENTE_ACEPTACION'
PHASES={f'P{i}':LOCAL for i in range(14)}
PHASES.update(P1='BLOQUEADA_POR_EVIDENCIA',P7='EN_CURSO',P9='BLOQUEADA_POR_EVIDENCIA',
              P10='BLOQUEADA_POR_EVIDENCIA',P12='BLOQUEADA_POR_EVIDENCIA')
DEPENDENCIES={
 'P0':'Continuación autorizada; conservar auditoría y golden del SHA original.',
 'P1':'Sesión autorizada y cotejo real por modalidad/pestaña; G1 pendiente.',
 'P2':'Contrato probado localmente; no certifica autenticidad de fuente institucional.',
 'P3':'G1 para anunciar bitácoras reales fiables; Excel 2010 pendiente.',
 'P4':'Aceptación institucional de fuente primaria, identidad y cobertura.',
 'P5':'Aceptación institucional de los paquetes; pruebas locales y paridad disponibles.',
 'P6':'Instalaciones institucionales separadas; código local ya desacoplado.',
 'P7':'Preview ficticio CT-01/02; producción NO_EVALUABLE. G1/G10 pendientes.',
 'P8':'Excel 2010/Outlook 365 y aceptación humana de productos.',
 'P9':'Contrato remoto auditado de historia/PDF. Extractor NO IMPLEMENTADO.',
 'P10':'CT-03..10 NO IMPLEMENTADAS; todas CT desactivadas. Aprobación por regla y G9.',
 'P11':'INT reales y ampliadas NO EJECUTADAS; MVP local comprobado.',
 'P12':'P9/P10 y acta institucional; ampliación excluida del MVP.',
 'P13':'CI e3a6369 aprobada en Python 3.12/3.13/3.14; el build actual verifica eliminación real de carpeta y conservación de datos externos. H01–H09/M01–M03 corregidos con evidencia sintética. Acta pendiente entregada; Windows 10 institucional/Office y promoción pendientes.',
}


def main():
    OUT.mkdir(exist_ok=True)
    hashes=json.loads((ROOT/'docs/evidencia_p0/adjuntos_sha256.json').read_text(encoding='utf-8'))
    for name,digest in hashes.items():
        if sha256((ROOT/'docs/plan_20261008'/name).read_bytes()).hexdigest()!=digest:
            raise SystemExit('Original modificado: '+name)
    with (ROOT/'docs/evidencia_p0/matriz_funcional.csv').open(encoding='utf-8',newline='') as stream:
        records=list(csv.DictReader(stream))
    updates={}
    def update(ids,code,tests,limit,state=LOCAL,result='PASÓ'):
        for key in ids.split():
            if key in updates:raise ValueError('Requisito repetido: '+key)
            updates[key]={'state':state,'local_result':result,'code':code,'tests':tests,'limit':limit}
    update('D-T11 F06 INT-10 INT-25','src/nurus/capture/diaries.py;descargador/ventana_resultados.py',
           'tests/test_capture_temporal_products.py;descargador/test_export_csmp_capture.py;descargador/test_reutilizacion_local.py;descargador/test_experiencia.py',
           'Excel automático/autónomo, texto íntegro y recuperación offline de 800 bitácoras; reutilización solo por tribunal/causa/ingreso y fecha original conservada. G1/Excel 2010 pendientes.')
    update('D-T03','descargador/lotes.py;descargador/extension/pagina.js',
           'descargador/test_lotes.py;descargador/verificar_pdf_extension.py',
           'PDF vacío con consulta solicitada visible en captura Chrome nativa: etiquetas/controles restablecidos, tribunal/modalidad/pestaña/fechas y píxeles comprobados. Filtro institucional por operador pendiente.')
    update('D-T09','descargador/vinculos.py;descargador/lectura_bitacoras.py;src/nurus/bitacora_html.py',
           'descargador/test_lectura_bitacoras.py;descargador/test_bitacoras_extension.cjs;descargador/verificar_bitacoras_extension.py',
           'HAR confirma enlaces de seis argumentos y tipos 11/13 para Espera/Informes; 80 lecturas ficticias con tipos 11/12/13 y UTF-8/Windows-1252. Respuestas reales 11/13, demás pantallas y modalidades requieren contraste institucional.')
    update('D-T15 D-T16 F07 INT-02','descargador/export_csmp_capture.py;src/nurus/capture/contract.py',
           'descargador/test_export_csmp_capture.py;tests/test_capture_contract.py;tests/test_capture_vertical.py;descargador/test_reutilizacion_local.py;descargador/test_experiencia.py',
           'ZIP completo/parcial desde varias descargas y bitácoras anteriores comprobadas; resume coincidencias/exclusiones y conserva antigüedad/cobertura. Fuente institucional no certificada por fixtures.')
    update('A-T01 A-T03 A-T04 A-T05 A-T06 A-T08 A-T14 A-T17 F08 F09 INT-09 INT-20 INT-21 INT-22',
           'src/nurus/personal/capture_input.py;src/nurus/personal/work.py',
           'tests/test_capture_input.py;tests/test_capture_gui.py;tests/test_review_corrections.py;tests/test_capture_contract.py',
           'Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente.')
    update('A-T02 F05 INT-12','src/nurus/capture/contract.py;src/nurus/capture/temporal.py',
           'tests/test_capture_contract.py;tests/test_review_corrections.py;tests/test_capture_temporal_products.py',
           'Fuentes ausentes e inciertas explícitas; contexto de producción NO_EVALUABLE.')
    update('A-T09 R-T29 F12 INT-16 INT-23','src/nurus/personal/product_state.py;src/nurus/personal/record_edits.py',
           'tests/test_capture_input.py;tests/test_capture_temporal_products.py;tests/test_personal_batch_revision.py',
           'Reanálisis/reimportación conserva decisiones; conciliación explícita de fuentes cambiadas/ambiguas y remapeo de IDs/productos; fuente/regla/corte invalida productos preparados.')
    update('A-T15 A-T16 A-T19 INT-24 INT-26','src/nurus/personal/app_base.py;src/nurus/personal/registration.py',
           'tests/test_capture_gui.py;tests/test_capture_temporal_products.py',
           'Sin lanzador remoto principal; submit rechaza antes de tocar adaptador. Marcas locales no persistidas como autoridad.')
    update('A-T10 F13 INT-17','src/nurus/personal/capture_input.py;src/nurus/personal/outputs.py',
           'tests/test_capture_input.py;tests/test_mail_attachment_regressions_20260930.py;tests/test_integral_ui.py',
           'Calendario independiente y métricas de nómina desde fuente, aun sin nueva observación; adjuntos automáticos/editables opcionalmente, prueba de 25 correos con edición del décimo y 24 adjuntos intactos. Repreparación conserva cambios; recepción no inferida.')
    update('A-T11 A-T13 R-T31 INT-19','src/nurus/personal/outputs.py;src/nurus/personal/capture_products.py',
           'tests/test_capture_temporal_products.py;tests/test_outlook_safety.py',
           'Contactos desconocidos vacíos, borrador sin Send y advertencia de registro no acreditado. Outlook real pendiente.')
    update('A-T07 F11 INT-15','src/nurus/personal/capture_input.py;src/nurus/personal/capture_products.py',
           'tests/test_capture_gui.py;tests/test_capture_input.py;tests/test_integral_ui.py',
           'Hallazgos/cobertura por ingreso e incidencias persistidas; UI local y benchmark de importación 2000/10000/20000; máximo operativo 20000 registros. No certifica productividad institucional.',state='EN_CURSO')
    update('F15','src/nurus/capture/requests.py;descargador/solicitud_antecedentes.py',
           'tests/test_capture_temporal_products.py;tests/test_review_corrections.py','Solicitud declarativa por fuente/ingreso; límite compartido 4 MiB UTF-8/20000 entradas antes de publicación. No ejecuta consultas.')
    update('D-T20 A-T18','src/nurus/personal/package_check.py;descargador/verificar_paquete.py;tools/build_integral.ps1;tools/windows_process_check.ps1;tools/verify_installer.ps1;tools/flow_dependency_inventory.py',
           'tests/test_capture_build_identity.py;tests/test_capture_selfcheck.py;tests/test_windows_installer_contract.py;tests/test_windows_process_check.py;tests/test_installer_removal.py;tests/test_dependency_inventory.py;tests/test_current_release_contract.py;.github/workflows/verify-delivered-installer.yml',
           'CI e3a6369 SUCCESS: 615 aprobadas/1 omitida por versión Windows, productos v1 desde EXE autónomo/instalado y paquete SHA coherente; el instalador actual acredita eliminación real y datos externos intactos. Dependencias bloqueadas por Python/plataforma y verificadas. Actas separan evidencia ficticia/institucional. Windows 10 institucional/Office pendientes.')
    update('F10 R-T17 R-T18 R-T19 R-T20 R-T21 INT-14','src/nurus/capture/temporal.py',
           'tests/test_capture_temporal_products.py','Preview ficticio acotado a cuatro meses y mismo ingreso; interpretación real NO_EVALUABLE.',
           state='EN_CURSO',result='NO EJECUTADO')
    update('D-T12 D-T13 F04 INT-11 R-T22 R-T23 R-T24 R-T25 R-T26 R-T32 INT-13',
           'contracts/csmp_context_v1/catalog.json','',
           'Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados.',
           state='BLOQUEADA_POR_EVIDENCIA',result='NO EJECUTADO')
    update('INT-27','src/nurus/personal/work.py;tools/flow_baseline_golden.py',
           'tests/test_personal_flow.py;tests/test_current_work_features.py',
           'Paridad de 17 productos baseline y regresión local. No mezclar ramas históricas ni presumir aceptación Office.')
    all_ids={r['id'] for r in records}
    if len(all_ids)!=115 or not set(updates)<=all_ids:raise ValueError('IDs de fuente no coinciden')
    for row in records:
        row['baseline_state']=row['state'];row['local_result']='NO EJECUTADO'
        row['state']=LOCAL if row['state']=='IMPLEMENTADO' else 'EN_CURSO'
        if row['baseline_state']=='BLOQUEADO POR PRUEBA REAL':row['state']='BLOQUEADA_POR_EVIDENCIA'
        row.update(updates.get(row['id'],{}))
        if row['id'] not in updates:row['limit']='Baseline preservado; '+row['limit']
        row['institutional_result']='NO EJECUTADO'
        for field in ('code','tests'):
            for path in filter(None,row[field].split(';')):
                if not (ROOT/path).is_file():raise ValueError('Ruta de evidencia inexistente: '+path)
        if row['state'] not in STATES:raise ValueError('Estado inválido')
    fields=list(records[0])
    with (OUT/'matriz_actual.csv').open('w',encoding='utf-8',newline='') as stream:
        writer=csv.DictWriter(stream,fieldnames=fields,lineterminator='\n');writer.writeheader();writer.writerows(records)
    (OUT/'matriz_actual.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    tasks=[];phase=None;ordinal=0
    for number,line in enumerate(PLAN.read_text(encoding='utf-8').splitlines(),1):
        header=re.match(r'^### (P\d+) —',line)
        if header:phase=header[1];ordinal=0
        if line.startswith('## 7.'):phase=None
        if not phase:continue
        explicit=re.match(r'^\| (P\d+\.\d+) \| (.*?) \|',line)
        special=re.match(r'^\*\*(P0\.07) — (.*?)\*\*',line)
        bullet=re.match(r'^(?:- |\d+\. )(.*)',line)
        if explicit or special:task_id,description=(explicit or special).groups()
        elif bullet:
            ordinal+=1;task_id=phase+'-E'+str(ordinal).zfill(2);description=bullet[1]
        else:continue
        tasks.append({'id':task_id,'phase':phase,'description':description,'source_line':number,
                      'state':PHASES[phase],'open_dependency':DEPENDENCIES[phase]})
    for phase in ('P11','P12'):
        # Sus entregables están expresados en prosa en el plan, no como subtareas.
        tasks.append({'id':phase+'-ACEPTACION','phase':phase,'description':'Aceptación integral según gate '+phase.replace('P','G'),
                      'source_line':341 if phase=='P11' else 349,'state':PHASES[phase],'open_dependency':DEPENDENCIES[phase]})
    (OUT/'tareas_plan.json').write_text(json.dumps(tasks,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    lines=['# Trazabilidad acumulativa — ejecución 2026-10-08','',
           'El usuario difirió SITFA/RUS: no hay entorno/evidencia disponibles en esta etapa.',
           '115 requisitos conservados. P0 permanece como evidencia histórica. Los IDs Pn-Enn',
           'son etiquetas locales de los entregables numerados/listados en el plan, no IDs nuevos',
           'de las especificaciones. **PASÓ** identifica evidencia local acotada; la aceptación',
           'institucional figura separadamente **NO EJECUTADO**. No se asignan porcentajes ni GO.',
           '', 'Fuente: `docs/plan_20261008/05_PLAN_COMPLETO_EJECUCION_NURUS_CSMP_20261008.md`.',
           'Pruebas, productos y hashes: [evidencia](evidencia_desarrollo_20261008/README.md).',
           '', '## Fases y dependencias','', '| Fase | Estado | Dependencia/alcance |','|---|---|---|']
    lines.extend(f'| {phase} | {PHASES[phase]} | {DEPENDENCIES[phase]} |' for phase in PHASES)
    lines+=['','## Todas las tareas y entregables','', '| ID | Línea del plan | Estado | Tarea |','|---|---|---|---|']
    lines.extend(f"| {t['id']} | {t['source_line']} | {t['state']} | {t['description'].replace('|','/')} |" for t in tasks)
    lines+=['','## Matriz acumulativa de requisitos','',
            '| ID | Estado | Prueba local acotada | Institucional | Límite |','|---|---|---|---|---|']
    lines.extend(f"| {r['id']} | {r['state']} | {r['local_result']} | {r['institutional_result']} | {r['limit'].replace('|','/')} |" for r in records)
    lines+=['','CSV/JSON adjuntos contienen requisito, aceptación, rutas de código/tests y línea',
            'de la especificación. NO EJECUTADO también se usa cuando los tests son parciales:',
            'su existencia o que la suite pase no demuestra el escenario integral completo.','']
    (ROOT/'docs/TRAZABILIDAD_EJECUCION_20261008.md').write_text('\n'.join(lines),encoding='utf-8')
    print(f'{len(records)} requisitos, {len(tasks)} tareas/entregables; 7 originales verificados.')


if __name__=='__main__':main()
