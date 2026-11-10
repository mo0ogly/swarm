'use strict';
let trustOIDCSession=null;
const trustTr=s=>globalThis.SwarmI18n?.t(s)??s,trust=id=>document.getElementById(id);let trustCSRF='',trustView=null,trustPending=null,trustReturnFocus=null,trustConfigEpoch=0;
async function trustAPI(path,data,token,encode=JSON.stringify,signal,decode=JSON.parse){const options={headers:{},signal};if(data!==undefined){options.method='POST';options.headers={'Content-Type':'application/json','X-Swarm-CSRF':trustCSRF};options.body=encode(data)}if(token)options.headers.Authorization="Bearer "+token;const response=await fetch(path,options),text=await response.text();let value;try{value=decode(text)}catch(error){if(decode!==JSON.parse)throw error;value={error:text||response.statusText}}if(!response.ok)throw Error(trustTr(value.error||response.statusText));return value}
function trustStatus(text,error=false){const el=trust('trust-state');el.textContent=text;el.className='notice '+(error?'alert':'info')}
function renderTrust(){if(typeof renderTrustObserve==='function')renderTrustObserve();if(typeof renderTrustKernel==='function')renderTrustKernel();if(typeof renderTrustConnectors==='function')renderTrustConnectors();if(typeof renderTrustSecrets==='function')renderTrustSecrets();renderTrustSealed();renderTrustDelegations();renderTrustOIDC();trust('trust-revision').textContent=trustView.revision;const actions=Object.values(trustView.actions||{}).sort((a,b)=>b.updated_at.localeCompare(a.updated_at));trust('trust-action-count').textContent=actions.length;trust('trust-ledger-count').textContent=(trustView.ledger||[]).length;trust('trust-proof-count').textContent=trustView.evidence_count;trust('trust-policy').textContent=JSON.stringify(trustView.policy,null,2);trust('trust-limits').replaceChildren(...(trustView.limits||[]).map(x=>{const li=document.createElement('li');li.textContent=trustTr(x);return li}));const host=trust('trust-actions-list');host.replaceChildren();if(!actions.length){const p=document.createElement('p');p.className='notice info';p.textContent=trustTr('Aucune demande pour le moment.');host.append(p)}for(const action of actions){const article=document.createElement('article');article.className='trust-action';const h=document.createElement('h3'),badge=document.createElement('span');h.textContent=action.id+' · ';badge.className='badge';badge.textContent=trustTr(({pending_approval:'En attente d’approbation',approved:'Approuvée',committed:'Effet engagé',unknown:'Résultat à réconcilier',reconciled:'Réconciliée'})[action.state]||action.state);h.append(badge);const dl=document.createElement('dl');for(const [label,value,system]of [[trustTr('Demandeur'),action.requester],[trustTr('Ressource'),action.resource],...(action.connector?.access?[[trustTr("Sujet de l'accès simulé"),action.connector.access.subject],[trustTr("Rôle de l'accès simulé"),action.connector.access.role]]:[[trustTr('Montant (unités mineures)'),(trustView.amount_minor_text?.[action.id]??String(action.amount_minor))+' '+action.currency],[trustTr('Bénéficiaire'),action.beneficiary]]),...(action.data_context?[[trustTr('Classification des données'),action.data_context.classification],[trustTr('Destination contrôlée'),action.data_context.destination],[trustTr('Fournisseur et outil'),action.data_context.provider+' / '+action.data_context.tool]]:[]),[trustTr('Motif'),action.reason||trustTr('Aucun motif bloquant'),true],[trustTr('Prochaine action'),action.next_action||trustTr('Aucune'),true]]){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=label;dd.textContent=system?trustTr(value):value;dl.append(dt,dd)}article.append(h,dl);renderTrustStepUpAction(article,action);host.append(article)}}
async function loadTrust(){trustStatus(trustTr('Chargement du pilote…'));try{if(!trustCSRF)trustCSRF=(await trustAPI('/api/v1/session')).csrf;trustView=await trustAPI('/api/v1/trust-actions');const ended=await checkTrustOIDCSession();renderTrust();await finishTrustOIDCReturn();trustStatus(trustTr(ended?'Session d’entreprise terminée ou révoquée. Reconnectez-vous.':'Pilote chargé. Les effets affichés sont simulés.'))}catch(e){trustStatus(e.message,true)}}
function configRequest(form){for(const key of ['mandate_ttl_seconds','unknown_reconcile_seconds','evidence_retention_days']){const value=Number(form[key].value),bounds=trustView.policy_bounds[key];if(!Number.isSafeInteger(value)||value<bounds.min||value>bounds.max)throw Error(trustTr('La valeur doit être un entier dans les limites techniques affichées.'))}let identities;try{identities=JSON.parse(form.identities.value)}catch{throw Error(trustTr('Le JSON des identités est invalide.'))}if(!Array.isArray(identities)||identities.some(v=>!v||typeof v.token!=='string'||!v.token.trim()))throw Error(trustTr('Renseignez un jeton privé pour chaque identité.'));if(identities.some(v=>v.disabled!==undefined&&typeof v.disabled!=='boolean'))throw Error(trustTr('L’état de suspension doit être un booléen.'));return{schema_version:1,event_id:trustPending?.event_id||crypto.randomUUID(),expected_revision:trustView.revision,policy:{mandate_ttl_seconds:Number(form.mandate_ttl_seconds.value),unknown_reconcile_seconds:Number(form.unknown_reconcile_seconds.value),evidence_retention_days:Number(form.evidence_retention_days.value),require_separation:form.require_separation.checked},identities,oidc:trustOIDCConfigRequest(form),workloads:trustWorkloadConfigRequest(form),delegation:trustDelegationConfigRequest(form),step_up:trustStepUpConfigRequest(form),oidc_logout:trustOIDCLogoutConfigRequest(form),group_mapping:trustGroupMappingConfigRequest(form),data_policy:trustDataPolicyConfigRequest(form),kernel_check:trustKernelPolicyRequest(form),kernel_observation:trustObservePolicyRequest(form),sealed_execution:trustSealedPolicyRequest(form),secret_broker:trustSecretPolicyRequest(form),workload_introspection:trustIntrospectionPolicyRequest(form),reason:form.reason.value.trim()}}
function renderTrustIdentityAccess(){
 const form=trust('trust-config-form'),host=trust('trust-identity-access');let identities;
 try{identities=JSON.parse(form.identities.value)}catch{host.replaceChildren();return}
 if(!Array.isArray(identities)){host.replaceChildren();return}
 host.replaceChildren(...identities.filter(id=>id&&typeof id.subject==='string').map(id=>{
  const label=document.createElement('label'),input=document.createElement('input'),text=document.createElement('span');input.type='checkbox';input.checked=!id.disabled;input.dataset.subject=id.subject;
  const refresh=()=>text.textContent=id.subject+' — '+trustTr(input.checked?'Active':'Suspendue');refresh();
  input.addEventListener('change',()=>{try{const values=JSON.parse(form.identities.value),current=values.find(v=>v.subject===id.subject);if(!current)throw Error(trustTr('Le JSON des identités est invalide.'));current.disabled=!input.checked;form.identities.value=JSON.stringify(values,null,2);refresh();invalidateTrustPreview()}catch(error){input.checked=!id.disabled;refresh();const box=trust('trust-config-error');box.textContent=error.message;box.hidden=false;box.focus()}});
  label.append(input,text);return label;
 }));
}
function invalidateTrustPreview(){trustConfigEpoch++;trustPending=null;trust('trust-config-preview').hidden=true;trust('trust-config-confirm').textContent=trustTr('Prévisualiser');trust('trust-config-confirm').disabled=false}
function openTrustConfig(){trustReturnFocus=document.activeElement;invalidateTrustPreview();const form=trust('trust-config-form'),p=trustView.policy;form.mandate_ttl_seconds.value=p.mandate_ttl_seconds;form.unknown_reconcile_seconds.value=p.unknown_reconcile_seconds;form.evidence_retention_days.value=p.evidence_retention_days;form.require_separation.checked=p.require_separation;form.identities.value=JSON.stringify((trustView.identities.length?trustView.identities:[{subject:'admin',integrity:3,roles:['admin']},{subject:'requester',integrity:1,roles:['requester']},{subject:'approver',integrity:2,roles:['approver']},{subject:'effect-service',integrity:2,roles:['effect']}]).map(v=>({...v,token:''})),null,2);renderTrustIdentityAccess();openTrustOIDCConfig(form);openTrustWorkloadConfig(form);openTrustStepUpConfig(form);openTrustOIDCLogoutConfig(form);openTrustGroupMappingConfig(form);openTrustDataPolicyConfig(form);openTrustSealedPolicy(form);openTrustKernelPolicy(form);openTrustObservePolicy(form);openTrustSecretPolicy(form);openTrustIntrospectionPolicy(form);form.delegation_max_depth.value=trustView.delegation.max_depth;form.reason.value='';trust('trust-config-error').hidden=true;trust('trust-config-preview').hidden=true;trust('trust-config-confirm').textContent=trustTr('Prévisualiser');trust('trust-policy-bounds').textContent=trustTr('Limites techniques de représentation :')+' '+trustTr('Durées (secondes)')+' 0–'+trustView.policy_bounds.mandate_ttl_seconds.max+' ; '+trustTr('Conservation (jours)')+' 1–'+trustView.policy_bounds.evidence_retention_days.max;trust('trust-config').showModal();form.mandate_ttl_seconds.focus()}
trust('trust-config-form').addEventListener('input',invalidateTrustPreview);
trust('trust-config-form').elements.identities.addEventListener('input',renderTrustIdentityAccess);
trust('trust-config-form').addEventListener('submit',async e=>{
 e.preventDefault();const button=trust('trust-config-confirm'),dialog=trust('trust-config');if(button.disabled)return;
 const epoch=++trustConfigEpoch,error=trust('trust-config-error');button.disabled=true;error.hidden=true;
 try{const request=configRequest(e.target);if(!trustPending){
  const preview=await trustAPI('/api/v1/trust-actions?action=config-preview',request);
  if(epoch!==trustConfigEpoch||!dialog.open||request.expected_revision!==trustView.revision)return;
  trustPending=request;const out=trust('trust-config-preview');out.textContent=JSON.stringify(preview,null,2);out.hidden=false;button.textContent=trustTr('Appliquer la configuration');out.focus();
 }else{await trustAPI('/api/v1/trust-actions?action=config-apply',trustPending);if(epoch===trustConfigEpoch&&dialog.open)dialog.close();await loadTrust()}}
 catch(err){if(epoch!==trustConfigEpoch||!dialog.open)return;error.textContent=err.message;error.hidden=false;error.focus()}
 finally{if(epoch===trustConfigEpoch)button.disabled=false}
});
for(const id of ['trust-config-close','trust-config-cancel'])trust(id).addEventListener('click',()=>trust('trust-config').close());trust('trust-config').addEventListener('close',()=>{invalidateTrustPreview();const active=document.activeElement;if(!active||active===document.body||active===trustReturnFocus||trust('trust-config').contains(active))trustReturnFocus?.focus()});trust('trust-config-open').addEventListener('click',openTrustConfig);trust('trust-refresh').addEventListener('click',loadTrust);trust('trust-theme').addEventListener('click',()=>{const dark=document.documentElement.dataset.theme==='sombre';document.documentElement.dataset.theme=dark?'etat':'sombre';trust('trust-theme').textContent=trustTr(dark?'Passer au thème sombre':'Passer au thème État')});loadTrust();

