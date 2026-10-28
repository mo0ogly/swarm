/** Read retained dialogue; enable explicit API/CLI text controls only from saved capabilities. */
import {renderPreparationTurns,PreparationDialogue} from './prephase-dialogue.js';
const tr=text=>globalThis.SwarmI18n?.t(text)??text;
/** Only null/undefined mean an absent optional object. Falsy scalars are invalid,
 * never successful empty data; validate consumes a plain decoded JSON object. */
const optionalObject=(value,validate)=>value===undefined||value===null||(typeof value==='object'&&!Array.isArray(value)&&validate(value));
/** Mount the existing reader and optional explicitly configured API/CLI text editor.
 * Requests are generation-bound to the selected saved preparation. Invalid200,
 * network and engine errors remain visible without erasing them into an empty
 * success. Source prompt fields are forbidden even in an injected DTO. No
 * provider controls are created only when callbacks and saved capabilities allow them;
 * merely reading retained history creates no request, adoption or polling timer. */
export function mountPreparationDialogue(api,callbacks=null){
 const $=id=>document.getElementById(id),section=$('chat'),status=$('chat-status'),recovery=document.createElement('div');let current=null,generation=0,live=null;
 recovery.id='chat-recovery';status.after(recovery);
 $('chat-title').textContent=tr('Historique des échanges IA');
 section.querySelector('[data-context-help="chat"]').hidden=true;
 section.querySelector('.chat-settings').hidden=true;$('chat-form').hidden=true;
 status.before($('chat-refresh'));
 /** Fetch one current saved preparation's history; late responses cannot replace
  * another selection. Refusal keeps an explicit error and enabled retry control. */
 async function load(){
  if(live){if(live.id===current?.id){live.preparation=current;live.refresh();}else{live.setSession(current);}return;}
  if(!current)return;const id=current.id,attempt=++generation;$('chat-refresh').disabled=true;status.className='notice info';status.setAttribute('role','status');status.textContent=tr('Chargement de la conversation…');$('chat-turns').replaceChildren();recovery.replaceChildren();
  try{
   const turns=await api('preparations/dialogue?id='+encodeURIComponent(id));if(attempt!==generation||current?.id!==id)return;
   if(!Array.isArray(turns)||!turns.every(t=>t&&typeof t.id==='string'&&t.preparation_id===id&&['provider','status','created_at','question'].every(k=>typeof t[k]==='string')&&['stale','used_in_brief','used_in_plan'].every(k=>typeof t[k]==='boolean')&&!Object.hasOwn(t,'prompt')&&optionalObject(t.answer,a=>typeof a.message==='string'&&typeof a.brief==='string'&&(a.plan===undefined||typeof a.plan==='string'))&&optionalObject(t.model_route,r=>typeof r.model==='string')&&optionalObject(t.plan_summary,s=>['missions','dependencies','open_questions'].every(k=>Number.isSafeInteger(s[k])))))throw Error(tr('Réponse de conversation incohérente.'));
   renderPreparationTurns($('chat-turns'),turns);recoveryControls(turns,id,attempt);status.textContent=(turns.length?tr('Échanges enregistrés : ')+turns.length:tr('Aucun échange enregistré.'))+' '+tr('Lecture des échanges : aucun appel ni utilisation de proposition. Un arrêt peut être demandé explicitement.');
  }catch(error){if(attempt===generation){status.className='notice alert';status.setAttribute('role','alert');status.textContent=tr('Historique indisponible : ')+(error.code==='preparation-dialogue-active-unported'?tr('Un échange IA est encore actif. Son superviseur reste à raccorder ; aucun arrêt ni redémarrage automatique n’a été effectué.'):error.message);}}
  finally{if(attempt===generation)$('chat-refresh').disabled=false;}
 }
 /** Expose explicit stop separately from model admission. Generation fences
  * selection changes; a receipt is validated before refreshing. No polling,
  * timeout, provider activation or automatic recovery; failed stops stay visible. */
 function recoveryControls(turns,id,attempt){
  for(const turn of turns.filter(t=>['pending','running','stopping'].includes(t.status))){
   const button=document.createElement('button');button.type='button';button.dataset.recoverTurn=turn.id;
   button.textContent=tr('Demander ou vérifier l’arrêt')+' · '+turn.provider;
   button.addEventListener('click',async event=>{if(!event.isTrusted||attempt!==generation||current?.id!==id)return;button.disabled=true;
    try{const request={preparation_id:id,turn_id:turn.id},value=await api('preparations/stop',request);await validateCallReceipt('preparations/stop',request,value);if(attempt===generation&&current?.id===id)await load();}
    catch(error){if(attempt===generation&&current?.id===id){status.className='notice attention';status.setAttribute('role','alert');status.textContent=tr('Arrêt non confirmé : ')+error.message;button.disabled=false;}}
   });recovery.append(button);
  }
  if(recovery.childElementCount){const help=document.createElement('p');help.className='hint';help.textContent=tr('Un arrêt demandé reste en attente du superviseur d’origine. Aucun signal ni redémarrage automatique.');recovery.append(help);}
 }
 $('chat-refresh').addEventListener('click',()=>{if(!live)load();});
 // Historical policy absence keeps the readonly path. A malformed capability
 // never enables sending; its visible diagnostic remains distinct from history.
 if(callbacks)(async()=>{try{const capability=await api('preparations/capabilities');validateCapabilities(capability);if(!capability.enabled&&capability.providers.length===0)return;live=new NativeDialogue(api,callbacks.current,callbacks.apply);live.initial(capability);generation++;recovery.replaceChildren();if(current)live.setSession(current);}catch(error){const notice=document.createElement('p');notice.className='notice attention';notice.textContent=tr('Appels IA indisponibles : ')+error.message;section.append(notice);}})();
 return {
  hasPending(){return !!live?.hasPending();},
  /** Refresh the saved revision or hide the view on a new unsaved preparation.
   * All earlier requests are invalidated; no unsaved text is sent to this read. */
  paint(value){current=value;generation++;section.hidden=!value;if(!value){$('chat-turns').replaceChildren();recovery.replaceChildren();return;}load();}
 };
}

