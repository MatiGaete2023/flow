"""Read the selected report literal in SITFA's audited option initializer.

The server sends the initial report SELECT with options 1/2/3 even for
Cumplimiento. cargarOpciones then replaces it with 0/4/5 and selects the
server's literal. Only this known initializer is recognized; no JS is run.
"""
import re


_INITIALIZER = '''function cargarOpciones(lengueta){
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
}'''

_LITERAL = r'''(?:"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')'''
_pattern = re.escape(re.sub(r'\s+', '', _INITIALIZER))
_pattern = _pattern.replace(re.escape("'__VALUE__'"), r'''(?P<selected>'[0-5]'|"[0-5]")''')
_pattern = _pattern.replace(re.escape('"__LABEL__"'), _LITERAL)
_KNOWN = re.compile(_pattern)


def selected_report(root, tab):
    """Return a validated literal, or None for a plain HTML selector."""
    from motor import PocError
    scripts = '\n'.join(root.xpath('//script/text()'))
    definitions = list(re.finditer(r'\bfunction\s+cargarOpciones\s*\(', scripts))
    if not definitions:
        return None
    if len(definitions) != 1:
        raise PocError('El selector dinámico de vencimiento es ambiguo.')
    block = scripts[definitions[0].start():]
    # The audited helper has no comments inside strings. Labels only display
    # text; altered code, expressions and additional statements are rejected.
    block = re.sub(r'/\*.*?\*/|//[^\r\n]*', '', block, flags=re.S)
    compact = re.sub(r'\s+', '', block)
    match = _KNOWN.match(compact)
    if not match:
        raise PocError('Cambió el selector dinámico de vencimiento; no se pudo comprobar el filtro aplicado.')
    selected = match['selected'][1:-1]
    allowed = {'tdCumplimiento': {'0', '4', '5'}, 'tdInforme': {'1', '2', '3'}}
    if selected not in allowed.get(tab, {'0'}):
        raise PocError('El filtro de vencimiento recibido no corresponde a la pestaña seleccionada.')
    return selected
