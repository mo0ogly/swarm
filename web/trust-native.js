'use strict';
/** Fixed trust UI; no authority from labels. Bearer remains page-local. Every
 * write is one explicit gesture; lost responses demand readback, never retry. */
(()=>{
const $=id=>document.getElementById(id),t=s=>globalThis.SwarmI18n?.t(s)||s;
let state=null,csrf='',proposal=null,busy=false,focus=null;
let roleTouched=false;
let signingTouched=false;
const keyStates={prepared:'Préparée',active:'Active',retired:'Retirée',revoked:'Révoquée'};
/** Availability guides gestures only; authoritative rights stay server-side. */
function syncGovernance(){
 const k=state?.public_evidence_registry?.keys||{},selected=k[$('trust-key-id').value],enabled=state?.evidence_signing?.enabled===true;
 $('trust-audit-read').disabled=!$('trust-audit-action').value;
 $('trust-key-prepare').disabled=!enabled||Object.values(k).some(v=>v.status==='prepared');
 $('trust-key-activate').disabled=!enabled||selected?.status!=='prepared'||!state?.public_evidence_runtime?.private_available?.[selected.key_id];
 $('trust-key-revoke').disabled=!selected||selected.status==='revoked';
 $('trust-public-anchor').disabled=!enabled||!Object.values(k).some(v=>v.status==='active'&&v.configuration_revision===state.revision&&state.public_evidence_runtime?.private_available?.[v.key_id]);
}
const roleNames={requester:'Demandeur',approver:'Approbateur',effect:'Exécuteur',admin:'Administrateur'};
const rolePairs=[['requester','approver'],['requester','effect'],['requester','admin'],['approver','effect'],['approver','admin'],['effect','admin']];
/** Display the persisted matrix without inventing principal groups. The private
 * identity form supplies only subjects; its tokens never enter this projection. */
function roleSettings(){
 const p=state.role_separation;$('trust-role-enabled').checked=p?.enabled===true;
 $('trust-role-pairs').replaceChildren();
 for(const pair of rolePairs){const label=document.createElement('label'),input=document.createElement('input'),span=document.createElement('span');input.type='checkbox';input.dataset.roles=pair.join(':');input.checked=!!p?.incompatible_roles.some(v=>v.includes(pair[0])&&v.includes(pair[1]));span.textContent=t(roleNames[pair[0]])+' / '+t(roleNames[pair[1]]);label.append(input,span);$('trust-role-pairs').append(label);}
 roleGroups(state.identities,p?.principals||{});roleTouched=false;
 const choices=$('trust-audit-action');choices.replaceChildren();for(const a of Object.values(state.actions)){const o=document.createElement('option');o.value=a.id;o.textContent=a.id+' · '+t(statusLabels[a.state]||a.state);choices.append(o);}
 $('trust-audit-read').disabled=!choices.options.length;
 $('trust-audit-summary').textContent='';$('trust-audit-report').textContent='';
 identitySettings();
 signingSettings();
}
/** Synchronize only the enrolled action button from the public source projection
 * and current gesture. Returns binding availability, never business authority;
 * does not change operation, private inputs or rendered identity lists. */
function syncEnrolledAvailability(){
 const source=state?.identity_authentication?.held_enrollment;
 const bound=source?.http_available===true&&source.scope==='single_prepared_intent';
 $('trust-enrolled-execute').disabled=busy||!bound;
 return bound;
}
/** Show declared mappings separately from an authenticated transport. Text only;
 * no mapping, cookie or unavailable source becomes a business credential. */
function identitySettings(){
 const source=state.identity_authentication?.held_enrollment;
 const bound=syncEnrolledAvailability();
 if(bound&&[...$('trust-enrolled-operation').options].some(o=>o.value===source.operation))$('trust-enrolled-operation').value=source.operation;
 $('trust-identity-availability').textContent=t(bound?'Source HTTP liée à une intention exacte ; le compte et le droit sont revérifiés à chaque appel':source?.root_api_available===true&&source.http_available===false?'Enrôlement : API moteur disponible ; raccord HTTP non actif. Cette page utilise les jetons locaux.':'Disponibilité de la source d’identité non vérifiée');
 const list=$('trust-identity-sources');list.replaceChildren();
 for(const id of state.identities){const item=document.createElement('li'),p=id.enrollment_principal;item.textContent=id.subject+' · '+t(p?'Principal enrôlé déclaré — autorité courante requise':'Identité locale — jeton requis')+(p?' · '+p.principal_id+' · '+p.issuer+' · '+p.subject:'');list.append(item);}
}
/** Render public lifecycle separately from private runtime availability. No key
 * from this display is silently promoted to the verifier's independent pin. */
function signingSettings(){
 const p=state.evidence_signing;$('trust-signing-enabled').checked=p?.enabled===true;$('trust-signing-ttl').value=String(p?.key_ttl_seconds??'');signingTouched=false;
 const select=$('trust-key-id'),selected=select.value;select.replaceChildren();
 for(const key of Object.values(state.public_evidence_registry?.keys||{})){const o=document.createElement('option');o.value=key.key_id;o.textContent=key.key_id+' · '+t(keyStates[key.status]||'Statut inconnu');select.append(o);}if([...select.options].some(o=>o.value===selected))select.value=selected;
 $('trust-key-registry').textContent=JSON.stringify({registry:state.public_evidence_registry||null,runtime:state.public_evidence_runtime||null},null,2);
 keyStatus();
}
/** Show memory possession as an observation, never as effective signing rights. */
function keyStatus(){const id=$('trust-key-id').value,key=state.public_evidence_registry?.keys?.[id];$('trust-key-status').textContent=key?t(keyStates[key.status]||'Statut inconnu')+' · '+t(state.public_evidence_runtime?.private_available?.[id]?'Matière privée présente en mémoire':'Matière privée indisponible')+' · '+t('Expiration')+' : '+key.expires_at:t('Aucune clé publique enregistrée');}
/** Render explicit editable canonical-principal declarations. No inferred person
 * identity, privilege, group membership or credential is produced here. */
function roleGroups(identities,groups){
 if(!Array.isArray(identities)||identities.some(v=>!v||typeof v.subject!=='string'))throw Error(t('Identités locales invalides'));
 $('trust-role-principals').replaceChildren();
 for(const identity of identities){const label=document.createElement('label'),span=document.createElement('span'),input=document.createElement('input');span.textContent=t('Principal de contrôle')+' · '+identity.subject;input.dataset.subject=identity.subject;input.value=groups[identity.subject]||'';input.autocomplete='off';label.append(span,input);$('trust-role-principals').append(label);}
}
/** Serialize this explicit operator choice; omission preserves legacy behavior.
 * Completeness and collisions remain authoritatively validated by the engine. */
function roleProposal(){
 const principals=Object.create(null);for(const input of $('trust-role-principals').querySelectorAll('input')){if(input.value)principals[input.dataset.subject]=input.value;}
 return {enabled:$('trust-role-enabled').checked,incompatible_roles:[...$('trust-role-pairs').querySelectorAll('input:checked')].map(v=>v.dataset.roles.split(':')),principals};
}
const labels={mandate_id:'Mandat',subject:'Sujet',resource:'Ressource',max_amount_minor:'Plafond en unités mineures',amount_minor:'Montant en unités mineures',currency:'Devise',beneficiary:'Bénéficiaire',idempotency_key:'Clé d’idempotence',action_id:'Action',content_digest:'Empreinte exacte',mandate_ttl_seconds:'Durée du mandat en secondes — 0 sans échéance',unknown_reconcile_seconds:'Délai de réconciliation en secondes — 0 immédiat',evidence_retention_days:'Conservation des preuves en jours — minimum 1'};
const shapes={'mandate-create':['mandate_id','subject','resource','max_amount_minor','currency','beneficiary'],request:['mandate_id','resource','amount_minor','currency','beneficiary','idempotency_key'],approve:['action_id','mandate_id','content_digest','resource','amount_minor','currency','beneficiary'],commit:['action_id'],reconcile:['action_id'],'direct-write':['resource']};
const statusLabels={pending_approval:'Approbation requise',approved:'Approuvée',committed:'Engagée',unknown:'Résultat incertain',reconciled:'Réconciliée',denied:'Refusée'},opLabels={approve:'Approuver',commit:'Engager l’effet simulé',reconcile:'Réconcilier'};
const numeric=new Set(['key_ttl_seconds','max_amount_minor','amount_minor',...Object.keys(labels).filter(k=>k.endsWith('_seconds')||k.endsWith('_days'))]);
/** Render editable fields with text semantics; integers are encoded later from
 * validated decimal strings, never rounded by JavaScript Number. */
function fields(container,keys,prefix,values={}){$(container).replaceChildren();for(const key of keys){const label=document.createElement('label'),span=document.createElement('span'),input=document.createElement('input');span.textContent=t(labels[key]);input.id=prefix+key;input.value=values[key]===undefined?'':String(values[key]);if(numeric.has(key))input.inputMode='numeric';label.append(span,input);$(container).append(label);}}
/** Serialize only fields supplied by our forms. BigInt checks int64 before emitting only canonical decimal bytes; all other
 * strings/keys use JSON.stringify without marker substitutions. */
function encode(value){function walk(v,key=''){if(numeric.has(key)){const text=String(v);if(!/^(0|[1-9][0-9]*)$/.test(text)||BigInt(text)>9223372036854775807n)throw Error(t('Entier décimal positif ou nul requis'));return text;}if(Array.isArray(v))return '['+v.map(x=>walk(x)).join(',')+']';if(v&&typeof v==='object')return '{'+Object.entries(v).map(([k,x])=>JSON.stringify(k)+':'+walk(x,k)).join(',')+'}';return JSON.stringify(v);}return walk(value);}

/** Validate transport/JSON and surface the exact denial. Never automatic retry;
 * API is same-origin behind the native operator session and CSRF gate. */
async function api(op,body,enrolledPassword=null){const headers={};if(body!==undefined){headers['Content-Type']='application/json';headers['X-Swarm-CSRF']=csrf;}const admin=op==='config-preview'||op==='config-apply'||op==='audit-transaction'||op.startsWith('evidence-key-')||op==='public-anchor',token=$(admin?'trust-admin-token':'token').value;if(enrolledPassword!==null){let binary='';for(const byte of new TextEncoder().encode(enrolledPassword))binary+=String.fromCharCode(byte);headers.Authorization='Enrollment '+btoa(binary).replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'');}else if(token)headers.Authorization='Bearer '+token;const r=await fetch('/api/v1/trust-actions'+(op==='show'?'':'?action='+encodeURIComponent(op)),{method:op==='show'?'GET':'POST',headers,body:body===undefined?undefined:body,cache:'no-store'});const raw=await r.text();let result;try{result=JSON.parse(raw);}catch{throw Error(t('Réponse illisible ; vérifier l’état avant reprise'));}if(!r.ok&&result.trust_state_may_be_recorded===true)throw Error(t('Effet possiblement enregistré — relire l’état avant toute reprise')+' · '+String(result.trust_publication||''));if(!r.ok)throw Error(result.failure?.code==='authorization_required'?t('Identité ou autorité insuffisante ; l’Admin exige un jeton I3 courant'):result.failure?.code||t('Opération refusée'));if(!result||typeof result!=='object')throw Error(t('Réponse invalide'));return {result,raw};}
function result(v){$('result').textContent=v.raw;}
/** Read authoritative operator projection. A malformed/error response leaves
 * controls unavailable; stored values never become an agent-selected label. */
async function load(){const v=await api('show');if(v.result.schema_version!==1||!Number.isSafeInteger(v.result.revision)||!v.result.policy||!v.result.actions)throw Error(t('Réponse invalide'));state=v.result;render();}
/** Render only text and explicit action buttons; user fields never become HTML. */
function render(){const box=$('actions');box.replaceChildren();for(const a of Object.values(state.actions)){const item=document.createElement('article'),text=document.createElement('p');text.textContent=a.id+' · '+t(statusLabels[a.state]||a.state)+' · '+(state.amount_minor_text[a.id]||'0')+' '+a.currency+' · '+a.resource;item.append(text);for(const op of ['approve','commit','reconcile']){const b=document.createElement('button');b.type='button';b.textContent=t(opLabels[op]||op);b.addEventListener('click',()=>{$('operation').value=op;fields('fields',shapes[op],'f-',{...a,amount_minor:state.amount_minor_text[a.id],action_id:a.id});$('token').focus();});item.append(b);}box.append(item);}}
/** Disable duplicate gestures during one request, surface failures in the active
 * dialog, and retain the requirement for readback after uncertain responses. */
async function guarded(action){if(busy)return;busy=true;for(const b of document.querySelectorAll('button'))b.disabled=true;try{await action();}catch(e){($('settings').open?$('admin-message'):$('message')).textContent=e.message+' — '+t('Vérifier l’état avant reprise');}finally{busy=false;for(const b of document.querySelectorAll('button'))b.disabled=false;$('save').disabled=!proposal;syncGovernance();syncEnrolledAvailability();}}
$('operation').addEventListener('change',()=>fields('fields',shapes[$('operation').value],'f-'));
$('command').addEventListener('submit',event=>{event.preventDefault();guarded(async()=>{const op=$('operation').value,v={schema_version:1,event_id:'web-'+crypto.randomUUID()};for(const key of shapes[op])v[key]=$('f-'+key).value;if(op==='mandate-create')v.actions=['settle'];if(op==='commit')v.lose_response=$('lose').checked;const reply=await api(op,encode(v));result(reply);$('message').textContent=reply.result.state==='denied'?t('Opération refusée')+' · '+(globalThis.SwarmI18n?.engine(reply.result.reason)||reply.result.reason):t('Résultat reçu ; état relu');await load();});});
$('refresh').addEventListener('click',()=>guarded(load));for(const op of ['export','anchor'])$(op).addEventListener('click',()=>guarded(async()=>result(await api(op,'{}'))));
$('admin').addEventListener('click',()=>guarded(async()=>{await load();focus=$('admin');proposal=null;$('identities').value='';$('trust-admin-token').value='';fields('policy',['mandate_ttl_seconds','unknown_reconcile_seconds','evidence_retention_days'],'p-',state.policy);$('separation').checked=state.policy.require_separation;roleSettings();$('admin-message').textContent=t('Renseigner toutes les identités pour modifier la configuration');$('settings').showModal();$('p-mandate_ttl_seconds').focus();}));
$('configuration').addEventListener('input',()=>{proposal=null;$('save').disabled=true;});
$('trust-role-settings').addEventListener('input',()=>{roleTouched=true;});
$('trust-signing-enabled').addEventListener('input',()=>{signingTouched=true;});
$('trust-signing-ttl').addEventListener('input',()=>{signingTouched=true;});
$('trust-key-id').addEventListener('change',()=>{keyStatus();syncGovernance();});
for(const [id,op] of [['trust-key-prepare','evidence-key-prepare'],['trust-key-activate','evidence-key-activate'],['trust-key-revoke','evidence-key-revoke']])$(id).addEventListener('click',()=>guarded(async()=>{
 const body={schema_version:1,event_id:'key-'+crypto.randomUUID(),expected_registry_revision:state.public_evidence_registry?.revision||0,reason:$('reason').value};
 if(op!=='evidence-key-prepare')body.key_id=$('trust-key-id').value;
 const reply=await api(op,JSON.stringify(body));result(reply);await load();signingSettings();if(reply.result.key_id){$('trust-key-id').value=reply.result.key_id;keyStatus();syncGovernance();}$('admin-message').textContent=t('Décision de clé enregistrée et relue');
}));
/** Decode exact signed JSON bytes, preserving int64 values through the UI. */
function publicBytes(raw){if(typeof raw!=='string'||!/^[A-Za-z0-9_-]+$/.test(raw))throw Error(t('Réponse de preuve invalide'));const text=raw.replace(/-/g,'+').replace(/_/g,'/');return new TextDecoder('utf-8',{fatal:true}).decode(Uint8Array.from(atob(text+'='.repeat((4-text.length%4)%4)),c=>c.charCodeAt(0)));}
$('trust-public-anchor').addEventListener('click',()=>guarded(async()=>{const reply=await api('public-anchor','{}'),v=reply.result;if(v.schema_version!==1||v.kind!=='trust-public-anchor-bundle-v1')throw Error(t('Réponse de preuve invalide'));$('trust-public-export').value=publicBytes(v.export_base64);$('trust-public-anchor-input').value=publicBytes(v.anchor_base64);result(reply);$('trust-key-status').textContent=t('Export signé chargé ; fournir une clé de confiance indépendante');}));
$('trust-public-verify').addEventListener('click',()=>guarded(async()=>{
 $('trust-public-verdict').textContent='';
 const entries=[['export','trust-public-export'],['anchor','trust-public-anchor-input'],['trusted_key','trust-public-pin']];if($('trust-public-checkpoint').value.trim())entries.push(['trusted_checkpoint','trust-public-checkpoint']);
 const raw='{'+entries.map(([key,id])=>{const raw=$(id).value;JSON.parse(raw);return JSON.stringify(key)+':'+raw;}).join(',')+'}';
 const reply=await api('public-verify',raw),v=reply.result;if(v.schema_version!==1||v.verified!==true||v.signature_verified!==true||typeof v.checkpoint_verified!=='boolean')throw Error(t('Réponse de preuve invalide'));
 $('trust-public-verdict').textContent=t(v.checkpoint_verified?'Signature et checkpoint vérifiés':'Signature vérifiée, sans checkpoint indépendant');
}));
for(const id of ['trust-public-export','trust-public-anchor-input','trust-public-pin','trust-public-checkpoint'])$(id).addEventListener('input',()=>{$('trust-public-verdict').textContent='';});
$('trust-role-refresh').addEventListener('click',()=>guarded(async()=>{const ids=$('identities').value.trim()?JSON.parse($('identities').value):state.identities;roleGroups(ids,roleProposal().principals);roleTouched=true;proposal=null;}));
$('trust-audit-read').addEventListener('click',()=>guarded(async()=>{
 const id=$('trust-audit-action').value;if(!id)throw Error(t('Choisir une transaction'));
 const reply=await api('audit-transaction',JSON.stringify({action_id:id})),v=reply.result;
 if(v.schema_version!==1||v.kind!=='trust-transaction-audit-v1'||v.action_id!==id||!Array.isArray(v.events)||typeof v.chain_verified!=='boolean'||typeof v.transaction_verified!=='boolean'||v.access_recorded!==true||typeof v.audit_access?.hash!=='string')throw Error(t('Réponse d’audit invalide'));
 $('trust-audit-summary').textContent=t(v.transaction_verified?'Chaîne de transaction vérifiée':'Chaîne de transaction incomplète')+' · '+t('Événements')+' : '+v.events.length+' · '+t('Simulation locale, sans certification ni immutabilité physique');
 $('trust-audit-report').textContent=JSON.stringify(v,null,2);
}));
$('configuration').addEventListener('submit',event=>{event.preventDefault();guarded(async()=>{const policy={require_separation:$('separation').checked};for(const key of ['mandate_ttl_seconds','unknown_reconcile_seconds','evidence_retention_days'])policy[key]=$('p-'+key).value;const v={schema_version:1,event_id:'config-'+crypto.randomUUID(),expected_revision:state.revision,policy,identities:JSON.parse($('identities').value),reason:$('reason').value};if(roleTouched||state.role_separation)v.role_separation=roleProposal();if(signingTouched)v.evidence_signing={enabled:$('trust-signing-enabled').checked,key_ttl_seconds:$('trust-signing-ttl').value};const raw=encode(v),reply=await api('config-preview',raw);if(reply.result.applied!==false||reply.result.current_revision!==state.revision)throw Error(t('Réponse invalide'));proposal=raw;$('admin-message').textContent=t('Prévisualisation validée ; aucun changement appliqué');});});
$('save').addEventListener('click',()=>guarded(async()=>{if(!proposal)return;const raw=proposal;proposal=null;const reply=await api('config-apply',raw);if(!Number.isSafeInteger(reply.result.revision)||reply.result.revision!==state.revision+1)throw Error(t('Réponse invalide'));$('identities').value='';await load();$('admin-message').textContent=t('Configuration enregistrée et relue');roleSettings();fields('policy',['mandate_ttl_seconds','unknown_reconcile_seconds','evidence_retention_days'],'p-',state.policy);}));
/** Send original UTF-8 textarea bytes under explicit enrollment authentication.
 * No JSON reserialization, role inference, automatic retry or bearer fallback. */
$('trust-enrolled-password').addEventListener('keydown',event=>{if(event.key==='Enter'){event.preventDefault();if(!$('trust-enrolled-execute').disabled)$('trust-enrolled-execute').click();}});
$('trust-enrolled-execute').addEventListener('click',async()=>{
 if(busy)return;busy=true;syncEnrolledAvailability();
 const password=$('trust-enrolled-password');
 // A journaled business refusal can be HTTP200. Preserve the received decision
 // even if the subsequent state refresh fails; never retry an uncertain effect.
 try{const {result}=await api($('trust-enrolled-operation').value,$('trust-enrolled-request').value,password.value);
  const label=result.state==='denied'?'Intention refusée — décision journalisée':result.state==='admitted'?'Intention admise — décision reçue':result.state==='unknown'?'Résultat incertain — vérifier l’état avant reprise':'Réponse reçue';
  const detail=result.state==='admitted'?'Admise':statusLabels[result.state];
  const message=t(label)+(detail?' · '+t(detail):'');$('trust-enrolled-message').textContent=message;
  try{await load();}catch(error){$('trust-enrolled-message').textContent=message+' · '+t('Relecture indisponible ; vérifier l’état avant reprise')+' · '+error.message;}
 }
 catch(error){$('trust-enrolled-message').textContent=error.message;}
 finally{password.value='';busy=false;syncEnrolledAvailability();}
});
$('close').addEventListener('click',()=>$('settings').close());$('settings').addEventListener('close',()=>{proposal=null;$('identities').value='';$('trust-admin-token').value='';$('trust-enrolled-password').value='';$('trust-enrolled-request').value='';if(focus?.isConnected)focus.focus();});
$('language').textContent=globalThis.SwarmI18n?.language==='en'?'Français':'English';$('language').addEventListener('click',()=>{const url=new URL(location.href);url.searchParams.set('lang',globalThis.SwarmI18n?.language==='en'?'fr':'en');location.href=url;});
$('theme').addEventListener('click',()=>{document.documentElement.dataset.theme=document.documentElement.dataset.theme==='etat'?'sombre':'etat';});
fields('fields',shapes[$('operation').value],'f-');
guarded(async()=>{const response=await fetch('/api/v1/native-session',{cache:'no-store'});const session=await response.json();if(!response.ok||session.schema_version!==1||!/^[a-f0-9]{64}$/.test(session.csrf_token))throw Error('web-native-session-invalid');csrf=session.csrf_token;await load();$('message').textContent=t('Prêt — configuration et identités locales requises');});
})();
