/* P2: detector pasivo. Validado solo en DOM ficticio; no autoriza uso real. */
(() => {
  'use strict';
  const normalize = value => String(value ?? '').replace(/\s+/g, ' ').trim().toLowerCase();
  function visible(element) {
    if (!element || !element.isConnected || !element.getClientRects().length) return false;
    for (let node = element; node; node = node.parentElement) {
      const style = node.ownerDocument.defaultView.getComputedStyle(node);
      if (node.hidden || node.inert || node.getAttribute('aria-hidden') === 'true' ||
          style.display === 'none' || style.visibility !== 'visible' || style.opacity === '0') return false;
    }
    return true;
  }
  const modal = element => element.closest('dialog, [role="dialog"], [aria-modal="true"]');
  const usable = (element, form) => element.form === form && visible(element) &&
    !element.matches(':disabled') && !modal(element);
  const unique = (items, code) => {
    if (items.length !== 1) throw new Error(code);
    return items[0];
  };
  function inspect(document) {
    if ([...document.querySelectorAll('dialog, [role="dialog"], [aria-modal="true"]')].some(visible)) {
      throw new Error('MODAL_VISIBLE');
    }
    const form = unique([...document.forms].filter(f => f.name === 'InformesPpalForm' && visible(f)),
      'FORMULARIO_NO_UNICO');
    const controls = [...form.elements].filter(e => usable(e, form));
    const editor = unique(controls.filter(e => e.tagName === 'TEXTAREA' && !e.readOnly), 'EDITOR_NO_UNICO');
    const labels = [...document.querySelectorAll('label')].filter(l => visible(l) &&
      normalize(l.textContent) === 'enviar observación al tribunal');
    const label = unique(labels, 'ETIQUETA_DESTINO_NO_UNICA');
    if (label.htmlFor && [...document.querySelectorAll('[id]')].filter(e => e.id === label.htmlFor).length !== 1) {
      throw new Error('CASILLA_NO_VINCULADA');
    }
    const checkbox = label.control;
    if (!controls.includes(checkbox) || checkbox.tagName !== 'INPUT' || checkbox.type !== 'checkbox') {
      throw new Error('CASILLA_NO_VINCULADA');
    }
    const state = unique(controls.filter(e => e.tagName === 'SELECT' &&
      (e.name === 'COD_Estado_Detalle_Nuevo' || e.id === 'COD_Estado_Detalle_Nuevo')), 'ESTADO_NO_UNICO');
    const create = unique(controls.filter(e =>
      ((e.tagName === 'BUTTON' && ['submit', 'button'].includes(e.type)) ||
       (e.tagName === 'INPUT' && ['submit', 'button'].includes(e.type))) &&
      normalize(e.tagName === 'INPUT' ? e.value : e.textContent) === 'nueva obs'), 'CREAR_NO_UNICO');
    return Object.freeze({schema: 'FormBindingV1', mode: 'CREATE_NEW_ONLY',
      evidence_state: 'SINTETICO_NO_VALIDADO_EN_RUS', form, editor, checkbox, state, create});
  }
  globalThis.RegistradorFormDetector = Object.freeze({inspect});
})();
