'use strict';
// Live dashboard: refreshes /api/v1/live/overview on pushed changes
// (/api/v1/live/events), or by polling when push is unavailable.
// Read-only: clicking an agent only opens its harness window.
(function(){
 const L=globalThis.SwarmLive,M=globalThis.SwarmLiveMetrics,{tr,el}=L,$=id=>document.getElementById(id);
 // Display cadences, not engine policies: 2 s polling (Go cockpit parity);
 // 3 s re-read while active agents have live /proc resources (RAM/CPU change
 // without any recorded event); 90 samples ≈ 3 minutes of sparkline.
 const POLL_MS=2000,RESOURCE_MS=3000,HISTORY=90;
 const ORDER=['running','starting','queued','stopping','completed','failed','interrupted','abandoned'];
 const history=[],rates=new M.Rates(),seen=new Map(),pulses=new Map();
 // Per-agent activity between renders: tokens and tool calls already served by
 // the overview; EKG keeps the last EKG_SAMPLES deltas (display window only).
 const EKG_SAMPLES=32;let paused=false,timer=null,last=null,inflight=false,mode='connecting',stream=null,again=false;
 const filter=$('live-filter'),activeOnly=$('live-active-only');
 // Deep link: ?mission=<work>&active=1 selects the view; the URL follows changes.
 const params=new URLSearchParams(location.search),wanted=params.get('mission')||'';
 activeOnly.checked=params.has('active')?params.get('active')==='1':L.storage.get('swarm.live.active-only')==='1';
 function syncUrl(){const u=new URL(location.href);filter.value?u.searchParams.set('mission',filter.value):u.searchParams.delete('mission');activeOnly.checked?u.searchParams.set('active','1'):u.searchParams.delete('active');window.history.replaceState(null,'',u)}
 // Browser notifications: opt-in per viewer (local preference, not an engine policy).
 let notify=L.storage.get('swarm.live.notify')==='1'&&typeof Notification==='function'&&Notification.permission==='granted';let notified=0;
 function setNotify(on){notify=on;L.storage.set('swarm.live.notify',on?'1':'0');$('live-notify').setAttribute('aria-pressed',String(on))}
 /** Tab icon: logo plus a red (critical) or amber (warning) badge with the count. */
 function favicon(count,critical){const icon=$('live-favicon');if(!count){icon.href='/swarm-logo.png';icon.type='image/png';return}
  const colour=critical?'#d1453b':'#e0a100',label=count>9?'9+':String(count);
  const svg=`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="#062b49"/><circle cx="44" cy="20" r="18" fill="${colour}"/><text x="44" y="27" font-size="20" font-family="sans-serif" font-weight="700" text-anchor="middle" fill="#fff">${label}</text></svg>`;
  icon.type='image/svg+xml';icon.href='data:image/svg+xml,'+encodeURIComponent(svg)}
 function exportJSON(){return JSON.stringify({exported_at:new Date().toISOString(),scope:'Recorded live overview as served by /api/v1/live/overview; provider usage is an unauthenticated declaration',overview:last},null,2)}
 function download(){if(!last)return;const blob=new Blob([exportJSON()],{type:'application/json'}),a=document.createElement('a');a.href=URL.createObjectURL(blob);a.download='swarm-live-'+new Date().toISOString().replace(/[:.]/g,'-')+'.json';document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(a.href),1000)}
 const svgNS='http://www.w3.org/2000/svg',reduced=matchMedia('(prefers-reduced-motion: reduce)');

 function sparkline(){const svg=$('live-spark');svg.replaceChildren();if(history.length<2)return;const max=Math.max(1,...history);const step=300/(HISTORY-1),off=(HISTORY-history.length)*step;
  const pts=history.map((v,i)=>[off+i*step,46-(v/max)*42]);
  const area=document.createElementNS(svgNS,'path');area.setAttribute('d','M'+pts[0][0]+',47 L'+pts.map(p=>p.join(',')).join(' L')+' L'+pts.at(-1)[0]+',47 Z');area.setAttribute('class','live-spark-area');
  const line=document.createElementNS(svgNS,'polyline');line.setAttribute('points',pts.map(p=>p.join(',')).join(' '));line.setAttribute('class','live-spark-line');svg.append(area,line)}

 function stack(target,counts,legend){target.replaceChildren();
  const keys=[...ORDER,...Object.keys(counts).filter(s=>!ORDER.includes(s))].filter(s=>counts[s]);
  for(const s of keys){const seg=el('span',{class:'live-seg status-'+s,title:L.statusLabel(s)+' : '+counts[s]});seg.style.flexGrow=String(counts[s]);target.append(seg)}
  target.setAttribute('aria-label',keys.map(s=>L.statusLabel(s)+' '+counts[s]).join(', ')||tr('Aucun agent'));
  if(legend)legend.replaceChildren(...keys.map(s=>el('li',{},el('i',{class:'live-dot status-'+s}),L.statusLabel(s)+' ',el('b',{text:String(counts[s])}))))}

 function openHarness(agent){const u=new URL('/live-session.html',location.href);u.searchParams.set('agent',agent.id);u.searchParams.set('work',agent.work_id);u.searchParams.set('lang',globalThis.SwarmI18n?.language||'fr');u.searchParams.set('theme',document.documentElement.dataset.theme);
  const w=window.open(u.href,'swarm-live-'+agent.id,'popup,width=1040,height=820');if(w)w.focus();else location.assign(u.href)}

 /** Harness window in run mode: provider telemetry of one review job or
  * planning run (subject `review:<id>` / `planning:<claim>`). */
 function openRun(w,run){const u=new URL('/live-session.html',location.href);u.searchParams.set('run',run.subject);u.searchParams.set('work',w.id);u.searchParams.set('lang',globalThis.SwarmI18n?.language||'fr');u.searchParams.set('theme',document.documentElement.dataset.theme);
  const win=window.open(u.href,'swarm-live-'+run.subject.replace(/[^\w-]/g,'-'),'popup,width=1040,height=820');if(win)win.focus();else location.assign(u.href)}

 /** Depth of each agent through its parent chain inside one Work; unknown or
  * cyclic parents are cut so a malformed record cannot hang the layout. */
 function depths(agents){const by=new Map(agents.map(a=>[a.id,a])),memo=new Map();
  const depth=(a,seen)=>{if(memo.has(a.id))return memo.get(a.id);const p=a.parent&&by.get(a.parent);const d=p&&!seen.has(p.id)?depth(p,new Set([...seen,a.id]))+1:0;memo.set(a.id,d);return d};
  for(const a of agents)depth(a,new Set([a.id]));return memo}

 /** Fields whose change flashes a node (status, activity, tokens, tools). */
 function signature(a){const t=M.tokens(a.usage);return [a.status,a.activity,t?.input,t?.output,a.progress?.tool_calls,a.progress?.last_tool].join('|')}

 /** Activity since the previous render (tokens, tool calls) and the EKG line.
  * Pure presentation of served counters: nothing is inferred or estimated. */
 // Global pulse: changes observed per render (token growth, tool calls) kept
 // for PULSE_WINDOW_MS; the header shows the last-minute rate and a waveform.
 const PULSE_WINDOW_MS=10*60*1000,PULSE_BUCKETS=30,beats=[];let beatNow=0;
 function pulse(agent,t,p){const tok=t?t.input+t.output:0,tools=p.tool_calls||0,prev=pulses.get(agent.id),samples=prev?prev.samples:[];
  const dTok=prev?Math.max(0,tok-prev.tok):0,dTools=prev?Math.max(0,tools-prev.tools):0;
  if(prev){samples.push(dTools+Math.min(dTok/500,4));while(samples.length>EKG_SAMPLES)samples.shift();beatNow+=dTools+(dTok>0?1:0)}
  pulses.set(agent.id,{tok,tools,samples});return {dTok,dTools,samples}}
 function ekg(samples){const svg=document.createElementNS(svgNS,'svg');svg.setAttribute('viewBox','0 0 200 24');svg.setAttribute('preserveAspectRatio','none');svg.setAttribute('class','live-ekg');svg.setAttribute('aria-hidden','true');
  const max=Math.max(1,...samples),pts=[];const n=EKG_SAMPLES;for(let i=0;i<n;i++){const v=samples[i-(n-samples.length)]??0;const x=i/(n-1)*200;pts.push(x+','+(20-v/max*16));}
  for(const cls of ['live-ekg-base','live-ekg-glow']){const pl=document.createElementNS(svgNS,'polyline');pl.setAttribute('points',pts.join(' '));pl.setAttribute('pathLength','400');pl.setAttribute('class',cls);svg.append(pl)}return svg}
 function node(agent,tasks){const t=M.tokens(agent.usage),p=agent.progress||{},state=L.usageState(agent),flags=M.attention(agent),res=M.resources(agent);
  const meta=[agent.provider,agent.reported_model?.model].filter(Boolean).join(' · ');
  const started=Date.parse(agent.started),secs=M.activeSeconds(agent);
  const duration=el('span',{class:'live-duration',title:tr("Durée d'activité")},el('i',{class:'live-ico',text:'⏱'}),el('span',agent.active&&Number.isFinite(started)?{'data-since':String(started),text:L.duration(secs)}:{text:L.duration(secs)}));
  const cpu=res?rates.rate(agent.id,res.cpu,res.at):null;
  const button=el('button',{type:'button',class:'live-node status-'+agent.status+(agent.active?' is-active':'')+(flags.some(f=>f.level==='critical')?' is-critical':flags.length?' is-warning':''),'data-agent':agent.id,'aria-label':tr('Ouvrir le harnais de')+' '+(agent.role||agent.id)+' — '+L.statusLabel(agent.status),onclick:()=>openHarness(agent)},
   el('span',{class:'live-node-head'},el('i',{class:'live-dot status-'+agent.status}),el('strong',{text:agent.role||tr('Agent')}),el('span',{class:'live-chip',text:L.statusLabel(agent.status)})),
   el('span',{class:'live-node-task',text:tasks[agent.task_id]||agent.task_id||''}),
   el('span',{class:'live-node-activity'+(agent.active?' is-live':''),text:agent.activity||p.action||''}),
   el('span',{class:'live-node-stats'},duration,state==='observed'&&t?el('span',{text:L.number(t.input+t.output)+' '+tr('jetons')}):el('span',{class:'live-warn',text:tr('usage')+' '+L.factLabel(state)}),t?.cost?el('span',{text:M.money(t.cost)}):null),
   t&&(t.input||t.output)?M.tokenBar(t,true):null,
   p.tool_calls||agent.limits?.max_tool_calls?M.meter(tr('outils'),p.tool_calls,agent.limits?.max_tool_calls):null,
   res&&res.rss!=null?el('span',{class:'live-node-res'},el('span',{text:'RAM '+M.bytes(res.rss)}),cpu!=null?el('span',{text:'CPU '+cpu.toFixed(0)+' %'}):null,res.descendants?el('span',{text:'+'+res.descendants+' '+tr('sous-proc.')}):null):null,
   agent.context?el('span',{class:'live-node-res'},el('span',{text:tr('contexte')+' '+M.bytes(agent.context.bytes)}),agent.context.memory?el('span',{text:agent.context.memory+' '+tr('mémoires')}):null):null,
   flags.length?el('span',{class:'live-flags'},...flags.slice(0,2).map(f=>el('span',{class:'live-flag is-'+f.level,text:f.text}))):null,
   el('span',{class:'live-node-meta'},meta?el('span',{text:meta}):null,p.last_tool?el('span',{text:tr('outil')+' '+p.last_tool}):null));
  const sig=signature(agent),prev=seen.get(agent.id);seen.set(agent.id,sig);
  if(prev&&prev!==sig&&!reduced.matches)button.classList.add('is-changed');
  const beat=pulse(agent,t,p);
  if(agent.active)button.append(ekg(beat.samples));
  if(!reduced.matches&&(beat.dTok||beat.dTools)){button.classList.add('is-pulse');
   if(beat.dTok)button.append(el('span',{class:'live-float',text:'+'+L.number(beat.dTok)+' '+tr('jetons')}));
   if(beat.dTools)button.append(el('span',{class:'live-float is-second',text:'+'+beat.dTools+' '+tr('outils')+(p.last_tool?' · '+p.last_tool:'')}))}
  return button}

 function connectors(graph){const svg=graph.querySelector('svg');svg.replaceChildren();const box=graph.getBoundingClientRect();svg.setAttribute('width',graph.scrollWidth);svg.setAttribute('height',graph.scrollHeight);
  for(const n of graph.querySelectorAll('[data-parent]')){const p=graph.querySelector('[data-agent="'+CSS.escape(n.dataset.parent)+'"]');if(!p)continue;const a=p.getBoundingClientRect(),b=n.getBoundingClientRect();
   const x1=a.right-box.left+graph.scrollLeft,y1=a.top+a.height/2-box.top,x2=b.left-box.left+graph.scrollLeft,y2=b.top+b.height/2-box.top,mx=(x1+x2)/2,d=`M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`;
   const active=n.classList.contains('is-active');
   const base=document.createElementNS(svgNS,'path');base.setAttribute('d',d);base.setAttribute('class','live-link'+(active?' is-active':''));svg.append(base);
   if(active){const flow=document.createElementNS(svgNS,'path');flow.setAttribute('d',d);flow.setAttribute('class','live-link-flow');svg.append(flow);
    if(!reduced.matches)for(const delay of [0,1.2]){const dot=document.createElementNS(svgNS,'circle');dot.setAttribute('r','3.5');dot.setAttribute('class','live-particle');
     const move=document.createElementNS(svgNS,'animateMotion');move.setAttribute('dur','2.4s');move.setAttribute('begin',delay+'s');move.setAttribute('repeatCount','indefinite');move.setAttribute('path',d);dot.append(move);svg.append(dot)}}}}

 /** Plan graph of one Work: tasks recorded by the initial planning phase, in
  * dependency columns, each linked to the live agents executing it (task_id).
  * A task with neither attempt nor agent is shown as not started. Clicking a
  * task moves focus to its first agent node. Cycles or unknown dependencies
  * fall back to column 0 (no inference). */
 function planGraph(w,agents){const plan=w.plan||[];if(!plan.length)return null;
  const by=new Map(plan.map(t=>[t.id,t])),memo=new Map();
  const depth=(t,seen)=>{if(memo.has(t.id))return memo.get(t.id);let d=0;for(const id of t.depends||[]){const p=by.get(id);if(p&&!seen.has(id))d=Math.max(d,depth(p,new Set([...seen,id]))+1)}memo.set(t.id,d);return d};
  const columns=[];for(const t of plan)(columns[depth(t,new Set([t.id]))]||=[]).push(t);
  // Fewer crossings: order each column by the mean row of its dependencies in
  // earlier columns (barycentre heuristic); ties keep plan order.
  const row=new Map();(columns[0]||[]).forEach((t,i)=>row.set(t.id,i));
  for(let c=1;c<columns.length;c++){const col=columns[c]||[];const key=t=>{const r=(t.depends||[]).map(id=>row.get(id)).filter(v=>v!=null);return r.length?r.reduce((a,b)=>a+b,0)/r.length:Infinity};
   col.forEach((t,i)=>t._i=i);col.sort((a,b)=>key(a)-key(b)||a._i-b._i);col.forEach((t,i)=>row.set(t.id,i))}
  const graph=el('div',{class:'live-graph live-plan','aria-label':tr('Plan de la mission')},document.createElementNS(svgNS,'svg'));
  for(const col of columns)graph.append(el('div',{class:'live-col'},...(col||[]).map(t=>{const mine=agents.filter(a=>a.task_id===t.id),live=mine.some(a=>a.active),idle=!t.attempts&&!mine.length;
   const b=el('button',{type:'button',class:'live-task'+(live?' is-active':'')+(idle?' is-idle':''),'data-task':t.id,'aria-label':tr('Tâche')+' '+t.title+' — '+(idle?tr('non démarrée'):L.taskStatus(t.status)),onclick:()=>{const n=document.querySelector('.live-node[data-task="'+CSS.escape(t.id)+'"]');if(n){n.focus();n.classList.add('is-target');setTimeout(()=>n.classList.remove('is-target'),1600)}}},
    el('span',{class:'live-node-head'},el('strong',{text:t.title||t.id,title:t.title||t.id}),el('span',{class:'live-chip',text:idle?tr('non démarrée'):L.taskStatus(t.status)})),
    el('span',{class:'live-node-task',text:[t.plan_role&&tr('rôle prévu')+' '+t.plan_role,t.plan_max_attempts?tr('tentatives')+' '+t.attempts+'/'+t.plan_max_attempts:t.attempts?t.attempts+' '+tr('tentatives'):''].filter(Boolean).join(' · ')}),
    el('span',{class:'live-node-meta'},el('span',{text:mine.length===1?tr('1 agent'):mine.length?tr('{n} agents',{n:mine.length}):tr('aucun agent')}),t.blocker?el('span',{class:'live-warn',text:tr('bloquée')}):null,scenarioChip(w.scenarios?.[t.id])));
   if((t.depends||[]).length)b.dataset.depends=t.depends.join(' ');
   b.addEventListener('mouseenter',()=>chain(graph,t.id));b.addEventListener('focus',()=>chain(graph,t.id));
   b.addEventListener('mouseleave',()=>chain(graph,null));b.addEventListener('blur',()=>chain(graph,null));return b})));
  return graph}
 /** Highlight a task's whole dependency chain (upstream and downstream) and dim
  * the rest; `id` null clears. Presentation only. */
 function chain(graph,id){const tasks=[...graph.querySelectorAll('.live-task')],paths=[...graph.querySelectorAll('.live-depends')];
  graph.classList.toggle('is-focusing',!!id);if(!id){for(const e of [...tasks,...paths])e.classList.remove('is-dim','is-hl');return}
  const deps=new Map(tasks.map(t=>[t.dataset.task,(t.dataset.depends||'').split(' ').filter(Boolean)])),keep=new Set([id]);
  const up=x=>{for(const d of deps.get(x)||[])if(!keep.has(d)){keep.add(d);up(d)}};
  const down=x=>{for(const [k,v] of deps)if(v.includes(x)&&!keep.has(k)){keep.add(k);down(k)}};up(id);down(id);
  for(const t of tasks)t.classList.toggle('is-dim',!keep.has(t.dataset.task));
  for(const p of paths){const on=keep.has(p.dataset.from)&&keep.has(p.dataset.to);p.classList.toggle('is-hl',on);p.classList.toggle('is-dim',!on)}}
 let arrowSeq=0;
 /** Two arrowhead markers per plan svg (static and flowing), ids unique per draw. */
 function arrowDefs(svg){const id='live-arrow-'+(++arrowSeq),defs=document.createElementNS(svgNS,'defs');
  for(const [suffix,cls] of [['','live-arrow-head'],['-flow','live-arrow-head is-flow']]){const m=document.createElementNS(svgNS,'marker');
   for(const [k,v] of [['id',id+suffix],['viewBox','0 0 10 10'],['refX','9'],['refY','5'],['markerWidth','7'],['markerHeight','7'],['orient','auto-start-reverse']])m.setAttribute(k,v);
   const p=document.createElementNS(svgNS,'path');p.setAttribute('d','M0,0 L10,5 L0,10 z');p.setAttribute('class',cls);m.append(p);defs.append(m)}
  svg.append(defs);return id}
 /** What blocks the mission, from the served plan only (no estimate): each
  * unfinished task waiting on unfinished dependencies, with the chain roots
  * (first unfinished tasks that wait on nothing); explicit blockers too. */
 const DONE=new Set(['accepted','waived']);
 /** L9 delivery ledger: per planned task with attempts or a reviewed
  * candidate — status, attempts (agents recorded on the task), declared
  * tokens and cost summed over observed usage only (others counted apart,
  * never estimated), and the candidate commit from the task's independent
  * review when one is recorded. Read-only; not a published release. */
 function deliveries(w){const rows=[];
  for(const t of w.plan||[]){const agents=(w.agents||[]).filter(a=>a.task_id===t.id);if(!agents.length&&!t.delivery)continue;
   let tokens=0,cost=0,unknown=0;for(const a of agents){const u=M.tokens(a.usage);if(u&&L.usageState(a)==='observed'){tokens+=u.input+u.output;cost+=u.cost}else unknown++}
   const d=t.delivery,commit=d?.commit?d.commit.slice(0,12):'—';
   rows.push(el('tr',{'data-delivery':t.id},el('th',{scope:'row',text:t.title||t.id}),el('td',{text:L.taskStatus(t.status)}),el('td',{class:'is-num',text:L.number(agents.length)}),
    el('td',{class:'is-num',text:L.number(tokens)+(unknown?' (+'+unknown+' '+tr('sans usage')+')':'')}),el('td',{class:'is-num',text:M.money(cost)}),
    el('td',{},el('code',{text:commit,title:d?.commit||''}),d?el('span',{class:'live-delivery-review',text:' · '+tr('revue')+' '+(d.review_state||'?')+(d.reviews>1?' ×'+d.reviews:'')}):null)))}
  if(!rows.length)return null;
  return el('div',{class:'live-deliveries'},el('h3',{class:'live-plan-title',text:tr('Livraisons')+' ('+rows.length+')'}),
   el('div',{class:'live-table-scroll'},el('table',{class:'live-delivery-table'},el('thead',{},el('tr',{},...[['Tâche'],['Statut'],['Tentatives',1],['Jetons',1],['Coût',1],['Commit candidat']].map(([h,num])=>el('th',{scope:'col',class:num?'is-num':'',text:tr(h)})))),el('tbody',{},...rows))))}
 /** L7 budget summary of a Work: declared cost against the saved limit and
  * the engine's committed estimates; overspend in alert colour and words.
  * Null budget (no policy) or a zero limit (unlimited) shows nothing. */
 function budgetChip(b){if(b?.state==='unavailable')return el('span',{class:'live-work-budget is-alert','data-budget':'unavailable'},tr('Budget indisponible')+' ('+b.code+')');
  if(!b||!(b.limit_usd>0))return null;const over=b.reported_usd>b.limit_usd;
  // Zero is a real amount here (M.money shows '—' for "no value").
  const usd=v=>v>0?M.money(v):new Intl.NumberFormat(globalThis.SwarmI18n?.locale||'fr-FR',{style:'currency',currency:'USD'}).format(0);
  return el('span',{class:'live-work-budget'+(over?' is-alert':''),'data-budget':over?'over':'ok',title:tr('Engagé (estimations réservées) : {c}',{c:usd(b.committed_usd)})},
   tr('Budget : {r} déclarés sur {l}',{r:usd(b.reported_usd),l:usd(b.limit_usd)})+(b.attempts_without_cost?' · '+tr('partiel ({n} sans coût)',{n:b.attempts_without_cost}):'')+' · '+tr('engagé {c}',{c:usd(b.committed_usd)})+(over?' · '+tr('dépassé'):''))}
 /** L6 scenario result of a planned task: expectations passed of the
  * judgeable ones; any failure in alert colour and words, undecided counted. */
 function scenarioChip(s){if(!s)return null;const judged=s.passed+s.failed;
  return el('span',{class:'live-scenario'+(s.failed?' is-alert':''),'data-scenario':s.failed?'failed':'ok'},tr('scénario {p}/{n}',{p:s.passed,n:judged})+(s.failed?' · '+tr('{f} en échec',{f:s.failed}):'')+(s.undecided?' · '+tr('{u} non jugeables',{u:s.undecided}):''))}
 /** L8 regression dataset of a Work: recorded facts only (task, outcome,
  * cause, usage, annotation, telemetry summary), downloaded as JSONL, one
  * attempt per line. The selection stays chosen across re-renders. */
 const exportSelect=new Map();
 function datasetExport(w){const sel=el('select',{'aria-label':tr('Sélection du jeu de régression'),onchange:e=>exportSelect.set(w.id,e.target.value)},
   ...[['failed','Échecs'],['bad','Annotées « mauvais »'],['all','Toutes les tentatives terminées']].map(([v,t])=>el('option',{value:v,text:tr(t)})));
  sel.value=exportSelect.get(w.id)||'failed';const state=el('span',{class:'live-export-state',role:'status'});
  const go=el('button',{type:'button',class:'live-run-open',onclick:async()=>{state.textContent=tr('Export…');
   try{const u=new URL('/api/v1/live/dataset',location.href);u.search=new URLSearchParams({work:w.id,select:sel.value});const d=await L.getJSON(u.href);
    // First line: header (format, provenance, Work, selection), then one attempt per line.
    const head={kind:'header',schema_version:d.schema_version,provenance:d.provenance,work:d.work,select:d.select,exported_at:new Date().toISOString(),entries:d.entries.length};
    const blob=new Blob([[head,...d.entries].map(e=>JSON.stringify(e)).join('\n')+'\n'],{type:'application/x-ndjson'}),a=document.createElement('a');
    a.href=URL.createObjectURL(blob);a.download='swarm-dataset-'+w.id+'-'+sel.value+'.jsonl';document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(a.href),1000);
    state.textContent=tr(d.entries.length===1?'1 entrée exportée':'{n} entrées exportées',{n:d.entries.length})}
   catch(e){state.textContent=tr('Export impossible')+' ('+(e.code||e.message||e)+')'}}},tr('Exporter (JSONL)'));
  return el('div',{class:'live-export'},el('h3',{class:'live-plan-title',text:tr('Jeu de régression')}),el('div',{class:'live-export-row'},sel,go,state))}
 /** Review jobs and planning runs of a Work (engine phases, newest first per
  * kind) with a button opening their telemetry; none recorded → no panel. */
 const RUN_KINDS={review:'Relecture',planning:'Planification'};
 const RUN_PHASES={claimed:'réservée',registered:'lancement enregistré',permitted:'en cours',unresolved:'non résolue',recorded:'verdict enregistré',applied:'appliquée',refused:'refusée'};
 function runsPanel(w){const runs=w.runs||[];if(!runs.length)return null;
  const rows=runs.map(r=>el('tr',{'data-run':r.subject,class:r.active?'is-active':null},el('th',{scope:'row',text:tr(RUN_KINDS[r.kind]||r.kind)}),el('td',{text:r.kind==='review'?(w.tasks?.[r.target]||r.target):r.target}),
   el('td',{text:tr(RUN_PHASES[r.phase]||r.phase)}),el('td',{},el('code',{text:r.id.slice(0,12),title:r.id})),el('td',{},el('button',{type:'button',class:'live-run-open',onclick:()=>openRun(w,r)},tr('Télémétrie')))));
  return el('div',{class:'live-deliveries live-runs'},el('h3',{class:'live-plan-title',text:tr('Relectures et planification')+' ('+runs.length+')'}),
   el('div',{class:'live-table-scroll'},el('table',{class:'live-delivery-table live-runs-table'},el('thead',{},el('tr',{},...['Type','Cible','Phase','Identifiant',''].map(h=>el('th',{scope:'col',text:h?tr(h):''})))),el('tbody',{},...rows))))}
 function blockers(plan){const by=new Map(plan.map(t=>[t.id,t])),open=t=>!DONE.has(t.status);
  const unmet=t=>(t.depends||[]).map(id=>by.get(id)).filter(d=>d&&open(d));
  const roots=(t,seen=new Set())=>{const u=unmet(t);if(!u.length)return [t];return u.flatMap(d=>seen.has(d.id)?[]:roots(d,new Set([...seen,d.id])))};
  const items=[];for(const t of plan){if(!open(t))continue;const u=unmet(t);
   if(u.length){const r=[...new Map(roots(t).map(x=>[x.id,x])).values()];items.push(el('li',{},el('b',{text:t.title}),' '+tr('attend')+' '+u.map(d=>d.title+' ('+L.taskStatus(d.status)+')').join(', '),r.some(x=>!u.includes(x))?el('span',{class:'live-root',text:' · '+tr('en tête de chaîne')+' : '+r.map(x=>x.title).join(', ')}):null))}
   else if(t.blocker||t.status==='blocked')items.push(el('li',{class:'is-blocked'},el('b',{text:t.title}),' '+tr('bloquée')+(t.blocker?' : '+t.blocker:'')))}
  return items.length?el('div',{class:'live-blockers'},el('h3',{class:'live-plan-title',text:tr('Ce qui bloque')+' ('+items.length+')'}),el('ul',{},...items)):null}
 function planConnectors(graph){const svg=graph.querySelector('svg');svg.replaceChildren();const arrow=arrowDefs(svg);const box=graph.getBoundingClientRect();svg.setAttribute('width',graph.scrollWidth);svg.setAttribute('height',graph.scrollHeight);
  for(const n of graph.querySelectorAll('[data-depends]'))for(const id of n.dataset.depends.split(' ')){const p=graph.querySelector('[data-task="'+CSS.escape(id)+'"]');if(!p)continue;const a=p.getBoundingClientRect(),b=n.getBoundingClientRect();
   const x1=a.right-box.left+graph.scrollLeft,y1=a.top+a.height/2-box.top,x2=b.left-box.left+graph.scrollLeft-2,y2=b.top+b.height/2-box.top,mx=(x1+x2)/2;
   const path=document.createElementNS(svgNS,'path');path.setAttribute('d',`M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`);const flow=n.classList.contains('is-active');path.setAttribute('class','live-depends'+(flow?' is-flow':''));path.setAttribute('marker-end','url(#'+arrow+(flow?'-flow':'')+')');path.dataset.from=id;path.dataset.to=n.dataset.task;svg.append(path)}}

 /** Agents carrying engine-recorded attention signals, most severe first. */
 function attentionPanel(works){const rows=[];for(const w of works)for(const a of w.agents){const f=M.attention(a);if(f.length)rows.push({a,f,w})}
  rows.sort((x,y)=>(y.f[0].level==='critical')-(x.f[0].level==='critical'));
  $('live-attention-count').textContent=String(rows.length);
  favicon(rows.length,rows.some(r=>r.f[0].level==='critical'));
  $('live-attention').replaceChildren(...(rows.length?rows.slice(0,8).map(({a,f,w})=>el('li',{},el('button',{type:'button',class:'live-attn is-'+f[0].level,onclick:()=>openHarness(a)},el('i',{class:'live-dot status-'+a.status}),el('strong',{text:a.role||a.id}),el('span',{class:'live-attn-work',text:w.title||w.id}),el('span',{class:'live-flags'},...f.map(x=>el('span',{class:'live-flag is-'+x.level,text:x.text})))))):[el('li',{class:'live-empty',text:tr('Rien à signaler.')})]))}

 // L5 supervisor alerts: the server evaluates engine marks and keeps the
 // fired/resolved history (.swarm/telemetry.db); this page triggers that
 // evaluation after each overview render, and on pushed changes while hidden
 // (not while paused). Each page notifies the open alerts it had not seen yet,
 // so every open console notifies (same tag: the browser shows one), whichever
 // page's request recorded the transition. The first evaluation of a page is
 // baseline: shown, not notified.
 const RULE_LABELS={'agent-failed':'Agent en échec','agent-error-limit':"Limite d'erreurs atteinte",'agent-time-threshold':'Seuil de durée dépassé','agent-tool-wait':"Attente d'outil prolongée",'agent-usage-invalid':'Usage invalide ou incomplet','task-blocked':'Tâche bloquée','agent-usage-mismatch':'Écart d’usage déclaré / télémétrie','work-budget-threshold':'Seuil de budget atteint','work-budget-overspent':'Budget dépassé (coût déclaré)'};
 // Display choice: more than NOTIFY_BATCH new alerts at once become one summary notification.
 const NOTIFY_BATCH=3;
 let csrf='',alertsBusy=false,alertsAgain=false,alertsBaseline=true,evaluations=0,retry=null,busyRetried=false;const knownOpen=new Set();
 /** CSRF token of the current session (needed by the evaluate POST). */
 async function session(){const s=await L.getJSON('/api/v1/native-session');if(s?.schema_version!==1||typeof s?.csrf_token!=='string'||!/^[0-9a-f]{64}$/.test(s.csrf_token))throw {code:'web-native-session-invalid'};csrf=s.csrf_token}
 /** POST evaluate; one session refresh on 403 (rotated token), no other retry. */
 async function evaluate(){for(let n=0;;n++){if(!csrf)await session();let r;try{r=await fetch('/api/v1/live/alerts/evaluate',{method:'POST',credentials:'same-origin',cache:'no-store',headers:{Accept:'application/json','Content-Type':'application/json','X-Swarm-CSRF':csrf},body:'{}'})}catch{throw {code:'network'}}
  const raw=await r.text();if(r.status===403&&n===0){csrf='';continue}let body=null;try{body=JSON.parse(raw)}catch{throw {code:/^[a-z0-9-]{1,64}$/.test(raw.trim())?raw.trim():'http-'+r.status}}
  if(!r.ok)throw {code:body?.error||'http-'+r.status};return body}}
 /** Agent role or task title for an alert subject, from the last overview. */
 function subjectOf(row){const w=(last?.works||[]).find(x=>x.id===row.work_id);if(row.subject.startsWith('budget:'))return {who:tr('Budget de la mission'),w,agent:null};if(row.rule==='task-blocked'){const t=row.subject.slice(row.work_id.length+1);return {who:w?.tasks?.[t]||t,w,agent:null}}
  const a=w?.agents?.find(x=>x.id===row.subject);return {who:a?.role||row.subject,w,agent:a||null}}
 const RESOLUTION={cleared:'résolue après {d}',gone:'sujet disparu après {d}'};
 function alertItem(row,resolved){const {who,w,agent}=subjectOf(row);const time=ms=>new Date(ms).toLocaleTimeString(globalThis.SwarmI18n?.locale||'fr-FR');
  const when=resolved?tr(RESOLUTION[row.resolution]||'fermée après {d}',{d:L.duration(Math.max(0,Math.round((row.resolved_ms-row.fired_ms)/1000)))}):tr('ouverte à {t}',{t:time(row.fired_ms)});
  // Engine time of the event when recorded (e.g. agent end), next to detection time.
  const at=Date.parse(row.occurred||'');const engine=Number.isFinite(at)?' · '+tr('événement à {t}',{t:time(at)}):'';
  const state=resolved?'resolved':row.level;
  const body=[el('span',{class:'live-flag is-'+state,text:tr(RULE_LABELS[row.rule]||row.rule)}),el('strong',{text:who}),el('span',{class:'live-attn-work',text:(w?.title||row.work_id)+' · '+when+engine})];
  return el('li',{'data-alert':row.rule+'|'+row.subject,class:resolved?'is-resolved':'','data-resolution':row.resolution||''},agent?el('button',{type:'button',class:'live-alert is-'+state,onclick:()=>openHarness(agent)},...body):el('div',{class:'live-alert is-'+state},...body))}
 function alertsPanel(view){const open=view?.open||[],hist=view?.history||[];$('live-alerts-count').textContent=String(open.length)+(view?.open_more?'+':'');
  $('live-alerts').replaceChildren(...(open.length?open.map(r=>alertItem(r,false)):[el('li',{class:'live-empty',text:tr('Aucune alerte ouverte.')})]));
  $('live-alerts-history-count').textContent=String(hist.length)+(view?.history_more?'+':'');$('live-alerts-history').replaceChildren(...hist.map(r=>alertItem(r,true)))}
 /** Saved console-store bounds (settings v33) and the last purge, as recorded. */
 function retentionLine(ret){const p=ret?.policy,last=ret?.last_purge;
  const parts=!p?[tr('Conservation : non réglée (aucune purge)')]:[p.days==null?tr('Conservation : sans limite'):tr('Conservation : {n} j',{n:p.days}),p.max_events_per_attempt==null?tr('sans plafond par tentative'):tr('plafond {n} points par tentative',{n:L.number(p.max_events_per_attempt)})];
  if(last)parts.push(tr('dernière purge {t} ({n} lignes au total)',{t:new Date(last.at_ms).toLocaleString(globalThis.SwarmI18n?.locale||'fr-FR'),n:L.number(last.removed.records+last.removed.alerts+last.removed.tokens)}));
  $('live-retention').textContent=parts.join(' · ')}
 /** Server-side evaluation loop (settings v35) as last recorded by the server. */
 const SERVER_STATES={off:'Évaluation serveur : désactivée (console ouverte seulement)',starting:'Évaluation serveur : démarrage, toutes les {n} s',running:'Évaluation serveur : toutes les {n} s · dernière à {t}',error:'Évaluation serveur : échec à {t} ({c}), nouvel essai dans {n} s',stopped:'Évaluation serveur : arrêtée (réglage null) ; redémarrer le serveur pour la reprendre'};
 function serverLine(s){const box=$('live-server-eval');if(!s){box.textContent='';return}
  box.textContent=tr(SERVER_STATES[s.state]||s.state,{n:s.interval_seconds,t:s.last_ms?new Date(s.last_ms).toLocaleTimeString(globalThis.SwarmI18n?.locale||'fr-FR'):'—',c:s.code||''});box.classList.toggle('is-error',s.state==='error')}
 /** Notify open alerts this page had not seen (baseline excluded). */
 function notifyNew(open){const fresh=open.filter(r=>!knownOpen.has(r.id));knownOpen.clear();for(const r of open)knownOpen.add(r.id);
  if(alertsBaseline||!notify||!fresh.length)return;
  const list=fresh.length>NOTIFY_BATCH?[null]:fresh;
  for(const row of list){try{if(row){const {who,w}=subjectOf(row);new Notification(who+' — '+tr(RULE_LABELS[row.rule]||row.rule),{body:w?.title||row.work_id,tag:'swarm-alert-'+row.id})}else new Notification(tr('Swarm — {n} nouvelles alertes',{n:fresh.length}),{tag:'swarm-alert-batch'});notified++}catch{}}}
 /** Run one evaluation; a call arriving while one is in flight is coalesced.
  * A busy store (another writer) gets one re-evaluation after POLL_MS per
  * failure streak, skipped while paused; any other refusal stays shown. */
 async function alerts(){if(alertsBusy){alertsAgain=true;return}alertsBusy=true;
  try{const r=await evaluate();evaluations++;busyRetried=false;alertsPanel(r.alerts);retentionLine(r.retention);serverLine(r.server_evaluation);notifyNew(r.alerts?.open||[]);alertsBaseline=false;$('live-alerts-state').textContent='';$('live-alerts-state').classList.remove('is-error')}
  catch(e){if(e.code==='telemetry-store-busy'&&!busyRetried&&!retry){busyRetried=true;retry=setTimeout(()=>{retry=null;if(!paused)alerts()},POLL_MS)}
   $('live-alerts-state').textContent=tr('Évaluation des alertes impossible')+' ('+(e.code||e)+')';$('live-alerts-state').classList.add('is-error')}
  finally{alertsBusy=false;if(alertsAgain){alertsAgain=false;alerts()}}}
 /** Local check of browser notifications; records nothing on the server. */
 async function testNotification(){if(typeof Notification!=='function'){status(tr('Notifications non prises en charge par ce navigateur'),true);return}
  const p=Notification.permission==='granted'?'granted':await Notification.requestPermission();if(p!=='granted'){status(tr('Notifications refusées par le navigateur'),true);return}
  try{new Notification(tr('Swarm — notification de test'),{body:tr('Les alertes du superviseur arriveront ainsi.'),tag:'swarm-alert-test'})}catch{}}

 /** L11: failed attempts grouped by the cause the engine recorded (no model,
  * no text similarity); each group lists its attempts, newest first, which
  * open their harness. GROUPS_SHOWN bounds the panel (display choice). */
 const openCauses=new Set();
 const GROUPS_SHOWN=8,CAUSE_SOURCES={recovery:'Reprise',limit:'Limite',usage:'Usage',diagnostic:'Diagnostic',exit:'Code de sortie',stop:'Arrêt',unclassified:'Non classé'};
 function causesPanel(groups,works){groups=groups||[];$('live-causes-count').textContent=String(groups.length);
  const agentOf=m=>works.find(w=>w.id===m.work_id)?.agents?.find(a=>a.id===m.agent_id);
  $('live-causes').replaceChildren(...(groups.length?groups.slice(0,GROUPS_SHOWN).map(g=>el('li',{'data-cause':g.key},el('details',{open:openCauses.has(g.key),ontoggle:e=>{if(e.target.open)openCauses.add(g.key);else openCauses.delete(g.key)}},
   el('summary',{},el('span',{class:'live-flag is-'+(g.source==='unclassified'?'warning':'critical'),text:tr(CAUSE_SOURCES[g.source]||g.source)}),el('strong',{text:g.label||tr('cause non enregistrée')}),
    el('span',{class:'live-attn-work',text:tr(g.agents.length===1?'1 tentative':'{n} tentatives',{n:g.agents.length})+' · '+tr(g.tasks===1?'1 tâche':'{n} tâches',{n:g.tasks})})),
   el('ul',{},...g.agents.map(m=>{const a=agentOf(m),t=works.find(w=>w.id===m.work_id)?.tasks?.[m.task_id]||m.task_id;
    return el('li',{},a?el('button',{type:'button',class:'live-run-open',onclick:()=>openHarness(a)},(a.role||a.id)+' · '+t):el('span',{text:m.agent_id+' · '+t}))}))))).concat(groups.length>GROUPS_SHOWN?[el('li',{class:'live-empty',text:tr('… {n} autres causes',{n:groups.length-GROUPS_SHOWN})})]:[])
   :[el('li',{class:'live-empty',text:tr('Aucun échec enregistré.')})]))}
 /** L10 queue: finished attempts without an operator annotation, newest
  * end first; each opens its harness where the form is. QUEUE_SHOWN bounds
  * the panel (display choice); the count is complete. */
 const QUEUE_SHOWN=8;
 function queuePanel(works,state){const ended=a=>Date.parse(a.ended||'')||0;
  const todo=works.flatMap(w=>w.agents.filter(a=>!a.active&&!a.annotation).map(a=>({a,w}))).sort((x,y)=>ended(y.a)-ended(x.a));
  $('live-queue-count').textContent=String(todo.length);
  $('live-queue-state').textContent=state?tr('Annotations indisponibles')+' ('+state.code+')':'';$('live-queue-state').classList.toggle('is-error',!!state);
  $('live-queue').replaceChildren(...(todo.length?todo.slice(0,QUEUE_SHOWN).map(({a,w})=>el('li',{'data-queue':a.id},el('button',{type:'button',class:'live-run-open',onclick:()=>openHarness(a)},(a.role||a.id)+' · '+(w.tasks?.[a.task_id]||a.task_id)+' · '+L.statusLabel(a.status)))).concat(todo.length>QUEUE_SHOWN?[el('li',{class:'live-empty',text:tr('… {n} autres tentatives',{n:todo.length-QUEUE_SHOWN})})]:[])
   :[el('li',{class:'live-empty',text:tr('Toutes les tentatives terminées sont annotées.')})]))}
 /** Tokens per provider · model and the five largest consumers. */
 function consumption(works){const groups=new Map(),agents=[];
  for(const w of works)for(const a of w.agents){const t=M.tokens(a.usage);if(!t||L.usageState(a)!=='observed')continue;const n=t.input+t.output;const k=[a.provider||'?',a.reported_model?.model].filter(Boolean).join(' · ');const g=groups.get(k)||{value:0,cost:0};g.value+=n;g.cost+=t.cost;groups.set(k,g);agents.push({label:(a.role||a.id)+' — '+(w.title||w.id),value:n})}
  const byModel=[...groups].map(([label,g])=>({label,value:g.value,title:label+' : '+L.number(g.value)+' '+tr('jetons')+(g.cost?' · '+M.money(g.cost):'')})).sort((a,b)=>b.value-a.value);
  $('live-by-model').replaceChildren(byModel.length?M.bars(byModel):el('p',{class:'live-empty',text:tr('Aucun usage observé.')}));
  const top=agents.sort((a,b)=>b.value-a.value).slice(0,5);
  $('live-top').replaceChildren(top.length?M.bars(top):el('p',{class:'live-empty',text:tr('Aucun usage observé.')}))}

 /** Header pulse from `beats` (time, count); redrawn on render and on a timer
  * so the rate decays visibly when nothing happens. Presentation only. */
 function drawPulse(){const now=Date.now();while(beats.length&&beats[0].t<now-PULSE_WINDOW_MS)beats.shift();
  const minute=beats.filter(b=>b.t>=now-60000).reduce((n,b)=>n+b.n,0);$('live-pulse').textContent=L.number(minute);
  const size=PULSE_WINDOW_MS/PULSE_BUCKETS,bins=new Array(PULSE_BUCKETS).fill(0);for(const b of beats){const i=Math.min(PULSE_BUCKETS-1,Math.floor((b.t-(now-PULSE_WINDOW_MS))/size));if(i>=0)bins[i]+=b.n}
  const max=Math.max(1,...bins),svg=$('live-pulse-wave');svg.replaceChildren();const pts=bins.map((v,i)=>(i/(PULSE_BUCKETS-1)*300)+','+(44-v/max*38)).join(' ');
  for(const cls of ['live-ekg-base','live-ekg-glow']){const pl=document.createElementNS(svgNS,'polyline');pl.setAttribute('points',pts);pl.setAttribute('pathLength','400');pl.setAttribute('class',cls);svg.append(pl)}
  svg.closest('.live-metric')?.classList.toggle('is-quiet',!minute)}
 function render(data){beatNow=0;const works=data.works||[];const known=new Set([...filter.options].map(o=>o.value));
  for(const w of works)if(!known.has(w.id))filter.append(el('option',{value:w.id,text:w.title||w.id}));
  if(wanted&&!render.ready&&works.some(w=>w.id===wanted))filter.value=wanted;
  const activeAgents=works.flatMap(w=>w.agents.filter(a=>a.active));
  sparkline();
  $('live-active').textContent=L.number(activeAgents.length);
  document.title=(activeAgents.length?'('+activeAgents.length+') ':'')+tr('Swarm — agents en direct');
  const total={input:0,output:0,cache:0,cost:0};let unknown=0;
  for(const w of works)for(const a of w.agents){const t=M.tokens(a.usage);if(t&&L.usageState(a)==='observed')for(const k in total)total[k]+=t[k];else unknown++}
  $('live-tokens').textContent=L.number(total.input+total.output);$('live-tokens-bar').replaceChildren(M.tokenBar(total),M.tokenLegend(total));
  $('live-unknown').hidden=!unknown;$('live-unknown').textContent=tr('{n} agents sans usage observé',{n:unknown});$('live-cost').textContent=M.money(total.cost);
  const secs=activeAgents.reduce((n,a)=>n+(M.activeSeconds(a)||0),0);$('live-active-time').textContent=L.duration(secs);
  const res=activeAgents.map(a=>[a,M.resources(a)]).filter(([,r])=>r&&r.rss!=null);
  $('live-ram').textContent=res.length?M.bytes(res.reduce((n,[,r])=>n+r.rss,0)):'—';
  const cpus=res.map(([a,r])=>rates.rate('sum:'+a.id,r.cpu,r.at)).filter(v=>v!=null);$('live-cpu').textContent=cpus.length?cpus.reduce((a,b)=>a+b,0).toFixed(0)+' %':'—';
  stack($('live-stack'),data.totals||{},$('live-legend'));
  attentionPanel(works);consumption(works);causesPanel(data.failure_groups,works);queuePanel(works,data.annotations_state);
  // The periodic re-render (RESOURCE_MS while agents are active) rebuilds the
  // works: keep keyboard focus (agent, planned task, run button) and the task
  // chain under the pointer or focus, keyed by Work and task id.
  const host=$('live-works'),kept=keepState(host);host.replaceChildren();let shown=0;
  for(const w of works){if(filter.value&&w.id!==filter.value)continue;const agents=w.agents.filter(a=>!activeOnly.checked||a.active);const plan=activeOnly.checked?null:planGraph(w,w.agents);if(!agents.length&&!plan)continue;shown++;
   const d=depths(agents),columns=[];for(const a of [...agents].reverse()){const i=d.get(a.id);(columns[i]||=[]).push(a)}
   const graph=el('div',{class:'live-graph'},document.createElementNS(svgNS,'svg'));
   for(const col of columns)graph.append(el('div',{class:'live-col'},...(col||[]).map(a=>{const n=node(a,w.tasks||{});n.dataset.task=a.task_id;if(a.parent&&agents.some(x=>x.id===a.parent))n.dataset.parent=a.parent;return n})));
   const bar=el('div',{class:'live-stack small',role:'img'});stack(bar,w.counts||{});
   const wt=w.agents.reduce((n,a)=>{const t=M.tokens(a.usage);return n+(t?t.input+t.output:0)},0);
   host.append(el('section',{class:'live-work','aria-label':w.title||w.id,'data-work':w.id},el('header',{},el('h2',{text:w.title||w.id}),el('span',{class:'live-work-id',text:w.id}),bar,el('span',{class:'live-work-tokens',text:L.number(wt)+' '+tr('jetons')}),budgetChip(w.budget)),plan?el('h3',{class:'live-plan-title',text:tr('Plan (phase initiale)')}):null,
    // A console-store failure while judging scenarios is shown, never hidden as "no scenario".
    plan&&w.scenarios?.state==='unavailable'?el('p',{class:'live-retention is-error',text:tr('Scénarios indisponibles')+' ('+w.scenarios.code+')'}):null,plan,plan?blockers(w.plan):null,plan?deliveries(w):null,runsPanel(w),datasetExport(w),plan?el('h3',{class:'live-plan-title',text:tr('Agents')}):null,graph));
   requestAnimationFrame(()=>{connectors(graph);if(plan){planConnectors(plan);if(kept.chain?.work===w.id&&plan.querySelector('.live-task[data-task="'+CSS.escape(kept.chain.task)+'"]'))chain(plan,kept.chain.task)}})}
  $('live-empty').hidden=shown>0;
  if(kept.focus)host.querySelector(kept.focus)?.focus({preventScroll:true});
  if(render.ready&&beatNow)beats.push({t:Date.now(),n:beatNow});drawPulse();
  render.ready=true;
  return res.length>0}

 /** Focus selector and highlighted task chain to restore after a re-render;
  * presentation only. */
 function keepState(host){const a=document.activeElement,work=a?.closest?.('.live-work')?.dataset.work,css=CSS.escape;let focus=null;
  if(a?.dataset?.agent)focus='[data-agent="'+css(a.dataset.agent)+'"]';
  else if(work&&a.classList.contains('live-task'))focus='.live-work[data-work="'+css(work)+'"] .live-task[data-task="'+css(a.dataset.task)+'"]';
  else if(work&&a.classList.contains('live-run-open'))focus='.live-work[data-work="'+css(work)+'"] tr[data-run="'+css(a.closest('tr').dataset.run)+'"] .live-run-open';
  const t=host.querySelector('.live-task:hover')||(a?.classList?.contains('live-task')?a:null);
  return {focus,chain:t?{work:t.closest('.live-work')?.dataset.work,task:t.dataset.task}:null}}

 async function tick(){timer=null;if(paused||document.hidden||inflight)return;inflight=true;let live=false;
  try{last=await L.getJSON('/api/v1/live/overview');if(!history.length)sample();live=render(last);alerts();status(tr('Mis à jour à')+' '+new Date().toLocaleTimeString(globalThis.SwarmI18n?.locale||'fr-FR'),false)}
  catch(e){status(tr('Lecture impossible')+' ('+(e.code||e.message||e)+')',true);
   // Saturation is transient: re-read after one cadence even in push mode.
   if(e.code==='web-workers-saturated'&&!timer)timer=setTimeout(tick,POLL_MS)}
  finally{inflight=false;if(again){again=false;tick()}else schedule(live)}}
 function status(text,error){const m={push:tr('direct poussé'),polling:tr('interrogation périodique'),reconnecting:tr('reconnexion'),connecting:tr('connexion')}[mode];$('live-status').textContent=text+' · '+m;$('live-status').classList.toggle('is-error',error)}
 /** Polling when push is unavailable; in push mode only resources need a timer. */
 function schedule(live){if(paused||document.hidden||timer)return;if(mode==='polling')timer=setTimeout(tick,POLL_MS);else if(live)timer=setTimeout(tick,RESOURCE_MS)}
 /** Sparkline samples the last known overview every POLL_MS, independent of
  * refresh mode, so its time axis stays regular without extra requests. */
 function sample(){if(!last||paused)return;history.push((last.works||[]).reduce((n,w)=>n+w.agents.filter(a=>a.active).length,0));while(history.length>HISTORY)history.shift();sparkline()}
 setInterval(sample,POLL_MS);
 function connect(){stream?.close();stream=L.liveEvents(()=>{if(paused)return;if(document.hidden){alerts();return}if(inflight)again=true;else{clearTimeout(timer);timer=null;tick()}},(next,reason)=>{mode=next;if(next==='polling'){stream=null;schedule(false);status(tr('Direct poussé indisponible')+(reason?' ('+reason+')':''),false)}})}

 $('live-pause').addEventListener('click',e=>{paused=!paused;e.currentTarget.setAttribute('aria-pressed',String(paused));e.currentTarget.textContent=paused?tr('Reprendre le direct'):tr('Mettre en pause');if(!paused){if(mode!=='polling'&&!stream)connect();tick()}else{clearTimeout(timer);timer=null;stream?.close();stream=null;mode='connecting';$('live-status').textContent=tr('Direct en pause')}});
 $('live-theme').addEventListener('click',L.toggleTheme);
 filter.addEventListener('change',()=>{syncUrl();last&&render(last)});
 activeOnly.addEventListener('change',()=>{L.storage.set('swarm.live.active-only',activeOnly.checked?'1':'0');syncUrl();last&&render(last)});
 setNotify(notify);
 $('live-notify').addEventListener('click',async()=>{if(notify){setNotify(false);return}if(typeof Notification!=='function'){status(tr('Notifications non prises en charge par ce navigateur'),true);return}
  const p=Notification.permission==='granted'?'granted':await Notification.requestPermission();setNotify(p==='granted');if(p!=='granted')status(tr('Notifications refusées par le navigateur'),true)});
 $('live-export').addEventListener('click',download);
 $('live-alerts-test').addEventListener('click',testNotification);
 /** Keyboard: p pause, a active-only, t theme, n notifications, e export, arrows move between agents. */
 addEventListener('keydown',e=>{if(e.ctrlKey||e.metaKey||e.altKey||/INPUT|TEXTAREA|SELECT/.test(document.activeElement?.tagName||''))return;
  const key={p:'live-pause',t:'live-theme',n:'live-notify',e:'live-export'}[e.key];if(key){e.preventDefault();$(key).click();return}
  if(e.key==='a'){e.preventDefault();activeOnly.click();return}
  if(['ArrowRight','ArrowLeft','ArrowDown','ArrowUp'].includes(e.key)){const nodes=[...document.querySelectorAll('.live-node')];if(!nodes.length)return;e.preventDefault();const i=nodes.indexOf(document.activeElement);const step=/Right|Down/.test(e.key)?1:-1;nodes[(i<0?(step>0?0:nodes.length-1):(i+step+nodes.length)%nodes.length)].focus()}});
 globalThis.SwarmLiveDashboard={exportJSON,notified:()=>notified,evaluations:()=>evaluations,favicon:()=>$('live-favicon').href};
 document.addEventListener('visibilitychange',()=>{if(!document.hidden)tick()});
 // Plan graphs draw dependency arrows, agent graphs parent links: redraw each
 // with its own painter (a shared one erased the plan arrows on resize).
 // Decay the header pulse between renders (display cadence, no network).
 setInterval(()=>{if(render.ready)drawPulse()},5000);
 addEventListener('resize',()=>document.querySelectorAll('.live-graph').forEach(g=>g.classList.contains('live-plan')?planConnectors(g):connectors(g)));
 connect();tick();
})();
