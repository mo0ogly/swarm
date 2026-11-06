'use strict';
/** Behavioral guard for the access-capability consumer contract (B8).
 *
 * Purpose — execute the real sources `web/native-authority-admin.js`,
 * `web/cockpit-authority.js` and `web/native-runtime.js` inside `node:vm`
 * sandboxes and assert the contract delivered in
 * `glm-capability-errors-code-20261103T234000Z`:
 *   - owner-key capability ⇒ resolved mode 'none', card hidden, exactly one
 *     capability fetch and zero daemon (`/api/v1/authority/*`) fetches;
 *   - authority capability ⇒ the first `GET …/me` is offered (call counted,
 *     then refused — no account, UID, enrollment or acceptance is fabricated);
 *   - capability HTTP refusal / network reject / JSON parse failure and the
 *     same classes on `GET …/me` resolve mode 'error' with the status and the
 *     original cause preserved on the structured `error`, while the DOM,
 *     console and `reason` carry only stable codes and the existing English
 *     translations — never the raw failure;
 *   - an absent/unknown capability mode resolves a visible 'error', never a
 *     silent 'none';
 *   - an unexpected mount rejection at BOTH consumers (`cockpit-authority.js`
 *     and `native-runtime.js`) is never folded into a hidden success: the
 *     card is shown with the safe translated message and stable code, the
 *     cause stays retained on the error, no sentinel leaks;
 *   - explicit recovery on a fresh host (as after a reload) with a valid
 *     owner-key capability performs zero automatic retry and zero parasitic
 *     `/me`;
 *   - a dialog-submit refusal carrying an unknown daemon code renders the
 *     generic refusal message and safe code only (shared renderer), the
 *     original error retained unmutated for diagnostics, with explicit
 *     recovery on resubmit.
 *
 * Inputs — the four source files read from the repository at run time; the
 * real generated `web/i18n-en.js` (loaded in the sandbox) and four targeted
 * keys of `locales/en.json` (read directly, no catalog pagination) for the
 * English-translation assertions and their catalog/generated parity.
 *
 * Method and doubles — unit scope with explicit local doubles: a recorded
 * fetch mock (routes + call log), minimal DOM element doubles, a recording
 * console. Microtask draining only (no timers, no watchdog, no retries in the
 * harness). The `PRIVATE_*` error messages are unit fixtures standing for an
 * operator-private failure detail; they are never live data.
 *
 * Corrected contract — three assertions were RED at the Codex run of
 * 2026-11-04 (exit 1, 84 PASS / 3 FAIL, `supervisor-guard-run/` and
 * `supervisor-red-reception.json`: `reason=PRIVATE_ME_CAUSE` rendered/logged
 * raw, /me parse `cause=undefined`). The FIX lot
 * `glm-capability-safe-error-fix-20261104T001200Z` corrected the sources;
 * the three assertions stay contract obligations, now expected green:
 *   - `me_network_reject_reason_is_stable_code` — the transport failure is
 *     wrapped under the stable code `authority-me-unreachable` (original
 *     error unmutated on `.cause`), never the raw message;
 *   - `me_network_reject_no_private_in_dom_console_reason` — no sentinel
 *     leak in DOM, console or `reason`;
 *   - `me_parse_reject_cause_identity` — the JSON decode failure keeps the
 *     original cause (stable `authority-response-unreadable` wrapper, the
 *     original object never mutated).
 * The former `me_network_reject_error_identity_original` assertion (outer
 * error identical to the original) contradicted the stable-wrapper contract
 * and was legitimately corrected after the recorded red run: the original
 * error must be reachable by identity inside the retained `cause` chain
 * (cycle-detected walk, no ceiling) while the outer code stays stable. The
 * two unexpected-rejection scenarios exercise a RAW private mount rejection
 * (the rejecting double rejects with the private sentinel itself, not a
 * stable wrapper) at both consumers: card visible, safe stable code, no
 * leak, and the original error reachable on the stable wrapper's `cause`
 * chain (asserted through a recording `Error` passthrough in each sandbox).
 *
 * Renderer lot — `glm-authority-renderer-safe-code-20261104T005700Z`, control
 * route repaired in `glm-dialog-guard-control-route-20261104T012700Z` (the
 * first version declared the fictitious `/me` DTO but never served it; the
 * unmatched fetch made mount resolve in visible error mode, which the
 * card-visible check alone mistook for a positive witness). Scenario: the
 * SHARED refusal renderer is exercised on the real dialog-submit path
 * (own-password dialog over POST /me/password). The fictitious `/me` DTO
 * really answers the first daemon read and the positive preconditions
 * (authority mode with administer=false, exactly one GET
 * `/api/v1/authority/me`, own-password button in the nav) are asserted
 * BEFORE the refusal is injected. An
 * unknown daemon code (`PRIVATE_UNKNOWN_DAEMON_DETAIL` sentinel — the
 * supervisor discriminant that motivated the lot) must render ONLY the
 * generic refusal message and the safe generic code, never the raw code;
 * the original error stays unmutated with its status for diagnostics
 * (observed through a recording `Error` passthrough added to the admin
 * sandbox) and never reaches DOM, console or a reason code; explicit
 * recovery — submit button re-enabled, refusal flipped to success, real
 * resubmit — completes with the translated success notice. The `/me`
 * document and the submitted values are strictly fictitious unit fixtures
 * (no UID, enrollment, session or acceptance). All pre-existing assertions
 * are unchanged obligations.
 *
 * Termination — completion is observed, never guessed. The admin and cockpit
 * scenarios await the actual promise `mount()`/the captured handler returns.
 * The runtime scenarios go through the source's REAL offered lifecycle:
 * with `readyState:'loading'` the source itself registers its global `load`
 * listener; the harness captures that exact handler (asserting it was
 * registered once, `{once:true}`), triggers it like the browser would and
 * awaits the actual promise `start()` returns. A tracked-promise barrier
 * (`track` on every mock fetch, body read and stub rejection, settled in
 * waves until the tracked graph is quiescent) only drains the remaining
 * detached jobs AFTER that awaited real completion — it is never the
 * completion proof for the product's own awaits, which a graph-empty
 * quiescence cannot establish (the 2026-11-04 harness red: barrier returned
 * before the runtime's async start had scheduled any fetch). A product
 * defect that scheduled jobs forever would keep the graph growing — a
 * diagnosable hang, never a masked pass; no timer, no round cap, no retry
 * exists in the harness.
 *
 * Errors — any harness error is caught at the bottom, printed with its cause
 * and fails the process (exit 1). Named `PASS`/`FAIL` lines are the record.
 *
 * Limits — no backend, no browser, no network, no human Authority login, no
 * owner-key browser integration (future Codex work), no acceptance of the
 * product; sentinel values are fixtures. Run: `node tests/authority_capability_consumer_test.cjs`.
 * Authored UNEXECUTED by GLM; Codex runs it for real after handoff. */
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const ROOT=path.join(__dirname,'..');
const read=rel=>fs.readFileSync(path.join(ROOT,rel),'utf8');
const SRC_ADMIN=read('web/native-authority-admin.js');
const SRC_COCKPIT=read('web/cockpit-authority.js');
const SRC_RUNTIME=read('web/native-runtime.js');
const SRC_EN=read('web/i18n-en.js');
const CATALOG=JSON.parse(read('locales/en.json'));

