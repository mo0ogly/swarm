'use strict';
// Shared indicators for the live views: durations, token split, CPU rates,
// engine-flagged attention signals and small bar/meter renderers. Display only:
// every value comes from the recorded agent view; nothing is estimated.
(function(root){
 const L=root.SwarmLive,{tr,el}=L;
 /** Input/output tokens and the cache figure as reported. Cache is shown apart:
  * depending on the provider it is a subset of, or separate from, input. */
 function tokens(u){if(!u)return null;const n=v=>Number(v)||0;return {input:n(u.input_tokens),output:n(u.output_tokens),cache:n(u.cache_read_input_tokens)+n(u.cache_creation_input_tokens)+n(u.cached_input_tokens),cost:Number(u.provider_reported_cost_usd)||0}}
 /** Activity seconds: started → ended (or now while active); falls back to the
  * engine's elapsed_seconds when no timestamp is recorded. */
 function activeSeconds(a,now=Date.now()){const s=Date.parse(a.started);if(Number.isFinite(s)){const e=a.ended?Date.parse(a.ended):(a.active?now:NaN);if(Number.isFinite(e))return Math.max(0,Math.round((e-s)/1000))}return a.progress?.elapsed_seconds??null}
 function bytes(n){if(n==null)return '—';const u=['o','Kio','Mio','Gio','Tio'];let i=0,v=n;while(v>=1024&&i<u.length-1){v/=1024;i++}return (i?v.toFixed(v<10?1:0):v)+' '+tr(u[i])}
 function money(v){return v?new Intl.NumberFormat(root.SwarmI18n?.locale||'fr-FR',{style:'currency',currency:'USD',maximumFractionDigits:v<1?4:2}).format(v):'—'}
 /** Sum of verified process numbers for one agent view; null when nothing was read. */
 function resources(a){const r=a.resources;if(!r||r.observation!=='observed')return null;const s=[r.supervisor,r.provider].filter(p=>p?.state==='observed');if(!s.length)return {rss:null,cpu:null,at:r.sampled_at_ms,states:[r.supervisor?.state,r.provider?.state]};
  const sum=k=>s.some(p=>p[k]==null)?null:s.reduce((n,p)=>n+p[k],0);return {rss:sum('rss_bytes'),cpu:sum('cpu_ns'),threads:sum('threads'),descendants:r.provider?.descendants??0,at:r.sampled_at_ms,states:[r.supervisor?.state,r.provider?.state]}}
 /** CPU % of one core between two samples of cumulative on-CPU nanoseconds. */
 class Rates{constructor(){this.prev=new Map()}
  rate(id,ns,at){if(ns==null||!at)return null;const p=this.prev.get(id);this.prev.set(id,{ns,at});if(!p||at<=p.at||ns<p.ns)return p&&at===p.at?p.value??null:null;const v=(ns-p.ns)/((at-p.at)*1e6)*100;this.prev.get(id).value=v;return v}}
 /** Engine-recorded reasons an agent needs attention, most severe first. */
 function attention(a){const out=[],p=a.progress||{},d=a.diagnostic||{},r=a.resources;
  if(a.status==='failed')out.push({level:'critical',text:tr('Échec')});
  if(d.limit_reached)out.push({level:'critical',text:tr("Limite d'erreurs atteinte")});
  if(p.time_threshold_exceeded)out.push({level:'warning',text:tr('Durée au-delà du seuil')});
  if(p.tool_time_threshold_exceeded)out.push({level:'warning',text:tr('Attente outil au-delà du seuil')});
  if(d.observed_errors)out.push({level:'warning',text:tr('{n} erreurs observées',{n:d.observed_errors})});
  if(['invalid','incomplete'].includes(L.usageState(a)))out.push({level:'warning',text:tr('Usage')+' '+L.factLabel(L.usageState(a))});
  if(r?.provider?.state==='identity-mismatch'||r?.supervisor?.state==='identity-mismatch')out.push({level:'warning',text:tr('Processus non vérifiable')});
  if(a.active&&r?.observation==='observed'&&r.provider?.state==='ended'&&r.supervisor?.state==='ended')out.push({level:'warning',text:tr('Processus terminés, agent encore actif')});
  return out}
 /** Two-segment input/output bar; the cache figure is text in the legend. */
 function tokenBar(t,small){const bar=el('div',{class:'live-tokbar'+(small?' small':''),role:'img','aria-label':tr('Jetons')+' : '+tr('entrée')+' '+L.number(t.input)+', '+tr('sortie')+' '+L.number(t.output)});
  for(const [k,v,c] of [['entrée',t.input,'live-s1'],['sortie',t.output,'live-s2']])if(v){const s=el('span',{class:c,title:tr(k)+' : '+L.number(v)});s.style.flexGrow=String(v);bar.append(s)}return bar}
 function tokenLegend(t){return el('ul',{class:'live-legend'},el('li',{},el('i',{class:'live-dot live-s1'}),tr('entrée')+' ',el('b',{text:L.number(t.input)})),el('li',{},el('i',{class:'live-dot live-s2'}),tr('sortie')+' ',el('b',{text:L.number(t.output)})),t.cache?el('li',{},tr('cache')+' ',el('b',{text:L.number(t.cache)})):null)}
 /** Horizontal magnitude bars, one hue, value labels always visible. */
 function bars(rows,format=L.number){const max=Math.max(1,...rows.map(r=>r.value));return el('ol',{class:'live-bars'},...rows.map(r=>{const fill=el('span',{class:'live-bar-fill'});fill.style.width=(r.value/max*100).toFixed(1)+'%';return el('li',{title:r.title||r.label},el('span',{class:'live-bar-label',text:r.label}),el('span',{class:'live-bar-track'},fill),el('b',{class:'live-bar-value',text:format(r.value)}))}))}
 /** Consumption vs the agent's own recorded limit; plain count when unlimited. */
 function meter(label,value,max){if(!(max>0))return el('span',{class:'live-meter-text',text:label+' '+L.number(value||0)});const pct=Math.min(100,(value||0)/max*100);const fill=el('span',{class:'live-meter-fill'+(pct>=90?' is-high':'')});fill.style.width=pct.toFixed(1)+'%';
  return el('span',{class:'live-meter',role:'meter','aria-valuemin':'0','aria-valuemax':String(max),'aria-valuenow':String(value||0),'aria-label':label},el('span',{class:'live-meter-label',text:label+' '+L.number(value||0)+'/'+L.number(max)}),el('span',{class:'live-meter-track'},fill))}
 /** Re-render elements carrying data-since (ms epoch) every second. */
 function tickDurations(){const now=Date.now();for(const e of document.querySelectorAll('[data-since]')){const s=Number(e.dataset.since);if(s)e.textContent=L.duration(Math.max(0,Math.round((now-s)/1000)))}}
 setInterval(tickDurations,1000);
 root.SwarmLiveMetrics={tokens,activeSeconds,bytes,money,resources,Rates,attention,tokenBar,tokenLegend,bars,meter,tickDurations};
})(globalThis);
