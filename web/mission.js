'use strict';
const Mission={
 key:'',
 overview(d){
  const needs=d.tasks.filter(t=>['review','intervention'].includes(t.state)).sort((a,b)=>b.impact-a.impact);
  const counts=d.validated+'/'+d.total+' résultats validés · '+d.running+' en cours · '+needs.length+' décision(s) attendue(s).';
  if(!d.total)return {summary:'Ce travail ne contient encore aucune tâche.',next:'Préparez le besoin avec l’IA pour construire un plan.',label:'Préparer les tâches',kind:'prepare',tone:'info'};
  if(needs.length){const t=needs[0];return {summary:counts,next:t.title+' : '+t.reason,label:t.state==='review'?'Examiner le résultat':'Résoudre le blocage',kind:'decision',task:t,tone:'attention'}}
  const running=d.tasks.find(t=>t.state==='running');
  if(running)return {summary:counts,next:'En cours : '+running.title+(d.paused?'. Les départs suivants sont en pause.':'. Ouvrez le journal pour voir la dernière activité reçue.'),label:'Suivre cette tâche',kind:'follow',task:running,tone:'info'};
  if(d.tasks.every(t=>['validated','waived','abandoned'].includes(t.state)))return {summary:counts,next:d.validated===d.total?'Tous les résultats sont validés sur les preuves actuelles.':'Travail clôturé avec des étapes non validées : consultez les décisions.',label:'Voir les résultats',kind:'results',tone:d.validated===d.total?'succes':'attention'};
  if(d.paused)return {summary:counts,next:'La mission est en pause. Reprenez-la pour autoriser les prochains départs.',label:'Reprendre la mission',kind:'resume',tone:'info'};
  if(!d.enabled)return {summary:counts,next:'Les tâches ne s’enchaînent pas encore automatiquement. Lancez la mission pour poursuivre.',label:'Lancer la mission',kind:'start',tone:'info'};
  return {summary:counts,next:'Swarm attend les conditions de départ et réessaie automatiquement. Vous pouvez consulter le détail.',label:'Voir ce qui attend',kind:'results',tone:'info'};
 },
 primary(o){
  if(o.kind==='decision'){if(o.task.state==='review')this.action(o.task);else MissionHelp.open(o.task.id);return}
  if(o.kind==='follow'){this.follow(o.task.id);return}
  if(o.kind==='resume'){this.control('mission-resume');return}
  if(o.kind==='start'){this.open();return}
  if(o.kind==='prepare'){location.href='/prepare.html';return}
  const details=$('mission-results');if(details){details.open=true;details.querySelector('summary').focus();details.scrollIntoView({block:'nearest'})}
 },
 open(settings=false){
  const d=snapshot.mission;
  if(d?.enabled&&!settings){
   openModal(d.paused?'Mission en pause':'La mission est déjà en cours',d.paused?'Les nouveaux départs sont suspendus. Cliquez sur « Reprendre la mission » pour autoriser la suite.':'Swarm garde la suite en attente et la reprend automatiquement lorsque les conditions sont réunies.',{action:'help'});
   $('confirm').hidden=true;$('cancel').textContent='Fermer';
   const host=node('section',undefined,'mission-summary mission-help');host.id='mission-live-status';
   this.status(host,d,true);
   $('modal-fields').append(host);return;
  }
  const p=snapshot.work.launch_profile||{};
  openModal(settings?'Réglages de la mission':'Lancer tout','Swarm enchaîne les tâches dans le bon ordre. Si une tâche doit attendre, elle reprend automatiquement dès que possible. Les résultats à valider vous seront signalés.',{action:'mission-start'});
  field('provider','IA utilisée',p.provider||(Object.keys(snapshot.providers?.providers||{}).length===1?Object.keys(snapshot.providers.providers)[0]:''), [['','Choisir…'],...Object.keys(snapshot.providers?.providers||{}).map(x=>[x,x])]);
  field('workspace','Dossier du projet',p.workspace||snapshot.root);
  field('slots','Agents simultanés',String(snapshot.slots||2));$('field-slots').type='number';$('field-slots').min='1';$('field-slots').max='16';
  addModelFields('work','',p.level||'auto');
  $('modal-fields').append(node('p','Les profils propres aux tâches sont conservés. Un espace commun est utilisé à tour de rôle. Budgets, prérequis, arrêts demandés et limites de tentatives restent appliqués. Une synthèse IA ne valide pas les preuves.','notice info'));
  if(p.instruction){const existing=node('details');existing.append(node('summary','Consignes communes conservées'),node('p',p.instruction));$('modal-fields').append(existing)}
  $('confirm').textContent=settings?'Enregistrer et poursuivre':'Lancer la mission';
 },
 follow(id){
  const a=snapshot.agents.find(x=>x.agent.task_id===id)?.agent;
  if($('modal').open)closeModal();
  if(a)AgentTerminal.open(a);else Pilot.inspect('task',id);
 },
 status(host,d,modal=false){
  const focused=document.activeElement?.dataset.missionAction;
  const counts=node('div',undefined,'mission-kpis');
  const waived=d.tasks.filter(t=>t.state==='waived').length,waiting=d.tasks.filter(t=>t.state==='waiting').length;
  for(const [value,label,tone] of [[d.validated,'validés',d.validated?'success':'neutral'],[d.running,'en cours','info'],[d.review,'à examiner','attention'],[waiting,'en attente','neutral'],[waived,'passés sans validation','attention']]){
   if(!value&&label!=='validés')continue;
   const item=node('p',undefined,'mission-kpi');item.dataset.tone=tone;item.append(node('strong',String(value)),node('span',label));counts.append(item);
  }
  host.replaceChildren(counts);let uncertain=false;
  if(waived)host.append(node('p','Certaines étapes ont été passées par décision explicite, sans valider leurs contrôles. Elles ne comptent pas comme des résultats validés.','mission-explanation'));
  for(const t of d.tasks.filter(t=>t.state==='running')){
   const a=snapshot.agents.find(x=>x.agent.task_id===t.id)?.agent,h=snapshot.pilotage?.health[a?.id];
   const row=node('section',undefined,'mission-current');row.append(node('h4','En ce moment : '+t.title));
   if(!a||h?.activity_state==='old'||h?.process_state?.startsWith('unknown/')||h?.process_state?.endsWith('/unconfirmed'))uncertain=true;
   if(a){
    const text=PilotInspector.activityExplanation(a,h,Pilot.uncertainExecution(snapshot.work.tasks.find(x=>x.id===t.id),a,h));
    row.append(node('p',text.state),node('p',text.operation));
   }else row.append(node('p','La tâche est marquée en cours, mais aucun agent associé n’est visible. Vérifiez son état.'));
   const follow=Pilot.command('Voir le journal de l’agent',()=>this.follow(t.id));follow.dataset.missionAction='follow-'+t.id;
   row.append(follow,this.helpButton(t));host.append(row);
  }
  if(waiting)host.append(node('p',waiting+' tâche(s) attendent une étape précédente ou un espace disponible.'+(d.enabled&&!d.paused?' Swarm les reprendra dès que les conditions seront réunies.':'')));
  const needs=d.tasks.filter(t=>['review','intervention'].includes(t.state));
  if(!uncertain&&!needs.length&&d.running&&d.enabled&&!d.paused)host.append(node('p','Aucune action demandée pour le moment.','notice info'));
  if(uncertain)host.append(node('p','L’activité d’un agent doit être vérifiée. Ouvrez son journal ou demandez une explication.','notice attention'));
  if(modal){
   $('modal-title').textContent=d.paused?'Mission en pause':d.enabled?'Suivi de la mission':'Mission arrêtée';
   $('modal-description').textContent=d.paused?'Les nouveaux départs sont suspendus. Reprenez la mission pour autoriser la suite.':d.enabled?'L’avancement se met à jour automatiquement.':'Aucun nouveau départ automatique.';
   for(const t of needs.slice(0,3)){const row=node('section',undefined,'mission-intervention');row.append(node('h4',t.title),node('p',t.reason),Pilot.command(t.label,()=>{closeModal();Mission.action(t)}),this.helpButton(t));host.append(row)}
   host.append(node('p',d.next));
   if(d.paused){const resume=Pilot.command('Reprendre la mission',async()=>{if(await this.control('mission-resume'))closeModal()},'primary');resume.dataset.missionAction='resume';host.append(resume)}
   host.append(Pilot.command('Modifier les réglages',()=>this.open(true)));
  }
  if(focused)[...host.querySelectorAll('[data-mission-action]')].find(e=>e.dataset.missionAction===focused)?.focus();
 },
 async control(kind){
  const requested=work;
  try{
   const fresh=await api('/api/v1/snapshot?work='+encodeURIComponent(requested));
   if(work!==requested)return false;
   await act(kind,{work:requested,expected_revision:fresh.work.revision});await refresh(true);return true;
  }catch(e){notice(e.message,true);return false}
 },
 action(t){
  if(t.action==='configure'){this.open(true);return}
  if(t.action==='prepare'){const plan=snapshot.work.action_plans?.find(p=>p.tasks?.includes(t.id));if(plan?.source){location.href='/prepare.html?id='+encodeURIComponent(plan.source);return}}
  if(t.action==='inspect'||t.action==='prepare'){Pilot.inspect('task',t.target||t.id);return}
  if(t.action==='start'){Pilot.go(t.target||t.id);return}
  PilotInspector.taskAction(t.target||t.id,null,t.action).catch(e=>notice(e.message,true));
 },
 helpButton(t){const b=Pilot.command('Expliquer avec l’IA',()=>MissionHelp.open(t.id));b.dataset.missionAction='help-'+t.id;b.dataset.missionHelp=t.id;return b},
 render(){
  const d=snapshot.mission,host=$('mission-summary');if(!d||!host)return;
  const key=JSON.stringify([work,d,snapshot.agents.map(x=>[x.agent.id,x.agent.status,x.agent.progress]),snapshot.pilotage?.health]);if(this.key===key)return;this.key=key;
  const focus=document.activeElement?.dataset.missionAction,expanded=host.querySelector('details')?.open===true;
  const launch=$('pilot-mission');if(launch){launch.textContent='Réglages de la mission';launch.hidden=!d.enabled;launch.classList.remove('primary')}
  const state=d.paused?'Mission en pause':d.enabled?'Mission continue active':'Mission à poursuivre';
  const overview=this.overview(d);
  const heading=node('h3',overview.kind==='results'&&d.tasks.every(t=>['validated','waived','abandoned'].includes(t.state))?'Travail terminé':state),summary=node('section');this.status(summary,d);
  const brief=node('section',undefined,'mission-brief');brief.dataset.tone=overview.tone;brief.append(node('p',overview.summary),node('p',overview.next));
  const controls=node('div',undefined,'mission-controls');
  const main=Pilot.command(overview.label,()=>this.primary(overview),'primary');main.id='mission-primary';main.dataset.missionAction='primary';controls.append(main);
  const link=node('a','Lien permanent vers ce travail');link.href='/session/'+encodeURIComponent(csrf)+'?work='+encodeURIComponent(work);link.id='mission-link';controls.append(link);
  $('pilot-launch-all')?.classList.remove('primary');
  if(d.enabled&&overview.kind!=='resume'){const b=Pilot.command(d.paused?'Reprendre la mission':'Mettre en pause',()=>this.control(d.paused?'mission-resume':'mission-pause'));b.dataset.missionAction='control';controls.append(b)}
  host.replaceChildren(heading,brief,controls,summary);
  const live=$('mission-live-status');if(live&&$('modal').open&&modalContext?.workID===work)this.status(live,d,true);
  const actionable=d.tasks.filter(t=>['review','intervention'].includes(t.state)).sort((a,b)=>b.impact-a.impact);
  for(const t of actionable.slice(0,3)){const row=node('section',undefined,'mission-intervention');row.append(node('h4',t.title),node('p',t.reason+(t.impact?' · '+t.impact+' tâche(s) dépendent de ce résultat.':'')));const b=Pilot.command(t.label,()=>this.action(t));b.dataset.missionAction=t.id;row.append(b,this.helpButton(t));host.append(row)}
  const details=node('details');details.id='mission-results';details.open=expanded;details.append(node('summary','Résultats et prochaines actions — '+d.total+' tâches'));
  for(const t of d.tasks){const row=node('section',undefined,'mission-task');row.append(node('h4',t.title),node('p',t.reason));const expected=node('details');expected.append(node('summary','Résultat attendu'),node('p',t.deliverable));row.append(expected);const b=Pilot.command(t.label,()=>this.action(t));b.dataset.missionAction='detail-'+t.id;row.append(b,this.helpButton(t));details.append(row)}host.append(details);
  if(focus)[...host.querySelectorAll('[data-mission-action]')].find(e=>e.dataset.missionAction===focus)?.focus();
 }
};