// The four diagnostic keys, copied verbatim from the messages map and the
// generic fallback of web/native-authority-admin.js (U+2019 apostrophes).
const KEY_CAP_UNREACHABLE='Capacité d’accès indisponible : le service natif n’a pas répondu.';
const KEY_CAP_REFUSED='Lecture de la capacité d’accès refusée par le service natif.';
const KEY_CAP_INVALID='Capacité d’accès illisible : document inattendu.';
const KEY_GENERIC_REFUSED='Refus du démon d’autorité.';
const KEY_GENERIC_REFUSAL_CODE='authority-refused'; // GENERIC_REFUSAL_CODE of web/native-authority-admin.js
const DIAGNOSTIC_KEYS=[KEY_CAP_UNREACHABLE,KEY_CAP_REFUSED,KEY_CAP_INVALID,KEY_GENERIC_REFUSED];

// Fixture-only sentinels (never live data): each stands for an
// operator-private detail that must stay on `error.cause` and never reach
// DOM, console or a `reason` code.
const SENT_CAP_NET=Error('PRIVATE_CAP_CAUSE');
const SENT_CAP_PARSE=Error('PRIVATE_PARSE_CAUSE');
const SENT_ME_NET=Error('PRIVATE_ME_CAUSE');
const SENT_ME_PARSE=Error('PRIVATE_ME_PARSE');
const SENT_MOUNT=Error('PRIVATE_MOUNT_CAUSE');

let pass=0,fail=0;
const STABLE_CODE=/^[a-z][a-z0-9-]*$/;
// Every failure is a red: after the FIX lot the three formerly-expected reds
// are plain contract obligations; no assertion is labeled ignorable.
const check=(name,ok,detail)=>{
 if(ok){pass++;console.log('PASS',name)}
 else{fail++;console.error('FAIL',name,detail??'')}
};

/** Cycle-detected walk of an error's `cause` chain (no depth ceiling): true
 * when the exact object `target` is reachable from `error`. Asserts that the
 * original error object survives stable wrapping unmutated and reachable. */
const causeChainHas=(error,target)=>{const seen=new WeakSet();
 for(let e=error;e&&typeof e==='object';){
  if(e===target)return true;
  if(seen.has(e))return false;
  seen.add(e);e=e.cause}
 return false};
/*
 * Observed barrier for detached double jobs — replaces the former
 * `settle(rounds=200)` microtask ceiling flagged by the supervisor review (a
 * fixed cap masks a completion defect as a pass). `track(p)` registers every
 * promise the doubles create (mock fetch, response reads, module stubs,
 * waiters); `barrier()` awaits the tracked set, but settlement callbacks run
 * BEFORE the awaiting continuation resumes, so any promise a settlement
 * schedules (chained fetch, then-handler, mount continuation) is already
 * tracked — the loop is re-entered only when the graph actually grew.
 * Termination of the tracked graph is observed, never a counted loop. The
 * barrier is a drain AFTER the scenario's awaited real completion (returned
 * promise / captured lifecycle handler); it is never the completion proof
 * for the product's own untracked awaits — a graph-empty quiescence cannot
 * establish those (2026-11-04 harness red). A product defect scheduling jobs
 * forever keeps growing the graph and surfaces as a diagnosable hang of the
 * guard, not a masked green.
 */
const tracked=new Set();
const track=p=>{tracked.add(p);return p};
const barrier=async()=>{
  for(let wave=0;;wave++){
    if(tracked.size===0)return wave;
    const pending=[...tracked];tracked.clear();
    await Promise.allSettled(pending);
  }
};

const domText=el=>(el?.textContent??'')+(el?.children??[]).map(domText).join('');

/** Minimal explicit DOM element double (textContent/className/children plus
 * the methods the executed paths touch; unexercised members are no-ops so an
 * unexpected path fails loudly through its own assertions, not here). */
const makeElementFactory=()=>{const all=[];const create=tag=>{const e={
 tag,children:[],textContent:'',className:'',hidden:false,dataset:{},attributes:{},listeners:{},
 append(...c){e.children.push(...c)},
 replaceChildren(...c){e.children.splice(0,e.children.length,...c)},
 insertBefore(n){e.children.push(n)},
 setAttribute(k,v){e.attributes[k]=v},getAttribute:k=>e.attributes[k],
 addEventListener(type,fn){(e.listeners[type]??=[]).push(fn)},
 remove(){},close(){},focus(){},select(){},scrollIntoView(){},showModal(){},
 querySelector(){return null},querySelectorAll(){return []}};
 all.push(e);return e};return {create,all}};

/** Recorded fetch mock: routes match by URL substring; every call is logged
 * with its URL and options. An unmatched URL rejects loudly (no silent
 * success). Route replies may return a response-like object or reject. The
 * promise of every call is tracked so the observed barrier can await the
 * REAL pending jobs (no microtask-count ceiling). */
const makeFetchMock=()=>{const log=[];let routes=[];
 const fetch=(url,opts)=>{log.push({url:String(url),opts});const route=routes.find(r=>String(url).includes(r.name));
  const p=route?Promise.resolve().then(route.reply):Promise.reject(Error('guard-unexpected-fetch:'+url));
  return track(p)};
 return {fetch,log,
  setRoutes:v=>{routes=v},count:name=>log.filter(e=>e.url.includes(name)).length,
  reset(){log.length=0},urls:()=>log.map(e=>e.url)}};
const jsonRes=(body,{status=200}={})=>({ok:status>=200&&status<300,status,
 headers:{get:k=>k==='Content-Type'?'application/json':null},
 json:()=>track(Promise.resolve(body)),text:()=>track(Promise.resolve(JSON.stringify(body)))});
const parseRejectRes=sentinel=>({ok:true,status:200,
 headers:{get:k=>k==='Content-Type'?'application/json':null},
 json:()=>track(Promise.reject(sentinel)),text:()=>track(Promise.reject(sentinel))});
const CAP='/api/v1/native-capabilities',AUTH='/api/v1/authority/',SESSION='/api/v1/native-session';

/** Recording console double for the sandboxed sources. */
const makeConsoleRec=()=>{const rec={errors:[],logs:[]};
 rec.console={error:(...a)=>rec.errors.push(a),log:(...a)=>rec.logs.push(a),warn:(...a)=>rec.errors.push(a)};
 rec.text=()=>JSON.stringify([...rec.errors,...rec.logs]);return rec};

