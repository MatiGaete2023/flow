'use strict';
const button=document.getElementById('inspect');
const output=document.getElementById('result');
button.addEventListener('click',async()=>{
  button.disabled=true;
  output.value='';
  try {
    const tabs=await chrome.tabs.query({active:true,currentWindow:true});
    if(tabs.length!==1||typeof tabs[0].id!=='number'||new URL(tabs[0].url).origin!=='https://familia.pjud.cl') {
      output.value='ORIGEN_NO_PERMITIDO';
      return;
    }
    const results=await chrome.scripting.executeScript({target:{tabId:tabs[0].id},files:['inspect.js'],world:'ISOLATED'});
    if(results.length!==1||!results[0].result||results[0].result.schema!=='FocusedEvidenceV1'||results[0].result.write_enabled!==false) {
      output.value='INSPECCION_NO_CONCLUYENTE';
      return;
    }
    output.value=JSON.stringify(results[0].result,null,2);
  } catch (_) {output.value='INSPECCION_NO_DISPONIBLE';}
  finally {button.disabled=false;}
});
