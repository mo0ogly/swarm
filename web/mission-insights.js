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
  for(const [kind,label]of [['worker','Exécutants'],['planner','Responsables'],['reviewer','Vérificateur indépendant']]){const box=node('section',undefined,'mission-current field-wide');box.append(node('h3',trMissionInsights(label)));const rows=c.rows.filter(r=>r.kind===kind);if(!rows.length)box.append(node('p',trMissionInsights('Aucun appel enregistré pour ce rôle.')));for(const r of rows){box.append(node('h4',kind==='worker'?r.label:missionText(r.label)),node('p',r.recorded_calls+trMissionInsights(' tentatives ou appels enregistrés · ')+r.observed_tool_calls+trMissionInsights(' appels d’outils observés')),node('p',(r.recorded_calls>0&&r.calls_without_usage>=r.recorded_calls)?trMissionInsights('Jetons non rapportés.'):r.input_tokens+' / '+r.output_tokens+trMissionInsights(' jetons entrée/sortie rapportés · ')+r.calls_without_usage+trMissionInsights(' usages absents')),node('p',missionText(r.cost.attempts_with_cost?(r.cost.reported_usd.toFixed(2)+' USD rapportés sur '+r.cost.attempts_with_cost+' tentative(s)'):'coût réel non rapporté')+(r.cost.attempts_without_cost?' · '+r.cost.attempts_without_cost+trMissionInsights(' sans coût rapporté'):'')));if(r.attempts_with_incomplete_tool_measurement)box.append(node('p',r.attempts_with_incomplete_tool_measurement+trMissionInsights(' mesures d’outils incomplètes'),'notice attention'))}host.append(box)}
  host.append(this.attemptsView(c.attempts));
  host.append(node('h3',trMissionInsights('Moteur')),node('p',c.recorded_control_executions+trMissionInsights(' contrôles enregistrés · ')+c.worker_retries+trMissionInsights(' reprises d’agents')),node('p',missionText(c.note),'notice info'));return host;
 },
 attemptsView(rows){
  const host=node('section',undefined,'field-wide');host.append(node('h3',trMissionInsights('Bilan par tentative')),node('p',trMissionInsights('Un départ enregistré n’est pas un appel au modèle. Les appels internes du fournisseur ne sont pas mesurés.'),'notice info'));
  if(!rows?.length)host.append(node('p',trMissionInsights('Aucune tentative enregistrée.')));
  for(const r of rows||[]){
   const box=node('section',undefined,'mission-current field-wide');box.dataset.attemptLedger=r.attempt;
   box.append(node('h4',r.label),node('p',trMissionInsights('Tentative : ')+r.attempt+' · '+trMissionInsights('Agent : ')+r.agent),node('p',trMissionInsights('Processus : ')+trMissionInsights(({completed:'Processus terminé',interrupted:'Processus interrompu',failed:'Processus en échec',running:'Processus en cours',queued:'Démarrage en attente'})[r.process_state]||'État du processus inconnu')),node('p',trMissionInsights('Validation enregistrée de la tâche (toutes tentatives) : ')+trMissionInsights(r.task_acceptance_recorded?'oui':'non')),node('p',r.observed_tool_calls+trMissionInsights(' appels d’outils observés')));
   if(r.measured){
    box.append(node('p',trMissionInsights('Lectures : ')+r.tool_reads+' · '+trMissionInsights('Écritures : ')+r.tool_writes+' · '+trMissionInsights('Non classés : ')+r.tool_unclassified),node('p',trMissionInsights('Erreurs : ')+r.tool_errors_total+' · '+trMissionInsights('Répétitions : ')+r.tool_repeats_total));
    if(r.measurement==='partial')box.append(node('p',trMissionInsights('Mesure partielle : ces compteurs couvrent uniquement les événements reçus.'),'notice attention'));
   }else box.append(node('p',trMissionInsights('Mesure détaillée indisponible pour cette tentative : lectures, écritures, erreurs et répétitions restent inconnues, pas zéro.'),'notice attention'));
   box.append(node('p',trMissionInsights('Tests : ')+missionText(r.tests)));
   if(r.degraded)box.append(node('p',trMissionInsights('Mesure incomplète : ')+missionText(r.degraded),'notice attention'));
   if(r.usage_missing)box.append(node('p',trMissionInsights('Coût et usage non rapportés par le fournisseur pour cette tentative.')));
   else {box.append(node('p',r.input_tokens+' / '+r.output_tokens+trMissionInsights(' jetons entrée/sortie rapportés')));box.append(node('p',r.cost.attempts_with_cost?r.cost.reported_usd.toFixed(2)+' USD':trMissionInsights('Coût non rapporté.')))}
   host.append(box);
  }return host;
 },
 spending(){this.readOnly(trMissionInsights('Où vont les appels et les coûts ?'),trMissionInsights('Appels IA, outils et contrôles sont présentés séparément.')).append(this.spendingView(snapshot.mission.spending))},
 recoveryPreviewView(p){
  const box=node('section',undefined,'field-wide');box.dataset.recoveryPreview=p.task;box.append(node('h3',trMissionInsights('Avant une relance')));
  box.append(node('h4',trMissionInsights('Depuis le refus')),node('p',missionText(p.refusal_note||'Historique du refus indisponible ; aucun changement n’est supposé.')));
  if(p.since_refusal){const list=node('ul');for(const item of p.since_refusal.items||[])list.append(node('li','r'+item.revision+' · '+item.at+' · '+missionText(item.label)));box.append(list);if(p.since_refusal.more)box.append(node('p',trMissionInsights('Historique partiel : d’autres événements restent à examiner.'),'notice attention'))}
  box.append(node('h4',trMissionInsights('Preuves à reprendre')),node('p',missionText(p.evidence_note||'Aucune preuve liée à un avis disponible ; réutilisation non démontrée.')));
  if(!p.evidence?.length)box.append(node('p',trMissionInsights('Aucune preuve liée à un avis disponible ; réutilisation non démontrée.')));
  const retained=node('details');retained.append(node('summary',trMissionInsights('Voir les preuves inchangées')));let count=0;
  for(const item of p.evidence||[]){const row=node('article',undefined,'mission-current');row.dataset.recoveryEvidence=item.state;row.append(node('strong',item.report||trMissionInsights('Autres preuves')),node('p',missionText(item.reason),'notice '+(item.state==='unchanged'?'info':'attention')));if(item.state==='unchanged'){retained.append(row);count++}else box.append(row)}
  if(count)box.append(retained)
  box.append(node('h4',trMissionInsights('Critères restant à vérifier')));const remaining=node('ul');for(const item of p.remaining_criteria||p.criteria||[])remaining.append(node('li',item));box.append(remaining);
  if(p.remaining_criteria?.length===0)box.append(node('p',trMissionInsights('Tous les critères ont un avis favorable sur ces entrées inchangées ; les contrôles et la décision restent requis.')));
  for(const [label,items]of [['Ce qui sera conservé',p.kept],['Ce qui sera refait',p.redone],['Critères inchangés',p.criteria]]){box.append(node('h4',trMissionInsights(label)));const list=node('ul');for(const item of items||[])list.append(node('li',label==='Critères inchangés'?item:missionText(item)));box.append(list)}
  box.append(node('h4',trMissionInsights('Correction attendue')),node('p',p.correction||trMissionInsights('À préciser avant confirmation.')),node('p',missionText(p.limits),'notice info'));return box;
 },
 async recoveryPreview(id){const requested=work,host=this.readOnly(trMissionInsights('Avant une relance'),trMissionInsights('Relisez la correction et les critères ; aucune action n’est exécutée ici.')),context=modalContext;host.append(node('p',trMissionInsights('Chargement de l’aperçu de reprise…')));try{const p=await api('/api/v1/recovery-preview?'+new URLSearchParams({work:requested,task:id}));if(modalContext===context&&work===requested&&host.isConnected)host.replaceChildren(this.recoveryPreviewView(p))}catch(e){if(modalContext===context&&host.isConnected)host.replaceChildren(node('p',trMissionInsights('Aperçu de reprise indisponible : ')+e.message,'notice attention'))}},
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