const leakScan=(els,rec,reason)=>{const parts=[reason??''];
 for(const e of els.all)parts.push(e.textContent);
 parts.push(rec.text());return parts.join('\n')};

/** Load `web/i18n-en.js` in a sandbox and return its English table. */
function englishTable(){const ctx={};vm.createContext(ctx);
 vm.runInContext(SRC_EN,ctx,{filename:'web/i18n-en.js'});return ctx.SwarmEnglish}

/** Sandbox for the shared Admin module: real i18n table, real admin source,
 * explicit document/fetch/console doubles. `Error` inside the sandbox is a
 * recording passthrough (same prototype, native `{cause}` handling) so an
 * error the view throws or wraps — and its preserved `status`/`reason` —
 * stays observable from the harness without changing behavior.
 * Returns the exported mount. */
function makeAdminCtx(mock,rec){const els=makeElementFactory();
 const errorSpy=[];
 const RecordingError=function(...args){const e=new Error(...args);errorSpy.push(e);return e};
 RecordingError.prototype=Error.prototype;
 const document={createElement:els.create,
  documentElement:{lang:'fr',dataset:{}},body:els.create('body'),
  getElementById:()=>null,querySelector:()=>null,querySelectorAll:()=>[],addEventListener(){}};
 const ctx={console:rec.console,document,fetch:mock.fetch,URL,Error:RecordingError};
 vm.createContext(ctx);
 vm.runInContext(SRC_EN,ctx,{filename:'web/i18n-en.js'});
 ctx.SwarmI18n={t:s=>ctx.SwarmEnglish?.[s]??s,language:'fr'};
 vm.runInContext(SRC_ADMIN,ctx,{filename:'web/native-authority-admin.js'});
 const tEN=s=>ctx.SwarmEnglish?.[s]??s;
 return {ctx,els,mount:ctx.SwarmAuthorityAdmin.mount,tEN,errorSpy}}

/** Sandbox for `web/cockpit-authority.js`: captures the DOMContentLoaded
 * handler for a manual awaited invocation; the admin card host starts
 * hidden so "made visible / not hidden" assertions are discriminating.
 * `Error` inside the sandbox is a recording passthrough (same prototype,
 * native `{cause}` handling) so the consumer's stable wrapper — and the
 * original error it keeps on `.cause` — stays observable from the harness. */
function makeCockpitCtx(mock,rec,authority){const els=makeElementFactory();
 const errorSpy=[];
 const RecordingError=function(...args){const e=new Error(...args);errorSpy.push(e);return e};
 RecordingError.prototype=Error.prototype;
 const host=els.create('div');host.hidden=true;
 const entry={hidden:false,dataset:{view:'authority'}};
 const dcl=[];
 const document={createElement:els.create,
  documentElement:{lang:'fr',dataset:{}},body:els.create('body'),
  getElementById:id=>id==='native-authority-admin'?host:null,
  querySelector:sel=>sel==='#tabs [data-view="authority"]'?entry:null,
  querySelectorAll:()=>[],addEventListener(type,fn){if(type==='DOMContentLoaded')dcl.push(fn)}};
 const ctx={console:rec.console,document,fetch:mock.fetch,URL,Error:RecordingError};
 vm.createContext(ctx);
 vm.runInContext(SRC_EN,ctx,{filename:'web/i18n-en.js'});
 ctx.SwarmI18n={t:s=>ctx.SwarmEnglish?.[s]??s,language:'fr'};
 if(authority==='real')vm.runInContext(SRC_ADMIN,ctx,{filename:'web/native-authority-admin.js'});
 else if(authority==='reject')ctx.SwarmAuthorityAdmin={mount:()=>track(Promise.reject(SENT_MOUNT))};
 vm.runInContext(SRC_COCKPIT,ctx,{filename:'web/cockpit-authority.js'});
 if(dcl.length!==1)throw Error('guard: expected exactly one DOMContentLoaded handler, got '+dcl.length);
 return {ctx,els,host,entry,errorSpy,run:async()=>dcl[0]()}}

/** Sandbox for `web/native-runtime.js`: `readyState==='loading'` drives the
 * source through its REAL offered lifecycle — the global `load` listener the
 * source registers itself (`addEventListener('load',start,{once:true})`) is
 * captured, and the scenario triggers it exactly like the browser would, then
 * awaits the actual promise `start()` returns. The session fetch and the
 * other Admin modules are explicit stubs; `authority` selects the real admin
 * module or a mount that rejects unexpectedly. `Error` is a recording
 * passthrough (same prototype, native `{cause}` handling) so the runtime's
 * stable wrapper and its retained `.cause` stay observable from the harness. */
function makeRuntimeCtx(mock,rec,authority){const els=makeElementFactory();const byId={};
 const errorSpy=[];
 const RecordingError=function(...args){const e=new Error(...args);errorSpy.push(e);return e};
 RecordingError.prototype=Error.prototype;
 const document={createElement:els.create,
  documentElement:{lang:'fr',dataset:{}},body:els.create('body'),readyState:'loading',
  getElementById:id=>byId[id]??(byId[id]=els.create('div')),
  querySelector:()=>null,querySelectorAll:()=>[],addEventListener(){}};
 const globalLoad={count:0,handler:null,opts:null};
 const stubCalls={};const moduleStub=name=>({mount:(el,...rest)=>{(stubCalls[name]??=[]).push({el,rest});
   if(name==='SwarmWebRequestsAdmin')return {ready:Promise.resolve()};
   if(name==='SwarmEngineSettingsAdmin')return {};return undefined},
  encode:x=>JSON.stringify(x)});
 const ctx={console:rec.console,document,fetch:mock.fetch,URL,Error:RecordingError,
  location:{origin:'https://unit.test',href:'https://unit.test/'},
  history:{pushState(){},replaceState(){}},
  addEventListener(type,handler,opts){if(type==='load'){globalLoad.count++;globalLoad.handler=handler;globalLoad.opts=opts}},
  SwarmRecordedCursor:{parse:s=>JSON.parse(s)},
  SwarmWebRequests:{text:async()=>{throw Error('guard: SwarmWebRequests.text not exercised in this unit')}}};
 for(const n of ['SwarmWebRequestsAdmin','SwarmLogReadPolicyAdmin','SwarmActivityPolicyAdmin',
  'SwarmGraphPerformanceAdmin','SwarmEngineSettingsAdmin','SwarmLifecycleInventory','SwarmNativeCockpit',
  'SwarmConnectionProbeAdmin','SwarmRunLimitsAdmin','SwarmProviderWaitAdmin','SwarmNativeQuotas',
  'SwarmRuntimeHealth'])ctx[n]=moduleStub(n);
 vm.createContext(ctx);
 vm.runInContext(SRC_EN,ctx,{filename:'web/i18n-en.js'});
 ctx.SwarmI18n={t:s=>ctx.SwarmEnglish?.[s]??s,language:'fr'};
 let authorityStub=null;
 if(authority==='real')vm.runInContext(SRC_ADMIN,ctx,{filename:'web/native-authority-admin.js'});
 else{authorityStub={mountCalls:0,mount(){this.mountCalls++;
   return track(Promise.reject(SENT_MOUNT))}};
  ctx.SwarmAuthorityAdmin=authorityStub}
 vm.runInContext(SRC_RUNTIME,ctx,{filename:'web/native-runtime.js'});
 return {ctx,els,byId,stubCalls,authorityStub,errorSpy,globalLoad,
  /** Trigger the REAL global `load` transition the source subscribed to and
   * await the actual promise its captured `start` handler returns — real
   * completion of the async bootstrap, not a quiescence guess. The captured
   * handler must be the source's own `start` (a function, registered once,
   * `{once:true}`): any other subscription would fail loudly here. */
  boot:async()=>{
   if(globalLoad.count!==1||typeof globalLoad.handler!=='function'||globalLoad.opts?.once!==true)
    throw Error('guard: expected exactly one global load subscription with {once:true}, got count='+globalLoad.count+' handler='+typeof globalLoad.handler+' opts='+JSON.stringify(globalLoad.opts));
   const outcome=await globalLoad.handler();
   await barrier();
   return outcome}}}

