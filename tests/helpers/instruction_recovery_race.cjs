'use strict';
const assert=require('node:assert/strict');
/** Hold a real POST before transport, refresh the same Work, then release it.
 * A callback wrapper exposes completion rather than using an arbitrary timeout.
 * The real backend response/readback is preserved. No business state is seeded. */
module.exports=async function verify(page,address,work){
 await page.goto(address+'/?'+new URLSearchParams({lang:'fr',work}),{waitUntil:'networkidle0'});
 await page.waitForFunction(()=>document.querySelector('#native-instruction-history button:not(:disabled)'));
 await page.evaluate(()=>{
  const widget=SwarmNativeInstructions;
  globalThis.SwarmNativeInstructions={...widget,render(container,work,callback,observed){return widget.render(container,work,async(...args)=>{try{return await callback(...args)}finally{window.rejectionCallbackFinished=true}},observed)}};
  window.raceOriginalFetch=window.fetch;
  window.raceRelease=new Promise(resolve=>{window.releaseHeldRejection=resolve});
  window.fetch=async(...args)=>{if(new URL(args[0],location.href).searchParams.get('action')==='reject-instructions'){window.rejectionHeld=true;await window.raceRelease}return window.raceOriginalFetch(...args)};
 });
 /** Wait for this refresh's actual observation, not an old idle DOM. */
 async function refresh(){
  const observed=page.waitForResponse(r=>{const u=new URL(r.url());return u.pathname==='/api/v1/cockpit-read'&&u.searchParams.get('work')===work});
  await page.click('#refresh');await observed;
  await page.waitForFunction(()=>!document.querySelector('#main-content').hasAttribute('aria-busy'));
 }
 // Host refresh binds the callback wrapper on freshly observed cards.
 await refresh();await page.waitForFunction(()=>document.querySelector('#native-instruction-history button:not(:disabled)'));
 await page.click('#native-instruction-history button');await page.waitForFunction(()=>window.rejectionHeld===true);
 await refresh();assert(await page.$('#native-instruction-history [data-state="applied"]'));
 assert(await page.$eval('#native-instruction-history button',e=>e.disabled),'concurrent refresh must preserve pending state');
 await page.evaluate(()=>window.releaseHeldRejection());await page.waitForFunction(()=>window.rejectionCallbackFinished===true);
 assert(await page.$eval('#native-instruction-history',e=>!!e.querySelector('[data-state="rejected"]')&&!e.querySelector('[role="alert"]')),'successful readback must never redraw the older same-Work revision');
 assert(await page.evaluate(async id=>(await(await window.raceOriginalFetch('/api/v1/planning?work='+id)).json()).tasks[0].instruction_revisions[0].state==='rejected',work),'real concurrent write must be observed');
};
