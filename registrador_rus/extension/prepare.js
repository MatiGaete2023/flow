/* P3: adaptador sintético. No ejecutar en sitios institucionales. */
(() => {
  'use strict';
  const origins = new Set(['ESPERA', 'CUMPLIMIENTO', 'INFORMES_POR_VENCER', 'INFORMES_VENCIDOS']);
  const fail = code => { throw new Error(code); };
  const fixture = doc => {
    if (doc.location.href !== 'about:blank' || doc.documentElement.dataset.fixture !== 'P3') {
      fail('ENTORNO_NO_SINTETICO');
    }
  };
  const identity = value => ['system', 'tribunal_id', 'causa_id', 'ingreso_id'].map(k => value?.[k]);
  const sameIdentity = (a, b) => identity(a).every((v, i) => typeof v === 'string' && v.length && v === identity(b)[i]);
  const decimal = '\\d{1,20}';
  const arg = `(?:(${decimal})|'(${decimal})'|"(${decimal})")`;
  const call = new RegExp('^\\s*(?:javascript:\\s*)?ShowObservaciones\\s*\\(\\s*' +
    Array(6).fill(arg).join('\\s*,\\s*') + '\\s*\\)\\s*;?\\s*$');
  function parseLinkContext(source) {
    const match = call.exec(source || '');
    if (!match) return null;
    const args = Array.from({length: 6}, (_, i) => match[1 + i * 3] ?? match[2 + i * 3] ?? match[3 + i * 3]);
    if (!['11', '12', '13'].includes(args[3])) return null;
    return Object.freeze({identity:Object.freeze({system: 'SITFA', tribunal_id: args[1], causa_id: args[0], ingreso_id: args[2]}), tipo_popUp:args[3], COD_Etapa:args[4], antiguo:args[5]});
  }
  function parseLink(source) { return parseLinkContext(source)?.identity ?? null; }
  function locate(doc, op) {
    fixture(doc);
    if (!origins.has(op.origin) || doc.body.dataset.menu !== '89' || doc.body.dataset.origin !== op.origin) fail('ORIGEN_INCORRECTO');
    const tables = [...doc.querySelectorAll('table[data-current="true"]')].filter(t => t.getClientRects().length && !t.closest('[hidden]'));
    if (tables.length !== 1) fail('LISTADO_NO_UNICO');
    const hits = [];
    for (const row of [...tables[0].tBodies].flatMap(body => [...body.rows])) {
      if (!row.getClientRects().length || row.closest('[hidden]')) continue;
      const links = [...row.querySelectorAll('a[onclick], a[href]')].filter(a => a.getClientRects().length);
      const parsed = links.flatMap(a => ['onclick', 'href'].map(k => [a, parseLink(a.getAttribute(k))]).filter(([, id]) => id));
      const matching = parsed.filter(([, id]) => sameIdentity(id, op.remote_identity));
      if (matching.length && parsed.length !== 1) fail('FILA_AMBIGUA');
      if (matching.length) hits.push(Object.freeze({row, link: matching[0][0], identity: matching[0][1], origin: op.origin}));
    }
    if (hits.length !== 1) fail('INGRESO_NO_UNICO');
    return hits[0];
  }
  function canonical(op) {
    return JSON.stringify([op.schema, op.operation_id, op.source?.tool, op.source?.work_id, op.source?.row_ref,
      ...identity(op.remote_identity), op.origin, op.destination, op.observation_state, op.approved_text,
      op.approved_text_sha256, op.approved_by, op.approved_at_utc, op.approval_version]);
  }
  // Both contracts must have passed the Python P1 validator at the local host boundary.
  function prepare(doc, op, current, listed) {
    fixture(doc);
    const frozen = canonical(op);
    if (frozen !== canonical(current) || op.schema !== 'RegistroFlowV1' || !Number.isInteger(op.approval_version) ||
        op.approval_version < 1 || typeof op.approved_text !== 'string' || !op.approved_text.trim() ||
        !['ADMINISTRATIVA', 'AL_TRIBUNAL'].includes(op.destination) || !/^[0-9a-fA-F]{64}$/.test(op.approved_text_sha256)) fail('APROBACION_INVALIDA');
    const expected = op.remote_identity;
    if (!listed || listed.origin !== op.origin || !sameIdentity(expected, listed.identity)) fail('IDENTIDAD_LISTADO_INCORRECTA');
    const detector = globalThis.RegistradorFormDetector;
    const binding = detector.inspect(doc);
    const live = locate(doc, op);
    if (live.row !== listed.row || live.link !== listed.link) fail('LISTADO_CAMBIADO');
    const formIdentity = () => ({system: binding.form.dataset.system, tribunal_id: binding.form.dataset.tribunal,
      causa_id: binding.form.dataset.causa, ingreso_id: binding.form.dataset.ingreso});
    const check = () => {
      const now = detector.inspect(doc);
      if (frozen !== canonical(op) || frozen !== canonical(current) || !sameIdentity(expected, formIdentity()) ||
          ['form','editor','checkbox','state','create'].some(k => now[k] !== binding[k])) fail('FORMULARIO_CAMBIADO');
    };
    check();
    const {editor, checkbox, state} = binding;
    const probe = doc.createElement('textarea'); probe.value = op.approved_text;
    if (probe.value !== op.approved_text || (editor.maxLength >= 0 && op.approved_text.length > editor.maxLength)) fail('TEXTO_NO_REPRESENTABLE');
    if (editor.value && editor.value !== op.approved_text) fail('BORRADOR_EXISTENTE');
    const options = [...state.options].filter(o => o.value === op.observation_state && !o.disabled && !o.closest('optgroup[disabled]'));
    if (state.multiple || options.length !== 1) fail('ESTADO_NO_DISPONIBLE');
    const desired = op.destination === 'AL_TRIBUNAL';
    const set = (el, key, value) => {
      check();
      if (el[key] !== value) {
        el[key] = value;
        el.dispatchEvent(new Event('input', {bubbles: true})); check();
        el.dispatchEvent(new Event('change', {bubbles: true})); check();
      }
    };
    set(editor, 'value', op.approved_text);
    set(state, 'value', op.observation_state);
    set(checkbox, 'checked', desired);
    check();
    if (editor.value !== op.approved_text || state.value !== op.observation_state || checkbox.checked !== desired || checkbox.indeterminate) fail('VALORES_NO_COINCIDEN');
    return Object.freeze({schema: 'PreparationResultV1', operation_id: op.operation_id, destination: op.destination,
      checked: desired, evidence_state: 'SINTETICO_NO_VALIDADO_EN_RUS', saved: false});
  }
  globalThis.RegistradorPreparation = Object.freeze({parseLink, parseLinkContext, locate, prepare});
})();
