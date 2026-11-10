'use strict';
// PA-09 native browser qualification of /paea-effects.html on the REAL native
// web server (provided candidate binary, isolated 0700 fixture root, real
// session credential). Replaces the stale 500-era driver: under a legitimate
// fixture session the reconciliation endpoint is guarded by the reader policy
// (rust/crates/swarm-engine/src/root_store/reader_guard.rs) and serves typed
// 403 refusals, never a 500:
//   - bare session (no reader token): 403 `paea-reader-authentication-required`
//     BEFORE any ledger/settings IO (reader_identity_precheck, :121-126);
//   - unknown reader token on a FRESH fixture (no trust journal; enrollment is
//     forbidden here): the really served identity refusal is 403
//     `paea-reader-policy-required` (reader_identity_state: absent journal at
//     :86 -> AbsentJournal -> :124-126). The literal `authorization_required`
//     cited by the order requires an EXISTING trust journal and is unreachable
//     without forbidden enrollment — the driver asserts the really served code
//     and records this divergence in the evidence instead of inventing a state.
// Order contract honored: no positive authorized read is ever issued, no
// identity/enrollment/account/role/UID is fabricated, refusals are asserted by
// exact method+path+status (no global 403 ignoring), readiness is EVENT-DRIVEN
// (first stdout JSON line, child error/close reject — no polling/sleep/15 s
// timeouts), Puppeteer timeouts are disabled through the public API (timeout
// 0), the fixture root is fresh via ./fixture_area.cjs (never /tmp), the
// output argument must be NEW (reuse forbidden), and cleanup covers ONLY the
// owned browser and server without an infinite wait. Wrapper/manifest/archive
// integration is a LATER Codex step (out of this delivery); usage is direct:
//   node tests/paea_effects_ui.cjs <binary> <fresh-output-dir>   (cwd: repo root)
// No hardcoded binary hash: the provided binary and the actually running
// /proc/<pid>/exe are compared at run time and recorded as evidence.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),crypto=require('node:crypto');
const pup=require(path.join(process.cwd(),'node_modules','puppeteer'));
const fixtureArea=require('./fixture_area.cjs');
const binary=path.resolve(process.argv[2]||''),out=path.resolve(process.argv[3]||'');
if(!process.argv[2]||!process.argv[3])throw Error('usage: node tests/paea_effects_ui.cjs <binary> <fresh-output-dir> (cwd: repository root)');
if(!fs.existsSync(binary))throw Error('provided binary not found: '+binary);
if(fs.existsSync(out))throw Error('output directory must be FRESH — reuse is forbidden: '+out);
fs.mkdirSync(out,{recursive:true,mode:0o700});
// Operator rule: never create project fixtures under /tmp (tmpfs, wiped on
// reboot) — the shared fixture-area allocator refuses /tmp itself.
const root=fixtureArea.fixtureRoot('paea-effects-ui-');
const sha=v=>crypto.createHash('sha256').update(v).digest('hex');
const checks=[],errors=[],refusals=[],authedRequests=[],serverStdout=[],serverStderr=[];
let expected=0,expectRefusal=false;
let browser=null,server=null,closed=null,failed=null,stopping=false,abort=null;
let token='',address='',binarySha='';
const observedFailure=new Promise((resolve,reject)=>{abort=reject});
// Unexpected browser/page diagnostic: retained, then rejects the pending run —
// only real terminal states or this abort end a wait, never a deadline.
function browserFailure(value){errors.push(value);abort(Error('unexpected-native-browser-diagnostic: '+JSON.stringify(value)))}
// Exact expected-refusal scope: under a session, ONLY the reconcile GET may
// 403, and only while a refusal phase is armed. Anonymous: the page asset and
// the API both refuse 403 with the plain `web-access-refused` code.
const REFUSAL_PATHS=['/api/v1/paea/reconcile-effects'];
const ANON_PATHS=['/paea-effects.html','/api/v1/paea/git-call'];
const sessionRespOk=(m,p,s)=>expectRefusal&&m==='GET'&&REFUSAL_PATHS.includes(p)&&s===403;
const sessionConsoleOk=(p,t)=>expectRefusal&&REFUSAL_PATHS.includes(p)&&t.includes('403');
const anonRespOk=(m,p,s)=>m==='GET'&&ANON_PATHS.includes(p)&&s===403;
const anonConsoleOk=(p,t)=>ANON_PATHS.includes(p)&&t.includes('403');
/** Start the provided binary against the fresh fixture root: run the public
 * init, spawn the JSON web listener, await readiness EVENT-DRIVEN (the first
 * stdout JSON line carries the bound address; child 'error'/'close' reject —
 * no polling, no sleep), prove the running process really is the provided
 * binary via /proc/<pid>/exe, then read the really-created session credential
 * with O_NOFOLLOW + ownership checks.
 * Inputs: module `binary`, `root`. Outputs: sets `address`, `token`,
 * `binarySha`, `server`, `closed`.
 * Errors: init/spawn/fs failures propagate verbatim; a listener exit before
 * announcement rejects readiness; an unexpected later exit rejects the run
 * through browserFailure. Effects: owns `server` until the caller's cleanup. */
