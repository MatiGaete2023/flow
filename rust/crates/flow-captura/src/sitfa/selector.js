function cargarOpciones(lengueta){
    var lengueta = lengueta;
    var select = document.getElementById("TIP_Informe");
    var valorSeleccionado = '__VALUE__';
    select.innerHTML = '';
    if (lengueta == 'tdInforme') {
        var options = [ { text: "__LABEL__", value: 1 },
                        { text: "__LABEL__", value: 2 },
                        { text: "__LABEL__", value: 3 } ];
    } else if (lengueta == 'tdCumplimiento') {
        var options = [ { text: "__LABEL__", value: 0 },
                        { text: "__LABEL__", value: 4 },
                        { text: "__LABEL__", value: 5 } ];
    } else { var options = [ { text: "__LABEL__", value: 0 } ]; }
    for (var i = 0; i < options.length; i++) {
        var newOption = document.createElement("option");
        newOption.value = options[i].value;
        newOption.innerText = options[i].text;
        select.appendChild(newOption);
    }
    var validValues = [];
    for (var i = 0; i < select.options.length; i++) {
        validValues.push(select.options[i].value);
    }
    var isValid = false;
    for (var j = 0; j < validValues.length; j++) {
        if ( valorSeleccionado == validValues[j]) { isValid = true; break; }
    }
    if (isValid) { document.InformesPpalForm.TIP_Informe.value= valorSeleccionado; }
}
