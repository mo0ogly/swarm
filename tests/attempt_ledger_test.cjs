'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
class Element{constructor(tag,text,kind){this.tag=tag;this.textContent=text||'';this.className=kind||'';this.children=[];this.dataset={}}append(...children){this.children.push(...children)}}
const en=JSON.parse(fs.readFileSync('locales/en.json','utf8'));
for(const lang of ['fr','en']){
 const translate=s=>lang==='en'?(en[s]??s):s;
 const context=vm.createContext({Mission:{},PilotActions:{},SwarmI18n:{t:translate},node:(...args)=>new Element(...args),missionText:translate});
 vm.runInContext(fs.readFileSync('web/mission-insights.js','utf8'),context);
 const rows=[{label:'Task',agent:'a',attempt:'failed',process_state:'interrupted',task_acceptance_recorded:true,observed_tool_calls:25,measured:false,tests:'inconnu : aucun signal fiable ne distingue un test dans les commandes observées',usage_missing:true},{label:'Task',agent:'b',attempt:'finished',process_state:'completed',task_acceptance_recorded:true,observed_tool_calls:3,measured:true,measurement:'partial',tool_reads:1,tool_writes:1,tool_unclassified:1,tool_errors_total:1,tool_repeats_total:0,tests:'unknown',usage_missing:false,input_tokens:100,output_tokens:20,cost:{attempts_with_cost:0}}];
 const view=context.Mission.attemptsView(rows);
 const flatten=e=>[e.textContent,...e.children.flatMap(flatten)];const text=flatten(view).join('\n');
 assert.equal(view.children.filter(e=>e.dataset.attemptLedger).length,2);
 assert.match(text,/failed/);assert.match(text,/finished/);assert.match(text,/25/);
 assert.match(text,lang==='fr'?/inconnues, pas zéro/:/unknown, not zero/);
 assert.match(text,lang==='fr'?/Mesure partielle/:/Partial measurement/);
 assert.match(text,lang==='fr'?/toutes tentatives/:/all attempts/);
 assert.match(text,lang==='fr'?/Coût non rapporté/:/Cost not reported/);
 assert.doesNotMatch(text,/0\.00 USD/);
 if(lang==='en')assert.doesNotMatch(text,/jetons entrée|: oui/);
}
console.log('PASS attempt histories, missing metrics/costs and partial measurements in FR/EN');

// Exercise the actual close handler after the refresh replaced its trigger.
const cockpit=fs.readFileSync('web/cockpit.js','utf8');
const closeSource=cockpit.slice(cockpit.indexOf('function closeModal(){'),cockpit.indexOf('function preview(text)'));
let focused;const replacement={dataset:{missionAction:'spending'},focus(){focused='spending'}};
const focusContext=vm.createContext({returnFocus:{isConnected:false,dataset:{missionAction:'spending'}},modalContext:{},document:{querySelectorAll:selector=>selector==='[data-mission-action]'?[replacement]:[]},$:id=>id==='modal'?{close(){}}:{focus(){focused='fallback'}}});
vm.runInContext(closeSource+';closeModal()',focusContext);assert.equal(focused,'spending');
console.log('PASS spending modal returns focus to the refreshed trigger');
