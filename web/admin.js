'use strict';
const tr_web_admin_js = source => globalThis.SwarmI18n?.t(source) ?? source;

Object.assign(globalThis.SwarmEnglish || {}, {
 'Attente du stockage SQLite':'SQLite storage wait',
 'Ces réglages traitent uniquement la cause stockage sqlite_busy. Ils ne relancent ni fournisseur ni agent et n’affichent aucun secret ou contenu de requête.':'These settings only handle the sqlite_busy storage cause. They do not restart a provider or agent and show no secret or query content.',
 'Reprises après contention':'Retries after contention',
 'Délai entre reprises (ms)':'Delay between retries (ms)',
 'Attente SQLite par tentative (ms)':'SQLite wait per attempt (ms)',
 'Enregistrer l’attente':'Save wait settings',
 'Chargement des valeurs effectives…':'Loading effective values…',
 'Source':'Source', 'Persisté':'Persisted', 'oui':'yes', 'non':'no', 'Valeurs configurées':'Configured values', 'Valeurs effectives':'Effective values',
 'Cause stockage visible':'Visible storage cause', 'Attente totale maximale':'Maximum total wait',
 'Valeurs effectives chargées.':'Effective values loaded.',
 'Réglages de stockage enregistrés et relus.':'Storage settings saved and read back.',
});

const runLimitsFieldList = [
 ['observation_mode', 'Mode d’exécution'],
 ['silence_seconds', 'Silence maximal avant relance'],
 ['tool_seconds', 'Durée maximale par outil'],
 ['max_tool_calls', 'Nombre maximal d’appels d’outil'],
 ['max_repeated_calls', 'Répétitions identiques tolérées'],
 ['max_consecutive_errors', 'Erreurs consécutives tolérées'],
];
let runLimitsAdmin = null, runLimitsLoading = false;
let storageRetryLoading = false;

function mountStorageRetryAdmin() {
 if ($('storage-retry-admin')) return;
 const article=node('article',undefined,'card'); article.id='storage-retry-admin'; article.setAttribute('aria-labelledby','storage-retry-title');
 const title=node('h3',tr_web_admin_js('Attente du stockage SQLite')); title.id='storage-retry-title';
 const description=node('p',tr_web_admin_js('Ces réglages traitent uniquement la cause stockage sqlite_busy. Ils ne relancent ni fournisseur ni agent et n’affichent aucun secret ou contenu de requête.'));
 const form=node('form',undefined,'toolbar'); form.id='storage-retry-form';
 const addNumber=(id,name,label,min,max)=>{const wrap=node('label',tr_web_admin_js(label));wrap.htmlFor=id;const input=node('input');input.id=id;input.name=name;input.type='number';input.min=String(min);input.max=String(max);input.step='1';input.required=true;wrap.append(input);form.append(wrap)};
 addNumber('storage-busy-retries','busy_retries','Reprises après contention',0,10);
 addNumber('storage-retry-delay','busy_retry_delay_ms','Délai entre reprises (ms)',0,1000);
 addNumber('storage-busy-timeout','busy_timeout_ms','Attente SQLite par tentative (ms)',1,60000);
 const save=node('button',tr_web_admin_js('Enregistrer l’attente'),'primary');save.id='storage-retry-save';save.type='submit';form.append(save);
 const state=node('p',tr_web_admin_js('Chargement des valeurs effectives…'),'notice info');state.id='storage-retry-state';state.setAttribute('role','status');state.tabIndex=-1;
 const effective=node('pre');effective.id='storage-retry-effective';effective.tabIndex=0;effective.hidden=true;
 article.append(title,description,form,state,effective);
 $('admin').insertBefore(article,$('admin-scope-form'));
 form.addEventListener('submit',saveStorageRetryAdmin);
}

function storageRetryDescription(data) {
 const policy=p=>`busy_retries=${p.busy_retries}\nbusy_retry_delay_ms=${p.busy_retry_delay_ms}\nbusy_timeout_ms=${p.busy_timeout_ms}`;
 return [
  tr_web_admin_js('Source')+' : '+data.source,
  tr_web_admin_js('Persisté')+' : '+tr_web_admin_js(data.persisted?'oui':'non'),
  tr_web_admin_js('Valeurs configurées')+' :\n'+policy(data.configured),
  tr_web_admin_js('Valeurs effectives')+' :\n'+policy(data.effective),
  tr_web_admin_js('Cause stockage visible')+' : '+data.storage_failure_cause,
  tr_web_admin_js('Attente totale maximale')+' : '+data.maximum_total_wait_ms+' ms',
 ].join('\n\n');
}

