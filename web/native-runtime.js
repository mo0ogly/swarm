'use strict';
/** Host the read-only cockpit and shared Admin on the native same-origin HTTP API. The cookie remains
 * HttpOnly; only the dedicated CSRF token lives in this closure. No provider,
 * worker, timeout/retry, operator default or running-policy mutation is started.
 */
(()=>{
 const tr=s=>globalThis.SwarmI18n?.t(s)??s,status=document.getElementById('native-startup-status'),retry=document.getElementById('native-startup-retry'),warning=document.getElementById('native-language-warning');
 let csrf,component,cockpit,starting=false,barrier=Promise.resolve();
 const paths=Object.freeze({session:'/api/v1/native-session',describe:'/api/v1/native-settings/describe',show:'/api/v1/native-settings',preview:'/api/v1/native-settings/preview',apply:'/api/v1/native-settings/apply',budgetShow:'/api/v1/admin-advice-budget',budgetPreview:'/api/v1/admin-advice-budget/preview',budgetApply:'/api/v1/admin-advice-budget/apply',quotaList:'/api/v1/works',missionControls:'/api/v1/mission',missionStatus:'/api/v1/mission?action=status',quotaShow:'/api/v1/quotas',quotaPreview:'/api/v1/quotas/preview',quotaApply:'/api/v1/quotas/apply'});
 /** Keep only a safe family in the public message and retain the original cause.
  * A refusal/invalid response is never a successful empty document or auto retry.
  * Structured response/status and original read/parse causes stay inspectable;
  * known publication uncertainty is visible without dumping raw response bodies. */
 function failure(code,cause,details,httpStatus){const safe=typeof code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(code)?code:'web-native-request-refused';const message=details?.configuration_may_be_recorded===true?'Des réglages peuvent déjà être enregistrés. Vérifiez l’historique avant de reprendre.':'Réponse native non confirmée. Vérifiez l’état avant de reprendre.';const error=new Error(tr(message)+' ('+safe+')',{cause});error.code=safe;if(details!==undefined)error.response=details;if(httpStatus!==undefined)error.httpStatus=httpStatus;return error}
 /** Perform one actual authenticated operation. Body parsing errors retain their
  * cause; non-JSON pre-Root refusals expose only a fixed safe code. Every caller
  * receives rejection. Optional Work names become encoded query data, never
  * another route or authority. No raw response/credential is rendered or logged. */
 async function send(operation,change,work){
  if(!paths[operation])throw failure('web-native-operation-invalid');const url=new URL(paths[operation],location.href);if(work!==undefined)url.searchParams.set('work',work);
  const headers={Accept:'application/json'},options={credentials:'same-origin',headers};
  if(change!==undefined){if(!csrf)throw failure('web-csrf-required');options.method='POST';headers['Content-Type']='application/json';headers['X-Swarm-CSRF']=csrf;options.body=JSON.stringify(change)}
  let response;try{response=await fetch(url,options)}catch(cause){throw failure('web-native-response-unavailable',cause)}
  let raw;try{raw=await response.text()}catch(cause){throw failure('web-native-response-unavailable',cause)}
  if(!response.headers.get('Content-Type')?.startsWith('application/json'))throw failure(/^web-[a-z0-9-]+$/.test(raw)?raw:'web-native-response-invalid',undefined,{content_type:response.headers.get('Content-Type'),body:raw},response.status);
  let value;try{value=JSON.parse(raw)}catch(cause){throw failure('web-native-response-invalid',cause,{content_type:response.headers.get('Content-Type'),body:raw},response.status)}
  if(!response.ok)throw failure(typeof value?.failure?.code==='string'?value.failure.code:typeof value?.error==='string'?value.error:'web-native-request-refused',undefined,value,response.status);
  return value;
 }
 /** Serialize this component's requests, including its initial Promise.all reads,
  * through completion barriers, without swallowing a caller's error. The saved
  * server Admin capacity stays authoritative. Failure releases the next caller;
  * no repeated operation, hidden queue cap or timeout is added. */
 function request(operation,change,work){const previous=barrier;let release;barrier=new Promise(resolve=>{release=resolve});return previous.then(async()=>{try{return await send(operation,change,work)}finally{release()}})}
 /** Bootstrap a protected dedicated CSRF and mount the existing component once.
  * Terminal errors stay visible with an explicit reload. Navigation/unload does
  * not claim to cancel an already admitted Root publication; history owns replay. */
 async function start(){if(starting||component)return;starting=true;retry.hidden=true;status.hidden=false;status.className='notice info';status.textContent=tr('Chargement…');
  try{const session=await request('session');if(session?.schema_version!==1||session.csrf_format!=='hmac-sha256-v1'||typeof session.csrf_token!=='string'||!/^[0-9a-f]{64}$/.test(session.csrf_token))throw failure('web-native-session-invalid');csrf=session.csrf_token;
   if(!cockpit)cockpit=SwarmNativeCockpit.mount(document.getElementById('native-cockpit'),{list:()=>request('quotaList'),controls:work=>request('missionControls',undefined,work),status:work=>request('missionStatus',undefined,work)},{translate:tr});
   component=SwarmEngineSettingsAdmin.mount(document.getElementById('native-settings-admin'),{describe:()=>request('describe'),show:()=>request('show'),preview:change=>request('preview',change),apply:change=>request('apply',change),budget:{show:()=>request('budgetShow'),preview:change=>request('budgetPreview',change),apply:change=>request('budgetApply',change)}},{translate:tr});SwarmNativeQuotas.mount(document.getElementById('native-quotas-admin'),{list:()=>request('quotaList'),show:work=>request('quotaShow',undefined,work),preview:(work,change)=>request('quotaPreview',change,work),apply:(work,change)=>request('quotaApply',change,work)},{translate:tr});status.hidden=true;
  }catch(error){status.className='notice alert';status.textContent=tr('Réponse native non confirmée. Vérifiez l’état avant de reprendre.')+' ('+(typeof error.code==='string'?error.code:'web-native-startup-failed')+')';retry.hidden=false}finally{starting=false}}
 /** Switch presentation only. Supported views use stable URL query values;
  * changing view never sends a command or creates mission consent. Keyboard
  * activation focuses the newly visible heading; dialogs keep their own focus. */
 function showView(name,focus=false){const view=name==='admin'?'admin':'missions';document.getElementById('native-admin-view').hidden=view!=='admin';document.getElementById('native-missions-view').hidden=view!=='missions';document.querySelectorAll('[data-native-view]').forEach(button=>{if(button.dataset.nativeView===view)button.setAttribute('aria-current','page');else button.removeAttribute('aria-current')});if(focus)document.getElementById(view==='admin'?'native-settings-title':'native-missions-title').focus()}
 document.querySelectorAll('[data-native-view]').forEach(button=>button.addEventListener('click',()=>{const url=new URL(location.href);url.searchParams.set('view',button.dataset.nativeView);history.pushState(null,'',url);showView(button.dataset.nativeView,true)}));
 addEventListener('popstate',()=>showView(new URL(location.href).searchParams.get('view')));
 showView(new URL(location.href).searchParams.get('view'));
 /** Carry only supported presentation preferences to the existing Live page;
  * no mission selection, agent identity or authorization is inferred. */
 function livePreferences(){const link=document.getElementById('native-live-link'),url=new URL('/live.html',location.href);url.searchParams.set('lang',globalThis.SwarmI18n?.language==='en'?'en':'fr');url.searchParams.set('theme',document.documentElement.dataset.theme==='etat'?'etat':'sombre');link.href=url.pathname+url.search}
 livePreferences();
 retry.addEventListener('click',start);
 document.querySelectorAll('[data-theme-choice]').forEach(button=>button.addEventListener('click',()=>{const theme=button.dataset.themeChoice;if(!['etat','sombre'].includes(theme))return;document.documentElement.dataset.theme=theme;const url=new URL(location.href);url.searchParams.set('theme',theme);history.replaceState(null,'',url);livePreferences()}));
 addEventListener('swarm:language-change',event=>{if(document.querySelector('dialog[data-engine-settings][open],dialog[data-native-quotas][open]')){event.preventDefault();warning.textContent=tr('Enregistrez ou terminez vos modifications avant de changer de langue.');warning.hidden=false}});
 // Resource completion precedes the first API read; no hidden timing assumption.
 if(document.readyState==='complete')start();else addEventListener('load',start,{once:true});
})();
