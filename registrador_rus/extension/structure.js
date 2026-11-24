/* Detector estructural de candidatos basado en V1.5/V1.6. Solo lectura. */
(() => {
  'use strict';
  const fail = code => { throw new Error(code); };
  const normalize = value => String(value ?? '').replace(/\s+/g, ' ').trim().toLowerCase();
  const modalSelector = 'dialog, [role="dialog"], [aria-modal="true"]';
  function visible(node) {
    if (!node || !node.isConnected || !node.getClientRects().length) return false;
    for (let n = node; n; n = n.parentElement) {
      const css = n.ownerDocument.defaultView.getComputedStyle(n);
      if (n.hidden || n.inert || n.getAttribute('aria-hidden') === 'true' || css.display === 'none' ||
          css.visibility !== 'visible' || css.opacity === '0') return false;
    }
    return true;
  }
  function kind(node) {
    if (node.tagName === 'SELECT') return 'select';
    if (node.tagName === 'TEXTAREA') return 'textarea';
    if (node.tagName === 'BUTTON') return 'button';
    if (node.tagName === 'INPUT' && ['submit', 'image', 'button'].includes(node.type)) return 'button';
    return node.type;
  }
  const technicalIdentity = ['COD_Tribunal', 'CRR_IdCausa', 'ID_Ingreso', 'COD_Etapa', 'TIP_Consulta'];
  const signature = ['select', 'checkbox', 'textarea', 'text', 'button', 'button'];
  function inspect(doc) {
    if ([...doc.querySelectorAll(modalSelector)].some(visible)) fail('MODAL_VISIBLE');
    const forms = [...doc.forms].filter(f => f.name === 'InformesPpalForm' && visible(f));
    if (forms.length !== 1) fail('FORMULARIO_NO_UNICO');
    const form = forms[0];
    if (form.method.toLowerCase() !== 'post') fail('METODO_INESPERADO');
    const tables = [...doc.querySelectorAll('[id="TablaInforme"]')];
    if (tables.length !== 1 || tables[0].tagName !== 'TABLE' || !form.contains(tables[0]) || !visible(tables[0])) fail('TABLA_NO_UNICA');
    // Se inspecciona existencia de campos de contexto, nunca sus valores.
    for (const name of technicalIdentity) {
      const hits = [...form.querySelectorAll('input')].filter(n => n.name === name);
      if (hits.length !== 1 || hits[0].type !== 'hidden' || hits[0].form !== form) fail('CONTEXTO_ESTRUCTURAL_INCOMPLETO');
    }
    const controls = [...form.querySelectorAll('input,select,textarea,button')].filter(visible);
    if (controls.length !== signature.length || controls.some((n,i) => kind(n) !== signature[i] ||
        n.form !== form || n.matches(':disabled') || n.closest(modalSelector) || /^HOBS_/.test(n.name))) fail('FIRMA_DISTINTA');
    const [state, checkbox, editor, auxiliary, create, other] = controls;
    if (state.name !== 'COD_Estado_Detalle_Nuevo' || state.multiple || state.options.length !== 3) fail('ESTADO_DISTINTO');
    if (editor.readOnly || checkbox.indeterminate || create.name !== 'irAccion' || other.name === 'irAccion') fail('CONTROLES_AMBIGUOS');
    const labels = [...doc.querySelectorAll('label')].filter(n => visible(n) && normalize(n.textContent) === 'enviar observación al tribunal');
    if (labels.length > 1 || (labels.length === 1 && labels[0].control && labels[0].control !== checkbox)) fail('ETIQUETA_CONTRADICTORIA');
    const labelAssociated = labels.length === 1 && labels[0].control === checkbox;
    const buttonLabel = normalize(create.tagName === 'INPUT' ? create.value : create.textContent);
    if (buttonLabel === 'grabar' || normalize(other.tagName === 'INPUT' ? other.value : other.textContent) === 'nueva obs') fail('ACCION_CONTRADICTORIA');
    return Object.freeze({schema: 'StructuralCandidateV1', evidence_state: 'CANDIDATO_NO_VALIDADO_EN_RUS',
      matches: true, visible_controls: signature.length, state_options: state.options.length,
      destination_label_associated: labelAssociated, create_label_matches: buttonLabel === 'nueva obs',
      editor_maxlength: editor.maxLength >= 0 ? editor.maxLength : null,
      context_fields_present: true, write_enabled: false});
  }
  globalThis.RegistradorStructuralCandidate = Object.freeze({inspect});
})();
