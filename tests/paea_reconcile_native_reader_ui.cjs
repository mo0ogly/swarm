'use strict';
// PA-09 native POSITIVE browser witness (GLM editor delivery). ONE real
// Chromium run against the ALREADY-CONFIGURED native listener owned by the
// opt-in integration test
// reconcile_effects_native_binary_positive_reader_frontier_empty_ledger
// (rust/crates/swarm-web/tests/paea_reconcile.rs). The Rust test owns the
// fixture roots, the ELF spawn, the Trust attribution and the raw-output
// witness; this script is a CHILD of that test, spawned only when
// SWARM_PAEA_NATIVE_BROWSER_WITNESS=1, AFTER public attribution and BEFORE
// the SIGINT stop, and receives everything it needs over PRIVATE STDIN JSON
// (never argv, never logs):
//   { address, cookieName, cookieValue, readerToken, unknownToken,
//     expectedReport, expectedDigest, parentDir }   (all strings)
// Reuse contract — same real producers as tests/paea_effects_ui.cjs and
// tests/paea_reconcile_reader_ui.cjs:
//   - Puppeteer installed by the real path node_modules/puppeteer (commonjs),
//     Chrome binary CHROME_BIN || '/usr/bin/google-chrome' (the native
//     paea_effects_ui.cjs model), flags headless, timeout 0,
//     protocolTimeout 0, --no-sandbox, profile owned inside the run dir;
//   - Puppeteer timeouts disabled via the public API (timeout 0) — event
//     waits (waitForFunction, polling 'raf', timeout 0) are the only end
//     conditions; no global timer, no sleep, no arbitrary delay, no watchdog;
//   - fresh exclusive 0700 run directory (mkdtempSync) under the private
//     evidence parent provided by the Rust test — NEVER /tmp (operator rule),
//     and the parent must live inside the repository;
//   - incognito browser contexts: the FR refusal->positive->unknown->recovery
//     sequence shares ONE page/context (the purge after a displayed success
//     is proven on that same page, no fresh cookie context between the
//     transitions); the EN and keyboard phases get their own contexts; the
//     REAL generated session cookie is injected as the browser would receive
//     it (httpOnly, sameSite Strict) on the real address origin;
//   - exact diagnostics classification (paea_effects_ui.cjs `watch` model):
//     every >=400 response must be an EXPECTED typed refusal, pageerror and
//     requestfailed always abort, console errors only tolerated while a
//     refusal is armed on the reconcile path;
//   - mode discriminator — the VALIDATED stdin expectedReport (parsed once,
//     never re-invented) drives the render assertions: settlement
//     `uninitialized` replays the qualified EMPTY behavior byte-for-byte
//     (same sentences, same 14 check names, witness kind
//     `paea-native-reader-ui-v1`, no report_scope delta); settlement
//     `consistent` arms the NON-EMPTY variant (witness kind
//     `paea-native-reader-ui-nonempty-v1` + report_scope "nonempty" +
//     explicit nonempty check names for the mode-specific assertions): the
//     positive read must render the exact product sentences derived from the
//     validated model (effectCount 1, the REAL finding line
//     intent/effect/verdict from web/paea-effects.js journal() templates,
//     git still explicitly uninitialized), and the same-page unknown refusal
//     must purge the REAL previously-rendered identifiers before the
//     recovery re-renders the identical finding line;
//   - nonempty HORIZONTAL overflow guard (supervisor receipt of
//     glm-pa09-native-nonempty-ui-20261103T050631Z: long REAL digests
//     visibly clipped at the right edge of the EN 390 sombre positive
//     screenshot while the 14 functional checks passed — the earlier
//     overflow assertion only ran BEFORE the report was displayed): named
//     guards run POST-POSITIVE and POST-RECOVERY at the REAL boundaries
//     (fr/1280 and en/390), after the credential field is already purged,
//     measuring documentElement.scrollWidth vs innerWidth and each finding's
//     scrollWidth/clientWidth plus the HORIZONTAL left/right placement of
//     its boundingClientRect within its card and the viewport span; the
//     contract is HORIZONTAL ONLY — normal vertical scrolling is allowed
//     (an earlier version also compared top/bottom against the viewport
//     height, forbidding scrollable content: hidden test policy removed by
//     lot glm-pa09-horizontal-guard-20261103T054211Z); on any overflow a
//     fresh explicit screenshot is taken BEFORE the throw (field already
//     empty, no secret on it) and the NUMERIC metrics surface in the
//     assertion journal and the failure JSON (counts/lengths only —
//     identifier bytes never enter any message).
//     This guard is an additional assertion with a named trace only — never
//     a 15th check push, so the EMPTY qualified case and its 14-name recipe
//     contract stay byte-identical; it proves NOTHING about a fix and makes
//     nothing pass: until the product CSS fix, an overflow FAILS the run
//     with reproducible metrics (that red is the conservable evidence);
//   - issue-time request ledger, end-of-run scoping assertion: the ONLY
//     credentialed request is the reconcile GET with the exact Bearer shape;
//     NO other method is ever issued — no POST, no write from the browser;
//     the ledger stores an in-memory CLASSIFICATION of each Authorization
//     header (present / bearer / readerKnown / unknownKnown booleans): the
//     raw credential value is compared once and immediately discarded, it is
//     NEVER retained in any ledger, diagnostic or artifact, not even private
//     0600.
// Real browser assertions (the order's positive frontier):
//   - the protected effects page (session-gated asset route) loads from the
//     REAL ELF-served listener; the only HTTP method issued by the whole run
//     is GET — no POST, no write from the browser;
//   - bare session: typed 403 paea-reader-authentication-required status,
//     ALL report DOM purged;
//   - fixture reader token in the dedicated password field + re-read: 200
//     report — the two journal states rendered EXACTLY as the product renders
//     them (uninitialized sentence, findings empty), the technical raw JSON
//     EXACTLY JSON.stringify(JSON.parse(expectedReport),null,2), the secret
//     field purged, tokens absent from DOM and both web storages;
//   - unknown credential ON THE SAME PAGE that just displayed the report (no
//     fresh cookie context between the transitions): typed 403
//     authorization_required (this witness runs AFTER attribution with the
//     trust journal configured — unlike the fresh-fixture divergence
//     recorded in paea_effects_ui.cjs), ALL report DOM purged again on that
//     same page — the purge is proven where the report REALLY was displayed,
//     not on a fresh empty context — field erased;
//   - recovery with the known reader ON THE SAME PAGE: 200 exact again;
//   - FR/EN x etat/sombre x 1280/390: exactly TWO justified combinations
//     (fr/etat/1280 carries the full positive path; en/sombre/390 proves the
//     language/theme/mobile rendering of the same states);
//   - keyboard: first Tab reaches the skip link, Escape clears the credential
//     field; theme toggle really switches the dataset;
//   - no console error, no unclassified network, GET-only scoping.
// Named evidence (run dir 0700, files 0600): screenshots (main region, never
// secrets — fr positive, en positive captured TWICE: sombre BEFORE the theme
// toggle and etat AFTER it), assertion journal, request ledger, refusal
// ledger, diagnostics ledger, run metadata JSON. Secrets (cookie, tokens,
// report bytes) never
// reach argv and never reach stdout — the only stdout line is the final
// counts line — and NEVER reach any artifact either, not even the private
// 0600 ones: credential values and report bytes are compared in memory and
// only their classification is journaled (presence, Bearer shape,
// readerKnown/unknownKnown; report equality via its digest and lengths). A
// leak or mismatch diagnostic reports a COUNT and a CHANNEL (dom/ls/ss),
// never the leaked or differing bytes — no secret survives anywhere under
// the reserved evidence directory, and no post-leak sanitizer exists to
// stand in for the safe producer: the producer itself never emits values.
// Erasure assertions are length-based (assertFieldEmpty): node:assert never
// receives the field value, so its AssertionError cannot print the password;
// both JSON parses (stdin input, expectedReport) are wrapped in safe catches
// refusing with a generic category — a truncated input's SyntaxError excerpt
// (which would contain the cookie) and any raw input never reach stderr or
// artifacts.
// Exit: 0 = every assertion held; nonzero = first failure (the Rust test then
// panics; its unwinding conserves the native cleanup evidence).
// Usage (spawned by the Rust test only, cwd: repository root):
//   node tests/paea_reconcile_native_reader_ui.cjs   (STDIN JSON, no argv)
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const pup=require(path.join(process.cwd(),'node_modules','puppeteer'));
// STDIN JSON contract: read to EOF, parse, refuse anything unexpected. The
// parse is wrapped so a truncated/malformed stdin NEVER publishes an input
// excerpt (a raw SyntaxError message would embed the cookie): the refusal is
// a generic category raised BEFORE any browser launch, with no raw input and
// no exception message/stack. A stdin READ IO error is reported separately
// (it carries no input content).
let input;
try{
 input=JSON.parse(fs.readFileSync(0,'utf8'));
}catch(parseErr){
 if(parseErr&&parseErr.code==='EAGAIN'){
  throw Error('stdin read failed before any browser launch (no input content reported): read would block');
 }
 throw Error('stdin JSON parse failed before any browser launch: input rejected as '+
  (parseErr instanceof SyntaxError?'malformed JSON':'unreadable stdin JSON')+
  ' (generic category only — raw input, exception message and stack are never reported)');
}
for(const key of ['address','cookieName','cookieValue','parentDir']) {
 if(typeof input[key]!=='string'||!input[key])throw Error('stdin JSON must carry nonempty string '+key);
}
for(const key of ['readerToken','unknownToken','expectedReport','expectedDigest']) {
 if(typeof input[key]!=='string'||!input[key])throw Error('stdin JSON must carry nonempty string '+key);
}
if(!/^127\.0\.0\.1:[1-9][0-9]*$/.test(input.address))throw Error('stdin address must be a bare 127.0.0.1:<port> loopback');
if(!/^[A-Za-z0-9_-]{1,64}$/.test(input.cookieName))throw Error('stdin cookieName must be a bare token');
const origin='http://'+input.address;
const PAGE_PATH='/paea-effects.html';
const RECONCILE_PATH='/api/v1/paea/reconcile-effects';
// Evidence run dir: fresh exclusive 0700 under the Rust-provided parent
// (already 0700 and repository-internal; never /tmp by Rust-side contract —
// re-checked here).
if(path.resolve(input.parentDir)==='/tmp'||path.resolve(input.parentDir).startsWith('/tmp/'))throw Error('evidence parent must never be /tmp (operator rule)');
fs.mkdirSync(input.parentDir,{recursive:true,mode:0o700});
const runDir=fs.mkdtempSync(path.join(input.parentDir,'browser-witness-'),{mode:0o700});
const evPath=path.join(runDir,'assertions.jsonl');
/** Append one journal line; on failure, record then throw the first error.
 * Inputs: step label, ok flag, optional detail (JSON-serializable). Effects:
 * appends to assertions.jsonl. Errors: throws Error on !ok. */
