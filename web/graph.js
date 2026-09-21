'use strict';
const tr_web_graph_js = source => globalThis.SwarmI18n?.t(source) ?? source;

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
  if (!Number.isFinite(dernier)) return tr_web_graph_js('signal jamais reçu');
  if (Date.now() - dernier > limite) return tr_web_graph_js('signal perdu depuis ') + Math.round((Date.now() - dernier) / 1000) + ' s';
  return '';
}

function graphLignesAgent(item) {
  const a = item.agent;
  const lignes = [a.provider + ' · ' + a.role];
  const action = a.progress?.detail || a.progress?.action;
  if (action) lignes.push(action);
  lignes.push((a.progress?.tool_calls || 0) + tr_web_graph_js(' appels · ') + (a.progress?.tool_results || 0) + tr_web_graph_js(' résultats'));
  // Coût rapporté par le fournisseur pour cette tentative ; jamais une facture.
  const cout = a.usage?.provider_reported_cost_usd;
  lignes.push(typeof cout === 'number' ? tr_web_graph_js('coût rapporté : ') + cout.toFixed(2) + ' USD' : tr_web_graph_js('coût : non rapporté'));
  const perdu = graphSignal(item);
  if (perdu) lignes.push(perdu);
  else if (a.progress?.last_result_at) lignes.push(tr_web_graph_js('dernier résultat : ') + new Date(a.progress.last_result_at).toLocaleTimeString((globalThis.SwarmI18n?.locale || 'fr-FR')));
  if (a.progress?.degraded) lignes.push(tr_web_graph_js('flux dégradé : ') + a.progress.degraded);
  return lignes;
}

