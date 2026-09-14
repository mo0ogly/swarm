'use strict';
// Graphe vivant : les tâches et leurs dépendances, avec la tentative en cours
// rattachée à sa tâche. Ce que le graphe montre est observé, pas déduit :
// un nœud « en cours » ne prouve pas qu'un processus tourne encore.
const graphTonalites = {
  todo: 'attente', running: 'active', blocked: 'alerte', submitted: 'attention',
  accepted: 'validee', waived: 'validee', abandoned: 'neutre', stale: 'attention'
};

function graphAgentsParTache() {
  const parTache = {};
  for (const item of snapshot.agents) {
    const a = item.agent;
    if (!active(a)) continue;
    (parTache[a.task_id] = parTache[a.task_id] || []).push(item);
  }
  return parTache;
}

function graphSignal(item) {
  // « Signal perdu » et « terminé » sont deux états distincts : le premier dit
  // qu'on ne sait plus, le second qu'on a vu la fin.
  const a = item.agent;
  const limite = Math.max(30, a.limits?.silence_seconds || 0) * 1000;
  const dernier = Date.parse(a.heartbeat || a.started || '');
  if (!Number.isFinite(dernier)) return 'signal jamais reçu';
  if (Date.now() - dernier > limite) return 'signal perdu depuis ' + Math.round((Date.now() - dernier) / 1000) + ' s';
  return '';
}

function graphLignesAgent(item) {
  const a = item.agent;
  const lignes = [a.provider + ' · ' + a.role];
  const action = a.progress?.detail || a.progress?.action;
  if (action) lignes.push(action);
  lignes.push((a.progress?.tool_calls || 0) + ' appels · ' + (a.progress?.tool_results || 0) + ' résultats');
  // Coût rapporté par le fournisseur pour cette tentative ; jamais une facture.
  const cout = a.usage?.provider_reported_cost_usd;
  lignes.push(typeof cout === 'number' ? 'coût rapporté : ' + cout.toFixed(2) + ' USD' : 'coût : non rapporté');
  const perdu = graphSignal(item);
  if (perdu) lignes.push(perdu);
  else if (a.progress?.last_result_at) lignes.push('dernier résultat : ' + new Date(a.progress.last_result_at).toLocaleTimeString('fr-FR'));
  if (a.progress?.degraded) lignes.push('flux dégradé : ' + a.progress.degraded);
  return lignes;
}

// Cumul de la tâche, toutes tentatives confondues. « non rapporté » quand
// aucune n'a déclaré son coût : un 0,00 ferait passer l'inconnu pour une mesure.
function graphCoutTache(id) {
  const c = (snapshot.cost?.by_task || {})[id];
  if (!c || !c.attempts_with_cost) return c && c.attempts_without_cost ? 'coût de la tâche : non rapporté' : '';
  let texte = 'coût de la tâche : ' + c.reported_usd.toFixed(2) + ' USD';
  if (c.attempts_without_cost) texte += ' (+' + c.attempts_without_cost + ' sans coût)';
  return texte;
}

function svgNode(nom, attributs, texte) {
  const e = document.createElementNS('http://www.w3.org/2000/svg', nom);
  for (const [cle, valeur] of Object.entries(attributs)) e.setAttribute(cle, valeur);
  if (texte !== undefined) e.textContent = texte;
  return e;
}

function graphMesures(tache, agents) {
  const cumul = graphCoutTache(tache.id) ? 1 : 0;
  const lignes = 2 + cumul + agents.reduce((n, item) => n + graphLignesAgent(item).length, 0) + (agents.length ? 1 : 0);
  return { width: 260, height: 30 + lignes * 17 };
}

