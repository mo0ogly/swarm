'use strict';
/** Consumer regression with an in-memory API double. Checks exact wire bytes,
 * readonly preview routing and invalidation; does not establish native storage. */
const assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
async function scenario(){
 const nodes=new Map();
 function element(id){if(!nodes.has(id))nodes.set(id,{value:'',disabled:false,selectedOptions:[],listeners:{},addEventListener(n,f){this.listeners[n]=f;},replaceChildren(){},textContent:''});return nodes.get(id);}
 let pending=null,hold=false;const calls=[];
 const context={document:{getElementById:element,createElement:()=>({})},crypto:{randomUUID:()=> 'fixture-event'},console};
 vm.runInNewContext(fs.readFileSync('web/trust-confidentiality.js','utf8'),context);
 const api=async(op,raw)=>{calls.push({op,raw});if(op==='blp-show')return {result:{trust_revision:'9007199254740993',capability:{policy_revision:'9007199254740995',runtime_enforced:false,preparation_quarantined:false},record:{history:[{policy:{levels:[{id:'confidential'}],compartments:[]}}]}}};
 if(op==='blp-policy-preview')return {result:{applied:false,runtime_enforced:false}};
 if(hold)return new Promise(resolve=>{pending=resolve;});
 return {result:op==='blp-object-attribute-preview'?{preview:true,applied:false,attributed:false,would_attribute:true,runtime_enforced:false}:{attributed:true,runtime_enforced:false}};};
 context.SwarmTrustConfidentiality.install({api,guarded:f=>f(),t:s=>s});
 await element('blp-load').listeners.click();
 element('blp-object-digest').value='a'.repeat(64);element('blp-object-path').value='/fixture';element('blp-object-level').value='confidential';element('blp-object-reason').value='Explicit fixture';
 await element('blp-attribute-preview').listeners.click();
 assert.equal(calls.at(-1).op,'blp-object-attribute-preview','preview must route to readonly producer');
 const preview=calls.at(-1).raw;
 assert.match(preview,/"expected_revision":9007199254740995[,}]/);assert.match(preview,/"expected_trust_revision":9007199254740993[,}]/);
 assert.equal(element('blp-attribute-apply').disabled,false);
 element('configuration').listeners.input();assert.equal(element('blp-attribute-apply').disabled,true,'field edit disarms attribution');
 await element('blp-preview').listeners.click();assert.equal(element('blp-save').disabled,false);await element('blp-attribute-preview').listeners.click();element('trust-admin-token').listeners.input();assert.equal(element('blp-save').disabled,true,'relocated token edit disarms policy');assert.equal(element('blp-attribute-apply').disabled,true,'relocated token edit disarms attribution');
 hold=true;const waiting=element('blp-attribute-preview').listeners.click();await Promise.resolve();element('configuration').listeners.input();pending({result:{preview:true,applied:false,attributed:false,would_attribute:true,runtime_enforced:false}});await waiting;
 assert.equal(element('blp-attribute-apply').disabled,true,'late preview cannot rearm edited proposal');
 hold=false;await element('blp-attribute-preview').listeners.click();const exact=calls.at(-1).raw;await element('blp-attribute-apply').listeners.click();assert.equal(calls.at(-2).op,'blp-object-attribute');assert.equal(calls.at(-2).raw,exact,'apply uses exact preview bytes');
 console.log('PASS BLP consumer routing, exact revisions, field/token/inflight invalidation and exact apply');
}
scenario().catch(e=>{console.error(e);process.exitCode=1;});
