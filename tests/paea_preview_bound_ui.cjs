'use strict';
// Bounded browser qualification of the served PA-08 preview with a REAL bound
// prepared Git call, fresh then stale. Owned two-phase control protocol with
// the --ignored Rust test preview_bound_serves_real_browser_session: that test
// binds one exact prepared call to a real native HTTP server with its ordinary
// persisted session, publishes server.json, then waits for owned marker files
// (nonce-bound). This harness drives the real served page in a real Chrome:
// phase 1 asserts the EXACT held action identity/declared terms presented
// (no mock HTTP/DOM), phase 2 runs a real public Trust config-apply through
// the engine (inside the test) and re-observes the explicit stale refusal.
// No consent, no grant, no one-shot consumption, no acceptance; screenshots
// and console/request assertions stay in the owned evidence directory.
const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path'),cp=require('node:child_process');
const pup=require(path.join(process.cwd(),'node_modules','puppeteer'));
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),control=path.resolve(process.argv[4]);
fs.mkdirSync(out,{recursive:true});
const server=JSON.parse(fs.readFileSync(path.join(control,'server.json'),'utf8'));
const root=server.service_root,projectRoot=server.project_root,origin=`http://${server.address}`;
const nonce=server.nonce;
const secret=fs.readFileSync(path.join(root,'.swarm',server.session_file),'utf8').trim();
const checks=[],errors=[];let expected=0;
let browser;
(async()=>{
 browser=await pup.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
 const page=await browser.newPage();
 page.setDefaultTimeout(20000);
 page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{if(m.type()==='error'&&!m.text().includes('status of 500'))errors.push(m.text())});
 page.on('response',r=>{
  if(r.status()<400)return;
  // The guarded reconcile read legitimately refuses on this fixture service
  // (no usable settlement ledger root) and the page renders that refusal
  // visibly; same expected-denial classification as the unbound harness
  // (tests/paea_effects_ui.cjs). Browser console duplicates of already
  // classified responses carry no URL and are classified by the listener
  // above, so they never count as unexpected diagnostics.
  if(r.url().endsWith('/api/v1/paea/reconcile-effects')){expected+=1;return}
  errors.push(r.status()+' '+r.url());
 });
 // Real session: the ordinary persisted credential, set as the ordinary cookie.
 async function open(lang,theme,width){
  await page.setCookie({name:server.cookie_name,value:secret,domain:'127.0.0.1',path:'/',httpOnly:true,sameSite:'Strict'});
  await page.setViewport({width,height:900});
  await page.goto(origin+`/paea-effects.html?lang=${lang}`,{waitUntil:'networkidle2'});
  if(await page.$eval('html',e=>e.dataset.theme)!==theme){await page.click('#paea-theme')}
 }
 function write(name,value){fs.writeFileSync(path.join(control,name),JSON.stringify(Object.assign({nonce},value)))}
 function body(id){return page.$eval(id,e=>({text:e.textContent,html:e.innerHTML,rows:[...e.querySelectorAll('dt,dd')].map(x=>x.textContent)}))}
 // ---------- Phase 1: fresh bound state ----------
 await open('fr','etat',1280);
 await page.waitForFunction(()=>document.getElementById('preview-status').dataset.state!=='loading',{timeout:20000});
 const freshStatus=await page.$eval('#preview-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(freshStatus.state,'success','fresh bound preview must render the explicit success state');
 assert.ok(freshStatus.text.includes('Aperçu exact'), 'FR exact-preview message, got: '+freshStatus.text);
 // Exact identity from the action held by the engine — asserted against the
 // startup event digests, never against page-internal copies.
 const identity=await body('#preview-identity');
 assert.ok(identity.text.includes(server.action_digest),'identity must present the exact bound action_digest');
 assert.ok(!identity.text.includes('undefined'),'no undefined placeholder in identity');
 // Witness section: both roots bound, current revision equals declared.
 const witness=await body('#preview-witness');
 assert.ok(witness.text.includes('oui'),'witness must report roots bound and revision current');
 // Declared terms: the real held proposal (target ref from the real fixture
 // preparation) and explicit limits, verbatim from the API payload.
 const declared=await body('#preview-declared');
 assert.ok(declared.text.includes('refs/heads/release'),'declared target_ref must be the exact held proposal value, got rows: '+JSON.stringify(declared.rows));
 const limits=await body('#preview-limits-list');
 assert.ok(limits.text.includes('none-captured'),'explicit no-consent limit must be visible');
 assert.ok(!await page.$eval('#preview-detail',e=>[...e.querySelectorAll('button')].length),'no consent control anywhere in the presented detail');
 assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),'1280px no overflow');
 await (await page.$('#preview-detail')).screenshot({path:path.join(out,'preview-fresh-fr.png')});
 // EN + dark theme on the same fresh state (page reload refetches).
 await open('en','sombre',390);
 await page.waitForFunction(()=>document.getElementById('preview-status').dataset.state!=='loading',{timeout:20000});
 const freshEn=await page.$eval('#preview-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(freshEn.state,'success');
 assert.ok(/Exact preview of the engine-held prepared action/i.test(freshEn.text),'EN exact-preview message, got: '+freshEn.text);
 assert.ok((await body('#preview-identity')).text.includes(server.action_digest),'EN identity presents the same exact digest');
 assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),'390px no overflow');
 await (await page.$('#preview-detail')).screenshot({path:path.join(out,'preview-fresh-en.png')});
 checks.push('fresh bound: exact digest+declared terms presented FR/EN, both themes, no consent control, no overflow');
 // Direct API cross-check through the page context: the presented values come
 // from the real endpoint payload (same values asserted twice, two surfaces).
 const api=await page.evaluate(async()=>{const r=await fetch('/api/v1/paea/git-call/preview',{cache:'no-store',credentials:'same-origin'});return {status:r.status,body:await r.json()}});
 assert.equal(api.status,200);assert.equal(api.body.available,true);
 assert.equal(api.body.canonical.action_digest,server.action_digest);
 assert.equal(api.body.canonical.digest_matches,true);
 assert.equal(api.body.current.roots_bound,true);
 assert.equal(api.body.current.trust_revision_current,true);
 assert.equal(api.body.limits.consent,'none-captured');
 write('phase1.done',{status:'ok'});
 // ---------- Phase 2: real public Trust revision change, stale state ----------
 while(!fs.existsSync(path.join(control,'phase2.ready')))await new Promise(r=>setTimeout(r,200));
 const phase2=JSON.parse(fs.readFileSync(path.join(control,'phase2.ready'),'utf8'));
 assert.equal(phase2.nonce,nonce);
 await open('fr','etat',1280);
 await page.waitForFunction(()=>document.getElementById('preview-status').dataset.state!=='loading',{timeout:20000});
 const staleStatus=await page.$eval('#preview-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(staleStatus.state,'error','stale preparation must stay a visible error, never a success');
 assert.ok(staleStatus.text.includes('périmée'),'FR stale message must name the stale state, got: '+staleStatus.text);
 const staleWitness=await body('#preview-witness');
 assert.ok(staleWitness.text.includes('non'),'stale witness must report revision-current = non');
 // No declared-terms success block and no consent control in the stale state.
 assert.ok(!await page.$eval('#preview-detail',e=>[...e.querySelectorAll('button')].length),'stale state still presents no consent control');
 await (await page.$('#preview-detail')).screenshot({path:path.join(out,'preview-stale-fr.png')});
 await open('en','sombre',390);
 await page.waitForFunction(()=>document.getElementById('preview-status').dataset.state!=='loading',{timeout:20000});
 const staleEn=await page.$eval('#preview-status',e=>({state:e.dataset.state,text:e.textContent}));
 assert.equal(staleEn.state,'error');
 assert.ok(/stale/i.test(staleEn.text),'EN stale message, got: '+staleEn.text);
 await (await page.$('#preview-detail')).screenshot({path:path.join(out,'preview-stale-en.png')});
 checks.push('stale: real Trust revision change → visible error FR/EN, no normalization, no consent');
 // One-shot call still unconsumed after everything: status stays bound.
 const cap=await page.evaluate(async()=>{const r=await fetch('/api/v1/paea/git-call',{cache:'no-store',credentials:'same-origin'});return {status:r.status,body:await r.json()}});
 assert.equal(cap.status,200);assert.equal(cap.body.bound,true,'preview never consumes the one-shot call');
 write('phase2.done',{status:'ok'});
 fs.writeFileSync(path.join(out,'preview-bound-ui.json'),JSON.stringify({schema_version:1,kind:'paea-preview-bound-ui-v1',checks,unexpected_diagnostics:errors,expected_denials:expected,server:{action_digest:server.action_digest,trust_revision_fresh:server.trust_revision,trust_revision_stale:phase2.trust_revision},at:new Date().toISOString()},null,2)+'\n');
 console.log(JSON.stringify({checks:checks.length,unexpected_diagnostics:errors.length}));
})().catch(e=>{
 try{fs.writeFileSync(path.join(control,'phase1.done'),JSON.stringify({nonce,error:e.message}))}catch{}
 fs.writeFileSync(path.join(out,'preview-bound-ui-failure.json'),JSON.stringify({message:e.message,stack:e.stack,checks,errors},null,2),{mode:0o600});
 console.error('FAIL:',e.message);process.exitCode=1;
}).finally(async()=>{if(browser)await browser.close()});
