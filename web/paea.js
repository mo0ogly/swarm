'use strict';
/** PAEA file correspondence UI. Inputs stay exact UTF-8 strings, never parsed as
 * JavaScript numbers. No approval, effect, automatic retry or local storage of
 * documents/receipts; one submit sends them to the authenticated local service.
 */
(() => {
  const $ = id => document.getElementById(id), t = text => globalThis.SwarmI18n?.t(text) || text;
  let csrf = '', busy = false, reportRaw = null;
  const states = new Set(['PROPOSED','ADMITTED','APPROVAL_PENDING','AUTHORIZED','COMMIT_REQUESTED',
    'COMMITTED','DENIED','UNKNOWN','RECONCILED_COMMITTED','RECONCILED_NOT_COMMITTED']);
  const outcomes = new Set(['COMMITTED','DENIED','UNKNOWN','RECONCILED_COMMITTED','RECONCILED_NOT_COMMITTED']);
  /** Invalidate every earlier result on input change or a new attempt; a failed
   * request cannot leave a stale success downloadable. Input files are retained. */
  function clearReport() {
    reportRaw = null; $('paea-download').disabled = true;
    $('paea-summary').textContent = ''; $('paea-report').textContent = '';
    $('paea-identities').replaceChildren();
  }
  /** Publish a visible translated state without treating UNKNOWN as success of an effect. */
  function status(kind, message) {
    $('paea-status').dataset.state = kind; $('paea-status').textContent = t(message);
  }
  /** Require exactly one valid UTF-8 file. Fatal decoding prevents byte replacement;
   * ignoreBOM preserves BOM bytes. JSON is decoded only by the authoritative checker. */
  async function readFile(id) {
    const file = $(id).files[0];
    if (!file) throw Error(t('Sélectionner les deux fichiers JSON.'));
    const bytes = new Uint8Array(await file.arrayBuffer());
    try { return {text: new TextDecoder('utf-8', {fatal:true, ignoreBOM:true}).decode(bytes), bytes}; }
    catch { throw Error(t('Fichier UTF-8 invalide ; choisir un fichier valide.')); }
  }
  /** Return SHA256 of the actual supplied bytes to bind a response to this gesture. */
  async function digest(bytes) {
    return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), byte => byte.toString(16).padStart(2,'0')).join('');
  }
  /** Match the closed v2 shape, non-authority claims, explicit state and exact
   * raw input hashes. This detects wrong/stale responses, never proves authenticity. */
  async function validate(value, document, receipt, previous) {
    const exact = (object, keys) => object && typeof object === 'object' && !Array.isArray(object) &&
      Object.keys(object).length === keys.length && keys.every(key => Object.hasOwn(object,key));
    const hash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
    if (!exact(value, ['schema_version','scope','document','receipt_sha256','previous_state','proposed_state',
      'authority','receipt_authenticity','effect']) || value.schema_version !== 2 || value.scope !== 'offline-data-only' ||
      !exact(value.document, ['document_sha256','action_digest','intent_id']) ||
      !Object.values(value.document).every(hash) || !hash(value.receipt_sha256) ||
      value.previous_state !== previous || !outcomes.has(value.proposed_state) ||
      ['authority','receipt_authenticity','effect'].some(key => value[key] !== 'not-established') ||
      value.document.document_sha256 !== await digest(document.bytes) || value.receipt_sha256 !== await digest(receipt.bytes)) {
      throw Error(t('Rapport invalide ou différent des fichiers fournis.'));
    }
  }
  /** Render only text; supplied JSON and diagnostics are never inserted as HTML. */
  function render(value, raw) {
    const state = value.proposed_state === 'UNKNOWN' ? t('UNKNOWN — effet indéterminé ; ce contrôle ne permet pas de réengager l’action.') : value.proposed_state;
    $('paea-summary').textContent = t('État déclaré par le reçu : ') + state;
    for (const [label, text] of [[t('Empreinte du document'),value.document.document_sha256],
      [t('Empreinte du reçu'),value.receipt_sha256], [t('Empreinte de l’action'),value.document.action_digest],
      [t('Identité de l’intention'),value.document.intent_id]]) {
      const term = document.createElement('dt'), description = document.createElement('dd');
      term.textContent = label; description.textContent = text; $('paea-identities').append(term,description);
    }
    $('paea-report').textContent = raw; reportRaw = raw; $('paea-download').disabled = false;
    status('success', 'Correspondance structurelle vérifiée.');
  }
  /** One explicit submit, no retry. Disable inputs during the request and retain
   * them after refusal. JSON framing preserves the original inner JSON strings;
   * validate the entire response before exposing a report or download. */
  async function check(event) {
    event.preventDefault(); if (busy || !csrf) return;
    busy = true; clearReport(); $('paea-check').disabled = true; $('paea-inputs').disabled = true;
    status('loading', 'Vérification en cours…');
    try {
      const previous = $('paea-state').value;
      if (!states.has(previous)) throw Error(t('Choisir un état précédent.'));
      const [document, receipt] = await Promise.all([readFile('paea-document'),readFile('paea-receipt')]);
      const response = await fetch('/api/v1/protection/receipt-report', {method:'POST',cache:'no-store',
        headers:{'Content-Type':'application/json','X-Swarm-CSRF':csrf},
        body:JSON.stringify({schema_version:1,document:document.text,receipt:receipt.text,
          expected_sink:$('paea-sink').value,previous_state:previous})});
      const raw = await response.text(); let value;
      try { value = JSON.parse(raw); } catch { throw Error(t('Réponse illisible ; relancer la vérification.')); }
      if (!response.ok) {
        const code = value?.failure?.code;
        throw Error(t('Vérification refusée') + (typeof code === 'string' && /^[a-z0-9_-]+$/.test(code) ? ' · ' + code : ''));
      }
      await validate(value,document,receipt,previous); render(value,raw);
    } catch (error) {
      clearReport(); status('error', error instanceof TypeError ? 'Connexion interrompue ; relancer la vérification.' : error.message);
    } finally { busy = false; $('paea-inputs').disabled = false; $('paea-check').disabled = false; }
  }
  /** Export only a fully validated report, then release the temporary browser URL. */
  function download() {
    if (reportRaw === null || busy) return;
    const url = URL.createObjectURL(new Blob([reportRaw], {type:'application/json'}));
    const link = document.createElement('a'); link.href = url; link.download = 'paea-receipt-report.json';
    link.click(); URL.revokeObjectURL(url);
  }
  /** Bootstrap the existing protected native session. Failure keeps submit blocked
   * and the cause visible; reloading retries only this read, never an effect. */
  async function start() {
    try {
      const response = await fetch('/api/v1/native-session', {cache:'no-store'}), session = await response.json();
      if (!response.ok || session.schema_version !== 1 || !/^[a-f0-9]{64}$/.test(session.csrf_token)) throw Error();
      csrf = session.csrf_token; $('paea-check').disabled = false;
      status('empty', 'Sélectionner les fichiers puis renseigner le service et l’état précédent.');
    } catch { status('error', 'Session indisponible ; recharger la page pour reprendre.'); }
  }
  $('paea-form').addEventListener('submit', check);
  for (const event of ['input','change']) $('paea-inputs').addEventListener(event, () => {
    if (busy) return; clearReport(); status('empty','Sélectionner les fichiers puis renseigner le service et l’état précédent.');
  });
  $('paea-download').addEventListener('click', download);
  const theme = new URL(location.href).searchParams.get('theme');
  if (['etat','sombre'].includes(theme)) document.documentElement.dataset.theme = theme;
  $('paea-theme').addEventListener('click', () => {
    document.documentElement.dataset.theme = document.documentElement.dataset.theme === 'etat' ? 'sombre' : 'etat';
  });
  start();
})();