function openTrustOIDCConfig(form){
 const c=trustView.oidc||trustView.oidc_defaults;form.oidc_enabled.checked=!!c.enabled;
 for(const key of ['issuer','client_id','redirect_uri','request_timeout_seconds','login_ttl_seconds','session_ttl_seconds'])form['oidc_'+key].value=c[key]??'';
 if(!form.oidc_redirect_uri.value)form.oidc_redirect_uri.value=location.origin+'/trust-oidc-callback.html';
 form.oidc_mappings.value=JSON.stringify(c.mappings||[],null,2);
}
function trustOIDCConfigRequest(form){
 const c={enabled:form.oidc_enabled.checked};
 for(const key of ['issuer','client_id','redirect_uri'])c[key]=form['oidc_'+key].value.trim();
 for(const key of ['request_timeout_seconds','login_ttl_seconds','session_ttl_seconds']){c[key]=Number(form['oidc_'+key].value);if(c.enabled&&(!Number.isSafeInteger(c[key])||c[key]<1))throw Error(trustTr('Les durées OIDC doivent être des entiers strictement positifs.'))}
 try{c.mappings=JSON.parse(form.oidc_mappings.value)}catch{throw Error(trustTr('Le JSON des correspondances OIDC est invalide.'))}
 return c;
}
function renderTrustOIDC(){
 if(trustOIDCSession&&(trustOIDCSession.revision!==trustView.revision||!(Date.parse(trustOIDCSession.expires_at)>Date.now())))trustOIDCSession=null;
 trust('trust-oidc-access').hidden=!trustView.oidc?.enabled;
 trust('trust-oidc-status').textContent=trustOIDCSession?trustTr('Identité connectée :')+' '+trustOIDCSession.subject:trustTr('Aucune identité d’entreprise connectée.');
 trust('trust-oidc-logout').hidden=!trustOIDCSession;
 trust('trust-identity-context-show').hidden=!trustOIDCSession;trust('trust-identity-context').hidden=true;trust('trust-identity-context-show').setAttribute('aria-expanded','false');trust('trust-identity-context-values').replaceChildren();
}
async function beginTrustOIDC(request={},token,button=trust('trust-oidc-login')){
 button.disabled=true;
 try{const login=await trustAPI('/api/v1/trust-actions?action=oidc-start',request,token);sessionStorage.setItem('swarm-trust-oidc',JSON.stringify({state:login.state,binding:login.binding,lang:document.documentElement.lang}));location.assign(login.authorization_url)}catch(error){trustStatus(error.message,true);button.disabled=false}
}
trust('trust-oidc-login').addEventListener('click',()=>beginTrustOIDC());

