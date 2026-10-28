'use strict';
const tr_web_providers_js = source => globalThis.SwarmI18n?.t(source) ?? source;

let providerAdmin=null,providerLoading=false;
const levelOptions=[['auto',tr_web_providers_js('Automatique — politique du fournisseur')],['simple',tr_web_providers_js('Simple — opération ciblée, reformulation, aide')],['standard',tr_web_providers_js('Standard — développement et tests usuels')],['exigeant',tr_web_providers_js('Exigeant — architecture ou analyse difficile, choix explicite')]];
/** Render a schema-level label without changing its wire value or granting a
 * policy choice. Unknown metadata stays text; no HTML or model output is parsed. */
function providerLevelText(level){const names={simple:'Simple',standard:'Standard',exigeant:'Exigeant'};return tr_web_providers_js(Object.hasOwn(names,level)?names[level]:String(level))}
/** Describe a retained requested route or absent adapter; no availability,
 * account, effective model, billing or execution authority is established. */
function routeText(r){return r?`${providerLevelText(r.level)} · ${r.model}${r.effort?tr_web_providers_js(' · effort ')+r.effort:''} · ${r.billing==='on_premise'?tr_web_providers_js('sur site, sans facture API déclarée'):tr_web_providers_js('fournisseur externe')}`:tr_web_providers_js('Modèle géré par cet exécutable ; adaptateur de sélection indisponible.')}
function addModelFields(purpose,fixedProvider='',initial='auto'){
 const c=modalContext;if(!c)return;
 const level=field('level',tr_web_providers_js('Niveau de l’agent'),initial,levelOptions);
 const hidden=field('model_policy_hash',tr_web_providers_js('Empreinte de la politique'),'');hidden.type='hidden';hidden.parentElement.hidden=true;
 const info=node('p',tr_web_providers_js('Résolution du modèle…'),'notice info');info.id='model-selection';$('modal-fields').append(info);
 let generation=0;
 const update=async()=>{const n=++generation;if(['task-model','role-model'].includes(c.action)&&$('field-inherit')?.value==='true'){c.modelReady=true;$('confirm').disabled=false;info.textContent=tr_web_providers_js(c.action==='role-model'?'Hériter du modèle de planification':'Hériter du profil de lancement');return}hidden.value='';c.modelReady=false;$('confirm').disabled=true;info.className='notice info';info.textContent=tr_web_providers_js('Résolution du modèle…');
  try{const provider=typeof fixedProvider==='function'?fixedProvider():fixedProvider||$('field-provider')?.value;if(!provider)throw Error(tr_web_providers_js('Choisir un fournisseur.'));const data=await api('/api/v1/providers/resolve?'+new URLSearchParams({provider,level:level.value,purpose}));if(modalContext!==c||n!==generation||!level.isConnected)return;hidden.value=data.route?.policy_hash||'';c.modelReady=true;info.textContent=routeText(data.route);$('confirm').disabled=false;
  }catch(e){if(modalContext!==c||n!==generation||!level.isConnected)return;info.className='notice alert';info.textContent=e.message;$('confirm').disabled=true}
 };
 if(typeof fixedProvider==='function'&&$('field-agent'))$('field-agent').addEventListener('change',update);if(purpose==='planning'&&$('field-planning-provider'))$('field-planning-provider').addEventListener('change',update);if(['task-model','role-model'].includes(c.action)&&$('field-inherit'))$('field-inherit').addEventListener('change',update);level.onchange=update;if(!fixedProvider&&$('field-provider'))$('field-provider').addEventListener('change',update);update();
}
async function loadProviderAdmin(){
 if(providerLoading)return;providerLoading=true;void AIConnections.load();void ProjectProfiles.load();$('providers-state').textContent=tr_web_providers_js('Chargement des fournisseurs et des modèles…');
 try{providerAdmin=await api('/api/v1/providers/admin');renderProviderAdmin();$('providers-state').className='notice info';$('providers-state').textContent=tr_web_providers_js('Politique locale : aucun appel IA au chargement. Un modèle exigeant demande un choix explicite ; aucun repli automatique.')}
 catch(e){$('providers-state').className='notice alert';$('providers-state').textContent=e.message}
 finally{providerLoading=false}
}
/** Render current API metadata with text nodes. Cards/controls describe saved
 * choices and observations, never authenticated identity or launch permission.
 * DOM/read errors propagate to the existing loading/error flow. */
