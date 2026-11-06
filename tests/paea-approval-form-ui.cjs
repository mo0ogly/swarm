'use strict';
// Bounded browser qualification of the served PAEA approval form with a REAL
// frozen approval command, fresh then stale. Owned two-phase control protocol
// with the --ignored Rust test approval_form_serves_real_browser_session:
// that test runs the real mandate/request producers, freezes the approve
// command, binds it to a real native HTTP server with its ordinary persisted
// session, produces the REAL BLP reader authorization for the exact prepared
// action (blp-policy-apply + blp-object-attribute), publishes server.json
// (including the fixture reader_token), then waits for owned marker files
// (nonce-bound). This harness drives the real served page in a real Chrome:
// the guarded presentation first refuses the bare session with its exact
// paea-reader code (no projection), then the harness TYPES the reader token
// into the page's ephemeral reader input and re-reads through the form's own
// re-read control — no header injection, no storage: the page carries the
// credential as a request-time Bearer and clears the input right after.
// Phase 1 then performs the EXPLICIT confirmation + enrollment consumption
// through the form (labeled fixture password — auth simulated, acknowledged),
// asserts the real approved ack is shown (never a denied-as-success) and the
// password field is erased; phase 2 runs a real public Trust config-apply
// through the engine plus the real policy renewal/re-attribution (inside the
// test) and re-observes the explicit stale refusal with no confirmation
// solicited. The command is NEVER transported: consumption POSTs an empty
// body and the server holds the frozen command (bound approval); no mock
// HTTP/DOM, no Git effect, no second approval system, no acceptance;
// screenshots and console/request assertions stay in the owned evidence dir.
// Phase 2 also carries the real-keyboard-gesture regression of the received
// counterexample (2026-11-04, supervisor-stale-confirm-discriminant): focus
// on the confirmation checkbox + a real Space press used to check the
// ENABLED checkbox and re-enable send on the stale error state — the change
// listener ignored admissibility. The corrected UI contract disables the
// checkbox itself on any non-admissible presentation, so the gesture cannot
// check it, send stays disabled and no approval POST is made; recovery runs
// through the existing re-read control (exercised in phase 1: usable
// confirmation again after the authorized re-read).
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const pup=require(path.join(process.cwd(),'node_modules','puppeteer'));
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),control=path.resolve(process.argv[4]);
fs.mkdirSync(out,{recursive:true});
const server=JSON.parse(fs.readFileSync(path.join(control,'server.json'),'utf8'));
const root=server.service_root,origin=`http://${server.address}`;
const nonce=server.nonce;
const secret=fs.readFileSync(path.join(root,'.swarm',server.session_file),'utf8').trim();
// Fixture enrollment password of the retained native source fixture
// (tests/common/authenticated_fixture.rs), labeled simulated — the real
// human enrollment stays a declared open precondition of live operation.
const PASSWORD='STARTUP_FIXTURE_PASSWORD';
const checks=[],errors=[];
// Exact reader-frontier classification (correction of the stale oracle): the
// bare-session GET /api/v1/paea/reconcile-effects fired by every page load is
// refused by the reconcile reader gate BEFORE any settings read, ledger open
// or journal read — typed 403 whose failure body carries ONLY the stable
// guard code on `failure.code`/`error`. The former oracle expected HTTP 500
// (the settings-paea-ledger-root-not-configured precondition): that branch is
// unreachable for a bare session since the reader gate precedes every ledger
// IO, so the 500 expectation is REMOVED, not relaxed — any 500, and any other
// status/method/URL, stays an unexpected failure. The body code is OBSERVED
// through the browser, never assumed: the classification (and the
// console-duplicate allowance it grants) is decided only on the confirmed
// exact code; a body that fails to confirm stays a recorded failure. The
// browser console error of a failed fetch carries NO URL (Chrome omits it),
// so the console listener cannot classify by URL: a fetch-failure message
// that arrives before its response body is confirmed is deferred — never
// dropped — and attaches at the settle points to exactly one confirmed
// refusal; a console error that attaches to no confirmed refusal becomes a
// failure. No blanket ignore of any status class remains.
const pendingConsole=[];   // confirmed refusals awaiting their console duplicate
const deferredConsole=[];  // fetch-failure console messages awaiting their confirmed response
const reconcileReads=[];   // in-flight body confirmations of bare reconcile reads
const expectedRefusals=[]; // observed coded refusals, kept verbatim in the proof
const READER_GUARD_CODE='paea-reader-authentication-required'; // the ONLY code a bare reconcile refusal may carry
let expected=0; // confirmed expected denials: classified reader refusals + the armed stale POST refusal
// Settle the in-flight body confirmations, then attach the deferred console
// duplicates to the allowances their confirmed responses granted. With flush
// set, every still-unattached entry becomes a recorded failure: the verdict
// is rendered only after all observations (responses, bodies, duplicates)
// have settled, and nothing is dropped or blanket-ignored on the way.
async function settleReaderObservations(flush){
 await Promise.all(reconcileReads.splice(0));
 for(let i=deferredConsole.length-1;i>=0;i--){
  const j=pendingConsole.indexOf(deferredConsole[i].status);
  if(j>=0){pendingConsole.splice(j,1);deferredConsole.splice(i,1)}
 }
 if(flush)while(deferredConsole.length){const d=deferredConsole.shift();errors.push(d.text+' [fetch-failure console message attached to no confirmed expected refusal]')}
}
// Armed just before the deliberate post-stale refusal probe and never reset:
// both the response and its browser-console duplicate must then classify as
// the expected denial instead of unexpected diagnostics. Any other status on
// the approval POST (a 500, for instance) stays unexpected.
let staleRefusalArmed=false;
// Approval POST counter over the response channel: the keyboard-gesture
// regression must make NO additional approval POST. Before the armed stale
// probe the exact count is 2 — the phase-1 explicit consumption and the
// replay-while-current, both 200 acks — read after the EN navigation, so
// the channel has fully settled (no racy same-tick read).
let approvalPosts=0;
let browser;
(async()=>{
 browser=await pup.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
 const page=await browser.newPage();
 page.setDefaultTimeout(20000);
 page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{
  if(m.type()!=='error')return;
  const match=m.text().match(/status of (\d{3})/);
  if(match){
   const i=pendingConsole.indexOf(match[1]);
   if(i>=0){pendingConsole.splice(i,1);return}
   // Chrome omits the URL: this duplicate may precede its response's body
   // confirmation. Defer it verbatim — never drop it — until a settle point
   // attaches it to exactly one confirmed refusal or records it as a failure.
   deferredConsole.push({status:match[1],text:m.text()});return;
  }
  errors.push(m.text());
 });
 page.on('response',r=>{
  if(r.url().endsWith('/api/v1/paea/git-call/approval')&&r.request().method().toUpperCase()==='POST')approvalPosts+=1;
  if(r.status()<400)return;
  if(r.url()===origin+'/api/v1/paea/reconcile-effects'){
   if(r.status()===403&&r.request().method().toUpperCase()==='GET'){
    // The reader refusal is confirmed through the browser, never assumed:
    // the allowance (and the console-duplicate pairing it grants) is decided
    // only on the observed body, which must carry the exact stable guard code
    // on both `error` and `failure.code`. The confirmation is awaited at the
    // settle points; a body that fails to confirm is a recorded failure.
    const confirm=(async()=>{
     try{
      const parsed=JSON.parse(await r.text());
      if(parsed===null||typeof parsed!=='object'||parsed.error!==READER_GUARD_CODE||parsed.failure===null||typeof parsed.failure!=='object'||parsed.failure.code!==READER_GUARD_CODE)throw new Error('body without the exact reader guard code: '+JSON.stringify(parsed));
      expectedRefusals.push({method:'GET',url:r.url(),status:403,body:parsed});
      expected+=1;pendingConsole.push('403');
     }catch(err){errors.push('unconfirmed reconcile refusal 403 '+r.url()+': '+err.message)}
    })();
    reconcileReads.push(confirm);
    return;
   }
   // The former oracle expected HTTP 500 (the settings ledger-root
   // precondition). The reader gate precedes every settings read, ledger open
   // and journal read, so a bare session can no longer reach that
   // precondition: no 500 expectation remains, and a 500 — like any other
   // status or method on this URL — stays an unexpected failure.
   errors.push(r.status()+' '+r.url());return;
  }
  if(staleRefusalArmed&&r.status()===403&&r.url().endsWith('/api/v1/paea/git-call/approval')){expected+=1;pendingConsole.push('403');return}
  errors.push(r.status()+' '+r.url());
 });
 // Real session: the ordinary persisted credential, set as the ordinary cookie.
 async function open(lang,theme,width){
  await page.setCookie({name:server.cookie_name,value:secret,domain:'127.0.0.1',path:'/',httpOnly:true,sameSite:'Strict'});
  await page.setViewport({width,height:900});
  await page.goto(origin+`/paea-effects.html?lang=${lang}`,{waitUntil:'networkidle2'});
  if(await page.$eval('html',e=>e.dataset.theme)!==theme){await page.click('#paea-theme')}
 }
 // Atomic marker publication: the Rust test polls this exact filename every
 // 100ms and treats a read that lacks our nonce as fatal, so a plain
 // writeFileSync (truncate-then-write window) raced that poll and failed one
 // run with "marker phase1.done present but not ours" on a file whose final
 // bytes matched. tmp+rename publishes either nothing or the complete bytes.
 function write(name,value){const tmp=path.join(control,'.'+name+'.tmp');fs.writeFileSync(tmp,JSON.stringify(Object.assign({nonce},value)));fs.renameSync(tmp,path.join(control,name))}
 function body(id){return page.$eval(id,e=>({text:e.textContent,rows:[...e.querySelectorAll('dt,dd')].map(x=>x.textContent)}))}
 // ---------- Phase 1: fresh frozen approval, explicit consumption ----------
 // Reader frontier first: the bare persisted session alone refuses the guarded
 // presentation with the exact typed code (HTTP 200 available:false) BEFORE
 // any projection — this IS the real product refusal the correction connects.
 await open('fr','etat',1280);
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 await settleReaderObservations(true);
 const bareStatus=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(bareStatus.state,'empty','bare session must stay the explicit empty/refused state, never a success');
 assert.ok(bareStatus.text.includes('paea-reader-authentication-required'),'bare-session refusal must name the exact reader code, got: '+bareStatus.text);
 assert.equal(await page.$eval('#approval-send',e=>e.disabled),true,'refused presentation must keep send disabled');
 assert.equal(await page.$eval('#approval-detail',e=>e.hidden),true,'refused presentation must show no projection');
 // The real reader connection: type the fixture reader credential into the
 // page's EPHEMERAL input and drive the form's own re-read control — the page
 // then carries the Bearer itself (no setExtraHTTPHeaders, no page-side
 // storage) and clears the input right after the guarded read.
 await page.type('#approval-reader-credential',server.reader_token);
 await page.click('#approval-reload');
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 const freshStatus=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(freshStatus.state,'success','reader-authorized presentation must be the explicit success state');
 assert.ok(freshStatus.text.includes('Approbation présentée'),'FR fresh message must name the presented approval, got: '+freshStatus.text);
 // Recovery through the existing path: this admissible read followed the
 // non-admissible bare refusal, so the confirmation checkbox being usable
 // again IS the common admissibility contract's recovery witness.
 assert.equal(await page.$eval('#approval-confirm',e=>e.disabled),false,'admissible re-read must restore a usable confirmation through the existing re-read control');
 // Reader credential erased by the page right after the guarded read: never
 // in the input, the page source or any storage.
 assert.equal(await page.$eval('#approval-reader-credential',e=>e.value),'','reader input must be cleared after the guarded read');
 assert.ok(!JSON.stringify(await page.evaluate(()=>({html:document.documentElement.innerHTML,ls:localStorage.getItem('swarm-approval-password'),ss:sessionStorage.getItem('swarm-approval-password')}))).includes(server.reader_token),'reader credential must not persist anywhere in the page');
 // The send button starts disabled: no confirmation gesture yet.
 assert.equal(await page.$eval('#approval-send',e=>e.disabled),true,'send must start disabled without explicit confirmation');
 // Authorized projection, verbatim from the served payload: the engine-derived
 // terms (server.json carries the same values asserted against the engine side
 // of the test). The private command identity (action_id, content_digest) is
 // never rendered.
 const command=await body('#approval-declared');
 assert.ok(command.text.includes(server.resource),'declared projection must present the exact resource');
 assert.ok(!command.text.includes(server.action_id),'private action_id must never be rendered');
 assert.ok(!command.text.includes(server.content_digest),'private content_digest must never be rendered');
 // Witness section: frozen digest verbatim, bound single operation, both roots
 // bound, revision current.
 const witness=await body('#approval-witness');
 assert.ok(witness.text.includes('approve'),'witness must present the bound single operation');
 assert.ok(witness.text.includes(server.request_digest),'witness must present the exact frozen command digest');
 assert.ok(witness.text.includes('oui'),'witness must report roots bound and revision current');
 // No password left in any field, page source or storage before the gesture.
 assert.ok(!JSON.stringify(await page.evaluate(()=>({html:document.documentElement.innerHTML,ls:localStorage.getItem('swarm-approval-password'),ss:sessionStorage.getItem('swarm-approval-password')}))).includes(PASSWORD),'password must not pre-exist anywhere in the page');
 // The explicit gesture: checkbox, fixture password (labeled simulated), send.
 await page.click('#approval-confirm');
 assert.equal(await page.$eval('#approval-send',e=>e.disabled),false,'explicit confirmation must enable send');
 await page.type('#approval-password',PASSWORD);
 await page.click('#approval-send');
 // The ack is the engine's real decision: approved must be shown as such.
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state==='success'&&document.getElementById('approval-status').textContent.includes('décision reçue'),{timeout:20000});
 const ackStatus=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.ok(ackStatus.text.includes('(approved)'),'ack must show the real approved state, got: '+ackStatus.text);
 assert.ok(ackStatus.text.includes('Aucun effet Git'),'ack must state no Git effect was executed by the form');
 // Password erased after use: field empty, nowhere in page/storage.
 assert.equal(await page.$eval('#approval-password',e=>e.value),'','password field must be erased after use');
 assert.ok(!JSON.stringify(await page.evaluate(()=>({html:document.documentElement.innerHTML,ls:localStorage.getItem('swarm-approval-password'),ss:sessionStorage.getItem('swarm-approval-password')}))).includes(PASSWORD),'password must not persist anywhere in the page');
 // Confirmation reset: a replay would need a fresh explicit gesture.
 assert.equal(await page.$eval('#approval-confirm',e=>e.checked),false,'confirmation must reset after consumption');
 assert.equal(await page.$eval('#approval-send',e=>e.disabled),true,'send must disable again after consumption');
 assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),'1280px no overflow');
 await (await page.$('#approval-detail')).screenshot({path:path.join(out,'approval-fresh-fr.png')});
 // EN + dark theme, 390px, on the same consumed state (reload refetches). The
 // reload starts from the bare session again (the reader input is always
 // empty after the page's guarded read), so the EN observation replays the
 // same real reader connection: refusal of the bare session, then the typed
 // credential through the form's re-read control.
 await open('en','sombre',390);
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 await settleReaderObservations(true);
 const bareEn=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(bareEn.state,'empty','EN bare session must refuse identically');
 assert.ok(bareEn.text.includes('Read refused'),'EN bare-session refusal must be translated, got: '+bareEn.text);
 await page.type('#approval-reader-credential',server.reader_token);
 await page.click('#approval-reload');
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 const freshEn=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(freshEn.state,'success');
 assert.ok(/Approval presented/i.test(freshEn.text),'EN fresh message, got: '+freshEn.text);
 assert.ok((await body('#approval-declared')).text.includes(server.resource),'EN declared projection presents the same exact resource');
 assert.equal(await page.$eval('#approval-reader-credential',e=>e.value),'','EN reader input must be cleared after the guarded read');
 assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),'390px no overflow');
 await (await page.$('#approval-detail')).screenshot({path:path.join(out,'approval-fresh-en.png')});
 checks.push('reader frontier connected: bare session refuses with the exact paea-reader code FR/EN (no projection), typed ephemeral credential through the form re-read control authorizes the real projection, credential cleared after every read');
 // Direct API cross-check through the page context: the presented values come
 // from the real endpoint payload (same values asserted twice, two surfaces).
 // The guarded read carries the SAME reader credential the page used, typed
 // here as the request-time Bearer of this one cross-check — a bare session
 // fetch must refuse, which is asserted first through the same context.
 const bareApi=await page.evaluate(async()=>{const r=await fetch('/api/v1/paea/git-call/approval/presentation',{cache:'no-store',credentials:'same-origin'});return {status:r.status,body:await r.json()}});
 assert.equal(bareApi.status,200);
 assert.equal(bareApi.body.available,false,'bare-session direct API must refuse the guarded presentation');
 assert.equal(bareApi.body.reason,'paea-reader-authentication-required');
 const api=await page.evaluate(async token=>{const r=await fetch('/api/v1/paea/git-call/approval/presentation',{cache:'no-store',credentials:'same-origin',headers:{Authorization:'Bearer '+token}});return {status:r.status,body:await r.json()}},server.reader_token);
 assert.equal(api.status,200);assert.equal(api.body.available,true);
 assert.equal(api.body.request_digest,server.request_digest);
 assert.equal(api.body.operation,'approve');
 assert.equal(api.body.agreement_terms.resource,server.resource);
 assert.equal(api.body.agreement_terms.revision,server.trust_revision);
 assert.ok(api.body.command_bytes_b64url===undefined,'presentation must never serve command bytes');
 assert.ok(api.body.command===undefined,'presentation must never serve the parsed private command');
 assert.equal(api.body.current.roots_bound,true);
 assert.equal(api.body.current.trust_revision_current,true);
 assert.equal(api.body.limits.consent,'none-captured');
 // Replay while still current, BEFORE the stale mutation: the engine replays
 // the recorded outcome of the same held frozen command (the server-held
 // command is not re-approved fresh). The Rust server-route test proves the
 // complement: AFTER the Trust revision change the same empty-body POST
 // refuses (the held enrolled authority is bound to the consumed revision),
 // so no post-stale replay is attempted here; the phase-2 refusal is asserted
 // in the browser observation below and classified expected.
 const replay=await page.evaluate(async pwd=>{
  const s=await (await fetch('/api/v1/native-session',{cache:'no-store',credentials:'same-origin'})).json();
  const b64=btoa(pwd);
  const r=await fetch('/api/v1/paea/git-call/approval',{method:'POST',cache:'no-store',credentials:'same-origin',headers:{'X-Swarm-CSRF':s.csrf_token,Authorization:'Enrollment '+b64.replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'')}});
  return {status:r.status,body:await r.json()};
 },PASSWORD);
 assert.equal(replay.status,200,'replay through the real channel stays an HTTP 200 ack while current');
 assert.equal(replay.body.state,'approved','replay returns the recorded decision of the same frozen command');
 assert.equal(replay.body.kind,'trust-status-ack-v1');
 checks.push('replay while current: empty-body POST through the real channel returns the recorded ack of the server-held command, never a second fresh approval');
 write('phase1.done',{status:'ok'});
 // ---------- Phase 2: real public Trust revision change, stale state ----------
 // Bounded wait on the Rust-side marker: the engine phase performs a real
 // config-apply before publishing phase2.ready; 60s at 100ms polls exceeds
 // any observed engine phase by an order of magnitude and stays a test
 // coordination bound, not a product policy.
 let phase2Ready=null;
 for(let i=0;i<600;i++){
  try{
   const bytes=fs.readFileSync(path.join(control,'phase2.ready'),'utf8');
   if(bytes.includes(nonce)){phase2Ready=JSON.parse(bytes);break}
  }catch(err){/* absent or mid-publication: keep polling within the window */}
  await new Promise(r=>setTimeout(r,100))
 }
 assert.ok(phase2Ready,'phase2.ready marker never appeared within the bounded coordination window');
 const phase2=JSON.parse(fs.readFileSync(path.join(control,'phase2.ready'),'utf8'));
 assert.equal(phase2.nonce,nonce);
 await open('fr','etat',1280);
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 await settleReaderObservations(true);
 const bareStaleFr=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(bareStaleFr.state,'empty','stale bare session must still refuse first, never project');
 assert.ok(bareStaleFr.text.includes('paea-reader-authentication-required'),'stale bare-session refusal must keep the exact reader code, got: '+bareStaleFr.text);
 await page.type('#approval-reader-credential',server.reader_token);
 await page.click('#approval-reload');
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 const staleStatus=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(staleStatus.state,'error','stale approval must stay a visible error, never a success');
 assert.ok(staleStatus.text.includes('périmée'),'FR stale message must name the stale state, got: '+staleStatus.text);
 // No confirmation solicited: send disabled, checkbox unchecked.
 assert.equal(await page.$eval('#approval-send',e=>e.disabled),true,'stale state must keep send disabled');
 assert.equal(await page.$eval('#approval-confirm',e=>e.checked),false,'stale state must keep confirmation unchecked');
 // Real-keyboard-gesture regression (received counterexample, 2026-11-04):
 // the checkbox is DISABLED on this non-admissible state, so no keyboard
 // gesture can check it or re-enable send. The gesture's focus precondition
 // is EXPLICIT and OBSERVED (received disabled-focus discriminant: a
 // disabled control cannot take focus — the former attempt left
 // activeElement on the reload button after its click, so the blind Space
 // acted on ANOTHER control and started the re-read loop; an oracle defect,
 // never a product defect): first neutralize the focus left by the
 // preceding re-read click and observe the active element, then attempt to
 // focus the disabled confirmation and observe that the focus cannot move
 // onto it. Only then the real Space is pressed: it can reach neither the
 // checkbox (unfocusable) nor any other control (focus is neutral on body).
 assert.equal(await page.$eval('#approval-confirm',e=>e.disabled),true,'stale state must disable the confirmation itself, not only send');
 await page.evaluate(()=>{const a=document.activeElement;if(a&&typeof a.blur==='function')a.blur()});
 const focusNeutral=await page.evaluate(()=>({active:document.activeElement===document.body?'body':(document.activeElement.id||document.activeElement.tagName)}));
 assert.equal(focusNeutral.active,'body','neutral focus precondition: focus must leave the re-read control before the gesture, got: '+JSON.stringify(focusNeutral));
 await page.focus('#approval-confirm');
 const focusAttempt=await page.evaluate(()=>({active:document.activeElement===document.body?'body':(document.activeElement.id||document.activeElement.tagName),confirmDisabled:document.getElementById('approval-confirm').disabled}));
 assert.equal(focusAttempt.active,'body','a disabled confirmation must not take focus, got: '+JSON.stringify(focusAttempt));
 assert.equal(focusAttempt.confirmDisabled,true,'the confirmation stays disabled through the focus attempt');
 await page.keyboard.press('Space');
 const staleKeyboard=await page.evaluate(()=>({checked:document.getElementById('approval-confirm').checked,confirmDisabled:document.getElementById('approval-confirm').disabled,sendDisabled:document.getElementById('approval-send').disabled,status:document.getElementById('approval-status').dataset.state,detailHidden:document.getElementById('approval-detail').hidden}));
 assert.equal(staleKeyboard.checked,false,'real Space on the disabled stale checkbox must not check it');
 assert.equal(staleKeyboard.confirmDisabled,true,'real Space must not re-enable the stale confirmation');
 assert.equal(staleKeyboard.sendDisabled,true,'real Space must not re-enable send on the stale state');
 assert.equal(staleKeyboard.status,'error','the visible stale error must stay after the gesture');
 assert.equal(staleKeyboard.detailHidden,false,'the maintained refusal projection must stay visible after the gesture');
 const staleWitness=await body('#approval-witness');
 assert.ok(staleWitness.text.includes('non'),'stale witness must report revision-current = non');
 await (await page.$('#approval-detail')).screenshot({path:path.join(out,'approval-stale-fr.png')});
 await open('en','sombre',390);
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 await settleReaderObservations(true);
 await page.type('#approval-reader-credential',server.reader_token);
 await page.click('#approval-reload');
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 const staleEn=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(staleEn.state,'error');
 assert.ok(/Stale approval/i.test(staleEn.text),'EN stale message, got: '+staleEn.text);
 // The non-confirmable state is language-independent: the disabled checkbox
 // and gated send hold in EN too.
 assert.equal(await page.$eval('#approval-confirm',e=>e.disabled),true,'EN stale state must disable the confirmation itself');
 assert.equal(await page.$eval('#approval-send',e=>e.disabled),true,'EN stale state must keep send disabled');
 await (await page.$('#approval-detail')).screenshot({path:path.join(out,'approval-stale-en.png')});
 checks.push('stale: real Trust revision change (policy renewed through real producers) → visible error FR/EN behind the reader frontier, no confirmation solicited, and the real keyboard gesture (focus + Space) cannot check the disabled confirmation nor re-enable send — the state stays error with the refusal projection maintained, no normalization');
 // Post-stale refusal cross-check: the same channel now refuses (the held
 // enrolled authority is bound to the consumed Trust revision — engine
 // contract proven by the server-route test). A browser console duplicate of
 // this already-classified response carries no URL; the listener classifies
 // only by URL, so the empty-body POST refusal is classified here as expected.
 // The browser page itself must not have consumed twice: exactly one phase-1
 // send happened.
 // Settled no-POST observation for the whole gesture window: between the
 // phase-1 consumption and here, the only approval POSTs are the explicit
 // consumption and the replay-while-current (both 200 acks) — the keyboard
 // gesture and the EN flow made none. Read after the EN navigation, so the
 // response channel has fully settled; a third POST here is a failure.
 assert.equal(approvalPosts,2,'no additional approval POST between the explicit consumption and the armed stale probe (gesture window included), got: '+approvalPosts);
 staleRefusalArmed=true;
 const stalePost=await page.evaluate(async pwd=>{
  const s=await (await fetch('/api/v1/native-session',{cache:'no-store',credentials:'same-origin'})).json();
  const b64=btoa(pwd);
  const r=await fetch('/api/v1/paea/git-call/approval',{method:'POST',cache:'no-store',credentials:'same-origin',headers:{'X-Swarm-CSRF':s.csrf_token,Authorization:'Enrollment '+b64.replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'')}});
  return {status:r.status,body:await r.json()};
 },PASSWORD);
 assert.equal(stalePost.status,403,'post-stale replay must refuse typed (403), never re-approve or replay');
 assert.notEqual(stalePost.body.state,'approved','post-stale replay must never return an approved state');
 checks.push('post-stale refusal: the same real channel refuses typed (403) after the actual Trust revision change, never re-approves and never replays');
 // Final verdict on the reader-frontier observations — rendered only once the
 // observations are really finished: never after an arbitrary duration, never
 // on a claimed measurement. Real completion, in order: (1) await the
 // in-flight reconcile body confirmations by their actual promises; (2) close
 // the controlled page — the page's own lifecycle is the end of its event
 // stream, so after its close no console/response event can still arrive for
 // it; (3) drain what remains before the verdict: attach any deferred console
 // duplicate to exactly one confirmed refusal and turn every leftover into a
 // recorded failure. Every bare reconcile read of this run must have
 // confirmed the exact typed GET 403 refusal — four page loads, four coded
 // refusals, kept verbatim in the proof's expected_refusals — and every
 // confirmed refusal must have its own console duplicate; anything else
 // already landed in the errors above.
 await settleReaderObservations(false);
 await page.close();
 await settleReaderObservations(true);
 if(pendingConsole.length)errors.push('confirmed expected refusals left without an observed console duplicate: '+pendingConsole.join(', '));
 assert.equal(expectedRefusals.length,4,'exactly four bare reconcile reads must have confirmed the typed reader refusal, got: '+JSON.stringify(expectedRefusals));
 for(const refusal of expectedRefusals){
  assert.equal(refusal.method,'GET','each reconcile refusal must be the bare GET');
  assert.equal(refusal.url,origin+'/api/v1/paea/reconcile-effects','each reconcile refusal must target the exact reconcile URL');
  assert.equal(refusal.status,403,'each reconcile refusal must be the typed 403');
  assert.equal(refusal.body.error,READER_GUARD_CODE,'each reconcile refusal body must name the exact reader code');
  assert.equal(refusal.body.failure.code,READER_GUARD_CODE,'each reconcile refusal failure.code must name the exact reader code');
 }
 // The Rust test's owned protocol waits for phase2.done to run its own
 // shutdown; publish it (atomic write) only after every phase-2 assertion
 // has held, so a successful harness always releases the waiter.
 write('phase2.done',{status:'ok'});
 fs.writeFileSync(path.join(out,'approval-form-ui.json'),JSON.stringify({schema_version:1,kind:'paea-approval-form-ui-v1',checks,unexpected_diagnostics:errors,expected_denials:expected,expected_refusals:expectedRefusals,server:{request_digest:server.request_digest,action_id:server.action_id,resource:server.resource,trust_revision_fresh:server.trust_revision,trust_revision_stale:phase2.trust_revision},at:new Date().toISOString()},null,2)+'\n');
 console.log(JSON.stringify({checks:checks.length,unexpected_diagnostics:errors.length}));
})().catch(e=>{
 // No diagnostics lost on early failure either: flush the deferred console
 // duplicates verbatim and report body confirmations still in flight.
 while(deferredConsole.length){const d=deferredConsole.shift();errors.push(d.text+' [fetch-failure console message attached to no confirmed expected refusal]')}
 if(reconcileReads.length)errors.push('reconcile 403 body confirmations still in flight at failure time: '+reconcileReads.length);
 try{fs.writeFileSync(path.join(control,'phase1.done'),JSON.stringify({nonce,error:e.message}))}catch{}
 // A phase-2 failure must reach the Rust waiter verbatim too: by then the
 // phase1.done write above is already consumed, so the waiter sits on
 // phase2.done — publish the same error payload there (best effort).
 try{fs.writeFileSync(path.join(control,'phase2.done'),JSON.stringify({nonce,error:e.message}))}catch{}
 fs.writeFileSync(path.join(out,'approval-form-ui-failure.json'),JSON.stringify({message:e.message,stack:e.stack,checks,errors},null,2),{mode:0o600});
 console.error('FAIL:',e.message);process.exitCode=1;
}).finally(async()=>{if(browser)await browser.close()});
