'use strict';
// Every page receives this policy from the authenticated server, before its
// application scripts. No client timeout fallback or policy ceiling exists.
globalThis.SwarmWebRequests={
 config:globalThis.SwarmWebRequestConfig,
 adopt(config){
  if(!config||!Number.isSafeInteger(config.revision)||config.revision<0)throw Error('Invalid web request configuration');
  for(const key of ['general_ms','snapshot_ms','connection_test_ms'])if(!Number.isSafeInteger(config.values?.[key])||config.values[key]<0)throw Error('Invalid web request duration');
  this.config={revision:config.revision,values:{...config.values}};
 },
 duration(path){
  this.adopt(this.config);
  const route=new URL(path,location.origin).pathname;
  return this.config.values[route==='/api/v1/providers/connections/test'?'connection_test_ms':route==='/api/v1/snapshot'?'snapshot_ms':'general_ms'];
 },
 armDeadline(duration,expire){
  let timer,remaining=duration,start=performance.now();
  const tick=()=>{const now=performance.now();remaining-=now-start;start=now;
   if(remaining<=0)expire(new DOMException('Browser request waiting time expired','TimeoutError'));
   else timer=setTimeout(tick,Math.min(remaining,0x7fffffff));
  };
  // Timer chunks are a browser representation bound, not an operator cutoff.
  if(duration>0)timer=setTimeout(tick,Math.min(remaining,0x7fffffff));
  return ()=>clearTimeout(timer);
 },
 async waitFor(operation,path){
  const duration=this.duration(path);let stop;
  const timeout=new Promise((_,reject)=>{stop=this.armDeadline(duration,reject)});
  try{return await Promise.race([operation,timeout])}finally{stop()}
 },
 async withResponse(path,options,read){
  const duration=this.duration(path),controller=new AbortController(),external=options?.signal;
  const cancel=()=>controller.abort(external.reason);
  if(external?.aborted)cancel();else external?.addEventListener('abort',cancel,{once:true});
  const stop=this.armDeadline(duration,error=>controller.abort(error));
  try{return await read(await fetch(path,{...options,signal:controller.signal}))}
  finally{stop();external?.removeEventListener('abort',cancel)}
 },
 text(path,options={}){return this.withResponse(path,options,async response=>({response,value:await response.text()}))},
 json(path,options={}){return this.withResponse(path,options,async response=>{
  try{return {response,value:await response.json()}}
  catch(error){error.status=response.status;throw error}
 })}
};
