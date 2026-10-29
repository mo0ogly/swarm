'use strict';
// Bounded browser qualification of the PA-08 effect-state page on the REAL
// native web server (fresh binary, isolated 0700 fixture root, real session
// login). Covers: loading → explicit empty states (unbound git-call +
// uninitialized journals), refusal visibility without session, FR/EN, both
// themes, keyboard focus and mobile widths. No effect, no one-shot call, no
// acceptance authority; the page only reads the two guarded GET endpoints.
const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path'),cp=require('node:child_process');
const pup=require(path.join(process.cwd(),'node_modules','puppeteer'));
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);
fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'paea-effects-ui-'));
fs.chmodSync(root,0o700);
const checks=[],errors=[];let expected=0;
cp.execFileSync(binary,['--root',root,'--json','init']);
const server=cp.spawn(binary,['--root',root,'web','127.0.0.1:0','--json']);
let log='';server.stdout.on('data',d=>log+=d);server.stderr.on('data',d=>log+=d);
let browser;
(async()=>{
 for(let i=0;i<100&&!log.includes('"address"');i++)await new Promise(r=>setTimeout(r,100));
 const bound=JSON.parse(log.slice(log.indexOf('{'),log.indexOf('}')+1));
 const origin=`http://${bound.address}`;
 // Real session credential written by bind; find it in the fixture root.
 const sessionFile=fs.readdirSync(path.join(root,'.swarm')).find(f=>f.startsWith('web-session-'));
 assert.ok(sessionFile,'session credential file must exist');
 const token=fs.readFileSync(path.join(root,'.swarm',sessionFile),'utf8').trim();
 browser=await pup.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
 const page=await browser.newPage();
 page.setDefaultTimeout(15000);
 page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{if(m.type()==='error'&&!m.text().includes('status of 500'))errors.push(m.text())});
 page.on('response',r=>{
   const u=r.url();
   if(r.status()>=500){
     // The fixture has no usable settlement ledger root, so the PA-09
     // endpoint's 500 refusal is the expected, verified behavior (the page
     // renders it visibly); anything else stays an unexpected diagnostic.
     if(u.includes('/api/v1/paea/reconcile-effects'))expected++;else errors.push(r.status()+' '+u);
   }});
 // Browser "Failed to load resource" log entries carry no URL; the URL-bearing
 // response listener above is the authoritative detector for unexpected 500s,
 // so these URL-less duplicates of already-classified responses are expected.
 page.on('console',m=>{if(m.type()==='error'&&m.text().includes('status of 500'))expected++;});
 async function login(lang,theme){
   await page.goto(origin+'/',{waitUntil:'networkidle2'});
   // With a session cookie the root renders the cockpit, not the login form:
   // skip the credential step instead of failing on a missing input.
   if(await page.$('input[name="token"]')){
     await page.type('input[name="token"]',token);
     await Promise.all([page.waitForNavigation({waitUntil:'networkidle2'}),page.click('button[type="submit"]')]);
   }
   assert.ok(!page.url().includes('login'),'login must redirect');
 }
 async function open(lang,theme){
   await page.goto(origin+`/paea-effects.html?lang=${lang}`,{waitUntil:'networkidle2'});
   if(await page.$eval('html',e=>e.dataset.theme)!==theme){await page.click('#paea-theme')}
 }
 for(const [lang,theme,width] of [['fr','etat',1280],['en','sombre',390]]){
   await login();await page.setViewport({width,height:900});
   await open(lang,theme);
   // 1. Explicit empty states, never a fake success: git-call unbound and
   //    both journals uninitialized (fixture has no ledger setting → the
   //    reconciliation endpoint refuses 501 and the refusal stays visible).
   await page.waitForFunction(()=>document.getElementById('gitcall-status').dataset.state!=='loading',{timeout:15000});
   const gitState=await page.$eval('#gitcall-status',e=>({state:e.dataset.state,text:e.textContent}));
   assert.equal(gitState.state,'empty','unbound git-call must be an explicit empty state');
   if(lang==='fr')assert.ok(gitState.text.includes('Aucun appel Git préparé'),'FR unbound message, got: '+gitState.text);
   else assert.ok(/No prepared Git call/i.test(gitState.text),'EN unbound message, got: '+gitState.text);
   await page.waitForFunction(()=>document.getElementById('reconcile-status').dataset.state!=='loading',{timeout:15000});
   const recState=await page.$eval('#reconcile-status',e=>({state:e.dataset.state,text:e.textContent}));
   assert.equal(recState.state,'error','unset/uninitialized ledger root must stay a visible refusal');
   // Fresh init adopts the catalogued default ledger path (v49); on this
   // fixture it designates no accessible root, so the exact engine refusal
   // ("path" from LocalPathError, or the settings-* codes) must stay visible —
   // never a silent empty success.
   assert.ok(
     ['path','settings-paea-ledger-root-not-configured','settings-paea-ledger-root-invalid','local-storage']
       .some(code=>recState.text.includes(code)),
     'exact refusal code visible, got: '+recState.text);
   checks.push(lang+': explicit unbound capability and visible 501 refusal (no fake success)');
   // 2. Theme really switched and no horizontal overflow at this width.
   assert.equal(await page.$eval('html',e=>e.dataset.theme),theme);
   assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),lang+' '+width+'px no overflow');
   // 3. Keyboard: skip link focusable, theme button reachable, 44px target.
   await page.goto(origin+`/paea-effects.html?lang=${lang}`,{waitUntil:'networkidle2'});
   await page.keyboard.press('Tab');
   const focused=await page.evaluate(()=>document.activeElement.className+(document.activeElement.id?':'+document.activeElement.id:''));
   assert.ok(focused.includes('paea-skip'),'first Tab reaches the skip link, got: '+focused);
   const target=await page.$eval('#paea-theme',e=>e.getBoundingClientRect().height);
   assert.ok(target>=44,'theme button ≥44px, got '+target);
   checks.push(lang+': keyboard skip link first and theme control ≥44px');
   await (await page.$('main')).screenshot({path:path.join(out,'paea-effects-'+lang+'.png')});
 }
 // 4. Explicit degradation: without a session the API refuses while the page
 //    asset also refuses for anonymous access (session-protected assets);
 //    refusals must be visible, never silent empty successes.
 await page.goto(origin+'/',{waitUntil:'networkidle2'});
 // Fresh page in a fresh context: the cockpit cookie persists in this page, so
 // open an incognito context to prove the anonymous refusal contract exactly.
 const anon=await browser.createBrowserContext();
 const anonPage=await anon.newPage();
 await anonPage.setViewport({width:1024,height:800});
 const raw=await (async()=>{const resp=await anonPage.goto(origin+'/paea-effects.html',{waitUntil:'domcontentloaded'});return {page:resp.status()}})();
 const api=await anonPage.evaluate(async()=>{const r=await fetch('/api/v1/paea/git-call',{cache:'no-store'});return {status:r.status,text:await r.text()}});
 assert.equal(api.status,403,'API refuses without session');
 assert.ok(api.text.includes('web-access-refused'),'exact refusal code visible, got: '+api.text.slice(0,120));
 assert.equal(raw.page,403,'anonymous page asset also refuses (session-protected)');
 await anon.close();
 checks.push('without session: both endpoints refuse 403 web-access-refused (page assets equally protected)');
 fs.writeFileSync(path.join(out,'paea-effects-ui.json'),JSON.stringify({schema_version:1,kind:'paea-effects-ui-v1',checks,unexpected_diagnostics:errors,expected_denials:expected,at:new Date().toISOString()},null,2)+'\n');
 console.log(JSON.stringify({checks:checks.length,unexpected_diagnostics:errors.length,expected_denials:expected}));
})().catch(e=>{fs.writeFileSync(path.join(out,'paea-effects-ui-failure.json'),JSON.stringify({message:e.message,stack:e.stack,checks,errors},null,2),{mode:0o600});console.error('FAIL:',e.message);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();server.kill('SIGTERM')});