function renderProviderAdmin(){
 const host=$('providers-list');host.replaceChildren();
 for(const [id,p]of Object.entries(providerAdmin.providers)){
  const card=node('article',undefined,'card provider-card');card.dataset.provider=id;
  card.append(node('h3',id),node('p',p.adapter+' · '+p.health,'assist-meta'));
  const policy=p.policy;
  if(!policy){card.append(node('p',tr_web_providers_js('Adaptateur de sélection non configuré pour cet exécutable.'),'notice attention'));host.append(card);continue}
  const meta=node('p',policy.billing==='on_premise'?tr_web_providers_js('Sur site : absence de facture API déclarée ; durée et ressources restent bornées.'):tr_web_providers_js('Fournisseur externe : niveau adapté à la tâche, sans estimation de prix inventée.'),'notice '+(policy.billing==='on_premise'?'info':'neutral'));card.append(meta);
  card.append(node('p',tr_web_providers_js('Défauts : travaux ')+providerLevelText(policy.work_level)+tr_web_providers_js(' · aide de page ')+providerLevelText(policy.page_level)+(p.saved?tr_web_providers_js(' · politique enregistrée'):tr_web_providers_js(' · politique proposée par le moteur'))));
  const table=node('table'),head=node('thead'),hr=node('tr');for(const label of [tr_web_providers_js('Niveau'),tr_web_providers_js('Modèle'),'Effort'])hr.append(node('th',label));head.append(hr);table.append(head);const body=node('tbody');
  for(const level of ['simple','standard','exigeant']){const row=node('tr');row.append(node('td',providerLevelText(level)),node('td',policy.levels[level]?.model||tr_web_providers_js('À configurer')),node('td',policy.levels[level]?.effort||tr_web_providers_js(p.adapter==='claude'?'Réglage hérité de Claude Code':'Défaut du modèle')));body.append(row)}table.append(body);card.append(table);
  const actions=node('div',undefined,'provider-actions');actions.append(button(tr_web_providers_js('Configurer les niveaux'),()=>editProviderPolicy(id)));
  const test=button(tr_web_providers_js('Tester avec une question'),async()=>{if(!snapshot||snapshot.work.id!==work)await refresh(true);if(!snapshot||snapshot.work.id!==work){notice(tr_web_providers_js('Sélectionnez un travail chargé pour y conserver la question de test.'),true);return}try{if(!assistMeta)await loadAssistMeta()}catch(e){notice(tr_web_providers_js('Assistant indisponible : ')+e.message,true);return}showView('tasks');openAssistAsk();if(!$('field-provider'))return;$('field-provider').value=id;$('field-provider').dispatchEvent(new Event('change'));$('modal-description').textContent=tr_web_providers_js('Test explicite du fournisseur : choisissez le niveau, examinez le contexte et confirmez l’appel. Le résultat et l’usage déclaré seront conservés dans l’historique.')});test.disabled=Boolean(p.assistant_unavailable);if(p.assistant_unavailable)test.title=p.assistant_unavailable;actions.append(test);card.append(actions);
  if(p.assistant_unavailable)card.append(node('p',p.assistant_unavailable,'notice attention'));
  const details=node('details');details.append(node('summary',tr_web_providers_js('Connexion et provenance du catalogue')),node('p',p.command,'assist-meta'),node('p',tr_web_providers_js('Authentification conservée par la CLI. Les secrets ne sont ni importés ni exportés.')));
  for(const m of p.models)details.append(node('p',m.id+' — '+m.source,'assist-meta'));card.append(details);host.append(card);
 }
}
/** Open the current provider policy snapshot for editing. Show every schema
 * default level so an existing choice cannot silently fall back to the first
 * option; authoritative saved rules are checked at preview/apply. No file writes
 * happen here. Missing/stale metadata errors remain in the existing modal flow. */
function editProviderPolicy(id){
 const p=providerAdmin.providers[id];openModal(tr_web_providers_js('Fournisseur — ')+id,tr_web_providers_js('Choisir le modèle adapté à chaque niveau. La sauvegarde affecte les prochains lancements ; elle ne change pas un agent déjà démarré.'),{action:'provider-policy',provider:id,digest:providerAdmin.digest});
 field('billing',tr_web_providers_js('Facturation déclarée'),p.policy.billing,[['external',tr_web_providers_js('Fournisseur externe')],['on_premise',tr_web_providers_js('Sur site — pas de facture API marginale')]]);
 const defaultsHelp=tr_web_providers_js('Les règles courantes déterminent quels niveaux peuvent devenir des défauts. Une proposition refusée reste non enregistrée.');
 field('work_level',tr_web_providers_js('Niveau automatique des travaux'),p.policy.work_level,levelOptions.filter(([level])=>level!=='auto').map(([level])=>[level,providerLevelText(level)]),false,defaultsHelp);field('page_level',tr_web_providers_js('Niveau automatique de l’aide'),p.policy.page_level,levelOptions.filter(([level])=>level!=='auto').map(([level])=>[level,providerLevelText(level)]),false,defaultsHelp);
 for(const level of ['simple','standard','exigeant']){
  const group=node('section',undefined,'provider-level');group.append(node('h3',tr_web_providers_js('Niveau ')+providerLevelText(level)));$('modal-fields').append(group);
  const model=field('model_'+level,tr_web_providers_js('Modèle — ')+providerLevelText(level),p.policy.levels[level].model,p.models.map(m=>[m.id,m.id]));
  const choices=()=>[['',tr_web_providers_js(p.adapter==='claude'?'Réglage hérité de Claude Code':'Défaut du modèle')],...(p.models.find(m=>m.id===model.value)?.efforts||[]).map(e=>[e,e])];const effort=field('effort_'+level,tr_web_providers_js('Effort — ')+providerLevelText(level),p.policy.levels[level].effort,choices());model.onchange=()=>selectOptions(effort,choices(),'');group.append(model.parentElement,effort.parentElement);
 }
 $('confirm').textContent=tr_web_providers_js('Examiner la politique');
}
/** Preview or confirm policy-only form data for this still-current dialog.
 * f is the collected form; c retains source digest, signature and optional native
 * Root-bound preview token. Returns false after a preview, true after application
 * and reload. Malformed imports/tokens and API failures throw to the modal's
 * visible error handler. Go servers without a token receive the unchanged wire
 * shape. A token is public configuration binding, never identity or launch rights;
 * only confirmed apply mutates configuration, and no agent is started here.
 */
