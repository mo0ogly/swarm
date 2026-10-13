'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
class Element{constructor(tag,text,kind){this.tag=tag;this.textContent=text??'';this.className=kind||'';this.children=[];this.dataset={};this.events={}}append(...x){this.children.push(...x)}addEventListener(type,fn){this.events[type]=fn}}
const walk=e=>[e,...e.children.flatMap(walk)];
for(const language of ['fr','en']){
 const en=JSON.parse(fs.readFileSync('locales/en.json','utf8'));const translate=s=>language==='en'?(en[s]??s):s;
 const context=vm.createContext({Mission:{readableDate:s=>s},PilotActions:{},SwarmI18n:{t:translate},node:(...a)=>new Element(...a),missionText:translate});
 vm.runInContext(fs.readFileSync('web/mission-insights.js','utf8'),context);
 const calls=[{id:'planning/missing',record_type:'planning_reservation',kind:'planner',role:'planner',scope:'root',state:'estimated',usage:null},{id:'agent/reported',record_type:'producer_attempt',kind:'worker',role:'worker',provider:'codex',task:'<img src=x onerror=alert(1)>',attempt:'attempt-1',state:'completed',requested_model:'configured-model',usage:{input_tokens:100,output_tokens:25,cached_input_tokens:40,provider_reported_cost_usd:0}},{id:'planning/reviewer',record_type:'planning_reservation',kind:'reviewer',role:'reviewer',state:'finished',usage:{input_tokens:20,output_tokens:0}}];
 const view=context.Mission.spendingView({rows:[],attempts:[],calls,recorded_control_executions:0,worker_retries:0,note:'Recorded calls are not sent inferences'});
 const details=walk(view).find(e=>e.dataset.callLedger==='calls');assert.ok(details,'actual spending view must expose detailed calls');
 details.open=true;details.events.toggle?.();
 const cards=walk(details).filter(e=>e.dataset.providerCall);assert.equal(cards.length,3,'one visible record per reservation/attempt');
 const text=e=>walk(e).map(x=>x.textContent).join('\n');
 const absent=cards.find(e=>e.dataset.providerCall==='planning/missing');assert.match(text(absent),language==='fr'?/Usage non rapporté/:/Usage not reported/);assert.match(text(absent),language==='fr'?/Coût non rapporté/:/Cost not reported/);assert.doesNotMatch(text(absent),/0.*USD|0 \/ 0/);
 const received=cards.find(e=>e.dataset.providerCall==='agent/reported');assert.match(text(received),/100 \/ 25/);assert.match(text(received),/40/);assert.match(text(received),/0.*USD/);assert.match(text(received),/<img src=x/);assert.equal(walk(received).filter(e=>e.tag==='img').length,0,'untrusted task is text');assert.doesNotMatch(text(received),/140 \/ 25/,'cache is not added to raw input');
 const reviewer=cards.find(e=>e.dataset.providerCall==='planning/reviewer');assert.match(text(reviewer),language==='fr'?/Coût non rapporté/:/Cost not reported/);assert.doesNotMatch(text(reviewer),/0.*USD/);assert.match(text(reviewer),language==='fr'?/Modèle historique non attribuable/:/Historical model not attributable/);
 details.open=false;details.events.toggle?.();details.open=true;details.events.toggle?.();assert.equal(walk(details).filter(e=>e.dataset.providerCall).length,3,'reopening must not duplicate ledger entries');
}
console.log('PASS FR/EN detailed calls, unknown usage/cost, explicit zero, cache semantics and safe text');
