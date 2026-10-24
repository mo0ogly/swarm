'use strict';
// Harness window: follows one agent through /api/v1/live/agent cursors, woken by
// pushed changes (/api/v1/live/events) or by polling when push is unavailable.
// Read-only — no input, lease or stop control is offered here.
(function(){
 const L=globalThis.SwarmLive,M=globalThis.SwarmLiveMetrics,{tr,el}=L,$=id=>document.getElementById(id);
 const q=new URLSearchParams(location.search),agentId=q.get('agent')||'',workId=q.get('work')||'',runId=q.get('run')||'';
 // Polling cadence when push is unavailable: 500 ms while active (Go terminal
 // page polls at 350 ms). A full page is re-read immediately in both modes.
 const ACTIVE_MS=500;
 // While the agent is active and its processes are verified, /proc numbers
 // change without any recorded event: re-read every RESOURCE_MS (display cadence).
 // HISTORY bounds the in-page samples kept for the RAM/CPU/token sparklines.
 const RESOURCE_MS=3000,HISTORY=60;
 const rates=new M.Rates(),series={ram:[],cpu:[],tok:[]},events=[],telCalls=[],svgNS='http://www.w3.org/2000/svg';
 let resTimer=null;
 let logsAfter=0,termAfter=0,timer=null,stopped=false,inflight=false,again=false,mode='connecting',stream=null,xterm=null;
 const term=$('h-terminal'),log=$('h-log'),follow=$('h-follow');
 // Read-only xterm when its bundle loaded; the stripped <pre> stays as fallback.
 try{if(globalThis.SwarmLiveTerminal){xterm=SwarmLiveTerminal.create($('h-xterm'));$('h-xterm').hidden=false;term.hidden=true}}catch{xterm=null}

 function facts(a){const u=L.usageOf(a.usage),p=a.progress||{},f=a.provider_facts,state=L.usageState(a);const rows=[
   ['État',L.statusLabel(a.status)+(a.desired?' → '+a.desired:'')],['Rôle',a.role],['Fournisseur',[a.provider,a.reported_model?.model].filter(Boolean).join(' · ')],
   ['Activité',a.activity],['Tâche',a.task_id],['Mission',a.work_id],['Durée',L.duration(M.activeSeconds(a))],
   ['Dernier signal',a.heartbeat?L.duration(L.ageSeconds(a.heartbeat))+' '+tr('écoulées'):'—'],
   ['Jetons',state==='observed'?L.number(u.tokens):'—'],
   ['Usage déclaré',L.factLabel(state)+(f?.invalid_usage_events?' · '+f.invalid_usage_events+' '+tr('déclarations invalides'):'')+(f?.first_usage_error?' · '+f.first_usage_error:'')],
   ['Modèle déclaré',f?.model_state?L.factLabel(f.model_state)+(f.first_model_error?' · '+f.first_model_error:''):''],
   ['Diagnostic',a.diagnostic?.summary||(a.diagnostic?.observed_errors?a.diagnostic.observed_errors+' '+tr('erreurs observées'):'')]];
  const since=a.active?Date.parse(a.started):NaN;
  $('h-facts').replaceChildren(...rows.filter(([,v])=>v).flatMap(([k,v])=>[el('dt',{text:tr(k)}),el('dd',{class:k==='État'?'status-'+a.status:(k==='Usage déclaré'&&state!=='observed'?'live-warn':null),text:String(v),'data-since':k==='Durée'&&Number.isFinite(since)?String(since):null})]));
  $('h-title').textContent=(a.role||tr('Agent'))+' · '+a.id;document.title=(a.active?'▶ ':'■ ')+(a.role||a.id)+' · '+L.statusLabel(a.status)}

 const procLabels={observed:'observé','not-recorded':'non enregistré',ended:'terminé','identity-mismatch':'identité non concordante',unavailable:'indisponible','not-read':'non lu'};
 function procState(s){return tr(procLabels[s]||s)}
 function push(list,v){if(v==null)return;list.push(v);while(list.length>HISTORY)list.shift()}
 /** One-series sparkline (no dual axis); values are also written as text.
  * Rolling samples fill from the right over HISTORY slots; `full` spreads a
  * complete short series (e.g. per-call tokens) over the whole width. */
 function spark(values,cls,label,full){const svg=document.createElementNS(svgNS,'svg');svg.setAttribute('viewBox','0 0 200 36');svg.setAttribute('preserveAspectRatio','none');svg.setAttribute('class','live-minispark '+cls);svg.setAttribute('role','img');svg.setAttribute('aria-label',label);
  if(values.length>1){const max=Math.max(...values,1e-9),slots=full?values.length:HISTORY,step=200/(slots-1),off=(slots-values.length)*step;const pl=document.createElementNS(svgNS,'polyline');pl.setAttribute('points',values.map((v,i)=>(off+i*step)+','+(34-v/max*30)).join(' '));svg.append(pl)}return svg}
 function figure(title,value,svg){return el('div',{class:'live-mini'},el('span',{class:'live-mini-title',text:title}),el('strong',{text:value}),svg)}

 function resources(a){const r=M.resources(a),box=$('h-res');
  if(!a.resources){box.replaceChildren(el('p',{class:'live-empty',text:tr('Non disponible sur cette plateforme.')}));return false}
  if(a.resources.observation!=='observed'){box.replaceChildren(el('p',{class:'live-empty',text:a.resources.observation==='other-host'?tr('Agent exécuté sur un autre hôte : processus non lus.'):tr('Ressources indisponibles')+(a.resources.error?' ('+a.resources.error+')':'')}));return false}
  const states=el('p',{class:'live-res-states'},el('span',{text:tr('superviseur')+' : '+procState(a.resources.supervisor.state)}),el('span',{text:tr('fournisseur')+' : '+procState(a.resources.provider.state)}));
  if(!r||r.rss==null){box.replaceChildren(states);return false}
  const cpu=rates.rate(a.id,r.cpu,r.at);push(series.ram,r.rss);push(series.cpu,cpu);
  box.replaceChildren(figure('RAM',M.bytes(r.rss),spark(series.ram,'is-s1',tr('RAM dans le temps'))),figure(tr('CPU (% d’un cœur)'),cpu==null?'…':cpu.toFixed(1)+' %',spark(series.cpu,'is-s3',tr('CPU dans le temps'))),el('p',{class:'live-res-states'},el('span',{text:(r.threads??'—')+' '+tr('threads')}),el('span',{text:(r.descendants||0)+' '+tr('sous-processus')})),states);
  return a.active}

 function tools(a){const p=a.progress||{},c=a.context;
  $('h-tools').replaceChildren(
   M.meter(tr('appels d’outils'),p.tool_calls,a.limits?.max_tool_calls),
   el('dl',{class:'live-facts'},...[['Résultats',p.tool_results],['En attente',p.pending_tools],['Dernier outil',p.last_tool],['Action',p.action],['Détail',p.detail],['Attente outil',p.tool_wait_seconds?L.duration(p.tool_wait_seconds):''],
    ['Contexte livré',c?M.bytes(c.bytes)+' · '+c.included+' '+tr('fichiers')+(c.excluded?' · '+c.excluded+' '+tr('exclus'):''):''],
    ['Mémoires',(a.context_memory||[]).map(m=>m.id+' v'+m.version).join(', ')]].filter(([,v])=>v!=null&&v!==''&&v!==0).flatMap(([k,v])=>[el('dt',{text:tr(k)}),el('dd',{text:String(v)})])))}

 /** Cumulative usage sampled at each refresh: shows observed progression
  * between refreshes, not per-call figures (the engine records totals only). */
 function tokensPanel(a){const t=M.tokens(a.usage);if(!t||L.usageState(a)!=='observed'){$('h-tok').replaceChildren(el('p',{class:'live-empty',text:tr('Usage')+' '+L.factLabel(L.usageState(a))}));return}
  push(series.tok,t.input+t.output);
  $('h-tok').replaceChildren(figure(tr('Total'),L.number(t.input+t.output)+(t.cost?' · '+M.money(t.cost):''),spark(series.tok,'is-s2',tr('Jetons cumulés observés'))),M.tokenBar(t),M.tokenLegend(t),el('p',{class:'live-res-states',text:tr('Progression observée entre rafraîchissements, pas par appel.')}))}

 /** The task as planned in the initial phase versus this attempt's reality.
  * Planned limits of 0 mean "not set by the plan" and show as a dash. */
 function planned(a){const t=a.planned_task,box=$('h-plan');
  if(!t){box.replaceChildren(el('p',{class:'live-empty',text:tr('Tâche absente du plan de la mission.')}));return}
  const m=t.model;
  const rows=[[tr('Rôle'),t.plan_role||'—',a.role||'—'],
   [tr('Modèle'),m?[m.provider,m.model,m.level].filter(Boolean).join(' · '):'—',[a.provider,a.reported_model?.model].filter(Boolean).join(' · ')||'—'],
   [tr('Tentatives'),t.plan_max_attempts?String(t.plan_max_attempts):'—',String(t.attempts)],
   [tr('Appels d’outils'),t.plan_tool_limit?String(t.plan_tool_limit):'—',String(a.progress?.tool_calls??'—')]];
  const over=(r)=>r[0]===tr('Tentatives')?t.plan_max_attempts&&t.attempts>t.plan_max_attempts:r[0]===tr('Appels d’outils')?t.plan_tool_limit&&(a.progress?.tool_calls||0)>t.plan_tool_limit:false;
  const facts=[['Tâche',t.title+' ('+t.id+')'],['Livrable',t.deliverable],['Critères',t.criteria],['Dépendances',(t.depends||[]).join(', ')],['Statut de la tâche',L.taskStatus(t.status)],['Blocage',t.blocker],['Prochaine étape',t.next]].filter(([,v])=>v!==''&&v!=null&&v!==0);
  box.replaceChildren(el('dl',{class:'live-facts'},...facts.flatMap(([k,v])=>[el('dt',{text:tr(k)}),el('dd',{text:String(v)})])),
   table(tr('Prévu et réel'),[tr('Élément'),tr('Prévu'),tr('Réel')],rows.map(r=>({cls:over(r)?'is-alert':null,cells:r}))))}

 /** Typed transcript (LangWatch L2) of a finished attempt: kind filters with
  * counts, then entries in stream order (no content: tool, ids, error flag,
  * omitted byte length, declared tokens). Filters persist across refreshes. */
 const trKinds={system_prompt:'début de session',model_call:'appel modèle',assistant_message:'message',tool:'outil',tool_result:'résultat d’outil',tool_rejected:'outil refusé',note:'note'},trOff=new Set(),trTokens={input:'entrée',output:'sortie',cache_read:'cache lu',cache_creation:'cache créé',reasoning:'raisonnement'};
 function transcript(a){const t=a.transcript,list=$('h-tr'),filters=$('h-tr-filters'),state=$('h-tr-state');
  if(!t||t.state!=='available'){filters.replaceChildren();list.replaceChildren();state.textContent=t?.code?tr('Transcript indisponible')+' ('+t.code+')':tr('Disponible quand la tentative est terminée (capture du flux fournisseur).');return}
  filters.replaceChildren(...Object.keys(trKinds).filter(k=>t.counts[k]).map(k=>el('button',{type:'button',class:'live-tr-chip is-'+k,'aria-pressed':String(!trOff.has(k)),onclick:()=>{trOff.has(k)?trOff.delete(k):trOff.add(k);transcript(a)}},tr(trKinds[k])+' · '+t.counts[k])));
  list.replaceChildren(...t.entries.filter(e=>!trOff.has(e.kind)).map(e=>{const tok=Object.entries(e.tokens||{}).map(([k,v])=>tr(trTokens[k]||k)+' '+L.number(v)).join(' · ');
   return el('li',{class:'live-tr-item is-'+e.kind+(e.error?' is-error':'')},el('span',{class:'live-tr-kind',text:tr(trKinds[e.kind]||e.kind)}),
    el('span',{class:'live-tr-main',text:[e.tool,e.label,e.id&&'#'+e.id.slice(-6)].filter(Boolean).join(' · ')||'—'}),
    el('span',{class:'live-tr-meta',text:[e.error?tr('erreur'):'',e.bytes!=null?M.bytes(e.bytes)+' '+tr('omis'):'',tok].filter(Boolean).join(' · ')}))}));
  const unknown=Object.values(t.unknown||{}).reduce((n,v)=>n+v,0);
  state.textContent=[tr('format')+' '+t.format,t.entries.length+' '+tr('entrées'),unknown?unknown+' '+tr('événements non reconnus'):'',t.truncated||t.capture_truncated?tr('capture tronquée'):''].filter(Boolean).join(' · ')}

 /** Provider OTLP telemetry (LIVE-UX-03): per-call rows from api_request events,
  * tool aggregates, decisions and metric totals as stored by the engine. Every
  * figure is a provider declaration and is labelled so; nothing is recomputed
  * across temporalities here (a mixed total arrives as null). */
 const telStates={'no-token':'aucun jeton émis pour cet agent',waiting:'jeton émis, rien reçu',received:'reçue',unavailable:'indisponible'};
 const num=v=>{const n=typeof v==='number'?v:parseFloat(v);return Number.isFinite(n)?n:null};
 const callTokens=f=>['input_tokens','output_tokens','cache_read_tokens','cache_creation_tokens'].reduce((s,k)=>s+(num(f[k])||0),0);
 function table(caption,heads,rows){return el('table',{class:'live-table'},el('caption',{text:caption}),el('thead',{},el('tr',{},...heads.map(h=>el('th',{scope:'col',text:h})))),el('tbody',{},...rows.map(r=>el('tr',{class:r.cls||null},...r.cells.map((c,i)=>el(i?'td':'th',i?{text:c}:{scope:'row',text:c}))))))}
 /** Declared usage (end-of-turn totals) versus the sum of telemetry calls,
  * input+output on both sides. Two provider declarations, neither authoritative:
  * shown as a difference, never reconciled. Partial when calls were truncated. */
 function gap(a,calls,more){const d=M.tokens(a.usage);if(!d||L.usageState(a)!=='observed')return ['',''];
  const tel=calls.reduce((s,c)=>s+(num(c.fields.input_tokens)||0)+(num(c.fields.output_tokens)||0),0),diff=d.input+d.output-tel;
  return [tr('Écart déclaré − télémétrie'),(diff===0?tr('identique'):(diff>0?'+':'−')+L.number(Math.abs(diff))+' '+tr('jetons'))+(more?' · '+tr('partiel : 100 derniers appels'):'')]}
 /** Tool-call waterfall: the latest WATERFALL_ROWS `tool_result` events in time
  * order, each bar from (at − duration) to at, failures marked in words too.
  * Provider declarations; a call without duration shows as a tick. */
 const WATERFALL_ROWS=20,wfSeen=new Set();
 function waterfall(list,loc){const calls=(list||[]).slice(0,WATERFALL_ROWS).reverse();if(!calls.length)return null;
  const span=c=>[c.at_ms-(num(c.duration_ms)||0),c.at_ms],t0=Math.min(...calls.map(c=>span(c)[0])),t1=Math.max(...calls.map(c=>span(c)[1])),w=Math.max(1,t1-t0);
  const rows=calls.map(c=>{const [a,b]=span(c),fail=c.success===false,dur=num(c.duration_ms);
   const key=c.at_ms+'|'+c.tool,fresh=!wfSeen.has(key);wfSeen.add(key);
   const bar=el('span',{class:'live-wf-bar'+(fail?' is-fail':'')+(dur?'':' is-tick')+(fresh?' is-new':'')});bar.style.left=((a-t0)/w*100)+'%';bar.style.width=dur?Math.max(0.6,(b-a)/w*100)+'%':'';
   const label=c.tool+' · '+(dur!=null?L.duration(dur/1000):'—')+(fail?' · '+tr('échec'):'');
   return el('li',{class:'live-wf-row','aria-label':label,title:new Date(c.at_ms).toLocaleTimeString(loc)+' · '+label},el('span',{class:'live-wf-name',text:c.tool}),el('span',{class:'live-wf-track'},bar),el('span',{class:'live-wf-dur'+(fail?' is-fail':''),text:(dur!=null?L.duration(dur/1000):'—')+(fail?' ✗':'')}))});
  return el('div',{class:'live-wf'},el('p',{class:'live-wf-title',text:tr('Cascade des appels d’outils')+' ('+calls.length+(list.length>calls.length?' / '+list.length:'')+')'}),el('ol',{class:'live-wf-list'},...rows))}
 /** Execution trace (Codex spans, newest trace): run → turn → model sampling /
  * tools, indented by parent depth, bars on the trace's own time axis.
  * Provider declarations; only allowlisted span names reach this view. */
 const SPAN_LABELS={'codex.exec':'Exécution','session_task.turn':'Tour','run_sampling_request':'Appel modèle','codex.tool_blocking':'Attente d’outil','exec_command':'Commande','handle_responses':'Réponse modèle',
  'claude_code.interaction':'Interaction','claude_code.llm_request':'Appel modèle','claude_code.tool':'Outil','claude_code.tool.execution':'Exécution de l’outil','claude_code.tool.blocked_on_user':'Attente d’autorisation'};
 function traceTree(spans,more,loc){if(!spans?.length)return null;
  const byId=new Map(spans.map(x=>[x.fields.span_id,x])),depth=x=>{let d=0,p=x.fields.parent_id;while(p&&byId.has(p)&&d<16){d++;p=byId.get(p).fields.parent_id}return d};
  const t0=Math.min(...spans.map(x=>x.start_ms)),t1=Math.max(...spans.map(x=>x.start_ms+x.duration_ms)),w=Math.max(1,t1-t0);
  const rows=spans.map(x=>{const bar=el('span',{class:'live-wf-bar'+(x.duration_ms?'':' is-tick')});bar.style.left=((x.start_ms-t0)/w*100)+'%';bar.style.width=x.duration_ms?Math.max(0.6,x.duration_ms/w*100)+'%':'';
   const name=x.fields.tool_name||tr(SPAN_LABELS[x.name]||x.name),dur=x.duration_ms<1000?Math.round(x.duration_ms)+' ms':(Math.round(x.duration_ms/100)/10).toLocaleString(loc)+' s';
   const label=el('span',{class:'live-wf-name',text:name});label.style.paddingLeft=(depth(x)*12)+'px';
   return el('li',{class:'live-wf-row','aria-label':name+' · '+dur,title:name+' · '+dur},label,el('span',{class:'live-wf-track'},bar),el('span',{class:'live-wf-dur',text:dur}))});
  return el('div',{class:'live-wf live-trace'},el('p',{class:'live-wf-title',text:tr('Trace d’exécution')+' ('+spans.length+(more?'+':'')+')'}),el('ol',{class:'live-wf-list'},...rows))}
 /** Cache health (LangWatch L3): per call, total input = input + cache read +
  * cache creation; a judged call (total >= saved minimum) whose cache-read share
  * is below the saved percent is a context re-read. Thresholds are the saved v31
  * Admin settings (`cache_policy`, null before v31); display only. */
 function cacheOf(f,policy){const input=num(f.input_tokens)||0,read=num(f.cache_read_tokens)||0,made=num(f.cache_creation_tokens)||0,total=input+read+made;
  const share=total?read/total*100:0,judged=!!policy&&total>=policy.min_input_tokens&&total>0;return {total,share,judged,miss:judged&&share<policy.miss_ratio_percent}}
 // Display choice: the burn rate looks at the last 5 minutes of declared calls.
 const BURN_WINDOW_MS=5*60000;
 // Reader labels for known latency histograms; unknown names stay raw.
 const LATENCY_LABELS={'api_request.duration_ms':'Appel modèle','turn.ttft.duration_ms':'Premier jeton','turn.ttfm.duration_ms':'Premier message','turn.e2e_duration_ms':'Tour complet'};
 // Display choice: the most frequent unrecognized names, the rest counted.
 const UNKNOWN_SHOWN=8;
 function telemetry(a){const t=a.telemetry||{state:'unavailable'},box=$('h-tel'),loc=globalThis.SwarmI18n?.locale||'fr-FR';
  $('h-tel-state').replaceChildren(el('span',{text:tr(telStates[t.state]||t.state)+(t.code?' ('+t.code+')':'')+(t.last_received_ms?' · '+tr('dernier envoi')+' '+new Date(t.last_received_ms).toLocaleTimeString(loc):'')}),
   t.provenance?el('span',{class:'live-warn',text:tr('déclaration fournisseur non authentifiée')}):null);
  if(t.state!=='received'){box.replaceChildren(el('p',{class:'live-empty',text:t.state==='no-token'?tr('Aucun jeton émis : télémétrie non activée pour ce fournisseur (réglage Admin) ou serveur web absent au lancement.'):''}));return}
  telCalls.length=0;telCalls.push(...(t.calls||[]));
  const calls=t.calls||[],perCall=calls.slice(0,HISTORY).reverse().map(c=>callTokens(c.fields));
  const cost=calls.reduce((s,c)=>s+(num(c.fields.cost_usd)||0),0);
  // Estimated cost (saved rate catalogue) for calls without a declared cost; labelled apart.
  const est=t.estimated_cost,estText=est?.amount!=null&&est.currency==='USD'?' · ≈ '+M.money(est.amount)+' '+tr('estimé'):'';
  const parts=[figure(tr('Appels modèle'),calls.length+(t.calls_more?'+':'')+(cost?' · '+M.money(cost):'')+estText,spark(perCall,'is-s2',tr('Jetons par appel'),true))];
  // Live burn rate over the last BURN_WINDOW_MS of calls, and the share of the
  // Work's launch reserve used by this attempt (declared + estimated cost).
  const recent=calls.filter(c=>c.at_ms>=(calls[0]?.at_ms??0)-BURN_WINDOW_MS);
  if(recent.length>1){const span=Math.max(1,(recent[0].at_ms-recent[recent.length-1].at_ms)/60000),tok=recent.reduce((n,c)=>n+callTokens(c.fields),0);
   parts.push(el('p',{class:'live-res-states',text:tr('Débit : {t} jetons/min sur les {m} dernières minutes d’appels',{t:L.number(Math.round(tok/span)),m:BURN_WINDOW_MS/60000})}))}
  const reserve=a.budget?.reserve_per_launch_usd,spent=cost+(est?.amount!=null&&est.currency==='USD'?est.amount:0);
  if(reserve>0&&spent>0){const pct=Math.round(spent/reserve*100);
   parts.push(el('p',{class:'live-res-states'+(spent>reserve?' is-warn':''),text:tr('Réserve de lancement : {s} sur {r} ({p} %)',{s:M.money(spent),r:M.money(reserve),p:pct})+(spent>reserve?' · '+tr('dépassée'):'')}))}
  if(est&&est.unpriced_calls)parts.push(el('p',{class:'live-res-states',text:tr(est.unpriced_calls===1?'1 appel sans tarif enregistré (CLI : swarm-rust pricing save)':'{n} appels sans tarif enregistré (CLI : swarm-rust pricing save)',{n:est.unpriced_calls})+(est.missing_rates?.length?' · '+tr('catégories manquantes')+' : '+est.missing_rates.join(', '):'')}));
  const policy=a.cache_policy,health=calls.map(c=>cacheOf(c.fields,policy)),judged=health.filter(h=>h.judged).length,misses=health.filter(h=>h.miss).length;
  parts.push(el('div',{class:'live-cache'+(misses?' is-warn':'')},el('span',{class:'live-mini-title',text:tr('Santé du cache')}),
   policy?el('strong',{text:tr(misses===1?'1 relu sur {j} jugés':'{m} relus sur {j} jugés',{m:misses,j:judged})}):el('span',{class:'live-empty',text:tr('Seuils non configurés (réglages Admin v31).')}),
   policy?el('span',{class:'live-res-states',text:tr('relu = part lue du cache sous {p} % pour au moins {n} jetons d’entrée',{p:policy.miss_ratio_percent,n:L.number(policy.min_input_tokens)})}):null,
   spark(calls.slice(0,HISTORY).reverse().map(c=>cacheOf(c.fields,policy).total),'is-s1',tr('Entrée totale par appel'),true)));
  if(calls.length)parts.push(table(tr('Derniers appels'),[tr('Heure'),tr('Modèle'),tr('Entrée (jetons)'),tr('Sortie (jetons)'),tr('Cache lu'),tr('Durée')],calls.slice(0,8).map((c,i)=>{const f=c.fields,h=health[i];return {cls:(Number.isFinite(lastCallAt)&&c.at_ms>lastCallAt?'is-new':'')+(h.miss?' is-warn':'')||null,cells:[new Date(c.at_ms).toLocaleTimeString(loc),f.model||'—',L.number(num(f.input_tokens)||0),L.number(num(f.output_tokens)||0),h.total?Math.round(h.share)+' %'+(h.miss?' · '+tr('relu'):''):'—',num(f.duration_ms)!=null?L.duration(num(f.duration_ms)/1000):'—']}})));
  const wf=waterfall(t.tool_calls,loc);if(wf)parts.push(wf);
  const tree=traceTree(t.spans,t.spans_more,loc);if(tree)parts.push(tree);
  if(t.tools?.length)parts.push(table(tr('Outils déclarés'),[tr('Outil'),tr('Appels'),tr('Échecs'),tr('Durée moyenne'),tr('Durée max')],t.tools.map(x=>({cls:x.failures?'is-alert':null,cells:[x.name,String(x.count),String(x.failures),x.count?L.duration(x.total_ms/x.count/1000):'—',L.duration(x.max_ms/1000)]}))));
  // Provider latency histograms (Codex): merged per name and model by the engine.
  if(t.latencies?.length)parts.push(table(tr('Latences déclarées'),[tr('Mesure'),tr('Modèle'),tr('Mesures'),tr('Moyenne'),tr('Médiane'),tr('95e centile'),tr('Max')],
   t.latencies.map(x=>({cells:[tr(LATENCY_LABELS[x.name]||x.name),x.model||'—',L.number(x.count),...[x.mean_ms,x.p50_ms,x.p95_ms,x.max_ms].map(ms=>ms==null?'—':ms<1000?Math.round(ms)+' ms':(Math.round(ms/100)/10).toLocaleString(loc)+' s')]}))));
  const unknownNames=Object.entries(t.unknown||{}).sort((a,b)=>b[1]-a[1]);
  const facts=[...Object.entries(t.decisions||{}).map(([k,v])=>[tr('Décision')+' '+k,v]),
   ...(t.totals||[]).map(x=>[x.name+(x.type?' ('+x.type+')':''),x.value==null?tr('temporalités mélangées : non totalisé'):L.number(Math.round(x.value*1000)/1000)]),
   gap(a,calls,t.calls_more),
   [tr('Noms non reconnus'),unknownNames.length?unknownNames.slice(0,UNKNOWN_SHOWN).map(([k,v])=>k+' ×'+v).join(', ')+(unknownNames.length>UNKNOWN_SHOWN?' … (+'+(unknownNames.length-UNKNOWN_SHOWN)+')':''):''],[tr('Éléments rejetés'),t.rejected||'']].filter(([,v])=>v!==''&&v!=null);
  if(facts.length)parts.push(el('dl',{class:'live-facts'},...facts.flatMap(([k,v])=>[el('dt',{text:k}),el('dd',{text:String(v)})])));
  if(calls.length)lastCallAt=Math.max(lastCallAt,...calls.map(c=>c.at_ms));
  box.replaceChildren(...parts)}

 /** Event-kind groups for the timeline; errors use the reserved alert colour. */
 function group(kind){if(/error|limit|cooldown|fail/.test(kind))return 'alert';if(/tool/.test(kind))return 's2';if(/lifecycle|command/.test(kind))return 'life';if(/message|reply|output/.test(kind))return 's1';return 'other'}
 const groupLabels={s1:'messages',model:'appels modèle',s2:'outils',life:'cycle de vie',alert:'erreurs et limites',other:'autres'};
 /** Swimlane timeline: one lane per group, in LANES order, so position (not
  * colour alone) tells groups apart; model calls come from provider telemetry
  * (`api_request` events, provider declarations) next to the recorded log. */
 const LANES=['s1','model','s2','life','alert','other'],LANE=12;
 function frise(){const svg=$('h-frise');svg.replaceChildren();
  const marks=[...events.map(e=>({t:Date.parse(e.at),g:group(e.kind),label:e.kind+' · '+e.message.slice(0,120)})),
   ...telCalls.map(c=>({t:c.at_ms,g:'model',label:tr('appel modèle')+' · '+(c.fields.model||'—')+' · '+L.number(callTokens(c.fields))+' '+tr('jetons')}))].filter(m=>Number.isFinite(m.t));
  if(!marks.length)return;
  const used=LANES.filter(g=>marks.some(m=>m.g===g)),t0=Math.min(...marks.map(m=>m.t)),t1=Math.max(Date.now(),...marks.map(m=>m.t)),span=Math.max(1,t1-t0);
  svg.setAttribute('viewBox','0 0 600 '+used.length*LANE);svg.style.height=(used.length*LANE*1.25)+'px';
  for(const m of marks){const r=document.createElementNS(svgNS,'rect');r.setAttribute('x',String((m.t-t0)/span*596));r.setAttribute('y',String(used.indexOf(m.g)*LANE+1));r.setAttribute('width','4');r.setAttribute('height',String(LANE-2));r.setAttribute('rx','1');r.setAttribute('class','live-tick is-'+m.g);
   const title=document.createElementNS(svgNS,'title');title.textContent=new Date(m.t).toLocaleTimeString(globalThis.SwarmI18n?.locale||'fr-FR')+' · '+m.label;r.append(title);svg.append(r)}
  $('h-frise-legend').replaceChildren(...used.map(g=>el('li',{},el('i',{class:'live-dot is-'+g}),tr(groupLabels[g]))))}

 /** Scroll to and highlight the newest error/limit log line (button enabled only when one exists). */
 function lastError(){const items=[...log.children].filter(li=>/error|limit|cooldown|fail/.test(li.className));$('h-last-error').disabled=!items.length;return items.at(-1)}
 function applyFilter(){const q=$('h-filter').value.trim().toLowerCase();for(const li of log.children)li.hidden=!!q&&!li.textContent.toLowerCase().includes(q)}

 function scroll(box){if(follow.checked)box.scrollTop=box.scrollHeight}

 let loaded=false,lastCallAt=-Infinity;
 function apply(data){const a=data.agent;facts(a);
  for(const e of data.logs.items)events.push(e);
  for(const e of data.logs.items)log.append(el('li',{class:'kind-'+e.kind+(loaded?' is-new':'')},el('time',{datetime:e.at,text:new Date(e.at).toLocaleTimeString(globalThis.SwarmI18n?.locale||'fr-FR')}),el('span',{class:'live-log-kind',text:e.kind}),el('span',{class:'live-log-msg',text:e.message})));
  logsAfter=data.logs.next;$('h-log-empty').hidden=log.childElementCount>0;
  if(data.terminal.items.length){$('h-term-panel').hidden=false;if(xterm)xterm.fit();let text=term.textContent;
   for(const c of data.terminal.items){if(xterm){xterm.resize(c.cols,c.rows);xterm.write(c.text)}
    text+=L.stripAnsi(c.text).replace(/\r\n/g,'\n').replace(/[^\n]*\r(?!\n)/g,'');
    if(c.invalid_utf8){const note='\n['+tr('octets non UTF-8 remplacés')+']\n';text+=note;xterm?.write(note.replace(/\n/g,'\r\n'))}}
   term.textContent=text}
  termAfter=data.terminal.next;
  if(a.reply){$('h-reply-panel').hidden=false;$('h-reply').textContent=a.reply}
  const live=resources(a);tools(a);tokensPanel(a);planned(a);transcript(a);telemetry(a);annotation(a);evaluations(a);
  scnTask=a.task_id;scnWork=a.work_id;if(scnWasActive!==a.active){const ended=scnWasActive===true;scnWasActive=a.active;scenario(ended)}frise();applyFilter();lastError();
  scroll(term);scroll(log);loaded=true;
  clearTimeout(resTimer);resTimer=live?setTimeout(tick,RESOURCE_MS):null;
  return {more:data.logs.more||data.terminal.more,active:a.active}}

 // Run mode (?run=review:<id>|planning:<claim>): provider telemetry of one
 // review job or planning run, from /api/v1/live/run. These runs record no
 // agent logs, terminal, resources or transcript here: those panels are hidden.
 const RUN_KINDS={review:'Relecture',planning:'Planification'};
 const RUN_PHASES={claimed:'réservée',registered:'lancement enregistré',permitted:'en cours',unresolved:'non résolue',recorded:'verdict enregistré',applied:'appliquée',refused:'refusée'};
 function runMode(){document.querySelector('.live-panels').hidden=true;
  for(const id of ['h-plan','h-tr','h-frise','h-term-panel','h-log','h-reply-panel','h-ann','h-eval','h-scn'])$(id).closest('section').hidden=true;
  $('h-kicker').textContent=tr('Relecture ou planification · lecture seule')}
 function applyRun(data){const r=data.run;
  $('h-facts').replaceChildren(...[['Type',tr(RUN_KINDS[r.kind]||r.kind)],[r.kind==='review'?'Tâche relue':'Périmètre planifié',r.target],['Phase',tr(RUN_PHASES[r.phase]||r.phase)],['Mission',data.work],['Identifiant',r.id],['Préparée le',r.prepared_at?new Date(r.prepared_at).toLocaleString(globalThis.SwarmI18n?.locale||'fr-FR'):'']]
   .filter(([,v])=>v).flatMap(([k,v])=>[el('dt',{text:tr(k)}),el('dd',{text:String(v)})]));
  $('h-title').textContent=tr(RUN_KINDS[r.kind]||r.kind)+' · '+r.id;document.title=(r.active?'▶ ':'■ ')+tr(RUN_KINDS[r.kind]||r.kind)+' · '+tr(RUN_PHASES[r.phase]||r.phase);
  telemetry({telemetry:data.telemetry,cache_policy:data.cache_policy,budget:null});
  return {more:false,active:r.active}}

 // L10 operator annotation of a finished attempt. The form is refilled from
 // the server only when the stored revision changes and nothing is being
 // typed; a longer note is refused by the server, never truncated here.
 // annRev -1 = form not filled yet (the first read always fills it).
 let annRev=-1,annDirty=false,annCsrf='',annLimit=null,annWork='',annNotice='',annConflict=false;
 const annForm=$('h-ann'),annNote=$('h-ann-note');
 function annCount(){const n=[...annNote.value].length,over=annLimit!=null&&n>annLimit;
  $('h-ann-count').textContent=annLimit==null?tr('{n} caractères',{n}):tr('{n} / {m} caractères',{n,m:annLimit})+(over?' · '+tr('trop long'):'');$('h-ann-count').classList.toggle('is-warn',over);return over}
 function annState(text,error){const p=$('h-ann-state');p.textContent=text;p.classList.toggle('is-error',!!error)}
 function annotation(a){const pol=a.annotation_policy||{},stored=a.annotation&&!a.annotation.state?a.annotation:null;annWork=a.work_id;
  annLimit=pol.note_max_chars??null;const off=a.active||!pol.enabled||a.annotation?.state==='unavailable';
  for(const e of annForm.elements)e.disabled=off;
  if(a.annotation?.state==='unavailable')annState(tr('Annotations indisponibles')+' ('+a.annotation.code+')',true);
  else if(!pol.enabled)annState(tr('Annotations non configurées (réglages Admin v36).'),false);
  else if(a.active)annState(tr('Annotation possible quand la tentative est terminée.'),false);
  else if(annConflict){
   // Conflict: keep what the operator typed, adopt the stored revision so the
   // next save replaces it knowingly; the stored verdict/note are shown.
   annConflict=false;annRev=stored?.revision||0;
   annState(tr('Modifiée ailleurs (révision {r}) : votre texte est conservé ; enregistrer remplacera la version enregistrée',{r:annRev})+(stored?' — '+tr('verdict enregistré')+' : '+tr({good:'Bon',bad:'Mauvais',unsure:'Incertain'}[stored.verdict]||stored.verdict):''),true)}
  else if(!annDirty&&(stored?.revision||0)!==annRev){annRev=stored?.revision||0;for(const r of annForm.verdict)r.checked=r.value===stored?.verdict;annNote.value=stored?.note||'';
   // A conflict notice stays visible next to the reloaded version.
   annState((annNotice?annNotice+' ':'')+(stored?tr('Révision {r} · {t}',{r:stored.revision,t:new Date(stored.updated_ms).toLocaleString(globalThis.SwarmI18n?.locale||'fr-FR')}):tr('Pas encore annotée.')),!!annNotice);annNotice=''}
  annCount()}
 async function annSession(){const s=await L.getJSON('/api/v1/native-session');if(typeof s?.csrf_token!=='string')throw {code:'web-native-session-invalid'};annCsrf=s.csrf_token}
 annNote.addEventListener('input',()=>{annDirty=true;annCount()});annForm.addEventListener('change',()=>{annDirty=true});
 annForm.addEventListener('submit',async e=>{e.preventDefault();const verdict=annForm.verdict.value;
  if(!verdict){annState(tr('Choisir un verdict.'),true);return}if(annCount()){annState(tr('Note trop longue : raccourcir avant d’enregistrer.'),true);return}
  $('h-ann-save').disabled=true;
  try{for(let n=0;;n++){if(!annCsrf)await annSession();
    const r=await fetch('/api/v1/live/annotations',{method:'POST',credentials:'same-origin',cache:'no-store',headers:{Accept:'application/json','Content-Type':'application/json','X-Swarm-CSRF':annCsrf},
     body:JSON.stringify({schema_version:1,work:annWork,agent:agentId,verdict,note:annNote.value,expected_revision:Math.max(0,annRev)})});
    if(r.status===403&&n===0){annCsrf='';continue}const body=await r.json().catch(()=>null);
    if(r.ok){annDirty=false;annRev=body.annotation.revision;annState(tr('Enregistrée : révision {r}',{r:annRev}),false);break}
    if(body?.error==='annotation-revision-conflict'){annConflict=true;annState(tr('Modifiée ailleurs : lecture de la version enregistrée…'),true);stopped=false;tick();break}
    annState(tr('Enregistrement refusé')+' ('+(body?.error||'http-'+r.status)+')',true);break}}
  catch(err){annState(tr('Enregistrement impossible')+' ('+(err.code||err.message||err)+')',true)}
  finally{$('h-ann-save').disabled=false}});

 /** L4 stored evaluations of this attempt: score as a share, verdict only
  * when the evaluator has one (engine mark or saved policy). */
 const EVALUATORS={outcome:'Issue de la tentative',limits:'Limites d’exécution','tool-reliability':'Fiabilité des outils','cache-reuse':'Réutilisation du cache','usage-consistency':'Cohérence d’usage déclaré / télémétrie'};
 function evaluations(a){const v=a.evaluations,box=$('h-eval');
  if(v?.state==='unavailable'){box.replaceChildren(el('p',{class:'live-res-states is-error',text:tr('Évaluations indisponibles')+' ('+v.code+')'}));return}
  if(!Array.isArray(v)||!v.length){box.replaceChildren(el('p',{class:'live-empty',text:a.active?tr('Évaluée une fois la tentative terminée.'):tr('Pas encore évaluée (prochaine évaluation des alertes), hors échantillon ou évaluations désactivées (réglages Admin v37).')}));return}
  box.replaceChildren(table(tr('Évaluations déterministes'),[tr('Évaluateur'),tr('Score'),tr('Verdict'),tr('Détail')],v.map(e=>({cls:e.passed===false?'is-alert':null,
   cells:[tr(EVALUATORS[e.evaluator]||e.evaluator),e.score==null?'—':Math.round(e.score*100)+' %',e.passed==null?tr('sans verdict'):e.passed?tr('réussi'):tr('échoué'),e.detail||'—']}))))}

 // L6 task scenario: operator expectations judged against what was recorded
 // for the task's latest attempt. Read at load, when the attempt ends and
 // after a save (not on every follow tick); the editor is never overwritten
 // while being edited.
 const SCN_KINDS={outcome:['Issue attendue','status','completed'],'tool-used':['Outil utilisé','tool','Bash'],'tool-succeeded':['Outil sans échec','tool','Bash'],'tool-not-used':['Outil interdit','tool','Write'],
  'max-tool-failures':['Échecs d’outils au plus','value','0'],'max-cost-usd':['Coût déclaré au plus (USD)','value','1'],'usage-observed':['Usage observé',null,null]};
 let scnRev=0,scnDirty=false,scnLoaded=false,scnWasActive=null,scnTask='',scnWork='';
 function scnRow(e){const kind=el('select',{'aria-label':tr('Type d’attente'),onchange:()=>{scnDirty=true;sync()}},...Object.entries(SCN_KINDS).map(([k,[t]])=>el('option',{value:k,text:tr(t)})));kind.value=e.kind;
  const input=el('input',{type:'text','aria-label':tr('Valeur'),oninput:()=>{scnDirty=true}});const li=el('li',{},kind,input,el('button',{type:'button',class:'live-run-open',onclick:()=>{li.remove();scnDirty=true}},tr('Retirer')));
  function sync(){const [,field,example]=SCN_KINDS[kind.value];input.hidden=!field;input.placeholder=example||'';input.dataset.field=field||''}
  sync();const f=SCN_KINDS[e.kind]?.[1];if(f)input.value=String(e[f]??'');return li}
 function scnCollect(){return [...$('h-scn-rows').children].map(li=>{const [kind,input]=li.querySelectorAll('select,input'),f=input.dataset.field,e={kind:kind.value};
  // An empty or non-numeric value is refused here, never saved as 0.
  if(f==='value'){const raw=input.value.trim(),n=Number(raw);if(raw===''||!Number.isFinite(n)){input.focus();throw {code:'scenario-value-missing'}}e.value=n}else if(f)e[f]=input.value.trim();return e})}
 async function scenario(force){if(!scnTask||(!force&&scnLoaded))return;scnLoaded=true;
  try{const u=new URL('/api/v1/live/scenario',location.href);u.search=new URLSearchParams({work:scnWork,task:scnTask});const d=await L.getJSON(u.href);
   const res=d.results||[];$('h-scn-results').replaceChildren(d.scenario?table(tr('Résultat sur la dernière tentative')+(d.attempt?' ('+d.attempt.agent_id+')':''),[tr('Attente'),tr('Verdict'),tr('Observé')],
    res.map(r=>{const x=r.expectation,[t,f]=SCN_KINDS[x.kind]||[x.kind];return {cls:r.passed===false?'is-alert':null,cells:[tr(t)+(f?' · '+x[f]:''),r.passed==null?tr('non jugeable'):r.passed?tr('réussi'):tr('échoué'),r.observed]}})):el('p',{class:'live-empty',text:tr('Aucun scénario pour cette tâche.')}));
   if(!scnDirty){scnRev=d.scenario?.revision||0;$('h-scn-rows').replaceChildren(...(d.scenario?.spec.expectations||[{kind:'outcome',status:'completed'}]).map(scnRow))}}
  catch(e){$('h-scn-state').textContent=tr('Lecture du scénario impossible')+' ('+(e.code||e.message||e)+')'}}
 $('h-scn-add').addEventListener('click',()=>{scnDirty=true;$('h-scn-rows').append(scnRow({kind:'tool-used',tool:''}))});
 $('h-scn').addEventListener('submit',async e=>{e.preventDefault();$('h-scn-save').disabled=true;
  try{const expectations=scnCollect();for(let n=0;;n++){if(!annCsrf)await annSession();
    const r=await fetch('/api/v1/live/scenario',{method:'POST',credentials:'same-origin',cache:'no-store',headers:{Accept:'application/json','Content-Type':'application/json','X-Swarm-CSRF':annCsrf},
     body:JSON.stringify({schema_version:1,work:scnWork,task:scnTask,expected_revision:scnRev,scenario:{schema_version:1,expectations}})});
    if(r.status===403&&n===0){annCsrf='';continue}const body=await r.json().catch(()=>null);
    if(r.ok){scnDirty=false;$('h-scn-state').textContent=tr('Scénario enregistré : révision {r}',{r:body.revision});scenario(true);break}
    if(body?.error==='scenario-revision-conflict'){scnDirty=false;$('h-scn-state').textContent=tr('Modifié ailleurs : version enregistrée rechargée.');scenario(true);break}
    $('h-scn-state').textContent=tr('Enregistrement refusé')+' ('+(body?.error||'http-'+r.status)+')';break}}
  catch(err){$('h-scn-state').textContent=tr('Enregistrement impossible')+' ('+(err.code||err.message||err)+')'}
  finally{$('h-scn-save').disabled=false}});

 function label(){return {push:tr('direct poussé'),polling:tr('interrogation périodique'),reconnecting:tr('reconnexion'),connecting:tr('connexion')}[mode]}
 // A finished attempt is no longer followed by polling, but while push is
 // available each pushed change re-reads it once, so evaluations, annotations
 // and scenario results recorded after the end appear without a reload.
 let ended=false;
 function finish(text){ended=true;clearTimeout(timer);timer=null;const s=$('h-status');s.classList.remove('is-error');
  s.textContent=text+(stream?' · '+tr('relu à chaque changement'):'');$('h-resume').hidden=!!stream}
 function stop(text,error){stopped=true;stream?.close();stream=null;clearTimeout(timer);timer=null;$('h-resume').hidden=false;const s=$('h-status');s.classList.toggle('is-error',error);s.textContent=text}

 async function tick(){timer=null;if(stopped)return;if(inflight){again=true;return}inflight=true;
  const u=new URL(runId?'/api/v1/live/run':'/api/v1/live/agent',location.href);u.search=new URLSearchParams(runId?{run:runId,work:workId}:{agent:agentId,work:workId,logs_after:logsAfter,terminal_after:termAfter});
  try{const data=await L.getJSON(u.href),r=runId?applyRun(data):apply(data);const status=$('h-status');status.classList.remove('is-error');
   if(r.more)again=true;
   else if(r.active){status.textContent=tr('Suivi en direct')+' · '+label();if(mode==='polling')timer=setTimeout(tick,ACTIVE_MS)}
   else finish(tr(runId?'Exécution terminée : suivi arrêté.':'Agent terminé : suivi arrêté.'))}
  catch(e){
   // Saturation is a transient, explicit server refusal (no queue): keep following
   // and read again after one cadence; every other refusal stops and offers resume.
   if(e.code==='web-workers-saturated'){$('h-status').textContent=tr('Serveur occupé : nouvelle lecture')+' · '+label();timer=setTimeout(tick,ACTIVE_MS)}
   else stop(tr('Lecture impossible')+' ('+(e.code||e.message||e)+')',true)}
  finally{inflight=false;if(again&&!stopped){again=false;tick()}}}

 function connect(){stream=L.liveEvents(()=>tick(),(next,reason)=>{mode=next;if(next==='polling'){stream=null;if(ended)$('h-resume').hidden=false;else if(!stopped&&!timer&&!inflight)timer=setTimeout(tick,ACTIVE_MS)}if(!stopped&&!ended)$('h-status').textContent=tr('Suivi en direct')+' · '+label()+(reason?' ('+reason+')':'')})}

 $('h-filter').addEventListener('input',applyFilter);
 $('h-last-error').addEventListener('click',()=>{const li=lastError();if(!li)return;follow.checked=false;li.hidden=false;li.scrollIntoView({block:'center'});li.classList.add('is-target');li.tabIndex=-1;li.focus();setTimeout(()=>li.classList.remove('is-target'),1600)});
 addEventListener('keydown',e=>{const typing=/INPUT|TEXTAREA|SELECT/.test(document.activeElement?.tagName||'');if(e.key==='/'&&!typing){e.preventDefault();$('h-filter').focus()}else if(e.key==='m'&&!typing&&!e.ctrlKey&&!e.metaKey&&!e.altKey){e.preventDefault();follow.checked=false;const p=$('h-tel-panel');p.scrollIntoView({block:'start'});p.focus()}else if(e.key==='Escape'&&document.activeElement===$('h-filter')){$('h-filter').value='';applyFilter()}});
 $('h-copy').addEventListener('click',async()=>{const lines=[...log.children].filter(li=>!li.hidden).map(li=>li.querySelector('time')?.dateTime+' '+li.querySelector('.live-log-kind')?.textContent+' '+li.querySelector('.live-log-msg')?.textContent);
  const text=lines.join('\n')+(term.textContent?'\n\n--- terminal ---\n'+term.textContent:'');
  // Clipboard API first; selection-copy fallback for contexts where the browser
  // withholds clipboard permission. A double refusal is shown, never hidden.
  let ok=false;try{await navigator.clipboard.writeText(text);ok=true}catch{const area=el('textarea',{class:'live-copy-buffer','aria-hidden':'true'});area.value=text;document.body.append(area);area.select();try{ok=document.execCommand('copy')}catch{ok=false}area.remove()}
  $('h-copy-status').textContent=ok?tr('{n} lignes copiées',{n:lines.length}):tr('Copie refusée par le navigateur')});
 $('h-resume').addEventListener('click',()=>{stopped=false;ended=false;$('h-resume').hidden=true;mode='connecting';connect();tick()});
 $('h-theme').addEventListener('click',()=>{L.toggleTheme();xterm?.retheme()});
 follow.addEventListener('change',()=>{scroll(term);scroll(log)});
 globalThis.SwarmLiveHarness={terminalText:()=>xterm?xterm.text():term.textContent,renderer:()=>xterm?'xterm':'text'};
 if(runId)runMode();
 if(!(agentId||runId)||!workId){$('h-status').textContent=tr('Lecture impossible')+' (live-query-invalid)';$('h-status').classList.add('is-error')}
 else{connect();tick()}
})();