/** Validate capabilities before enabling any control. Missing/null/scalar/falsy
 * values never adopt a hidden policy, timer or successful empty provider list. */
function validateCapabilities(v){
 const p=v?.policy,cap=n=>n===null||(Number.isSafeInteger(n)&&n>=0),text=s=>typeof s==='string'&&s;
 const provider=x=>{
  if(!x||!text(x.name)||typeof x.model!=='string'||typeof x.digest!=='string')return false;
  if(x.levels===undefined)return text(x.model)&&text(x.digest);
  if(!Array.isArray(x.levels)||x.levels.length!==4)return false;
  const names=new Set();
  for(const mode of x.levels){
   if(!mode||!['auto','simple','standard','exigeant'].includes(mode.level)||names.has(mode.level)||typeof mode.available!=='boolean'||!['model','digest','policy_hash'].every(k=>typeof mode[k]==='string'))return false;
   names.add(mode.level);
   if(mode.available){if(!text(mode.model)||!text(mode.digest)||!text(mode.policy_hash))return false;}
   else if(!text(mode.reason)||mode.model||mode.digest||mode.policy_hash)return false;
  }
  const auto=x.levels.find(l=>l.level==='auto');
  return x.levels.some(l=>l.available)&&x.model===auto.model&&x.digest===auto.digest;
 };
 if(typeof v?.enabled!=='boolean'||!p||!['message_max_bytes','context_max_bytes','turn_limit','response_max_bytes'].every(k=>cap(p[k]))||!Number.isSafeInteger(p.refresh_ms)||p.refresh_ms<=0||!['timeout_seconds','timeout_maximum_seconds'].every(k=>Number.isSafeInteger(p[k])&&p[k]>=0)||!['json','sse'].includes(p.transport)||!Array.isArray(v.providers)||(v.unavailable_providers!==undefined&&(!Array.isArray(v.unavailable_providers)||!v.unavailable_providers.every(p=>p&&text(p.name)&&text(p.reason))))||!v.providers.every(provider))throw Error(tr('Réponse des capacités IA incohérente.'));
}
/** Reuse existing send/stop/comparison behavior with current configured provider metadata.
 * Native refresh uses saved policy and runs only while active. Model choices are
 * readonly observations; selecting a level never sends or adopts a proposal. */
class NativeDialogue extends PreparationDialogue{
 /** Validate the admitted receipt before shared send clears its pending intent.
  * A malformed200 remains uncertain and keeps the exact event for replay. */
 constructor(api,current,apply){super(async(path,body)=>{const value=await api(path,body);if(path==='preparations/send'||path==='preparations/stop')await validateCallReceipt(path,body,value);return value;},current,apply);}