async function loadStorageRetryAdmin() {
 if(storageRetryLoading)return; storageRetryLoading=true;
 const state=$('storage-retry-state');state.className='notice info';state.textContent=tr_web_admin_js('Chargement des valeurs effectives…');
 try{
  const data=await api('/api/v1/storage-retry');
  $('storage-busy-retries').value=data.configured.busy_retries;
  $('storage-retry-delay').value=data.configured.busy_retry_delay_ms;
  $('storage-busy-timeout').value=data.configured.busy_timeout_ms;
  $('storage-retry-effective').textContent=storageRetryDescription(data);$('storage-retry-effective').hidden=false;
  state.textContent=tr_web_admin_js('Valeurs effectives chargées.');
 }catch(e){state.className='notice alert';state.textContent=e.message}
 finally{storageRetryLoading=false}
}

async function saveStorageRetryAdmin(event) {
 event.preventDefault();const form=event.currentTarget;if(!form.reportValidity())return;
 const state=$('storage-retry-state'),save=$('storage-retry-save');state.className='notice info';save.disabled=true;
 try{
  const data=await api('/api/v1/storage-retry',{schema_version:1,busy_retries:Number($('storage-busy-retries').value),busy_retry_delay_ms:Number($('storage-retry-delay').value),busy_timeout_ms:Number($('storage-busy-timeout').value)});
  $('storage-retry-effective').textContent=storageRetryDescription(data);$('storage-retry-effective').hidden=false;
  state.textContent=tr_web_admin_js('Réglages de stockage enregistrés et relus.');state.focus();
 }catch(e){state.className='notice alert';state.textContent=e.message;state.tabIndex=-1;state.focus()}
 finally{save.disabled=false}
}

mountStorageRetryAdmin();

// Open the exact task scope; never silently edit limits or restart an agent.
function openTaskRunLimits(taskID) {
 $('admin-scope').value='task';$('admin-mission').value=work;$('admin-mission').disabled=false;
 $('admin-key').value=taskID;$('admin-key').disabled=false;
 showView('admin');$('admin-load').focus();
}
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
 loadStorageRetryAdmin();
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
 const task=snapshot?.work.tasks.find(t=>t.id===runLimitsScopeQuery().scope_key);
 if(runLimitsScopeQuery().scope==='task'&&task?.plan_tool_limit)card.append(node('p',tr_web_admin_js('Plafond du plan : ')+task.plan_tool_limit+' · '+tr_web_admin_js('Ce plafond reste applicable. Modifier une limite d’exécution ne modifie pas le plan ni une tentative déjà lancée.'),'notice attention'));
 const agent=snapshot?.agents.map(x=>x.agent).filter(a=>a.task_id===task?.id).sort((a,b)=>(b.started||'').localeCompare(a.started||''))[0];
 if(runLimitsScopeQuery().scope==='task'&&agent){const used=agent.progress?.tool_calls,cap=agent.limits?.max_tool_calls;card.append(node('p',tr_web_admin_js('Dernière tentative : ')+agent.attempt_id+' · '+tr_web_admin_js('Appels d’outils : ')+(used??tr_web_admin_js('Non rapporté'))+' / '+(cap||tr_web_admin_js('Sans plafond'))+' · '+tr_web_admin_js('Restant : ')+(typeof used==='number'&&cap?Math.max(0,cap-used):tr_web_admin_js('Non rapporté'))));}
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