async function finishTrustOIDCReturn(){
 const q=new URLSearchParams(location.search);if(!q.has('state')&&!q.has('code')&&!q.has('error'))return;
 const code=q.get('code'),state=q.get('state');q.delete('code');q.delete('state');q.delete('error');q.delete('error_description');q.delete('iss');history.replaceState(null,'',location.pathname+(q.size?'?'+q.toString():''));
 const saved=sessionStorage.getItem('swarm-trust-oidc');sessionStorage.removeItem('swarm-trust-oidc');let login;try{login=JSON.parse(saved)}catch{}
 if(!code||!state||state!==login?.state)throw Error(trustTr('Retour OIDC invalide. Recommencez la connexion.'));
 trustOIDCSession=await trustAPI('/api/v1/trust-actions?action=oidc-finish',{code,state,binding:login.binding});renderTrust();
}
trust('trust-oidc-logout').addEventListener('click',async()=>{
 try{if(trustOIDCSession)await trustAPI('/api/v1/trust-actions?action=oidc-logout',{session_token:trustOIDCSession.session_token});trustOIDCSession=null;renderTrust()}catch(error){trustStatus(error.message,true)}
});

function openTrustWorkloadConfig(form){
 const c=trustView.workloads||trustView.workload_defaults;form.workloads_enabled.checked=!!c.enabled;
 for(const key of ['issuer','audience','request_timeout_seconds','assertion_ttl_seconds','session_ttl_seconds'])form['workloads_'+key].value=c[key]??'';
 form.workloads_clients.value=JSON.stringify(c.clients||[],null,2);
}
function trustWorkloadConfigRequest(form){
 const c={enabled:form.workloads_enabled.checked};
 for(const key of ['issuer','audience'])c[key]=form['workloads_'+key].value.trim();
 for(const key of ['request_timeout_seconds','assertion_ttl_seconds','session_ttl_seconds']){c[key]=Number(form['workloads_'+key].value);if(c.enabled&&(!Number.isSafeInteger(c[key])||c[key]<1))throw Error(trustTr('Les durées des services doivent être des entiers strictement positifs.'))}
 try{c.clients=JSON.parse(form.workloads_clients.value)}catch{throw Error(trustTr('Le JSON des clients de service est invalide.'))}
 return c;
}

