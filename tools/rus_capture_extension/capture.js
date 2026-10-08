/* Función autónoma inyectada en cada documento accesible de la pestaña actual. */
function captureRusDocument() {
  if (location.origin !== "https://familia.pjud.cl") {
    return { skipped: true, reason: "Documento fuera del origen institucional admitido." };
  }
  const secret = /password|passwd|clave|contras|token|csrf|xsrf|session|sesion|cookie|authorization|credential|login/i;
  const identities = /^(COD_(Tribunal(?:_sel|Actual|Origen)?|Causa|Rus|Ingreso|Persona|Centro|Etapa|Modalidad)|FLG_Antiguo|TIP_Consulta|TIP_Causa|ROL_Causa|ERA_Causa)$/i;
  const path = raw => {
    try { const url = new URL(raw, location.href); return url.origin + url.pathname; }
    catch (_) { return ""; }
  };
  const visible = e => !!(e.getClientRects().length && getComputedStyle(e).visibility !== "hidden");
  // No se exportan scripts, HTML original, cabeceras ni almacenes de autenticación.
  const describe = e => {
    const sensitive = e.type === "password" || secret.test([e.name, e.id, e.autocomplete].join(" "));
    const hidden = e.type === "hidden" || !visible(e);
    const permitted = !sensitive && (!hidden || identities.test(e.name));
    const result = {
      tag: e.tagName.toLowerCase(), type: e.type || "", name: e.name || "", id: e.id || "",
      labels: Array.from(e.labels || []).map(x => x.innerText.trim()),
      visible: visible(e), disabled: !!e.disabled, required: !!e.required,
      max_length: e.hasAttribute("maxlength") ? e.getAttribute("maxlength") : null,
      value: permitted ? e.value : null, redacted: !permitted,
      text: !sensitive && e.tagName === "BUTTON" ? e.innerText.trim() : "",
      checked: !sensitive && ["checkbox", "radio"].includes(e.type) ? e.checked : null
    };
    if (e.tagName === "SELECT" && !sensitive) result.options = Array.from(e.options)
      .map(o => ({ value: o.value, text: o.textContent.trim(), selected: o.selected, disabled: o.disabled }));
    return result;
  };
  const forms = Array.from(document.forms).map((form, index) => ({
    index, id: form.id, name: form.name, action: path(form.action), method: form.method,
    controls: Array.from(form.elements).filter(e => /^(INPUT|SELECT|TEXTAREA|BUTTON)$/.test(e.tagName)).map(describe)
  }));
  const unbound_controls = Array.from(document.querySelectorAll("input,select,textarea,button"))
    .filter(e => !e.form).map(describe);
  const tables = Array.from(document.querySelectorAll("table")).filter(visible).map((table, index) => ({
    index, id: table.id, rows: Array.from(table.rows).map(row => ({
      id: row.id, cells: Array.from(row.cells).map(cell => cell.innerText)
    }))
  }));
  // La captura de una pantalla nunca acredita cobertura completa o fin de paginación.
  const links = Array.from(document.querySelectorAll("a")).filter(visible).map(e => ({
    id: e.id, text: e.innerText.trim(), href: /^(https?:|\/|\.)/.test(e.getAttribute("href") || "") ? path(e.href) : "",
    scripted: e.hasAttribute("onclick") || (e.getAttribute("href") || "").startsWith("javascript:")
  }));
  return { schema: "rus-document-v1", observed_at: new Date().toISOString(),
    url: path(location.href), title: document.title, forms, unbound_controls, tables, links,
    capabilities: { writes: false, complete_entries: false, server_time_verified: false },
    redactions: "Contraseñas, campos de autenticación y valores ocultos ajenos a identidad omitidos. URLs sin parámetros."
  };
}