 /** Refresh observations while preserving an explicit mode for the same provider.
  * Unavailable auto leaves a deliberate empty choice; no other model is selected.
  * Mode labels are policy-v1 protocol values, with all permission from the engine. */
 initial(v){
  const selected=this.$('chat-provider').value,level=this.$('chat-level').value,same=this.levelProvider===selected;
  this.policy=v.policy;this.enabled=v.enabled;
  this.providers=v.providers.map(p=>({provider:p.name,model:p.model,capability_hash:p.digest,levels:p.levels??[{level:'auto',available:true,model:p.model,digest:p.digest,policy_hash:p.digest}],available:v.enabled,timeout_seconds:v.policy.timeout_seconds}));
  this.$('chat-title').textContent=tr('Conversation IA');this.$('chat-form').hidden=false;document.querySelector('#chat .chat-settings').hidden=false;
  this.$('chat-provider').replaceChildren(new Option(tr('Choisir une IA…'),''));
  for(const p of this.providers){const o=new Option(p.provider,p.provider);o.disabled=!p.available;this.$('chat-provider').add(o);}
  for(const p of v.unavailable_providers??[]){const o=new Option(p.name+' · '+(globalThis.SwarmI18n?.engine(p.reason)??p.reason),p.name);o.disabled=true;this.$('chat-provider').add(o);}
  if(this.providers.some(p=>p.provider===selected))this.$('chat-provider').value=selected;
  const provider=this.providers.find(p=>p.provider===selected),modes=provider?.levels??[];
  this.$('chat-level').replaceChildren();
  if(!modes.find(m=>m.level==='auto')?.available)this.$('chat-level').add(new Option(tr('Choisissez un niveau autorisé.'),''));
  const labels={auto:tr('Modèle configuré'),simple:tr('Simple'),standard:tr('Standard'),exigeant:tr('Exigeant')};
  for(const mode of modes){const o=new Option(labels[mode.level],mode.level);o.disabled=!mode.available;if(!mode.available)o.title=globalThis.SwarmI18n?.engine(mode.reason)??mode.reason;this.$('chat-level').add(o);}
  this.$('chat-level').value=same&&modes.some(m=>m.level===level)?level:modes.some(m=>m.level==='auto'&&m.available)?'auto':'';
  this.levelProvider=selected;
  this.$('chat-scope').textContent=tr('Documents, extraits sélectionnés, méthode et conversation transmis au fournisseur configuré. Une proposition reste à relire.');this.resolveConfigured();
 }
 /** Fence late metadata responses. Current authority remains in the Root send;
  * a pending event keeps its original payload even if capability is now denied. */
 async capability(){const generation=(this.routeGeneration??0)+1;this.routeGeneration=generation;this.routeReady=false;this.controls();try{const value=await this.api('preparations/capabilities');if(generation!==this.routeGeneration)return false;validateCapabilities(value);this.initial(value);this.capabilityError='';return true;}catch(error){if(generation===this.routeGeneration){this.capabilityError=tr('Appels IA indisponibles : ')+error.message;this.note(this.capabilityError,'attention');this.controls();}return false;}}
 /** Bind both public hashes to the explicit mode, never a neighboring model.
  * API retains its fixed route. Unknown/unavailable selection disables new sends. */
 resolveConfigured(){
  const p=this.providers?.find(p=>p.provider===this.$('chat-provider').value),mode=p?.levels.find(m=>m.level===this.$('chat-level').value);
  this.route=mode?.available?{model:mode.model,policy_hash:mode.policy_hash,level:mode.level}:null;
  this.routeReady=!!p?.available&&!!this.route;
  if(p)p.capability_hash=mode?.available?mode.digest:'';
  const auto=p?.levels.find(m=>m.level==='auto');this.$('chat-model').textContent=this.route?this.route.model:mode?.reason?(globalThis.SwarmI18n?.engine(mode.reason)??mode.reason):auto&&!auto.available?tr('Niveau automatique indisponible : ')+(globalThis.SwarmI18n?.engine(auto.reason)??auto.reason):tr('Choisissez un niveau autorisé.');
  this.$('chat-capability').textContent=!this.enabled?tr('Les appels IA doivent être activés explicitement dans Admin.'):tr('Appel texte avec outils demandés désactivés ; restrictions du fournisseur à vérifier. Une annulation locale ne prouve pas l’absence de coût distant.');this.controls();
 }
 /** Freeze choices during an active/pending request. Exact-event replay remains
  * possible after capability withdrawal; Root decides whether it already exists. */
 controls(){super.controls();this.$('chat-stop').disabled=this.busy;const active=this.turns.some(t=>['pending','running','stopping'].includes(t.status));this.$('chat-level').disabled=this.busy||!!this.pending||active||!this.$('chat-provider').value;if(this.pending&&!this.busy&&!active)this.$('chat-send').disabled=false;}
 /** Retry explicit stopping recovery without send authority. No signal, wait
  * or retry loop: Root decides whether the original supervisor is still alive. */
 async stop(){const turn=this.turns.find(t=>['pending','running','stopping'].includes(t.status));if(!turn)return;this.busy=true;this.controls();try{await this.api('preparations/stop',{preparation_id:this.id,turn_id:turn.id});await this.refresh();}catch(error){this.note(tr('Arrêt non confirmé : ')+error.message,'attention');}finally{this.busy=false;this.controls();}}
 async refresh(refreshCapability=true){if(!this.id)return;clearTimeout(this.timer);const id=this.id,generation=this.generation;try{if(refreshCapability)await this.capability();if(id!==this.id||generation!==this.generation)return;const turns=await this.api('preparations/dialogue?id='+encodeURIComponent(id));if(id!==this.id||generation!==this.generation)return;if(!Array.isArray(turns)||!turns.every(t=>t&&t.preparation_id===id&&typeof t.id==='string'&&['provider','status','created_at','question'].every(k=>typeof t[k]==='string')&&['stale','used_in_brief','used_in_plan'].every(k=>typeof t[k]==='boolean')&&!Object.hasOwn(t,'prompt')&&optionalObject(t.answer,a=>typeof a.message==='string'&&typeof a.brief==='string'&&(a.plan===undefined||typeof a.plan==='string'))&&optionalObject(t.model_route,r=>typeof r.model==='string')&&optionalObject(t.plan_summary,s=>['missions','dependencies','open_questions'].every(k=>Number.isSafeInteger(s[k])))))throw Error(tr('Réponse de conversation incohérente.'));this.turns=turns;this.render();const active=turns.some(t=>['pending','running','stopping'].includes(t.status));this.note(this.capabilityError?this.capabilityError:active?tr('L’IA prépare sa réponse. Vous pouvez demander son arrêt.'):tr('Échanges enregistrés : ')+turns.length+' · '+(this.policy.turn_limit===null?tr('Sans plafond'):this.policy.turn_limit));this.controls();if(active)this.timer=scheduleRefresh(this,()=>this.refresh(false),this.policy.refresh_ms,()=>this.id===id&&this.generation===generation);}catch(error){if(id===this.id&&generation===this.generation)this.note(tr('Historique indisponible : ')+error.message,'alert');}}
}

