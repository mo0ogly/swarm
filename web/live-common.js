'use strict';
// Shared helpers for the read-only live views (live.html, live-session.html).
// Loaded synchronously first so the saved theme applies before first paint.
(function(root){
 const storage={get(k){try{return localStorage.getItem(k)}catch{return null}},set(k,v){try{localStorage.setItem(k,v)}catch{}}};
 const requested=new URLSearchParams(location.search).get('theme');
 const theme=['etat','sombre'].includes(requested)?requested:(['etat','sombre'].includes(storage.get('swarm-theme'))?storage.get('swarm-theme'):'etat');
 document.documentElement.dataset.theme=theme;
 const tr=(s,values)=>root.SwarmI18n?.t(s,values)??(values?Object.entries(values).reduce((t,[k,v])=>t.replaceAll('{'+k+'}',String(v)),s):s);
 /** Toggle and persist the presentation theme; never alters a mission. */
 function toggleTheme(){const next=document.documentElement.dataset.theme==='etat'?'sombre':'etat';document.documentElement.dataset.theme=next;storage.set('swarm-theme',next);return next}
 /** Fetch JSON from a same-origin read-only route. Rejects with {status,code}
  * on HTTP failure or invalid JSON so callers can show the engine's refusal
  * code instead of an empty view. */
 async function getJSON(url){let response;try{response=await fetch(url,{credentials:'same-origin',cache:'no-store',headers:{Accept:'application/json'}})}catch(e){throw {status:0,code:'network'}}
  // Transport refusals (e.g. 503 web-workers-saturated) are plain-text codes.
  const raw=await response.text();let body=null;try{body=JSON.parse(raw)}catch{if(response.ok)throw {status:response.status,code:'invalid-json'};throw {status:response.status,code:/^[a-z0-9-]{1,64}$/.test(raw.trim())?raw.trim():'http-'+response.status}}
  if(!response.ok)throw {status:response.status,code:body?.error||'http-'+response.status};return body}
 /** Remove ANSI/OSC escape sequences from PTY text; output stays plain text
  * rendered via textContent, so no markup from the agent can execute. */
 function stripAnsi(text){return text.replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g,'').replace(/\x1b\[[0-?]*[ -/]*[@-~]/g,'').replace(/\x1b[@-_]/g,'').replace(/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/g,'')}
 const active=new Set(['queued','starting','running','stopping']);
 const statusLabels={queued:'En file',starting:'Démarrage',running:'En cours',stopping:'Arrêt',completed:'Terminé',failed:'Échec',interrupted:'Interrompu',abandoned:'Abandonné'};
 function statusLabel(s){return tr(statusLabels[s]||s||'Inconnu')}
 /** Work task statuses (engine values) as reader labels; unknown values stay raw. */
 const taskLabels={todo:'À faire',running:'En cours',blocked:'Bloquée',review:'En revue',accepted:'Acceptée',waived:'Dispensée'};
 function taskStatus(s){return tr(taskLabels[s]||s||'Inconnu')}
 /** Total input+output tokens and provider-reported cost of a usage record;
  * absent values count as zero and are reported as such, never estimated. */
 function usageOf(u){if(!u)return {tokens:0,cost:0};return {tokens:(Number(u.input_tokens)||0)+(Number(u.output_tokens)||0),cost:Number(u.provider_reported_cost_usd)||0}}
 function ageSeconds(ts){const t=Date.parse(ts);return Number.isFinite(t)?Math.max(0,Math.round((Date.now()-t)/1000)):null}
 function duration(seconds){if(seconds==null)return '—';if(seconds<60)return seconds+' s';if(seconds<3600)return Math.floor(seconds/60)+' min '+String(seconds%60).padStart(2,'0');return Math.floor(seconds/3600)+' h '+String(Math.floor(seconds%3600/60)).padStart(2,'0')}
 function number(n){return new Intl.NumberFormat(root.SwarmI18n?.locale||'fr-FR').format(n)}
 const factLabels={not_reported:'non déclaré',observed:'observé',invalid:'invalide',incomplete:'incomplet'};
 /** Provider usage quality of one agent view: 'observed', another engine state,
  * or 'absent' when no native provider facts exist (legacy/Go-recorded agents). */
 function usageState(agent){return agent.provider_facts?.usage_state||(agent.usage?'observed':'absent')}
 function factLabel(state){return tr(factLabels[state]||(state==='absent'?'non disponible':state))}
 /** Subscribe to /api/v1/live/events. Calls onChange(fingerprint) per pushed
  * change and onMode('push'|'reconnecting'|'polling', reason). One EventSource per
  * browser profile: the window holding the Web Lock `swarm-live-events` owns the
  * stream and relays it over BroadcastChannel, so many live windows never exhaust
  * the HTTP/1.1 per-host connection pool (Chrome: 6). When the owner closes, the
  * lock passes to another window. A `refused`/`closed` event or a permanently
  * closed stream switches every window to polling. Returns {close()}. */
 function liveEvents(onChange,onMode){
  if(typeof EventSource!=='function'){onMode('polling','no-eventsource');return {close(){}}}
  let closed=false,es=null,release=null;const channel=typeof BroadcastChannel==='function'?new BroadcastChannel('swarm-live-events'):null;
  const deliver=m=>{if(closed)return;if(m.kind==='change')onChange(m.fingerprint);else if(m.kind==='mode'){onMode(m.mode,m.reason);if(m.mode==='polling')stop()}};
  const relay=m=>{channel?.postMessage(m);deliver(m)};
  function stop(){if(closed)return;closed=true;es?.close();release?.();channel?.close()}
  function own(){es=new EventSource('/api/v1/live/events');
   es.addEventListener('open',()=>relay({kind:'mode',mode:'push'}));
   es.addEventListener('change',e=>{let fingerprint=null;try{fingerprint=JSON.parse(e.data).fingerprint}catch{}relay({kind:'change',fingerprint})});
   for(const kind of ['refused','closed'])es.addEventListener(kind,e=>{let reason=kind;try{reason=JSON.parse(e.data).error||kind}catch{}relay({kind:'mode',mode:'polling',reason})});
   es.addEventListener('error',()=>{if(closed)return;relay(es.readyState===EventSource.CLOSED?{kind:'mode',mode:'polling',reason:'stream-closed'}:{kind:'mode',mode:'reconnecting'})})}
  if(channel&&navigator.locks){channel.onmessage=e=>deliver(e.data);
   navigator.locks.request('swarm-live-events',()=>closed?null:new Promise(done=>{release=done;own()}));
   onMode('push','shared')}
  else own();
  return {close:stop}}
 function el(tag,props,...children){const e=document.createElement(tag);for(const [k,v]of Object.entries(props||{})){if(v==null||v===false)continue;if(k==='class')e.className=v;else if(k==='text')e.textContent=v;else if(k.startsWith('on'))e.addEventListener(k.slice(2),v);else e.setAttribute(k,v===true?'':v)}for(const c of children)if(c!=null)e.append(c);return e}
 root.SwarmLive={tr,toggleTheme,getJSON,stripAnsi,active,statusLabel,taskStatus,usageOf,usageState,factLabel,liveEvents,ageSeconds,duration,number,el,storage};
})(globalThis);
