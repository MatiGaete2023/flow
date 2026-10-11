"""Formulario SITFA enteramente ficticio, compartido por las verificaciones de extensión."""

import motor as m
from perfiles import TABS
from test_lotes import CATALOG


def master():
    fields = {
        'COD_Lengueta': '',
        'FLG_Consulta': '0',
        'FEC_Inicio': '01/09/2026',
        'FEC_Fin': '30/09/2026',
        'RUT_Consulta': '12345678',
        'RUT_DvConsulta': '9',
        'ROL_Causa': '',
        'ERA_Causa': '0',
        'TIP_Causa': '0',
    }
    fields.update({k: '' for k in m.PAGINATION if k in m.ALLOWED_FIELDS})
    source = '<html><meta charset="utf-8"><body><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">'
    source += ''.join(f'<input name="{k}" value="{v}">' for k, v in fields.items())
    source += '<input type="checkbox" name="CHK_Consulta"><select name="COD_CentroResidencial"><option value="-1">Todos</option><option value="7" selected>Centro ficticio</option></select>'
    for name, key in [('COD_Tribunal_sel', 'tribunales'), ('TIP_Consulta', 'modalidades'), ('TIP_Informe', 'informes')]:
        source += (
            f'<select name="{name}" id="{name}" onchange="document.InformesPpalForm.COD_CentroResidencial.value=\'-1\';">'
            + ''.join(f'<option value="{k}">{v}</option>' for k, v in CATALOG[key].items())
            + '</select>'
        )
    source += '<input type="submit" name="irAccion" value="Buscar medida"></form>'
    source += ''.join(
        f'<a id="{cfg[0]}" onclick="Lengueta(\'{cfg[0]}\')">{label}</a><table id="{cfg[3]}"></table>'
        for label, cfg in TABS.items()
    )
    source += '''<script>
    var idLengueta='tdEspera';
    function Lengueta(id){idLengueta=id;var s=document.getElementById('TIP_Informe');s.innerHTML='';
      var values=id==='tdInforme'? [['1','Por vencer'],['2','Vencidos'],['3','Recibidos']]:[['0','...'],['4','Medidas por vencer'],['5','Medidas vencidas']];
      for(var x of values)s.add(new Option(x[1],x[0]));s.disabled=id==='tdEspera';}
    function Seleccion(){var f=document.InformesPpalForm;f.FEC_Inicio.disabled=f.FEC_Fin.disabled=!f.CHK_Consulta.checked;}
    function Envio(){var f=document.InformesPpalForm;
      if(f.RUT_Consulta.value==='0'){alert('fixture: debe normalizarse después');return false;}
      f.COD_Lengueta.value=idLengueta;f.FLG_Consulta.value=f.CHK_Consulta.checked?'1':'0';
      if(!f.CHK_Consulta.checked){f.FEC_Inicio.disabled=false;f.FEC_Fin.disabled=false;}
      if(!f.RUT_Consulta.value){f.RUT_Consulta.value='0';f.RUT_DvConsulta.value='0';}
      if(!f.ROL_Causa.value)f.ROL_Causa.value='0';
    }
    Lengueta('tdEspera');Seleccion();
    </script></body></html>'''
    return source
