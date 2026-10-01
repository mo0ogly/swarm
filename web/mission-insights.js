'use strict';
const trMissionInsights=source=>globalThis.SwarmI18n?.t(source)??source;
Object.assign(Mission,{
 readOnly(title,description){openModal(title,description,{action:'help'});$('confirm').hidden=true;$('cancel').textContent=trMissionInsights('Fermer');return $('modal-fields')},
 changes(){
  const c=snapshot.mission.changes_since_visit,host=this.readOnly(trMissionInsights('Depuis votre dernière visite'),trMissionInsights('Ces événements sont historiques ; l’état actuel de la tâche fait foi.'));if(!c)return;
  if(c.first_visit){host.append(node('p',trMissionInsights('Première visite : aucun repère précédent. L’historique complet reste disponible.')));return}
  if(c.more)host.append(node('p',trMissionInsights('Extrait des 200 derniers événements ; consultez le fil complet pour les plus anciens.'),'notice attention'));
  if(!c.items.length)host.append(node('p',trMissionInsights('Aucun nouveau résultat, blocage ou décision dans cet extrait.')));
  for(const [category,label]of [['result','Résultats'],['block','Blocages'],['decision','Décisions']]){const items=c.items.filter(x=>x.category===category);if(!items.length)continue;const box=node('section',undefined,'mission-current field-wide');box.append(node('h3',trMissionInsights(label)+' · '+items.length));for(const x of items){const row=node('div');row.append(node('p',this.readableDate(x.at)+' · r'+x.revision+' · '+missionText(x.label)+(x.title?' — '+x.title:'')));if(x.task)row.append(Pilot.command(trMissionInsights('Voir l’état actuel'),()=>{closeModal();Pilot.inspect('task',x.task)}));box.append(row)}host.append(box)}
 },
 waits(id){
  const tasks=id?snapshot.mission.tasks.filter(t=>t.id===id):snapshot.mission.tasks.filter(t=>t.state==='waiting'||t.waiting_on?.length),host=this.readOnly(trMissionInsights('Pourquoi cette tâche attend ?'),trMissionInsights('Les conditions viennent du moteur ; ouvrir un prérequis ne lance aucun agent.'));
  if(!tasks.length)host.append(node('p',trMissionInsights('Aucune tâche ne signale d’attente.')));
  for(const t of tasks){const box=node('section',undefined,'mission-current field-wide');box.append(node('h3',t.title));if(!t.waiting_on?.length)box.append(node('p',missionText(t.reason)));for(const x of t.waiting_on||[]){box.append(node('p',x.title+' · '+missionText(x.reason)));if(snapshot.work.tasks.some(task=>task.id===x.task))box.append(Pilot.command(trMissionInsights('Ouvrir le prérequis')+' — '+x.title,()=>{closeModal();Pilot.inspect('task',x.task)}))}host.append(box)}
 },
 spendingView(c){
  const host=node('section',undefined,'field-wide');if(!c)return host;
  for(const [kind,label]of [['worker','Exécutants'],['planner','Responsables'],['reviewer','Vérificateur indépendant']]){const box=node('section',undefined,'mission-current field-wide');box.append(node('h3',trMissionInsights(label)));const rows=c.rows.filter(r=>r.kind===kind);if(!rows.length)box.append(node('p',trMissionInsights('Aucun appel enregistré pour ce rôle.')));for(const r of rows){box.append(node('h4',kind==='worker'?r.label:missionText(r.label)),node('p',r.recorded_calls+trMissionInsights(' tentatives ou appels enregistrés · ')+r.observed_tool_calls+trMissionInsights(' appels d’outils observés')),node('p',r.input_tokens+' / '+r.output_tokens+trMissionInsights(' jetons entrée/sortie rapportés · ')+r.calls_without_usage+trMissionInsights(' usages absents')),node('p',missionText(r.cost.attempts_with_cost?(r.cost.reported_usd.toFixed(2)+' USD rapportés sur '+r.cost.attempts_with_cost+' tentative(s)'):'coût réel non rapporté')+(r.cost.attempts_without_cost?' · '+r.cost.attempts_without_cost+trMissionInsights(' sans coût rapporté'):'')));if(r.attempts_with_incomplete_tool_measurement)box.append(node('p',r.attempts_with_incomplete_tool_measurement+trMissionInsights(' mesures d’outils incomplètes'),'notice attention'))}host.append(box)}
  host.append(node('h3',trMissionInsights('Moteur')),node('p',c.recorded_control_executions+trMissionInsights(' contrôles enregistrés · ')+c.worker_retries+trMissionInsights(' reprises d’agents')),node('p',missionText(c.note),'notice info'));return host;
 },
 spending(){this.readOnly(trMissionInsights('Où vont les appels et les coûts ?'),trMissionInsights('Appels IA, outils et contrôles sont présentés séparément.')).append(this.spendingView(snapshot.mission.spending))},
 recoveryPreviewView(p){
  const box=node('section',undefined,'mission-current field-wide');box.dataset.recoveryPreview=p.task;box.append(node('h3',trMissionInsights('Avant une relance')));for(const [label,items]of [['Ce qui sera conservé',p.kept],['Ce qui sera refait',p.redone],['Critères inchangés',p.criteria]]){box.append(node('h4',trMissionInsights(label)));const list=node('ul');for(const item of items||[])list.append(node('li',label==='Critères inchangés'?item:missionText(item)));box.append(list)}box.append(node('h4',trMissionInsights('Correction attendue')),node('p',p.correction||trMissionInsights('À préciser avant confirmation.')),node('p',missionText(p.limits),'notice info'));return box;
 },
 recoveryPreview(id){const p=snapshot.mission.tasks.find(t=>t.id===id)?.recovery_preview;if(p)this.readOnly(trMissionInsights('Avant une relance'),trMissionInsights('Relisez la correction et les critères ; aucune action n’est exécutée ici.')).append(this.recoveryPreviewView(p))},
 insightButtons(controls){for(const [label,action,id]of [['Depuis votre dernière visite',()=>this.changes(),'changes'],['Pourquoi cette tâche attend ?',()=>this.waits(),'waits'],['Où vont les appels et les coûts ?',()=>this.spending(),'spending']]){const b=Pilot.command(trMissionInsights(label),action);b.dataset.missionAction=id;controls.append(b)}},
 taskInsightButtons(row,t){if(t.waiting_on?.length||t.state==='waiting'){const b=Pilot.command(trMissionInsights('Pourquoi cette tâche attend ?'),()=>this.waits(t.id));b.dataset.missionAction='waits-'+t.id;row.append(b)}if(['intervention','review','manual','ready'].includes(t.state)){const b=Pilot.command(trMissionInsights('Avant une relance'),()=>this.recoveryPreview(t.id));b.dataset.missionAction='recovery-preview-'+t.id;row.append(b)}}
});
// Bind a read-only preview to the selected attempt and the actual instruction.
// A late response must not replace another task's or attempt's preview.
PilotActions.addRecoveryPreview=function(d){
 const context=modalContext,host=node('section',undefined,'field-wide');host.id='recovery-preview';$('modal-fields').append(host);
 const selected=$('field-agent'),instruction=$('field-recovery_instruction')||$('field-instruction');let generation=0;
 const update=async()=>{
  const current=++generation,agent=selected?.value||'',requested=context.workID;
  host.replaceChildren(node('p',trMissionInsights('Chargement de l’aperçu de reprise…')));
  try{const p=await api('/api/v1/recovery-preview?'+new URLSearchParams({work:requested,task:d.task.id,agent}));
   if(current!==generation||modalContext!==context||work!==requested||!host.isConnected)return;
   if(instruction)p.correction=instruction.value;host.replaceChildren(Mission.recoveryPreviewView(p));
  }catch(e){if(current===generation&&modalContext===context&&host.isConnected)host.replaceChildren(node('p',trMissionInsights('Aperçu de reprise indisponible : ')+e.message,'notice attention'))}
 };
 selected?.addEventListener('change',update);
 instruction?.addEventListener('input',()=>{const p=host.querySelector('[data-recovery-preview]');if(p){const headings=[...p.querySelectorAll('h4')];headings.at(-1)?.nextElementSibling?.replaceChildren(document.createTextNode(instruction.value||trMissionInsights('À préciser avant confirmation.')))}});
 update();
};
