'use strict';
/** Exercise the actual enrolled click handler with explicit DOM/API doubles.
 * This proves decision/error presentation, not authentication or browser layout. */
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const code=fs.readFileSync('web/trust-native.js','utf8');
const start=code.indexOf("$('trust-enrolled-execute').addEventListener('click'");
const end=code.indexOf("$('close').addEventListener",start);
assert(start>=0&&end>start,'actual handler located');
async function run(reply,refreshError,duplicate=false){
 const fields={'trust-enrolled-password':{value:'fixture-password'},'trust-enrolled-operation':{value:'mandate-create'},'trust-enrolled-request':{value:'exact fixture bytes'},'trust-enrolled-message':{textContent:''}};let handler,calls=0,during=false;
 fields['trust-enrolled-execute']={addEventListener:(name,fn)=>{assert.equal(name,'click');handler=fn;}};
 const labels=code.slice(code.indexOf('const statusLabels='),code.indexOf('\n',code.indexOf('const statusLabels=')));
 const availability=code.match(/function syncEnrolledAvailability\(\)\{[\s\S]*?\n\}/)[0];
 vm.runInNewContext(labels+'\n'+availability+'\n'+code.slice(start,end),{state:{identity_authentication:{held_enrollment:{http_available:true,scope:'single_prepared_intent'}}},busy:false,$:id=>fields[id],t:s=>s,api:async()=>{calls++;during=fields['trust-enrolled-execute'].disabled===true;return {result:reply};},load:async()=>{if(refreshError)throw Error('fixture-refresh-failed');}});
 if(duplicate){await Promise.all([handler(),handler()]);assert.equal(calls,1,'concurrent gesture sends once');}else await handler();assert.equal(fields['trust-enrolled-password'].value,'');assert(during,'enrolled button disabled during its own request');return fields['trust-enrolled-message'].textContent;
}
(async()=>{
 assert.match(await run({state:'denied'}),/^Intention refusée/,'denied200 is visibly refused');
 assert.match(await run({state:'admitted'}),/Admise/,'admitted label translated');
 assert.match(await run({state:'unknown'}),/^Résultat incertain/,'unknown outcome never claims execution');
 assert.match(await run({state:'admitted'},true),/Intention admise.*fixture-refresh-failed/,'known decision survives failed readback');
 await run({state:'admitted'},false,true);
 console.log('PASS unknown-visible; concurrent-gesture-once; denied200-visible; admitted-labelled; readback-failure-preserves-decision; password-cleared-all-cases');
})().catch(e=>{console.error(e);process.exitCode=1;});