function renderGraph() {
  const hote = $('graph');
  if (!hote || !snapshot) return;
  hote.replaceChildren();
  const taches = snapshot.work.tasks;
  if (!taches.length) {
    hote.append(node('p', 'Aucune tâche : le graphe apparaîtra dès qu’un plan existe.'));
    return;
  }
  if (typeof dagre === 'undefined') {
    hote.append(node('p', 'Bibliothèque de disposition indisponible ; la liste ci-dessous reste complète.', 'notice attention'));
    return;
  }
  const parTache = graphAgentsParTache();
  const g = new dagre.graphlib.Graph({ multigraph: false, compound: false });
  g.setGraph({ rankdir: 'LR', nodesep: 26, ranksep: 60, marginx: 12, marginy: 12 });
  g.setDefaultEdgeLabel(() => ({}));
  const connues = new Set(taches.map(t => t.id));
  for (const t of taches) g.setNode(t.id, graphMesures(t, parTache[t.id] || []));
  for (const t of taches) for (const d of t.depends || []) if (connues.has(d)) g.setEdge(d, t.id);
  dagre.layout(g);

  const info = g.graph();
  const svg = svgNode('svg', {
    viewBox: `0 0 ${Math.max(1, info.width)} ${Math.max(1, info.height)}`,
    width: Math.max(1, info.width), height: Math.max(1, info.height),
    class: 'graph-svg', role: 'img',
    'aria-label': `Graphe de ${taches.length} tâche(s) ; le détail textuel suit sous le graphe.`
  });
  const defs = svgNode('defs', {});
  const marker = svgNode('marker', { id: 'graph-fleche', viewBox: '0 0 8 8', refX: '7', refY: '4', markerWidth: '7', markerHeight: '7', orient: 'auto' });
  marker.append(svgNode('path', { d: 'M0,0 L8,4 L0,8 z', class: 'graph-fleche' }));
  defs.append(marker);
  svg.append(defs);

  for (const e of g.edges()) {
    const points = g.edge(e).points.map(p => `${p.x},${p.y}`).join(' ');
    svg.append(svgNode('polyline', { points, class: 'graph-arete', 'marker-end': 'url(#graph-fleche)' }));
  }

  for (const t of taches) {
    const n = g.node(t.id);
    const x = n.x - n.width / 2, y = n.y - n.height / 2;
    const groupe = svgNode('g', {
      class: 'graph-noeud', 'data-etat': graphTonalites[t.status] || 'neutre',
      tabindex: '0', role: 'button',
      'aria-label': `${t.id} ${t.title} — ${labels[t.status] || t.status}`
    });
    groupe.append(svgNode('rect', { x, y, rx: 10, ry: 10, width: n.width, height: n.height, class: 'graph-cadre' }));
    groupe.append(svgNode('text', { x: x + 12, y: y + 22, class: 'graph-titre' }, t.id + ' · ' + (labels[t.status] || t.status)));
    groupe.append(svgNode('text', { x: x + 12, y: y + 40, class: 'graph-sous-titre' }, t.title.length > 30 ? t.title.slice(0, 29) + '…' : t.title));
    let ligne = y + 60;
    const cumul = graphCoutTache(t.id);
    if (cumul) {
      groupe.append(svgNode('text', { x: x + 12, y: ligne, class: 'graph-agent' }, cumul));
      ligne += 17;
    }
    for (const item of parTache[t.id] || []) {
      for (const texte of graphLignesAgent(item)) {
        const classe = /signal perdu|jamais reçu|dégradé/.test(texte) ? 'graph-alerte' : 'graph-agent';
        groupe.append(svgNode('text', { x: x + 12, y: ligne, class: classe }, texte.length > 34 ? texte.slice(0, 33) + '…' : texte));
        ligne += 17;
      }
    }
    const ouvrir = () => taskDialog(t.id);
    groupe.addEventListener('click', ouvrir);
    groupe.addEventListener('keydown', ev => { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); ouvrir() } });
    svg.append(groupe);
  }

  const conteneur = node('div', undefined, 'graph-cadre-defilant');
  conteneur.append(svg);
  hote.append(conteneur);
  const liens = snapshot.agents.filter(x => x.agent.parent && active(x.agent));
  if (liens.length) hote.append(node('p', 'Tentatives issues d’un parent : ' + liens.map(x => x.agent.task_id + ' ← ' + x.agent.parent).join(' · '), 'graph-legende'));
}
