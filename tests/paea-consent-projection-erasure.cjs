'use strict';
// Maintained counterexample for the STALE PROJECTION contract of the public
// PAEA consent page (web/paea-consent.js). The served page is the REAL
// workspace frontend in real Chrome; the prerequisite endpoints are EXPLICIT
// synthetic fixtures on a local HTTP server (same counterexample method as
// tests/paea-reader-credential-erasure.cjs: isolated synthetic API fixtures
// against the actual frontend — they prove the page's projection contract,
// never identity, never a real enrollment, never a product approval). The
// native-session CSRF prerequisite is served like the real product (the
// supervisor's first probe omitted it and proved nothing — retained lesson).
//
// Defect reproduced 2026-11-01 (Codex supervisor counterexample,
// glm-consent-refusal-cache-20261101T035126Z): after one authorized labelled
// read rendered the exact digests, a re-read refused with HTTP 403 on BOTH
// guarded reads left BOTH old digests visible in the DOM while
// approval-status moved to the empty/refusal wording — a refusal text claiming
// no data is displayed sat over stale success projections. Root cause:
// guarded reads that THROW (HTTP error, network error, malformed JSON) skip
// presentation()/preview() entirely, so their replaceChildren() purge never
// runs.
//
// Contract proven here, on the real page: after any successful labelled read,
// every subsequent load exit must leave NO value from the EARLIER success in
// the containers the new exit did not legitimately re-render. A container the
// new observation itself rendered carries its CURRENT exact values (that is
// presentation, not staleness); a container the new exit skipped or refused
// must be empty of the previous observation's values. Covered exits: both
// guarded reads refused HTTP 403 (the supervisor scenario), each read refused
// alone (403), each read verdict 200 available:false, source unbound,
// HTTP 503 on status/presentation/preview, malformed JSON on
// presentation/preview, and recovery successes after refusals. The consent
// form stays reachable through refusal exits; the reader credential erasure
// is re-witnessed on the refusal path (sibling test owns the full erasure
// contract). No new write/acceptance policy; no backend change; no acceptance.
const assert=require('node:assert/strict'),http=require('node:http'),fs=require('node:fs'),path=require('node:path');
const pup=require(path.join(process.cwd(),'node_modules','puppeteer'));
const out=path.resolve(process.argv[2]);
fs.mkdirSync(out,{recursive:true});
const root=process.cwd();
const marker='d'.repeat(64),previewMarker='e'.repeat(64);
const checks=[],errors=[];
let mode='normal',browser,server;
(async()=>{
 server=http.createServer((q,s)=>{
  const url=new URL(q.url,'http://localhost');
  s.setHeader('Cache-Control','no-store');
  const j=(v,status=200)=>{s.writeHead(status,{'Content-Type':'application/json'});s.end(typeof v==='string'?v:JSON.stringify(v))};
  if(url.pathname==='/api/v1/native-session')return j({schema_version:1,csrf_token:'a'.repeat(64)});
  if(url.pathname==='/api/v1/paea/git-call/approval')return mode==='status-error'?j({error:'fixture-status-error'},503):j({bound:mode!=='unbound',capability:null});
  if(url.pathname==='/api/v1/paea/git-call/approval/presentation'){
   if(mode==='denied'||mode==='presentation-403')return j({error:'authorization_required'},403);
   if(mode==='presentation-503')return j({error:'fixture-presentation-error'},503);
   if(mode==='presentation-garbage')return j('not-json',200);
   if(mode==='presentation-unavailable')return j({available:false,reason:'paea-reader-authentication-required'});
   return j({available:true,request_digest:marker,operation:'approve'});
  }
  if(url.pathname==='/api/v1/paea/git-call/preview'){
   if(mode==='denied'||mode==='preview-403')return j({error:'authorization_required'},403);
   if(mode==='preview-503')return j({error:'fixture-preview-error'},503);
   if(mode==='preview-garbage')return j('not-json',200);
   if(mode==='preview-unavailable')return j({available:false,reason:'paea-reader-authentication-required'});
   return j({available:true,canonical:{action_digest:previewMarker,intent_id:'labelled-intent',digest_matches:true},current:{roots_bound:true,trust_revision:'1',trust_revision_current:true},proposal:{action:{kind:'git',repository_id:'labelled-repository'}},limits:{consent:'none'}});
  }
  if(url.pathname.startsWith('/api/'))return j({},404);
  const f=path.join(root,'web',url.pathname.slice(1));
  if(fs.existsSync(f)&&fs.statSync(f).isFile()){s.setHeader('Content-Type',f.endsWith('.js')?'text/javascript':f.endsWith('.html')?'text/html':'text/css');s.end(fs.readFileSync(f))}else{s.writeHead(404);s.end()}
 });
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 browser=await pup.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
 const page=await browser.newPage();
 page.setDefaultTimeout(20000);
 page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{
  if(m.type()!=='error')return;
  // Chrome mirrors every denied fetch as a generic 'Failed to load resource'
  // console error — including the expected refusal fixtures below. Count
  // only that exact echo; any other console error stays a diagnostic.
  if(/^Failed to load resource.*status of \d{3}/.test(m.text()))return;
  errors.push(m.text());
 });
 await page.goto('http://127.0.0.1:'+server.address().port+'/paea-consent.html?lang=en',{waitUntil:'networkidle0'});
 await page.waitForFunction(()=>document.querySelector('#approval-status').dataset.state!=='loading');
 const token='labelled-reader-fixture';
 const typeToken=v=>page.$eval('#approval-reader-credential',(e,value)=>{e.value=value},v);
 const projections=()=>page.evaluate(()=>({
  capability:document.querySelector('#approval-capability').textContent,
  preview:document.querySelector('#approval-preview').textContent,
  result:document.querySelector('#result-detail').textContent,
  detailHidden:document.querySelector('#approval-detail').hidden,
  status:document.querySelector('#approval-status').dataset.state,
  statusText:document.querySelector('#approval-status').textContent,
  detail:document.querySelector('#preview-status').dataset.state,
  detailText:document.querySelector('#preview-status').textContent,
  reader:document.querySelector('#approval-reader-credential').value,
 }));
 async function observe(variant){
  mode=variant;
  // The witness path: retype the credential, dispatch the page's own re-read
  // control's DOM click (JS-state evidence only — see the scope note below),
  // then sync on the load() exit — the reader input is cleared in
  // the load's `finally`, so an empty input proves the terminal state is the
  // post-load one (no mid-render race, observed on the first red run).
  await typeToken(token);
  // Click through the page's real event path: the sticky header overlays the
  // re-read control at some scroll offsets (measured 2026-11-01: Puppeteer's
  // synthetic center click landed on HEADER and the load never started), so
  // the control is scrolled into view first and the click is dispatched on
  // the element itself. Scope of proof: this DOM-level click() verifies the
  // EXACT JS state transitions (purge, refusal pair, recovery) — it does NOT
  // qualify physical pointer hit-testing of the overlaid header layout; that
  // was separately verified with a real trusted page.click
  // (supervisor-personal/actual-click-execution.json, 2026-11-01). A future
  // real-browser fixture must verify the real interaction, never bypass the
  // layout defect if it reappears.
  await page.evaluate(()=>{document.querySelector('#approval-reload').scrollIntoView({block:'center'})});
  await page.evaluate(()=>document.querySelector('#approval-reload').click());
  try{
   await page.waitForFunction(()=>document.querySelector('#approval-reader-credential').value==='');
  }catch(error){
   // Diagnostic dump on a stuck wait: the failure file must carry the real
   // page state (status, hidden flags, input) instead of a bare timeout.
   const dump=await projections();
   fs.writeFileSync(path.join(out,'observe-stuck-dump.json'),JSON.stringify({variant,error:error.message,...dump},null,2));
   throw error;
  }
  return projections();
 }
 // No value from the EARLIER success may survive in a container the new exit
 // did not re-render: `current` names the containers whose values the new
 // observation itself legitimately rendered (presentation, not staleness).
 const noStale=(obs,variant,current={capability:false,preview:false})=>{
  if(!current.capability)assert.ok(!obs.capability.includes(marker),variant+': stale request digest must be purged from #approval-capability');
  if(!current.preview)assert.ok(!obs.preview.includes(previewMarker),variant+': stale action digest must be purged from #approval-preview');
  assert.ok(!obs.result.includes('labelled-intent'),variant+': stale business intent must never sit in #result-detail');
 };
 // Baseline: one authorized labelled read renders BOTH exact digests.
 const success=await observe('normal');
 assert.equal(success.status,'success','baseline labelled read must reach the explicit success state');
 assert.ok(success.capability.includes(marker),'baseline renders the exact request digest');
 assert.ok(success.preview.includes(previewMarker),'baseline renders the exact action digest');
 checks.push('baseline success: exact request digest + action digest rendered');
 // 1. BOTH guarded reads refused HTTP 403 — the supervisor defect scenario:
 // the refusal wording (final state empty, typed code visible) must sit over
 // fully purged projections, never over the earlier success's digests.
 const denied=await observe('denied');
 assert.equal(denied.status,'empty','both-reads 403 stays the explicit empty refusal state');
 assert.ok(denied.statusText.includes('authorization_required'),'the typed refusal code stays visible on approval-status');
 noStale(denied,'denied');
 assert.equal(denied.reader,'','reader credential erased on the refusal exit');
 checks.push('both guarded reads HTTP 403: typed refusal visible, both digests purged, credential erased');
 // 2. Recovery: the same reader re-reads successfully again.
 const recovery=await observe('normal');
 assert.equal(recovery.status,'success','recovery read must reach success again');
 assert.ok(recovery.capability.includes(marker),'recovery renders the exact request digest again');
 checks.push('recovery: labelled read succeeds again, exact digests rendered anew');
 // 3. Presentation refused alone (HTTP 403): the capability list must be
 // purged; the preview that THIS observation re-rendered carries current
 // values; success must NOT be claimed over the refused exact metadata.
 const presentationRefusal=await observe('presentation-403');
 assert.ok(['empty','error'].includes(presentationRefusal.status),'presentation 403 stays an explicit non-success terminal state');
 assert.ok(!presentationRefusal.statusText.includes('Commande figée liée et action préparée présentées'),'no success wording over a refused presentation');
 noStale(presentationRefusal,'presentation-403',{preview:true});
 checks.push('presentation HTTP 403 alone: capability purged, current preview kept, no success claim');
 // 4. Preview refused alone (HTTP 403): the preview list must be purged; the
 // presentation digest THIS observation rendered is current.
 const previewRefusal=await observe('preview-403');
 assert.ok(['empty','error'].includes(previewRefusal.status),'preview 403 stays an explicit terminal state');
 noStale(previewRefusal,'preview-403',{capability:true});
 checks.push('preview HTTP 403 alone: preview purged, current presentation digest kept');
 // 5. Verdict 200 available:false on presentation (the real BLP-guard refusal
 // shape): same purge contract on the capability list; the no-data wording
 // must be literally true.
 const unavailable=await observe('presentation-unavailable');
 assert.equal(unavailable.status,'empty','presentation available:false stays the explicit empty refusal state');
 noStale(unavailable,'presentation-unavailable',{preview:true});
 checks.push('presentation verdict 200 available:false: no-data wording over a purged capability list');
 // 6. Verdict 200 available:false on preview: same contract on the preview list.
 const previewUnavailable=await observe('preview-unavailable');
 assert.equal(previewUnavailable.status,'empty','preview available:false stays the explicit empty state');
 noStale(previewUnavailable,'preview-unavailable',{capability:true});
 checks.push('preview verdict 200 available:false: preview list purged');
 // 7. Source unbound: startup bound=false after a prior success — every
 // projection container the refused load did not render must be empty.
 const unbound=await observe('unbound');
 assert.equal(unbound.status,'empty','unbound stays the explicit empty state');
 noStale(unbound,'unbound');
 checks.push('source unbound: stale success projections purged');
 // 8. HTTP 503 on each guarded read: error states, still no stale values.
 const statusError=await observe('status-error');
 assert.equal(statusError.status,'error','status 503 stays a visible error');
 noStale(statusError,'status-error');
 const presentationError=await observe('presentation-503');
 assert.equal(presentationError.status,'error','presentation 503 stays a visible error');
 noStale(presentationError,'presentation-503',{preview:true});
 const previewError=await observe('preview-503');
 // Preview throw exit (HTTP 503 here): the product contract renders the typed
 // code on #preview-status ('error') and the paired terminal notice on
 // #approval-status ('empty', exact code carried). Both are asserted together
 // (supervisor-replan-20261101T0402Z: the pair IS the refusal state; a bare
 // status dataset is not the contract). No success wording over a refusal.
 assert.equal(previewError.detail,'error','preview 503: typed code visible on preview-status');
 assert.ok(previewError.statusText.includes('fixture-preview-error'),'preview 503: exact refusal code carried on approval-status');
 assert.equal(previewError.status,'empty','preview 503: approval-status paired terminal state (no success, no loading)');
 noStale(previewError,'preview-503',{capability:true});
 checks.push('HTTP 503 on status/presentation/preview: visible refusals (paired typed states), refused containers purged');
 // 9. Malformed JSON replies on the guarded reads: parse-error exits.
 const presentationGarbage=await observe('presentation-garbage');
 assert.equal(presentationGarbage.status,'error','garbage presentation reply stays a visible error');
 noStale(presentationGarbage,'presentation-garbage',{preview:true});
 const previewGarbage=await observe('preview-garbage');
 // Same preview throw exit as the 503: paired typed states (preview-status
 // 'error' with the caught code, approval-status 'empty' carrying it).
 assert.equal(previewGarbage.detail,'error','garbage preview: typed code visible on preview-status');
 assert.ok(previewGarbage.statusText.includes('Unreadable response'),'garbage preview: refusal wording carried on approval-status');
 assert.equal(previewGarbage.status,'empty','garbage preview: approval-status paired terminal state');
 noStale(previewGarbage,'preview-garbage',{capability:true});
 checks.push('malformed JSON on presentation/preview: visible errors, refused containers purged');
 // 10. Final recovery: the page keeps serving exact values on a valid read.
 const final=await observe('normal');
 assert.equal(final.status,'success','final recovery must reach success again');
 assert.ok(final.capability.includes(marker),'final recovery renders the exact request digest');
 checks.push('final recovery: exact projections rendered anew');
 // The consent form stayed reachable through every refusal exit (the reader
 // frontier never hides the form — enrollment authority is never clearance).
 assert.equal(await page.$eval('#consent-form',e=>!e.hidden),true,'consent form must stay reachable through refusal exits');
 fs.writeFileSync(path.join(out,'consent-projection-erasure.json'),JSON.stringify({schema_version:1,kind:'paea-consent-projection-erasure-v1',checks,unexpected_diagnostics:errors,at:new Date().toISOString()},null,2)+'\n');
 console.log(JSON.stringify({checks:checks.length,unexpected_diagnostics:errors.length}));
})().catch(e=>{
 fs.writeFileSync(path.join(out,'consent-projection-erasure-failure.json'),JSON.stringify({message:e.message,stack:e.stack,checks,errors},null,2),{mode:0o600});
 console.error('FAIL:',e.message);process.exitCode=1;
}).finally(async()=>{if(browser)await browser.close();if(server)await new Promise(r=>server.close(r))});
