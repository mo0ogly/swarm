'use strict';
/** Attempt every owned cleanup once, then finalize evidence, retaining all failures.
 * primary is absent on success or {error: thrownValue}, even for thrown undefined.
 * cleanups contains caller-owned async functions; no resources are selected,
 * stopped, retried or timed out here. finalize receives a frozen failure snapshot
 * so failed fixtures can be retained. Success returns undefined; any primary,
 * cleanup, invalid callback or finalize failure throws AggregateError containing
 * original values in order. Callers must handle it; messages are not classifications.
 */
async function finishOwnedResources(primary,cleanups,finalize){
 const failures=[];let original;
 if(primary!==undefined){
  if(primary!==null&&typeof primary==='object'&&Object.hasOwn(primary,'error')){original=primary.error;failures.push(original)}
  else failures.push(primary,new TypeError('Primary failure wrapper required'));
 }
 if(!Array.isArray(cleanups))failures.push(new TypeError('Owned cleanup function list required'));
 else for(const cleanup of cleanups){
  if(typeof cleanup!=='function'){failures.push(new TypeError('Owned cleanup callback required'));continue}
  try{await cleanup()}catch(error){failures.push(error)}
 }
 if(finalize!==undefined){
  if(typeof finalize!=='function')failures.push(new TypeError('Evidence finalizer callback required'));
  else try{await finalize(Object.freeze([...failures]))}catch(error){failures.push(error)}
 }
 if(failures.length)throw new AggregateError(failures,'Verification failed; original and cleanup causes retained',{cause:original});
}
module.exports={finishOwnedResources};
