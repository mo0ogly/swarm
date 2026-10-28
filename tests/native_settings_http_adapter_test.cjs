'use strict';
/** Controller/adapter unit guards with a minimal host and controlled fetch.
 * No real HTTP/Root/auth/browser claim. Exercise the actual transport passed to
 * the component; do not duplicate its implementation or expose a fixture API.
 */
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
(async()=>{
 let transport,quotas,resolveMounted,mode='session',active=0,maxActive=0,release;const mounted=new Promise(resolve=>{resolveMounted=resolve}),gate=new Promise(resolve=>{release=resolve});
 const element=()=>({hidden:false,className:'',textContent:'',addEventListener(){}}),nodes=new Map();
 const privateCause=new TypeError('private fixture transport context');
 const dto={failure:{code:'fixture-state-preserved'},configuration_may_be_recorded:true,recorded_configuration:{event_id:'receipt-event',event_revision:'2',current_revision:'2',replayed:false}};
 const reply=(status,body,type='application/json')=>({status,ok:status>=200&&status<300,headers:{get:()=>type},text:async()=>body});
 const context={URL,console,document:{readyState:'complete',getElementById:id=>{if(!nodes.has(id))nodes.set(id,element());return nodes.get(id)},querySelectorAll:()=>[]},addEventListener(){},SwarmEngineSettingsAdmin:{mount(host,value){transport=value;resolveMounted();return{}}},SwarmNativeQuotas:{mount(host,value){quotas=value;return{}}},fetch:async(url,options)=>{
  if(url==='/api/v1/native-session')return reply(200,JSON.stringify({schema_version:1,csrf_format:'hmac-sha256-v1',csrf_token:'a'.repeat(64)}));
  assert.equal(options.credentials,'same-origin');if(options.method==='POST')assert.equal(options.headers['X-Swarm-CSRF'],'a'.repeat(64));
  if(mode==='network')throw privateCause;if(mode==='invalid-json')return reply(200,'invalid raw fixture');if(mode==='refusal')return reply(403,'web-access-refused','text/plain');if(mode==='recorded')return reply(500,JSON.stringify(dto));
  active++;maxActive=Math.max(active,maxActive);await gate;active--;return reply(200,'{}');
 }};
 vm.runInNewContext(fs.readFileSync('web/native-runtime.js','utf8'),context);await mounted;assert(quotas,'Native quota component must mount successfully');
 mode='recorded';await assert.rejects(transport.apply({event_id:'x'}),e=>e.code==='fixture-state-preserved'&&e.httpStatus===500&&e.response.recorded_configuration.event_id==='receipt-event'&&e.message.includes('peuvent déjà être enregistrés'));console.log('PASS native HTTP adapter retains recorded state/status safely');
 mode='network';await assert.rejects(transport.show(),e=>e.cause===privateCause&&!e.message.includes('private fixture'));console.log('PASS native HTTP adapter preserves original network cause without raw text');
 mode='invalid-json';await assert.rejects(transport.show(),e=>e.code==='web-native-response-invalid'&&e.cause.name==='SyntaxError'&&e.response.body==='invalid raw fixture'&&!e.message.includes('invalid raw fixture'));console.log('PASS native HTTP adapter preserves parse cause without empty fallback');
 mode='refusal';await assert.rejects(transport.show(),e=>e.code==='web-access-refused'&&e.httpStatus===403&&e.response.body==='web-access-refused');console.log('PASS native HTTP adapter retains non-JSON refusal');
 mode='queue';const a=transport.describe(),b=transport.show();release();await Promise.all([a,b]);assert.equal(maxActive,1);console.log('PASS native HTTP adapter serializes after errors without swallowing caller failures');
 console.log('PASS native HTTP adapter 5 cases');
})().catch(error=>{console.error(error);process.exitCode=1});
