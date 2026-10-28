const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const parser=fs.readFileSync('web/trust-evidence.js','utf8').split('\n').find(x=>x.startsWith('function trustProofJSON('));
const api=fs.readFileSync('web/trust-actions.js','utf8').split('\n').find(x=>x.startsWith('async function trustAPI('));
const checks=[];
(async()=>{
 for(const integer of ['9007199254740993','9223372036854775807','-9223372036854775808']){
  const raw=`{"export":{"evidence":[{"effect_receipt":{"native_result":{"amount_minor":${integer}}}}]}}`;
  const context=vm.createContext({JSON,Error,Number,peText:x=>x,trustCSRF:'fixture',trustTr:x=>x,fetch:async()=>({ok:true,text:async()=>raw})});
  vm.runInContext(parser+'\n'+api,context);
  const value=await context.trustAPI('/api/v1/trust-evidence',undefined,'',JSON.stringify,undefined,context.trustProofJSON);
  assert.equal(JSON.stringify(value),raw);
  const pasted=context.trustProofJSON(JSON.stringify(value));assert.equal(JSON.stringify(pasted),raw);
  checks.push('API/export/pasted verification preserves '+integer);
 }
 const parse=vm.runInNewContext(parser+';trustProofJSON',{JSON,Number,Error,peText:x=>x});
 for(const number of ['1.00000000000000001','1e0','-0','9007199254740991.1']){
  const raw='{"sequence":'+number+'}';assert.equal(JSON.stringify(parse(raw)),raw);checks.push('numeric source preserved for server validation: '+number);
 }
 const noSource=vm.runInNewContext(parser+';trustProofJSON',{JSON:{parse:(raw,revive)=>JSON.parse(raw,(key,value)=>revive(key,value)),stringify:JSON.stringify,rawJSON:JSON.rawJSON},Number,Error,peText:x=>x});
 assert.throws(()=>noSource('{"sequence":1.00000000000000001}'),/CLI/);checks.push('missing numeric source refuses instead of normalizing');
 assert.equal(JSON.stringify(parse('{"text":"9007199254740993","normal":42}')),'{"text":"9007199254740993","normal":42}');checks.push('strings and ordinary values preserved');
 const unsupported=vm.runInNewContext(parser+';trustProofJSON',{JSON:{parse:JSON.parse,stringify:JSON.stringify},Number,Error,peText:x=>x});
 assert.throws(()=>unsupported('{"large":9007199254740993}'),/CLI/);checks.push('unsupported exact transport refuses explicitly');
 const context=vm.createContext({JSON:{parse:JSON.parse,stringify:JSON.stringify},Number,Error,peText:x=>x,trustCSRF:'fixture',trustTr:x=>x,fetch:async()=>({ok:true,text:async()=>'{"large":9007199254740993}'})});
 vm.runInContext(parser+'\n'+api,context);
 await assert.rejects(()=>context.trustAPI('/api/v1/trust-evidence',undefined,'',JSON.stringify,undefined,context.trustProofJSON),/CLI/);checks.push('API propagates unsupported exact transport refusal');
 console.log(JSON.stringify({checks,fixture_transport:true,browser_executed:false}));
})().catch(e=>{console.error(e);process.exitCode=1});
