'use strict';
/** Typed metadata on actual HTTP; malformed/partial/empty/late variants below
 * are declared client fixtures after a real witness. No capture text is rendered
 * or exported. Wait for actual body consumption, never a local deadline. */
module.exports=async function transcriptFlow({page,cfg,label,check}){
 const open=()=>page.$eval('#native-agent-transcript',e=>e.open=true),read=()=>page.click('#native-transcript-read');
 const settled=()=>page.waitForFunction(()=>!document.querySelector('#native-agent-transcript').hasAttribute('aria-busy')&&!document.querySelector('#native-transcript-read').disabled);
 await open();check(label+'-transcript-explicit-read-required',await page.$eval('#native-transcript-text',e=>e.textContent===''));await read();await settled();
 const text=await page.$eval('#native-transcript-text',e=>e.textContent);
 check(label+'-transcript-actual-metadata-visible',text.split('\n').length===cfg.transcript.transcript.entries.length&&text.includes('input=3'));
 check(label+'-transcript-content-omitted',!text.includes('log witness')&&!text.includes('local model route fixture')&&!text.includes('<b>literal'));
 check(label+'-transcript-only-text',await page.$eval('#native-transcript-text',e=>!e.querySelector('*')));await page.focus('#native-transcript-text');check(label+'-transcript-keyboard',await page.$eval('#native-transcript-text',e=>e===document.activeElement&&!!e.getAttribute('aria-label')));
 for(const width of [1440,390]){await page.setViewport({width,height:1000});check(label+'-transcript-no-overflow-'+width,await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));if(width===390)await page.screenshot({path:require('node:path').join(cfg.output,label+'-transcript-390.png'),fullPage:true})}
 if(label==='fr-etat'){
  /** Replace only the transcript response; all history/Work reads stay real. */
  async function variant(transform){await page.evaluate((spec)=>{if(!window.transcriptFetch)window.transcriptFetch=window.fetch;window.fetch=async(...args)=>{const r=await window.transcriptFetch(...args);if(new URL(args[0],location.href).pathname!=='/api/v1/agent-transcript')return r;const v=await r.json();if(spec==='partial')v.capture_truncated=true;if(spec==='empty'){v.transcript.entries=[];v.transcript.counts={};v.transcript.unknown={};}if(spec==='absent'){v.state='unavailable';v.reason='capture-not-retained';v.transcript=null;v.capture_truncated=null;v.capture_invalid_utf8=null;v.output_complete=null;}if(spec==='malformed')return new Response('{}',{status:200,headers:{'Content-Type':'application/json'}});return new Response(JSON.stringify(v),{status:200,headers:{'Content-Type':'application/json'}})}},transform);await read();await settled();}
  await variant('partial');check('transcript-explicit-partial-warning',await page.$eval('#native-transcript-status',e=>e.classList.contains('attention')&&e.textContent.includes('partielle')));
  await variant('empty');check('transcript-empty-not-error',await page.$eval('#native-transcript-text',e=>e.textContent==='')&&await page.$eval('#native-transcript-status',e=>!e.classList.contains('alert')&&e.textContent.includes('Aucun événement')));
  await variant('absent');check('transcript-absence-not-empty-capture',await page.$eval('#native-transcript-status',e=>e.textContent.includes('Aucun transcript retenu'))&&await page.$eval('#native-transcript-text',e=>e.textContent===''));
  await variant('malformed');check('transcript-malformed-not-success',await page.$eval('#native-transcript-status',e=>e.classList.contains('alert')&&e.textContent.includes('agent-transcript-view-invalid'))&&await page.$eval('#native-transcript-text',e=>e.textContent===''));
  await page.evaluate(()=>window.fetch=window.transcriptFetch);await read();await settled();check('transcript-explicit-recovery',await page.$eval('#native-transcript-text',e=>e.textContent.includes('input=3')));
  await page.evaluate(()=>{window.fetch=(...args)=>new URL(args[0],location.href).pathname==='/api/v1/agent-transcript'?new Promise(resolve=>{window.releaseTranscript=()=>window.transcriptFetch(...args).then(r=>{const json=r.json.bind(r);r.json=async()=>{const v=await json();window.transcriptBodyRead=true;return v};resolve(r)})}):window.transcriptFetch(...args)});
  await read();await page.waitForFunction(()=>typeof window.releaseTranscript==='function');check('transcript-pending-clears-old-text',await page.$eval('#native-transcript-text',e=>e.textContent===''));
  await page.select('#log-agent','');await page.evaluate(()=>window.releaseTranscript());await page.waitForFunction(()=>window.transcriptBodyRead===true);check('transcript-late-old-source-ignored',await page.$eval('#native-transcript-text',e=>e.textContent==='')&&await page.$eval('#native-transcript-read',e=>e.disabled));
  await page.evaluate(()=>{window.fetch=window.transcriptFetch;delete window.transcriptFetch;delete window.releaseTranscript;delete window.transcriptBodyRead});await page.select('#log-agent','session');await page.waitForFunction(()=>!document.querySelector('#logs').hasAttribute('aria-busy')&&!document.querySelector('#log-prev').disabled);
 }
};
