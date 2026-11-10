'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(require('node:path').join(__dirname,'../web/web-requests.js'),'utf8');
function fixture(values){
 let now=0,seq=0;const timers=new Map(),calls=[];
 const ctx=vm.createContext({location:{origin:'http://local.test'},URL,AbortController,DOMException,performance:{now:()=>now},SwarmWebRequestConfig:{revision:3,values},
  setTimeout:(fn,delay)=>{timers.set(++seq,{fn,delay});return seq},clearTimeout:id=>timers.delete(id),
  fetch:async(path,options)=>{calls.push({path,options});return {status:200,text:()=>new Promise((resolve,reject)=>{const abort=()=>reject(options.signal.reason);if(options.signal.aborted)abort();else options.signal.addEventListener('abort',abort,{once:true});calls.at(-1).complete=resolve})}}
 });vm.runInContext(source,ctx);
 return {policy:ctx.SwarmWebRequests,timers,calls,advance:async ms=>{now+=ms;const [id,t]=timers.entries().next().value;timers.delete(id);t.fn();await Promise.resolve()}};
}
(async()=>{
 const f=fixture({general_ms:73,snapshot_ms:101,connection_test_ms:127});
 assert.equal(f.policy.duration('/api/v1/snapshot?work=fixture'),101);assert.equal(f.policy.duration('/api/v1/providers/connections/test?debug=1'),127);assert.equal(f.policy.duration('/api/v1/works'),73);
 const waiting=f.policy.text('/api/v1/works',{headers:{'X-Fixture':'received'}});await new Promise(resolve=>setImmediate(resolve));assert.equal(f.timers.size,1);assert.equal(f.calls[0].options.headers['X-Fixture'],'received');
 const expires=assert.rejects(waiting,e=>e.name==='TimeoutError');await f.advance(73);await expires;assert.equal(f.timers.size,0);assert.equal(f.calls.length,1,'a timeout must never replay the operation');
 f.policy.adopt({revision:4,values:{general_ms:0,snapshot_ms:0,connection_test_ms:0}});
 const unlimited=f.policy.text('/api/v1/works');await new Promise(resolve=>setImmediate(resolve));assert.equal(f.timers.size,0);f.calls.at(-1).complete('body fully received');assert.equal((await unlimited).value,'body fully received');
 const long=fixture({general_ms:0x7fffffff+300,snapshot_ms:0,connection_test_ms:0});const pending=long.policy.text('/api/v1/works');await new Promise(resolve=>setImmediate(resolve));assert.equal([...long.timers.values()][0].delay,0x7fffffff);await long.advance(0x7fffffff);assert.equal(long.calls[0].options.signal.aborted,false);assert.equal([...long.timers.values()][0].delay,300);long.calls[0].complete('long timer respected');await pending;assert.equal(long.timers.size,0);
 const cancelled=fixture({general_ms:100,snapshot_ms:0,connection_test_ms:0}),external=new AbortController();const request=cancelled.policy.text('/api/v1/works',{signal:external.signal});await new Promise(resolve=>setImmediate(resolve));const rejected=assert.rejects(request,e=>e.message==='operator cancelled');external.abort(new Error('operator cancelled'));await rejected;assert.equal(cancelled.timers.size,0);
 for(const v of [-1,1.5,Number.MAX_SAFE_INTEGER+1])assert.throws(()=>f.policy.adopt({revision:4,values:{general_ms:v,snapshot_ms:0,connection_test_ms:0}}));
 const missing=fixture(undefined);assert.throws(()=>missing.policy.duration('/api/v1/works'),'no hidden fallback when policy is unavailable');
 console.log('PASS: received category values, full body timeout, zero, long timer chunks, cancellation, cleanup, no replay, missing/invalid policy refusal');
})().catch(e=>{console.error(e);process.exitCode=1});
