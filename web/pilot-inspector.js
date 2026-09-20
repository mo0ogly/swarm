'use strict';
const tr_web_pilot_inspector_js = source => globalThis.SwarmI18n?.t(source) ?? source;

const PilotInspector={
 generation:0,key:'',loaded:null,opener:null,
 mount(){
  if($('pilot-inspector'))return;
  const panel=node('dialog',undefined,'pilot-inspector');panel.id='pilot-inspector';panel.setAttribute('aria-labelledby','pilot-inspector-title');
  const header=node('header',undefined,'pilot-inspector-head');
  const title=node('h2');title.id='pilot-inspector-title';
  const close=Pilot.command(tr_web_pilot_inspector_js('Fermer le détail'),()=>this.close());header.append(title,close);
  const body=node('div',undefined,'pilot-inspector-body');body.id='pilot-inspector-body';
  panel.append(header,body);document.body.append(panel);
  panel.addEventListener('cancel',e=>{e.preventDefault();this.close()});
  panel.addEventListener('keydown',e=>{if(e.key==='Escape'&&!$('modal').open){e.preventDefault();this.close()}});
 },
 close(){
  this.generation++;$('pilot-inspector')?.close();Pilot.state.selection=null;Pilot.inspectorKey='';Pilot.save();
  if(this.opener?.isConnected)this.opener.focus();else $('pilot-view')?.focus();
 },
 async render(open=false){
  this.mount();
  const selected=Pilot.state.selection;if(!selected)return;
  const panel=$('pilot-inspector'),requestWork=work;
  if(open){this.opener=document.activeElement;if(!panel.open){if(matchMedia('(max-width: 760px)').matches)panel.showModal();else panel.show()}}
  if(!panel.open&&!open){if(matchMedia('(max-width: 760px)').matches)panel.showModal();else panel.show()}
  let decision=selected.kind==='decision'?snapshot.decisions.find(d=>d.id===selected.id):null;
  const agentId=selected.kind==='agent'?selected.id:decision?.agent_id;
  let a=agentId?snapshot.agents.find(x=>x.agent.id===agentId)?.agent:null;
  if(agentId&&!a){
   if(this.loaded?.agent.id===agentId&&this.loaded.agent.work_id===work)a=this.loaded.agent;
   else{
    const token=++this.generation;
    $('pilot-inspector-title').textContent=tr_web_pilot_inspector_js('Chargement de la tentative');$('pilot-inspector-body').textContent=tr_web_pilot_inspector_js('Lecture de l’historique…');
    try{
     const data=await api('/api/v1/agent-detail?'+new URLSearchParams({work,agent:agentId}));
     if(token!==this.generation||work!==requestWork||Pilot.state.selection?.id!==selected.id)return;
     this.loaded=data;a=data.agent;
    }catch(e){if(token===this.generation&&work===requestWork){$('pilot-inspector-title').textContent=tr_web_pilot_inspector_js('Tentative indisponible');$('pilot-inspector-body').textContent=tr_web_pilot_inspector_js('Cette tentative ne peut plus être consultée dans ce travail. Revenez à votre vue pour choisir un autre agent.');this.queueFooter($('pilot-inspector-body'))}return}
   }
  }
  const tid=selected.kind==='task'?selected.id:decision?.task_id||a?.task_id;
  const t=snapshot.work.tasks.find(t=>t.id===tid);
  if(!a&&t&&selected.kind==='task')a=snapshot.agents.find(x=>x.agent.task_id===tid)?.agent;
  const h=snapshot.pilotage?.health[a?.id]||(a&&this.loaded?.agent.id===a.id?this.loaded.health:null);
  const validation=snapshot.validation?.tasks[tid],uncertain=Pilot.uncertainExecution(t,a,h);
  const shown=PilotGraph.visible(snapshot.work.tasks,Pilot.state.collapsed);
  const masked=t&&(!shown.has(t.id)||!Pilot.matches(t,a));
  const signature=JSON.stringify([work,selected,t,a?.progress,a?.status,a?.usage,(globalThis.SwarmI18n?.engine(h?.process_label) ?? h?.process_label),(globalThis.SwarmI18n?.engine(h?.activity_label) ?? h?.activity_label),validation,decision,masked,Pilot.queue,Pilot.interventions(),snapshot.task_actions?.[tid]]);
  if(signature===Pilot.inspectorKey&&!open)return;Pilot.inspectorKey=signature;const token=++this.generation;
  const body=$('pilot-inspector-body'),focusKey=document.activeElement?.dataset.inspectorAction;
  const activityExpanded=body.querySelector('#pilot-activity-details')?.open===true;
  body.replaceChildren();
  $('pilot-inspector-title').textContent=decision?tr_web_pilot_inspector_js('Intervention à examiner'):t?.status==='submitted'?tr_web_pilot_inspector_js('Résultat à examiner')+(a?' — '+a.provider:''):a?'Agent '+a.provider:Pilot.taskTitle(t);
  const mission=node('section',undefined,'pilot-mission');mission.append(node('p',snapshot.work.title,'pilot-eyebrow'));body.append(mission);
  if(!t&&!decision){body.append(node('p',tr_web_pilot_inspector_js('Élément supprimé ou indisponible. Aucune commande ne sera exécutée.'),'notice attention'));this.queueFooter(body);return}
  if(t){
   mission.append(node('h3',Pilot.taskTitle(t)));
   if(t.launch_held){
    const plan=snapshot.work.plans?.find(p=>p.source?.startsWith('prep-')&&p.task_ids?.includes(t.id));
    const held=node('section',undefined,'notice attention');held.append(node('p',tr_web_pilot_inspector_js('Mission créée ; son démarrage attend votre autorisation.')));
    if(plan){const link=node('a',tr_web_pilot_inspector_js('Ouvrir la préparation pour autoriser les missions'));link.href='/prepare.html?id='+encodeURIComponent(plan.source);link.dataset.preparation=plan.source;held.append(link)}
    body.append(held);
   }
   const state=validation?.state||t.status;const badge=node('p',uncertain||(state==='running'?tr_web_pilot_inspector_js('Résultat de la tâche : pas encore validé'):tr_web_pilot_inspector_js('Validation actuelle : ')+(labels[state]||state)),'pilot-validation');badge.dataset.state=uncertain?'stale':state;mission.append(badge);
   if(t.status==='submitted')body.append(this.reviewControls(t,a?.id));
   else if(!t.launch_held){const recovery=this.recoveryControls(t,a,h);if(recovery)body.append(recovery)}
   body.append(node('p',snapshot.work.objective));
   const purpose=node('section',undefined,'pilot-section');purpose.classList.add('pilot-purpose');purpose.append(node('h4',tr_web_pilot_inspector_js('Ce qui était demandé')));
   const criteria=node('ul');for(const c of t.criteria||[])criteria.append(node('li',c));purpose.append(criteria);
   body.append(purpose);
  }
  if(decision){
   body.append(node('p',decision.resolved_at?tr_web_pilot_inspector_js('Cette demande a été traitée ailleurs.'):decisionObservation(decision).summary,'notice attention'),node('p',decision.evidence));
   if(!decision.resolved_at)body.append(this.action(tr_web_pilot_inspector_js('Examiner et décider'),()=>openDecision(decision),'decision'));
  }
  if(masked){body.append(node('p',tr_web_pilot_inspector_js('Cette sélection est masquée dans le graphe ou par un filtre.'),'notice info'),this.action(tr_web_pilot_inspector_js('Révéler et retirer les filtres masquants'),()=>Pilot.revealSelection(),'reveal'))}
  if(a){
   body.append(this.action(['terminal','dialogue'].includes(a.mode)?tr_web_pilot_inspector_js('Ouvrir la session interactive'):tr_web_pilot_inspector_js('Voir la session de l’agent'),()=>AgentTerminal.open(a),'terminal'));
   const health=node('section',undefined,'pilot-section');health.id='pilot-current-activity';
   health.append(node('h4',tr_web_pilot_inspector_js('Ce que fait cet agent')));
   const explanation=this.activityExplanation(a,h,uncertain);
   health.append(node('p',explanation.state),node('p',explanation.operation),node('p',explanation.next));
   if(t?.deliverable)health.append(node('p',tr_web_pilot_inspector_js('Résultat attendu : ')+t.deliverable));
   health.append(node('p',typeof a.usage?.provider_reported_cost_usd==='number'?tr_web_pilot_inspector_js('Coût de cette tentative : ')+a.usage.provider_reported_cost_usd.toFixed(2)+' USD':tr_web_pilot_inspector_js('Coût inconnu : le fournisseur n’a pas transmis de montant à Swarm.')));
   const details=node('details');details.id='pilot-activity-details';details.open=activityExpanded;
   details.append(node('summary',tr_web_pilot_inspector_js('Voir la commande et les signaux techniques')));
   details.append(node('p',tr_web_pilot_inspector_js('Validation actuelle de la tâche : ')+(t?.status==='running'?tr_web_pilot_inspector_js('pas encore validée'):labels[validation?.state||t?.status]||'inconnue')));
   for(const [title,text]of [[tr_web_pilot_inspector_js('Exécution'),uncertain||(globalThis.SwarmI18n?.engine(h?.process_label) ?? h?.process_label)||tr_web_pilot_inspector_js('Observation indisponible')],[tr_web_pilot_inspector_js('Activité reçue'),(globalThis.SwarmI18n?.engine(h?.activity_label) ?? h?.activity_label)||tr_web_pilot_inspector_js('Non connue')]]){
    const row=node('p');row.append(node('strong',title+' : '),node('span',text));details.append(row);
   }
   details.append(node('p',tr_web_pilot_inspector_js('Dernière commande ou information reçue (peut être abrégée) :')),node('pre',a.progress?.detail||a.progress?.action||tr_web_pilot_inspector_js('Aucun détail disponible.')));
   if(h?.heartbeat)details.append(node('p',tr_web_pilot_inspector_js('Dernier contact avec le superviseur : ')+new Date(h.heartbeat).toLocaleString('fr')));
   if(a.progress?.last_result_at)details.append(node('p',tr_web_pilot_inspector_js('Dernière réponse d’un outil : ')+new Date(a.progress.last_result_at).toLocaleString('fr')));
   if(a.ended)details.append(node('p',tr_web_pilot_inspector_js('Fin observée : ')+new Date(a.ended).toLocaleString('fr')));
   if(a.exit_code!==undefined)details.append(node('p',tr_web_pilot_inspector_js('Code de sortie : ')+a.exit_code));
   health.append(details);body.insertBefore(health,mission.nextSibling);
   if(a.parent||a.previous)body.append(this.relations(a));
  }
  if(t){
   const descendants=PilotGraph.descendants(snapshot.work.tasks,t.id);
   const blockers=snapshot.pilotage?.tasks[t.id]?.waiting_on||[];
   const impacts=node('section',undefined,'pilot-section');
   impacts.append(node('h4',tr_web_pilot_inspector_js('Dépendances et impact')),node('p',descendants.length?descendants.length+tr_web_pilot_inspector_js(' tâches en aval : ')+descendants.join(', ')+'. Elles peuvent avoir d’autres prérequis.':tr_web_pilot_inspector_js('Aucune tâche ne dépend de celle-ci.')));
   if(blockers.length)impacts.append(node('p',tr_web_pilot_inspector_js('Prérequis manquants : ')+blockers.join(', ')));
   for(const message of validation?.blockers||[])impacts.append(node('p',message,'notice attention'));
   body.append(impacts);
   const actions=node('section',undefined,'pilot-section');actions.id='pilot-inspector-commands';
   actions.append(node('h4',tr_web_pilot_inspector_js('Que faire maintenant ?')));
   if(t.status==='submitted')actions.append(node('p',tr_web_pilot_inspector_js('L’exécution a produit un résultat. Lisez le rapport et les preuves avant toute décision de validation.')));
   if(Pilot.queue&&selected.kind==='task'&&!['blocked','submitted'].includes(t.status))actions.append(node('p',tr_web_pilot_inspector_js('Cette tâche ne requiert plus cette intervention. Son état a changé.'),'notice info'));
   if(t.status==='blocked')actions.append(node('p',t.blocker||tr_web_pilot_inspector_js('Examiner le motif de blocage avant toute reprise.')));
   const opts=snapshot.task_actions?.[t.id]||[];
   const primary=opts.find(x=>x.conseillee&&x.disponible);
   if(h?.stop_requested)actions.append(node('p',tr_web_pilot_inspector_js('Arrêt déjà demandé — confirmation attendue.'),'notice info'));
   if(t.status!=='submitted'&&primary&&!(primary.kind==='stop'&&h?.stop_requested))actions.append(this.action(primary.kind==='report'?tr_web_pilot_inspector_js('Ouvrir les conclusions du rapport'):primary.label,()=>this.taskAction(t.id,a?.id,primary.kind),'primary','primary'));
   actions.append(this.action(tr_web_pilot_inspector_js('Toutes les actions autorisées'),()=>taskDialog(t.id,a?.id),'actions'));
   for(const x of opts.filter(x=>!x.disponible&&['start','retry','stop','accepted'].includes(x.kind)))actions.append(node('p',x.label+' : '+x.raison,'pilot-muted'));
   body.append(actions);
   const reports=node('section',undefined,'pilot-section');reports.id='pilot-reports';reports.dataset.review=String(t.status==='submitted');reports.append(node('h4',tr_web_pilot_inspector_js('Rapports et preuves de la tâche')),node('p',tr_web_pilot_inspector_js('Chargement des rapports disponibles…')));body.append(reports);
   api('/api/v1/task?'+new URLSearchParams({work,task:t.id})).then(data=>{
    if(token!==this.generation||work!==requestWork||!reports.isConnected)return;
    reports.replaceChildren(node('h4',tr_web_pilot_inspector_js('Rapports et preuves de la tâche')));
    if(!data.reports?.length)reports.append(node('p',tr_web_pilot_inspector_js('Aucun rapport détecté pour cette tâche.')));
    for(const path of data.reports||[])reports.append(this.action(tr_web_pilot_inspector_js('Ouvrir ')+path.split('/').pop(),()=>this.readReport(path,t.id),'report:'+path));
    reports.append(this.action(tr_web_pilot_inspector_js('Examiner les contrôles de validation'),()=>this.taskAction(t.id,a?.id,'gate'),'gate'));
   }).catch(e=>{if(token===this.generation&&reports.isConnected)reports.append(node('p',tr_web_pilot_inspector_js('Rapports indisponibles : ')+e.message,'notice alert'))});
  }
  this.queueFooter(body);
  const technical=node('details',undefined,'pilot-section');technical.append(node('summary',tr_web_pilot_inspector_js('Identifiants et détails techniques')));
  for(const [name,value]of [[tr_web_pilot_inspector_js('Tâche'),t?.id],[tr_web_pilot_inspector_js('Session agent'),a?.id],[tr_web_pilot_inspector_js('Tentative métier'),a?.attempt_id],[tr_web_pilot_inspector_js('Espace de travail'),a?.workspace],[tr_web_pilot_inspector_js('Livrable attendu'),t?.deliverable]])if(value)technical.append(node('p',name+' : '+value));
  body.append(technical);
  if(focusKey)[...body.querySelectorAll('[data-inspector-action]')].find(n=>n.dataset.inspectorAction===focusKey)?.focus();
 },
 activityExplanation(a,h,uncertain=''){
  const progress=a.progress||{},detail=progress.detail||'',action=progress.action||'';
  let operation=tr_web_pilot_inspector_js('Dernière action : ')+(action||tr_web_pilot_inspector_js('aucune description disponible.'));
  if(/sha256|hashlib/.test(detail)&&/source-snapshot|stable|diff/.test(detail))operation=tr_web_pilot_inspector_js('Dernière action observée : calculer et comparer les empreintes des fichiers pour vérifier si le code a changé pendant les contrôles.');
  else if(/swarm.*work show/.test(detail))operation=tr_web_pilot_inspector_js('Dernière action observée : consulter les tâches et leurs validations dans Swarm.');
  else if(/\bgo test\b|\bpytest\b|\bunittest\b/.test(detail))operation=tr_web_pilot_inspector_js('Dernière action observée : exécuter des tests. Leur résultat doit encore être examiné.');
  else if(/\bgo build\b|\bnpm run build\b/.test(detail))operation=tr_web_pilot_inspector_js('Dernière action observée : compiler le projet.');
  const unknown=Boolean(uncertain||h?.process_state?.startsWith('unknown/')||h?.process_state?.endsWith('/unconfirmed'));
  let state=tr_web_pilot_inspector_js('L’agent est actif ; son avancement exact n’est pas connu.'),next=tr_web_pilot_inspector_js('Aucune action nécessaire pour ce lancement. Le résultat de la tâche reste à vérifier.');
  if(unknown){state=tr_web_pilot_inspector_js('Swarm ne peut pas confirmer que l’agent travaille encore.');next=tr_web_pilot_inspector_js('Consultez les signaux ci-dessous puis vérifiez la tentative avant de la relancer.')}
  else if(['queued','starting'].includes(a.status)){state=tr_web_pilot_inspector_js('Le démarrage est en attente de confirmation.');next=tr_web_pilot_inspector_js('Aucun travail de l’agent n’est encore confirmé.')}
  else if(a.status==='completed'){state=tr_web_pilot_inspector_js('L’exécution est terminée.');next=tr_web_pilot_inspector_js('Examinez le rapport et ses preuves pour vérifier le résultat de la tâche.')}
  else if(['failed','interrupted'].includes(a.status)){state=tr_web_pilot_inspector_js('L’exécution s’est arrêtée avant une fin normale.');next=tr_web_pilot_inspector_js('Examinez le motif d’arrêt et le rapport éventuel avant de décider de la suite.')}
  else if(progress.pending_tools>0){state=tr_web_pilot_inspector_js('L’agent attend la réponse d’un outil.');next=tr_web_pilot_inspector_js('Laissez cette opération se terminer ; sa réussite n’est pas encore connue.')}
  else if(progress.last_result_at){state=tr_web_pilot_inspector_js('La dernière réponse d’un outil a été reçue. Swarm attend la prochaine activité ou la fin de l’agent.')}
  if(h?.activity_state==='old'&&active(a)&&!unknown){state=tr_web_pilot_inspector_js('Aucune activité récente ne permet de confirmer la progression.');next=tr_web_pilot_inspector_js('Consultez la dernière action et les signaux avant de décider d’attendre ou d’intervenir.')}
  if(h?.stop_requested&&active(a)){state=tr_web_pilot_inspector_js('Un arrêt a été demandé ; sa confirmation est attendue.');next=tr_web_pilot_inspector_js('Attendez la confirmation de l’arrêt avant de relancer.')}
  return {state,operation,next};
 },
 relations(agent){
  const section=node('section',undefined,'pilot-section');section.id='pilot-relations';section.append(node('h4',tr_web_pilot_inspector_js('Origine de cette tentative')));
  const svgNode=(name,attrs={},text)=>{const el=document.createElementNS('http://www.w3.org/2000/svg',name);for(const [key,value]of Object.entries(attrs))el.setAttribute(key,value);if(text!==undefined)el.textContent=text;return el};
  const relations=[['parent','Agent parent',agent.parent],['previous',tr_web_pilot_inspector_js('Tentative précédente'),agent.previous]].filter(x=>x[2]);
  const svg=svgNode('svg',{viewBox:'0 0 330 '+relations.length*68,role:'img','aria-label':tr_web_pilot_inspector_js('Relations vers cette tentative : ')+relations.map(x=>x[1]).join(tr_web_pilot_inspector_js(' et ')),class:'pilot-relations-graph'});
  for(const [index,[kind,label,id]]of relations.entries()){
   const y=index*68+24;
   svg.append(svgNode('text',{x:4,y:y-8},label));
   svg.append(svgNode('path',{d:kind==='parent'?`M4 ${y+4} H236 M4 ${y+10} H236`:`M4 ${y+7} H236`,class:'pilot-relation-line '+kind,'data-relation':kind,'data-source-agent':id,'data-target-agent':agent.id}));
   svg.append(svgNode('path',{d:`M232 ${y+1} L240 ${y+7} L232 ${y+13}`,class:'pilot-relation-arrow'}));
   svg.append(svgNode('text',{x:247,y:y+11},'Actuelle'));
   section.append(this.action(label,()=>{this.loaded=null;Pilot.inspect('agent',id)},label));
  }
  section.insertBefore(svg,section.children[1]);
  section.append(node('p',tr_web_pilot_inspector_js('Double trait : filiation d’agent. Pointillés : reprise d’une tentative. Les dépendances de tâches restent dans la vue Dépendances.'),'pilot-muted'));
  return section;
 },
 queueFooter(body){
  if(!Pilot.queue)return;
  body.append(node('p',(Pilot.queue.index+1)+tr_web_pilot_inspector_js(' sur ')+Pilot.queue.ids.length+tr_web_pilot_inspector_js(' demandes de cette visite')));
  const existing=new Set(Pilot.queue.ids.map(x=>x.kind+':'+x.id));
  const added=Pilot.interventions().filter(x=>!existing.has(x.kind+':'+x.id)).length;
  if(added)body.append(node('p',added+tr_web_pilot_inspector_js(' nouvelles interventions ; elles seront accessibles à la prochaine visite.'),'notice info'));
  const prev=this.action(tr_web_pilot_inspector_js('Précédente'),()=>Pilot.nextIntervention(-1),'prev');prev.disabled=Pilot.queue.index===0;
  const next=this.action(tr_web_pilot_inspector_js('Voir la suivante'),()=>Pilot.nextIntervention(1),'next');next.disabled=Pilot.queue.index===Pilot.queue.ids.length-1;
  body.append(prev,next,this.action(tr_web_pilot_inspector_js('Revenir à ma vue'),()=>Pilot.returnToView(),'return'));
 },
 recoveryControls(t,a,h){
  const options=snapshot.task_actions?.[t.id]||[];
  if(!['running','blocked','todo'].includes(t.status))return null;
  const section=node('section',undefined,'pilot-section pilot-review-controls');section.id='pilot-recovery';section.append(node('h4',tr_web_pilot_inspector_js('Agir sur cette tâche')));
  const add=(kind,label,primary=false)=>{const opt=options.find(x=>x.kind===kind);if(!opt?.disponible)return false;const b=this.action(label,()=>this.recoveryAction(t.id,a?.id,kind), 'recovery:'+kind,primary?'primary':'');b.dataset.recoveryAction=kind;section.append(b);return true};
  if(a&&active(a)){
   if(h?.can_cancel_pending){
    section.append(node('p',tr_web_pilot_inspector_js('Ce démarrage est resté en attente sans prise en charge. Annulez-le pour libérer la tâche, puis choisissez de la relancer.')));
    add('reconcile',tr_web_pilot_inspector_js('Annuler ce démarrage en attente'),true);
   }else{
    if(h?.stop_requested)section.append(node('p',tr_web_pilot_inspector_js('Un arrêt a déjà été demandé. La tâche sera libérée après confirmation de la fin.')));
    else add('stop',tr_web_pilot_inspector_js('Demander l’arrêt de la tentative'),true);
    if(h?.same_host)add('reconcile',tr_web_pilot_inspector_js('Vérifier la fin et libérer la tâche'),Boolean(h?.stop_requested));
    else section.append(node('p',tr_web_pilot_inspector_js('Cette session ne peut pas vérifier les processus de la session d’origine. Vérifiez leur arrêt depuis cette session avant de libérer la tâche.'),'notice attention'));
   }
  }else{
   section.append(node('p',t.status==='blocked'?(t.blocker||tr_web_pilot_inspector_js('La tentative est arrêtée. Vous pouvez préparer une nouvelle exécution.')):tr_web_pilot_inspector_js('Choisissez une exécution pour cette tâche.')));
   if(!add('extend-attempt',tr_web_pilot_inspector_js('Autoriser une tentative supplémentaire'),true)&&!add('retry',tr_web_pilot_inspector_js('Relancer cette tâche'),true))add('start',tr_web_pilot_inspector_js('Lancer cette tâche'),true);
   const refusal=options.find(x=>x.kind==='retry'&&!x.disponible)||options.find(x=>x.kind==='start'&&!x.disponible);
   if(!section.querySelector('button')&&refusal?.raison)section.append(node('p',refusal.raison,'notice attention'));
  }
  const refreshButton=this.action(tr_web_pilot_inspector_js('Actualiser l’état'),async()=>{refreshButton.disabled=true;try{await refresh(true)}catch(e){notice(e.message,true)}finally{if(refreshButton.isConnected)refreshButton.disabled=false}},'recovery:refresh');section.append(refreshButton);
  return section;
 },
 async recoveryAction(task,agent,kind){
  await this.taskAction(task,agent,kind);
  if(modalContext?.task!==task||modalContext.action!==kind)return;
  const h=snapshot.pilotage?.health[agent];
  if(kind==='reconcile'){
   modalContext.cancelPending=Boolean(h?.can_cancel_pending);
   $('modal-title').textContent=h?.can_cancel_pending?tr_web_pilot_inspector_js('Annuler le démarrage en attente'):tr_web_pilot_inspector_js('Vérifier la fin de la tentative');
   $('modal-description').textContent=h?.can_cancel_pending?tr_web_pilot_inspector_js('Le moteur annulera uniquement un démarrage encore en attente. Si sa prise en charge a commencé, il refusera l’annulation. La tâche restera non validée et pourra être relancée.'):tr_web_pilot_inspector_js('Le moteur vérifie la disparition du superviseur et des processus avant de libérer la tâche. Si la vérification est impossible, il explique pourquoi.');
   $('confirm').textContent=h?.can_cancel_pending?tr_web_pilot_inspector_js('Confirmer l’annulation du démarrage'):tr_web_pilot_inspector_js('Vérifier et libérer si terminé');
   for(const id of ['field-action','field-agent']){const field=$(id);if(field){const target=node('input');target.type='hidden';target.name=field.name;target.id=field.id;target.value=field.value;field.parentElement.replaceWith(target)}}
   $('cancel').textContent=tr_web_pilot_inspector_js('Revenir au détail');
   preview(h?.can_cancel_pending?tr_web_pilot_inspector_js('Après confirmation : démarrage annulé, réservation libérée, puis bouton « Relancer cette tâche ». Aucune nouvelle exécution ne sera lancée automatiquement.'):tr_web_pilot_inspector_js('La tâche reste verrouillée si un processus est encore présent ou si sa fin ne peut pas être vérifiée.'));

  }
 },
 reviewControls(t,agent,includeReport=true){
  const section=node('section',undefined,'pilot-section pilot-review-controls');section.setAttribute('aria-label',tr_web_pilot_inspector_js('Décider après lecture du rapport'));
  section.append(node('h4',tr_web_pilot_inspector_js('Décider après lecture')));
  const options=snapshot.task_actions?.[t.id]||[];
  const accepted=options.find(x=>x.kind==='accepted');
  const invoke=kind=>{if($('modal').open)closeModal();this.taskAction(t.id,agent,kind)};
  const add=(kind,label,cls='')=>{const option=options.find(x=>x.kind===kind);if(!option)return;const b=this.action(label,()=>invoke(kind),'review:'+kind,cls);b.dataset.reviewAction=kind;b.disabled=!option.disponible;section.append(b);return b};
  if(includeReport)add('report',tr_web_pilot_inspector_js('Lire le rapport'),accepted?.disponible?'':'primary');
  add('gate',tr_web_pilot_inspector_js('Vérifier les preuves'));
  const accept=add('accepted',tr_web_pilot_inspector_js('Valider la tâche'),accepted?.disponible?'primary':'');
  if(!accepted?.disponible){
   const reason=node('p',tr_web_pilot_inspector_js('Validation indisponible : une évaluation des preuves (« gate delivery ») valide doit être enregistrée.'),'pilot-review-reason');
   reason.id='review-reason-'+t.id+(includeReport?'-panel':'-report');if(accept)accept.setAttribute('aria-describedby',reason.id);section.append(reason);
  }
  const override=options.find(x=>x.kind==='override');
  if(override?.disponible){
   section.append(node('p',tr_web_pilot_inspector_js('Décision manuelle possible avec un motif. Elle sera enregistrée comme dérogation ; les contrôles ne seront pas déclarés réussis.'),'pilot-review-reason'));
   add('override',tr_web_pilot_inspector_js('Accepter par dérogation motivée'),'pilot-review-override');
  }
  return section;
 },
 action(text,fn,key,cls=''){const b=Pilot.command(text,fn,cls);b.dataset.inspectorAction=key;return b},
 async taskAction(task,agent,kind){
  if(kind==='report'){const requested=work,selection=JSON.stringify(Pilot.state.selection);try{const data=await api('/api/v1/task?'+new URLSearchParams({work,task}));if(work!==requested||JSON.stringify(Pilot.state.selection)!==selection)return;if(data.reports?.length===1){await this.readReport(data.reports[0],task);return}}catch(e){notice(e.message,true);return}}
  await taskDialog(task,agent);
  if(modalContext?.task===task&&(modalContext.data.actions||[]).some(x=>x.kind===kind&&x.disponible)){
   taskFields(kind);
   if(kind==='report'){$('confirm').textContent=tr_web_pilot_inspector_js('Ouvrir les conclusions du rapport');$('modal-description').textContent=tr_web_pilot_inspector_js('Ce rapport présente le résultat de la mission. Sa lecture ne valide pas la tâche.')}
  }
 },
 async readReport(path,taskID){
  const selected=JSON.stringify(Pilot.state.selection),requested=work;
  try{
   const data=await api('/api/v1/report?'+new URLSearchParams({path}));
   if(work!==requested||JSON.stringify(Pilot.state.selection)!==selected)return;
   openModal(tr_web_pilot_inspector_js('Conclusions du rapport'),tr_web_pilot_inspector_js('Lisez les constats, les preuves et les limites avant de décider.'),{action:'help'});
   $('confirm').hidden=true;$('cancel').textContent=tr_web_pilot_inspector_js('Fermer le rapport');preview(data.text);
   const selection=Pilot.state.selection;
   taskID=taskID||(selection?.kind==='task'?selection.id:selection?.kind==='agent'?snapshot.agents.find(x=>x.agent.id===selection.id)?.agent.task_id:snapshot.decisions.find(x=>x.id===selection?.id)?.task_id);
   if(taskID){mountReportSummary(path,taskID);const task=snapshot.work.tasks.find(t=>t.id===taskID);if(task?.status==='submitted')$('modal-fields').append(this.reviewControls(task,null,false))}
  }catch(e){if(work!==requested||JSON.stringify(Pilot.state.selection)!==selected)return;const host=$('pilot-reports');host?.querySelector('.pilot-report-error')?.remove();const error=node('p',tr_web_pilot_inspector_js('Rapport inaccessible. Le fichier a pu être déplacé, supprimé ou son accès interrompu. Vérifiez sa disponibilité puis réessayez.'),'notice alert pilot-report-error');error.setAttribute('role','alert');host?.append(error);notice(error.textContent,true)}
 }
};