Object.assign(globalThis.SwarmEnglish || {}, {
 'Objectifs de performance du graphe':'Graph performance targets',
 'p95 : 95 % des mesures doivent rester sous le délai choisi. Une mesure de 30 ms est un résultat, pas un réglage.':'p95: 95% of measurements must stay below the chosen target. A 30 ms measurement is a result, not a setting.',
 'Ces seuils évaluent la réactivité mesurée. Ils ne règlent pas la vitesse du graphe et ne changent aucun budget d’agent.':'These targets evaluate measured responsiveness. They do not change graph speed or agent budgets.',
 'Configurer les seuils':'Configure targets', 'Recharger les seuils':'Reload targets',
 'Valeurs par défaut':'Default values', 'Historique des seuils':'Target history',
 'Clavier — p95 maximal (ms)':'Keyboard — maximum p95 (ms)',
 'Échantillons par charge':'Samples per workload', 'Nombre de cartes':'Card count',
 'Rendu — p95 maximal (ms)':'Rendering — maximum p95 (ms)',
 'Ajouter une charge':'Add workload', 'Retirer cette charge':'Remove workload',
 'Modifier les objectifs de performance':'Edit performance targets',
 'Les prochaines recettes utilisent une copie figée de ces valeurs. Les résultats antérieurs gardent leurs seuils et leur verdict.':'Future recipes use a frozen copy of these values. Previous results keep their targets and verdict.',
 'Valeurs proposées':'Proposed values', 'Enregistrer les seuils':'Save targets',
 'Seuils enregistrés.':'Targets saved.', 'Fermer':'Close',
 'Prévisualiser':'Preview', 'Chargement des seuils…':'Loading targets…'
});
(function installPerformanceAdmin(){
 const tr=tr_web_admin_js, host=node('article',undefined,'card');host.id='graph-performance-admin';
 const title=node('h3',tr('Objectifs de performance du graphe'));host.append(title,node('p',tr('Ces seuils évaluent la réactivité mesurée. Ils ne règlent pas la vitesse du graphe et ne changent aucun budget d’agent.')));
 const state=node('p',tr('Chargement des seuils…'),'notice info');state.id='graph-performance-state';state.setAttribute('role','status');
 const values=node('div'),history=node('details'),summary=node('summary',tr('Historique des seuils')),hist=node('div');history.append(summary,hist);
 const actions=node('div',undefined,'toolbar');let config;
 const edit=button(tr('Configurer les seuils'),()=>openPerformance());edit.id='graph-performance-edit';edit.disabled=true;
 const reload=button(tr('Recharger les seuils'),()=>load());reload.id='graph-performance-reload';actions.append(edit,reload);host.append(state,values,actions,history);$('admin').append(host);
 function describe(c){const table=node('table'),body=node('tbody');for(const [label,value]of [[tr('Clavier — p95 maximal (ms)'),c.keyboard_p95_ms],[tr('Échantillons par charge'),c.samples],...c.loads.map(x=>[x.cards+' '+tr('Nombre de cartes'),x.render_p95_ms+' ms'])]){const row=node('tr');row.append(node('td',label),node('td',String(value)));body.append(row)}table.append(body);return table}
 async function load(){edit.disabled=true;state.className='notice info';state.textContent=tr('Chargement des seuils…');try{config=await api('/api/v1/graph-performance');values.replaceChildren(describe(config.values));state.textContent=config.revision?tr('Révision ')+config.revision:tr('Valeurs par défaut');hist.replaceChildren();for(const h of config.history.slice().reverse()){const entry=node('article',undefined,'card');entry.append(node('p',tr('Révision ')+h.revision+' · '+h.at+' · '+h.actor),node('p',h.change.reason),describe(h.values));hist.append(entry)}edit.disabled=false}catch(e){config=null;state.className='notice alert';state.textContent=e.message}}
 function openPerformance(){
  if(!config)return;
  const frozen=structuredClone(config),dialog=node('dialog'),header=node('header',undefined,'modal-header'),heading=node('h2',tr('Modifier les objectifs de performance')),close=button(tr('Fermer'),()=>dialog.close());heading.id='graph-performance-modal-title';dialog.id='graph-performance-modal';dialog.setAttribute('aria-labelledby',heading.id);header.append(heading,close);
  const form=node('form'),body=node('div',undefined,'modal-body'),footer=node('footer',undefined,'modal-footer');body.append(node('p',tr('Les prochaines recettes utilisent une copie figée de ces valeurs. Les résultats antérieurs gardent leurs seuils et leur verdict.'),'notice info'),node('p',tr('p95 : 95 % des mesures doivent rester sous le délai choisi. Une mesure de 30 ms est un résultat, pas un réglage.')));
  function numeric(parent,id,label,value,key){const {min,max}=frozen.bounds[key];const lab=node('label',label),input=node('input');input.id=id;input.type='number';input.required=true;input.min=String(min);input.max=String(max);input.step='1';input.value=String(value);lab.htmlFor=id;lab.append(input);parent.append(lab);return input}
  const fields=node('div',undefined,'toolbar'),keyboard=numeric(fields,'performance-keyboard',tr('Clavier — p95 maximal (ms)'),frozen.values.keyboard_p95_ms,'keyboard_p95_ms'),samples=numeric(fields,'performance-samples',tr('Échantillons par charge'),frozen.values.samples,'samples');body.append(fields);
  const loads=node('div');body.append(loads);let rows=[],serial=0,pending=null;
  function invalidate(){pending=null;proposal.hidden=true;submit.textContent=tr('Prévisualiser')}
  function addRow(value){const row=node('div',undefined,'toolbar'),id=serial++,cards=numeric(row,'performance-cards-'+id,tr('Nombre de cartes'),value.cards,'cards'),render=numeric(row,'performance-render-'+id,tr('Rendu — p95 maximal (ms)'),value.render_p95_ms,'render_p95_ms');const item={row,cards,render};rows.push(item);row.append(button(tr('Retirer cette charge'),()=>{rows=rows.filter(x=>x!==item);row.remove();invalidate()}));loads.append(row)}
  const add=button(tr('Ajouter une charge'),()=>{if(rows.length<frozen.bounds.loads.max){addRow({cards:rows.length?Number(rows.at(-1).cards.value)+1:frozen.bounds.cards.min,render:Number(rows.at(-1)?.render.value||keyboard.value)});invalidate()}});add.id='performance-add-load';body.append(add);
  const reasonLabel=node('label',tr('Motif de ce changement')),reason=node('textarea');reason.id='performance-reason';reason.required=true;reason.minLength=8;reason.maxLength=2000;reasonLabel.htmlFor=reason.id;reasonLabel.append(reason);body.append(reasonLabel);
  const error=node('p',undefined,'notice alert');error.id='performance-error';error.hidden=true;error.setAttribute('role','alert');const proposal=node('div',undefined,'card');proposal.id='performance-preview';proposal.hidden=true;body.append(error,proposal);
  const submit=node('button',tr('Prévisualiser'),'primary');submit.id='performance-confirm';submit.type='submit';footer.append(button(tr('Fermer'),()=>dialog.close()),submit);form.append(body,footer);dialog.append(header,form);frozen.values.loads.forEach(addRow);
  form.addEventListener('input',invalidate);let sending=false;
  form.addEventListener('submit',async e=>{e.preventDefault();if(sending)return;sending=true;submit.disabled=true;error.hidden=true;try{
   const request={schema_version:1,event_id:pending?.event_id||crypto.randomUUID(),expected_revision:frozen.revision,values:{keyboard_p95_ms:Number(keyboard.value),samples:Number(samples.value),loads:rows.map(x=>({cards:Number(x.cards.value),render_p95_ms:Number(x.render.value)}))},reason:reason.value.trim()};
   if(!pending){const result=await api('/api/v1/graph-performance/preview',request);if(!dialog.open)return;pending=request;proposal.replaceChildren(node('h3',tr('Valeurs proposées')),describe(result.config.values));proposal.hidden=false;submit.textContent=tr('Enregistrer les seuils')}
   else {await api('/api/v1/graph-performance/apply',pending);if(!dialog.open)return;dialog.close();await load();state.textContent=tr('Seuils enregistrés.')+' '+tr('Révision ')+config.revision}
  }catch(e){if(dialog.open){error.textContent=e.message;error.hidden=false;pending=null;proposal.hidden=true;submit.textContent=tr('Prévisualiser')}}finally{sending=false;submit.disabled=false}});
  dialog.addEventListener('close',()=>{dialog.remove();edit.focus()},{once:true});document.body.append(dialog);dialog.showModal();keyboard.focus();
 }
 load();
})();