function trustDelegationConfigRequest(form){
 const value=Number(form.delegation_max_depth.value),bounds=trustView.policy_bounds.max_delegation_depth;
 if(!Number.isSafeInteger(value)||value<bounds.min||value>bounds.max)throw Error(trustTr('La profondeur doit être un entier positif ou nul dans les limites techniques affichées.'));
 return{max_depth:value};
}
function renderTrustDelegations(){
 const host=trust('trust-mandates-list'),mandates=Object.values(trustView.mandates||{}).sort((a,b)=>a.id.localeCompare(b.id));host.replaceChildren();
 if(!mandates.length){const p=document.createElement('p');p.textContent=trustTr('Aucun mandat pour le moment.');host.append(p);return}
 for(const m of mandates){const article=document.createElement('article');article.className='trust-mandate';const h=document.createElement('h3');h.textContent=m.id;const dl=document.createElement('dl');
  for(const[label,value]of [['Titulaire',m.subject],['Mandat parent',m.parent_mandate_id||trustTr('Mandat racine')],...(m.connector?.access?[["Sujet de l'accès simulé",m.connector.access.subject],["Rôle de l'accès simulé",m.connector.access.role]]:[['Plafond (unités mineures)',(trustView.mandate_amount_minor_text[m.id]??String(m.max_amount_minor))+' '+m.currency]]),['Ressource',m.resource],['Résultat des contrôles',trustTr(trustView.mandate_status[m.id]||'Mandat disponible')]]){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=trustTr(label);dd.textContent=value;dl.append(dt,dd)}
  article.append(h,dl);if(m.parent_content_digest){const details=document.createElement('details'),summary=document.createElement('summary'),code=document.createElement('code');summary.textContent=trustTr('Empreinte du parent');code.textContent=m.parent_content_digest;details.append(summary,code);article.append(details)}host.append(article);
 }
}

