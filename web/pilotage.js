'use strict';
// A stable shell: refreshes update data without rebuilding controls or preferences.
const Pilot = {
 state:PilotGraph.preferences(),key:'',listKey:'',inspectorKey:'',queue:null,returnView:null,storageWarning:'',
 command(text,fn,cls=''){const b=button(text,e=>{if(e.isTrusted)fn(e)});b.className=cls;return b},
 save(){
  const port=$('pilot-canvas');if(port&&!port.hidden&&!this.restorePosition){this.state.x=port.scrollLeft;this.state.y=port.scrollTop}
  try{localStorage.setItem(this.key,JSON.stringify({version:2,...this.state}))}catch{this.storageWarning='Affichage non mémorisé sur cet appareil.'}
 },
 restore(){
  const key='swarm-pilot-v1:'+snapshot.pilotage?.project_view_key+':'+work;
  if(key===this.key)return;
  this.key=key;this.restorePosition=true;this.queue=null;this.returnView=null;this.inspectorKey='';this.listKey='';this.graphKey='';
  let stored={};try{stored=JSON.parse(localStorage.getItem(key)||'{}')||{}}catch{this.storageWarning='Préférences indisponibles ; affichage réinitialisé.'}
  this.fitPending=stored.version!==2;
  this.state=PilotGraph.preferences(stored.version===2?stored:stored.version===1?{...stored,view:'dependencies',filter:'all',search:'',collapsed:[],x:0,y:0}:{});
  const ids=new Set(snapshot.work.tasks.map(t=>t.id));this.state.collapsed=this.state.collapsed.filter(id=>ids.has(id));
  if(this.state.selection?.kind==='task'&&!ids.has(this.state.selection.id)){this.state.selection=null;this.storageWarning='La tâche sélectionnée n’existe plus ; sélection effacée.'}
 },
 control(id,title,options,property){
  const label=node('label',title),select=node('select');select.id=id;label.htmlFor=id;
  selectOptions(select,options,this.state[property]);
  select.addEventListener('change',e=>{if(!e.isTrusted)return;this.state[property]=select.value;this.changed()});label.append(select);return label;
 },
 mount(){
  if($('pilot-toolbar'))return;
  $('conduite-graph-title').textContent='Pilotage des agents';
  $('conduite-purpose').textContent='Repérez qui travaille, ce qui attend votre décision et ce qui bloque la suite.';
  $('conduite-plan-title').hidden=true;$('conduite-plan').hidden=true;
  const toolbar=node('div',undefined,'pilot-toolbar');toolbar.id='pilot-toolbar';
  toolbar.append(this.control('pilot-view','Vue',[['dependencies','Graphe des dépendances'],['agents','Liste des agents']],'view'),
   this.control('pilot-orientation','Orientation',[['LR','Horizontale →'],['TB','Verticale ↓']],'orientation'),
   this.control('pilot-detail','Informations',[['simple','Simplifiées'],['detailed','Détaillées']],'detail'),
   this.control('pilot-filter','Afficher',[['all','Tout le travail'],['active','Agents démarrés'],['unknown','Sans signal confirmé'],['review','Résultats à examiner'],['waiting','Tâches à préparer'],['finished','Tentatives terminées']],'filter'));
  const label=node('label','Rechercher'),input=node('input');input.type='search';input.id='pilot-search';input.placeholder='Tâche, rôle ou fournisseur';label.htmlFor=input.id;label.append(input);
  input.addEventListener('input',e=>{if(e.isTrusted){this.state.search=input.value;this.changed()}});toolbar.append(label);
  const actions=node('div',undefined,'pilot-actions');actions.id='pilot-actions';
  const items=[
   ['pilot-mission','Réglages de la mission',()=>Mission.open(true)],
   ['pilot-launch-all','Lancer tout',()=>Mission.open()],
   ['pilot-next','Prochaine intervention',()=>this.nextIntervention()],
   ['pilot-group','Regrouper',()=>{this.state.grouped=!this.state.grouped;this.changed()}],
   ['pilot-all-links','Toutes les dépendances',()=>{this.state.view='dependencies';this.state.collapsed=[];this.state.filter='all';this.state.search='';this.fitPending=true;this.changed()}],
   ['pilot-collapse','Tout replier',()=>{this.state.collapsed=snapshot.work.tasks.filter(t=>PilotGraph.children(snapshot.work.tasks).get(t.id)?.length).map(t=>t.id);this.changed()}],
   ['pilot-expand','Tout déplier',()=>{this.state.collapsed=[];this.changed()}],
   ['pilot-zoom-out','Réduire le zoom',()=>this.zoom(-.15)],
   ['pilot-zoom-in','Agrandir le zoom',()=>this.zoom(.15)],
   ['pilot-fit','Vue d’ensemble',()=>{this.fitPending=true;this.changed()}],
   ['pilot-reveal','Retrouver ma sélection',()=>this.revealSelection()],
   ['pilot-reset','Réinitialiser l’affichage',()=>{this.state=PilotGraph.preferences();this.fitPending=true;this.changed();$('pilot-canvas').scrollTo(0,0)}],
   ['pilot-help','Aide du pilotage',()=>this.help()]
  ];
  for(const [id,text,fn]of items){const b=this.command(text,fn,id==='pilot-launch-all'?'primary':'');b.id=id;actions.append(b)}
  const status=node('p','', 'pilot-status');status.id='pilot-status';status.setAttribute('role','status');
  const canvas=node('div',undefined,'pilot-canvas');canvas.id='pilot-canvas';canvas.tabIndex=0;canvas.setAttribute('aria-label','Graphe des dépendances, défilement avec les flèches du clavier');
  canvas.addEventListener('scroll',()=>this.save(),{passive:true});
  let drag=null;canvas.addEventListener('pointerdown',e=>{if(e.isTrusted&&!e.target.closest('[role=button],button')&&e.button===0){drag={x:e.clientX,y:e.clientY,left:canvas.scrollLeft,top:canvas.scrollTop};canvas.setPointerCapture(e.pointerId)}});
  canvas.addEventListener('pointermove',e=>{if(drag){canvas.scrollLeft=drag.left+drag.x-e.clientX;canvas.scrollTop=drag.top+drag.y-e.clientY}});
  canvas.addEventListener('pointerup',()=>{drag=null;this.save()});canvas.addEventListener('pointercancel',()=>{drag=null});
  const list=node('div',undefined,'pilot-list');list.id='pilot-list';
  const mission=node('section',undefined,'mission-summary');mission.id='mission-summary';mission.setAttribute('aria-label','Résultats et conduite de la mission');
  $('graph').replaceChildren(mission,toolbar,actions,status,canvas,list);
 },
 changed(){this.save();this.listKey='';this.render();},
 zoom(delta){this.state.zoom=Math.max(.15,Math.min(2,this.state.zoom+delta));this.changed()},
 uncertainExecution(t,a,h=snapshot.pilotage?.health[a?.id]){
  if(t?.status!=='running'||!a)return '';
  if(['queued','starting'].includes(a.status)&&!a.heartbeat)return 'Démarrage non confirmé'+(h?.stop_requested?' — arrêt demandé':'');
  if(h?.process_state?.startsWith('unknown/')||h?.process_state?.endsWith('/unconfirmed'))return h.process_label||'Exécution non confirmée';
  return '';
 },
 taskTitle(t){return t?.title||'Tâche indisponible'},
 goState(t){const action=snapshot.task_actions?.[t.id]?.find(x=>x.kind==='start');return {visible:['todo','blocked'].includes(t.status),ready:action?.disponible===true,reason:action?.raison||'Consultez les conditions de lancement.'}},
 async go(id){
  const requested=work,t=snapshot.work.tasks.find(t=>t.id===id);if(!t)return;
  if(!this.goState(t).ready){this.inspect('task',id);return;}
  await taskDialog(id);
  if(work!==requested||modalContext?.task!==id||!$('modal').open)return;
  if(modalContext.data.actions?.some(a=>a.kind==='start'&&a.disponible)){taskFields('start');$('modal-description').textContent='Préparer le lancement — les conditions sont vérifiées automatiquement.';}
  else{closeModal();this.inspect('task',id);notice('Les conditions ont changé. Consultez le blocage avant de lancer.',true);}
 },

 matches(t,a){
  const query=this.state.search.toLocaleLowerCase('fr');
  if(query&&![t?.id,t?.title,a?.provider,a?.role,a?.id].filter(Boolean).join(' ').toLocaleLowerCase('fr').includes(query))return false;
  const health=a?snapshot.pilotage?.health[a.id]:null;
  return this.state.filter==='all'||this.state.filter==='review'&&t?.status==='submitted'||
   this.state.filter==='waiting'&&!a&&['todo','blocked'].includes(t?.status)||
   this.state.filter==='active'&&a&&['running','stopping'].includes(health?.process_state)||
   this.state.filter==='unknown'&&a&&active(a)&&!['running','stopping'].includes(health?.process_state)||
   this.state.filter==='finished'&&a&&!active(a);
 },
 inspect(kind,id){const a=kind==='agent'?snapshot.agents.find(x=>x.agent.id===id)?.agent:kind==='task'?snapshot.agents.find(x=>x.agent.task_id===id)?.agent:null;if(['terminal','dialogue'].includes(a?.mode)){AgentTerminal.open(a);return}this.state.selection={kind,id};this.inspectorKey='';this.save();PilotInspector.render(true)},
 selectedTask(){
  const s=this.state.selection;
  return s?.kind==='task'?s.id:s?.kind==='agent'?(snapshot.agents.find(x=>x.agent.id===s.id)?.agent.task_id||PilotInspector.loaded?.agent?.task_id):
   s?.kind==='decision'?snapshot.decisions.find(d=>d.id===s.id)?.task_id:null;
 },
 revealSelection(){
  const id=this.selectedTask();if(!id)return;
  this.state.collapsed=PilotGraph.reveal(snapshot.work.tasks,id,this.state.collapsed);
  this.state.filter='all';this.state.search='';this.state.view='dependencies';this.changed();
  const n=[...document.querySelectorAll('.graph-noeud')].find(n=>n.dataset.task===id);
  n?.focus();n?.scrollIntoView({block:'center',inline:'center'});this.save();
 },
 toggle(id){
  this.state.collapsed=this.state.collapsed.includes(id)?this.state.collapsed.filter(x=>x!==id):[...this.state.collapsed,id];
  this.changed();[...document.querySelectorAll('.graph-fold')].find(n=>n.dataset.task===id)?.focus();
 },
 render(){
  if(!snapshot||snapshot.work.id!==work)return;
  this.restore();this.mount();Mission.render();
  for(const [id,property]of [['pilot-view','view'],['pilot-orientation','orientation'],['pilot-detail','detail'],['pilot-filter','filter'],['pilot-search','search']]){
   if(this.controlsWork!==work||document.activeElement!==$(id))$(id).value=this.state[property];
  }
  this.controlsWork=work;
  const graph=this.state.view==='dependencies';$('pilot-canvas').hidden=!graph;$('pilot-list').hidden=graph;
  for(const id of ['pilot-orientation','pilot-collapse','pilot-expand','pilot-fit','pilot-zoom-in','pilot-zoom-out'])$(id).disabled=!graph;
  $('pilot-group').hidden=graph;$('pilot-group').textContent=this.state.grouped?'Afficher les agents':'Regrouper';
  $('pilot-reveal').disabled=!this.selectedTask();
  if(graph){drawPilotGraph();if(this.fitPending){
   const svg=$('pilot-canvas').querySelector('svg');
   if(svg){const port=$('pilot-canvas');this.state.zoom=Math.max(.15,Math.min(1,(port.clientWidth-20)/Number(svg.dataset.width),(innerHeight*.65-50)/Number(svg.dataset.height)));scalePilotGraph(svg,this.state.zoom);this.state.x=0;this.state.y=0;port.scrollTo(0,0);this.fitPending=false;this.save()}
  }if(this.restorePosition){$('pilot-canvas').scrollTo(this.state.x,this.state.y);this.restorePosition=false}}else this.cards();
  const edges=snapshot.pilotage?.edges||[];
  $('pilot-status').textContent=this.storageWarning||(this.state.search.trim()?'Recherche dans toutes les tâches, y compris les branches repliées.':!snapshot.work.tasks.length?'Aucune tâche : le graphe apparaîtra dès qu’un plan existe.':graph&&!edges.length?'Tâches indépendantes : aucune dépendance déclarée, donc aucune flèche.':graph?'Les flèches vont du prérequis vers la tâche qui en dépend. Zoom '+Math.round(this.state.zoom*100)+' %.':'Sélectionnez un agent ou une tâche pour comprendre son état et examiner ses résultats.');
  $('pilot-status').classList.toggle('notice',graph&&!edges.length&&snapshot.work.tasks.length>0);$('pilot-status').classList.toggle('info',graph&&!edges.length&&snapshot.work.tasks.length>0);
  if(graph&&edges.length){const visible=$('pilot-canvas').querySelectorAll('.graph-arete').length;$('pilot-status').append(' '+visible+' dépendances affichées sur '+edges.length+'.');if(visible<edges.length)$('pilot-status').append(' Des branches sont repliées ou filtrées : utilisez « Toutes les dépendances ».')}
  if(graph&&(this.state.filter!=='all'||this.state.search.trim()))$('pilot-status').append(' Les liens dont une extrémité est filtrée restent masqués.');
  if(this.state.selection)PilotInspector.render();
 },
 cards(){
  const entries=[];
  for(const t of snapshot.work.tasks){
   const agents=snapshot.agents.filter(x=>x.agent.task_id===t.id).map(x=>x.agent);
   for(const a of agents.length?agents:[null])if(this.matches(t,a))entries.push({t,a});
  }
  const sig=JSON.stringify([this.state.detail,this.state.grouped,entries.map(({t,a})=>[t.id,t.title,t.status,this.goState(t),a?.id,a?.progress,snapshot.validation?.tasks[t.id]?.state,snapshot.pilotage?.health[a?.id]?.activity_label,graphCoutTache(t.id),snapshot.pilotage?.health[a?.id]?.process_label])]);
  if(sig===this.listKey)return;this.listKey=sig;
  const focused=document.activeElement?.dataset.pilotIdentity,host=$('pilot-list');host.replaceChildren();
  const groups=this.state.grouped?['En activité','À examiner','Historique','À préparer']:[''];
  for(const group of groups){
   const set=entries.filter(({t,a})=>!group||(a&&active(a)?'En activité':t.status==='submitted'||t.status==='blocked'?'À examiner':a?'Historique':'À préparer')===group);
   if(!set.length)continue;
   let target=host;if(group){const details=node('details',undefined,'pilot-group'),summary=node('summary',group+' · '+set.length);details.open=this.state.groups[group]??(group!=='Historique');summary.dataset.pilotIdentity='group:'+group;details.addEventListener('toggle',()=>{if(details.isConnected){this.state.groups[group]=details.open;this.save()}});details.append(summary);host.append(details);target=details;}
   for(const {t,a}of set){
    const card=node('article',undefined,'pilot-card');card.dataset.state=this.uncertainExecution(t,a)?'stale':snapshot.validation?.tasks[t.id]?.state||t.status;
    const select=this.command(this.taskTitle(t),()=>this.inspect(a?'agent':'task',a?.id||t.id),'pilot-card-title');select.dataset.pilotIdentity=a?.id||t.id;
    card.append(node('p',a?a.provider+' · '+a.role:'Tâche à préparer','pilot-eyebrow'),select);
    const h=snapshot.pilotage?.health[a?.id];
    card.append(node('p',a?this.uncertainExecution(t,a)||h?.process_label||'Observation indisponible':labels[t.status]||t.status,'pilot-card-state'));
    if(a)card.append(node('p',a.progress?.detail||a.progress?.action||h?.activity_label||'Aucune activité publique reçue','pilot-card-activity'));
    else if(t.depends?.length)card.append(node('p','Prérequis : '+t.depends.join(', ')));
    if(this.state.detail==='detailed')card.append(node('p',a?(h?.activity_label||'Activité inconnue')+' · '+(a.mode==='terminal'?'Appels non mesurés':(a.progress?.tool_calls||0)+' appels')+' · '+graphCoutTache(t.id):t.deliverable));
    const go=this.goState(t);if(go.visible){const b=this.command(go.ready?'Go — lancer':'Voir le blocage',()=>this.go(t.id),'pilot-go');b.dataset.pilotIdentity='go:'+t.id+':'+(a?.id||'');b.dataset.goTask=t.id;b.dataset.ready=String(go.ready);b.title=go.ready?'Configurer puis confirmer le lancement de '+t.title:go.reason;card.append(b);if(!go.ready)card.append(node('p',go.reason,'pilot-go-reason'));}
    target.append(card);
   }
  }
  if(!entries.length)host.append(node('p','Aucun élément pour ce filtre. Effacez la recherche ou affichez tout le travail.','notice info'));
  const summary=snapshot.pilotage?.summary;
  if(summary?.total>summary.loaded)host.append(node('p',summary.loaded+' tentatives chargées sur '+summary.total+' ; les relations précédentes restent consultables dans le détail.','pilot-status'));
  if(focused)[...host.querySelectorAll('[data-pilot-identity]')].find(n=>n.dataset.pilotIdentity===focused)?.focus();
 },
 summary(host){
  const p=snapshot.pilotage?.summary,open=snapshot.decisions.filter(d=>!d.resolved_at);
  const change=typeof filChangementsDepuisVisite==='function'?filChangementsDepuisVisite():null;
  const signature=JSON.stringify([work,p,this.interventions(),snapshot.work.tasks.map(t=>t.status),change,snapshot.cost_text]);
  if(host.dataset.signature===signature)return;const focused=document.activeElement?.dataset.summaryKey;host.dataset.signature=signature;host.replaceChildren();
  const parts=node('span',undefined,'pilot-summary');
  const add=(text,filter)=>{const b=this.command(text,()=>{this.state.filter=filter;this.state.view='agents';this.changed();$('pilot-list').scrollIntoView({block:'start'})});b.dataset.summaryKey=filter;parts.append(b)};
  add(!p||p.complete===false?'Nombre d’agents indisponible':p.confirmed?p.confirmed+(p.confirmed===1?' agent démarré':' agents démarrés'):'Aucun agent confirmé en cours','active');
  if(p?.unknown)add(p.unknown+' tentatives sans signal confirmé','unknown');
  const reports=snapshot.work.tasks.filter(t=>t.status==='submitted').length;
  if(reports)add(reports+' résultats à examiner','review');
  if(this.interventions().length){const b=this.command(this.interventions().length+' interventions à examiner',()=>this.nextIntervention());b.dataset.summaryKey='interventions';parts.append(b)}
  else parts.append(node('span','Aucune décision en attente'));
  host.append(parts);
  const visit=change?.entrees?(change.complet?'':'Au moins ')+change.entrees+(change.entrees===1?' événement':' événements')+' depuis votre visite':'Rien de neuf depuis votre visite';
  const visitButton=this.command(visit,()=>{$('fil-bloc').open=true;$('fil-titre').scrollIntoView({block:'start'});$('fil-titre').focus()},'pilot-summary-note accueil-segment');host.append(visitButton);
  if(snapshot.cost?.attempts_with_cost||snapshot.cost?.attempts_without_cost)host.append(node('span',snapshot.cost_text,'pilot-summary-note'));
  if(focused)[...host.querySelectorAll('[data-summary-key]')].find(n=>n.dataset.summaryKey===focused)?.focus();
 },
 interventions(){
  const decisions=snapshot.decisions.filter(d=>!d.resolved_at).sort((a,b)=>(a.created||'').localeCompare(b.created||'')||a.id.localeCompare(b.id));
  const covered=new Set(decisions.map(d=>d.task_id));
  return [...decisions.map(d=>({kind:'decision',id:d.id})),...snapshot.work.tasks.filter(t=>['blocked','submitted'].includes(t.status)&&!covered.has(t.id)).map(t=>({kind:'task',id:t.id}))];
 },
 nextIntervention(delta=1){
  if(!this.queue){this.returnView=JSON.parse(JSON.stringify(this.state));this.queue={ids:this.interventions(),index:-1}}
  if(!this.queue.ids.length){notice('Aucune intervention requise.');this.queue=null;return}
  this.queue.index=Math.max(0,Math.min(this.queue.ids.length-1,this.queue.index+delta));
  const item=this.queue.ids[this.queue.index];this.inspect(item.kind,item.id);
 },
 returnToView(){const saved=this.returnView;PilotInspector.close();this.queue=null;this.returnView=null;if(saved){this.state=PilotGraph.preferences(saved);this.restorePosition=true}this.changed()},
 help(){
  openModal('Piloter les agents','Comprendre, inspecter puis agir sur une tentative précise.',{action:'help'});$('confirm').hidden=true;$('cancel').textContent='Fermer l’aide';
  preview('Agents : liste des tentatives et tâches à préparer. Dépendances : flèches du prérequis vers la suite.\nHorizontal / Vertical règle le sens des flèches ; Simplifié / Détaillé règle le contenu des cartes.\n+ / − replie une branche sans changer le travail. Une tâche partagée reste visible par un autre chemin ouvert. Les badges comptent les éléments réellement masqués.\nProchaine intervention : décisions les plus anciennes d’abord (date puis identifiant), puis tâches bloquées ou résultats à examiner. L’ordre reste figé pendant la visite. Aucune commande n’est exécutée.\nUn arrêt demandé n’est pas un arrêt confirmé. Une exécution terminée n’est pas un livrable validé.\nRetrouver ma sélection révèle la branche et retire les filtres masquants. Réinitialiser l’affichage ne modifie aucune tâche.');
 }
};
