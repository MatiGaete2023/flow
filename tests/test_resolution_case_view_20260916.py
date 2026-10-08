from nurus.personal.resolutions import unique_case_selections
from nurus.personal.work import Row


def _work():
    rows=[
        Row('a',2,{'RIT':'X-1','TRIBUNAL':'Jgdo. L. y G. de Mulchén'},'',[],[],[],False,{}),
        Row('b',3,{'RIT':'X-1','TRIBUNAL':'Jgdo. L. y G. de Mulchén'},'',[],[],[],False,{}),
        Row('c',4,{'RIT':'X-2','TRIBUNAL':'Jgdo. L. y G. de Mulchén'},'',[],[],[],False,{}),
    ]
    work=type('Work',(),{})()
    work.rows=rows;work.mapping={'rit':'RIT','tribunal':'TRIBUNAL'}
    return work


def test_resolution_list_deduplicates_same_case_and_kind_but_keeps_other_kind():
    work=_work()
    selections=[('a','PC_IE'),('b','PC_IE'),('a','PC_INFO'),('c','PC_IE')]
    assert unique_case_selections(work,selections)==[
        ('a','PC_IE'),('a','PC_INFO'),('c','PC_IE')
    ]
