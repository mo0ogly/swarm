'use strict';
let providerAdmin=null,providerLoading=false;
const levelOptions=[['auto','Automatique — politique du fournisseur'],['simple','Simple — opération ciblée, reformulation, aide'],['standard','Standard — développement et tests usuels'],['exigeant','Exigeant — architecture ou analyse difficile, choix explicite']];
function routeText(r){return r?`${r.level} · ${r.model}${r.effort?' · effort '+r.effort:''} · ${r.billing==='on_premise'?'sur site, sans facture API déclarée':'fournisseur externe'}`:'Modèle géré par cet exécutable ; adaptateur de sélection indisponible.'}
function addModelFields(purpose,fixedProvider='',initial='auto'){
 const c=modalContext;if(!c)return;
 const level=field('level','Niveau de l’agent',initial,levelOptions);
 const hidden=field('model_policy_hash','Empreinte de la politique','');hidden.type='hidden';hidden.parentElement.hidden=true;
 const info=node('p','Résolution du modèle…','notice info');info.id='model-selection';$('modal-fields').append(info);
 let generation=0;
 const update=async()=>{const n=++generation;hidden.value='';c.modelReady=false;$('confirm').disabled=true;info.className='notice info';info.textContent='Résolution du modèle…';
  try{const provider=typeof fixedProvider==='function'?fixedProvider():fixedProvider||$('field-provider')?.value;if(!provider)throw Error('Choisir un fournisseur.');const data=await api('/api/v1/providers/resolve?'+new URLSearchParams({provider,level:level.value,purpose}));if(modalContext!==c||n!==generation||!level.isConnected)return;hidden.value=data.route?.policy_hash||'';c.modelReady=true;info.textContent=routeText(data.route);$('confirm').disabled=false;
  }catch(e){if(modalContext!==c||n!==generation||!level.isConnected)return;info.className='notice alert';info.textContent=e.message;$('confirm').disabled=true}
 };
 if(typeof fixedProvider==='function'&&$('field-agent'))$('field-agent').addEventListener('change',update);if(purpose==='planning'&&$('field-planning-provider'))$('field-planning-provider').addEventListener('change',update);level.onchange=update;if(!fixedProvider&&$('field-provider'))$('field-provider').addEventListener('change',update);update();
}
async function loadProviderAdmin(){
 if(providerLoading)return;providerLoading=true;void AIConnections.load();$('providers-state').textContent='Chargement des fournisseurs et des modèles…';
 try{providerAdmin=await api('/api/v1/providers/admin');renderProviderAdmin();$('providers-state').className='notice info';$('providers-state').textContent='Politique locale : aucun appel IA au chargement. Un modèle exigeant demande un choix explicite ; aucun repli automatique.'}
 catch(e){$('providers-state').className='notice alert';$('providers-state').textContent=e.message}
 finally{providerLoading=false}
}
function renderProviderAdmin(){
 const host=$('providers-list');host.replaceChildren();
 for(const [id,p]of Object.entries(providerAdmin.providers)){
  const card=node('article',undefined,'card provider-card');card.dataset.provider=id;
  card.append(node('h3',id),node('p',p.adapter+' · '+p.health,'assist-meta'));
  const policy=p.policy;
  if(!policy){card.append(node('p','Adaptateur de sélection non configuré pour cet exécutable.','notice attention'));host.append(card);continue}
  const meta=node('p',policy.billing==='on_premise'?'Sur site : absence de facture API déclarée ; durée et ressources restent bornées.':'Fournisseur externe : niveau adapté à la tâche, sans estimation de prix inventée.','notice '+(policy.billing==='on_premise'?'info':'neutral'));card.append(meta);
  card.append(node('p','Défauts : travaux '+policy.work_level+' · aide de page '+policy.page_level+(p.saved?' · politique enregistrée':' · politique proposée par le moteur')));
  const table=node('table'),head=node('thead'),hr=node('tr');for(const label of ['Niveau','Modèle','Effort'])hr.append(node('th',label));head.append(hr);table.append(head);const body=node('tbody');
  for(const level of ['simple','standard','exigeant']){const row=node('tr');row.append(node('td',level),node('td',policy.levels[level]?.model||'À configurer'),node('td',policy.levels[level]?.effort||'Défaut du modèle'));body.append(row)}table.append(body);card.append(table);
  const actions=node('div',undefined,'provider-actions');actions.append(button('Configurer les niveaux',()=>editProviderPolicy(id)));
  const test=button('Tester avec une question',async()=>{if(!snapshot||snapshot.work.id!==work)await refresh(true);if(!snapshot||snapshot.work.id!==work){notice('Sélectionnez un travail chargé pour y conserver la question de test.',true);return}try{if(!assistMeta)await loadAssistMeta()}catch(e){notice('Assistant indisponible : '+e.message,true);return}showView('tasks');openAssistAsk();if(!$('field-provider'))return;$('field-provider').value=id;$('field-provider').dispatchEvent(new Event('change'));$('modal-description').textContent='Test explicite du fournisseur : choisissez le niveau, examinez le contexte et confirmez l’appel. Le résultat et l’usage déclaré seront conservés dans l’historique.'});test.disabled=Boolean(p.assistant_unavailable);if(p.assistant_unavailable)test.title=p.assistant_unavailable;actions.append(test);card.append(actions);
  if(p.assistant_unavailable)card.append(node('p',p.assistant_unavailable,'notice attention'));
  const details=node('details');details.append(node('summary','Connexion et provenance du catalogue'),node('p',p.command,'assist-meta'),node('p','Authentification conservée par la CLI. Les secrets ne sont ni importés ni exportés.'));
  for(const m of p.models)details.append(node('p',m.id+' — '+m.source,'assist-meta'));card.append(details);host.append(card);
 }
}
function editProviderPolicy(id){
 const p=providerAdmin.providers[id];openModal('Fournisseur — '+id,'Choisir le modèle adapté à chaque niveau. La sauvegarde affecte les prochains lancements ; elle ne change pas un agent déjà démarré.',{action:'provider-policy',provider:id,digest:providerAdmin.digest});
 field('billing','Facturation déclarée',p.policy.billing,[['external','Fournisseur externe'],['on_premise','Sur site — pas de facture API marginale']]);
 field('work_level','Niveau automatique des travaux',p.policy.work_level,levelOptions.slice(1,3));field('page_level','Niveau automatique de l’aide',p.policy.page_level,levelOptions.slice(1,3));
 for(const level of ['simple','standard','exigeant']){
  const group=node('section',undefined,'provider-level');group.append(node('h3','Niveau '+level));$('modal-fields').append(group);
  const model=field('model_'+level,'Modèle — '+level,p.policy.levels[level].model,p.models.map(m=>[m.id,m.id]));
  const choices=()=>[['','Défaut du modèle'],...(p.models.find(m=>m.id===model.value)?.efforts||[]).map(e=>[e,e])];const effort=field('effort_'+level,'Effort — '+level,p.policy.levels[level].effort,choices());model.onchange=()=>selectOptions(effort,choices(),'');group.append(model.parentElement,effort.parentElement);
 }
 $('confirm').textContent='Examiner la politique';
}
async function submitProviderPolicy(f,c){
 let policies;
 if(c.action==='provider-import'){
  let d;try{d=JSON.parse(f.document)}catch{throw Error('JSON invalide : utiliser un export de politiques version 1.');}
  if(d.version!==1||!d.policies||Object.keys(d).some(k=>!['version','policies'].includes(k)))throw Error('Import limité à version et policies. Les connexions, clés et commandes ne sont pas importées.');policies=d.policies;
 }else{const levels={};for(const level of ['simple','standard','exigeant'])levels[level]={model:f['model_'+level],effort:f['effort_'+level]};policies={[c.provider]:{version:1,billing:f.billing,work_level:f.work_level,page_level:f.page_level,levels}}}
 const request={version:1,expected_digest:c.digest,policies},signature=JSON.stringify(request);
 if(c.signature!==signature){const data=await api('/api/v1/providers/policies',{...request,preview:true});if(modalContext!==c)return false;c.signature=signature;preview('POLITIQUE À ENREGISTRER\n'+Object.entries(data.policies).map(([id,p])=>id+' · '+(p.billing==='on_premise'?'sur site':'externe')+'\nDéfauts : travaux '+p.work_level+' · aide '+p.page_level+'\n'+Object.entries(p.levels).map(([level,m])=>'  '+level+' → '+m.model+(m.effort?' · effort '+m.effort:'')).join('\n')).join('\n\n')+'\n\nAucun agent lancé. Aucune commande ni clé modifiée.');$('confirm').textContent='Confirmer l’enregistrement';return false}
 const result=await api('/api/v1/providers/policies',{...request,preview:false});if(!result.applied)throw Error('Enregistrement non confirmé.');notice(result.audit_warning||'Politique enregistrée. Les prochains lancements utiliseront ces niveaux.',Boolean(result.audit_warning));await loadProviderAdmin();return true;
}
$('providers-refresh').onclick=loadProviderAdmin;
$('providers-export').onclick=()=>{if(!providerAdmin)return;const url=URL.createObjectURL(new Blob([JSON.stringify(providerAdmin.export,null,2)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download='swarm-model-policies.v1.json';a.click();URL.revokeObjectURL(url)};
$('providers-import').onclick=()=>{if(!providerAdmin)return;openModal('Importer des politiques de modèles','Contrat portable version 1. Les fournisseurs doivent déjà être configurés sur cette machine. Aucun secret ni commande dans ce document.',{action:'provider-import',digest:providerAdmin.digest});field('document','Document JSON','',null,true);$('confirm').textContent='Examiner l’import'};

if(view==='providers')loadProviderAdmin();