async function submitProviderPolicy(f,c){
 let policies;
 if(c.action==='provider-import'){
  let d;try{d=JSON.parse(f.document)}catch{throw Error(tr_web_providers_js('JSON invalide : utiliser un export de politiques version 1.'));}
  if(d.version!==1||!d.policies||Object.keys(d).some(k=>!['version','policies'].includes(k)))throw Error(tr_web_providers_js('Import limité à version et policies. Les connexions, clés et commandes ne sont pas importées.'));policies=d.policies;
 }else{const levels={};for(const level of ['simple','standard','exigeant'])levels[level]={model:f['model_'+level],effort:f['effort_'+level]};policies={[c.provider]:{version:1,billing:f.billing,work_level:f.work_level,page_level:f.page_level,levels}}}
 const request={version:1,expected_digest:c.digest,policies},signature=JSON.stringify(request);
 if(c.signature!==signature){const data=await api('/api/v1/providers/policies',{...request,preview:true});if(modalContext!==c)return false;if(Object.hasOwn(data,'preview_token')&&(typeof data.preview_token!=='string'||!/^[a-f0-9]{64}$/.test(data.preview_token)))throw Error(tr_web_providers_js('Empreinte de prévisualisation invalide ; examiner à nouveau la politique.'));c.providerPreviewToken=data.preview_token||'';c.signature=signature;preview(tr_web_providers_js('POLITIQUE À ENREGISTRER\n')+Object.entries(data.policies).map(([id,p])=>id+' · '+(p.billing==='on_premise'?tr_web_providers_js('sur site'):tr_web_providers_js('externe'))+tr_web_providers_js('\nDéfauts : travaux ')+providerLevelText(p.work_level)+tr_web_providers_js(' · aide ')+providerLevelText(p.page_level)+'\n'+Object.entries(p.levels).map(([level,m])=>'  '+providerLevelText(level)+' → '+m.model+(m.effort?tr_web_providers_js(' · effort ')+m.effort:'')).join('\n')).join('\n\n')+tr_web_providers_js('\n\nAucun agent lancé. Aucune commande ni clé modifiée.'));$('confirm').textContent=tr_web_providers_js('Confirmer l’enregistrement');return false}
 const result=await api('/api/v1/providers/policies',{...request,preview:false,...(c.providerPreviewToken?{preview_token:c.providerPreviewToken}:{})});if(!result.applied)throw Error(tr_web_providers_js('Enregistrement non confirmé.'));notice(result.audit_warning||tr_web_providers_js('Politique enregistrée. Les prochains lancements utiliseront ces niveaux.'),Boolean(result.audit_warning));await loadProviderAdmin();return true;
}
$('providers-refresh').onclick=loadProviderAdmin;
$('providers-export').onclick=()=>{if(!providerAdmin)return;const url=URL.createObjectURL(new Blob([JSON.stringify(providerAdmin.export,null,2)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download='swarm-model-policies.v1.json';a.click();URL.revokeObjectURL(url)};
$('providers-import').onclick=()=>{if(!providerAdmin)return;openModal(tr_web_providers_js('Importer des politiques de modèles'),tr_web_providers_js('Contrat portable version 1. Les fournisseurs doivent déjà être configurés sur cette machine. Aucun secret ni commande dans ce document.'),{action:'provider-import',digest:providerAdmin.digest});field('document',tr_web_providers_js('Document JSON'),'',null,true);$('confirm').textContent=tr_web_providers_js('Examiner l’import')};

if(view==='providers')loadProviderAdmin();
