'use strict';
/** Deterministic owned-resource doubles; no browser, OS process or live Root stopped. */
const assert=require('node:assert/strict'),{finishOwnedResources}=require('./helpers/verification_cleanup.cjs');
(async()=>{
 let cases=0,steps=[];
 // Valid witness before each injected close/finalize/shape failure.
 const witness=async()=>{steps=[];await finishOwnedResources(undefined,[async()=>steps.push('first'),async()=>steps.push('second')],async failures=>{assert(Object.isFrozen(failures));assert.equal(failures.length,0);steps.push('evidence')});assert.deepEqual(steps,['first','second','evidence'])};
 await witness();cases++;console.log('PASS cleanup success preserves order');
 await witness();const primary=Error('original');steps=[];await assert.rejects(()=>finishOwnedResources({error:primary},[async()=>steps.push('closed')],async f=>{assert.deepEqual(f,[primary]);steps.push('retained')}),e=>e instanceof AggregateError&&e.errors.length===1&&e.errors[0]===primary&&e.cause===primary);assert.deepEqual(steps,['closed','retained']);cases++;console.log('PASS cleanup primary retained');
 await witness();const close1=Error('close1'),close2=Error('close2'),write=Error('evidence write');steps=[];await assert.rejects(()=>finishOwnedResources({error:primary},[async()=>{steps.push('first');throw close1},async()=>{steps.push('second');throw close2}],async f=>{assert.deepEqual(f,[primary,close1,close2]);steps.push('retained');throw write}),e=>e instanceof AggregateError&&e.errors.length===4&&e.errors.every((v,i)=>v===[primary,close1,close2,write][i]));assert.deepEqual(steps,['first','second','retained']);cases++;console.log('PASS cleanup all secondary and evidence failures retained');
 await witness();await assert.rejects(()=>finishOwnedResources({error:undefined},[],async f=>assert.deepEqual(f,[undefined])),e=>e instanceof AggregateError&&e.errors.length===1&&e.errors[0]===undefined);cases++;console.log('PASS cleanup thrown undefined stays failure');
 await witness();steps=[];await assert.rejects(()=>finishOwnedResources(undefined,[null,async()=>steps.push('later')],async f=>{assert(f[0] instanceof TypeError);steps.push('evidence')}),e=>e instanceof AggregateError&&e.errors[0] instanceof TypeError);assert.deepEqual(steps,['later','evidence']);cases++;console.log('PASS cleanup malformed callback does not skip later cleanup');
 await witness();steps=[];await assert.rejects(()=>finishOwnedResources(null,[async()=>steps.push('closed')]),e=>e instanceof AggregateError&&e.errors[0]===null&&e.errors[1] instanceof TypeError);assert.deepEqual(steps,['closed']);cases++;console.log('PASS cleanup invalid primary preserves input and closes resources');
 await witness();console.log('PASS verification cleanup '+cases+' cases');
})().catch(error=>{console.error(error);process.exitCode=1});