// Cumul de la tâche, toutes tentatives confondues. « non rapporté » quand
// aucune n'a déclaré son coût : un 0,00 ferait passer l'inconnu pour une mesure.
function graphCoutTache(id) {
  const c = (snapshot.cost?.by_task || {})[id];
  if (!c || !c.attempts_with_cost) return c && c.attempts_without_cost ? tr_web_graph_js('coût de la tâche : non rapporté') : '';
  let texte = tr_web_graph_js('coût de la tâche : ') + c.reported_usd.toFixed(2) + ' USD';
  if (c.attempts_without_cost) texte += ' (+' + c.attempts_without_cost + tr_web_graph_js(' sans coût)');
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

function renderGraph(){Pilot.render()}
function scalePilotGraph(svg,zoom){
 svg.setAttribute('width',Number(svg.dataset.width)*zoom);svg.setAttribute('height',Number(svg.dataset.height)*zoom);
 // Keep arrowheads readable in the overview; task geometry still follows zoom.
 for(const marker of svg.querySelectorAll('marker')){marker.setAttribute('markerUnits','userSpaceOnUse');marker.setAttribute('markerWidth',9/zoom);marker.setAttribute('markerHeight',9/zoom)}
}
function drawPilotGraph(){
 const canvas=$('pilot-canvas'),state=Pilot.state,tasks=snapshot.work.tasks;
 const shown=PilotGraph.visible(tasks,state.collapsed);
 const kept=tasks.filter(t=>(state.search.trim()||shown.has(t.id))&&(Pilot.matches(t,null)||snapshot.agents.some(x=>x.agent.task_id===t.id&&Pilot.matches(t,x.agent))));
 const ids=new Set(kept.map(t=>t.id));
 const links=(snapshot.pilotage?.edges||[]).filter(e=>ids.has(e.from_task_id)&&ids.has(e.to_task_id)&&!state.collapsed.includes(e.from_task_id));
 const organization=PilotGraph.organization(snapshot.work,kept,snapshot.paused);
 const shape=JSON.stringify([work,organization.nodes.map(n=>[n.id,n.title,n.tone]),organization.edges,kept.map(t=>[t.id,t.depends]),links.map(e=>[e.from_task_id,e.to_task_id]),state.orientation,state.detail,state.collapsed]);
 const height=state.detail==='detailed'?300:186,width=310;
 if(shape!==Pilot.graphKey){
  Pilot.graphKey=shape;
  const focus=document.activeElement?.dataset.task,foldFocus=document.activeElement?.classList.contains('graph-fold'),goFocus=document.activeElement?.classList.contains('graph-go');
  const x=canvas.scrollLeft,y=canvas.scrollTop;
  const g=new dagre.graphlib.Graph();g.setGraph({rankdir:state.orientation,nodesep:28,ranksep:70,marginx:20,marginy:20});g.setDefaultEdgeLabel(()=>({}));
  for(const t of kept)g.setNode(t.id,{width,height});
  for(const e of links)g.setEdge(e.from_task_id,e.to_task_id);
  for(const n of organization.nodes)g.setNode(n.id,{width,height:130});
  for(const e of organization.edges)g.setEdge(e.from,e.to);
  dagre.layout(g);
  // Independent tasks have no ranks: respect the explicitly chosen orientation.
  if(!links.length&&!organization.nodes.length)kept.forEach((t,i)=>g.setNode(t.id,{width,height,x:20+width/2+(state.orientation==='LR'?i*(width+28):0),y:20+height/2+(state.orientation==='TB'?i*(height+28):0)}));
  const totalWidth=links.length||organization.nodes.length?g.graph().width:state.orientation==='LR'?kept.length*(width+28)+12:width+40;
  const totalHeight=links.length||organization.nodes.length?g.graph().height:state.orientation==='TB'?kept.length*(height+28)+12:height+40;
  const svg=svgNode('svg',{viewBox:'0 0 '+Math.max(1,totalWidth)+' '+Math.max(1,totalHeight),class:'graph-svg',role:'group','aria-label':tr_web_graph_js('Dépendances : du prérequis vers la tâche')});
  svg.dataset.width=Math.max(1,totalWidth);svg.dataset.height=Math.max(1,totalHeight);
  const defs=svgNode('defs',{});
  for(const [id,cls]of [['pilot-arrow','graph-fleche'],['pilot-arrow-ok','graph-fleche-ok'],['pilot-arrow-role','graph-fleche-role']]){
   const marker=svgNode('marker',{id,viewBox:'0 0 8 8',refX:7,refY:4,markerWidth:7,markerHeight:7,orient:'auto'});
   marker.append(svgNode('path',{d:'M0,0 L8,4 L0,8 z',class:cls}));defs.append(marker);
  }
  svg.append(defs);
  for(const e of links){
   const data=g.edge(e.from_task_id,e.to_task_id);
   const edge=svgNode('polyline',{points:data.points.map(p=>p.x+','+p.y).join(' '),class:'graph-arete'});
   edge.dataset.from=e.from_task_id;edge.dataset.to=e.to_task_id;svg.append(edge);
  }
  for(const e of organization.edges){const data=g.edge(e.from,e.to);svg.append(svgNode('polyline',{points:data.points.map(p=>p.x+','+p.y).join(' '),class:'graph-organisation-link','marker-end':'url(#pilot-arrow-role)'}))}
  for(const n of organization.nodes){const pos=g.node(n.id),left=pos.x-width/2,top=pos.y-65;
   const group=svgNode('g',{class:'graph-responsibility',tabindex:0,role:'button','aria-label':n.title});group.dataset.responsibility=n.id;group.id='graph-role-'+encodeURIComponent(n.id);group.dataset.tone=n.tone;group.dataset.agentRole=n.role||n.kind;
   group.append(svgNode('rect',{x:left,y:top,width,height:130,rx:12}));
   for(let i=0;i<4;i++)group.append(svgNode('text',{x:left+14,y:top+28+i*25,'data-role-line':i}));
   const open=()=>Planning.inspectRole(n.kind);group.addEventListener('click',e=>{if(e.isTrusted)open()});group.addEventListener('keydown',e=>{if(e.isTrusted&&['Enter',' '].includes(e.key)){e.preventDefault();open()}});svg.append(group);
  }
  const children=PilotGraph.children(tasks);
  for(const t of kept){
   const n=g.node(t.id),left=n.x-width/2,top=n.y-height/2;
   const group=svgNode('g',{class:'graph-noeud',tabindex:0,role:'button','aria-label':t.title+tr_web_graph_js(' — examiner la tâche')});
   group.dataset.task=t.id;
   group.append(svgNode('rect',{x:left,y:top,width,height,rx:12,class:'graph-cadre'}));
   group.append(svgNode('rect',{x:left+10,y:top+48,width:width-20,height:20,rx:4,class:'graph-role-surface'}));
   for(let line=0;line<(state.detail==='detailed'?10:5);line++)group.append(svgNode('text',{x:left+14,y:top+24+line*19,class:line===0?'graph-titre':line===1?'graph-sous-titre':'graph-agent','data-line':line}));
   const open=()=>Pilot.inspect('task',t.id);
   group.addEventListener('click',e=>{if(e.isTrusted)open()});
   group.addEventListener('keydown',e=>{if(e.isTrusted&&['Enter',' '].includes(e.key)){e.preventDefault();open()}});
   svg.append(group);
   const go=svgNode('g',{class:'graph-go',tabindex:0,role:'button'});go.dataset.task=t.id;
   go.append(svgNode('rect',{x:left+12,y:top+height-62,width:width-24,height:26,rx:5}),svgNode('text',{x:left+20,y:top+height-44}),svgNode('title',{}));
   const openSessionOrLaunch=()=>{const session=PilotGraph.taskSession(snapshot.agents,t.id);if(session)AgentTerminal.open(session.agent);else Pilot.go(t.id)};
   go.addEventListener('click',e=>{if(e.isTrusted)openSessionOrLaunch()});
   go.addEventListener('keydown',e=>{if(e.isTrusted&&['Enter',' '].includes(e.key)){e.preventDefault();openSessionOrLaunch()}});svg.append(go);

   if(children.get(t.id)?.length){
    const closed=state.collapsed.includes(t.id),fold=svgNode('g',{class:'graph-fold',tabindex:0,role:'button','aria-expanded':String(!closed),'aria-label':(closed?tr_web_graph_js('Déplier'):'Replier')+tr_web_graph_js(' la branche ')+t.title});
    fold.dataset.task=t.id;
    fold.append(svgNode('rect',{x:left+12,y:top+height-30,width:width-24,height:24,rx:5,class:'graph-fold-surface'}),svgNode('text',{x:left+20,y:top+height-13,class:'graph-fold-label'}));
    fold.addEventListener('click',e=>{if(e.isTrusted)Pilot.toggle(t.id)});
    fold.addEventListener('keydown',e=>{if(e.isTrusted&&['Enter',' '].includes(e.key)){e.preventDefault();Pilot.toggle(t.id)}});
    svg.append(fold);
   }
  }
  canvas.replaceChildren(svg,graphLegendeEtats());
  if(!kept.length)canvas.append(node('p',tasks.length?tr_web_graph_js('Aucun nœud pour ces filtres. Affichez tout le travail.'):tr_web_graph_js('Aucune tâche : le graphe apparaîtra dès qu’un plan existe.')));
  canvas.scrollLeft=x||state.x;canvas.scrollTop=y||state.y;
  if(focus)[...canvas.querySelectorAll(goFocus?'.graph-go':foldFocus?'.graph-fold':'.graph-noeud')].find(n=>n.dataset.task===focus)?.focus();
 }
 const svg=canvas.querySelector('svg');if(!svg)return;
 scalePilotGraph(svg,state.zoom);
 for(const n of organization.nodes){const group=[...svg.querySelectorAll('.graph-responsibility')].find(e=>e.dataset.responsibility===n.id);if(!group)continue;const lines=n.role==='subplanner'?[n.title,n.scopeLabel,n.description,n.detail]:[n.title,n.description,n.detail,tr_web_graph_js('Ouvrir les décisions et avis')];group.setAttribute('aria-label',lines.join('. '));for(const text of group.querySelectorAll('[data-role-line]')){const value=lines[Number(text.dataset.roleLine)];text.textContent=value.length>40?value.slice(0,39)+'…':value}}
 const byTask=graphAgentsParTache();
 for(const group of canvas.querySelectorAll('.graph-noeud')){
  const t=tasks.find(t=>t.id===group.dataset.task),v=snapshot.validation?.tasks[t.id],agent=byTask[t.id]?.[0]?.agent;
  const uncertain=Pilot.uncertainExecution(t,agent);
  group.dataset.etat=uncertain?'attention':graphTonalites[v?.state||t.status]||'neutre';
  group.dataset.selected=String(Pilot.selectedTask()===t.id);
  const lines=[t.title,uncertain||labels[v?.state||t.status]||t.status,agent?agent.provider+' · '+(uncertain?(snapshot.pilotage?.health[agent.id]?.stop_requested?tr_web_graph_js('Arrêt demandé'):tr_web_graph_js('Activité non confirmée')):agent.progress?.detail||agent.progress?.action||tr_web_graph_js('Activité non reçue')):t.id];
  const role=PilotGraph.role(t,agent);
  group.querySelector('.graph-role-surface').dataset.tone=role.tone;
  lines.splice(2,0,role.icon+' '+role.label);
  lines.push(PilotGraph.guidance(t,Pilot.goState(t),v));
  group.setAttribute('aria-label',lines.join('. ')+tr_web_graph_js(' — examiner la tâche'));
  if(state.detail==='detailed'){
   lines.push(agent?(agent.progress?.tool_calls||0)+tr_web_graph_js(' appels · ')+(agent.progress?.tool_results||0)+tr_web_graph_js(' résultats'):(t.depends||[]).length+tr_web_graph_js(' prérequis'));
   lines.push(agent?snapshot.pilotage?.health[agent.id]?.process_label||tr_web_graph_js('Observation inconnue'):t.blocker||'');
   lines.push(agent?snapshot.pilotage?.health[agent.id]?.activity_label||tr_web_graph_js('Activité inconnue'):'');
   lines.push(agent&&typeof agent.usage?.provider_reported_cost_usd==='number'?tr_web_graph_js('coût rapporté : ')+agent.usage.provider_reported_cost_usd.toFixed(2)+' USD':agent?tr_web_graph_js('coût : non rapporté'):'');
   lines.push(graphCoutTache(t.id));
  }
  for(const text of group.querySelectorAll('[data-line]')){const value=lines[Number(text.dataset.line)]||'';text.setAttribute('aria-label',value);text.classList.toggle('graph-role',text.dataset.line==='2');if(text.dataset.line==='2')text.dataset.tone=role.tone;const shortened=value.length>41?value.slice(0,40)+'…':value;if(text.firstChild?.nodeValue!==shortened){text.replaceChildren(document.createTextNode(shortened),svgNode('title',{},value))}}
 }
 for(const button of canvas.querySelectorAll('.graph-go')){
  const t=tasks.find(t=>t.id===button.dataset.task),go=Pilot.goState(t),session=PilotGraph.taskSession(snapshot.agents,t.id);
  if(session){button.style.display='';button.dataset.ready='true';button.dataset.taskSession=t.id;button.dataset.sessionLocation='graph';button.dataset.agentSession=session.agent.id;button.setAttribute('aria-label',session.label+' : '+t.title);button.querySelector('text').textContent=session.label;button.querySelector('title').textContent=tr_web_graph_js('Ouvrir la dernière tentative de cette tâche');continue}
  delete button.dataset.taskSession;delete button.dataset.agentSession;delete button.dataset.sessionLocation;
  const label=go.ready?tr_web_graph_js('Go — lancer'):tr_web_graph_js('Voir le blocage');button.style.display=go.visible?'':'none';button.dataset.ready=String(go.ready);button.setAttribute('aria-label',label+' : '+t.title+(go.ready?'':'. '+go.reason));if(button.querySelector('text').textContent!==label)button.querySelector('text').textContent=label;button.querySelector('title').textContent=go.ready?tr_web_graph_js('Configurer et confirmer le lancement'):go.reason;
 }
 for(const fold of canvas.querySelectorAll('.graph-fold')){
  const id=fold.dataset.task,closed=state.collapsed.includes(id),hidden=PilotGraph.hiddenBelow(tasks,id,state.collapsed);
  const agents=snapshot.agents.filter(x=>hidden.includes(x.agent.task_id)&&active(x.agent)).length;
  const alerts=Pilot.interventions().filter(item=>hidden.includes(item.kind==='task'?item.id:snapshot.decisions.find(d=>d.id===item.id)?.task_id)).length;
  const value=closed?'+ '+(hidden.length?hidden.length+tr_web_graph_js(' tâches · ')+agents+tr_web_graph_js(' agents · ')+alerts+tr_web_graph_js(' alertes'):tr_web_graph_js('Descendants visibles ailleurs')):tr_web_graph_js('− Replier la branche');
  const label=fold.querySelector('text');if(label.textContent!==value)label.textContent=value;
  fold.setAttribute('aria-label',(closed?tr_web_graph_js('Déplier'):'Replier')+tr_web_graph_js(' la branche ')+id+(closed?' : '+hidden.length+tr_web_graph_js(' tâches masquées, ')+agents+tr_web_graph_js(' agents, ')+alerts+tr_web_graph_js(' alertes'):''));
 }
 for(const edge of canvas.querySelectorAll('.graph-arete')){
  const e=links.find(e=>e.from_task_id===edge.dataset.from&&e.to_task_id===edge.dataset.to);
  edge.dataset.lien=e?.satisfied_now?'satisfait':'attente';
  edge.setAttribute('marker-end',e?.satisfied_now?'url(#pilot-arrow-ok)':'url(#pilot-arrow)');
 }
}

// La couleur d'un nœud ne s'invente pas : cette légende dit ce que chaque
// teinte signifie, pour qui la découvre ou ne la distingue pas.
function graphLegendeEtats() {
  const p = node('p', undefined, 'graph-legende');
  p.append(node('span', tr_web_graph_js('Teintes : ')));
  const etats = [['attente', tr_web_graph_js('à faire')], ['active', tr_web_graph_js('en cours')], ['attention', tr_web_graph_js('à vérifier')],
    ['alerte', tr_web_graph_js('bloquée')], ['validee', tr_web_graph_js('acceptée')], ['neutre', tr_web_graph_js('abandonnée')]];
  etats.forEach(([etat, mot], i) => {
    if (i) p.append(node('span', ' · '));
    const e = node('span', mot, 'graph-teinte');
    e.dataset.etat = etat;
    p.append(e);
  });
  return p;
}
