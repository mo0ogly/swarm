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
const checks=[],errors=[];let expected=0;
// Exact refusal classification (correction of the earlier blanket filters):
// the response listener classifies by EXACT URL+status — the fixture service
// configures no settlement ledger root, so the guarded reconcile read refuses
// with the typed settings-paea-ledger-root-not-configured cause which maps to
// HTTP 500 for the reconcile operation (service.rs maps it to 501 only for
// the settings-ledger-root operation). Any other status on that URL (404, 403,
// …) stays unexpected, and any 500 on any other URL stays unexpected too.
// The browser console error of a failed fetch carries NO URL (Chrome omits
// it), so the console listener cannot classify by URL: it consumes exactly
// one allowance per already-classified expected response with the same
// status, and any console error without a matching pending allowance stays
// unexpected. No blanket ignore of any status class remains.
const pendingConsole=[];
// Armed just before the deliberate post-stale refusal probe and never reset:
// both the response and its browser-console duplicate must then classify as
// the expected denial instead of unexpected diagnostics. Any other status on
// the approval POST (a 500, for instance) stays unexpected.
let staleRefusalArmed=false;
let browser;
(async()=>{
 browser=await pup.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
 const page=await browser.newPage();
 page.setDefaultTimeout(20000);
 page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{
  if(m.type()!=='error')return;
  const match=m.text().match(/status of (\d{3})/);
  if(match){const i=pendingConsole.indexOf(match[1]);if(i>=0){pendingConsole.splice(i,1);return}}
  errors.push(m.text());
 });
 page.on('response',r=>{
  if(r.status()<400)return;
  if(r.url().endsWith('/api/v1/paea/reconcile-effects')){
   if(r.status()===500){expected+=1;pendingConsole.push('500');return}
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
 const staleWitness=await body('#approval-witness');
 assert.ok(staleWitness.text.includes('non'),'stale witness must report revision-current = non');
 await (await page.$('#approval-detail')).screenshot({path:path.join(out,'approval-stale-fr.png')});
 await open('en','sombre',390);
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 await page.type('#approval-reader-credential',server.reader_token);
 await page.click('#approval-reload');
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 const staleEn=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(staleEn.state,'error');
 assert.ok(/Stale approval/i.test(staleEn.text),'EN stale message, got: '+staleEn.text);
 await (await page.$('#approval-detail')).screenshot({path:path.join(out,'approval-stale-en.png')});
 checks.push('stale: real Trust revision change (policy renewed through real producers) → visible error FR/EN behind the reader frontier, no confirmation solicited, no normalization');
 // Post-stale refusal cross-check: the same channel now refuses (the held
 // enrolled authority is bound to the consumed Trust revision — engine
 // contract proven by the server-route test). A browser console duplicate of
 // this already-classified response carries no URL; the listener classifies
 // only by URL, so the empty-body POST refusal is classified here as expected.
 // The browser page itself must not have consumed twice: exactly one phase-1
 // send happened.
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
 // The Rust test's owned protocol waits for phase2.done to run its own
 // shutdown; publish it (atomic write) only after every phase-2 assertion
 // has held, so a successful harness always releases the waiter.
 write('phase2.done',{status:'ok'});
 fs.writeFileSync(path.join(out,'approval-form-ui.json'),JSON.stringify({schema_version:1,kind:'paea-approval-form-ui-v1',checks,unexpected_diagnostics:errors,expected_denials:expected,server:{request_digest:server.request_digest,action_id:server.action_id,resource:server.resource,trust_revision_fresh:server.trust_revision,trust_revision_stale:phase2.trust_revision},at:new Date().toISOString()},null,2)+'\n');
 console.log(JSON.stringify({checks:checks.length,unexpected_diagnostics:errors.length}));
})().catch(e=>{
 try{fs.writeFileSync(path.join(control,'phase1.done'),JSON.stringify({nonce,error:e.message}))}catch{}
 // A phase-2 failure must reach the Rust waiter verbatim too: by then the
 // phase1.done write above is already consumed, so the waiter sits on
 // phase2.done — publish the same error payload there (best effort).
 try{fs.writeFileSync(path.join(control,'phase2.done'),JSON.stringify({nonce,error:e.message}))}catch{}
 fs.writeFileSync(path.join(out,'approval-form-ui-failure.json'),JSON.stringify({message:e.message,stack:e.stack,checks,errors},null,2),{mode:0o600});
 console.error('FAIL:',e.message);process.exitCode=1;
}).finally(async()=>{if(browser)await browser.close()});
