'use strict';
// Mode Conduite : un seul écran, ce qui attend une décision, et rien d'autre.
// Mode Expert : les mêmes données plus tous les outils. Aucune fonction n'est
// retirée par le mode Conduite ; elle est rangée.
const conduiteOnly = ['conduite'];
let cockpitMode = 'conduite';

function applyMode(mode) {
  cockpitMode = mode === 'expert' ? 'expert' : 'conduite';
  document.body.dataset.mode = cockpitMode;
  cockpitStorage.setItem('swarm-mode', cockpitMode);
  $('mode').textContent = cockpitMode === 'conduite' ? 'Passer en mode expert' : 'Revenir au mode conduite';
  for (const b of $('tabs').children) b.hidden = cockpitMode === 'conduite' && !conduiteOnly.includes(b.dataset.view);
  $('assistant').hidden = false;
  if (cockpitMode === 'conduite' && !conduiteOnly.includes(view)) showView('conduite');
}

function openDecision(d) {
  openModal('Décision — ' + (d.task_id || 'Travail'),
    'L’acquittement ne valide pas la tâche et n’arrête pas l’agent.', { action: 'decision', decision: d.id });
  preview(decisionObservation(d).summary + '\n' + d.evidence);
  field('note', 'Décision motivée', '', null, true, 'Journalisée avec votre compte local et la date.');
}

function decisionObservation(d) {
  const restored=d.kind==='silence'&&snapshot.pilotage?.health[d.agent_id]?.process_state==='running';
  return {label:restored?'Signal rétabli — incident à examiner':(escalationLabels[d.kind]||d.kind),
    summary:restored?'Un signal récent est de nouveau reçu. L’incident enregistré reste à examiner ; aucun acquittement automatique.':d.summary};
}

function inboxCard(d) {
  const c = node('article', undefined, 'card');
  c.dataset.decision = d.id;
  const observation=decisionObservation(d);
  c.append(node('h4', (d.task_id || 'Travail') + ' · ' + observation.label),
    node('p', observation.summary), node('p', d.evidence, 'inbox-proof'));
  if (d.kind === 'gate' || d.kind === 'handoff') {
    c.append(button(d.kind === 'gate' ? 'Revalider cette tâche' : 'Examiner le rapport', () => taskDialog(d.task_id)));
  }
  c.append(button('Examiner et décider', () => openDecision(d)));
  return c;
}

function agentLine(item) {
  const a = item.agent || item;
  const parts = [a.provider + ' · ' + a.role];
  if (a.progress && (a.progress.detail || a.progress.action)) parts.push(a.progress.detail || a.progress.action);
  parts.push((a.progress?.tool_calls || 0) + ' appels');
  // Même règle que sur le nœud du graphe : une tentative muette ne doit pas
  // avoir l'air normale parce qu'elle est affichée ailleurs que sur le graphe.
  const perdu = typeof graphSignal === 'function' ? graphSignal({ agent: a }) : '';
  if (perdu) parts.push(perdu);
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
  if (running.length) for (const x of running) row.append(node('p', agentLine(x), 'plan-agent'));
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
    (snapshot.cost_text || 'coût réel non rapporté'),
    'réservé : ' + budget.reserved_usd + ' + ' + budget.committed_estimate_usd + ' USD estimés sur ' + budget.budget.limit_usd];
  if (snapshot.paused) lines.push('départs suspendus');
  const state = $('conduite-state');
  state.textContent = lines.join(' · ');
  state.className = 'notice ' + (snapshot.paused ? 'attention' : 'info');
  renderAccueil(open, budget);

  const inbox = $('conduite-inbox');
  inbox.replaceChildren();
  for (const d of open) inbox.append(inboxCard(d));
  if (!open.length) inbox.append(node('p', 'Aucune demande de décision ouverte. Les tâches bloquées et résultats à examiner restent accessibles par « Prochaine intervention ».'));

  if (typeof renderGraph === 'function') renderGraph();
  if (typeof filRafraichir === 'function') filRafraichir();
  const plan = $('conduite-plan');
  if(plan.hidden){if(plan.childElementCount)plan.replaceChildren();return}
  plan.replaceChildren();
  for (const t of snapshot.work.tasks) plan.append(planRow(t));
  if (!snapshot.work.tasks.length) plan.append(node('p', 'Aucune tâche dans ce travail. Ouvrez le mode expert pour en ajouter.'));
}

// Bandeau d'accueil : une phrase qui dit ce qui a changé et ce qui attend, à la
// même place à chaque visite. Chaque segment mène à la zone concernée.
function segment(texte, cible) {
  const b = button(texte, () => {
    const n = $(cible);
    if (n) n.scrollIntoView({ block: 'center', behavior: 'auto' });
  });
  b.className = 'accueil-segment';
  return b;
}

function renderAccueil(ouvertes, budget) {
  const hote = $('conduite-accueil');
  if (typeof Pilot !== "undefined" && snapshot.pilotage) { Pilot.summary(hote); return; }
  hote.replaceChildren();
  const parts = [];

  const change = typeof filChangementsDepuisVisite === 'function' ? filChangementsDepuisVisite() : null;
  if (change && change.entrees > 0) {
    const quoi = change.taches > 0
      ? change.taches + (change.taches > 1 ? ' tâches ont avancé' : ' tâche a avancé')
      : change.entrees + (change.entrees > 1 ? ' événements' : ' événement');
    // « au moins » tant que le trait de visite n'est pas dans la page chargée :
    // le total réel peut être plus grand.
    parts.push(segment((change.complet ? '' : 'au moins ') + quoi + ' depuis votre visite', 'fil-titre'));
  }

  if (ouvertes.length) {
    parts.push(segment(ouvertes.length + (ouvertes.length > 1 ? ' décisions vous attendent' : ' décision vous attend'), 'conduite-inbox-title'));
  }

  if (!parts.length) {
    // Ni zéro ni tableau vide : une phrase qui dit franchement qu'il n'y a rien.
    hote.append(node('span', change ? 'Rien de neuf depuis votre visite · aucune décision en attente'
                                    : 'Aucune décision en attente'));
  } else {
    parts.forEach((p, i) => {
      if (i) hote.append(node('span', ' · ', 'accueil-separateur'));
      hote.append(p);
    });
  }

  // Le coût est un fait sur ce qui s'est passé : il s'affiche dès qu'une
  // tentative existe, qu'un plafond soit réglé ou non. Le réservé, lui, n'a de
  // sens que si un budget a été fixé. Les deux ne se confondent pas : l'un
  // mesure ce que les fournisseurs ont déclaré, l'autre ce que le moteur a mis
  // de côté.
  const c = snapshot.cost;
  const tentatives = c ? c.attempts_with_cost + c.attempts_without_cost : 0;
  const morceaux = [];
  if (tentatives > 0) morceaux.push(snapshot.cost_text || 'coût réel non rapporté');
  if (budget && budget.budget.limit_usd > 0) {
    morceaux.push(budget.reserved_usd + ' USD réservés sur ' + budget.budget.limit_usd);
  }
  if (morceaux.length) {
    hote.append(node('span', ' · ', 'accueil-separateur'));
    hote.append(node('span', morceaux.join(' · '), 'accueil-budget'));
  }
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
  applyMode(asked || cockpitStorage.getItem('swarm-mode') || 'conduite');
}