function ev(step,ok,detail){
 fs.appendFileSync(evPath,JSON.stringify({utc:new Date().toISOString(),step,ok,detail:detail===undefined?null:detail})+'\n',{mode:0o600});
 if(!ok)throw Error('assertion failed: '+step+' — '+JSON.stringify(detail===undefined?null:detail));
}
// Diagnostics ledgers + abort wiring (same shape as paea_effects_ui.cjs).
const checks=[],refusals=[],requests=[],unexpected=[];
let expectRefusal=false,aborted=null;
const observedFailure=new Promise((resolve,reject)=>{aborted=reject});
// Exact expected-refusal scope: under a real session, ONLY the reconcile GET
// may 403, and only while a refusal phase is armed. `sessionConsoleOk`
// mirrors the product page's own console.error on 403s (same classification
// as paea_effects_ui.cjs).
const REFUSAL_PATHS=[RECONCILE_PATH];
const sessionRespOk=(m,p,s)=>expectRefusal&&m==='GET'&&REFUSAL_PATHS.includes(p)&&s===403;
const sessionConsoleOk=(p,t)=>expectRefusal&&REFUSAL_PATHS.includes(p)&&t.includes('403');
/** Classify one Authorization header IN MEMORY — the raw credential value is
 * compared here once and immediately discarded; only booleans are returned
 * and only booleans are ever stored (no secret value in any ledger,
 * diagnostic or artifact, not even the private 0600 ones). */