async function start(){
 cp.execFileSync(binary,['--root',root,'--json','init']);
 server=cp.spawn(binary,['--root',root,'web','127.0.0.1:0','--json'],{stdio:['ignore','pipe','pipe']});
 closed=new Promise(resolve=>server.once('close',(code,signal)=>{resolve({code,signal});if(!stopping)browserFailure({kind:'owned-server-exit',code,signal})}));
 server.stdout.on('data',b=>serverStdout.push(b));
 server.stderr.on('data',b=>serverStderr.push(b));
 const line=await new Promise((resolve,reject)=>{
  server.once('error',reject);
  server.once('close',(code,signal)=>reject(Error('native listener exited before announcement (code='+code+' signal='+signal+')')));
  let buffer='';
  server.stdout.on('data',b=>{buffer+=b.toString();const n=buffer.indexOf('\n');if(n>=0)resolve(buffer.slice(0,n))});
 });
 const bound=JSON.parse(line);
 assert(/^127\.0\.0\.1:[1-9][0-9]*$/.test(bound.address),'native announcement must carry a bound 127.0.0.1 address, got: '+line);
 address=bound.address;
 binarySha=sha(fs.readFileSync(binary));
 assert.equal(sha(fs.readFileSync('/proc/'+server.pid+'/exe')),binarySha,'the running process must be the provided binary');
 const key=path.join(root,'.swarm','web-session-'+sha(address).slice(0,16));
 const fd=fs.openSync(key,fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW);
 try{const info=fs.fstatSync(fd);assert(info.isFile()&&info.uid===process.getuid()&&(info.mode&0o077)===0,'session credential file must be a private owned regular file');token=fs.readFileSync(fd,'utf8').trim()}finally{fs.closeSync(fd)}
 assert(/^session-[A-Za-z0-9_-]{24}$/.test(token),'bind must have created a real session token');
}
/** Attach the exact diagnostics classification to one page: every >=400
 * response must satisfy `respOk(method,path,status)` or the run aborts;
 * console errors must satisfy `consoleOk(path,text)`; pageerror and
 * requestfailed always abort. Every reconcile-path or Authorization-bearing
 * request is ledgered at ISSUE time (path, Bearer or null, armed flag) for
 * the end-of-run scoping check — immune to a later un-arming race.
 * Disables Puppeteer timeouts through the public API: the assertions are
 * the only end conditions. Returns the page. */
