'use strict';
// Mode Conduite : un seul écran, ce qui attend une décision, et rien d'autre.
// Mode Expert : les mêmes données plus tous les outils. Aucune fonction n'est
// retirée par le mode Conduite ; elle est rangée.
const conduiteOnly = ['conduite'];
let cockpitMode = 'conduite';

function applyMode(mode) {
  cockpitMode = mode === 'expert' ? 'expert' : 'conduite';
  document.body.dataset.mode = cockpitMode;
  localStorage.setItem('swarm-mode', cockpitMode);
  $('mode').textContent = cockpitMode === 'conduite' ? 'Passer en mode expert' : 'Revenir au mode conduite';
  for (const b of $('tabs').children) b.hidden = cockpitMode === 'conduite' && !conduiteOnly.includes(b.dataset.view);
  $('assistant').hidden = cockpitMode === 'conduite';
  if (cockpitMode === 'conduite' && !conduiteOnly.includes(view)) showView('conduite');
}

function openDecision(d) {
  openModal('Décision — ' + (d.task_id || 'Travail'),
    'L’acquittement ne valide pas la tâche et n’arrête pas l’agent.', { action: 'decision', decision: d.id });
  preview(d.summary + '\n' + d.evidence);
  field('note', 'Décision motivée', '', null, true, 'Journalisée avec votre compte local et la date.');
}

function inboxCard(d) {
  const c = node('article', undefined, 'card');
  c.dataset.decision = d.id;
  c.append(node('h4', (d.task_id || 'Travail') + ' · ' + (escalationLabels[d.kind] || d.kind)),
    node('p', d.summary), node('p', d.evidence, 'inbox-proof'));
  if (d.kind === 'gate' || d.kind === 'handoff') {
    c.append(button(d.kind === 'gate' ? 'Revalider cette tâche' : 'Examiner le rapport', () => taskDialog(d.task_id)));
  }
  c.append(button('Examiner et décider', () => openDecision(d)));
  return c;
}

function agentLine(a) {
  const parts = [a.provider + ' · ' + a.role];
  if (a.progress && (a.progress.detail || a.progress.action)) parts.push(a.progress.detail || a.progress.action);
  parts.push((a.progress?.tool_calls || 0) + ' appels');
  if (a.progress?.degraded) parts.push('flux dégradé : ' + a.progress.degraded);
  return parts.join(' · ');
}

function planRow(t) {
  const row = node('article', undefined, 'plan-row');
  // Attribut distinct : data-task désigne les lignes du tableau expert.
  row.dataset.plan = t.id;
  const head = node('div', undefined, 'plan-head');
  head.append(node('strong', t.id + ' · ' + t.title), badge(t.status));
  row.append(head);
  const running = snapshot.agents.filter(x => x.agent.task_id === t.id && active(x.agent));
  if (running.length) for (const x of running) row.append(node('p', agentLine(x.agent), 'plan-agent'));
  else if (t.status === 'todo' && (t.depends || []).length) row.append(node('p', 'En attente de : ' + t.depends.join(', '), 'plan-wait'));
  else if (t.blocker) row.append(node('p', t.blocker, 'plan-wait'));
  const actions = (snapshot.task_actions || {})[t.id] || [];
  const advised = actions.find(x => x.conseillee);
  row.append(button(advised ? advised.label : 'Piloter', () => taskDialog(t.id)));
  return row;
}

function renderConduite() {
  if (!snapshot) return;
  const levels = [['manuel', 'Manuel — départs et relais humains'],
    ['assiste', 'Assisté — relais automatique du handoff prouvé'],
    ['autonome', 'Autonome — départs automatiques dans les créneaux']];
  const select = $('conduite-autonomy');
  if (document.activeElement !== select) selectOptions(select, levels, snapshot.autonomy || 'autonome');
  if (document.activeElement !== $('conduite-slots')) $('conduite-slots').value = snapshot.slots || 2;
  $('conduite-pause').textContent = snapshot.paused ? 'Autoriser les départs' : 'Suspendre les départs';
  $('conduite-dispatch').disabled = !!snapshot.paused;

  const busy = snapshot.agents.filter(x => active(x.agent)).length;
  const open = snapshot.decisions.filter(d => !d.resolved_at);
  const budget = snapshot.budget;
  const lines = [snapshot.autonomy_label || snapshot.autonomy,
    busy + ' créneau(x) occupé(s) sur ' + (snapshot.slots || 2),
    'budget estimé : ' + budget.reserved_usd + ' + ' + budget.committed_estimate_usd + ' USD sur ' + budget.budget.limit_usd];
  if (snapshot.paused) lines.push('départs suspendus');
  const state = $('conduite-state');
  state.textContent = lines.join(' · ');
  state.className = 'notice ' + (snapshot.paused ? 'attention' : 'info');

  const inbox = $('conduite-inbox');
  inbox.replaceChildren();
  for (const d of open) inbox.append(inboxCard(d));
  if (!open.length) inbox.append(node('p', 'Rien à traiter. Les agents avancent ; vous serez sollicité pour une décision, pas pour un relais.'));

  if (typeof renderGraph === 'function') renderGraph();
  if (typeof filRafraichir === 'function') filRafraichir();
  const plan = $('conduite-plan');
  plan.replaceChildren();
  for (const t of snapshot.work.tasks) plan.append(planRow(t));
  if (!snapshot.work.tasks.length) plan.append(node('p', 'Aucune tâche dans ce travail. Ouvrez le mode expert pour en ajouter.'));
}

$('mode').onclick = () => applyMode(cockpitMode === 'conduite' ? 'expert' : 'conduite');
$('conduite-apply').onclick = async () => {
  try {
    await act('autonomy', { autonomy: $('conduite-autonomy').value, slots: Number($('conduite-slots').value) });
    await refresh(true);
    notice('Réglage appliqué ; sans effet sur les tentatives déjà lancées.');
  } catch (e) { notice(e.message, true) }
};
$('conduite-pause').onclick = async () => {
  try { await act(snapshot.paused ? 'unpause' : 'pause'); await refresh(true) } catch (e) { notice(e.message, true) }
};
$('conduite-dispatch').onclick = async () => {
  try {
    const r = await act('dispatch');
    await refresh(true);
    notice(r.message || 'Ordonnancement demandé.');
  } catch (e) { notice(e.message, true) }
};
{
  const asked = new URLSearchParams(location.search).get('mode');
  applyMode(asked || localStorage.getItem('swarm-mode') || 'conduite');
}
