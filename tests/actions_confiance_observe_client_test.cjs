'use strict';
const assert=require('node:assert'),vm=require('node:vm'),fs=require('node:fs');
const context=vm.createContext({trust:()=>({addEventListener(){}}),trustView:null,trustTr:x=>x});
vm.runInContext(fs.readFileSync('web/trust-observe.js','utf8'),context);
assert.equal(typeof context.validateTrustObserveResponse,'function','observer client must reject malformed responses');
const policy={enabled:true,window_seconds:5,timeout_seconds:15,ring_bytes:65536,max_events:128};
const value={schema_version:1,available:false,source:'ebpf',scope:'registered_main_thread',attribution:{agent_id:'agent',mission_id:'mission',task_id:'task',attempt_id:'attempt',pid:123,pid_start_time:'42',resource_digest:'a'.repeat(64)},effective_policy:policy,revision:1,events:[],attempts:'0',emitted:'0',kernel_lost:'0',omitted:'0',cleanup_verified:true,complete:false,global_qualification:false,stop_reason:'unavailable'};
assert.doesNotThrow(()=>context.validateTrustObserveResponse(value,policy,'agent'));
for(const key of Object.keys(value)){const bad={...value};delete bad[key];assert.throws(()=>context.validateTrustObserveResponse(bad,policy,'agent'),key)}
for(const change of [{available:true},{complete:true},{cleanup_verified:false},{omitted:'1'},{attribution:{...value.attribution,agent_id:'other'}},{effective_policy:{...policy,max_events:1}}])assert.throws(()=>context.validateTrustObserveResponse({...value,...change},policy,'agent'));
assert.doesNotThrow(()=>context.validateTrustObserveResponse({...value,available:true,stop_reason:'window'},policy,'agent'));
console.log(JSON.stringify({named:'observer-response-contract',required_fields:Object.keys(value).length,invalid_variants:6,quiet_window_partial:true}));