function openTrustStepUpConfig(form){
 const p=trustView.step_up;form.step_up_enabled.checked=!!p.enabled;form.step_up_max_age_seconds.value=p.max_age_seconds;form.step_up_proof_ttl_seconds.value=p.proof_ttl_seconds;form.step_up_acr_values.value=JSON.stringify(p.acr_values||[],null,2);
}
function trustStepUpConfigRequest(form){
 if(!form.step_up_max_age_seconds.value.trim()||!form.step_up_proof_ttl_seconds.value.trim())throw Error(trustTr('Âge maximal positif ou nul et durée de preuve strictement positive requis.'));
 const p={enabled:form.step_up_enabled.checked,operations:['approve'],max_age_seconds:Number(form.step_up_max_age_seconds.value),proof_ttl_seconds:Number(form.step_up_proof_ttl_seconds.value)};
 for(const key of ['max_age_seconds','proof_ttl_seconds']){const bounds=trustView.policy_bounds['step_up_'+key];if(!Number.isSafeInteger(p[key])||p[key]<bounds.min||p[key]>bounds.max)throw Error(trustTr('Âge maximal positif ou nul et durée de preuve strictement positive requis.'))}
 try{p.acr_values=JSON.parse(form.step_up_acr_values.value)}catch{throw Error(trustTr('Le JSON des ACR acceptés est invalide.'))}
 return p;
}
function renderTrustStepUpAction(article,action){
 if(!trustView.step_up.enabled||action.state!=='pending_approval')return;
 const ready=trustOIDCSession?.step_up_action_id===action.id,p=document.createElement('p');p.textContent=trustTr(ready?'Authentification renforcée prête pour cette action.':'Cette approbation nécessite une preuve renforcée liée à cette action.');article.append(p);
 if(ready)return;
 const button=document.createElement('button');button.type='button';button.textContent=trustTr('Renforcer mon authentification pour cette action');button.dataset.stepUpAction=action.id;
 button.disabled=!trustOIDCSession||!trustOIDCSession.roles?.includes('approver')||trustOIDCSession.integrity<2||(trustView.policy.require_separation&&trustOIDCSession.subject===action.requester);
 button.addEventListener('click',()=>beginTrustOIDC({step_up_action_id:action.id},trustOIDCSession?.session_token,button));article.append(button);
 if(!trustOIDCSession){const hint=document.createElement('p');hint.textContent=trustTr('Connectez une identité d’approbateur pour continuer.');article.append(hint)}
}


