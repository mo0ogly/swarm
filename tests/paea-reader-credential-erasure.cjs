'use strict';
// Maintained counterexample test for the READER credential erasure contract of
// the public PAEA approval form (web/paea-approval-form.js). The served page is
// the REAL workspace frontend in real Chrome; the API prerequisite endpoints
// are EXPLICIT synthetic fixtures on a local HTTP server so each load exit can
// be isolated (this is the same counterexample method Codex used on
// 2026-11-01: isolated synthetic API error fixtures against the actual
// frontend — they prove the page's erasure contract, never identity, never a
// real enrollment, never a product approval). The form must erase the typed
// reader credential on EVERY exit of `load()`: normal guarded read (refused
// presentation), native-session HTTP error, capability HTTP error, unbound
// capability, malformed JSON reply, and a follow-up successful re-observation
// (recovery) — the credential never survives in the input, the page HTML or
// any storage. The reader input carries NO maximum-length cutoff: a 400-byte
// token (valid producer shape: ≥16 bytes, no whitespace) must be accepted
// into the guarded read unchanged, and a too-short token must reach the
// server rather than be pre-cut by the frontend. No storage/URL/log carriage;
// no guard or authentication change; no acceptance authority.
const assert=require('node:assert/strict'),http=require('node:http'),fs=require('node:fs'),path=require('node:path');
const pup=require(path.join(process.cwd(),'node_modules','puppeteer'));
const out=path.resolve(process.argv[2]);
fs.mkdirSync(out,{recursive:true});
const root=process.cwd();
// 400-byte token: proves no frontend maximum-length cutoff (far over the old
// arbitrary 128); ≥16 bytes, no whitespace, so it satisfies the producer-side
// shape check (trust_types.rs credential_valid).
const LONG_TOKEN='labelled-reader-fixture-'.padEnd(400,'x');
const SHORT_TOKEN='court'; // <16 bytes: valid frontend input, refused server-side shape
const checks=[],errors=[];
let mode='normal',browser,server;
(async()=>{
 server=http.createServer((q,s)=>{
  const url=new URL(q.url,'http://localhost');s.setHeader('Cache-Control','no-store');
  const j=(v,status=200)=>{s.writeHead(status,{'Content-Type':'application/json'});s.end(typeof v==='string'?v:JSON.stringify(v))};
  if(url.pathname==='/api/v1/native-session')return mode==='session-error'?j({error:'fixture-session-error'},503)
   :mode==='session-garbage'?j('not-json',200):j({schema_version:1,csrf_token:'a'.repeat(64)});
  if(url.pathname==='/api/v1/paea/git-call/approval')return mode==='status-error'?j({error:'fixture-status-error'},503):j({bound:mode!=='unbound'});
  if(url.pathname==='/api/v1/paea/git-call/approval/presentation')return j({available:false,reason:'paea-reader-authentication-required'});
  if(url.pathname.startsWith('/api/'))return j({},404);
  const f=path.join(root,'web',url.pathname.slice(1));
  if(fs.existsSync(f)&&fs.statSync(f).isFile()){s.setHeader('Content-Type',f.endsWith('.js')?'text/javascript':f.endsWith('.html')?'text/html':'text/css');s.end(fs.readFileSync(f))}else{s.writeHead(404);s.end()}
 });
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 browser=await pup.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
 const page=await browser.newPage();
 page.setDefaultTimeout(20000);
 page.on('pageerror',e=>errors.push(e.message));
 // The host page's SIBLING sections (preview / git-call capability /
 // reconcile) each fetch their own endpoint once per load; this fixture only
 // mocks the approval-prerequisite endpoints, so those three fetches refuse
 // 404 here. That is the fixture's isolation, not a frontend defect: classify
 // exactly these URL-less console duplicates (Chrome omits the URL) of the
 // three already-404 sibling endpoints; any other diagnostic stays unexpected.
 let sibling404=0;
 page.on('response',r=>{if(r.status()===404&&['/api/v1/paea/git-call/preview','/api/v1/paea/git-call','/api/v1/paea/reconcile-effects'].some(u=>r.url().endsWith(u)))sibling404++});
 page.on('console',m=>{if(m.type()==='error'&&m.text().includes('status of 404')&&sibling404>0){sibling404--;return}if(m.type()==='error'&&!m.text().includes('status of 503'))errors.push(m.text())});
 await page.goto('http://127.0.0.1:'+server.address().port+'/paea-effects.html?lang=en',{waitUntil:'networkidle0'});
 await page.waitForFunction(()=>document.querySelector('#approval-status').dataset.state!=='loading');
 async function typeToken(v){
  await page.$eval('#approval-reader-credential',(e,value)=>{e.value=value},v);
 }
 // Full residue witness: input value, page HTML and any web storage must never
 // retain the typed credential once load() has exited.
 const residue=t=>page.evaluate(token=>({
  input:document.querySelector('#approval-reader-credential').value,
  html:document.documentElement.innerHTML.includes(token),
  storage:JSON.stringify({ls:Object.entries(localStorage),ss:Object.entries(sessionStorage)}).includes(token)
 }),t);
 async function observe(variant){
  mode=variant;
  await page.click('#approval-reload');
  await page.waitForFunction(()=>document.querySelector('#approval-status').dataset.state!=='loading');
  return page.$eval('#approval-status',e=>e.dataset.state);
 }
 // 1. Normal: refused guarded presentation (synthetic) — erasure witnessed.
 await typeToken(LONG_TOKEN);
 let seen=await page.$eval('#approval-reader-credential',e=>e.value);
 assert.equal(seen.length,400,'the 400-byte token must sit in the input uncut (no frontend maxlength cutoff)');
 let state=await observe('normal');
 let res=await residue(LONG_TOKEN);
 assert.equal(state,'empty','normal refused presentation stays the explicit empty state');
 assert.ok(!res.input&&!res.html&&!res.storage,'normal exit must erase the credential everywhere: '+JSON.stringify(res));
 checks.push('normal refused presentation: 400-byte token uncut, erased from input/HTML/storage');
 // 2. Session HTTP error: erasure BEFORE the error state renders.
 state=await observe('session-error');
 res=await residue(LONG_TOKEN);
 assert.equal(state,'error','session HTTP error stays a visible error');
 assert.ok(!res.input&&!res.html&&!res.storage,'session-error exit must erase the credential everywhere: '+JSON.stringify(res));
 checks.push('native-session 503: visible error, credential erased everywhere');
 // 3. Capability HTTP error: same erasure contract.
 state=await observe('status-error');
 res=await residue(LONG_TOKEN);
 assert.equal(state,'error','capability HTTP error stays a visible error');
 assert.ok(!res.input&&!res.html&&!res.storage,'status-error exit must erase the credential everywhere: '+JSON.stringify(res));
 checks.push('capability 503: visible error, credential erased everywhere');
 // 4. Unbound capability: erasure before the explicit empty state.
 state=await observe('unbound');
 res=await residue(LONG_TOKEN);
 assert.equal(state,'empty','unbound capability stays the explicit empty state');
 assert.ok(!res.input&&!res.html&&!res.storage,'unbound exit must erase the credential everywhere: '+JSON.stringify(res));
 checks.push('unbound capability: explicit empty, credential erased everywhere');
 // 5. Malformed session reply (JSON garbage): erasure on the parse-error exit.
 state=await observe('session-garbage');
 res=await residue(LONG_TOKEN);
 assert.equal(state,'error','malformed session reply stays a visible error');
 assert.ok(!res.input&&!res.html&&!res.storage,'garbage-reply exit must erase the credential everywhere: '+JSON.stringify(res));
 checks.push('malformed session reply: visible error, credential erased everywhere');
 // 6. Recovery: after a fresh successful prerequisite chain the form re-reads
 //    the guarded presentation (still refused synthetically) — a typed token
 //    is accepted and erased again, proving erasure never blocks re-reading.
 state=await observe('normal');
 assert.equal(state,'empty','recovery observation must reach the guarded read again');
 await typeToken(SHORT_TOKEN);
 state=await observe('normal');
 res=await residue(SHORT_TOKEN);
 assert.equal(state,'empty','short-token recovery stays the explicit empty state');
 assert.ok(!res.input&&!res.html&&!res.storage,'recovery exit must erase the short token everywhere: '+JSON.stringify(res));
 // The short token was NOT pre-cut away by the frontend: it must have reached
 // the server. Assert the last presentation request carried it verbatim as a
 // Bearer (the synthetic listener records nothing here, so re-check through a
 // request interception on a final typed observation).
 const sent=await (async()=>{
  let captured=null;
  const handler=q=>{const h=q.headers['authorization'];if(h&&!captured)captured=h};
  server.on('request',handler);
  await typeToken(SHORT_TOKEN);
  await observe('normal');
  server.off('request',handler);
  return captured;
 })();
 assert.equal(sent,'Bearer '+SHORT_TOKEN,'the frontend must send the typed token verbatim (no pre-cut, no rejection): '+JSON.stringify(sent));
 checks.push('recovery: short token sent verbatim as Bearer (no frontend cutoff), erased after the read');
 fs.writeFileSync(path.join(out,'reader-credential-erasure.json'),JSON.stringify({schema_version:1,kind:'paea-reader-credential-erasure-v1',checks,unexpected_diagnostics:errors,at:new Date().toISOString()},null,2)+'\n');
 console.log(JSON.stringify({checks:checks.length,unexpected_diagnostics:errors.length}));
})().catch(e=>{fs.writeFileSync(path.join(out,'reader-credential-erasure-failure.json'),JSON.stringify({message:e.message,stack:e.stack,checks,errors},null,2),{mode:0o600});console.error('FAIL:',e.message);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(server)await new Promise(r=>server.close(r))});
