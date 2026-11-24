/* Consumidor de demostración. No content scripts, navegación RUS ni guardado. */
(() => {
  'use strict';
  const $ = id => document.getElementById(id);
  const fail = () => { throw new Error('MENSAJE_FICTICIO_INVALIDO'); };
  const fields = (value, names) => value && Object.keys(value).sort().join(' ') === names.split(' ').sort().join(' ');
  const sorted = value => Array.isArray(value) ? value.map(sorted) : value && typeof value === 'object' ?
    Object.fromEntries(Object.keys(value).sort().map(k => [k, sorted(value[k])])) : value;
  const digest = async text => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text))),
    byte => byte.toString(16).padStart(2, '0')).join('');
  const origins = ['ESPERA','CUMPLIMIENTO','INFORMES_POR_VENCER','INFORMES_VENCIDOS'];
  async function validate(command, session) {
    if (!command || command.schema !== 'BridgeCommandV1' || command.session_id !== session || command.simulation_only !== true ||
        !/^[a-f0-9]{32}$/.test(command.command_id)) fail();
    if (command.command === 'get_capabilities') {
      if (!fields(command,'schema session_id command_id command simulation_only')) fail();
      return null;
    }
    if (command.command !== 'prepare' || !fields(command,'schema session_id command_id command simulation_only operation_id remote_identity origin approval_digest approved_operation')) fail();
    const op = command.approved_operation;
    if (!fields(op,'schema operation_id source remote_identity origin destination observation_state approved_text approved_text_sha256 approved_by approved_at_utc approval_version') ||
        !fields(op.source,'tool work_id row_ref') || !fields(op.remote_identity,'system tribunal_id causa_id ingreso_id')) fail();
    // Solo acepta la familia exacta de operaciones de demo_operations, no datos arbitrarios.
    const match = /^demo-([1-9]\d{0,2})$/.exec(op.operation_id);
    if (!match) fail();
    const n = Number(match[1]), i = n - 1, ident = op.remote_identity;
    if (n > 400 || op.schema !== 'RegistroFlowV1' || op.source.tool !== 'DEMO' || op.source.work_id !== 'lote-demo' || op.source.row_ref !== String(n) ||
        ident.system !== 'SITFA' || ident.tribunal_id !== '900000' || ident.causa_id !== String(900000 + Math.floor(i/2)) || ident.ingreso_id !== String(910000+i) ||
        op.origin !== origins[i%4] || op.destination !== (i%2?'AL_TRIBUNAL':'ADMINISTRATIVA') || op.observation_state !== 'demo' ||
        op.approved_text !== `Constancia ficticia ${n}. Sin expediente real.` || op.approved_by !== 'actor-ficticio' || op.approved_at_utc !== '2026-11-23T00:00:00Z' ||
        op.approval_version !== 1 || command.operation_id !== op.operation_id || command.origin !== op.origin ||
        JSON.stringify(sorted(command.remote_identity)) !== JSON.stringify(sorted(ident)) ||
        await digest(op.approved_text) !== op.approved_text_sha256 || await digest(JSON.stringify(sorted(op))) !== command.approval_digest) fail();
    return op;
  }
  function render(op) {
    const doc = $('fixture').contentDocument;
    if (!doc || doc.location.href !== 'about:blank') fail();
    doc.documentElement.dataset.fixture = 'P3';
    doc.body.dataset.menu = '89'; doc.body.dataset.origin = op.origin;
    doc.body.replaceChildren();
    const i = op.remote_identity;
    const table = doc.createElement('table'); table.dataset.current = 'true';
    const row = table.createTBody().insertRow(); const link = doc.createElement('a');
    link.setAttribute('href', `javascript:ShowObservaciones(${i.causa_id},${i.tribunal_id},${i.ingreso_id},11,1,0)`);
    link.textContent = `Ingreso ficticio ${i.ingreso_id}`; link.addEventListener('click', e => e.preventDefault()); row.insertCell().append(link);
    const form = doc.createElement('form'); form.name = 'InformesPpalForm';
    Object.assign(form.dataset,{system:i.system,tribunal:i.tribunal_id,causa:i.causa_id,ingreso:i.ingreso_id});
    const area = doc.createElement('textarea'); area.maxLength = 2000; area.rows = 5; area.cols = 65;
    const label = doc.createElement('label'); label.htmlFor = 'demo-check'; label.textContent = 'Enviar observación al tribunal';
    const check = doc.createElement('input'); check.type = 'checkbox'; check.id = 'demo-check';
    const select = doc.createElement('select'); select.name = 'COD_Estado_Detalle_Nuevo';
    const option = doc.createElement('option'); option.value = 'demo'; option.textContent = 'Estado ficticio'; select.append(option);
    const button = doc.createElement('button'); button.type = 'submit'; button.textContent = 'Nueva obs';
    // Nunca ejecutar un submit; el sandbox impide formularios y scripts del iframe.
    form.addEventListener('submit', e => e.preventDefault());
    form.append(area,doc.createElement('br'),label,check,select,button); doc.body.append(table,form);
    const listed = RegistradorPreparation.locate(doc, op);
    const result = RegistradorPreparation.prepare(doc, op, structuredClone(op), listed);
    if (result.saved !== false || result.checked !== (op.destination === 'AL_TRIBUNAL')) fail();
    button.disabled = true;
    return result;
  }
  async function receive() {
    const port = $('port').value.trim(), session = $('session').value.trim(), token = $('token').value;
    if (!/^\d{1,5}$/.test(port) || Number(port) < 1 || Number(port) > 65535 || !/^[a-f0-9]{32}$/.test(session) || !/^[A-Za-z0-9_-]{43}$/.test(token)) fail();
    const base = `http://127.0.0.1:${Number(port)}`;
    const request = async (path, body) => {
      const response = await fetch(base+path,{method:'POST',headers:{'Content-Type':'application/json',Authorization:'Bearer '+token},
        body:JSON.stringify(body),cache:'no-store',redirect:'error',credentials:'omit',signal:AbortSignal.timeout(6000)});
      if (!response.ok) fail(); return response.json();
    };
    const result = await request('/v1/poll',{schema:'BridgePollV1',session_id:session});
    if (!fields(result,'command')) fail();
    if (result.command === null) { $('status').textContent = 'Sin orden pendiente'; return; }
    let status = 'BLOCKED';
    const command = result.command;
    try { const op = await validate(command,session); if (op) render(op); status='OK'; }
    catch (_) { $('fixture').contentDocument.body.replaceChildren(); }
    if (!command || !/^[a-f0-9]{32}$/.test(command.command_id)) fail();
    await request('/v1/ack',{schema:'BridgeAckV1',session_id:session,command_id:command.command_id,status});
    $('status').textContent = status === 'OK' ? 'Preparación ficticia recibida. No se guardó.' : 'Orden bloqueada. No se guardó.';
  }
  $('origin').textContent = location.origin;
  $('poll').addEventListener('click', async () => {
    $('poll').disabled = true;
    try { await receive(); } catch (_) { $('status').textContent = 'Conexión u orden rechazada. No se guardó.'; }
    finally { $('poll').disabled = false; }
  });
})();