/** Schedule the complete saved positive millisecond interval. Browser timer
 * chunks use the signed32-bit transport representation, without a second policy
 * ceiling or wrap into rapid polling. owner.timer always holds the latest chunk
 * for existing clearTimeout callers; current fences navigation/raced callbacks.
 * Inputs must be a validated safe integer and a live owner; mutates only its timer.
 * The monotonic millisecond observation does not promise real-time scheduling. */
export function scheduleRefresh(owner,callback,delay,current){
 const end=BigInt(Math.floor(performance.now()))+BigInt(delay),maximum=2147483647n;
 function tick(){if(!current())return;const remaining=end-BigInt(Math.floor(performance.now()));if(remaining<=0)callback();else owner.timer=setTimeout(tick,Number(remaining>maximum?maximum:remaining));}
 return setTimeout(tick,Number(BigInt(delay)>maximum?maximum:BigInt(delay)));
}

/** Check the native public turn shape and bind acknowledgements to this exact
 * event (protocol SHA256 ID), or the explicit stopped turn. This proves neither
 * provider execution nor acceptance. Invalid200 throws without an HTTP refusal
 * status so the shared editor preserves its pending intent for exact replay. */
async function validateCallReceipt(path,request,t){
 let expected=request.turn_id;
 if(path==='preparations/send'){const bytes=new TextEncoder().encode(request.preparation_id+'\0'+request.event_id),digest=await crypto.subtle.digest('SHA-256',bytes);expected='pt-'+[...new Uint8Array(digest)].map(b=>b.toString(16).padStart(2,'0')).join('').slice(0,32);}
 if(!t||typeof t!=='object'||Array.isArray(t)||t.id!==expected||t.preparation_id!==request.preparation_id||!Number.isSafeInteger(t.source_revision)||t.source_revision<1||['prompt','_native'].some(k=>Object.hasOwn(t,k))||!['pending','running','stopping','answered','failed','interrupted'].includes(t.status)||['provider','question','created_at','method_hash','request_hash','provider_digest'].some(k=>typeof t[k]!=='string')||!['stale','used_in_brief','used_in_plan'].every(k=>typeof t[k]==='boolean')||!optionalObject(t.answer,a=>typeof a.message==='string'&&typeof a.brief==='string'&&(a.plan===undefined||typeof a.plan==='string'))||(t.status==='answered'&&!t.answer)||(path==='preparations/send'&&(t.provider!==request.provider||t.source_revision!==request.expected_revision))){const error=Error(tr('Réponse d’appel IA incohérente. Vérifiez le même envoi avant de reprendre.'));error.code='preparation-call-response-invalid';throw error;}
}
