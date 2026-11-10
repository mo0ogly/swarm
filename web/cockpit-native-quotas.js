'use strict';
/** Reuse the existing native quota component from the canonical Budget screen.
 * Fixed protected routes, serialized reads/writes, no policy/default/timeout or
 * provider operation. Missing planning stays an explicit unselected mission. */
(()=>{
 /** Require actual quota view shapes before the shared component can make a
  * malformed reply editable. Missing optional limits remain missing/unlimited
  * according to the producer, never filled with local numeric defaults. */
 function view(value){if(!value||!Number.isSafeInteger(value.revision)||value.revision<1)throw Error('quota-view-invalid');for(const key of ['limits','consumed','remaining','bounds']){const fields=value[key];if(!fields||typeof fields!=='object'||Array.isArray(fields))throw Error('quota-view-invalid');for(const required of ['planning_activations','planning_decisions'])if(!Object.hasOwn(fields,required))throw Error('quota-view-invalid');for(const n of Object.values(fields))if(n!==null&&(!Number.isSafeInteger(n)||n<0))throw Error('quota-view-invalid')}
  if(!Array.isArray(value.scopes)||!value.scopes.every(s=>s&&typeof s.id==='string'&&s.id.length>0)||new Set(value.scopes.map(s=>s.id)).size!==value.scopes.length||typeof value.review_manifest_supported!=='boolean'||typeof value.context_window_override_supported!=='boolean')throw Error('quota-view-invalid');if(value.review_timeout_seconds!=null&&(!Number.isSafeInteger(value.review_timeout_seconds)||value.review_timeout_seconds<0))throw Error('quota-view-invalid');return value;
 }
 /** Mount on the existing button; a non-visible host holds only shared dialogs.
  * CSRF stays in this closure. All requests are serialized through completion,
  * including session acquisition; failures release the barrier without retry.
  * Returned shared component has its existing error/read-back semantics. */
 function mount({opener,selectedWork}){
  const host=document.createElement('div');host.id='cockpit-native-quotas';document.body.append(host);let csrf='',barrier=Promise.resolve();
  /** Same-origin fixed route exchange preserving safe codes, original causes,
   * HTTP status and recorded_work through response/parse/output failure. */
  async function send(path,body){let response,value;try{response=await fetch(path,{method:body===undefined?'GET':'POST',credentials:'same-origin',headers:{Accept:'application/json',...(body===undefined?{}:{'Content-Type':'application/json','X-Swarm-CSRF':csrf})},...(body===undefined?{}:{body:JSON.stringify(body)})})}catch(cause){throw new Error('quota-response-unknown',{cause})}
   try{value=await response.json()}catch(cause){const error=new Error('quota-response-invalid',{cause});error.httpStatus=response.status;throw error}if(!response.ok){const raw=value?.failure?.code||value?.error,code=typeof raw==='string'&&/^[a-z][a-z0-9_.-]*$/.test(raw)?raw:'quota-request-refused',error=new Error(code);error.code=code;error.response=value;error.httpStatus=response.status;throw error}return value;
  }
  /** Admit one exact operation/work as query data. A failed read never becomes
   * an empty list or fabricated quota view; preview/apply keep actual native DTOs. */
  async function exchange(operation,work,body){if(!csrf){const session=await send('/api/v1/native-session');if(session?.schema_version!==1||typeof session.csrf_token!=='string'||!/^[0-9a-f]{64}$/.test(session.csrf_token))throw Error('quota-session-invalid');csrf=session.csrf_token}
   if(operation==='list'){const rows=await send('/api/v1/works');if(!Array.isArray(rows)||!rows.every(w=>w&&typeof w.id==='string'&&typeof w.title==='string'&&(w.planning==null||typeof w.planning==='object'&&!Array.isArray(w.planning))))throw Error('quota-work-list-invalid');return rows}
   const route={show:'/api/v1/quotas',preview:'/api/v1/quotas/preview',apply:'/api/v1/quotas/apply'}[operation];if(!route||typeof work!=='string'||!work)throw Error('quota-work-required');const value=await send(route+'?'+new URLSearchParams({work}),body);
   if(operation==='show')return view(value);if(operation==='preview'){if(!value||!Object.hasOwn(value,'settings_revision'))throw Error('quota-preview-invalid');view(value.before);view(value.after);if(value.before.revision!==body.expected_revision||value.after.revision!==body.expected_revision)throw Error('quota-preview-invalid')}return value;
  }
  /** A completion barrier adds no retry, deadline, local cap or policy override. */
  function request(operation,work,body){const previous=barrier;let release;barrier=new Promise(resolve=>{release=resolve});return previous.then(async()=>{try{return await exchange(operation,work,body)}finally{release()}})}
  const component=SwarmNativeQuotas.mount(host,{list:()=>request('list'),show:id=>request('show',id),preview:(id,value)=>request('preview',id,value),apply:(id,value)=>request('apply',id,value)},{translate:s=>globalThis.SwarmI18n?.t(s)??s,opener,selectedWork});opener.disabled=false;return component;
 }
 globalThis.SwarmCockpitQuotas=Object.freeze({mount});
})();
