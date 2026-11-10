'use strict';
/** Native hooks for the existing pricing renderer. Reuses a cloned canonical
 * modal structure with disjoint IDs; does not borrow the mission-budget form.
 * No provider, price lookup, polling, retry or mission policy is introduced. */
(()=>{
 const tr=s=>globalThis.SwarmI18n?.t(s)??s,original=id=>document.getElementById(id),dialog=original('modal').cloneNode(true),ids=new Map();
 for(const node of [dialog,...dialog.querySelectorAll('[id]')]){ids.set(node.id,node);node.id='pricing-'+node.id}
 for(const node of [dialog,...dialog.querySelectorAll('[aria-labelledby],[aria-describedby],[aria-controls]')])for(const key of ['aria-labelledby','aria-describedby','aria-controls'])if(node.hasAttribute(key))node.setAttribute(key,node.getAttribute(key).split(' ').map(id=>'pricing-'+id).join(' '));
 document.body.append(dialog);const $=id=>ids.get(id)||original(id);let context=null,focus=null,busy=false,uncertain=false,csrf='',queue=Promise.resolve();
 const priceKeys=['input_per_million','output_per_million','cache_read_per_million','cache_write_per_million'];
 /** Validate displayed actual DTOs, including legal null unknown prices/rates.
  * Number checks are browser representation, never monetary/token policy caps. */
 function rate(value){if(!value)throw Error('pricing-native-rate-invalid');for(const key of ['version','provider','model','currency','source','reference_date','actor','updated'])if(typeof value[key]!=='string')throw Error('pricing-native-rate-invalid');if(!value.version)throw Error('pricing-native-rate-invalid');for(const key of priceKeys)if(value[key]!==null&&(typeof value[key]!=='number'||!Number.isFinite(value[key])||value[key]<0))throw Error('pricing-native-rate-invalid');return value}
 /** Reject malformed success without hiding it as an empty catalogue/quote. */
 function validate(path,body,value){if(path==='/api/v1/pricing'&&body===undefined){if(value?.schema_version!==1||typeof value.digest!=='string'||!/^[a-f0-9]{64}$/.test(value.digest)||!(value.rates===null||Array.isArray(value.rates)))throw Error('pricing-native-catalogue-invalid');const versions=new Set();for(const r of value.rates??[]){rate(r);if(versions.has(r.version))throw Error('pricing-native-catalogue-invalid');versions.add(r.version)}}else if(path==='/api/v1/pricing'){rate(value)}else{if(!['estimated','unknown'].includes(value?.status)||!Array.isArray(value.missing_rates)||!value.missing_rates.every(k=>typeof k==='string'))throw Error('pricing-native-quote-invalid');rate(value.rate);if(value.rate.version!==body.rate_version||!value.tokens||!Object.keys(body).every(k=>value.tokens[k]===body[k]))throw Error('pricing-native-quote-invalid');if(value.status==='estimated'?(typeof value.estimated_amount!=='number'||!Number.isFinite(value.estimated_amount)||value.estimated_amount<0||value.missing_rates.length!==0):(value.estimated_amount!==null||value.missing_rates.length===0))throw Error('pricing-native-quote-invalid')}return value}
 /** Keep cause/status/structured publication evidence on errors; show only safe
  * codes. A failed save means a fresh explicit read, never automatic replay. */
 async function send(path,body){let response,value;try{response=await fetch(path,{method:body===undefined?'GET':'POST',credentials:'same-origin',headers:{Accept:'application/json',...(body===undefined?{}:{'Content-Type':'application/json','X-Swarm-CSRF':csrf})},...(body===undefined?{}:{body:JSON.stringify(body)})})}catch(cause){throw new Error('pricing-native-response-unavailable',{cause})}try{value=await response.json()}catch(cause){const error=new Error('pricing-native-response-invalid',{cause});error.httpStatus=response.status;throw error}if(!response.ok){const code=value?.failure?.code||value?.error,error=new Error(typeof code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(code)?code:'pricing-native-refused');error.response=value;error.httpStatus=response.status;throw error}return value}
 /** Serialize this dialog's exchanges with completion, no cap or timer. Fetch a
  * fresh protected CSRF on explicit POST; session failure precedes publication. */
 async function api(path,body){if(!['/api/v1/pricing','/api/v1/pricing/estimate'].includes(path))throw Error('pricing-native-operation-invalid');const previous=queue;let release;queue=new Promise(resolve=>{release=resolve});await previous;try{if(body!==undefined){const session=await send('/api/v1/native-session');if(session?.schema_version!==1||session.csrf_format!=='hmac-sha256-v1'||typeof session.csrf_token!=='string'||!/^[a-f0-9]{64}$/.test(session.csrf_token))throw Error('pricing-native-session-invalid');csrf=session.csrf_token}return validate(path,body,await send(path,body))}finally{release()}}
 /** Safe shared elements; text is never HTML and actions stay local listeners. */
 function node(tag,text,cls){const n=document.createElement(tag);if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n}
 function button(text,action){const n=node('button',text);n.type='button';n.addEventListener('click',()=>{if(!busy&&!uncertain)action()});return n}
 /** Unique labels preserve the existing pricing field order and styling. */
 function field(name,label,value=''){const holder=node('label',label),input=node('input');input.name=name;input.id='pricing-field-'+name;input.value=value;holder.htmlFor=input.id;holder.append(input);$('modal-fields').append(holder);return input}
 /** Reuse canonical modal with caller-owned context; all transitions are local.
  * No close/reopen while a request is pending; fresh open clears uncertainty. */
 function openModal(title,description,next){context=next;uncertain=false;$('modal-title').textContent=title;$('modal-description').textContent=description;$('modal-context').textContent='SWARM';$('modal-fields').replaceChildren();for(const id of ['preview','modal-error','modal-help-toggle','modal-help','modal-rich','modal-steps'])$(id).hidden=true;$('confirm').hidden=false;$('confirm').disabled=false;$('confirm').textContent=tr('Confirmer');if(!dialog.open)dialog.showModal()}
 /** Toggle actual pending state only; no time bound or permission inferred. */
 function controls(){for(const el of dialog.querySelectorAll('button,input,textarea'))el.disabled=busy||uncertain;if(!busy)for(const id of ['close','cancel'])$(id).disabled=false;for(const id of ['pricing-open','admin-pricing-open'])original(id).disabled=busy}
 /** Render a safe actionable result. Unknown save outcome requires reopening;
  * original cause remains on the caught error, never exposed as private content. */
 function failed(error,writing){if(writing)uncertain=true;const code=/^[a-z][a-z0-9_.-]*$/.test(error.message)?error.message:'pricing-native-failed';$('modal-error').hidden=false;$('modal-error').textContent=tr(writing?'La sauvegarde peut déjà être enregistrée. Fermez puis rouvrez le réglage pour relire l’état avant toute nouvelle modification.':'Lecture ou prévisualisation refusée. Corrigez les valeurs ou rouvrez le réglage.')+' ('+code+')';$('modal-error').focus()}
 globalThis.SwarmNativePricingHooks={api,$,openModal,button,node,field,preview:text=>{$('preview').textContent=text;$('preview').hidden=false},closeModal:dismiss,notice:(text,error)=>{if(error){openModal(tr('Tarifs des modèles'),'',{action:'pricing-read'});failed(new Error(text),false);$('confirm').hidden=true}else{original('message').textContent=text;original('message').hidden=false;original('message').className='notice info'}},getContext:()=>context,setReturnFocus:value=>{focus=value}};
 /** Close and restore focus synchronously, including Escape. A queued close
  * event cannot clear the context of a subsequently reopened dialog. */
 function dismiss(){dialog.close();context=null;(focus?.isConnected?focus:original('pricing-open')).focus()}
 /** Bind only once after the legacy renderer attaches its entry controls.
  * Capture explicit opener activity through a gate to avoid concurrent opens. */
 function enable(){for(const id of ['pricing-open','admin-pricing-open']){const entry=original(id);entry.disabled=false;entry.onclick=async()=>{if(busy)return;focus=entry;uncertain=false;busy=true;controls();try{await Pricing.open(entry)}finally{busy=false;controls();globalThis.SwarmShell?.focusWindowStart(dialog)}}}
 $('action-form').addEventListener('submit',async event=>{event.preventDefault();if(busy||uncertain||!context)return;const c=context,writing=c.action==='pricing-save',values=Object.fromEntries(new FormData($('action-form')));busy=true;$('modal-error').hidden=true;controls();try{await Pricing.submit(c,values)}catch(error){failed(error,writing)}finally{busy=false;controls()}});
 for(const id of ['close','cancel'])$(id).addEventListener('click',()=>{if(!busy)dismiss()});dialog.addEventListener('cancel',event=>{event.preventDefault();if(!busy)dismiss()});dialog.addEventListener('close',()=>{if(!dialog.open){context=null;(focus?.isConnected?focus:original('pricing-open')).focus()}});}
 globalThis.SwarmNativePricing=Object.freeze({enable});
})();
