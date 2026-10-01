'use strict';
const tr_web_admin_js = source => globalThis.SwarmI18n?.t(source) ?? source;

const runLimitsFieldList = [
 ['observation_mode', 'Mode d’exécution'],
 ['silence_seconds', 'Silence maximal avant relance'],
 ['tool_seconds', 'Durée maximale par outil'],
 ['max_tool_calls', 'Nombre maximal d’appels d’outil'],
 ['max_repeated_calls', 'Répétitions identiques tolérées'],
 ['max_consecutive_errors', 'Erreurs consécutives tolérées'],
];
let runLimitsAdmin = null, runLimitsLoading = false;

function runLimitsScopeQuery() {
 return { scope: $('admin-scope').value, mission_id: $('admin-mission').value.trim(), scope_key: $('admin-key').value.trim() };
}
function runLimitsFieldHint(name) {
 const bound = runLimitsAdmin?.bounds?.find(b => b.name === name);
 return bound ? tr_web_admin_js('0 = hérité de la portée englobante · 1 à {max} {unit}').replace('{max}', bound.max).replace('{unit}', tr_web_admin_js(bound.unit)) : '';
}
function runLimitsDisplay(key, value) {
 if(key === 'observation_mode') return tr_web_admin_js(value === 1 ? 'Observation — sans plafonds d’exécution' : 'Plafonds actifs');
 return value || tr_web_admin_js('hérité (0)');
}
function runLimitsValueRow(values) {
 return runLimitsFieldList.map(([key, label]) => tr_web_admin_js(label) + ' : ' + runLimitsDisplay(key, values[key])).join('\n');
}
async function loadRunLimitsAdmin() {
 if (runLimitsLoading) return; runLimitsLoading = true;
 $('admin-state').className = 'notice info'; $('admin-state').textContent = tr_web_admin_js('Chargement de cette portée…');
 try {
  const q = runLimitsScopeQuery();
  runLimitsAdmin = await api('/api/v1/run-limits?' + new URLSearchParams(q));
  renderRunLimitsCurrent(); renderRunLimitsHistory();
  $('admin-state').className = 'notice info';
  $('admin-state').textContent = tr_web_admin_js('Portée chargée. Une valeur à 0 est héritée du niveau englobant ; aucune tentative en cours n’est affectée par un changement (REQ-ADM-05).');
 } catch (e) {
  runLimitsAdmin = null; $('admin-current').replaceChildren(); $('admin-history').replaceChildren();
  $('admin-state').className = 'notice alert'; $('admin-state').textContent = e.message;
 } finally { runLimitsLoading = false }
}
function renderRunLimitsCurrent() {
 const host = $('admin-current'); host.replaceChildren();
 const card = node('article', undefined, 'card');
 const entry = runLimitsAdmin.entry;
 if (!entry.revision) {
  card.append(node('h3', tr_web_admin_js('Aucun réglage pour cette portée')), node('p', tr_web_admin_js('Toutes les valeurs sont héritées du niveau englobant (0 = hérité).'), 'notice info'));
 } else {
  card.append(node('h3', tr_web_admin_js('Révision ') + entry.revision), node('p', entry.updated + ' · ' + entry.actor + (entry.reason ? ' · ' + entry.reason : ''), 'assist-meta'));
  const table = node('table'), body = node('tbody');
  for (const [key, label] of runLimitsFieldList) { const row = node('tr'); row.append(node('td', tr_web_admin_js(label)), node('td', String(runLimitsDisplay(key, entry.values[key])))); body.append(row) }
  table.append(body); card.append(table);
 }
 const actions = node('div', undefined, 'toolbar');
 actions.append(button(entry.revision ? tr_web_admin_js('Modifier cette portée') : tr_web_admin_js('Configurer cette portée'), () => openRunLimitsConfigModal()));
 card.append(actions); host.append(card);
}
function renderRunLimitsHistory() {
 const host = $('admin-history'); host.replaceChildren();
 const hist = runLimitsAdmin.history || [];
 if (!hist.length) { host.append(node('p', tr_web_admin_js('Aucun historique pour cette portée.'), 'notice info')); return }
 const currentRevision = runLimitsAdmin.entry.revision;
 const table = node('table'); const head = node('thead'); const hr = node('tr');
 for (const label of [tr_web_admin_js('Révision'), tr_web_admin_js('Valeurs'), tr_web_admin_js('Motif'), tr_web_admin_js('Auteur'), tr_web_admin_js('Date'), tr_web_admin_js('Action')]) hr.append(node('th', label));
 head.append(hr); table.append(head);
 const body = node('tbody');
 for (const h of hist) {
  const row = node('tr');
  row.append(node('td', String(h.revision) + (h.rollback_of ? ' ' + tr_web_admin_js('(retour à ') + h.rollback_of + ')' : '')));
  row.append(node('td', runLimitsValueRow(h.values)));
  row.append(node('td', h.reason || ''));
  row.append(node('td', h.actor));
  row.append(node('td', h.at));
  const cell = node('td');
  const revert = button(tr_web_admin_js('Revenir à cette révision'), () => openRunLimitsRollbackModal(h));
  revert.disabled = h.revision === currentRevision;
  cell.append(revert); row.append(cell);
  body.append(row);
 }
 table.append(body); host.append(table);
}
function openRunLimitsConfigModal() {
 const q = runLimitsScopeQuery(); const entry = runLimitsAdmin.entry;
 openModal(entry.revision ? tr_web_admin_js('Modifier les limites — ') + (q.scope) : tr_web_admin_js('Configurer les limites — ') + (q.scope),
  tr_web_admin_js('Ces limites bornent le silence, la durée d’outil et les répétitions autorisées pour de nouvelles tentatives. Elles ne changent jamais une tentative déjà lancée (REQ-ADM-05).'),
  { action: 'admin-config', scope: q.scope, mission: q.mission_id, key: q.scope_key, revision: entry.revision });
 for (const [key, label] of runLimitsFieldList) {
  if(key === 'observation_mode') {
   if(q.scope === 'mission') field(key, tr_web_admin_js(label), entry.values[key] || 0, [['0',tr_web_admin_js('Plafonds actifs')],['1',tr_web_admin_js('Observation — sans plafonds d’exécution')]],false,tr_web_admin_js('Les appels, erreurs et durées restent mesurés. Aucun arrêt sur leurs plafonds ni sur le nombre de tentatives. Les dépendances, validations et arrêts manuels restent applicables.'));
   continue;
  }
  const f = field(key, tr_web_admin_js(label), entry.values[key] || 0, null, false, runLimitsFieldHint(key));
  f.type = 'number'; f.min = '0'; f.step = '1';
 }
 field('reason', tr_web_admin_js('Motif de ce changement'), entry.reason || '', null, true);
 $('confirm').textContent = tr_web_admin_js('Prévisualiser le changement');
 $('modal-fields').oninput = () => { const c = modalContext; if (c?.action === 'admin-config') { c.signature = null; $('preview').hidden = true; $('confirm').textContent = tr_web_admin_js('Prévisualiser le changement') } };
}
function openRunLimitsRollbackModal(h) {
 const q = runLimitsScopeQuery();
 openModal(tr_web_admin_js('Revenir à la révision ') + h.revision, tr_web_admin_js('Cette action crée une nouvelle révision reprenant les valeurs de la révision choisie ; l’historique complet reste conservé. Effet réservé aux prochains départs (REQ-ADM-05).'),
  { action: 'admin-rollback', scope: q.scope, mission: q.mission_id, key: q.scope_key, toRevision: h.revision, currentRevision: runLimitsAdmin.entry.revision, targetValues: h.values });
 preview(tr_web_admin_js('VALEURS DE LA RÉVISION ') + h.revision + '\n' + runLimitsValueRow(h.values));
 field('reason', tr_web_admin_js('Motif du retour'), tr_web_admin_js('retour à la révision ') + h.revision, null, true);
 $('confirm').textContent = tr_web_admin_js('Confirmer le retour à cette révision');
}
function runLimitsFormValues(f) {
 const values = {};
 for (const [key] of runLimitsFieldList) { const n = Number(f[key] || 0); if (!Number.isInteger(n) || n < 0 || (key === 'observation_mode' && n > 1)) throw new Error(tr_web_admin_js('Valeurs entières positives ou nulles uniquement.')); values[key] = n }
 return values;
}
async function submitRunLimitsConfig(f, c) {
 const values = runLimitsFormValues(f);
 const signature = JSON.stringify(values);
 if (c.signature !== signature) {
  const data = await api('/api/v1/run-limits/preview', { scope: c.scope, mission_id: c.mission, scope_key: c.key, expected_revision: c.revision, values });
  if (modalContext !== c) return false;
  if (!data.revision_ok) throw new Error(tr_web_admin_js('La configuration a changé depuis le chargement ; fermez et rouvrez cette portée avant de confirmer.'));
  preview(tr_web_admin_js('VALEURS ACTUELLES\n') + runLimitsValueRow(data.current.values || {}) + tr_web_admin_js('\n\nVALEURS PROPOSÉES\n') + runLimitsValueRow(values) + tr_web_admin_js('\n\nAucun agent déjà lancé n’est affecté ; seuls les prochains départs utiliseront ces valeurs.'));
  c.signature = signature; $('confirm').textContent = tr_web_admin_js('Confirmer l’enregistrement'); return false;
 }
 const entry = await api('/api/v1/run-limits/apply', { schema_version: 1, event_id: crypto.randomUUID(), scope: c.scope, mission_id: c.mission, scope_key: c.key, expected_revision: c.revision, values, reason: f.reason || '' });
 if (modalContext !== c) return false;
 notice(tr_web_admin_js('Limites enregistrées (révision ') + entry.revision + tr_web_admin_js('). Seuls les prochains départs sont concernés.'));
 await loadRunLimitsAdmin(); return true;
}
async function submitRunLimitsRollback(f, c) {
 const entry = await api('/api/v1/run-limits/rollback', { scope: c.scope, mission_id: c.mission, scope_key: c.key, to_revision: c.toRevision, event_id: crypto.randomUUID(), reason: f.reason || '', expected_revision: c.currentRevision });
 if (modalContext !== c) return false;
 notice(tr_web_admin_js('Retour enregistré comme nouvelle révision ') + entry.revision + tr_web_admin_js('. Seuls les prochains départs sont concernés.'));
 await loadRunLimitsAdmin(); return true;
}
async function loadRunLimitsEffective(f) {
 $('admin-effective-state').className = 'notice info'; $('admin-effective-state').textContent = tr_web_admin_js('Résolution en cours…'); $('admin-effective-output').hidden = true;
 try {
  const q = { mission_id: (f.get('mission') || '').trim(), role: (f.get('role') || '').trim(), task: (f.get('task') || '').trim() };
  const data = await api('/api/v1/run-limits/effective?' + new URLSearchParams(q));
  $('admin-effective-state').textContent = tr_web_admin_js('Valeur qu’obtiendrait un NOUVEAU départ maintenant. Les tentatives déjà lancées gardent leurs limites gelées.');
  $('admin-effective-output').textContent = runLimitsValueRow(data.effective); $('admin-effective-output').hidden = false;
 } catch (e) { $('admin-effective-state').className = 'notice alert'; $('admin-effective-state').textContent = e.message }
}
$('admin-scope-form').addEventListener('submit', e => { e.preventDefault(); loadRunLimitsAdmin() });
$('admin-scope').addEventListener('change', () => {
 const scope = $('admin-scope').value;
 $('admin-mission').disabled = scope === 'project';
 $('admin-key').disabled = scope === 'project' || scope === 'mission';
});
$('admin-effective-form').addEventListener('submit', e => { e.preventDefault(); loadRunLimitsEffective(new FormData(e.target)) });

if (view === 'admin') loadRunLimitsAdmin();
