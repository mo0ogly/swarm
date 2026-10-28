'use strict';
const trAutomation=source=>globalThis.SwarmI18n?.t(source)??source;
let automationPreview=null,automationConfig=null,automationLoading=false;
const automationCodeLabels={
 disabled:'Désactivé',enabled:'Activé',paused:'Suspendu',archived:'Archivé',
 once:'Une fois',daily:'Chaque jour',weekly:'Chaque semaine',
 received:'Reçue',waiting:'En attente',claimed:'Prise en charge',executed:'Exécutée',rejected:'Refusée',uncertain_effect:'Effet incertain',cancelled:'Annulée',
 not_validated:'Non validée',validated_closed:'Validée et clôturée',
 enable_when_authorized:'Activer après autorisation',preview_then_enable:'Prévisualiser puis activer',
 unknown:'Inconnu',planning:'Planification',authorization_required:'Autorisation requise',cancelled_by_operator:'Annulée par l’opérateur'
};
const automationLabel=value=>trAutomation(automationCodeLabels[value]||value||'Inconnu');
function automationBadge(state){const value=badge(state);value.textContent=automationLabel(state);return value}
const automationConfigFields=[
 ['claim_lease_seconds','Durée du bail de prise en charge (secondes)'],
 ['due_grace_seconds','Grâce d’échéance (secondes)'],
 ['max_preview_occurrences','Occurrences maximales dans un aperçu'],
 ['max_schedule_occurrences','Occurrences maximales par programme'],
 ['external_max_payload_bytes','Taille maximale d’un événement externe (octets)'],
 ['external_rate_limit','Événements externes par fenêtre'],
 ['external_rate_window_seconds','Fenêtre de fréquence externe (secondes)'],
 ['external_timestamp_window_seconds','Fenêtre de signature externe (secondes)']
];
function automationRequest(form){
 const values=Object.fromEntries(new FormData(form));const count=Number(values.count);const localTime=value=>value.length===16?value+':00':value;
 return {schema_version:1,event_id:'program-'+crypto.randomUUID(),name:values.name.trim(),target_work_id:values.target.trim(),action:'request_resume',timezone:values.timezone.trim(),schedule:{kind:values.kind,local_time:localTime(values.start),until_local:values.kind==='once'?'':localTime(values.until),max_occurrences:count},missed_policy:'skip',concurrency_policy:'coalesce'};
}
function invalidateAutomationPreview(){automationPreview=null;$('automation-create-confirm').disabled=true;$('automation-preview-output').hidden=true}
function automationCost(target){
 if(target.cost_state==='unknown')return trAutomation('Coût réel : inconnu (aucune valeur fournisseur rapportée)');
 return trAutomation('Coût rapporté : ')+target.cost.reported_usd+' USD · '+target.cost.attempts_without_cost+' '+trAutomation('tentative(s) sans coût rapporté');
}
function automationTargetText(target){
 const profile=target.profile||{},limits=target.limits||{},budget=target.budget||{};
 return [trAutomation('Mission : ')+target.work_id,trAutomation('Validation de mission : ')+automationLabel(target.mission_validation_state),trAutomation('Autorisation actuelle : ')+(target.authorized?trAutomation('accordée'):trAutomation('refusée — ')+(target.authorization_reason?automationLabel(target.authorization_reason):trAutomation('raison indisponible'))),trAutomation('Profil : ')+(profile.provider==='unknown'?automationLabel(profile.provider):(profile.provider||trAutomation('inconnu')))+' · '+automationLabel(profile.role),trAutomation('Limites restantes : activations ')+(limits.remaining_activations??trAutomation('inconnu'))+' · '+trAutomation('décisions ')+(limits.remaining_decisions??trAutomation('inconnu')),automationCost(target),trAutomation('Budget disponible estimé : ')+(budget.remaining_usd??trAutomation('inconnu')),trAutomation('Prochaine action : ')+automationLabel(target.next_action)].join('\n');
}
async function automationTransition(view,state){
 try{await api('/api/v1/automation/state',{schedule_id:view.schedule.schedule_id,state,expected_revision:view.schedule.revision});notice(trAutomation('Programme mis à jour.'));await loadAutomationAdmin()}
 catch(error){$('automation-state').className='notice alert';$('automation-state').textContent=error.message}
}
async function automationCancel(occurrence){
 try{await api('/api/v1/automation/cancel',{occurrence_id:occurrence.occurrence_id,expected_revision:occurrence.revision});notice(trAutomation('Occurrence en attente annulée.'));await loadAutomationAdmin()}
 catch(error){$('automation-state').className='notice alert';$('automation-state').textContent=error.message}
}
function renderAutomationPrograms(programs){
 const host=$('automation-list');host.replaceChildren();
 if(!programs.length){host.append(node('p',trAutomation('Aucun programme enregistré.'),'notice info'));return}
 for(const view of programs){
  const schedule=view.schedule,card=node('article',undefined,'card');card.dataset.schedule=schedule.schedule_id;
  card.append(node('h3',schedule.name),automationBadge(schedule.state),node('p',schedule.schedule_id+' · '+automationLabel(schedule.kind)+' · '+schedule.timezone),node('p',trAutomation('Prochaine occurrence : ')+(schedule.next_at||trAutomation('aucune'))),node('pre',automationTargetText(view.target)));
  const actions=node('div',undefined,'toolbar');
  if(['disabled','paused'].includes(schedule.state)){const enable=button(trAutomation('Activer'),()=>automationTransition(view,'enabled'));enable.disabled=!view.target.authorized;actions.append(enable)}
  if(schedule.state==='enabled')actions.append(button(trAutomation('Suspendre'),()=>automationTransition(view,'paused')));
  if(schedule.state!=='archived')actions.append(button(trAutomation('Archiver'),()=>automationTransition(view,'archived')));
  card.append(actions,node('h4',trAutomation('Occurrences')));
  if(!view.occurrences.length)card.append(node('p',trAutomation('Aucune occurrence traitée.')));
  for(const occurrence of view.occurrences){const row=node('div',undefined,'card');row.append(node('p',occurrence.occurrence_id+' · '+automationLabel(occurrence.state)),node('p',trAutomation('Demande : ')+occurrence.request_id+' · '+trAutomation('Mission : ')+automationLabel(occurrence.mission_validation_state)));if(['received','waiting'].includes(occurrence.state))row.append(button(trAutomation('Annuler cette attente'),()=>automationCancel(occurrence)));card.append(row)}
  host.append(card);
 }
}
function renderAutomationConfig(){
 const form=$('automation-config');form.replaceChildren();
 for(const [key,label] of automationConfigFields){const wrapper=node('label',trAutomation(label));const input=node('input');input.type='number';input.min='0';input.step='1';input.name=key;input.value=automationConfig.values[key];wrapper.append(input);form.append(wrapper)}
 const submit=button(trAutomation('Enregistrer une nouvelle révision'));submit.type='submit';submit.className='primary';form.append(submit,node('p',trAutomation('Révision actuelle : ')+automationConfig.revision,'assist-meta'));
}
async function loadAutomationAdmin(){
 if(automationLoading)return;automationLoading=true;$('automation-state').className='notice info';$('automation-state').textContent=trAutomation('Chargement des programmes…');
 try{const [programs,config]=await Promise.all([api('/api/v1/automation/programs'),api('/api/v1/automation/config')]);automationConfig=config;renderAutomationPrograms(programs);renderAutomationConfig();$('automation-state').textContent=trAutomation('Programmes chargés. Une occurrence traitée reste distincte de la validation de sa mission.')}
 catch(error){$('automation-state').className='notice alert';$('automation-state').textContent=error.message}
 finally{automationLoading=false}
}
$('automation-create').addEventListener('input',invalidateAutomationPreview);
$('automation-create').addEventListener('submit',async event=>{event.preventDefault();try{const request=automationRequest(event.currentTarget);const previewValue=await api('/api/v1/automation/preview?count=5',request);automationPreview={request,value:previewValue};$('automation-preview-output').textContent=previewValue.occurrences_utc.join('\n')+'\n\n'+automationTargetText(previewValue.target)+'\n'+trAutomation('Effet de la confirmation : création désactivée, aucun départ implicite.');$('automation-preview-output').hidden=false;$('automation-create-confirm').disabled=false;$('automation-preview-output').focus()}catch(error){invalidateAutomationPreview();$('automation-state').className='notice alert';$('automation-state').textContent=error.message}});
$('automation-create-confirm').addEventListener('click',async()=>{if(!automationPreview)return;try{await api('/api/v1/automation/create',{schedule:automationPreview.request,preview_token:automationPreview.value.preview_token});invalidateAutomationPreview();$('automation-create').reset();document.querySelector('#automation-create [name=timezone]').value='UTC';document.querySelector('#automation-create [name=count]').value='1';notice(trAutomation('Programme créé désactivé. Activez-le explicitement après vérification.'));await loadAutomationAdmin()}catch(error){invalidateAutomationPreview();$('automation-state').className='notice alert';$('automation-state').textContent=error.message}});
$('automation-config').addEventListener('submit',async event=>{event.preventDefault();try{const values={};for(const [key]of automationConfigFields)values[key]=Number(new FormData(event.currentTarget).get(key));automationConfig=await api('/api/v1/automation/config',{schema_version:1,expected_revision:automationConfig.revision,values});renderAutomationConfig();notice(trAutomation('Réglages opérationnels enregistrés dans une nouvelle révision.'))}catch(error){$('automation-state').className='notice alert';$('automation-state').textContent=error.message}});
$('automation-help').addEventListener('click',()=>{openModal(trAutomation('Aide des programmes'),trAutomation('Prévisualisez avant de créer. La création reste désactivée ; activer revalide les autorisations et budgets. Suspendre empêche de nouvelles occurrences. Archiver est définitif. Annuler est limité à une occurrence encore en attente, avant prise en charge ou effet.'),{action:'help'});$('confirm').hidden=true;$('cancel').textContent=trAutomation('Fermer')});
if(view==='automation')loadAutomationAdmin();