function classifyAuth(header){
 const present=typeof header==='string'&&header.length>0;
 return {
  present,
  bearer:present&&header.startsWith('Bearer '),
  readerKnown:header==='Bearer '+input.readerToken,
  unknownKnown:header==='Bearer '+input.unknownToken,
 };
}
/** Assert one captured request carries the EXPECTED credential kind (`reader`
 * or `unknown`), comparing in memory; the failure message carries only the
 * observed classification booleans — never any credential value. */
function assertAuth(req,kind,label){
 const c=classifyAuth(req.headers()&&req.headers().authorization);
 const ok=kind==='reader'?(c.present&&c.bearer&&c.readerKnown&&!c.unknownKnown)
  :kind==='unknown'?(c.present&&c.bearer&&c.unknownKnown&&!c.readerKnown)
  :false;
 assert.ok(ok,label+' — observed classification (booleans only): '+JSON.stringify(c));
 return c;
}
/** Compare two report texts IN MEMORY; on mismatch the message carries only
 * the observed/expected LENGTHS and the expected digest — never the
 * differing bytes (no report byte may enter any diagnostic or artifact). */
function assertReportEqual(actual,expected,label){
 assert.ok(actual===expected,label+' — report mismatch (compared in memory; observed length '+actual.length+', expected length '+expected.length+', expected digest '+input.expectedDigest+')');
}
/** Attach the exact diagnostics classification to one page (paea_effects_ui.cjs
 * `watch` model): every >=400 response must satisfy `sessionRespOk` or the run
 * aborts; pageerror/requestfailed always abort; console errors are tolerated
 * only while a refusal is armed on the reconcile path (the product page logs
 * its own 403); every request is ledgered at ISSUE time for the end-of-run
 * scoping assertion. Disables Puppeteer timeouts via the public API — the
 * assertions are the only end conditions. Inputs: a fresh Page. Returns it. */
function watch(page){
 page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 page.on('pageerror',e=>{unexpected.push({kind:'pageerror',message:e.message});aborted(Error('unexpected-native-browser-diagnostic: '+JSON.stringify(unexpected[unexpected.length-1])))});
 page.on('requestfailed',r=>{const d={kind:'requestfailed',path:new URL(r.url()).pathname};unexpected.push(d);aborted(Error('unexpected-native-browser-diagnostic: '+JSON.stringify(d)))});
 page.on('request',r=>{
  const p=new URL(r.url()).pathname;
  const auth=classifyAuth(r.headers()&&r.headers().authorization);
  // The raw header was classified above and is NOT retained: only the
  // classification booleans (or null when no credential was carried) enter
  // the ledger.
  requests.push({method:r.method(),path:p,authorization:auth.present?auth:null,armed:expectRefusal});
 });
 page.on('response',r=>{
  if(r.status()<400)return;
  const p=new URL(r.url()).pathname,m=r.request().method();
  refusals.push({method:m,path:p,status:r.status()});
  if(!sessionRespOk(m,p,r.status())){const d={kind:'http',method:m,path:p,status:r.status()};unexpected.push(d);aborted(Error('unexpected-native-browser-diagnostic: '+JSON.stringify(d)))}
 });
 page.on('console',m=>{
  if(m.type()!=='error')return;
  const loc=m.location()&&m.location().url?new URL(m.location().url).pathname:'';
  if(!sessionConsoleOk(loc,m.text())){const d={kind:'console',path:loc,message:m.text()};unexpected.push(d);aborted(Error('unexpected-native-browser-diagnostic: '+JSON.stringify(d)))}
 });
 return page;
}
/** Credentialed subset of the issue-time ledger (same shared predicate model
 * as paea_effects_ui.cjs): only entries carrying a classified Authorization
 * (booleans only — no credential value is stored anywhere). */
const credentialedReads=()=>requests.filter(r=>r.authorization);
/** Wait until #reconcile-status leaves 'loading' (rAF polling, timeout 0 —
 * 'loading' is never a result; an absent state is pending too) and return the
 * settled terminal state. Throws with the observed snapshot otherwise. */
async function settle(page){
 await page.waitForFunction(()=>{const s=document.getElementById('reconcile-status').dataset.state;return !!s&&s!=='loading'},{timeout:0,polling:'raf'});
 return page.$eval('#reconcile-status',e=>({state:e.dataset.state,text:e.textContent}));
}
/** Assert the settled reconcile status is the typed refusal `code` (error
 * state, code visible). Returns the settled state for the evidence. */
async function waitRefusal(page,code){
 const st=await settle(page);
 assert.equal(st.state,'error','reconcile refusal must render the explicit error state, got: '+st.state);
 assert.ok(st.text.includes(code),'typed refusal code must be visible, got: '+st.text);
 return st;
}
/** Assert a refusal purged ALL report DOM (container hidden, technical pre
 * empty — compared by LENGTH so no report byte can ever enter a diagnostic —
 * and both domain states empty, both findings lists empty). In the NON-EMPTY
 * mode (`identifiers` = the two real strings from the validated expected
 * model) they are additionally required ABSENT from the whole live DOM: the
 * scan runs in-page over arguments, and only a count may reach the failure
 * diagnostic (identifier values never enter any message). Throws with a
 * length/emptiness-only snapshot on any residue. */
async function assertPurged(page,label,identifiers){
 const dom=await page.evaluate(()=>({
  reportHidden:document.getElementById('reconcile-report').hidden,
  rawLength:document.getElementById('reconcile-report-raw').textContent.length,
  settlementState:document.getElementById('settlement-state').textContent,
  gitState:document.getElementById('git-state').textContent,
  settlementFindings:document.getElementById('settlement-findings').children.length,
  gitFindings:document.getElementById('git-findings').children.length
 }));
 assert.ok(dom.reportHidden===true&&dom.rawLength===0&&dom.settlementState===''&&dom.gitState===''&&dom.settlementFindings===0&&dom.gitFindings===0,label+': refusal must purge all report DOM, residue (lengths/emptiness only, never bytes): '+JSON.stringify(dom));
 if(identifiers){
  const residue=await page.evaluate(ids=>{
   const html=document.documentElement.innerHTML;
   return {intent:html.includes(ids.intent),digest:html.includes(ids.digest)};
  },identifiers);
  const count=(residue.intent?1:0)+(residue.digest?1:0);
  assert.equal(count,0,label+': previously rendered real identifiers must be purged, residue count: '+count);
 }
}
/** Assert the ephemeral credential field is REALLY empty WITHOUT ever handing
 * the value to node:assert (its AssertionError would print the ACTUAL
 * password): the comparison is boolean/length in memory and the diagnostic
 * carries only the observed LENGTH. Assertion strength unchanged: zero length
 * is still required. Inputs: page, failure label. Throws assert on residue. */
