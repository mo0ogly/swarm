'use strict';
/** Execute actual availability/guard functions with explicit DOM doubles.
 * No backend, authentication or browser layout is inferred from these checks. */
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const code=fs.readFileSync('web/trust-native.js','utf8');
const guarded=code.slice(code.indexOf('async function guarded(action)'),code.indexOf("$('operation').addEventListener"));
const identity=code.slice(code.indexOf('function identitySettings()'),code.indexOf('/** Render public lifecycle'));
const helper=code.match(/function syncEnrolledAvailability\(\)\{[\s\S]*?\n\}/)?.[0]||'';
/** Exercise actual functions on explicit DOM doubles; observe outside guarded
 * so its normal exception handling cannot turn a failed assertion into a pass. */
async function scenario(source,fail=false,rerender=false){
 const elements={};for(const id of ['trust-enrolled-execute','trust-enrolled-operation','trust-identity-availability','trust-identity-sources','save','settings','admin-message','message','trust-enrolled-password','trust-enrolled-request'])elements[id]={disabled:false,value:'private-fixture-text',textContent:'',open:true,replaceChildren(){},append(){},options:[{value:'mandate-create'}]};
 elements['trust-enrolled-execute'].disabled=true;
 const buttons=[elements['save'],elements['trust-enrolled-execute']];
 const ctx={busy:false,state:{identities:[],identity_authentication:{held_enrollment:source}},proposal:null,$:id=>elements[id],t:s=>s,document:{querySelectorAll:()=>buttons,createElement:()=>({})},syncGovernance(){}};
 vm.createContext(ctx);vm.runInContext(helper+'\n'+identity+'\n'+guarded,ctx);vm.runInContext('identitySettings()',ctx);
 const initial=elements['trust-enrolled-execute'].disabled;let during=false,afterRender=true;await ctx.guarded(async()=>{during=buttons.every(b=>b.disabled);if(rerender){ctx.identitySettings();afterRender=elements['trust-enrolled-execute'].disabled;}if(fail)throw Error('fixture-action-failed');});assert(during,'buttons disabled during request');assert(afterRender,'render cannot reenable during busy request');
 assert.equal(elements['trust-enrolled-password'].value,'private-fixture-text');assert.equal(elements['trust-enrolled-request'].value,'private-fixture-text');return {initial,disabled:elements['trust-enrolled-execute'].disabled};
}
(async()=>{
 const bound={http_available:true,scope:'single_prepared_intent',operation:'mandate-create'};
 assert.deepEqual(await scenario(bound),{initial:false,disabled:false});console.log('PASS positive-bound-source');
 await scenario(bound,false,true);console.log('PASS bound-render-during-busy');
 assert.deepEqual(await scenario(bound,true),{initial:false,disabled:false});console.log('PASS bound-after-error');
 for(const [name,source] of [['absent',undefined],['inactive',{http_available:false,scope:'single_prepared_intent'}],['wrong-scope',{http_available:true,scope:'arbitrary'}]])for(const fail of [false,true]){
  assert.deepEqual(await scenario(source,fail),{initial:true,disabled:true},name+' after '+(fail?'error':'success'));console.log('PASS '+name+'-'+(fail?'error':'success'));
 }
 console.log('AVAILABILITY_PASS');
})().catch(e=>{console.error(e);process.exitCode=1;});
