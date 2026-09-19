'use strict';
const fs=require('fs'),vm=require('vm'),assert=require('assert/strict'),path=require('path');
(async()=>{
 const cockpit=fs.readFileSync(path.join(__dirname,'../web/cockpit.js'),'utf8');
 const api=vm.runInNewContext(cockpit.slice(cockpit.indexOf('async function api('),cockpit.indexOf('async function act('))+'\napi',{AbortSignal,fetch:async()=>({ok:false,status:409,text:async()=>JSON.stringify({error:'refus confirmé',failure:{code:'agent_command_refused'}})})});
 await assert.rejects(()=>api('/fixture'),e=>e.code==='agent_command_refused'&&e.status===409);
 for(const code of [undefined,'agent_command_refused','agent_command_pending']){
  let index=0;const requests=[];
  const ctx={work:'work',snapshot:{work:{revision:7}},modalContext:{revision:7},crypto:require('crypto'),refresh:async()=>{},notice:()=>{},api:async(url,body)=>{requests.push(body);if(index++===0){const e=new Error('fixture');e.code=code;throw e}return {message:'ok'}}};
  const actions=vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../web/pilot-actions.js'),'utf8')+'\nPilotActions',ctx);
  const fields={task:'task',agent:'agent'};await assert.rejects(()=>actions.send('stop',fields));await actions.send('stop',fields);
  assert.equal(requests[0].task,'task');assert.equal(requests[0].agent,'agent');
  assert.equal(requests[0].event_id===requests[1].event_id,code!=='agent_command_refused');
 }
 console.log('PASS : code erreur HTTP, reçu conservé après perte réseau/incertitude, nouvelle demande après refus confirmé');
})().catch(e=>{console.error(e);process.exitCode=1});