function openTrustOIDCLogoutConfig(form){
 const p=trustView.oidc_logout;form.oidc_logout_enabled.checked=!!p.enabled;form.oidc_logout_max_age_seconds.value=p.max_age_seconds;form.oidc_logout_replay_retention_seconds.value=p.replay_retention_seconds;
 trust('trust-oidc-logout-uri').textContent=location.origin+'/api/v1/trust-actions/oidc-backchannel-logout';trust('trust-oidc-logout-consumption').textContent=trustTr('Empreintes anti-rejeu conservées :')+' '+trustView.oidc_logout_replay_records;
}
function trustOIDCLogoutConfigRequest(form){
 const p={enabled:form.oidc_logout_enabled.checked};
 for(const key of ['max_age_seconds','replay_retention_seconds']){const raw=form['oidc_logout_'+key].value,bounds=trustView.policy_bounds['oidc_logout_'+key];p[key]=Number(raw);if(!raw.trim()||!Number.isSafeInteger(p[key])||p[key]<bounds.min||p[key]>bounds.max)throw Error(trustTr('Âge de déconnexion positif et conservation positive ou nulle requis.'))}
 if(p.replay_retention_seconds>0&&p.replay_retention_seconds<p.max_age_seconds)throw Error(trustTr('La conservation anti-rejeu doit couvrir l’âge maximal du jeton.'));
 return p;
}
async function checkTrustOIDCSession(){
 if(!trustOIDCSession)return false;
 const status=await trustAPI('/api/v1/trust-actions?action=oidc-session',{session_token:trustOIDCSession.session_token});
 if(!status.active){trustOIDCSession=null;return true}return false;
}

trust('trust-identity-context-show').addEventListener('click',async()=>{
 const button=trust('trust-identity-context-show'),session=trustOIDCSession;if(!session||button.disabled)return;button.disabled=true;
 try{const c=await trustAPI('/api/v1/trust-actions?action=identity-context',{},session.session_token);if(trustOIDCSession!==session)return;
  const fields=[[trustTr('Identité locale'),c.principal_id],[trustTr('Émetteur vérifié'),c.issuer??trustTr('Non disponible')],[trustTr('Compte externe vérifié'),c.subject??trustTr('Non disponible')],[trustTr('Méthode de connexion'),trustTr(c.assurance.method==='oidc_code_pkce'?'OIDC avec PKCE':c.assurance.method==='private_key_jwt'?'Clé de service':'Jeton local')],[trustTr('Droits locaux'),c.attributes.roles.join(', ')],[trustTr('Expiration de la session'),c.expires_at??trustTr('Non disponible')]];
  const dl=trust('trust-identity-context-values');dl.replaceChildren();for(const[label,value]of fields){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=label;dd.textContent=value;dl.append(dt,dd)}trust('trust-identity-context').hidden=false;button.setAttribute('aria-expanded','true');
 }catch(error){trust('trust-identity-context').hidden=true;trust('trust-identity-context-values').replaceChildren();trustStatus(error.message,true)}finally{button.disabled=false}
});

function openTrustGroupMappingConfig(form){const p=trustView.group_mapping;form.group_mapping_enabled.checked=!!p.enabled;form.group_mapping_claim.value=p.claim;form.group_mapping_rules.value=JSON.stringify(p.rules,null,2)}
function trustGroupMappingConfigRequest(form){let rules;try{rules=JSON.parse(form.group_mapping_rules.value)}catch{throw Error(trustTr('Le JSON des règles de groupes est invalide.'))}return{enabled:form.group_mapping_enabled.checked,claim:form.group_mapping_claim.value,rules}}

function openTrustDataPolicyConfig(form){const p=trustView.data_policy;form.data_policy_enabled.checked=!!p.enabled;for(const key of ["classifications","destinations","providers","tools","resources"])form["data_policy_"+key].value=JSON.stringify(p[key])}
function trustDataPolicyConfigRequest(form){const p={enabled:form.data_policy_enabled.checked};for(const key of ["classifications","destinations","providers","tools","resources"]){try{p[key]=JSON.parse(form["data_policy_"+key].value)}catch{throw Error(trustTr("Le JSON de la politique de données est invalide."))}}return p}

