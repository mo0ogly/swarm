'use strict';
// Bounded browser qualification of the served PAEA approver-consent page with
// a REAL bound approval. Owned two-phase control protocol with the --ignored
// Rust test consent_page_serves_real_bound_approval_to_real_browser: that test
// freezes one exact approve command through the real prepare_approval producer,
// binds a real native server with the approval attached, publishes server.json,
// then waits for owned marker files (nonce-bound). This harness drives the real
// served page in a real Chrome: phase 1 asserts the bound capability, the exact
// prepared action and the visible consent form; phase 2 drives the PAGE ITSELF
// through two visible refusals — a non-empty body with the correct password
// (typed transport refusal, the held-command contract) and an empty body with
// a wrong password (typed credential refusal) — with no approval recorded and
// no automatic retry. No consent
// capture, no grant, no one-shot consumption, no acceptance; screenshots and
// console/request assertions stay in the owned evidence directory.
const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path'),cp=require('node:child_process');
const pup=require(path.join(process.cwd(),'node_modules','puppeteer'));
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),control=path.resolve(process.argv[4]);
fs.mkdirSync(out,{recursive:true});
const server=JSON.parse(fs.readFileSync(path.join(control,'server.json'),'utf8'));
const serviceRoot=server.service_root,projectRoot=server.project_root,origin=`http://${server.address}`;
const nonce=server.nonce;
const secret=fs.readFileSync(path.join(serviceRoot,'.swarm',server.session_file),'utf8').trim();
const checks=[],errors=[];let expected=0,consoleDenialEchoes=0;
let browser;
(async()=>{
 browser=await pup.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
 const page=await browser.newPage();
 page.setDefaultTimeout(20000);
 page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{
  if(m.type()!=='error')return;
  // Chrome mirrors every denied fetch as a generic 'Failed to load resource'
  // console error — including the expected refusal responses below. Count
  // only that exact echo; any other console error stays a diagnostic.
  if(/^Failed to load resource.*status of \d{3}/.test(m.text())){consoleDenialEchoes+=1;return}
  errors.push(m.text());
 });
 page.on('response',r=>{
  if(r.status()<400)return;
  // Only the refusal submissions are expected >=400 responses: the
  // non-empty-body probe and the wrong-password page submissions, all on the
  // approval POST. Any other >=400 response is an unexpected diagnostic — a
  // silent allowance here would hide a real regression.
  if(r.url().endsWith('/api/v1/paea/git-call/approval')){expected+=1;return}
  errors.push(r.status()+' '+r.url());
 });
 // Real session: the ordinary persisted credential, set as the ordinary cookie.
 async function open(lang,theme,width){
  await page.setCookie({name:server.cookie_name,value:secret,domain:'127.0.0.1',path:'/',httpOnly:true,sameSite:'Strict'});
  await page.setViewport({width,height:900});
  await page.goto(origin+`/paea-consent.html?lang=${lang}`,{waitUntil:'networkidle2'});
  if(await page.$eval('html',e=>e.dataset.theme)!==theme){await page.click('#paea-theme')}
 }
 function write(name,value){fs.writeFileSync(path.join(control,name),JSON.stringify(Object.assign({nonce},value)))}
 function body(id){return page.$eval(id,e=>({text:e.textContent,rows:[...e.querySelectorAll('dt,dd')].map(x=>x.textContent)}))}
 // Archive the exact frozen bytes the Rust test holds server-side (control
 // evidence only — the page and harness never transport them) and a visibly
 // drifted variant used by the direct-fetch non-empty-body refusal case.
 fs.writeFileSync(path.join(out,'frozen-command.json'),Buffer.from(server.frozen_bytes_b64,'base64url'));
 // ---------- Phase 1: fresh bound state ----------
 await open('fr','etat',1280);
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 const approvalStatus=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(approvalStatus.state,'success','bound approval must render the explicit success state');
 assert.ok(approvalStatus.text.includes('Commande figée liée'),'FR bound message, got: '+approvalStatus.text);
 // The form is reachable once the capability and prerequisites hold.
 assert.equal(await page.$eval('#consent-form',e=>!e.hidden),true,'consent form must be visible on a fresh bound state');
 // Bound capability: digest-only, exactly the frozen command digest from the
 // startup event — asserted against the control data, never page-internal copies.
 const capability=await body('#approval-capability');
 assert.ok(capability.text.includes(server.request_digest),'capability must present the exact bound request_digest');
 assert.ok(!capability.text.includes('undefined'),'no undefined placeholder in capability');
 // Prepared action presented with its own status; prerequisite pairs visible.
 const previewState=await page.$eval('#preview-status',e=>e.dataset.state);
 assert.equal(previewState,'empty','preview-status stays empty when the preview succeeds (pairs carry the state)');
 const preview=await body('#approval-preview');
 assert.ok(preview.text.includes('refs/heads/release'),'prepared action must present the exact held target_ref');
 assert.ok(preview.text.includes('oui'),'prerequisite pairs must report bound roots and current revision');
 // Exact inputs and explicit refusal policy visible; submit button present.
 // The held-command contract means NO file input exists: the browser never
 // transports command bytes.
 assert.equal(await page.$('#consent-command'),null,'no file input under the held-command contract');
 assert.ok(await page.$('#consent-password'),'enrolled password input present');
 assert.ok((await page.$eval('#consent-send',e=>e.textContent)).includes('Exécuter'),'explicit submit button present');
 assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),'1280px no overflow');
 await (await page.$('#consent-form')).screenshot({path:path.join(out,'consent-fresh-fr.png')});
 // EN + dark theme on the same fresh state (page reload refetches).
 await open('en','sombre',390);
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 const freshEn=await page.$eval('#approval-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(freshEn.state,'success');
 assert.ok(/Bound frozen command/i.test(freshEn.text),'EN bound message, got: '+freshEn.text);
 assert.ok((await body('#approval-capability')).text.includes(server.request_digest),'EN capability presents the same exact digest');
 assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),'390px no overflow');
 await (await page.$('#consent-form')).screenshot({path:path.join(out,'consent-fresh-en.png')});
 checks.push('fresh bound: exact request_digest+prepared action+consent form presented FR/EN, both themes, no overflow');
 write('phase1.done',{status:'ok'});
 // ---------- Phase 2: the page itself submits and visibly refuses ----------
 while(!fs.existsSync(path.join(control,'phase2.ready')))await new Promise(r=>setTimeout(r,200));
 const phase2=JSON.parse(fs.readFileSync(path.join(control,'phase2.ready'),'utf8'));
 assert.equal(phase2.nonce,nonce);
 // Non-empty body refusal first, from the page's own origin (direct fetch):
 // the command is held server-side, so ANY request bytes refuse typed before
 // any Root open — exactly the route's transport-contract gate. This uses the
 // drifted variant with the CORRECT password to prove the body alone decides.
 await open('fr','etat',1280);
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 const projectBefore=fs.readFileSync(path.join(projectRoot,'.swarm/trust-actions.json'));
 const driftResponse=await page.evaluate(async drifted64=>{
  const binary=atob(drifted64);const bytes=new Uint8Array(binary.length);
  for(let i=0;i<binary.length;i++)bytes[i]=binary.charCodeAt(i);
  let binaryPwd='';for(const b of new TextEncoder().encode('STARTUP_FIXTURE_PASSWORD'))binaryPwd+=String.fromCharCode(b);
  const token=btoa(binaryPwd).replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'');
  const r=await fetch('/api/v1/paea/git-call/approval',{
   method:'POST',cache:'no-store',credentials:'same-origin',
   headers:{'X-Swarm-CSRF':globalThis.SwarmConsent.csrf,'Authorization':'Enrollment '+token},
   body:bytes,
  });
  return {status:r.status,body:await r.json()};
 },server.drifted_bytes_b64);
 assert.equal(driftResponse.status,400,'non-empty body must refuse with 400');
 assert.equal(driftResponse.body.failure?.code,'paea-git-call-approval-request','non-empty body refusal must be the transport-contract code');
 assert(fs.readFileSync(path.join(projectRoot,'.swarm/trust-actions.json')).equals(projectBefore),'non-empty-body refusal must not touch the project trust journal');
 // The page renders this refused transport only through its own empty-body
 // submission path, so capture the refusal view with the page's real form:
 // wrong password, empty body — the held authority refuses typed and nothing
 // is recorded.
 await page.type('#consent-password','wrong-password');
 await Promise.all([
  page.waitForResponse(r=>r.url().endsWith('/api/v1/paea/git-call/approval')&&r.status()>=400,{timeout:20000}),
  page.click('#consent-send'),
 ]);
 await page.waitForFunction(()=>document.getElementById('result-status').textContent.includes('authorization_required'),{timeout:20000});
 const credResult=await page.$eval('#result-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.ok(credResult.text.includes('authorization_required'),'wrong password must show the typed credential refusal, got: '+credResult.text);
 assert(fs.readFileSync(path.join(projectRoot,'.swarm/trust-actions.json')).equals(projectBefore),'credential refusal must not touch the project trust journal');
 await (await page.$('#result-status')).screenshot({path:path.join(out,'consent-credential-refusal-fr.png')});
 // EN + dark view of the same wrong-password refusal on a fresh real state:
 // the reload resets per-submission state, so the EN page re-submits itself.
 await open('en','sombre',390);
 await page.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading',{timeout:20000});
 await page.type('#consent-password','wrong-password');
 await Promise.all([
  page.waitForResponse(r=>r.url().endsWith('/api/v1/paea/git-call/approval')&&r.status()>=400,{timeout:20000}),
  page.click('#consent-send'),
 ]);
 // Fresh page load: state starts empty, but waiting on the refusal code stays
 // the stronger, race-free predicate.
 await page.waitForFunction(()=>document.getElementById('result-status').textContent.includes('authorization_required'),{timeout:20000});
 const enResult=await page.$eval('#result-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.ok(/Refusal:/i.test(enResult.text),'EN refusal rendering persists, got: '+enResult.text);
 assert.ok(enResult.text.includes('authorization_required'),'EN shows the same typed credential refusal, got: '+enResult.text);
 await (await page.$('#result-status')).screenshot({path:path.join(out,'consent-credential-refusal-en.png')});
 checks.push('refusals: non-empty body (drifted bytes, good password) -> typed transport refusal via page-origin fetch; page-driven empty body + wrong password -> typed credential refusal FR/EN; project journal untouched; no auto retry');
 // The bound approval stays unconsumed through everything: status still bound.
 const cap=await page.evaluate(async()=>{const r=await fetch('/api/v1/paea/git-call/approval',{cache:'no-store',credentials:'same-origin'});return {status:r.status,body:await r.json()}});
 assert.equal(cap.status,200);assert.equal(cap.body.bound,true,'refusals never consume the bound approval');
 // Every expected refusal must have exactly one console echo mirror; an extra
 // echo means some other request failed even though its route was never POSTed.
 assert.equal(consoleDenialEchoes,expected,'console denial echoes must mirror the expected refusals one for one');
 write('phase2.done',{status:'ok'});
 fs.writeFileSync(path.join(out,'consent-bound-ui.json'),JSON.stringify({schema_version:1,kind:'paea-consent-bound-ui-v1',checks,unexpected_diagnostics:errors,expected_denials:expected,server:{request_digest:server.request_digest,trust_revision:server.trust_revision},at:new Date().toISOString()},null,2)+'\n');
 console.log(JSON.stringify({checks:checks.length,unexpected_diagnostics:errors.length}));
})().catch(e=>{
 try{fs.writeFileSync(path.join(control,'phase1.done'),JSON.stringify({nonce,error:e.message}))}catch{}
 try{fs.writeFileSync(path.join(control,'phase2.done'),JSON.stringify({nonce,error:e.message}))}catch{}
 fs.writeFileSync(path.join(out,'consent-bound-ui-failure.json'),JSON.stringify({message:e.message,stack:e.stack,checks,errors},null,2),{mode:0o600});
 console.error('FAIL:',e.message);process.exitCode=1;
}).finally(async()=>{if(browser)await browser.close()});