async function assertFieldEmpty(page,label){
 const len=await page.$eval('#reconcile-reader-credential',e=>e.value.length);
 assert.ok(len===0,label+': credential field must be empty (observed length '+len+', value never reported)');
}
/** Assert the reader form is REALLY served: exact FR/EN label for the
 * ephemeral password field, exact re-read command label, plain button. */
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
 * storages (no credential residue after erasure). The failure reports a leak
 * COUNT per CHANNEL (dom/ls/ss) — never which value leaked, never any byte
 * of it. */
async function credNotStored(page,label,...secrets){
 const snap=await page.evaluate(()=>({html:document.documentElement.innerHTML,ls:JSON.stringify(localStorage),ss:JSON.stringify(sessionStorage)}));
 const leaks={dom:0,ls:0,ss:0};
 for(const s of secrets){
  if(snap.html.includes(s))leaks.dom++;
  if(snap.ls.includes(s))leaks.ls++;
  if(snap.ss.includes(s))leaks.ss++;
 }
 const total=leaks.dom+leaks.ls+leaks.ss;
 assert.equal(total,0,label+': credentials must be absent from DOM and web storage, leak counts per channel (values never reported): '+JSON.stringify(leaks));
}
/** Launch the real Chrome with the native-driver flags (paea_effects_ui.cjs
 * model): no sandbox, Puppeteer timeouts 0, profile owned by the run dir. */
const launchBrowser=()=>pup.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(runDir,'browser-profile')});
/** In-page helpers receive the REAL identifiers as ARGUMENTS (`ids`) — no
 * bridge global, no page-context state: the two identifier strings from the
 * validated expected model cross each evaluate boundary as parameters, are
 * compared in-page, and only a boolean/li-text result returns to Node.
 * Find one journal's finding <li> holding BOTH real identifiers: scans the
 * list items' text and returns it, or null. The li text is only ever
 * compared in memory, never reported (identifier values never enter a
 * diagnostic). Inputs: page, DOM list id, the two real identifier strings. */
async function findFindingLi(page,listId,intent,digest){
 return page.$eval('#'+listId,(ul,ids)=>{
  for(const li of ul.children){
   const txt=li.textContent;
   if(txt.includes(ids.intent)&&txt.includes(ids.digest))return txt;
  }
  return null;
 },{intent,digest});
}
/** NONEMPTY HORIZONTAL overflow guard — named evidence, NOT a 15th recipe
 * check. supervisor receipt glm-pa09-native-nonempty-ui-20261103T050631Z: the
 * REAL long digests were visibly clipped at the right edge of the EN 390
 * sombre positive screenshot while the 14 functional checks passed; the
 * earlier overflow probe only ran BEFORE the report existed. This guard runs
 * POST-POSITIVE and POST-RECOVERY on the populated page, with the credential
 * field ALREADY purged (assertFieldEmpty held before it — no secret can
 * appear on the failure screenshot). Contract: HORIZONTAL ONLY — no
 * horizontal overflow of the populated report; normal VERTICAL scrolling is
 * allowed (a wrapped report is taller than the viewport by design; the
 * first guard version also compared top/bottom against the viewport height
 * and so forbade scrollable content — a hidden test policy removed by lot
 * glm-pa09-horizontal-guard-20261103T054211Z after the real 317 metrics
 * showed bottom 867.05 > height 844 with wrapping NORMAL, document 390 vs
 * 390, li 258 vs 258). All measurements stay NUMERIC:
 * documentElement.scrollWidth vs innerWidth (document horizontal
 * containment) and each finding's own scrollWidth/clientWidth plus the
 * HORIZONTAL placement of its boundingClientRect (left/right) contained
 * within its card section and the viewport span (with 1px tolerance for
 * sub-pixel rounding); the full rect is retained as evidence but only its
 * left/right enter the verdict. identifier BYTES never enter any message,
 * ledger or artifact: the finding rows are numbered by index only. On the
 * FIRST detected overflow: a fresh explicit screenshot is written BEFORE
 * the throw (private run dir, numeric metrics retained in the ev() line and
 * the failure JSON for reproduction). This assertion proves nothing beyond
 * the horizontal contract and makes nothing pass. Inputs: page, boundary
 * label (e.g. 'fr-1280-post-positive'). Throws with the numeric snapshot. */
async function assertNoOverflow(page,label){
 const shot=path.join(runDir,'overflow-'+label.replace(/[^a-z0-9-]+/gi,'-')+'.png');
 const m=await page.evaluate(()=>{
  const vw=window.innerWidth;
  const doc=document.documentElement;
  const viewportSpan={left:0,right:vw};
  const hContains=(o,i)=>i.left>=o.left-1&&i.right<=o.right+1;
  const rows=[...document.querySelectorAll('#settlement-findings li,#git-findings li')];
  const findings=rows.map((li,i)=>{
   const r=li.getBoundingClientRect();
   const card=li.closest('section').getBoundingClientRect();
   return {
    index:i,
    scrollWidth:li.scrollWidth,
    clientWidth:li.clientWidth,
    rect:{left:r.left,top:r.top,right:r.right,bottom:r.bottom},
    cardContained:hContains(card,r),
    viewportContained:hContains(viewportSpan,r)
   };
  });
  return {
   innerWidth:vw,
   scrollWidth:doc.scrollWidth,
   findingCount:rows.length,
   findings
  };
 });
 const dOver=m.scrollWidth-m.innerWidth;
 const over=findingsOverflow(m);
 const ok=dOver<=0&&over.length===0;
 if(!ok){
  // Screenshot FIRST (field already empty — no secret on it), throw AFTER:
  // the red must stay reproducible even if the screenshot itself fails (the
  // original metrics stay the thrown cause).
  let shotNote='written';
  try{await (await page.$('main')).screenshot({path:shot});}
  catch(e){shotNote='unavailable: '+e.message;}
  ev('horizontal overflow guard '+label,false,{scrollWidth:m.scrollWidth,innerWidth:m.innerWidth,documentDelta:dOver,overflowingFindingCount:over.length,overflowingFindings:over,screenshot:shotNote});
 }
 assert.ok(ok,label+' — horizontal overflow of the populated nonempty report (metrics only; vertical scrolling is normal): '+JSON.stringify({scrollWidth:m.scrollWidth,innerWidth:m.innerWidth,documentDelta:dOver,overflowingFindingCount:over.length,overflowingFindings:over,screenshot:shot}));
}
/** Overflow subset of one guard measurement: findings whose own content
 * scrollWidth exceeds their clientWidth OR whose rect escapes its card or
 * the viewport HORIZONTALLY (left/right; top/bottom never enter the
 * verdict) — numeric entries only, index-addressed. */
