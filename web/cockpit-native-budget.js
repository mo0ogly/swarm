'use strict';
/** Connect the existing mission-budget dialog to native financial validation and
 * CAS writes. No launch, consumption reset, automatic retry or price default.
 * Only explicit gestures post; uncertain publication requires a fresh read. */
(()=>{
 const tr=s=>globalThis.SwarmI18n?.t(s)??s,$=id=>document.getElementById(id);
 /** Same-origin fixed budget endpoints. Original IO/codec causes and structured
  * known publication metadata stay on errors; only safe codes reach the UI.
  * A failed apply never establishes unchanged state or triggers a retry. */
 async function request(path,body,csrf){let response,value;try{response=await fetch(path,{method:body===undefined?'GET':'POST',credentials:'same-origin',headers:{Accept:'application/json',...(body===undefined?{}:{'Content-Type':'application/json','X-Swarm-CSRF':csrf})},...(body===undefined?{}:{body:JSON.stringify(body)})})}catch(cause){throw new Error('web-native-budget-response-unknown',{cause})}
  try{value=await response.json()}catch(cause){const error=new Error('web-native-budget-response-invalid',{cause});error.httpStatus=response.status;throw error}
  if(!response.ok){const raw=value?.failure?.code||value?.error,code=typeof raw==='string'&&/^[a-z][a-z0-9_.-]*$/.test(raw)?raw:'web-native-budget-refused';const error=new Error(code);error.response=value;error.httpStatus=response.status;throw error}return value;
 }
 /** Validate exactly the displayed financial/provenance values. No business
  * policy is introduced: the engine remains the monetary/date authority. */
 function budget(value){if(value===null)return;for(const key of ['limit_usd','reserve_per_launch_usd'])if(typeof value?.[key]!=='number'||!Number.isFinite(value[key])||value[key]<0)throw Error('web-native-budget-invalid');for(const key of ['estimate_source','reference_date'])if(typeof value[key]!=='string')throw Error('web-native-budget-invalid')}
 /** Require a bound explicit Work/revision and nullable policy before display.
  * Unknown amounts remain unknown. Throws on malformed or cross-Work replies. */
 function view(value,id){if(value?.schema_version!==1||value.work_id!==id||!Number.isSafeInteger(value.revision)||value.revision<1||!Object.hasOwn(value,'budget')||!Object.hasOwn(value,'state'))throw Error('web-native-budget-view-invalid');budget(value.budget);if(value.state!==null){if(value.state?.work_id!==id)throw Error('web-native-budget-view-invalid');for(const key of ['committed_usd','reported_usd'])if(typeof value.state[key]!=='number'||!Number.isFinite(value.state[key])||value.state[key]<0)throw Error('web-native-budget-view-invalid')}return value}
 /** Compare caller-supplied intent only; actor/time are engine-owned metadata. */
 function same(a,b){return a!==null&&b!==null&&['limit_usd','reserve_per_launch_usd','estimate_source','reference_date'].every(k=>a[k]===b[k])}
 /** Mount once on canonical modal/fields/buttons, preserving title, keyboard
  * dismissal and focus restoration. API errors stay visible with form values;
  * closing during a request is prevented until its outcome is delivered.
  * reload must reread the existing cockpit after an acknowledged/re-read write.
  * Returned setWork never supplies a default mission or starts an operation. */
 function mount({reload}){
  const dialog=$('modal'),form=$('action-form'),entry=$('budget-edit'),confirm=$('confirm'),fields=$('modal-fields'),errorBox=$('modal-error'),comparison=$('preview');
  let selected='',target='',current=null,proposal=null,csrf='',busy=false,uncertain=false,returnFocus=null;const inputs=new Map();
  /** Reflect only observed state; unknown apply outcomes cannot be resubmitted
   * from this form. A fresh open reads the authoritative state before a new edit. */
  function controls(){entry.disabled=!selected||busy;confirm.disabled=busy||uncertain||!current;for(const input of inputs.values())input.disabled=busy||uncertain;$('close').disabled=busy;$('cancel').disabled=busy}
  /** Reset approval on any edit; preview never authorizes a later changed intent. */
  function invalidate(){proposal=null;comparison.hidden=true;confirm.textContent=tr('Prévisualiser le budget')}
  /** Keep safe diagnostic and original field data. Apply failure is explicitly
   * uncertain when the response was lost after a successful transaction. Known
   * 400/403/409 refusals are distinguished; every failed apply still requires a new read. */
  function failed(cause,writing){if(writing)uncertain=true;proposal=null;comparison.hidden=true;errorBox.hidden=false;const code=/^[a-z][a-z0-9_.-]*$/.test(cause.message)?cause.message:'web-native-budget-failed';errorBox.textContent=tr(writing&&!cause.response?.recorded_work&&[400,403,409].includes(cause.httpStatus)?'Enregistrement refusé. Fermez puis rouvrez le réglage pour relire la révision avant de réessayer.':writing&&cause.response?.recorded_work?'Une modification est signalée comme enregistrée, mais sa relecture n’est pas confirmée. Fermez puis rouvrez le réglage.':writing?'La sauvegarde peut déjà être enregistrée. Fermez puis rouvrez le réglage pour relire l’état avant toute nouvelle modification.':'Lecture ou prévisualisation refusée. Corrigez les valeurs ou rouvrez le réglage.')+' ('+code+')';errorBox.focus()}
  /** Reuse canonical field container. Null policy leaves inputs blank rather
   * than introducing an implicit budget/reserve/provenance/default date. */
  function fill(value){fields.replaceChildren();inputs.clear();for(const [key,label] of [['limit_usd','Plafond USD — 0 signifie illimité'],['reserve_per_launch_usd','Réservation estimée par départ, USD'],['estimate_source','Source de l’estimation'],['reference_date','Date de référence — AAAA-MM-JJ']]){const holder=document.createElement('label'),input=document.createElement('input');input.id='native-budget-'+key;input.name=key;holder.htmlFor=input.id;holder.textContent=tr(label);input.value=value?.[key]??'';if(key.endsWith('_usd')){input.type='number';input.step='any';input.min='0';input.required=true}else input.type='text';input.addEventListener('input',()=>{invalidate();errorBox.hidden=true});inputs.set(key,input);holder.append(input);fields.append(holder)}}
  /** Build one explicit financial intent, with a new event only after a healthy
   * read/edit. Invalid/blank JS numbers refuse without POST; other rules are server-owned. */
  function prepare(){const values={};for(const [key,input] of inputs){if(key.endsWith('_usd')){if(!input.value.trim()||!Number.isFinite(Number(input.value))||Number(input.value)<0)throw Error('budget-nonnegative-finite-amounts-required');values[key]=Number(input.value)}else values[key]=input.value}return {schema_version:1,event_id:'budget-'+crypto.randomUUID(),expected_revision:current.revision,budget:values}}
  /** Show actual before/proposed policy and recorded estimates without deriving
   * available funds or a launch permission. Null data is never shown as zero. */
  function preview(value,intent){if(value?.schema_version!==1||value.work_id!==target||value.revision!==current.revision||!Object.hasOwn(value,'current')||!Object.hasOwn(value,'state'))throw Error('web-native-budget-preview-invalid');budget(value.current);budget(value.proposed);if(!same(value.proposed,intent.budget))throw Error('web-native-budget-preview-invalid');view({...value,budget:value.current},target);const lines=[tr('Avant / proposé :')];for(const [key,label] of [['limit_usd','Plafond (USD)'],['reserve_per_launch_usd','Réserve par lancement (USD)'],['estimate_source','Source de l’estimation'],['reference_date','Date de référence — AAAA-MM-JJ']])lines.push(tr(label)+' : '+(value.current===null?tr('Non renseigné'):String(value.current[key]))+' → '+String(value.proposed[key]));lines.push(tr('Prévisualisation financière uniquement. Les réservations et consommations sont conservées ; les autres conditions de lancement restent applicables.'));comparison.textContent=lines.join('\n');comparison.hidden=false;proposal=intent;confirm.textContent=tr('Enregistrer ce budget')}
  /** Explicit open fetches current state and CSRF; no user content/credential
   * is persisted in preferences. Failed reads leave saving disabled. */
  async function open(){target=selected;if(!target||busy)return;returnFocus=document.activeElement;current=null;uncertain=false;errorBox.hidden=true;invalidate();fields.replaceChildren();inputs.clear();$('modal-title').textContent=tr('Régler le budget');$('modal-context').textContent=target;$('modal-description').textContent=tr('Budget estimatif non garanti. Les agents actifs continuent ; les nouveaux départs sont contrôlés.');$('modal-help-toggle').hidden=true;$('modal-help').hidden=true;$('modal-rich').hidden=true;$('modal-steps').hidden=true;dialog.showModal();busy=true;controls();try{const session=await request('/api/v1/native-session');if(session?.schema_version!==1||typeof session.csrf_token!=='string'||!/^[0-9a-f]{64}$/.test(session.csrf_token))throw Error('web-native-session-invalid');csrf=session.csrf_token;current=view(await request('/api/v1/work-budget?'+new URLSearchParams({work:target})),target);fill(current.budget);$('modal-description').textContent=tr(current.budget===null?'Aucun plafond enregistré : le budget est illimité. Saisissez explicitement les valeurs souhaitées.':'Budget estimatif non garanti. Les agents actifs continuent ; les nouveaux départs sont contrôlés.')}catch(cause){failed(cause,false)}finally{busy=false;controls();globalThis.SwarmShell?.focusWindowStart(dialog)}}
  /** Preview is a separate gesture before applying the exact retained document.
   * An acknowledged write is reread; a lost/malformed response is uncertain and
   * never automatically replayed. Follow-up read failures preserve that uncertainty. */
  async function submit(event){event.preventDefault();if(busy||uncertain||!current)return;const writing=proposal!==null;let acknowledged=null;busy=true;errorBox.hidden=true;controls();try{if(!writing){const intent=prepare();preview(await request('/api/v1/work-budget/preview?'+new URLSearchParams({work:target}),intent,csrf),intent)}else{const intent=proposal;proposal=null;const result=await request('/api/v1/work-budget/apply?'+new URLSearchParams({work:target}),intent,csrf);if(result?.id!==target||!Number.isSafeInteger(result.revision)||result.revision!==intent.expected_revision+1)throw Error('web-native-budget-apply-response-invalid');acknowledged={id:target,revision:result.revision};const observed=view(await request('/api/v1/work-budget?'+new URLSearchParams({work:target})),target);if(!same(observed.budget,intent.budget)||observed.revision<result.revision)throw Error('web-native-budget-post-apply-state-changed');await reload();busy=false;controls();dialog.close();$('message').hidden=false;$('message').className='notice info';$('message').textContent=tr('Budget enregistré et relu. Réservations et consommations conservées.')}}catch(cause){if(acknowledged)cause.response={...cause.response,recorded_work:acknowledged};failed(cause,writing)}finally{busy=false;controls()}}
  entry.addEventListener('click',open);form.addEventListener('submit',submit);for(const id of ['close','cancel'])$(id).addEventListener('click',()=>{if(!busy)dialog.close()});dialog.addEventListener('cancel',event=>{if(busy)event.preventDefault()});dialog.addEventListener('close',()=>{(returnFocus?.isConnected?returnFocus:entry).focus()});controls();
  return {setWork(id){selected=id;if(dialog.open&&target!==id&&!busy)dialog.close();controls()}};
 }
 globalThis.SwarmNativeWorkBudget=Object.freeze({mount});
})();