const OWNER_KEY_ROUTES=[{name:CAP,reply:()=>jsonRes({access:{mode:'owner-key'}})}];
const AUTHORITY_ROUTES=[{name:CAP,reply:()=>jsonRes({access:{mode:'authority'}})},
 {name:AUTH,reply:()=>jsonRes({},{status:403})}];

// ——— i18n parity (targeted keys; catalog ↔ generated table) ———
function i18nParity(){const table=englishTable();
 check('en_catalog_generated_parity_for_capability_diagnostics',
  DIAGNOSTIC_KEYS.every(k=>typeof CATALOG[k]==='string'&&CATALOG[k]===table[k]),
  'catalog/generated mismatch on targeted diagnostic keys');
 check('en_translations_resolve_for_all_four_diagnostics',
  DIAGNOSTIC_KEYS.every(k=>typeof table[k]==='string'&&table[k].length>0&&table[k]!==k),
  'missing or untranslated diagnostic key')}

// ——— Scenario A: owner-key positive ———
async function ownerKeyPositive(){const mock=makeFetchMock();mock.setRoutes(OWNER_KEY_ROUTES);
 const rec=makeConsoleRec();const {els,mount,tEN}=makeAdminCtx(mock,rec);
 const host=els.create('div');host.hidden=true;
 const result=await mount(host,undefined,{translate:tEN});
 check('owner_key_resolves_mode_none',result?.mode==='none',JSON.stringify(result));
 check('owner_key_administer_false',result?.administer===false,JSON.stringify(result));
 check('owner_key_no_error_attached',result?.error===undefined&&result?.reason===undefined,JSON.stringify(result));
 check('owner_key_hides_card',host.hidden===true,'host.hidden='+host.hidden);
 check('owner_key_exactly_one_capability_fetch',mock.count(CAP)===1,'urls='+JSON.stringify(mock.urls()));
 check('owner_key_zero_daemon_fetches',mock.count(AUTH)===0,'urls='+JSON.stringify(mock.urls()));
 await barrier();
 check('owner_key_no_auto_retry_after_settle',mock.count(CAP)===1&&mock.count(AUTH)===0,'urls='+JSON.stringify(mock.urls()));
 check('owner_key_no_console_errors',rec.errors.length===0,rec.text())}

// ——— Scenario B: authority positive — the first /me is offered, then
// ——— refused (counted; no account is fabricated) ———
async function authorityPositiveOffersMe(){const mock=makeFetchMock();mock.setRoutes(AUTHORITY_ROUTES);
 const rec=makeConsoleRec();const {els,mount,tEN}=makeAdminCtx(mock,rec);
 const host=els.create('div');host.hidden=true;
 const result=await mount(host,undefined,{translate:tEN});
 check('authority_mode_offers_first_me_call',mock.count(CAP)===1&&mock.count(AUTH)===1,'urls='+JSON.stringify(mock.urls()));
 check('authority_me_refusal_resolves_visible_error_mode',result?.mode==='error'&&result?.administer===false,JSON.stringify(result));
 check('authority_me_refusal_status_preserved_403',result?.error?.status===403,'status='+result?.error?.status);
 check('authority_me_refusal_reason_stable_code',STABLE_CODE.test(result?.reason??''),'reason='+result?.reason);
 check('authority_me_refusal_no_views_granted',result?.views===undefined,'views='+JSON.stringify(result?.views));
 check('authority_me_refusal_card_visible',host.hidden===false,'host.hidden='+host.hidden);
 const text=domText(host);
 check('authority_me_refusal_diagnostic_in_english',
  text.includes(table(KEY_GENERIC_REFUSED))&&text.includes('http-403'),'text='+text);
 check('authority_me_refusal_console_stable_code',
  rec.errors.some(a=>a.includes('http-403'))&&rec.text().length>0,rec.text())}
function table(key){return englishTable()[key]??key}

// ——— Scenario C: capability HTTP 403 ———
async function capability403(){const mock=makeFetchMock();
 mock.setRoutes([{name:CAP,reply:()=>jsonRes({},{status:403})}]);
 const rec=makeConsoleRec();const {els,mount,tEN}=makeAdminCtx(mock,rec);
 const host=els.create('div');host.hidden=true;
 const result=await mount(host,undefined,{translate:tEN});
 check('capability_403_resolves_error_mode',result?.mode==='error'&&result?.administer===false,JSON.stringify(result));
 check('capability_403_status_preserved',result?.error?.status===403,'status='+result?.error?.status);
 check('capability_403_reason_stable_code',result?.reason==='authority-capability-refused','reason='+result?.reason);
 check('capability_403_single_fetch',mock.count(CAP)===1&&mock.count(AUTH)===0,'urls='+JSON.stringify(mock.urls()));
 check('capability_403_card_visible',host.hidden===false,'host.hidden='+host.hidden);
 const text=domText(host);
 check('capability_403_diagnostic_in_english',text.includes(table(KEY_CAP_REFUSED))&&text.includes('authority-capability-refused'),'text='+text)}