function openTrustSealedPolicy(form){const p=trustView.sealed_execution;form.sealed_execution_enabled.checked=!!p.enabled;form.sealed_execution_timeout_seconds.value=p.timeout_seconds}
function trustSealedPolicyRequest(form){const value=Number(form.sealed_execution_timeout_seconds.value);if(form.sealed_execution_timeout_seconds.value.trim()===''||!Number.isSafeInteger(value)||value<=0)throw Error(trustTr('La durée de vérification scellée doit être un entier strictement positif.'));return{enabled:form.sealed_execution_enabled.checked,timeout_seconds:value}}
function renderTrustSealed(){trust('trust-sealed-check').disabled=!trustView.sealed_execution.enabled;trust('trust-sealed-result').hidden=true;trust('trust-sealed-status').textContent=trustTr(trustView.sealed_execution.enabled?'Vérification disponible. Elle ne qualifie pas les fournisseurs ni les secrets.':'Activez les vérifications scellées dans les paramètres pour contrôler ce processus.');trust('trust-sealed-status').className='notice info'}
trust('trust-sealed-check').addEventListener('click',async()=>{const button=trust('trust-sealed-check'),status=trust('trust-sealed-status'),result=trust('trust-sealed-result'),revision=trustView.revision;button.disabled=true;result.hidden=true;status.textContent=trustTr('Vérification du processus scellé…');status.className='notice info';try{const v=await trustAPI('/api/v1/trust-actions?action=confinement-check',{});if(trustView.revision!==revision||v.revision!==revision)throw Error(trustTr('Configuration modifiée : recommencez la vérification.'));result.textContent=JSON.stringify(v,null,2);result.hidden=false;status.textContent=trustTr('Processus scellé vérifié. Les fournisseurs et les secrets restent non qualifiés.')}catch(error){status.textContent=error.message;status.className='notice alert'}finally{button.disabled=!trustView.sealed_execution.enabled}});

function openTrustIntrospectionPolicy(form){const p=trustView.workload_introspection;form.introspection_enabled.checked=!!p.enabled;for(const k of ['endpoint','client_id','timeout_seconds','validation_window_seconds','max_sessions','max_token_bytes'])form['introspection_'+k].value=p[k];const usage=trustView.workload_introspection_status;trust('trust-introspection-consumption').textContent=trustTr('Sessions conservées en mémoire :')+' '+(usage?.retained_sessions??0)+' / '+p.max_sessions+' ; '+trustTr('Octets conservés :')+' '+(usage?.retained_token_bytes??0);trust('trust-introspection-credential').value='';trust('trust-introspection-output').hidden=true;trust('trust-introspection-output').textContent='';trust('trust-introspection-bounds').textContent=trustTr('Limites techniques de représentation :')+' '+JSON.stringify(Object.fromEntries(Object.entries(trustView.policy_bounds).filter(([k])=>k.startsWith('introspection_'))))}
function trustIntrospectionPolicyRequest(form){const p={enabled:form.introspection_enabled.checked,endpoint:form.introspection_endpoint.value.trim(),client_id:form.introspection_client_id.value.trim()};for(const k of ['timeout_seconds','validation_window_seconds','max_sessions','max_token_bytes']){const f=form['introspection_'+k],v=Number(f.value),b=trustView.policy_bounds['introspection_'+k];if(!f.value.trim()||!Number.isSafeInteger(v)||v<b.min||v>b.max)throw Error(trustTr('Les limites d’introspection doivent être des entiers positifs dans les bornes affichées.'));p[k]=v}return p}
trust('trust-introspection-key').addEventListener('click',async()=>{const b=trust('trust-introspection-key'),field=trust('trust-introspection-credential'),output=trust('trust-introspection-output');b.disabled=true;output.hidden=false;try{const result=await trustAPI('/api/v1/trust-actions?action=workload-introspection-key',{},field.value);output.textContent=JSON.stringify(result,null,2)}catch(e){output.textContent=e.message}finally{field.value='';b.disabled=false}});
