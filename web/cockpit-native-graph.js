'use strict';
(function () {
  const t = (source) => globalThis.SwarmI18n?.t ? globalThis.SwarmI18n.t(source) : source;
  const SVG_NS = 'http://www.w3.org/2000/svg';

  /**
   * Text node helper using textContent (no innerHTML).
   * @param {string} text literal user-facing text
   * @returns {Text}
   */
  function textNode(text) { return document.createTextNode(text); }

  /**
   * Locate the original #graph container.
   * @returns {Element} container
   * @throws {Error} when the container is absent
   */
  function container() {
    const el = document.getElementById('graph');
    if (!el) throw new Error(t('Conteneur #graph introuvable.'));
    return el;
  }

  /**
   * Validate a Work object and its tasks/dependencies.
   * Pure check; throws on any structural defect.
   * @param {*} work candidate Work
   * @returns {{tasks: Array, depPairs: Array<{from: string, to: string}>, unknown: Array<{taskId: string, dep: string}>}}
   * @throws {Error} on invalid Work, task ids, statuses, duplicate ids, bad dependency types
   */
  function validateWork(work) {
    if (!work || typeof work !== 'object' || Array.isArray(work)) {
      throw new Error(t('Graphe : objet Work attendu.'));
    }
    if (typeof work.id !== 'string' || !work.id) throw new Error(t('Graphe : identifiant de Work manquant.'));
    if (typeof work.title !== 'string') throw new Error(t('Graphe : titre de Work manquant.'));
    if (!Array.isArray(work.tasks)) throw new Error(t('Graphe : liste de tâches attendue.'));
    const seen = new Set();
    const depPairs = [];
    const unknown = [];
    for (const task of work.tasks) {
      if (!task || typeof task !== 'object' || Array.isArray(task)) {
        throw new Error(t('Graphe : tâche invalide.'));
      }
      if (typeof task.id !== 'string' || !task.id) throw new Error(t('Graphe : identifiant de tâche manquant.'));
      if (typeof task.title !== 'string' || !task.title) throw new Error(t('Graphe : titre de tâche manquant.'));
      if (typeof task.status !== 'string' || !task.status) throw new Error(t('Graphe : état de tâche manquant.'));
      if (seen.has(task.id)) throw new Error(t('Graphe : identifiant de tâche en double : ') + task.id);
      seen.add(task.id);
    }
    const ids = seen;
    for (const task of work.tasks) {
      if (task.depends === undefined || task.depends === null) continue;
      if (!Array.isArray(task.depends)) throw new Error(t('Graphe : dépendances de tâche invalides : ') + task.id);
      for (const dep of task.depends) {
        if (typeof dep !== 'string' || !dep) throw new Error(t('Graphe : dépendance invalide pour la tâche : ') + task.id);
        if (ids.has(dep)) depPairs.push({ from: dep, to: task.id });
        else unknown.push({ taskId: task.id, dep });
      }
    }
    if (depPairs.length) detectCycles(work.tasks, depPairs);
    return { tasks: work.tasks, depPairs, unknown };
  }

  /**
   * Reject dependency cycles before layout (no arbitrary cap: full traversal).
   * @param {Array} tasks validated tasks
   * @param {Array<{from:string,to:string}>} depPairs prereq->task pairs
   * @throws {Error} when a cycle exists; remaining IDs may include downstream tasks
   */
  function detectCycles(tasks, depPairs) {
    const dependents = new Map();
    for (const e of depPairs) {
      if (!dependents.has(e.from)) dependents.set(e.from, []);
      dependents.get(e.from).push(e.to);
    }
    const indegree = new Map(tasks.map(task => [task.id, 0]));
    for (const e of depPairs) indegree.set(e.to, (indegree.get(e.to) || 0) + 1);
    const queue = tasks.filter(task => (indegree.get(task.id) || 0) === 0).map(task => task.id);
    let visited = 0;
    while (queue.length) {
      const id = queue.pop();
      visited++;
      for (const next of dependents.get(id) || []) {
        indegree.set(next, indegree.get(next) - 1);
        if (indegree.get(next) === 0) queue.push(next);
      }
    }
    if (visited !== tasks.length) {
      const remaining = tasks.map(task => task.id).filter(id => (indegree.get(id) || 0) > 0);
      throw new Error(t('Graphe : cycle de dépendances détecté entre : ') + remaining.join(', '));
    }
  }

  /**
   * Show an explicit empty-graph message in the original container.
   * @param {Element} host container
   * @param {string} message literal message
   */
  function showEmpty(host, message) {
    const canvas = document.createElement('div');
    canvas.className = 'pilot-canvas';
    const p = document.createElement('p');
    p.textContent = message;
    canvas.append(p);
    host.replaceChildren(canvas);
  }

  /**
   * Render the dependency graph of a Work into the original #graph container.
   * Read-only projection of the recorded task state: no inference, no buttons.
   * Side effect: replaces the container's children (scrollable canvas + svg).
   * @param {object} work Work {id,title,tasks:[{id,title,status,depends?:string[]}]}
   * @throws {Error} on invalid Work, duplicate ids, unknown-shaped dependencies or cycles
   */
  function render(work) {
    const host = container();
    const { tasks, depPairs, unknown } = validateWork(work);
    if (!tasks.length) { showEmpty(host, t('Aucune tâche : le graphe apparaîtra dès qu’un plan existe.')); return; }
    if (typeof graphNodeKey !== 'function' || typeof svgNode !== 'function' || typeof graphEdgePath !== 'function' || typeof graphTonalites !== 'object' || !globalThis.dagre || !globalThis.dagre.graphlib || !globalThis.dagre.layout) {
      throw new Error(t('Graphe : moteur dagre indisponible.'));
    }
    const canvas = document.createElement('div');
    canvas.className = 'pilot-canvas';
    canvas.tabIndex=0;canvas.setAttribute('aria-label',t('Dépendances : du prérequis vers la tâche'));

    const labels = {
      todo: t('à faire'), running: t('en cours'), blocked: t('bloquée'),
      submitted: t('soumise'), accepted: t('acceptée'), waived: t('levée'),
      abandoned: t('abandonnée'), stale: t('à vérifier')
    };
    const tonalites = graphTonalites;

    const nodeWidth = 260, nodeHeight = 72;
    const g = new globalThis.dagre.graphlib.Graph();
    g.setGraph({ rankdir: 'LR', ranker: 'tight-tree', nodesep: 28, ranksep: 70, marginx: 20, marginy: 20 });
    g.setDefaultEdgeLabel(() => ({}));
    for (const task of tasks) g.setNode(graphNodeKey(task.id), { width: nodeWidth, height: nodeHeight });
    for (const e of depPairs) g.setEdge(graphNodeKey(e.from), graphNodeKey(e.to));
    if (!depPairs.length) {
      tasks.forEach((task, i) => g.setNode(graphNodeKey(task.id), {
        width: nodeWidth, height: nodeHeight,
        x: 20 + nodeWidth / 2 + i * (nodeWidth + 28), y: 20 + nodeHeight / 2
      }));
    } else {
      globalThis.dagre.layout(g);
    }
    const totalWidth = depPairs.length ? g.graph().width : tasks.length * (nodeWidth + 28) + 12;
    const totalHeight = depPairs.length ? g.graph().height : nodeHeight + 40;

    const svg = svgNode('svg', {});
    svg.setAttribute('width',String(totalWidth));svg.setAttribute('height',String(totalHeight));
    const defs=svgNode('defs',{}),marker=svgNode('marker',{id:'recorded-dependency-arrow',viewBox:'0 0 8 8',refX:7,refY:4,markerWidth:7,markerHeight:7,orient:'auto'});
    marker.append(svgNode('path',{d:'M0,0 L8,4 L0,8 z',class:'graph-fleche'}));defs.append(marker);svg.append(defs);
    svg.setAttribute('viewBox', '0 0 ' + Math.max(1, totalWidth) + ' ' + Math.max(1, totalHeight));
    svg.setAttribute('class', 'graph-svg');
    svg.setAttribute('role', 'group');
    svg.setAttribute('aria-label', t('Dépendances : du prérequis vers la tâche'));

    for (const e of depPairs) {
      const data = g.edge(graphNodeKey(e.from), graphNodeKey(e.to));
      const d = graphEdgePath(data.points);
      const edge = document.createElementNS(SVG_NS, 'path');
      edge.setAttribute('d', d);
      edge.setAttribute('class', 'graph-arete');
      edge.setAttribute('marker-end','url(#recorded-dependency-arrow)');
      edge.dataset.from = e.from;
      edge.dataset.to = e.to;
      edge.append(svgNodeTitle(t('Dépendance : ') + e.from + t(' vers ') + e.to));
      svg.append(edge);
    }

    for (const task of tasks) {
      const pos = g.node(graphNodeKey(task.id));
      const left = pos.x - nodeWidth / 2, top = pos.y - nodeHeight / 2;
      const stateLabel = labels[task.status] || task.status;
      const group = document.createElementNS(SVG_NS, 'g');
      group.setAttribute('class', 'graph-noeud');
      group.setAttribute('role', 'img');
      group.setAttribute('aria-label', t('Tâche : ') + task.title + t(' — identifiant : ') + task.id + t(' — état : ') + stateLabel);
      group.dataset.task = task.id;
      group.dataset.etat = tonalites[task.status] || 'neutre';
      group.append(document.createElementNS(SVG_NS, 'title'));
      group.lastChild.textContent = task.title + ' (' + task.id + ') — ' + stateLabel;
      const rect = document.createElementNS(SVG_NS, 'rect');
      rect.setAttribute('x', String(left));
      rect.setAttribute('y', String(top));
      rect.setAttribute('width', String(nodeWidth));
      rect.setAttribute('height', String(nodeHeight));
      rect.setAttribute('rx', '12');
      rect.setAttribute('class', 'graph-cadre');
      group.append(rect);
      const title = document.createElementNS(SVG_NS, 'text');
      title.setAttribute('x', String(left + 14));
      title.setAttribute('y', String(top + 28));
      title.setAttribute('class', 'graph-titre');
      appendTruncated(title, task.title);
      group.append(title);
      const sub = document.createElementNS(SVG_NS, 'text');
      sub.setAttribute('x', String(left + 14));
      sub.setAttribute('y', String(top + 52));
      sub.setAttribute('class', 'graph-sous-titre');
      appendTruncated(sub, task.id + ' · ' + stateLabel);
      group.append(sub);
      svg.append(group);
    }

    canvas.append(svg);
    if (unknown.length) {
      const warning = document.createElement('p');
      warning.setAttribute('role', 'alert');
      warning.textContent = t('Dépendances inconnues (non tracées) : ') +
        unknown.map(u => u.dep + ' → ' + u.taskId).join(', ');
      canvas.append(warning);
    }
    host.replaceChildren(canvas);
  }

  /** @param {SVGElement} text target text element */
  function appendTruncated(text, value) {
    const shown = value.length > 40 ? value.slice(0, 39) + '…' : value;
    text.textContent = shown;
    if (shown !== value) {
      const full = document.createElementNS(SVG_NS, 'title');
      full.textContent = value;
      text.append(full);
    }
  }

  /** @returns {SVGElement} a <title> element with textContent */
  function svgNodeTitle(value) {
    const el = document.createElementNS(SVG_NS, 'title');
    el.textContent = value;
    return el;
  }

  /**
   * Clear the original #graph container and replace its content with a message.
   * Side effect: replaces the container's children.
   * @param {string} message literal message to display
   * @throws {Error} when the container is absent
   */
  function clear(message) {
    const host = container();
    const p = document.createElement('p');
    p.textContent = typeof message === 'string' ? message : '';
    host.replaceChildren(p);
  }

  globalThis.SwarmRustDependencyGraph = { render, clear };
})();