// ——— Scenario D: capability network reject (cause identity, no leak) ———
async function capabilityNetworkReject(){const mock=makeFetchMock();
 mock.setRoutes([{name:CAP,reply:()=>Promise.reject(SENT_CAP_NET)}]);
 const rec=makeConsoleRec();const {els,mount,tEN}=makeAdminCtx(mock,rec);
 const host=els.create('div');host.hidden=true;
 const result=await mount(host,undefined,{translate:tEN});
 check('capability_network_reject_resolves_error_mode',result?.mode==='error'&&result?.administer===false,JSON.stringify(result));
 check('capability_network_reject_cause_identity',result?.error?.cause===SENT_CAP_NET,
  'cause='+(result?.error?.cause?.message??String(result?.error?.cause)));
 check('capability_network_reject_reason_stable_code',result?.reason==='authority-capability-unreachable','reason='+result?.reason);
 const text=domText(host);
 check('capability_network_reject_diagnostic_in_english',text.includes(table(KEY_CAP_UNREACHABLE)),'text='+text);
 check('capability_network_reject_no_private_in_dom_console_reason',
  !/private/i.test(leakScan(els,rec,result?.reason)),'dom/console/reason scan')}

// ——— Scenario O: explicit recovery after negative cases (fresh mounts) ———
async function explicitRecovery(){const mock=makeFetchMock();
 mock.setRoutes([{name:CAP,reply:()=>Promise.reject(SENT_CAP_NET)}]);
 const rec=makeConsoleRec();const {els,mount,tEN}=makeAdminCtx(mock,rec);
 const host1=els.create('div');host1.hidden=true;
 const bad=await mount(host1,undefined,{translate:tEN});
 check('recovery_setup_error_mode_reached',bad?.mode==='error','result='+JSON.stringify(bad));
 await barrier();const logAfterBad=mock.log.length;
 check('recovery_no_auto_retry_after_error',mock.log.length===logAfterBad,'urls='+JSON.stringify(mock.urls()));
 // Fresh mount on a fresh host, like after a reload, with a real valid
 // owner-key capability: no automatic retry, no parasitic /me.
 mock.setRoutes(OWNER_KEY_ROUTES);mock.reset();
 const host2=els.create('div');host2.hidden=true;
 const good=await mount(host2,undefined,{translate:tEN});
 check('recovery_after_capability_network_failure_resolves_none',good?.mode==='none'&&host2.hidden===true,JSON.stringify(good)+' hidden='+host2.hidden);
 check('recovery_single_capability_fetch_zero_me',mock.count(CAP)===1&&mock.count(AUTH)===0,'urls='+JSON.stringify(mock.urls()));
 await barrier();
 check('recovery_stable_after_settle',mock.log.length===1,'urls='+JSON.stringify(mock.urls()));
 // Same recovery shape after a refused /me (authority capability first).
 mock.setRoutes(AUTHORITY_ROUTES);mock.reset();
 const host3=els.create('div');host3.hidden=true;
 const refused=await mount(host3,undefined,{translate:tEN});
 check('recovery_setup_me_refusal_reached',refused?.mode==='error'&&refused?.error?.status===403,'result='+JSON.stringify(refused));
 mock.setRoutes(OWNER_KEY_ROUTES);mock.reset();
 const host4=els.create('div');host4.hidden=true;
 const reloaded=await mount(host4,undefined,{translate:tEN});
 check('recovery_after_me_refusal_owner_key_none_zero_me',
  reloaded?.mode==='none'&&host4.hidden===true&&mock.count(CAP)===1&&mock.count(AUTH)===0,
  JSON.stringify(reloaded)+' urls='+JSON.stringify(mock.urls()))}

// ——— Scenario E: capability JSON parse failure ———
async function capabilityParseReject(){const mock=makeFetchMock();
 mock.setRoutes([{name:CAP,reply:()=>parseRejectRes(SENT_CAP_PARSE)}]);
 const rec=makeConsoleRec();const {els,mount,tEN}=makeAdminCtx(mock,rec);
 const host=els.create('div');host.hidden=true;
 const result=await mount(host,undefined,{translate:tEN});
 check('capability_parse_reject_resolves_error_mode',result?.mode==='error'&&result?.administer===false,JSON.stringify(result));
 check('capability_parse_reject_cause_identity',result?.error?.cause===SENT_CAP_PARSE,
  'cause='+(result?.error?.cause?.message??String(result?.error?.cause)));
 check('capability_parse_reject_reason_stable_code',result?.reason==='authority-capability-invalid','reason='+result?.reason);
 const text=domText(host);
 check('capability_parse_reject_diagnostic_in_english',text.includes(table(KEY_CAP_INVALID)),'text='+text);
 check('capability_parse_reject_no_private_leak',!/private/i.test(leakScan(els,rec,result?.reason)),'dom/console/reason scan')}

// ——— Scenario F: capability key absent / mode unknown ———
async function capabilityKeyAbsent(){const mock=makeFetchMock();
 mock.setRoutes([{name:CAP,reply:()=>jsonRes({})}]);
 const rec=makeConsoleRec();const {els,mount,tEN}=makeAdminCtx(mock,rec);
 const host=els.create('div');host.hidden=true;
 const result=await mount(host,undefined,{translate:tEN});
 check('capability_key_absent_resolves_error_not_none',result?.mode==='error'&&result?.administer===false,JSON.stringify(result));
 check('capability_key_absent_reason_stable_code',result?.reason==='authority-capability-invalid','reason='+result?.reason);
 check('capability_key_absent_card_visible',host.hidden===false,'host.hidden='+host.hidden);
 const text=domText(host);
 check('capability_key_absent_diagnostic_in_english',text.includes(table(KEY_CAP_INVALID)),'text='+text);
 // Unknown mode value takes the same invalid branch (fresh host, reset log).
 mock.reset();const host2=els.create('div');host2.hidden=true;
 mock.setRoutes([{name:CAP,reply:()=>jsonRes({access:{mode:'shared-key'}})}]);
 const result2=await mount(host2,undefined,{translate:tEN});
 check('capability_mode_unknown_resolves_error_not_none',
  result2?.mode==='error'&&host2.hidden===false&&result2?.reason==='authority-capability-invalid',
  'result='+JSON.stringify(result2)+' hidden='+host2.hidden)}

// ——— Scenario G: malformed /me document (authority mode) ———
async function meMalformed(){const mock=makeFetchMock();
 mock.setRoutes([{name:CAP,reply:()=>jsonRes({access:{mode:'authority'}})},
  {name:AUTH,reply:()=>jsonRes({unexpected:true})}]);
 const rec=makeConsoleRec();const {els,mount,tEN}=makeAdminCtx(mock,rec);
 const host=els.create('div');host.hidden=true;
 const result=await mount(host,undefined,{translate:tEN});
 check('me_malformed_resolves_error_mode',result?.mode==='error'&&result?.administer===false,JSON.stringify(result));
 check('me_malformed_reason_stable_code',result?.reason==='authority-me-invalid','reason='+result?.reason);
 check('me_malformed_card_visible',host.hidden===false,'host.hidden='+host.hidden);
 const text=domText(host);
 check('me_malformed_diagnostic_in_english',
  text.includes(table(KEY_GENERIC_REFUSED))&&text.includes('authority-me-invalid'),'text='+text);
 check('me_malformed_no_private_leak',!/private/i.test(leakScan(els,rec,result?.reason)),'dom/console/reason scan')}

