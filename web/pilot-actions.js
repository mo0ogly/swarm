'use strict';
const tr_web_pilot_actions_js = source => globalThis.SwarmI18n?.t(source) ?? source;

const PilotActions={
 requests:new Map(),inflight:new Map(),
 preparedFields(d){
  const p=(d.actions||[]).find(a=>a.kind==='resume-launch')?.prepared_launch;
  if(!p){$('confirm').disabled=true;return}
  modalContext.prepared=p;
  $('modal-description').textContent=tr_web_pilot_actions_js('Le lancement a été préparé, puis interrompu avant la création de l’agent. Reprendre conserve la copie et les réglages ; les conditions seront vérifiées à nouveau.');
  const box=node('section',undefined,'notice info field-wide');box.id='prepared-launch-summary';
  box.append(node('p',tr_web_pilot_actions_js('Fournisseur')+' : '+p.provider),node('p',tr_web_pilot_actions_js('Espace de travail')+' : '+p.workspace),node('p',tr_web_pilot_actions_js('Consigne pour l’agent')+' : '+p.instruction));
  box.append(node('p',tr_web_pilot_actions_js('Durée maximale : ')+(p.timeout_seconds||1800)+' s'),node('p',tr_web_pilot_actions_js('Les plafonds de tentatives et d’outils sont conservés. Aucun résultat n’est validé par cette reprise.')));
  $('modal-fields').append(box);$('confirm').textContent=tr_web_pilot_actions_js('Reprendre le lancement préparé');
 },
 extensionFields(d){
  const t=d.task,limit=t.plan_max_attempts,toolCaps=[t.plan_tool_limit,t.launch_profile?.limits?.max_tool_calls||snapshot.work.launch_profile?.limits?.max_tool_calls].filter(n=>n>0),tools=toolCaps.length?Math.min(...toolCaps):0;
  $('modal-title').textContent=tr_web_pilot_actions_js('Autoriser une tentative supplémentaire')+' — '+t.title;
  $('modal-description').textContent=tr_web_pilot_actions_js('Le plafond est atteint. Vous pouvez ajouter une seule tentative après avoir précisé ce qui sera corrigé.');
  const summary=node('section',undefined,'notice attention field-wide');summary.id='attempt-extension-summary';
  summary.append(node('p',tr_web_pilot_actions_js('Tentatives consommées : ')+t.attempts.length+' / '+limit+' → '+(limit+1)),node('p',tr_web_pilot_actions_js('Limite d’outils conservée : ')+(tools||tr_web_pilot_actions_js('voir les limites du profil au lancement'))),node('p',tr_web_pilot_actions_js('Historique, rapports et avis conservés. La tâche reste non validée. Les contrôles et la revue indépendante restent obligatoires selon sa politique.')));
  $('modal-fields').append(summary);
  const reason=field('reason',tr_web_pilot_actions_js('Pourquoi autoriser cette reprise ?'),'',null,true);reason.required=true;reason.minLength=8;reason.maxLength=2000;
  const correction=field('recovery_instruction',tr_web_pilot_actions_js('Ce que l’agent doit corriger avant de refaire les contrôles'),'',null,true,tr_web_pilot_actions_js('Décrivez la correction ou la précondition vérifiée. Cette consigne remplace la prochaine action ; les critères de réussite restent inchangés.'));correction.required=true;correction.minLength=20;correction.maxLength=16000;
  $('confirm').textContent=tr_web_pilot_actions_js('Autoriser et préparer la reprise');
  preview(tr_web_pilot_actions_js('Cette confirmation ajoute une tentative au plafond. Elle ne lance aucun agent. Le formulaire de lancement s’ouvrira ensuite pour vérifier les conditions.'));
 },
 async extensionDialog(task){await taskDialog(task);if(modalContext?.task===task&&(modalContext.data.actions||[]).some(a=>a.kind==='extend-attempt'&&a.disponible))taskFields('extend-attempt')},
 async recoveryDialog(task){await taskDialog(task);if(modalContext?.task===task&&(modalContext.data.actions||[]).some(a=>a.kind==='authorize-recovery'&&a.disponible))taskFields('authorize-recovery')},
 recoveryFields(d){
  const t=d.task,r=t.independent_review;
  $('modal-title').textContent=tr_web_pilot_actions_js('Préparer un essai correctif')+' — '+t.title;
  $('modal-description').textContent=tr_web_pilot_actions_js('Relisez la correction proposée à partir du refus. Votre confirmation autorise un seul essai supplémentaire pour cette tâche.');
  const summary=node('section',undefined,'notice attention field-wide');summary.id='corrective-recovery-summary';
  summary.append(node('p',tr_web_pilot_actions_js('Tentatives consommées : ')+t.attempts.length+' / 3 → 4'),node('p',tr_web_pilot_actions_js('Une seule autorisation exceptionnelle. Historique et critères conservés ; aucune validation sans contrôles et nouvel avis indépendant.')),node('p',r.reason));
  const cfg=snapshot.work.planning?.reviewer;if(cfg)summary.append(node('p',tr_web_pilot_actions_js('Vérificateur indépendant')+' : '+cfg.provider+' · '+cfg.calls+' / '+cfg.max_calls));
  $('modal-fields').append(summary);
  const reason=field('reason',tr_web_pilot_actions_js('Pourquoi autoriser cette reprise ?'),tr_web_pilot_actions_js('Corriger les écarts du dernier avis indépendant en conservant les critères et les preuves.'),null,true);reason.required=true;reason.minLength=8;reason.maxLength=2000;
  const proposal=[tr_web_pilot_actions_js('Corriger les écarts ci-dessous, vérifier tous les critères inchangés et fournir les commandes exécutées, leurs résultats et les preuves du candidat corrigé.'),r.reason,...(r.criteria||[]).filter(c=>c.verdict!=='pass').map(c=>tr_web_pilot_actions_js('Critère ')+c.index+' : '+c.evidence)].join('\n\n');
  const correction=field('recovery_instruction',tr_web_pilot_actions_js('Correction proposée — à relire'),proposal,null,true,tr_web_pilot_actions_js('Cette proposition reprend le refus ; elle ne prouve pas que le diagnostic est complet. Précisez les corrections nécessaires avant de confirmer.'));correction.required=true;correction.minLength=20;correction.maxLength=16000;
  $('confirm').textContent=tr_web_pilot_actions_js('Autoriser cet essai correctif');
  preview(tr_web_pilot_actions_js('Si la mission est active, Swarm lancera cet essai dès que les dépendances, le budget, le stockage et le vérificateur le permettent. Une mission en pause reste en pause. Aucun cinquième essai ne sera autorisé par ce parcours.'));
 },
 async authorizeRecovery(c,f){
  c.recoveryEvent ||= crypto.randomUUID();const r=c.data.task.independent_review;
  await api('/api/v1/planning?'+new URLSearchParams({work:c.workID,action:'authorize-recovery'}),{schema_version:1,event_id:c.recoveryEvent,expected_revision:c.revision,task_id:c.task,review_id:r.id,attempt_id:r.attempt,confirm_recovery:true,reason:f.reason,recovery_instruction:f.recovery_instruction});
  if(modalContext!==c||work!==c.workID)return;
  closeModal();await refresh(true);notice(tr_web_pilot_actions_js('Essai correctif autorisé. La mission le prendra en charge selon ses conditions de lancement ; le résultat reste à vérifier.'));
 },
 extensionButton(task,scope='priority'){
  const recovery=snapshot.task_actions?.[task]?.find(a=>a.kind==='authorize-recovery');
  if(recovery){const b=Pilot.command(tr_web_pilot_actions_js('Préparer un essai correctif'),()=>this.recoveryDialog(task));b.dataset.missionAction='recovery-'+scope+'-'+task;b.dataset.correctiveRecovery=task;b.disabled=!recovery.disponible;if(recovery.raison)b.title=globalThis.SwarmI18n?.engine(recovery.raison)||recovery.raison;return b}
  const option=snapshot.task_actions?.[task]?.find(a=>a.kind==='extend-attempt');if(!option)return null;
  const b=Pilot.command(tr_web_pilot_actions_js('Autoriser une tentative supplémentaire'),()=>this.extensionDialog(task));b.id='attempt-extension-'+scope+'-'+encodeURIComponent(task);b.dataset.missionAction='extend-'+scope+'-'+task;b.dataset.extendAttempt=task;b.disabled=!option.disponible;if(option.raison)b.title=globalThis.SwarmI18n?.engine(option.raison)||option.raison;return b;
 },
 async extendAttempt(c,f){
  c.extensionEvent ||= crypto.randomUUID();
  await api('/api/v1/planning?'+new URLSearchParams({work:c.workID,action:'extend-attempt'}),{schema_version:1,event_id:c.extensionEvent,expected_revision:c.revision,task_id:c.task,reason:f.reason,recovery_instruction:f.recovery_instruction});
  if(modalContext!==c||work!==c.workID)return;
  closeModal();await refresh(true);
  notice(tr_web_pilot_actions_js('Tentative supplémentaire autorisée. Préparez son lancement ; aucun agent n’a encore été lancé.'));
  if(work===c.workID)await Pilot.go(c.task);
 },
 async send(kind,fields){
  const data={kind,work,expected_revision:modalContext?.revision??snapshot.work.revision,...fields};
  const signature=JSON.stringify(data);let event=this.requests.get(signature);
  if(!event){event=crypto.randomUUID();this.requests.set(signature,event)}
  if(this.inflight.has(signature))return this.inflight.get(signature);
  const request=(async()=>{try{const result=await api('/api/v1/action',{...data,event_id:event});this.requests.delete(signature);return result}
   catch(e){if(e.code==='agent_command_refused')this.requests.delete(signature);if(work===data.work){await refresh(true);notice(tr_web_pilot_actions_js('Réception non confirmée ou demande refusée : ')+e.message+'. État courant relu ; vérifiez-le avant de confirmer une nouvelle action.',true)}throw e}
   finally{this.inflight.delete(signature)}})();
  this.inflight.set(signature,request);return request;
 },
 outcome(action){
  return {'resume-launch':tr_web_pilot_actions_js('Lancement repris avec la copie conservée ; démarrage du fournisseur à confirmer.'),stop:tr_web_pilot_actions_js('Arrêt demandé — confirmation du superviseur attendue.'),start:tr_web_pilot_actions_js('Tentative créée — démarrage du fournisseur à confirmer.'),retry:tr_web_pilot_actions_js('Nouvelle tentative créée — démarrage à confirmer.'),submit:tr_web_pilot_actions_js('Rapport soumis pour examen ; tâche non acceptée.'),accepted:tr_web_pilot_actions_js('Acceptation enregistrée après revue.'),decision:tr_web_pilot_actions_js('Décision enregistrée ; l’état du processus reste distinct.'),reconcile:tr_web_pilot_actions_js('Réconciliation effectuée ; consulter le nouvel état.')}[action]||tr_web_pilot_actions_js('Action enregistrée. Le nouvel état est consultable dans le détail.');
 },
 addPreview(){
  if(modalContext?.action!=='start')return;
  const context=modalContext,host=node('section',undefined,'notice info');host.id='launch-guidance';
  const status=node('p',tr_web_pilot_actions_js('Vérification des conditions…'));status.id='launch-eligibility';status.setAttribute('role','status');
  const actions=node('div');actions.id='launch-resolution';
  const check=Pilot.command(tr_web_pilot_actions_js('Actualiser les conditions'),()=>this.preview(context).catch(e=>notice(e.message,true)));
  host.append(status,actions,check);$('modal-fields').prepend(host);
  const advanced=node('details',undefined,'launch-options');advanced.append(node('summary',tr_web_pilot_actions_js('Options avancées — mode, rôle et journal')));
  for(const name of ['mode','role','capture']){const field=$('field-'+name);if(field)advanced.append(field.parentElement)}
  $('modal-fields').append(advanced);
  let timer;
  const checkLater=()=>{clearTimeout(timer);timer=setTimeout(async()=>{
   if(modalContext!==context||context.action!=='start'||!host.isConnected)return;
   if(context.modelReady===false){checkLater();return}
   const expected=(context.previewGeneration||0)+1;
   try{await this.preview(context)}catch(e){if(modalContext===context&&context.previewGeneration===expected&&host.isConnected)status.textContent=tr_web_pilot_actions_js('Vérification indisponible : ')+e.message}
  },400)};
  for(const el of $('modal-fields').querySelectorAll('input,select,textarea'))el.addEventListener('input',()=>{context.previewGeneration=(context.previewGeneration||0)+1;actions.replaceChildren();$('confirm').hidden=false;status.textContent=tr_web_pilot_actions_js('Réglages modifiés : vérification en cours…');checkLater()});
  checkLater();
 },
 async preview(context,fields=Object.fromEntries(new FormData($('action-form')))){
  const generation=context.previewGeneration=(context.previewGeneration||0)+1;
  const signature=JSON.stringify(fields);
  const coordinates={mode:fields.mode,task:context.task,provider:fields.provider,workspace:fields.workspace,role:fields.role||'worker',instruction:fields.instruction,level:fields.level,model_policy_hash:fields.model_policy_hash};
  const requested=work;
  const current=await api('/api/v1/task?'+new URLSearchParams({work:requested,task:context.task}));
  if(modalContext!==context||context.action!=='start'||work!==requested||context.previewGeneration!==generation)return false;
  const result=await api('/api/v1/action',{kind:'launch-preview',work:requested,expected_revision:current.revision,...coordinates});
  if(modalContext!==context||context.action!=='start'||work!==requested||context.previewGeneration!==generation||JSON.stringify(Object.fromEntries(new FormData($('action-form'))))!==signature)return false;
  context.revision=current.revision;
  const state=$('launch-eligibility'),actions=$('launch-resolution');
  if(state){state.textContent=result.reason_label;$('launch-guidance').className='notice '+(result.eligible?'success':'attention')}
  actions?.replaceChildren();$('confirm').hidden=result.reason_code==='workspace_busy';
  if(result.reason_code==='workspace_busy'&&actions){
   $('modal-description').textContent=tr_web_pilot_actions_js('Cette tâche attend un espace disponible. Une autre tâche travaille déjà dans ce répertoire.');
   actions.append(node('p',result.next_label));
   const b=result.blocker;
   if(b)actions.append(Pilot.command(tr_web_pilot_actions_js('Suivre la tâche active — ')+b.title,async()=>{
    closeModal();if(work!==b.work_id){$('work').value=b.work_id;await chooseWork()}
    if(work===b.work_id)Pilot.inspect('task',b.task_id);
   }));
  }else if(result.eligible){$('modal-description').textContent=tr_web_pilot_actions_js('Cette tâche peut démarrer. Vérifiez la consigne, puis choisissez « Lancer cette tâche ».');$('confirm').textContent=tr_web_pilot_actions_js('Lancer cette tâche')}
  return result.eligible===true;
 }
};
