'use strict';
/** PAEA receipt/debt/comparison file UI. Inputs stay exact UTF-8 strings, never parsed as
 * JavaScript numbers. No approval, effect, automatic retry or local storage of
 * documents/receipts; one submit sends them to the authenticated local service.
 */
(() => {
  const $ = id => document.getElementById(id), t = text => globalThis.SwarmI18n?.t(text) || text;
  const operation = document.body.dataset.paeaOperation;
  if (!['receipt-report','resolve-payment','compare'].includes(operation)) throw Error('paea-operation-invalid');
  const payment = operation === 'resolve-payment';
  const comparison = operation === 'compare', receiptView = operation === 'receipt-report';
  const emptyMessage = comparison ? 'Sélectionner les deux documents d’action.' : payment ? 'Sélectionner le registre des dettes et la demande de facture.' : 'Sélectionner les fichiers puis renseigner le service et l’état précédent.';
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
  /** Check closed objects without accepting arrays or silently ignoring extra keys. */
  function exact(object, keys) {
    return object && typeof object === 'object' && !Array.isArray(object) &&
      Object.keys(object).length === keys.length && keys.every(key => Object.hasOwn(object,key));
  }
  /** SHA256 representation only; a matching hash authenticates no producer. */
  function hash(value) { return typeof value === 'string' && /^[a-f0-9]{64}$/.test(value); }
  /** Decode the wire report without rounding its i64 registry revision. For the
   * payment view only, retain that numeric lexeme as an internal decimal string;
   * downloads keep the original raw report. Missing lexeme support refuses with
   * an actionable CLI alternative, never a narrower silent monetary policy. */
  function parseResponse(raw) {
    return JSON.parse(raw, payment ? (key, value, context) => {
      if (key !== 'registry_revision') return value;
      if (typeof value !== 'number') throw Error(t('Rapport invalide ou différent des fichiers fournis.'));
      if (!context?.source) throw Error(t('Ce navigateur ne conserve pas exactement les nombres de cette preuve. Utilisez le CLI.'));
      if (!/^[1-9][0-9]*$/.test(context.source)) throw Error(t('Rapport invalide ou différent des fichiers fournis.'));
      return context.source;
    } : undefined);
  }
  /** Match the closed v2 shape, non-authority claims, explicit state and exact
   * raw input hashes. This detects wrong/stale responses, never proves authenticity. */
  async function validate(value, document, receipt, previous) {
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
  /** Validate the closed resolver report and exact raw input hashes. Decimal/name
   * bounds mirror v1 technical representations; no policy or debt existence is
   * inferred. Internal revision text came from a numeric lexeme, not coercion. */
  async function validatePayment(value, registry, request) {
    const name = text => typeof text === 'string' && /^[A-Za-z0-9_-]{1,100}$/.test(text);
    const integer = text => typeof text === 'string' && text.length <= 19 && /^[1-9][0-9]*$/.test(text) && BigInt(text) <= 9223372036854775807n;
    const proposal = value?.proposal, action = proposal?.action;
    if (!exact(value,['schema_version','scope','authority','registry_authenticity','effect','registry_sha256','request_sha256','registry_revision','proposal']) ||
      value.schema_version !== 1 || value.scope !== 'offline-data-only' ||
      ['authority','registry_authenticity','effect'].some(key => value[key] !== 'not-established') ||
      !hash(value.registry_sha256) || !hash(value.request_sha256) || !integer(value.registry_revision) ||
      !exact(proposal,['schema_version','request_id','action']) || proposal.schema_version !== 1 || !name(proposal.request_id) ||
      !exact(action,['kind','invoice_id','invoice_version','amount_minor','currency','beneficiary_account_id']) ||
      action.kind !== 'payment.settle' || !name(action.invoice_id) || !name(action.invoice_version) ||
      !name(action.beneficiary_account_id) || !integer(action.amount_minor) ||
      typeof action.currency !== 'string' || !/^[A-Z]{3}$/.test(action.currency) ||
      value.registry_sha256 !== await digest(registry.bytes) || value.request_sha256 !== await digest(request.bytes)) {
      throw Error(t('Rapport invalide ou différent des fichiers fournis.'));
    }
  }
  /** Fixed v1 serializer fields; labels contain no supplied values or operator
   * policy. This catalogue must evolve with the shared comparison schema. */
  const comparisonFields = {
    "/action/kind": "Type d’action",
    "/action/invoice_id": "Objet de dette",
    "/action/invoice_version": "Version de facture",
    "/action/amount_minor": "Montant entier — unités mineures",
    "/action/currency": "Devise",
    "/action/beneficiary_account_id": "Compte bénéficiaire",
    "/action/repository_id": "Dépôt",
    "/action/target_ref": "Référence Git cible",
    "/action/expected_old_oid": "Ancien objet Git attendu",
    "/action/candidate_oid": "Objet Git candidat",
    "/context/project_id": "Projet",
    "/context/principal_id": "Principal déclaré",
    "/context/instance_id": "Instance déclarée",
    "/context/mandate_id": "Mandat déclaré",
    "/context/mandate_revision": "Révision du mandat",
    "/context/policy_revision": "Révision de la politique",
    "/context/policy_digest": "Empreinte de la politique",
    "/context/tool_manifest_digest": "Empreinte du manifeste d’outil",
    "/context/sources": "Sources — collection ordonnée"
};
  /** Validate a closed existing v1 comparison and bind both raw input byte sets.
   * Fixed paths must be unique/sorted; change flags must agree with the producer.
   * Hash equality alone is not used to prove identical canonical bytes. No input
   * JSON numbers are parsed, and a matching report authenticates no approval or
   * context. Refuse before rendering/download; no IO beyond local byte hashing. */
  async function validateComparison(value, before, after) {
    const c = value?.comparison, invalid = () => Error(t('Rapport invalide ou différent des fichiers fournis.'));
    if (!exact(value,['schema_version','scope','comparison']) || value.schema_version !== 1 || value.scope !== 'offline-data-only' ||
      !exact(c,['schema_version','authority','effect','approval_validity','before','after','changed_paths','canonical_content_changed','intent_changed','transport_request_changed']) ||
      c.schema_version !== 1 || ['authority','effect','approval_validity'].some(key=>c[key]!=='not-established') ||
      !['before','after'].every(side=>exact(c[side],['document_sha256','action_digest','intent_id'])&&Object.values(c[side]).every(hash)) ||
      !['canonical_content_changed','intent_changed','transport_request_changed'].every(key=>typeof c[key]==='boolean') ||
      !Array.isArray(c.changed_paths) || !c.changed_paths.every((path,index)=>typeof path==='string'&&Object.hasOwn(comparisonFields,path)&&(index===0||c.changed_paths[index-1]<path))) throw invalid();
    if (c.canonical_content_changed !== (c.changed_paths.length>0) ||
      c.intent_changed !== (c.before.intent_id!==c.after.intent_id) ||
      (!c.canonical_content_changed&&(c.before.action_digest!==c.after.action_digest||c.intent_changed)) ||
      c.before.document_sha256 !== await digest(before.bytes) || c.after.document_sha256 !== await digest(after.bytes)) throw invalid();
  }
  /** Render validated field names, flags and byte identities as text only; field
   * values remain absent. Validity grants no approval or effect. Caller has cleared
   * old results; this enables only exact raw-report download after validation. */
  function renderComparison(value, raw) {
    const c=value.comparison;
    $('paea-summary').textContent=t(c.canonical_content_changed?'Contenu canonique modifié.':'Contenu canonique identique.');
    const pairs=[
      [t('Champs modifiés'),c.changed_paths.length?c.changed_paths.map(path=>t(comparisonFields[path])).join(' · '):t('Aucun champ modifié')],
      [t('Contenu canonique modifié'),t(c.canonical_content_changed?'Oui':'Non')],
      [t('Intention métier modifiée'),t(c.intent_changed?'Oui':'Non')],
      [t('Clé de tentative modifiée'),t(c.transport_request_changed?'Oui':'Non')],
      [t('Avant — empreinte du document'),c.before.document_sha256],[t('Après — empreinte du document'),c.after.document_sha256],
      [t('Avant — empreinte de l’action'),c.before.action_digest],[t('Après — empreinte de l’action'),c.after.action_digest],
      [t('Avant — identité de l’intention'),c.before.intent_id],[t('Après — identité de l’intention'),c.after.intent_id]
    ];
    for(const [label,text]of pairs){const term=document.createElement('dt'),description=document.createElement('dd');term.textContent=label;description.textContent=text;$('paea-identities').append(term,description)}
    $('paea-report').textContent=raw;reportRaw=raw;$('paea-download').disabled=false;
    status('success','Comparaison structurelle vérifiée. Aucun accord validé.');
  }
  /** Render only bounded textual terms from a validated data report. The integer
   * amount is displayed in minor units, without inventing a currency scale. */
  function renderPayment(value, raw) {
    const action = value.proposal.action;
    $('paea-summary').textContent = t('Référence résolue dans le registre fourni. Aucun règlement déclenché.');
    for (const [label,text] of [[t('Objet de dette'),action.invoice_id],[t('Version de facture'),action.invoice_version],
      [t('Montant entier — unités mineures'),action.amount_minor+' '+action.currency],
      [t('Compte bénéficiaire'),action.beneficiary_account_id],[t('Révision du registre'),value.registry_revision],
      [t('Empreinte du registre'),value.registry_sha256],[t('Empreinte de la demande'),value.request_sha256]]) {
      const term = document.createElement('dt'), description = document.createElement('dd');
      term.textContent = label; description.textContent = text; $('paea-identities').append(term,description);
    }
    $('paea-report').textContent = raw; reportRaw = raw; $('paea-download').disabled = false;
    status('success','Résolution structurelle vérifiée.');
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
      const previous = receiptView ? $('paea-state').value : null;
      if (receiptView && !states.has(previous)) throw Error(t('Choisir un état précédent.'));
      const [document, receipt] = await Promise.all(comparison ? [readFile('paea-before'),readFile('paea-after')] : payment ? [readFile('paea-registry'),readFile('paea-request')] : [readFile('paea-document'),readFile('paea-receipt')]);
      const body = comparison ? {schema_version:1,before:document.text,after:receipt.text} : payment ? {schema_version:1,registry:document.text,request:receipt.text} :
        {schema_version:1,document:document.text,receipt:receipt.text,expected_sink:$('paea-sink').value,previous_state:previous};
      const response = await fetch('/api/v1/protection/'+operation, {method:'POST',cache:'no-store',
        headers:{'Content-Type':'application/json','X-Swarm-CSRF':csrf},
        body:JSON.stringify(body)});
      const raw = await response.text(); let value;
      try { value = parseResponse(raw); } catch (error) {
        if (!(error instanceof SyntaxError)) throw error;
        throw Error(t('Réponse illisible ; relancer la vérification.'), {cause:error});
      }
      if (!response.ok) {
        const code = value?.failure?.code;
        throw Error(t('Vérification refusée') + (typeof code === 'string' && /^[a-z0-9_-]+$/.test(code) ? ' · ' + code : ''));
      }
      if (payment) { await validatePayment(value,document,receipt); renderPayment(value,raw); }
      else if (comparison) { await validateComparison(value,document,receipt); renderComparison(value,raw); }
      else { await validate(value,document,receipt,previous); render(value,raw); }
    } catch (error) {
      clearReport(); status('error', error instanceof TypeError ? 'Connexion interrompue ; relancer la vérification.' : error.message);
    } finally { busy = false; $('paea-inputs').disabled = false; $('paea-check').disabled = false; }
  }
  /** Export only a fully validated report, then release the temporary browser URL. */
  function download() {
    if (reportRaw === null || busy) return;
    const url = URL.createObjectURL(new Blob([reportRaw], {type:'application/json'}));
    const link = document.createElement('a'); link.href = url; link.download = comparison ? 'paea-comparison.json' : payment ? 'paea-payment-resolution.json' : 'paea-receipt-report.json';
    link.click(); URL.revokeObjectURL(url);
  }
  /** Bootstrap the existing protected native session. Failure keeps submit blocked
   * and the cause visible; reloading retries only this read, never an effect. */
  async function start() {
    try {
      const response = await fetch('/api/v1/native-session', {cache:'no-store'}), session = await response.json();
      if (!response.ok || session.schema_version !== 1 || !/^[a-f0-9]{64}$/.test(session.csrf_token)) throw Error();
      csrf = session.csrf_token; $('paea-check').disabled = false;
      status('empty', emptyMessage);
    } catch { status('error', 'Session indisponible ; recharger la page pour reprendre.'); }
  }
  $('paea-form').addEventListener('submit', check);
  for (const event of ['input','change']) $('paea-inputs').addEventListener(event, () => {
    if (busy) return; clearReport(); status('empty',emptyMessage);
  });
  $('paea-download').addEventListener('click', download);
  const theme = new URL(location.href).searchParams.get('theme');
  if (['etat','sombre'].includes(theme)) document.documentElement.dataset.theme = theme;
  /** Keep language/theme on links between the PAEA data views; these are
   * presentation choices only, without storing documents or changing authority. */
  function navigation() {
    for (const link of document.querySelectorAll('[data-paea-link]')) {
      const url = new URL(link.href);
      url.searchParams.set('lang',globalThis.SwarmI18n.language);
      url.searchParams.set('theme',document.documentElement.dataset.theme);
      link.href = url.pathname+url.search;
    }
  }
  $('paea-theme').addEventListener('click', () => {
    document.documentElement.dataset.theme = document.documentElement.dataset.theme === 'etat' ? 'sombre' : 'etat';
    navigation();
  });
  navigation();
  start();
})();
