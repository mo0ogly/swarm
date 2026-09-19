#!/usr/bin/env node
'use strict';

const assert=require('node:assert/strict');
const fs=require('node:fs');
const vm=require('node:vm');

class Element{
 constructor(tag,text='',kind=''){this.tag=tag;this.textContent=text||'';this.className=kind||'';this.children=[];this.dataset={};this.attributes={};}
 get childNodes(){return this.children}
 append(...children){this.children.push(...children)}
 replaceChildren(...children){this.children=children}
 setAttribute(name,value){this.attributes[name]=value}
}
const node=(tag,text,kind)=>new Element(tag,text,kind);
const statusCode=fs.readFileSync('web/status-contract.js','utf8')+';SwarmStatusContract';
const states=vm.runInNewContext(statusCode);

const fr=s=>s;
const en=s=>({
 'Tous les résultats sont validés.':'All results are validated.',
 'La validation finale reste à obtenir.':'Final validation is still required.',
 ' tâches validées.':' validated tasks.',
 ' résultat(s) doivent être vérifiés à nouveau.':' result(s) must be checked again.',
 ' Des agents travaillent actuellement.':' Agents are currently working.'
}[s]||s);

for(const translate of [fr,en]){
 const completed=new Element('p');
 states.renderValidation(completed,{state:'validated',validated:2,stale:0},2,false,translate);
 assert.equal(completed.dataset.validationState,'validated');
 assert.equal(completed.className,'notice success');
 assert.match(completed.textContent,translate===en?/All results/:/Tous les résultats/);

 const partial=new Element('p');
 states.renderValidation(partial,{state:'partial',validated:1,stale:0},2,false,translate);
 assert.equal(partial.dataset.validationState,'partial');
 assert.equal(partial.className,'notice attention');

 const stale=new Element('p');
 states.renderValidation(stale,{state:'blocked',validated:0,stale:1},1,false,translate);
 assert.equal(stale.dataset.validationState,'blocked');
 assert.match(stale.textContent,/1/);
}
assert.throws(()=>states.validation({state:'VALIDÉ'}),/unknown validation state/,
 'a translated label must never be accepted as a business state');
assert.throws(()=>states.validation({state:'future-state'}),/unknown validation state/,
 'unknown states must fail explicitly');

assert.equal(states.planning({failure:'provider down',paused:false,scopes:[]}), 'error');
assert.equal(states.planning({paused:true,scopes:[]}), 'paused');
assert.equal(states.planning({paused:false,scopes:[{state:'closed'}]}), 'completed');
assert.equal(states.planning({paused:false,scopes:[{state:'ready'}]}), 'active');

const noReview=states.review({reviewer:null},[],false);
assert.deepEqual({...noReview},{availability:'absent',state:'absent',message:''});
const reviewer={provider:'fixture',calls:0,max_calls:2};
assert.equal(states.review({reviewer},[],false).state,'idle');
assert.equal(states.review({reviewer},[{independent_review:{state:'running'}}],false).state,'running');
assert.equal(states.review({reviewer:{...reviewer,failure:'HTTP 503'}},[],false).state,'error');
assert.equal(states.review({reviewer},[],true).state,'paused');
assert.equal(states.review({reviewer},[{independent_review:{state:'passed'}}],false).state,'recorded');

// Exercise the real Planning.roles DOM builder, not a source-text assertion.
const planningCode=fs.readFileSync('web/planning.js','utf8')+';Planning';
function renderRoles(planning,tasks=[],paused=false){
 const context={SwarmStatusContract:states,node,snapshot:{paused,work:{tasks}},tr_web_planning_js:s=>s};
 const Planning=vm.runInNewContext(planningCode,context);
 const panel=new Element('section');Planning.roles(panel,planning);return panel.children[0].children.filter(x=>x?.dataset?.role==='reviewer')[0];
}
const absent=renderRoles({provider:'fixture',decisions:0,reviewer_required:true});
assert.equal(absent.dataset.reviewAvailability,'absent');
assert.equal(absent.dataset.reviewState,'absent');
assert.match(absent.children[0].textContent,/absent/);
const configured=renderRoles({provider:'fixture',decisions:0,reviewer});
assert.equal(configured.dataset.reviewAvailability,'configured');
assert.equal(configured.dataset.reviewState,'idle');
assert.match(configured.children[1].textContent,/Aucun avis pour le moment/);
assert.doesNotMatch(configured.children[1].textContent,/Attend un résultat à examiner/);
const errored=renderRoles({provider:'fixture',decisions:0,reviewer:{...reviewer,failure:'HTTP 503'}});
assert.equal(errored.dataset.reviewState,'error');
assert.match(errored.children[1].textContent,/Vérification interrompue/);
const paused=renderRoles({provider:'fixture',decisions:0,reviewer},[],true);
assert.equal(paused.dataset.reviewState,'paused');
assert.match(paused.children[1].textContent,/Mission en pause/);

console.log('PASS states DOM: invariant verdicts, FR/EN, complete, partial, stale, error, pause and reviewer absent/configured');