// ——— Scenario I: /me network reject — the FIX lot's target: transport
// ——— failure must arrive wrapped under a stable code with the original
// ——— error reachable by identity on the cause chain, and no private leak.
async function meNetworkReject(){const mock=makeFetchMock();
 mock.setRoutes([{name:CAP,reply:()=>jsonRes({access:{mode:'authority'}})},
  {name:AUTH,reply:()=>Promise.reject(SENT_ME_NET)}]);
 const rec=makeConsoleRec();const {els,mount,tEN}=makeAdminCtx(mock,rec);
 const host=els.create('div');host.hidden=true;
 const result=await mount(host,undefined,{translate:tEN});
 check('me_network_reject_resolves_error_mode',result?.mode==='error'&&result?.administer===false&&host.hidden===false,JSON.stringify(result)+' hidden='+host.hidden);
 // The original error object survives unmutated and reachable through the
 // stable wrapper's `cause` chain (cycle-detected walk, no ceiling).
 check('me_network_reject_error_identity_original',causeChainHas(result?.error,SENT_ME_NET),
  'error='+(result?.error?.message??String(result?.error)));
 const text=domText(host);
 check('me_network_reject_diagnostic_in_english',text.includes(table(KEY_GENERIC_REFUSED)),'text='+text);
 check('me_network_reject_reason_is_stable_code',result?.reason==='authority-me-unreachable',
  'reason='+result?.reason);
 check('me_network_reject_no_private_in_dom_console_reason',
  !/private/i.test(leakScan(els,rec,result?.reason)),'dom/console/reason scan');
 check('me_network_reject_card_visible_no_retry',host.hidden===false&&mock.count(AUTH)===1&&mock.count(CAP)===1,
  'hidden='+host.hidden+' urls='+JSON.stringify(mock.urls()))}

// ——— Scenario J: /me JSON parse failure — cause identity is a fixed
// ——— obligation since the red run: the decode cause must reach
// ——— `error.cause` (flattened carrier), never swallowed. ———
async function meParseReject(){const mock=makeFetchMock();
 mock.setRoutes([{name:CAP,reply:()=>jsonRes({access:{mode:'authority'}})},
  {name:AUTH,reply:()=>parseRejectRes(SENT_ME_PARSE)}]);
 const rec=makeConsoleRec();const {els,mount,tEN}=makeAdminCtx(mock,rec);
 const host=els.create('div');host.hidden=true;
 const result=await mount(host,undefined,{translate:tEN});
 check('me_parse_reject_resolves_error_mode',result?.mode==='error'&&result?.administer===false&&host.hidden===false,JSON.stringify(result)+' hidden='+host.hidden);
 // Contract: the original decode failure stays inspectable on `error.cause`
 // (the 2xx unreadable-document carrier is flattened so the cause sits
 // directly there) — the former catch-empty swallow was the observed red.
 check('me_parse_reject_cause_identity',result?.error?.cause===SENT_ME_PARSE,
  'cause='+(result?.error?.cause?.message??String(result?.error?.cause)));
 check('me_parse_reject_reason_stable_code',result?.reason==='authority-me-invalid','reason='+result?.reason);
 const text=domText(host);
 check('me_parse_reject_diagnostic_in_english',text.includes(table(KEY_GENERIC_REFUSED)),'text='+text);
 check('me_parse_reject_no_private_leak',!/private/i.test(leakScan(els,rec,result?.reason)),'dom/console/reason scan')}