function watch(page,respOk,consoleOk){
 page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 page.on('pageerror',e=>browserFailure({kind:'pageerror',message:e.message}));
 page.on('requestfailed',r=>browserFailure({kind:'requestfailed',path:new URL(r.url()).pathname}));
 // The request event is the ONLY path where `expectRefusal` is read for the
 // credential-scoping ledger: captured at issue time, immune to a later
 // un-arming race between the response and the runner.
 page.on('request',r=>{
  const a=r.headers()&&r.headers().authorization;
  const p=new URL(r.url()).pathname;
  if(a||REFUSAL_PATHS.includes(p))authedRequests.push({path:p,authorization:a||null,armed:expectRefusal});
 });
 page.on('response',r=>{
  if(r.status()<400)return;
  const p=new URL(r.url()).pathname,m=r.request().method();
  refusals.push({method:m,path:p,status:r.status()});
  if(respOk(m,p,r.status()))expected++;else browserFailure({kind:'http',method:m,path:p,status:r.status()});
 });
 page.on('console',m=>{
  if(m.type()!=='error')return;
  const loc=m.location()&&m.location().url?new URL(m.location().url).pathname:'';
  if(!consoleOk(loc,m.text()))browserFailure({kind:'console',path:loc,message:m.text()});
 });
 return page;
}
/** Credentialed subset of the issue-time request ledger — the ONE shared
 * predicate consumed BOTH by the phase-local baseline and by the final
 * scoping assertion, so the two counts can never mix predicates again (the
 * received FAIL compared a filtered count against a whole-ledger baseline
 * and failed 1 !== 6 with 5 bare reads in the ledger).
 * Input: the ledger entries pushed by watch() ({path,authorization,armed}).
 * Output: only entries carrying an Authorization header (all on the reconcile
 * GET; the scoping assertion separately proves path and Bearer shape).
 * Errors: none — pure filter, no side effect. */
const credentialedReads=()=>authedRequests.filter(r=>r.authorization);
/** Wait until #reconcile-status leaves 'loading' (rAF polling of the real DOM,
 * timeout 0 — 'loading' is never a result) and assert the settled terminal
 * state is the explicit error carrying the typed refusal `code`.
 * Returns {state,text}. Throws with the observed state/text when the settled
 * state is not the expected typed refusal. */
