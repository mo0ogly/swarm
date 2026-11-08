'use strict';
// Maintained counterexample for the FAIL-CLOSED SUCCESS contract of the two
// connected PAEA approval surfaces: web/paea-approval-form.js (explicit
// approval form) and web/paea-consent.js (consent page). The served pages are
// the REAL workspace frontend in real Chrome; every endpoint is an EXPLICIT
// synthetic fixture on a local HTTP server (same method as the maintained
// tests/paea-consent-projection-erasure.cjs: isolated synthetic API fixtures
// against the actual frontend — they prove the pages' presentation contract,
// never identity, never a real enrollment, never a product approval). The
// native-session CSRF prerequisite is served like the real product.
//
// Defect under correction (observed 2026-11-06):
// web/paea-approval-form.js send() returned a `state:"admitted"` reply as a
// success although an approve consumption must be exactly `approved`;
// web/paea-consent.js result() rendered the success wording for EVERY other
// state (`admitted`, `unknown`, missing, null, empty).
//
// Contract proven here, on the real pages, in FR and EN: the ONLY success of
// the approval POST is an exact `state:"approved"` — the ordinary full
// approved object and the redacted `trust-status-ack-v1` acknowledgement
// (`content_redacted:true`) both carry that state and both are accepted, so
// no redacted-only shape is imposed. `denied` stays a visible refusal with
// its exact reason. `admitted`, `unknown`, `pending_approval`, `committed`
// and any missing, null or empty state render a non-success terminal state
// and NEVER a success. On every exit the enrollment password is erased; the
// POST keeps the EMPTY body, the Enrollment Authorization header and the CSRF
// header; each explicit gesture produces exactly ONE POST with no automatic
// retry.
//
// Bilingual claim, both surfaces: every human-wording assertion runs in FR
// and in EN. The EN surface must show the catalog TRANSLATION and must NOT
// show the FR source wording (independent FR-leak check). Only existing
// catalog keys are used — none invented. The unexpected-state key
// `Réponse inattendue : {state} — vérifier l’état avant reprise` has the
// supplied catalog EN value `Unexpected reply: {state} — check the state
// before resuming` (asserted directly); the approved/denied wordings are
// read live from each page's own SwarmI18n as the oracle.
//
// Outcome waits are causal, never expectation-shaped: each consent wait
// resolves on THIS reply's verbatim nonce echo plus the page's own release of
// the submit control plus ANY terminal status; each approval-form wait
// resolves on the guarded exit's password re-enable plus ANY terminal status.
// The EXPECTED state is asserted only OUTSIDE the waits, so the faulty pages
// record their failure (exit 1, failure artifact) instead of stalling a
// check. No deadline: a genuinely stalled predicate hangs by instruction; a
// surface missing from the served web root fails fast at its 'open ...' step.
//
// Diagnostics contract: page errors, console errors, unexpected HTTP
// refusals (every non-2xx response is classified; failed requests are
// recorded — nothing is globally ignored), fixture-handler, clientError,
// server and close errors all fail the run with the evidence retained; the
// zero-diagnostics gate runs AFTER cleanup and BEFORE the PASS artifact and
// exit 0, so a real page error can no longer exit 0. No console-error
// suppression exists. Known expected refusals are enumerated EXACTLY
// (method + endpoint + status) and the enumeration is empty by construction:
// every fixture endpoint answers 2xx (the main approval POST is 200 in ALL
// cases — no expected denial status) and the browser's favicon request is
// met by an explicit synthetic 204. Cleanup errors never replace the first
// failure and never disappear: they join the same failure artifact.
//
// Exit-path contract (corrected): the fixture-server listen promise REJECTS
// on a real bind/listen error — the error object is preserved as the cause
// of the fatal failure and is still recorded among the collected
// diagnostics — instead of settling only on 'listening', which could leave
// the setup pending forever on a real bind error. ONE shared, idempotent,
// never-throwing cleanup helper closes the owned Puppeteer browser and the
// owned fixture server on EVERY exit path — normal completion, recorded
// failure, and fatal setup/browser/page error alike — and every close
// error (including the server.close callback error) joins the retained
// evidence without replacing the first failure. A failure-artifact write
// error is never swallowed: its real cause is logged to stderr beside the
// primary cause; if the artifact ultimately cannot be written, the
// collected diagnostics are logged to stderr as well (never secrets — no
// password, no header value) and the run still exits 1 — no silent success
// and no empty artifact. No watchdog, no retry, no force-kill, no
// unrelated process: causal closes of the owned resources only.
//
// Baseline procedure (integrator-run; the author executes nothing): copy the
// CURRENT exact web assets into a private owned directory and run this file
// with --web-root pointing there against a FRESH EMPTY output directory — it
// must FAIL (exit 1, approval-state-fail-closed-failure.json) first at the
// step 'approval fr admitted' (the admitted reply rendered as a success). A
// staged copy in which only web/paea-consent.js is the faulty original
// (one-surface-only baseline, exact files in the copied web dir) must
// instead fail at 'consent fr admitted'. After the product edits, rerun
// against the canonical web directory: it must PASS with 36 named checks,
// zero unexpected diagnostics and exit 0. The test performs NO checkout, NO
// source mutation, NO package install and NO network use beyond its local
// 127.0.0.1 fixture server; unrelated dirty work in the repository is
// untouched. --web-root is the STATIC ASSET ROOT ONLY (default: <repo>/web,
// validated to exist); the repository root stays the root for Puppeteer's
// node_modules resolution. A non-empty output directory is refused:
// historical proofs are never overwritten.
//
// No arbitrary watchdog: browser and protocol timeouts are disabled (0) and
// every wait is a causal predicate on the page's own DOM state. The
// synthetic replies are NOT actual HTTP acceptance of anything: they
// exercise the browser-side state interpretation only. No backend change, no
// write/acceptance policy, no installation, no global acceptance.
const assert=require('node:assert/strict'),http=require('node:http'),fs=require('node:fs'),path=require('node:path');
const pup=require(path.join(process.cwd(),'node_modules','puppeteer'));
const enrollmentPassword='fixture-enrollment-password';
const EXPECTED_CHECKS=36;
const checks=[],errors=[],postLog=[];
let step='init',postMode='approved-full',expectedPosts=0,browser,server;
let firstFailure=null,failureWritten=-1,outDir=null;
// Owned-cleanup contract: the fixture HTTP server and the Puppeteer browser
// are owned by this test and are closed by THIS ONE helper on EVERY exit
// path — the normal path below and the fatal catch at the end both await
// it. Idempotent: each owned resource is captured and detached first, so a
// second call is a safe no-op. It never throws: each real close error
// (including the server.close callback error) is appended to `errors`, so
// it joins the retained evidence without ever replacing the first failure.
// A server that never reached 'listening' has nothing to close. No
// watchdog, no retry, no force-kill, no unrelated process: causal closes
// of the owned resources only.
const closeOwned=async()=>{
 const ownedBrowser=browser;browser=null;
 if(ownedBrowser){try{await ownedBrowser.close()}catch(error){errors.push('browser.close: '+String(error&&error.message||error))}}
 const ownedServer=server;server=null;
 if(ownedServer){try{
  if(ownedServer.listening){
   if(ownedServer.closeIdleConnections)ownedServer.closeIdleConnections();
   await new Promise((resolve,reject)=>ownedServer.close(error=>error?reject(error):resolve()));
  }
 }catch(error){errors.push('server.close: '+String(error&&error.message||error))}}
};
const POST_REPLIES={
 'approved-full':()=>({state:'approved',reason:null,next_action:'none',business_mutated:false}),
 'approved-redacted':()=>({schema_version:1,kind:'trust-status-ack-v1',state:'approved',content_redacted:true}),
 'denied':()=>({state:'denied',reason:'fixture-separation-of-duties',next_action:'relire-etat'}),
 'admitted':()=>({state:'admitted'}),
 'unknown':()=>({schema_version:1,kind:'trust-status-ack-v1',state:'unknown',content_redacted:true}),
 'pending':()=>({state:'pending_approval'}),
 'committed':()=>({state:'committed'}),
 'missing':()=>({schema_version:1,kind:'trust-status-ack-v1',content_redacted:true}),
 'null-state':()=>({state:null}),
 'empty-state':()=>({state:''}),
};
// Existing catalog keys ONLY, verbatim from the two surfaces — no invented
// locale key. The approved/denied EN wordings are read live from each page's
// own SwarmI18n in the current language (the oracle); the unexpected-state
// wording is asserted through its supplied catalog pair (FR source key and
// EN value) with an independent FR-leak check on the EN surface.
const KEYS={
 approvalApproved:'Approbation enregistrée par le moteur : décision reçue ({state}). Aucun effet Git n’a été exécuté par ce formulaire ; relire les états d’effets pour la suite.',
 approvalDenied:'Intention refusée — décision journalisée',
 consentApproved:'Accusé du canal d’approbation : état rapporté ci-dessous, attesté par le moteur sur le Root projet. Le contenu de l’action reste rédigé (content_redacted) ; l’accord exact est ensuite observable côté moteur.',
 consentDenied:'Refus enregistré par le moteur : la commande n’a pas produit d’accord. La raison et la suite indiquée s’affichent telles quelles ; relire l’état avant toute reprise.',
};
// Known expected refusals — enumerated EXACTLY (method + endpoint + status).
// Empty by construction: every served endpoint answers 2xx (the main
// approval POST is 200 in ALL cases — no expected denial status) and the
// favicon is an explicit synthetic 204, so NO non-2xx is pre-ignored; any
// non-2xx response becomes a recorded unexpected diagnostic.
const EXPECTED_REFUSALS=[];
const isExpectedRefusal=(method,url,status)=>EXPECTED_REFUSALS.some(e=>e.method===method&&e.url===url&&e.status===status);
const USAGE='usage: node paea-approval-state-fail-closed.cjs <fresh-empty-output-dir> [--web-root <existing-static-asset-dir>]';
function parseArgs(argv){
 if(!argv[0])throw new Error(USAGE+' — a fresh empty output directory is required (historical proofs are never overwritten)');
 const outPath=path.resolve(argv[0]);
 if(fs.existsSync(outPath)){
  const st=fs.statSync(outPath);
  if(!st.isDirectory())throw new Error('output path is not a directory: '+outPath);
  const entries=fs.readdirSync(outPath);
  if(entries.length>0)throw new Error('output directory is not empty — refusing to overwrite historical proofs: '+outPath+' ('+entries.length+' entries)');
 }
 let webRoot=null;
 for(let i=1;i<argv.length;i++){
  let value=null;
  if(argv[i]==='--web-root'){value=argv[i+1];i++;}
  else if(argv[i].startsWith('--web-root='))value=argv[i].slice('--web-root='.length);
  else throw new Error('unknown argument: '+argv[i]+' — '+USAGE);
  if(value===undefined||String(value).trim()==='')throw new Error('--web-root requires an explicit existing directory (static asset root only)');
  webRoot=value;
 }
 webRoot=path.resolve(webRoot===null?path.join(process.cwd(),'web'):webRoot);
 if(!fs.existsSync(webRoot)||!fs.statSync(webRoot).isDirectory())throw new Error('--web-root must be an existing directory (static asset root only): '+webRoot);
 return {out:outPath,webRoot};
}
const config=(()=>{try{return parseArgs(process.argv.slice(2))}catch(error){console.error('FAIL: '+error.message);process.exitCode=1;return null}})();
if(config)(async()=>{
 const out=config.out;outDir=out;
 const webRoot=config.webRoot;
 fs.mkdirSync(out,{recursive:true});
 server=http.createServer(async(q,s)=>{
  const url=new URL(q.url,'http://localhost');
  const j=(v,status=200)=>{if(!s.headersSent)s.writeHead(status,{'Content-Type':'application/json'});s.end(typeof v==='string'?v:JSON.stringify(v))};
  try{
   s.setHeader('Cache-Control','no-store');
   if(url.pathname==='/api/v1/paea/git-call/approval'&&q.method==='POST'){
    let bodyLength=0;for await(const chunk of q)bodyLength+=chunk.length;
    const index=postLog.length;
    postLog.push({bodyLength,hasCsrf:Object.prototype.hasOwnProperty.call(q.headers,'x-swarm-csrf'),auth:String(q.headers.authorization||'')});
    const build=POST_REPLIES[postMode];
    if(!build)return j({error:'fixture-unknown-post-mode'},500);
    const reply=build();
    // Unique per-submit object id: the verbatim raw reply becomes the causal
    // "this submit's outcome" marker for the consent waits (staleness-free).
    reply.object_id='fixture-approval-object-'+index;
    return j(reply);
   }
   if(url.pathname==='/api/v1/native-session')return j({schema_version:1,csrf_token:'a'.repeat(64)});
   if(url.pathname==='/api/v1/paea/git-call/approval')return j({bound:true,capability:null});
   if(url.pathname==='/api/v1/paea/git-call/approval/presentation')return j({available:true,request_digest:'c'.repeat(64),operation:'approve',agreement_terms:{resource:'fixture-resource',amount_minor:'1',currency:'EUR',beneficiary:'fixture-beneficiary',revision:'1'},current:{roots_bound:true,trust_revision:'1',trust_revision_current:true},canonical:{digest_matches:true}});
   if(url.pathname==='/api/v1/paea/git-call/preview')return j({available:false,reason:'fixture-out-of-scope'});
   if(url.pathname==='/api/v1/paea/git-call')return j({bound:false,capability:null});
   if(url.pathname==='/api/v1/paea/reconcile-effects')return j({schema_version:1,settlement:{state:'uninitialized'},git:{state:'uninitialized'}});
   if(url.pathname.startsWith('/api/'))return j({error:'fixture-not-served'},404);
   // Explicit synthetic optional sibling: the browser requests the favicon on
   // every navigation. Serving it here keeps the response classification
   // exact — no non-2xx is expected anywhere in a correct run.
   if(url.pathname==='/favicon.ico'){s.writeHead(204);return s.end()}
   // Static assets from the validated web root only: the path is decoded
   // first, then containment is enforced on the RESOLVED path, so neither an
   // encoded nor a literal traversal can escape the served root.
   let rel=null;try{rel=decodeURIComponent(url.pathname.slice(1))}catch{rel=null}
   const f=rel===null?null:path.resolve(webRoot,rel);
   if(f!==null&&!rel.includes('\0')&&(f===webRoot||f.startsWith(webRoot+path.sep))&&fs.existsSync(f)&&fs.statSync(f).isFile()){
    s.setHeader('Content-Type',f.endsWith('.js')||f.endsWith('.mjs')?'text/javascript':f.endsWith('.html')?'text/html':f.endsWith('.css')?'text/css':f.endsWith('.json')?'application/json':f.endsWith('.svg')?'image/svg+xml':f.endsWith('.png')?'image/png':f.endsWith('.ico')?'image/x-icon':'application/octet-stream');
    return s.end(fs.readFileSync(f));
   }
   return j('fixture-not-found',404);
  }catch(error){errors.push('fixture handler ['+url.pathname+']: '+error.message);return j({error:'fixture-internal',detail:String(error&&error.message||error)},500)}
 });
 server.on('clientError',(error,socket)=>{errors.push('clientError: '+error.message);if(socket.writable)socket.end('HTTP/1.1 400 Bad Request\r\n\r\n')});
 server.on('error',error=>errors.push('server: '+error.message));
 step='fixture server';
 // Bind contract: settle on 'listening' OR reject on the first real
 // listen/bind error — a resolve-only listen could leave the setup pending
 // forever on a real bind error. The error object is preserved as the
 // cause of the fatal failure, and the persistent 'error' collector above
 // still records it among the diagnostics; after a successful bind the
 // rejector is detached so later server errors remain collected-only,
 // exactly as before.
 await new Promise((resolve,reject)=>{
  const onListening=()=>{server.off('error',onBindError);resolve()};
  const onBindError=error=>{server.off('listening',onListening);reject(error)};
  server.once('listening',onListening);
  server.once('error',onBindError);
  server.listen(0,'127.0.0.1');
 });
 step='browser launch';
 browser=await pup.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage'],protocolTimeout:0});
 step='page setup';
 const page=await browser.newPage();
 page.setDefaultTimeout(0);
 page.on('pageerror',e=>errors.push('pageerror: '+e.message));
 // No console-error suppression of any kind: a correct run produces none.
 page.on('console',m=>{if(m.type()==='error')errors.push('console: '+m.text())});
 // Response/requestfailed classification: every non-2xx response and every
 // failed request is recorded unless it matches the (empty) exact
 // expected-refusal enumeration above.
 page.on('response',r=>{const st=r.status();if(st>=400){const req=r.request();const method=req?req.method():'?';if(!isExpectedRefusal(method,r.url(),st))errors.push('http '+method+' '+r.url()+' -> '+st)}});
 page.on('requestfailed',rq=>{const failure=rq.failure&&rq.failure();errors.push('requestfailed '+rq.method()+' '+rq.url()+' -> '+(failure&&failure.errorText||'unknown'))});
 const base='http://127.0.0.1:'+server.address().port;
 const assertPostContract=label=>{
  const last=postLog[postLog.length-1];
  assert.ok(last,label+': the gesture reached the fixture approval endpoint');
  assert.equal(last.bodyLength,0,label+': the approval POST keeps the EMPTY body contract');
  assert.ok(last.hasCsrf,label+': the approval POST carries the session CSRF header');
  assert.ok(last.auth.startsWith('Enrollment '),label+': the enrollment password travels only in the Authorization header');
  assert.equal(postLog.length,expectedPosts,label+': exactly one POST per explicit gesture, no automatic retry');
 };
 // ---- approval form: web/paea-effects.html + web/paea-approval-form.js ----
 const approvalObs=()=>page.evaluate(()=>({
  state:document.querySelector('#approval-status').dataset.state,
  text:document.querySelector('#approval-status').textContent,
  checked:document.querySelector('#approval-confirm').checked,
  password:document.querySelector('#approval-password').value,
 }));
 // Causal readiness: only the page's own load can unhide #approval-detail and
 // move #approval-status out of ''/'loading' into a terminal state; the
 // admissibility facts are then asserted OUTSIDE the wait, before any gesture.
 const waitApprovalLoaded=()=>page.waitForFunction(()=>{const st=document.querySelector('#approval-status'),d=document.querySelector('#approval-detail');return !!st&&!!d&&st.dataset.state!==''&&st.dataset.state!=='loading'&&d.hidden===false});
 // One explicit gesture. The guarded exit sets its disable flags
 // synchronously inside the click dispatch, so once this resolves the cycle
 // has begun and the outcome wait below can never be satisfied by stale
 // state. DOM-level gestures only (same scope note as the maintained consent
 // example): they verify the EXACT JS state transitions, not physical
 // pointer hit-testing of the page layout.
 const approvalSubmit=async()=>{
  await page.evaluate(()=>document.querySelector('#approval-confirm').scrollIntoView({block:'center'}));
  await page.evaluate(()=>{const c=document.querySelector('#approval-confirm');c.checked=true;c.dispatchEvent(new Event('change',{bubbles:true}))});
  await page.waitForFunction(()=>{const b=document.querySelector('#approval-send');return !!b&&b.disabled===false});
  await page.evaluate(v=>{document.querySelector('#approval-password').value=v},enrollmentPassword);
  await page.evaluate(()=>document.querySelector('#approval-send').scrollIntoView({block:'center'}));
  expectedPosts++;
  await page.evaluate(()=>document.querySelector('#approval-send').click());
 };
 // Causal outcome wait (never expectation-shaped): the guarded exit disables
 // the password input synchronously inside the click dispatch and re-enables
 // it in its `finally` AFTER the outcome render, so `password re-enabled AND
 // any terminal status` can only be THIS submit's outcome. The EXPECTED state
 // is asserted outside the wait: a faulty page (an `admitted` reply rendered
 // as a success) records its failure instead of stalling the wait.
 const waitApprovalOutcome=()=>page.waitForFunction(()=>{const st=document.querySelector('#approval-status'),pw=document.querySelector('#approval-password');return !!st&&!!pw&&pw.disabled===false&&st.dataset.state!==''&&st.dataset.state!=='loading'});
 // ---- consent page: web/paea-consent.html + web/paea-consent.js ----
 const consentObs=()=>page.evaluate(()=>({
  state:document.querySelector('#result-status').dataset.state,
  text:document.querySelector('#result-status').textContent,
  detail:document.querySelector('#result-detail').textContent,
  password:document.querySelector('#consent-password').value,
 }));
 // Causal readiness: #consent-form is unhidden only by the page's own load,
 // after the bound/presentation reads; the CSRF prerequisite is re-asserted
 // OUTSIDE the wait, before any gesture.
 const waitConsentLoaded=()=>page.waitForFunction(()=>{const st=document.querySelector('#approval-status'),f=document.querySelector('#consent-form');return !!st&&!!f&&st.dataset.state!==''&&st.dataset.state!=='loading'&&f.hidden===false});
 const waitConsentSendable=()=>page.waitForFunction(()=>{const b=document.querySelector('#consent-send');return !!b&&b.disabled===false});
 // Causal outcome wait with THIS reply's nonce, never the EXPECTED state: the
 // submit disables the button and moves #result-status to 'loading'
 // synchronously inside the click dispatch, so a stale previous outcome can
 // never satisfy it; the wait resolves on the button released by the page's
 // own `finally`, ANY terminal status, and THIS reply's verbatim nonce echo
 // in the raw result box (the terminal-error escape covers a refusal that
 // renders without the raw echo). The EXPECTED state is asserted outside the
 // wait: a faulty page (`admitted` rendered as a success) records its
 // failure instead of hanging the check.
 const waitConsentOutcome=nonce=>page.waitForFunction(n=>{
  const b=document.querySelector('#consent-send'),st=document.querySelector('#result-status'),raw=document.querySelector('#result-raw');
  if(!b||!st||!raw)return false;
  const terminal=st.dataset.state!==''&&st.dataset.state!=='loading';
  return !b.disabled&&terminal&&(raw.textContent.includes(n)||st.dataset.state==='error');
 },{},nonce);
 const consentNonce=()=>('fixture-approval-object-'+postLog.length);
 const consentSubmit=async()=>{
  await waitConsentSendable();
  await page.evaluate(v=>{document.querySelector('#consent-password').value=v},enrollmentPassword);
  await page.evaluate(()=>document.querySelector('#consent-send').scrollIntoView({block:'center'}));
  expectedPosts++;
  await page.evaluate(()=>document.querySelector('#consent-send').click());
 };
 // Catalog oracle for the approved/denied human wordings: read live from the
 // page's own SwarmI18n in the CURRENT language — never an invented key.
 const say=(key,params)=>page.evaluate(([k,p])=>String(globalThis.SwarmI18n.t(k,p)),[key,params||{}]);
 const failurePath=path.join(out,'approval-state-fail-closed-failure.json');
 const writeFailure=()=>{
  failureWritten=errors.length;
  fs.writeFileSync(failurePath,JSON.stringify({step,message:firstFailure.message,stack:firstFailure.stack,checks,errors,postLog},null,2),{mode:0o600});
 };
 async function runApprovalForm(lang){
  const fr=lang==='fr';
  step='open approval form ('+lang+')';
  await page.goto(base+'/paea-effects.html?lang='+lang,{waitUntil:'load'});
  // Fail fast (no hang) when a one-surface baseline omits this surface.
  assert.ok(await page.evaluate(()=>!!document.querySelector('#approval-status')&&!!document.querySelector('#approval-detail')&&!!document.querySelector('#approval-confirm')&&!!document.querySelector('#approval-send')&&!!document.querySelector('#approval-password')),'approval form surface is not served from the web root (paea-effects.html must carry #approval-status/#approval-detail/#approval-confirm/#approval-send/#approval-password)');
  await waitApprovalLoaded();
  const loaded=await approvalObs();
  assert.equal(loaded.state,'success',lang+' approval form: the fixture presentation loads admissible (state '+loaded.state+')');
  const ready=await page.evaluate(()=>({detail:document.querySelector('#approval-detail').hidden,send:document.querySelector('#approval-send').disabled}));
  assert.equal(ready.detail,false,lang+' approval form: the admissible presentation is rendered before any gesture');
  assert.equal(ready.send,true,lang+' approval form: the send control waits for the explicit confirmation');
  const approvedWording=await say(KEYS.approvalApproved,{state:'approved'});
  const deniedWording=await say(KEYS.approvalDenied,{});
  // Valid approved control BEFORE any bad state: the ordinary full approved
  // object is a success.
  step='approval '+lang+' approved-full';
  postMode='approved-full';
  await approvalSubmit();
  await waitApprovalOutcome();
  let obs=await approvalObs();
  assert.equal(obs.state,'success',lang+' approval form: the ordinary full approved object is a success');
  assert.ok(obs.text.includes(approvedWording),lang+' approval form: the approved decision shows the catalog wording ('+lang+')');
  if(!fr)assert.ok(!obs.text.includes('Approbation enregistrée par le moteur'),lang+' approval form: the FR source approved wording must not leak on the EN surface');
  assert.equal(obs.checked,false,lang+' approval form: the explicit gesture resets after the recorded decision');
  assert.equal(obs.password,'',lang+' approval form: the approved exit erases the enrollment password');
  assertPostContract(lang+' approval form approved-full');
  checks.push(lang+' approval form: full approved object is the success');
  // Fail-closed bad states: NEVER a success. The unexpected-state wording is
  // the existing catalog key on BOTH surfaces (FR source and the supplied EN
  // translation), with the independent FR-leak check on the EN surface.
  for(const mode of ['admitted','unknown','missing','null-state','empty-state']){
   step='approval '+lang+' '+mode;
   postMode=mode;
   await approvalSubmit();
   await waitApprovalOutcome();
   obs=await approvalObs();
   assert.equal(obs.state,'error',lang+' approval form: '+mode+' must never render success');
   const shown={admitted:'admitted',unknown:'unknown'}[mode]??'';
   const prefix=fr?'Réponse inattendue':'Unexpected reply';
   assert.ok(obs.text.includes(prefix),lang+' approval form: '+mode+' shows the catalog unexpected-state wording ('+lang+')');
   if(shown)assert.ok(obs.text.includes(prefix+(fr?' : ':': ')+shown),lang+' approval form: '+mode+' shows the state inside the catalog wording');
   if(!fr)assert.ok(!obs.text.includes('Réponse inattendue'),lang+' approval form: the FR source unexpected wording must not leak on the EN surface');
   if(fr)assert.ok(!obs.text.includes('Unexpected reply'),lang+' approval form: the EN catalog wording must not leak on the FR surface');
   assert.equal(obs.password,'',lang+' approval form: '+mode+' erases the enrollment password on the refused exit');
   assert.equal(obs.checked,true,lang+' approval form: '+mode+' keeps the confirmation for the existing recovery');
   assertPostContract(lang+' approval form '+mode);
   checks.push(lang+' approval form: '+mode+' is fail-closed (never a success)');
  }
  // Denied stays a visible refusal with its exact reason.
  step='approval '+lang+' denied';
  postMode='denied';
  await approvalSubmit();
  await waitApprovalOutcome();
  obs=await approvalObs();
  assert.equal(obs.state,'error',lang+' approval form: the denial is never a success');
  assert.ok(obs.text.includes(deniedWording),lang+' approval form: the denial shows the catalog refusal wording ('+lang+')');
  assert.ok(obs.text.includes('fixture-separation-of-duties'),lang+' approval form: the denial keeps its exact reason visible');
  if(!fr)assert.ok(!obs.text.includes('Intention refusée'),lang+' approval form: the FR source denial wording must not leak on the EN surface');
  assert.equal(obs.password,'',lang+' approval form: the denial erases the enrollment password');
  assertPostContract(lang+' approval form denied');
  checks.push(lang+' approval form: denied stays a refusal with its reason');
  // Valid approved control AFTER the bad states: the redacted
  // trust-status-ack-v1 shape is equally a success — no redacted-only shape
  // is imposed — and the form recovers through its existing controls.
  step='approval '+lang+' approved-redacted';
  postMode='approved-redacted';
  await approvalSubmit();
  await waitApprovalOutcome();
  obs=await approvalObs();
  assert.equal(obs.state,'success',lang+' approval form: the redacted trust-status-ack-v1 with the approved state is a success');
  assert.ok(obs.text.includes(approvedWording),lang+' approval form: the redacted ack shows the catalog approved wording ('+lang+')');
  if(!fr)assert.ok(!obs.text.includes('Approbation enregistrée par le moteur'),lang+' approval form: the FR source approved wording must not leak on the EN surface (redacted ack)');
  assert.equal(obs.password,'',lang+' approval form: the redacted approved exit erases the enrollment password');
  assertPostContract(lang+' approval form approved-redacted');
  checks.push(lang+' approval form: redacted approved ack is the success (no redacted-only shape imposed)');
 }
 async function runConsent(lang){
  const fr=lang==='fr';
  step='open consent ('+lang+')';
  await page.goto(base+'/paea-consent.html?lang='+lang,{waitUntil:'load'});
  // Fail fast (no hang) when a one-surface baseline omits this surface.
  assert.ok(await page.evaluate(()=>!!document.querySelector('#result-status')&&!!document.querySelector('#result-detail')&&!!document.querySelector('#result-raw')&&!!document.querySelector('#consent-form')&&!!document.querySelector('#consent-send')&&!!document.querySelector('#consent-password')),'consent surface is not served from the web root (paea-consent.html must carry #result-status/#result-detail/#result-raw/#consent-form/#consent-send/#consent-password)');
  await waitConsentLoaded();
  assert.ok(await page.evaluate(()=>!!globalThis.SwarmConsent&&typeof globalThis.SwarmConsent.csrf==='string'&&globalThis.SwarmConsent.csrf.length>0),'consent surface: the native-session CSRF prerequisite is not established before the form');
  const approvedWording=await say(KEYS.consentApproved,{});
  const deniedWording=await say(KEYS.consentDenied,{});
  // Valid approved controls BEFORE any bad state, on BOTH producer shapes.
  for(const mode of ['approved-full','approved-redacted']){
   step='consent '+lang+' '+mode;
   postMode=mode;
   const nonce=consentNonce();
   await consentSubmit();
   await waitConsentOutcome(nonce);
   const obs=await consentObs();
   assert.equal(obs.state,'success',lang+' consent: '+mode+' is a success');
   assert.ok(obs.text.includes(approvedWording),lang+' consent: '+mode+' shows the catalog approved wording ('+lang+')');
   if(!fr)assert.ok(!obs.text.includes('Accusé du canal d’approbation'),lang+' consent: the FR source approved wording must not leak on the EN surface');
   assert.equal(obs.password,'',lang+' consent: '+mode+' erases the enrollment password');
   assertPostContract(lang+' consent '+mode);
   checks.push(lang+' consent: '+mode+' is the success');
  }
  // Fail-closed bad states: NEVER a success. The unexpected-state wording is
  // the existing catalog key asserted on BOTH surfaces (FR source and the
  // supplied EN translation) across the status AND detail rendering, with
  // the independent FR-leak check on the EN surface.
  for(const mode of ['admitted','unknown','pending','committed','missing','null-state','empty-state']){
   step='consent '+lang+' '+mode;
   postMode=mode;
   const nonce=consentNonce();
   await consentSubmit();
   await waitConsentOutcome(nonce);
   const obs=await consentObs();
   assert.equal(obs.state,'error',lang+' consent: '+mode+' must never render success');
   const shown={admitted:'admitted',unknown:'unknown',pending:'pending_approval',committed:'committed'}[mode]??'';
   const rendered=obs.text+'\n'+obs.detail;
   const prefix=fr?'Réponse inattendue':'Unexpected reply';
   assert.ok(rendered.includes(prefix),lang+' consent: '+mode+' shows the catalog unexpected-state wording ('+lang+')');
   if(shown)assert.ok(rendered.includes(prefix+(fr?' : ':': ')+shown),lang+' consent: '+mode+' shows the state inside the catalog wording');
   if(!fr)assert.ok(!rendered.includes('Réponse inattendue'),lang+' consent: the FR source unexpected wording must not leak on the EN surface');
   if(shown)assert.ok(obs.detail.includes(shown),lang+' consent: '+mode+' keeps the state visible verbatim');
   assert.equal(obs.password,'',lang+' consent: '+mode+' erases the enrollment password');
   assertPostContract(lang+' consent '+mode);
   checks.push(lang+' consent: '+mode+' is fail-closed (never a success)');
  }
  // Denied stays a visible refusal with its exact reason.
  step='consent '+lang+' denied';
  postMode='denied';
  {
   const nonce=consentNonce();
   await consentSubmit();
   await waitConsentOutcome(nonce);
   const obs=await consentObs();
   assert.equal(obs.state,'error',lang+' consent: the denial is never a success');
   assert.ok(obs.text.includes(deniedWording),lang+' consent: the denial shows the catalog refusal wording ('+lang+')');
   assert.ok(obs.detail.includes('fixture-separation-of-duties'),lang+' consent: the denial keeps its exact reason visible');
   if(!fr)assert.ok(!obs.text.includes('Refus enregistré par le moteur'),lang+' consent: the FR source denial wording must not leak on the EN surface');
   assert.equal(obs.password,'',lang+' consent: the denial erases the enrollment password');
   assertPostContract(lang+' consent denied');
   checks.push(lang+' consent: denied stays a refusal with its reason');
  }
 }
 try{
  await runApprovalForm('fr');
  await runApprovalForm('en');
  await runConsent('fr');
  await runConsent('en');
  assert.equal(checks.length,EXPECTED_CHECKS,'the regression must record exactly the '+EXPECTED_CHECKS+' named checks (recorded '+checks.length+')');
 }catch(error){
  firstFailure=error;
  writeFailure();
  console.error('FAIL ['+step+']: '+error.message);
  process.exitCode=1;
 }
 // Cleanup after the outcome is recorded: its errors never replace the first
 // failure and never disappear — they join the same retained evidence. The
 // SAME shared owned-cleanup helper (closeOwned) is also awaited on the
 // fatal path at the end of this contract, so the owned browser and server
 // close on EVERY exit, and a server.close callback error (if any) is
 // recorded, never ignored.
 await closeOwned();
 if(firstFailure){
  if(errors.length!==failureWritten)writeFailure();
 }else{
  try{
   // The PASS gate runs AFTER cleanup and BEFORE the PASS artifact and
   // exit 0: zero unexpected diagnostics of any kind (page errors, console
   // errors, unexpected non-2xx responses, failed requests, handler,
   // clientError, server or close errors).
   assert.equal(errors.length,0,'unexpected diagnostics: '+(errors.join(' | ')||'none'));
   fs.writeFileSync(path.join(out,'approval-state-fail-closed.json'),JSON.stringify({schema_version:1,kind:'paea-approval-state-fail-closed-v1',checks,unexpected_diagnostics:errors,at:new Date().toISOString()},null,2)+'\n');
   console.log(JSON.stringify({checks:checks.length,unexpected_diagnostics:errors.length}));
  }catch(error){
   firstFailure=error;
   writeFailure();
   console.error('FAIL [diagnostics]: '+error.message);
   process.exitCode=1;
  }
 }
})().catch(async error=>{
 const caught='FAIL: '+String(error&&error.message||error);
 errors.push('fatal: '+String(error&&error.message||error));
 if(!firstFailure)firstFailure=error;
 // Failure-artifact contract (write errors are never swallowed): the
 // artifact is attempted BEFORE the owned cleanup — the first cause
 // survives even a stalled close — and rewritten AFTER it only if that
 // first write succeeded and cleanup actually added diagnostics; a failed
 // first write is never reattempted — its exact cause joins the retained
 // errors. A real write error is kept and printed to stderr beside the
 // primary cause; if the artifact ultimately
 // cannot be written, the collected diagnostics are logged to stderr too
 // (they never contain secrets — no password, no header value), and the
 // run still exits 1: no silent success and no empty artifact.
 const artifactPath=outDir?path.join(outDir,'approval-state-fail-closed-failure.json'):null;
 const writeArtifact=()=>{fs.writeFileSync(artifactPath,JSON.stringify({step,message:firstFailure.message,stack:firstFailure.stack,checks,errors,postLog},null,2),{mode:0o600});return errors.length};
 let artifactCause=null,written=0;
 if(artifactPath){try{written=writeArtifact()}catch(writeError){artifactCause='FAIL [failure artifact not written]: '+String(writeError&&writeError.message||writeError);errors.push(artifactCause)}}
 // Owned cleanup on EVERY exit — the same shared helper as the normal path;
 // its close errors join `errors` (never ignored, never replacing the
 // first failure).
 try{await closeOwned()}catch(cleanupError){errors.push('fatal cleanup: '+String(cleanupError&&cleanupError.message||cleanupError))}
 if(artifactPath&&!artifactCause&&errors.length!==written){
  try{written=writeArtifact()}catch(writeError){artifactCause='FAIL [failure artifact update not written]: '+String(writeError&&writeError.message||writeError);errors.push(artifactCause)}
 }
 if(firstFailure!==error)console.error('FAIL ['+step+']: '+String(firstFailure&&firstFailure.message||firstFailure));
 console.error(caught);
 if(artifactCause){
  console.error(artifactCause);
  if(errors.length>0)console.error('FAIL [artifact unwritten; collected diagnostics]: '+errors.join(' | '));
 }
 process.exitCode=1;
});