// ——— Scenario P (renderer lot glm-authority-renderer-safe-code-20261104T005700Z;
// ——— control-route repair glm-dialog-guard-control-route-20261104T012700Z):
// ——— the dialog-submit path exercises the SHARED refusal renderer for real.
// ——— The fictitious /me DTO really answers the first daemon read, and the
// ——— positive preconditions (authority mode, exact GET /me, own button in
// ——— nav) are asserted BEFORE the refusal is injected. An unknown daemon
// ——— code (the supervisor discriminant sentinel) must
// ——— render only the generic refusal message and safe code — never the raw
// ——— code — while the original error stays unmutated with its status for
// ——— diagnostics; explicit recovery via a real resubmit completes. The /me
// ——— document, the submitted values and the account are strictly fictitious
// ——— unit fixtures (no UID, enrollment, session or acceptance); nothing
// ——— here touches a live daemon, network, payment or database.
async function dialogSubmitUnknownDaemonCode(){
 const meDto={account:{subject:'unit-fixture',display_name:'Unit Fixture',
  roles:['viewer'],integrity:1},administer:false,permissions:[],views:[]};
 // Mount-phase control routes: the mock matches routes by SUBSTRING
 // (`String(url).includes(r.name)`), so the specific `/me/password` and
 // `/me/keys` routes are declared BEFORE the general `/me` route — a
 // leading `/me` would otherwise swallow those specific URLs. The `/me`
 // entry is the control route: it really answers the first daemon read
 // with the fictitious DTO declared above (this wiring was missing in the
 // received red — `meDto` was declared but never served, the unmatched
 // fetch made mount resolve in visible error mode, and the card-visible
 // check alone mistook that error mode for a positive witness).
 const controlRoutes=[
  {name:'/me/password',reply:()=>jsonRes({error:'PRIVATE_UNKNOWN_DAEMON_DETAIL'},{status:500})},
  {name:'/me/keys',reply:()=>jsonRes({keys:[],now:0})},
  {name:'/me',reply:()=>jsonRes(meDto)},
  {name:CAP,reply:()=>jsonRes({access:{mode:'authority'}})}];
 const mock=makeFetchMock();mock.setRoutes(controlRoutes);
 const rec=makeConsoleRec();const {els,mount,tEN,errorSpy}=makeAdminCtx(mock,rec);
 const host=els.create('div');host.hidden=true;const nav=els.create('nav');
 const mounted=await mount(host,nav,{translate:tEN});
 check('dialog_path_mount_authority_card_visible',host.hidden===false,'hidden='+host.hidden);
 // Positive preconditions, asserted BEFORE the refusal path is exercised:
 // an invalid precondition must fail the harness loudly here, never let the
 // scenario continue into a fake witness.
 const meReads=mock.log.filter(e=>e.url===AUTH+'me');
 check('dialog_path_single_exact_me_read',
  meReads.length===1&&meReads[0].opts?.method==='GET',
  'me='+(meReads.length===1?meReads[0].opts?.method+' '+meReads[0].url:JSON.stringify(mock.urls())));
 check('dialog_path_mount_resolves_authority_mode',
  mounted?.mode==='authority'&&mounted?.administer===false,JSON.stringify(mounted));
 const own=els.all.find(e=>e.tag==='button'&&'authorityOwnPassword' in e.dataset);
 check('dialog_path_own_password_button_present',own!==undefined&&nav.children.includes(own),
  'own='+(own?'in-nav='+nav.children.includes(own):'absent'));
 // Preconditions are harness obligations, not behavioral assertions: if any
 // failed above, stop here with an explicit guard error instead of
 // continuing into a fake witness (the received red continued to
 // `own.listeners` on an absent button and died on a TypeError).
 if(meReads.length!==1||meReads[0].opts?.method!=='GET'||
   mounted?.mode!=='authority'||mounted?.administer!==false||
   own===undefined||!nav.children.includes(own))
  throw Error('guard: dialog-submit positive preconditions failed — see the FAIL lines above');
 own.listeners.click[0]();
 const form=els.all.find(e=>e.tag==='form');
 const submit=form?.listeners?.submit?.[0];
 check('dialog_submit_handler_registered',typeof submit==='function',
  'form='+(form?'handler='+typeof submit:'absent'));
 // `twice()` requires matching non-empty new-password values before the
 // POST is issued; the values are fictitious unit fixtures, never a real
 // credential.
 const pwInputs=els.all.filter(e=>e.tag==='input'&&e.type==='password');
 check('dialog_submit_password_inputs_present',pwInputs.length===3,'count='+pwInputs.length);
 for(const i of pwInputs)i.value='unit-fixture-password';
 await submit({preventDefault(){}});
 const stateNotice=els.all.find(e=>e.tag==='p'&&e.className==='notice alert');
 check('dialog_submit_unknown_daemon_code_refusal_visible',
  stateNotice!==undefined&&stateNotice.hidden===false,'state='+(stateNotice?.className??'absent'));
 const okBtn=els.all.find(e=>e.tag==='footer')?.children.find(e=>e.tag==='button'&&e.className==='primary');
 check('dialog_submit_ok_reenabled_for_recovery',okBtn?.disabled===false,'disabled='+okBtn?.disabled);
 const rendered=stateNotice?.textContent??'';
 check('dialog_submit_renders_generic_refusal_message_only',
  rendered===table(KEY_GENERIC_REFUSED)+' ('+KEY_GENERIC_REFUSAL_CODE+')','rendered='+rendered);
 check('dialog_submit_unknown_daemon_code_never_renders',
  !rendered.includes('PRIVATE_UNKNOWN_DAEMON_DETAIL'),'rendered='+rendered);
 check('dialog_submit_no_private_leak_in_dom_console',
  !/private/i.test(leakScan(els,rec,'')),'dom/console scan');
 const raw500=errorSpy.find(e=>e?.message==='PRIVATE_UNKNOWN_DAEMON_DETAIL'&&e.status===500);
 check('dialog_submit_original_error_unmutated_with_status',
  raw500!==undefined&&raw500.status===500&&raw500.reason===undefined,
  'raw='+JSON.stringify({message:raw500?.message,status:raw500?.status,reason:raw500?.reason}));
 check('dialog_submit_reason_not_contaminated',
  !String(raw500?.reason??'').includes('PRIVATE_UNKNOWN_DAEMON_DETAIL'),'reason='+raw500?.reason);
 // Explicit recovery: the refusal is flipped to success (daemon decision),
 // the SAME dialog is resubmitted for real — no reset, no reload.
 mock.setRoutes([
  {name:'/me/password',reply:()=>jsonRes({ok:true})},
  {name:'/me/keys',reply:()=>jsonRes({keys:[],now:0})},
  {name:CAP,reply:()=>jsonRes({access:{mode:'authority'}})}]);
 await submit({preventDefault(){}});
 // On success the notice lands on the CARD status (product `notice()`),
 // not the dialog state (which stays at the in-flight "Sending…" text):
 // assert the translated success notice where the product really puts it.
 const successNotice=els.all.find(e=>e.tag==='p'&&
  e.textContent===table('Mot de passe changé. Vos autres sessions sont terminées.')&&
  e.className==='notice info'&&e.hidden===false);
 check('dialog_submit_recovery_resubmit_success_notice_on_card',
  successNotice!==undefined,
  'notice='+JSON.stringify(els.all.filter(e=>e.tag==='p').map(e=>e.className+':'+e.textContent)));
 check('dialog_submit_recovery_single_retried_post',
  mock.count('/me/password')===2&&mock.count(CAP)===1&&mock.count('/me/keys')===1,
  'urls='+JSON.stringify(mock.urls()));
 check('dialog_submit_recovery_no_private_leak_after_success',
  !/private/i.test(leakScan(els,rec,'')),'dom/console scan after recovery');
}

// ——— Scenario K: cockpit — RAW private unexpected mount rejection. The
// ——— double rejects with the private sentinel itself (no pre-wrapping), so
// ——— the consumer's own catch must build the stable wrapper: card visible,
// ——— safe stable code only in DOM/console, original cause reachable.
async function cockpitUnexpectedRejection(){const mock=makeFetchMock();mock.setRoutes(OWNER_KEY_ROUTES);
 const rec=makeConsoleRec();
 const {els,host,entry,errorSpy,run}=makeCockpitCtx(mock,rec,'reject');
 await run();
 check('cockpit_unexpected_rejection_card_visible_not_hidden',host.hidden===false,'host.hidden='+host.hidden);
 check('cockpit_unexpected_rejection_entry_not_hidden',entry.hidden===false,'entry.hidden='+entry.hidden);
 const text=domText(host);
 check('cockpit_unexpected_rejection_notice_english_with_stable_code',
  text.includes(table(KEY_GENERIC_REFUSED))&&text.includes('authority-mount-unexpected'),'text='+text);
 check('cockpit_unexpected_rejection_console_reason',
  rec.errors.some(a=>a[0]==='SwarmAuthorityAdmin mount:'&&a[1]==='authority-mount-unexpected'),rec.text());
 check('cockpit_unexpected_rejection_no_private_leak',!/private/i.test(leakScan(els,rec,'authority-mount-unexpected')),'dom/console scan');
 // The consumer's stable wrapper was built through the sandbox `Error` spy:
 // it carries the RAW private sentinel on its `cause` chain unmutated, while
 // only the stable code is rendered/logged.
 const wrapper=errorSpy.find(e=>e?.message==='authority-mount-unexpected');
 check('cockpit_unexpected_rejection_notice_carries_cause',
  wrapper!==undefined&&causeChainHas(wrapper,SENT_MOUNT),'wrappers='+JSON.stringify(errorSpy.map(e=>e?.message)))}