async function waitRefusal(page,code){
 // An absent state (before the page's first cycle arms 'loading') is pending
 // too — same rule as the component classifier classifyStatus().
 await page.waitForFunction(()=>{const s=document.getElementById('reconcile-status').dataset.state;return !!s&&s!=='loading'},{timeout:0,polling:'raf'});
 const st=await page.$eval('#reconcile-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(st.state,'error','reconcile refusal must render the explicit error state, got: '+st.state);
 assert.ok(st.text.includes(code),'typed refusal code must be visible, got: '+st.text);
 return st;
}
/** Assert a refusal purged ALL report DOM (container hidden, technical pre and
 * both domain states empty, both findings lists empty) — no report data may
 * survive a denial. Throws with the observed DOM snapshot on any residue. */
async function assertPurged(page,label){
 const dom=await page.evaluate(()=>({
  reportHidden:document.getElementById('reconcile-report').hidden,
  raw:document.getElementById('reconcile-report-raw').textContent,
  settlementState:document.getElementById('settlement-state').textContent,
  gitState:document.getElementById('git-state').textContent,
  settlementFindings:document.getElementById('settlement-findings').children.length,
  gitFindings:document.getElementById('git-findings').children.length
 }));
 assert.ok(dom.reportHidden===true&&dom.raw===''&&dom.settlementState===''&&dom.gitState===''&&dom.settlementFindings===0&&dom.gitFindings===0,label+': refusal must purge all report DOM, got: '+JSON.stringify(dom));
}
/** Assert the reader form is REALLY served by the running binary: exact FR/EN
 * label for the ephemeral password field, exact re-read command label, plain
 * button (no form submit). Throws on any label/shape deviation. */
async function checkReaderForm(page,lang){
 const label=await page.$eval('label[for="reconcile-reader-credential"]',e=>e.textContent.trim());
 const cmd=await page.$eval('#reconcile-reload',e=>e.textContent.trim());
 const shape=await page.$eval('#reconcile-reader-credential',e=>({type:e.type,autocomplete:e.getAttribute('autocomplete')}));
 const btn=await page.$eval('#reconcile-reload',e=>e.type);
 if(lang==='fr'){assert.equal(label,'Jeton lecteur (lecture du rapport de rapprochement classifié)','FR reader label, got: '+label);assert.equal(cmd,'Relire le rapport','FR re-read label, got: '+cmd)}
 else{assert.equal(label,'Reader token (read of the classified reconciliation report)','EN reader label, got: '+label);assert.equal(cmd,'Re-read the report','EN re-read label, got: '+cmd)}
 assert.deepEqual(shape,{type:'password',autocomplete:'off'},'reader field must be an ephemeral password input, got: '+JSON.stringify(shape));
 assert.equal(btn,'button','re-read command must be a plain button, got: '+btn);
 checks.push(lang+': reader credential field (password, autocomplete=off) and re-read button really served');
}
/** Assert the given secrets are absent from the live DOM snapshot and both web
 * storages (no credential residue after erasure). Throws listing any leak. */
async function credNotStored(page,label,...secrets){
 const snap=await page.evaluate(()=>({html:document.documentElement.innerHTML,ls:JSON.stringify(localStorage),ss:JSON.stringify(sessionStorage)}));
 const leak=secrets.filter(s=>snap.html.includes(s)||snap.ls.includes(s)||snap.ss.includes(s));
 assert.equal(leak.length,0,label+': credentials must be absent from DOM and web storage, leaked: '+JSON.stringify(leak));
}
(async()=>{
 try{
  await start();
  const origin='http://'+address;
  browser=await pup.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(out,'browser-profile')});
  await Promise.race([(async()=>{
   // Matrix FR/état/1280 and EN/sombre/390: legitimate session cookie (the
   // really-created fixture token, set as the browser would), explicit empty
   // states, bare typed 403 refusal with purged DOM, reader form served,
   // theme, overflow, keyboard, screenshot. No positive read is ever issued.
   for(const [lang,theme,width] of [['fr','etat',1280],['en','sombre',390]]){
    const context=await browser.createBrowserContext();
    const page=watch(await context.newPage(),sessionRespOk,sessionConsoleOk);
    await context.setCookie({name:'swarm_session_'+sha(address).slice(0,12),value:token,url:origin,httpOnly:true,sameSite:'Strict'});
    await page.setViewport({width,height:900});
    expectRefusal=true; // the page's initial no-credential reconcile read is the expected bare 403
    await page.goto(origin+'/paea-effects.html?lang='+lang+'&theme='+theme,{waitUntil:'domcontentloaded'});
    await page.waitForSelector('#reconcile-reload',{visible:true,timeout:0});
    await waitRefusal(page,'paea-reader-authentication-required');
    expectRefusal=false;
    checks.push(lang+': bare reader refusal 403 paea-reader-authentication-required visible under a legitimate session');
    await assertPurged(page,'bare refusal '+lang);
    checks.push(lang+': bare refusal purges all report DOM (no report data shown)');
    await page.waitForFunction(()=>document.getElementById('preview-status').dataset.state!=='loading',{timeout:0,polling:'raf'});
    const prev=await page.$eval('#preview-status',e=>({state:e.dataset.state,text:e.textContent}));
    assert.equal(prev.state,'empty','preview without bound source must be an explicit empty state, got: '+prev.state);
    if(lang==='fr')assert.ok(prev.text.includes('Aperçu explicitement indisponible'),'FR unavailable message, got: '+prev.text);
    else assert.ok(/Preview explicitly unavailable/i.test(prev.text),'EN unavailable message, got: '+prev.text);
    const detail=await page.$eval('#preview-detail',e=>({hidden:e.hidden,buttons:e.querySelectorAll('button').length}));
    assert.ok(detail.hidden&&detail.buttons===0,'preview detail stays hidden with no consent control, got: '+JSON.stringify(detail));
    checks.push(lang+': explicit preview-unavailable state, no consent control (no fake approval)');
    await page.waitForFunction(()=>document.getElementById('gitcall-status').dataset.state!=='loading',{timeout:0,polling:'raf'});
    const git=await page.$eval('#gitcall-status',e=>({state:e.dataset.state,text:e.textContent}));
    assert.equal(git.state,'empty','unbound git-call must be an explicit empty state, got: '+git.state);
    if(lang==='fr')assert.ok(git.text.includes('Aucun appel Git préparé'),'FR unbound message, got: '+git.text);
    else assert.ok(/No prepared Git call/i.test(git.text),'EN unbound message, got: '+git.text);
    checks.push(lang+': unbound git-call explicit empty state');
    await checkReaderForm(page,lang);
    assert.equal(await page.$eval('html',e=>e.dataset.theme),theme,lang+' theme from URL');
    assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),lang+' '+width+'px must not overflow horizontally');
    checks.push(lang+': theme '+theme+' applied, no horizontal overflow at '+width+'px');
    // Keyboard on a fresh load (its own bare 403, armed until settled).
    expectRefusal=true;
    await page.goto(origin+'/paea-effects.html?lang='+lang+'&theme='+theme,{waitUntil:'domcontentloaded'});
    await page.waitForSelector('#reconcile-reload',{visible:true,timeout:0});
    await waitRefusal(page,'paea-reader-authentication-required');
    expectRefusal=false;
    await page.keyboard.press('Tab');
    const focused=await page.evaluate(()=>document.activeElement.className+(document.activeElement.id?':'+document.activeElement.id:''));
    assert.ok(focused.includes('paea-skip'),'first Tab must reach the skip link, got: '+focused);
    const target=await page.$eval('#paea-theme',e=>e.getBoundingClientRect().height);
    assert.ok(target>=44,'theme button must be ≥44px, got '+target);
    checks.push(lang+': keyboard skip link first and theme control ≥44px');
    await (await page.$('main')).screenshot({path:path.join(out,'paea-effects-'+lang+'.png')});
    await context.close();
   }
   // Unknown-credential phase: a clearly synthetic token (never enrolled, no
   // identity fabricated) typed into the REAL served field, re-read via the
   // REAL button; the served identity refusal is asserted with its exact
   // typed code, the report DOM is purged, the field is erased and the
   // recovery command stays available. No positive read, ever.
   {
    const context=await browser.createBrowserContext();
    const page=watch(await context.newPage(),sessionRespOk,sessionConsoleOk);
    await context.setCookie({name:'swarm_session_'+sha(address).slice(0,12),value:token,url:origin,httpOnly:true,sameSite:'Strict'});
    await page.setViewport({width:1280,height:900});
    expectRefusal=true;
    await page.goto(origin+'/paea-effects.html?lang=fr&theme=etat',{waitUntil:'domcontentloaded'});
    await page.waitForSelector('#reconcile-reload',{visible:true,timeout:0});
    await waitRefusal(page,'paea-reader-authentication-required'); // settle the bare refusal first
    expectRefusal=false;
    const UNKNOWN='effects-ui-unknown-0123456789abcdef';
    const before=credentialedReads().length; // credentialed-count baseline, SAME predicate as the assertion below
    await page.type('#reconcile-reader-credential',UNKNOWN);
    expectRefusal=true; // the re-read's 403 is the expected identity refusal
    const reqP=page.waitForRequest(r=>new URL(r.url()).pathname==='/api/v1/paea/reconcile-effects'&&r.method()==='GET');
    await page.click('#reconcile-reload');
    const req=await reqP;
    assert.equal(req.headers().authorization,'Bearer '+UNKNOWN,'the re-read must carry the typed credential as Bearer');
    const unknownSt=await waitRefusal(page,'paea-reader-policy-required'); // really served identity code (divergence documented in the evidence)
    expectRefusal=false;
    // The only credentialed request of the run is the one armed refusal just
    // observed (issue-time arming ledger, not a response-time race): the
    // credentialed count advanced by exactly one — same predicate as the
    // baseline above; the 5 bare reads in the ledger never enter this count.
    assert.equal(credentialedReads().length,before+1,'exactly one credentialed re-read must have been issued');
    checks.push('unknown fixture token: typed 403 identity refusal really served — '+unknownSt.text.trim()+' — on a fresh journal-less fixture (authorization_required would require an existing trust journal: reader_guard.rs:86,:124-126)');
    await assertPurged(page,'unknown-credential refusal');
    assert.equal(await page.$eval('#reconcile-reader-credential',e=>e.value),'','credential field must be erased after the refusal');
    await credNotStored(page,'unknown-credential refusal',UNKNOWN);
    assert.equal(await page.$eval('#reconcile-reload',e=>e.disabled),false,'re-read command must stay available after the refusal (recovery path)');
    checks.push('unknown-credential refusal: report purged, field erased, re-read command available');
    await context.close();
   }
   // Anonymous phase: no cookie at all — the page asset and the API both
   // refuse 403 with the exact `web-access-refused` code, never a silent
   // empty success.
   {
    const anon=await browser.createBrowserContext();
    const anonPage=watch(await anon.newPage(),anonRespOk,anonConsoleOk);
    await anonPage.setViewport({width:1024,height:800});
    const resp=await anonPage.goto(origin+'/paea-effects.html',{waitUntil:'domcontentloaded'});
    assert.equal(resp.status(),403,'anonymous page asset must refuse (session-protected)');
    const api=await anonPage.evaluate(async()=>{const r=await fetch('/api/v1/paea/git-call',{cache:'no-store'});return {status:r.status,text:await r.text()}});
    assert.equal(api.status,403,'API must refuse without a session');
    assert.ok(api.text.includes('web-access-refused'),'exact anonymous refusal code visible, got: '+api.text.slice(0,200));
    checks.push('without session: page asset and API refuse 403 web-access-refused');
    await anon.close();
   }
   // Credential scoping (whole run): every Authorization header belongs to
   // the reconcile GET, the only credentialed one carries the unknown
   // fixture token as Bearer, and the armed re-read received its 403
   // (nothing armed-but-unserved slipped). Same shared predicate
   // credentialedReads() as the phase-local baseline.
   assert.ok(authedRequests.length>=1,'the run must have observed at least one reconcile request');
   assert.ok(authedRequests.every(r=>r.path==='/api/v1/paea/reconcile-effects'),'reconcile request observed off-path, got: '+JSON.stringify(authedRequests));
   const credentialed=credentialedReads();
   assert.equal(credentialed.length,1,'exactly one credentialed request in the run, got: '+JSON.stringify(authedRequests));
   assert.ok(credentialed[0].authorization.startsWith('Bearer '),'the credentialed request must be Bearer-scoped, got: '+credentialed[0].authorization);
   assert.ok(credentialed[0].armed,'the credentialed request must be the armed refused one');
   assert.ok(refusals.some(x=>x.method==='GET'&&x.path==='/api/v1/paea/reconcile-effects'&&x.status===403),'the armed re-read must have received its 403, got: '+JSON.stringify(refusals));
   checks.push('Authorization header scoped to the reconcile GET only ('+refusals.length+' classified 403(s), 1 credentialed re-read)');
   assert.equal(errors.length,0,'no unexpected browser diagnostic allowed, got: '+JSON.stringify(errors));
   checks.push('no unexpected browser diagnostic across all phases');
  })(),observedFailure]);
  fs.writeFileSync(path.join(out,'paea-effects-ui.json'),JSON.stringify({schema_version:1,kind:'paea-effects-ui-v1',checks,unexpected_diagnostics:errors,expected_denials:expected,observed_refusals:refusals,binary,binary_sha256:binarySha,fixture_root:root,refusal_contract:{bare:'paea-reader-authentication-required',unknown_credential_on_fresh_fixture:'paea-reader-policy-required',divergence_note:'order cited authorization_required; that code requires an existing trust journal (reader_guard.rs:86,124-126) and is unreachable without enrollment, which this driver must not perform'},at:new Date().toISOString()},null,2)+'\n');
  console.log(JSON.stringify({checks:checks.length,unexpected_diagnostics:errors.length,expected_denials:expected}));
 }catch(e){
  failed=failed||e;
  console.error('FAIL:',e.message);
  try{fs.writeFileSync(path.join(out,'paea-effects-ui-failure.json'),JSON.stringify({message:e.message,stack:e.stack,checks,errors,refusals,binary,binary_sha256:binarySha,fixture_root:root},null,2),{mode:0o600})}catch(_e){}
 }finally{
  try{if(browser)await browser.close()}catch(e){failed=failed||e}
  if(server){stopping=true;server.kill('SIGINT');const stopped=await closed;if(!failed&&stopped&&stopped.code!==0)failed=Error('owned native fixture did not stop cleanly (code='+stopped.code+' signal='+stopped.signal+')')}
  try{fs.writeFileSync(path.join(out,'server.stdout'),Buffer.concat(serverStdout),{mode:0o600});fs.writeFileSync(path.join(out,'server.stderr'),Buffer.concat(serverStderr),{mode:0o600})}catch(e){failed=failed||e}
  if(failed)process.exitCode=1;
 }
})();
