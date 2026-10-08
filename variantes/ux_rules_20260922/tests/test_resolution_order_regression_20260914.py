from openpyxl import Workbook

from nurus.personal.config import defaults
from nurus.personal.resolutions import automatic_project_selections
from nurus.personal.work import Work


def test_reviewed_resolution_types_follow_workbook_order(tmp_path):
    book=Workbook();sheet=book.active;sheet.title='Registros'
    sheet.append(['RIT','TRIBUNAL','NOMBRE','RUT','DERIVACION','OBSERVACION','RES'])
    sheet.append(['X-TEST-1','LAJA','Persona Alfa','RUT-ALFA','PRM EJEMPLO','Texto','PC_INFO'])
    sheet.append(['X-TEST-1','LAJA','Persona Beta','RUT-BETA','PRM EJEMPLO','Texto','PC_IE'])
    path=tmp_path/'orden_res.xlsx';book.save(path)

    work=Work.external(path,defaults())
    selected=automatic_project_selections(work,'NOMENCL')

    assert selected==[
        (work.rows[0].id,'PC_INFO'),(work.rows[0].id,'PC_IE'),
        (work.rows[1].id,'PC_INFO'),(work.rows[1].id,'PC_IE'),
    ]