// ——— Scenario L1: cockpit — capability error keeps the entry visible ———
async function cockpitCapabilityError(){const mock=makeFetchMock();
 mock.setRoutes([{name:CAP,reply:()=>jsonRes({},{status:403})}]);
 const rec=makeConsoleRec();const {els,host,entry,run}=makeCockpitCtx(mock,rec,'real');
 await run();
 check('cockpit_capability_error_entry_stays_visible',entry.hidden===false,'entry.hidden='+entry.hidden);
 check('cockpit_capability_error_card_visible',host.hidden===false,'host.hidden='+host.hidden);
 const text=domText(host);
 check('cockpit_capability_error_diagnostic_in_english',text.includes(table(KEY_CAP_REFUSED)),'text='+text);
 check('cockpit_capability_error_console_reason',
  rec.errors.some(a=>a[0]==='SwarmAuthorityAdmin mount:'&&a[1]==='authority-capability-refused'),rec.text());
 check('cockpit_capability_error_single_fetch',mock.count(CAP)===1&&mock.count(AUTH)===0,'urls='+JSON.stringify(mock.urls()))}

// ——— Scenario L2: cockpit — owner-key folds the entry away with one fetch ———
async function cockpitOwnerKey(){const mock=makeFetchMock();mock.setRoutes(OWNER_KEY_ROUTES);
 const rec=makeConsoleRec();const {els,host,entry,run}=makeCockpitCtx(mock,rec,'real');
 await run();
 check('cockpit_owner_key_entry_hidden',entry.hidden===true,'entry.hidden='+entry.hidden);
 check('cockpit_owner_key_card_hidden',host.hidden===true,'host.hidden='+host.hidden);
 check('cockpit_owner_key_single_fetch_zero_me',mock.count(CAP)===1&&mock.count(AUTH)===0,'urls='+JSON.stringify(mock.urls()));
 check('cockpit_owner_key_no_console_errors',rec.errors.length===0,rec.text())}

// ——— Scenario M: runtime — RAW private unexpected mount rejection (the
// ——— double rejects with the sentinel itself). The runtime's own catch must
// ——— wrap it: card visible, stable code only, sentinel reachable on cause.
async function runtimeUnexpectedRejection(){const mock=makeFetchMock();
 mock.setRoutes([{name:SESSION,reply:()=>jsonRes({schema_version:1,csrf_format:'hmac-sha256-v1',csrf_token:'a'.repeat(64)})}]);
 const rec=makeConsoleRec();
 const {els,byId,authorityStub,errorSpy,boot}=makeRuntimeCtx(mock,rec,'reject');
 await boot();
 const host=byId['native-authority-admin'];
 check('runtime_unexpected_rejection_mount_called_once_no_retry',authorityStub?.mountCalls===1,'mountCalls='+authorityStub?.mountCalls);
 check('runtime_unexpected_rejection_card_visible',host?.hidden===false,'host.hidden='+host?.hidden);
 const text=domText(host);
 check('runtime_unexpected_rejection_notice_english_stable_code',
  text.includes(table(KEY_GENERIC_REFUSED))&&text.includes('authority-mount-unexpected'),'text='+text);
 check('runtime_unexpected_rejection_console_reason',
  rec.errors.some(a=>a[0]==='SwarmAuthorityAdmin mount:'&&a[1]==='authority-mount-unexpected'),rec.text());
 check('runtime_unexpected_rejection_no_private_leak',!/private/i.test(leakScan(els,rec,'authority-mount-unexpected')),'dom/console scan');
 check('runtime_unexpected_rejection_no_extra_fetches',
  mock.count(SESSION)===1&&mock.count(AUTH)===0,'urls='+JSON.stringify(mock.urls()));
 // Same cause-chain obligation as the cockpit consumer, observed through the
 // sandbox `Error` spy: the RAW private sentinel stays reachable unmutated on
 // the stable wrapper's `cause` chain.
 const wrapper=errorSpy.find(e=>e?.message==='authority-mount-unexpected');
 check('runtime_unexpected_rejection_notice_carries_cause',
  wrapper!==undefined&&causeChainHas(wrapper,SENT_MOUNT),'wrappers='+JSON.stringify(errorSpy.map(e=>e?.message)));
 await barrier();
 check('runtime_unexpected_rejection_stable_after_settle',authorityStub?.mountCalls===1,'mountCalls='+authorityStub?.mountCalls)}

// ——— Scenario N: runtime — real admin module, owner-key capability ———
async function runtimeOwnerKey(){const mock=makeFetchMock();
 mock.setRoutes([{name:SESSION,reply:()=>jsonRes({schema_version:1,csrf_format:'hmac-sha256-v1',csrf_token:'a'.repeat(64)})},
  ...OWNER_KEY_ROUTES]);
 const rec=makeConsoleRec();const {els,byId,boot}=makeRuntimeCtx(mock,rec,'real');
 await boot();
 const host=byId['native-authority-admin'];
 check('runtime_owner_key_real_mount_card_hidden',host?.hidden===true,'host.hidden='+host?.hidden);
 check('runtime_owner_key_no_daemon_fetch',mock.count(AUTH)===0,'urls='+JSON.stringify(mock.urls()));
 check('runtime_owner_key_session_then_capability',mock.count(SESSION)===1&&mock.count(CAP)===1,'urls='+JSON.stringify(mock.urls()));
 await barrier();
 check('runtime_owner_key_no_mount_retry_after_settle',
  mock.count(SESSION)===1&&mock.count(CAP)===1&&mock.count(AUTH)===0,'urls='+JSON.stringify(mock.urls()));
 check('runtime_owner_key_no_console_errors',rec.errors.length===0,rec.text())}

(async()=>{
 i18nParity();
 await ownerKeyPositive();
 await authorityPositiveOffersMe();
 await capability403();
 await capabilityNetworkReject();
 await capabilityParseReject();
 await capabilityKeyAbsent();
 await meMalformed();
 await meNetworkReject();
 await meParseReject();
 await dialogSubmitUnknownDaemonCode();
 await cockpitUnexpectedRejection();
 await cockpitCapabilityError();
 await cockpitOwnerKey();
 await runtimeUnexpectedRejection();
 await runtimeOwnerKey();
 await explicitRecovery();
 console.log('GUARD_SUMMARY pass='+pass+' fail='+fail+
  ' (all assertions are contract obligations; no expected-red set remains)');
 if(fail===0)console.log('AUTHORITY_CAPABILITY_CONSUMER_GUARD_PASS');
 else{console.log('AUTHORITY_CAPABILITY_CONSUMER_GUARD_RED fail='+fail);process.exitCode=1}
})().catch(error=>{console.error('GUARD_HARNESS_ERROR',error);process.exitCode=1});
