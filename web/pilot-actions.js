'use strict';
const tr_web_pilot_actions_js = source => globalThis.SwarmI18n?.t(source) ?? source;

const PilotActions={
 requests:new Map(),inflight:new Map(),
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
  return {stop:tr_web_pilot_actions_js('Arrêt demandé — confirmation du superviseur attendue.'),start:tr_web_pilot_actions_js('Tentative créée — démarrage du fournisseur à confirmer.'),retry:tr_web_pilot_actions_js('Nouvelle tentative créée — démarrage à confirmer.'),submit:tr_web_pilot_actions_js('Rapport soumis pour examen ; tâche non acceptée.'),accepted:tr_web_pilot_actions_js('Acceptation enregistrée après revue.'),decision:tr_web_pilot_actions_js('Décision enregistrée ; l’état du processus reste distinct.'),reconcile:tr_web_pilot_actions_js('Réconciliation effectuée ; consulter le nouvel état.')}[action]||tr_web_pilot_actions_js('Action enregistrée. Le nouvel état est consultable dans le détail.');
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
