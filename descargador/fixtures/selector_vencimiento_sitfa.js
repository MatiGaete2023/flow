function cargarOpciones(lengueta){

	var lengueta = lengueta;

	var select = document.getElementById("TIP_Informe");

	

	var valorSeleccionado = '0';

	

	select.innerHTML = '';



    

	if (lengueta == 'tdInforme') {

	    var options = [

	        { text: "Por vencer (a 25 días)", value: 1 },

	        { text: "Vencidos", value: 2 },

	        { text: "Recibidos", value: 3 }

	    ];

	} else if (lengueta == 'tdCumplimiento') {

	    var options = [

	        { text: "...", value: 0 },

	        { text: "Por vencer (a 45 días)", value: 4 },

	        { text: "Vencidos", value: 5 }

	    ];

	} else {

	    var options = [

	        { text: "...", value: 0 }

	    ];

	}

	

	for (var i = 0; i < options.length; i++) {

	    var newOption = document.createElement("option");

	    newOption.value = options[i].value;

	    newOption.innerText = options[i].text;

	    select.appendChild(newOption);

	}

	

	// Array de lista de valores del select creado.

	var validValues = [];

	for (var i = 0; i < select.options.length; i++) {

		validValues.push(select.options[i].value);

	}



	  

	var isValid = false;

	for (var j = 0; j < validValues.length; j++) {

		if ( valorSeleccionado == validValues[j]) {

	    	isValid = true;

	    	break;

	    }

	}



	if (isValid) {

		document.InformesPpalForm.TIP_Informe.value= valorSeleccionado;

	}

}