function findingsOverflow(m){
 return m.findings.filter(f=>f.scrollWidth>f.clientWidth||!f.cardContained||!f.viewportContained);
}

(async()=>{
 let browser=null,failed=null;
 try{
  browser=await launchBrowser();
  await Promise.race([(async()=>{
   // Same safe-parse rule as stdin: a SyntaxError here would embed an excerpt
   // of the expected report (private bytes) — refuse with a generic category.
   // The parsed report ALSO feeds the validated in-memory render model below:
   // the DOM assertions are parametrized from the REAL expected bytes (never
   // invented JSON) and the settlement state is the mode discriminator.
   let expectedRaw,expectedView;
   try{
    expectedView=JSON.parse(input.expectedReport);
    expectedRaw=JSON.stringify(expectedView,null,2);
   }catch(_){
    throw Error('stdin expectedReport is not valid JSON (generic category only — raw report bytes, exception message and stack are never reported)');
   }
   // Validated render model (in memory only): the settlement state
   // discriminates the EMPTY qualified behavior (kept byte-for-byte: same
   // assertions, same 14 check names, same witness kind) from the NON-EMPTY
   // variant (its own witness kind, report_scope metadata and explicit
   // nonempty check names for the mode-specific assertions). Any other
   // expected shape is refused with a generic category BEFORE any browser
   // phase — an invented expectation would pretend a render the product
   // never served. The derived sentences and finding lines below replicate
   // the REAL render contract of web/paea-effects.js journal() (its exact
   // t() templates, catalog-backed for EN) from the VALIDATED model values
   // only, interpolated like SwarmI18n.t (literal {key} substitution).
   if(!expectedView||typeof expectedView!=='object'||!expectedView.settlement||typeof expectedView.settlement!=='object'||!expectedView.git||typeof expectedView.git!=='object'||typeof expectedView.settlement.state!=='string'||typeof expectedView.git.state!=='string'){
    throw Error('stdin expectedReport is out of contract: both journal states must be explicit objects (generic category only — raw report bytes are never reported)');
   }
   const settlementState=expectedView.settlement.state,gitState=expectedView.git.state;
   if(settlementState!=='uninitialized'&&settlementState!=='consistent'){
    throw Error('stdin expectedReport settlement state has no render contract in this witness (uninitialized or consistent only — generic category)');
   }
   const nonempty=settlementState==='consistent';
   /** The two REAL identifiers as one page.evaluate argument object, or
    * null in the empty mode (no identifier assertion exists there). */
   const realIdentifiers=()=>nonempty?{intent:finding.intent_id,digest:finding.action_digest}:null;
   let settleFr='',settleEn='',findLineFr='',findLineEn='',finding=null;
   if(nonempty){
    const f=expectedView.settlement.findings,n=expectedView.settlement.effectCount??expectedView.settlement.effect_count;
    if(n!==1||!Array.isArray(f)||f.length!==1||typeof f[0].intent_id!=='string'||!f[0].intent_id||typeof f[0].action_digest!=='string'||!f[0].action_digest||f[0].verified!==true||(f[0].refusals??[]).length!==0||gitState!=='uninitialized'||expectedView.schema_version!==1){
     throw Error('stdin expectedReport consistent shape is out of the witnessed contract (schema_version 1, effectCount 1, exactly one verified refusal-free finding, git uninitialized — generic category only)');
    }
    finding=f[0];
    // Exact product render of the consistent settlement journal: FR source
    // template and EN catalog entry of web/paea-effects.js + web/i18n-en.js;
    // the finding line keeps the product's trailing structure (`{verdict}
    // {codes}` with an empty refusals join — the li text ends with a space).
    const ok=f.filter(x=>x.verified).length,ko=f.length-ok;
    settleFr=n+' effet(s) durable(s) rapproché(s) : '+ok+' correspondance(s) recalculée(s), '+ko+' écart(s) signalé(s).';
    settleEn=n+' durable effect(s) reconciled: '+ok+' recomputed correspondence(s), '+ko+' reported deviation(s).';
    const codes=(finding.refusals??[]).join(' ');
    findLineFr='intent '+finding.intent_id+' · effet '+finding.action_digest+' · correspondance recalculée '+codes;
    findLineEn='intent '+finding.intent_id+' · effect '+finding.action_digest+' · recomputed correspondence '+codes;
   }
   // Phase A — FR/etat/1280: full positive path on the REAL ELF-served page.
   {
    const context=await browser.createBrowserContext();
    const page=watch(await context.newPage());
    await context.setCookie({name:input.cookieName,value:input.cookieValue,url:origin,httpOnly:true,sameSite:'Strict'});
    await page.setViewport({width:1280,height:900});
    expectRefusal=true; // the page's initial no-credential read is the expected bare 403
    await page.goto(origin+PAGE_PATH+'?lang=fr&theme=etat',{waitUntil:'domcontentloaded'});
    await page.waitForSelector('#reconcile-reload',{visible:true,timeout:0});
    const bareSt=await waitRefusal(page,'paea-reader-authentication-required');
    expectRefusal=false;
    ev('fr bare refusal typed',true,{state:bareSt.state,code:'paea-reader-authentication-required'});
    checks.push('fr: bare reader refusal 403 paea-reader-authentication-required under a real session');
    await assertPurged(page,'bare refusal fr',realIdentifiers());
    checks.push('fr: bare refusal purges all report DOM');
    await checkReaderForm(page,'fr');
    ev('fr reader form served',true,null);
    // Positive read: fixture token in the dedicated field + REAL button.
    await page.type('#reconcile-reader-credential',input.readerToken);
    const reqP=page.waitForRequest(r=>new URL(r.url()).pathname===RECONCILE_PATH&&r.method()==='GET');
    await page.click('#reconcile-reload');
    const req=await reqP;
    assertAuth(req,'reader','fr positive re-read credential scope');
    const okSt=await settle(page);
    assert.equal(okSt.state,'success','positive read must settle success, got: '+okSt.state);
    assert.ok(okSt.text.includes('Rapprochement recalculé affiché pour les deux domaines réels'),'FR success message, got: '+okSt.text);
    // The rendered report REALLY matches the attributed report: both journal
    // states show the product's explicit uninitialized sentence with NO
    // findings line, and the technical raw is EXACTLY the expected JSON.
    const dom=await page.evaluate(()=>({
     reportHidden:document.getElementById('reconcile-report').hidden,
     raw:document.getElementById('reconcile-report-raw').textContent,
     settlementState:document.getElementById('settlement-state').textContent,
     gitState:document.getElementById('git-state').textContent,
     settlementFindings:document.getElementById('settlement-findings').children.length,
     gitFindings:document.getElementById('git-findings').children.length
    }));
    assert.equal(dom.reportHidden,false,'report container must be visible after the positive read');
    const UNINIT='Journal non initialisé : aucun effet rapproché (état explicite, pas un succès vide).';
    const expSettle=nonempty?settleFr:UNINIT;
    assert.equal(dom.settlementState,expSettle,'settlement state must render the exact expected sentence, got: '+dom.settlementState);
    assert.equal(dom.gitState,UNINIT,'git state must render the exact uninitialized sentence, got: '+dom.gitState);
    if(nonempty){
     // The REAL settlement finding line: the li holding BOTH real
     // identifiers must exist and carry the EXACT product-rendered text
     // derived from the validated model (template + verdict + empty refusals
     // join, trailing structure included) — not a counter alone.
     const li=await findFindingLi(page,'settlement-findings',finding.intent_id,finding.action_digest);
     assert.ok(li!==null,'the real settlement finding line (both real identifiers) must be rendered');
     assert.equal(li,findLineFr,'settlement finding line must be the exact product render, observed length differs: '+li.length+' vs '+findLineFr.length);
     assert.equal(await findFindingLi(page,'git-findings',finding.intent_id,finding.action_digest),null,'the explicitly uninitialized git journal must render no real-identifier finding');
    }else{
     assert.equal(dom.settlementFindings,0,'uninitialized settlement must show no finding');
     assert.equal(dom.gitFindings,0,'uninitialized git must show no finding');
    }
    assertReportEqual(dom.raw,expectedRaw,'fr positive technical raw must equal the attributed report');
    ev('fr positive 200 report exact',true,{digest_sha256:input.expectedDigest});
    if(nonempty){
     checks.push('fr: fixture-token re-read 200 — settlement rendered consistent ('+settleFr.length+' chars) with the real finding line (intent+effect+verdict), git uninitialized, raw JSON exactly the attributed report');
    }else{
     checks.push('fr: fixture-token re-read 200 — both journals rendered uninitialized (exact product sentence, no finding), raw JSON exactly the attributed report');
    }
    await assertFieldEmpty(page,'positive read');
    await credNotStored(page,'positive read',input.readerToken,input.cookieValue);
    checks.push('fr: secret field purged; reader token and session cookie absent from DOM and web storage');
    // NONEMPTY visual guard POST-POSITIVE at the REAL fr/1280 boundary,
    // field already purged (never a 15th recipe check — named trace only).
    if(nonempty){await assertNoOverflow(page,'fr-1280-post-positive');}
    await (await page.$('main')).screenshot({path:path.join(runDir,'paea-effects-fr-positive.png')});
    // Keyboard on the SAME page (no reload): Escape clears the field.
    await page.type('#reconcile-reader-credential',input.readerToken);
    await page.keyboard.press('Escape');
    await assertFieldEmpty(page,'Escape');
    ev('fr keyboard escape clears field',true,null);
    checks.push('fr: Escape clears the ephemeral credential field');
    // Unknown credential ON THE SAME PAGE that just displayed the report —
    // no fresh cookie context between the transitions: the purge below is
    // proven where the report REALLY was displayed, not on a fresh empty
    // context. The Escape test above left the field EMPTY, so typing the
    // unknown token starts from a clean field.
    const before=credentialedReads().length;
    await page.type('#reconcile-reader-credential',input.unknownToken);
    expectRefusal=true; // the re-read's 403 is the expected identity refusal
    const unknownReqP=page.waitForRequest(r=>new URL(r.url()).pathname===RECONCILE_PATH&&r.method()==='GET');
    await page.click('#reconcile-reload');
    const unknownReq=await unknownReqP;
    assertAuth(unknownReq,'unknown','fr unknown re-read credential scope (same page)');
    const st=await waitRefusal(page,'authorization_required');
    expectRefusal=false;
    ev('unknown credential typed 403 authorization_required (same page)',true,{state:st.state});
    checks.push('unknown fixture token: typed 403 authorization_required really served ON THE SAME PAGE that displayed the report (purge proven where it matters)');
    assert.equal(credentialedReads().length,before+1,'exactly one credentialed re-read must have been issued');
    await assertPurged(page,'unknown-credential refusal (same page, after a displayed success)',realIdentifiers());
    await assertFieldEmpty(page,'unknown-credential refusal');
    await credNotStored(page,'unknown-credential refusal',input.unknownToken,input.cookieValue);
    assert.equal(await page.$eval('#reconcile-reload',e=>e.disabled),false,'re-read command must stay available after the refusal');
    if(nonempty){
     checks.push('unknown-credential refusal: report purged on the SAME page INCLUDING the previously rendered real identifiers, field erased, re-read command available');
    }else{
     checks.push('unknown-credential refusal: report purged on the SAME page, field erased, re-read command available');
    }
    // Recovery with the known reader ON THE SAME PAGE: 200, exact attributed
    // bytes — the read channel is intact after the unknown refusal.
    await page.type('#reconcile-reader-credential',input.readerToken);
    expectRefusal=false;
    const recReqP=page.waitForRequest(r=>new URL(r.url()).pathname===RECONCILE_PATH&&r.method()==='GET');
    await page.click('#reconcile-reload');
    const recReq=await recReqP;
    assertAuth(recReq,'reader','fr recovery re-read credential scope (same page)');
    const st2=await settle(page);
    assert.equal(st2.state,'success','recovery read must settle success, got: '+st2.state);
    const dom2=await page.evaluate(()=>({raw:document.getElementById('reconcile-report-raw').textContent}));
    assertReportEqual(dom2.raw,expectedRaw,'fr recovery raw must equal the attributed report');
    if(nonempty){
     // Recovery restores the REAL rendered report on that same page: the
     // identical finding line (both real identifiers, exact product text)
     // must be back after the purge.
     const li2=await findFindingLi(page,'settlement-findings',finding.intent_id,finding.action_digest);
     assert.ok(li2!==null,'recovery must re-render the real settlement finding line');
     assert.equal(li2,findLineFr,'recovered settlement finding line must be the exact product render, observed length differs: '+li2.length+' vs '+findLineFr.length);
    }
    ev('fr recovery 200 exact (same page)',true,{digest_sha256:input.expectedDigest});
    if(nonempty){
     checks.push('fr: recovery re-read 200 with the exact attributed report on the SAME page — the real finding line is re-rendered identically (channel intact after refusals)');
    }else{
     checks.push('fr: recovery re-read 200 with the exact attributed report on the SAME page (channel intact after refusals)');
    }
    // NONEMPTY visual guard POST-RECOVERY (same real fr/1280 page, populated
    // again after the purge) — named trace only, never a recipe count.
    if(nonempty){await assertNoOverflow(page,'fr-1280-post-recovery');}
    await context.close();
   }
   // Phase D — EN/sombre/390: language, theme and mobile rendering of the
   // same states (bare refusal + positive read), theme toggle included.
   {
    const context=await browser.createBrowserContext();
    const page=watch(await context.newPage());
    await context.setCookie({name:input.cookieName,value:input.cookieValue,url:origin,httpOnly:true,sameSite:'Strict'});
    await page.setViewport({width:390,height:844});
    expectRefusal=true;
    await page.goto(origin+PAGE_PATH+'?lang=en&theme=sombre',{waitUntil:'domcontentloaded'});
    await page.waitForSelector('#reconcile-reload',{visible:true,timeout:0});
    await waitRefusal(page,'paea-reader-authentication-required');
    expectRefusal=false;
    assert.equal(await page.$eval('html',e=>e.dataset.theme),'sombre','EN theme from URL');
    assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),'390px must not overflow horizontally');
    await checkReaderForm(page,'en');
    await page.type('#reconcile-reader-credential',input.readerToken);
    const reqP=page.waitForRequest(r=>new URL(r.url()).pathname===RECONCILE_PATH&&r.method()==='GET');
    await page.click('#reconcile-reload');
    const req=await reqP;
    assertAuth(req,'reader','en positive re-read credential scope');
    const st=await settle(page);
    assert.equal(st.state,'success','EN positive read must settle success, got: '+st.state);
    assert.ok(st.text.includes('Recomputed reconciliation shown for both real domains'),'EN success message, got: '+st.text);
    const dom=await page.evaluate(()=>({
     raw:document.getElementById('reconcile-report-raw').textContent,
     settlementState:document.getElementById('settlement-state').textContent,
     gitState:document.getElementById('git-state').textContent,
     settlementFindings:document.getElementById('settlement-findings').children.length,
     gitFindings:document.getElementById('git-findings').children.length
    }));
    const UNINIT='Uninitialized journal: no effect reconciled (explicit state, not an empty success).';
    const expSettle=nonempty?settleEn:UNINIT;
    assert.equal(dom.settlementState,expSettle,'EN settlement state must render the exact expected sentence, got: '+dom.settlementState);
    assert.equal(dom.gitState,UNINIT,'EN git state must render the exact uninitialized sentence, got: '+dom.gitState);
    if(nonempty){
     // EN rendering of the REAL settlement finding line: exact product text
     // (catalog EN verdict "recomputed correspondence") with BOTH real
     // identifiers, derived from the validated model.
     const li=await findFindingLi(page,'settlement-findings',finding.intent_id,finding.action_digest);
     assert.ok(li!==null,'the real EN settlement finding line (both real identifiers) must be rendered');
     assert.equal(li,findLineEn,'EN settlement finding line must be the exact product render, observed length differs: '+li.length+' vs '+findLineEn.length);
     assert.equal(await findFindingLi(page,'git-findings',finding.intent_id,finding.action_digest),null,'the explicitly uninitialized EN git journal must render no real-identifier finding');
    }else{
     assert.equal(dom.settlementFindings,0,'EN uninitialized settlement must show no finding');
     assert.equal(dom.gitFindings,0,'EN uninitialized git must show no finding');
    }
    assertReportEqual(dom.raw,expectedRaw,'EN technical raw must equal the attributed report');
    // NONEMPTY visual guard POST-POSITIVE at the REAL en/390 boundary (the
    // supervisor-inspected clipped screenshot was exactly here), field
    // purged first inside the branch so the EMPTY path stays byte-identical
    // — named trace only, never a recipe count.
    if(nonempty){
     await assertFieldEmpty(page,'en positive read');
     await assertNoOverflow(page,'en-390-post-positive');
    }
    // Sombre rendering captured BEFORE the toggle (receipt 2026-11-03 review:
    // the dark dataset behavior was asserted but never rendered on screen).
    await (await page.$('main')).screenshot({path:path.join(runDir,'paea-effects-en-positive-sombre.png')});
    // Theme toggle switches the dataset in place (no page reload): sombre -> etat.
    await page.click('#paea-theme');
    await page.waitForFunction(()=>document.documentElement.dataset.theme==='etat',{timeout:0,polling:'raf'});
    ev('en theme toggle renders etat',true,null);
    checks.push('en: theme toggle really switches sombre->etat dataset');
    await (await page.$('main')).screenshot({path:path.join(runDir,'paea-effects-en-positive.png')});
    await context.close();
   }
   // Phase E — keyboard skip link on a fresh load (its own bare 403 armed).
   {
    const context=await browser.createBrowserContext();
    const page=watch(await context.newPage());
    await context.setCookie({name:input.cookieName,value:input.cookieValue,url:origin,httpOnly:true,sameSite:'Strict'});
    await page.setViewport({width:1280,height:900});
    expectRefusal=true;
    await page.goto(origin+PAGE_PATH+'?lang=fr&theme=etat',{waitUntil:'domcontentloaded'});
    await page.waitForSelector('#reconcile-reload',{visible:true,timeout:0});
    await waitRefusal(page,'paea-reader-authentication-required');
    expectRefusal=false;
    await page.keyboard.press('Tab');
    const focused=await page.evaluate(()=>document.activeElement.className+(document.activeElement.id?':'+document.activeElement.id:''));
    assert.ok(focused.includes('paea-skip'),'first Tab must reach the skip link, got: '+focused);
    const target=await page.$eval('#paea-theme',e=>e.getBoundingClientRect().height);
    assert.ok(target>=44,'theme button must be >=44px, got '+target);
    ev('fr keyboard skip link + theme control 44px',true,null);
    checks.push('fr: keyboard skip link first, theme control >=44px');
    await context.close();
   }
   // Whole-run credential scoping: every Authorization header belongs to the
   // reconcile GET; the credentialed requests are exactly the four re-reads
   // (fr positive, unknown, fr recovery — all on the SAME fr page — plus en
   // positive) with exact Bearer classifications — only the unknown one was
   // armed (the three successes were issued un-armed and settled 200); the
   // four reconcile 403s are the three bare page-load reads (fr same-page
   // phase, en, keyboard) plus the armed unknown re-read; no other method
   // was ever issued from the browser. The ledger holds classification
   // booleans ONLY — every assertion below compares booleans, never values.
   {
    assert.ok(requests.length>=1,'the run must have observed at least one request');
    const offPath=requests.filter(r=>r.path!==RECONCILE_PATH&&r.authorization);
    assert.equal(offPath.length,0,'Authorization observed off the reconcile path (count): '+offPath.length);
    const methods=[...new Set(requests.map(r=>r.method))];
    assert.deepEqual(methods,['GET'],'only GET requests may be issued from the browser, got: '+JSON.stringify(methods));
    const cred=credentialedReads();
    assert.equal(cred.length,4,'exactly four credentialed reads (fr positive, unknown, fr recovery on the same page, en positive), got: '+cred.length);
    assert.ok(cred.every(r=>r.authorization.bearer),'every credentialed read must be Bearer-scoped');
    assert.ok(cred.filter(r=>r.authorization.readerKnown&&!r.authorization.unknownKnown).length===3,'the fixture token must be used exactly three times (fr positive, fr recovery, en positive), got: '+cred.filter(r=>r.authorization.readerKnown&&!r.authorization.unknownKnown).length);
    assert.ok(cred.filter(r=>r.authorization.unknownKnown&&!r.authorization.readerKnown).length===1,'the unknown token must be used exactly once, got: '+cred.filter(r=>r.authorization.unknownKnown&&!r.authorization.readerKnown).length);
    const armedCred=cred.filter(r=>r.armed);
    assert.equal(armedCred.length,1,'exactly the unknown re-read must have been armed, got: '+armedCred.length);
    assert.ok(armedCred.length===1&&armedCred[0].authorization.unknownKnown&&!armedCred[0].authorization.readerKnown,'the armed credentialed read must be the unknown-token one');
    const reconcile403=refusals.filter(x=>x.method==='GET'&&x.path===RECONCILE_PATH&&x.status===403);
    assert.equal(reconcile403.length,4,'exactly four reconcile 403s (three bare page-load reads + the armed unknown re-read), got: '+JSON.stringify(refusals));
    const reconcileGets=requests.filter(r=>r.method==='GET'&&r.path===RECONCILE_PATH);
    assert.equal(reconcileGets.length,7,'exactly seven reconcile GETs (three bare page-load reads + four credentialed re-reads), got: '+reconcileGets.length);
    checks.push('Authorization scoped to the reconcile GET only; 4 Bearer reads (3 fixture + 1 armed unknown), 4 reconcile 403s (3 bare + 1 armed), 7 reconcile GETs, GET-only; ledger holds classification booleans only');
   }
   assert.equal(unexpected.length,0,'no unexpected browser diagnostic allowed, got: '+JSON.stringify(unexpected));
   checks.push('no unexpected browser diagnostic across all phases');
   // Witness artifact: the EMPTY qualified mode keeps the byte-identical
   // shape (kind paea-native-reader-ui-v1, the 14 recipe-verified names); the
   // NON-EMPTY mode emits its own explicit kind + report_scope metadata and
   // its own nonempty-variant check names (never the empty names).
   fs.writeFileSync(path.join(runDir,'browser-witness.json'),JSON.stringify({schema_version:1,kind:nonempty?'paea-native-reader-ui-nonempty-v1':'paea-native-reader-ui-v1',report_scope:nonempty?'nonempty':'empty',checks,unexpected_diagnostics:unexpected,expected_refusals:refusals,request_ledger:requests,at:new Date().toISOString()},null,2)+'\n',{mode:0o600});
   console.log(JSON.stringify({checks:checks.length,unexpected:unexpected.length,refusals:refusals.length,scope:nonempty?'nonempty':'empty'}));
  })(),observedFailure]);
 }catch(e){
  failed=failed||e;
  console.error('FAIL:',e.message);
  try{
   fs.writeFileSync(path.join(runDir,'browser-witness-failure.json'),JSON.stringify({message:e.message,stack:e.stack,checks,unexpected,refusals},null,2),{mode:0o600});
  }catch(writeErr){
   // No silent catch: the original cause stays the exit condition, and the
   // secondary evidence-write failure is reported too (safe messages only —
   // assertion texts carry no credential or report bytes by construction).
   console.error('FAIL: cannot write browser-witness-failure.json: '+writeErr.message+' (original cause retained, run exits nonzero)');
  }
 }finally{
  try{if(browser)await browser.close()}catch(closeErr){
   failed=failed||closeErr;
   console.error('FAIL: browser.close reported: '+closeErr.message);
  }
  try{fs.chmodSync(runDir,0o700)}catch(chmodErr){
   failed=failed||chmodErr;
   console.error('FAIL: cannot restore 0700 on the run directory: '+chmodErr.message);
  }
  if(failed)process.exitCode=1;
 }
})();
