'use strict';
/** Read-only acceptance on the identified managed local instance. No business POST,
 * live database fixture, key output or provider call. Version screenshots crop
 * the public dialog; the private browser profile stays in ignored local proof.
 * Requires SWARM_PAEA_LIVE_STATUS from swarm.sh status and a fresh
 * SWARM_PAEA_LIVE_EVIDENCE. No global watchdog or implicit restart. Safe failures
 * propagate; unexpected network/console/page failures abort pending DOM waits
 * through an event-driven rejection, with no polling or arbitrary deadline.
 * The recipe retains diagnostics, and the browser is closed in finally.
 * Verifies current FR/EN notes, both linked data pages and nineteen asset byte sets;
 * empty comparison/invoice fields are checked without submitting any live data.
 * --fault-version-script blocks only this browser's runtime-health.js GET and
 * must fail; the recipe runs a valid control first. No server is interrupted.
 */
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const fault=process.argv.length===3&&process.argv[2]==='--fault-version-script';assert(process.argv.length===2||fault);
const cfg=JSON.parse(fs.readFileSync(process.env.SWARM_PAEA_LIVE_STATUS,'utf8')),out=process.env.SWARM_PAEA_LIVE_EVIDENCE+(fault?'-fault-script':''),root=process.cwd(),notes=JSON.parse(fs.readFileSync(path.join(root,'version_history.json'),'utf8')).local_changes;fs.mkdirSync(out);
const sha=x=>crypto.createHash('sha256').update(x).digest('hex'),url=new URL(cfg.url),address=url.host,checks=[],errors=[];
assert(url.protocol==='http:'&&url.hostname==='127.0.0.1'&&url.port);assert.equal(sha(fs.readFileSync('/proc/'+cfg.pid+'/exe')),cfg.binary_sha256);
const key=path.join(cfg.root,'.swarm','web-session-'+sha(address).slice(0,16)),fd=fs.openSync(key,fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW);let token,browser;
try{const info=fs.fstatSync(fd);assert(info.isFile()&&info.uid===process.getuid()&&(info.mode&0o077)===0);token=fs.readFileSync(fd,'utf8').trim()}finally{fs.closeSync(fd)}
assert(/^session-[A-Za-z0-9_-]{24}$/.test(token));
/** Assert observed content with a unique name, never log a session or live work. */
function check(name,value){assert(value,name);assert(!checks.includes(name));checks.push(name);console.log('PASS '+name)}
(async()=>{let failure,abort;
const observedFailure=new Promise((resolve,reject)=>{abort=reject});
/** Preserve an unexpected browser cause and interrupt a pending DOM wait. No retry. */
function browserFailure(message){errors.push(message);abort(new Error('unexpected-browser-diagnostic'))}
try{await Promise.race([(async()=>{
 browser=await puppeteer.launch({headless:true,executablePath:'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.resolve(out,'browser')});
 for(const lang of ['fr','en']){
  const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width:1440,height:1000});
  await context.setCookie({name:'swarm_session_'+sha(address).slice(0,12),value:token,url:cfg.url,httpOnly:true,sameSite:'Strict'});
  page.on('pageerror',e=>browserFailure(e.message));page.on('console',m=>{if(m.type()==='error')browserFailure(m.text())});page.on('requestfailed',r=>browserFailure('failed:'+new URL(r.url()).pathname));page.on('response',r=>{if(r.status()>=400)browserFailure('HTTP'+r.status()+':'+new URL(r.url()).pathname)});
  await page.setRequestInterception(true);page.on('request',request=>{if(!['GET','HEAD'].includes(request.method())){browserFailure('refused non-read request:'+new URL(request.url()).pathname);request.abort().catch(e=>browserFailure('request-abort:'+e.message))}else if(fault&&new URL(request.url()).pathname==='/runtime-health.js'){console.log('INJECTED_VERSION_SCRIPT_FAILURE');request.abort('failed').catch(e=>browserFailure('request-abort:'+e.message))}else request.continue().catch(e=>browserFailure('request-continue:'+e.message))});
  const response=await page.goto(cfg.url+'paea.html?'+new URLSearchParams({lang,theme:lang==='fr'?'etat':'sombre'}),{waitUntil:'domcontentloaded'});check(lang+'-authenticated-PAEA-page',response.status()===200&&new URL(page.url()).pathname==='/paea.html');
  await page.waitForFunction(()=>['empty','error'].includes(document.getElementById('paea-status')?.dataset.state));check(lang+'-live-session-ready',await page.$eval('#paea-check',e=>!e.disabled));
  check(lang+'-live-receipt-example-links',await page.$eval('#paea-example-help',(e,data)=>!e.open&&e.querySelectorAll('a[download]').length===data.count&&e.querySelector('summary').textContent===(data.lang==='en'?'Download fictional examples':'Exemples fictifs à télécharger'),{lang,count:4}));
  const assets=await page.evaluate(async()=>{const data={};for(const p of ['/paea.html','/paea-payment.html','/paea-compare.html','/paea.js','/paea.css','/i18n-en.js','/i18n.js','/swarm-design.css','/wattson_themes.css','/swarm-logo.png','/trust-native.css','/paea-examples/v1/git-after.json','/paea-examples/v1/git-document.json','/paea-examples/v1/git-receipt.json','/paea-examples/v1/payment-after.json','/paea-examples/v1/payment-document.json','/paea-examples/v1/payment-receipt.json','/paea-examples/v1/payment-registry.json','/paea-examples/v1/payment-request.json']){const r=await fetch(p);if(!r.ok)throw Error('asset refused');data[p]=Array.from(new Uint8Array(await r.arrayBuffer()))}return data});
  check(lang+'-live-protected-asset-bytes',Object.entries(assets).every(([p,bytes])=>sha(Buffer.from(bytes))===sha(fs.readFileSync(path.join(root,'web',p==='/trust-native.css'?'trust-native.css':p.slice(1))))));
  const version=await page.evaluate(async()=>{const r=await fetch('/api/v1/native-version');if(!r.ok)throw Error('version refused');return r.json()});check(lang+'-live-version-identity',version.binary.version===cfg.binary.version&&version.binary.commit===cfg.binary.commit&&version.binary.modified===cfg.binary.modified);check(lang+'-current-evolution-notes',require('node:util').isDeepStrictEqual(version.history.local_changes,notes));
  await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click('a[href^="/paea-compare.html"]')]);await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
  check(lang+'-live-comparison-menu-and-fields',new URL(page.url()).pathname==='/paea-compare.html'&&await page.$eval('#paea-before',e=>e.type==='file'&&e.files.length===0)&&await page.$eval('#paea-after',e=>e.type==='file'&&e.files.length===0));
  check(lang+'-live-comparison-example-links',await page.$eval('#paea-example-help',(e,data)=>!e.open&&e.querySelectorAll('a[download]').length===data.count&&e.querySelector('summary').textContent===(data.lang==='en'?'Download fictional examples':'Exemples fictifs à télécharger'),{lang,count:4}));
  check(lang+'-live-comparison-limits-no-submission',await page.$eval('#paea-limits',(e,l)=>e.textContent.includes(l==='en'?'validates no approval':'ne valide aucun accord'),lang)&&await page.$eval('#paea-download',e=>e.disabled));
  await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click('a[data-paea-link]')]);await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
  await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click('a[data-paea-link]')]);await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
  check(lang+'-live-payment-menu-and-fields',new URL(page.url()).pathname==='/paea-payment.html'&&await page.$eval('#paea-registry',e=>e.type==='file'&&e.files.length===0)&&await page.$eval('#paea-request',e=>e.type==='file'&&e.files.length===0));
  check(lang+'-live-payment-example-links',await page.$eval('#paea-example-help',(e,data)=>!e.open&&e.querySelectorAll('a[download]').length===data.count&&e.querySelector('summary').textContent===(data.lang==='en'?'Download fictional examples':'Exemples fictifs à télécharger'),{lang,count:2}));
  check(lang+'-live-payment-limits-no-submission',await page.$eval('#paea-limits',(e,l)=>e.textContent.includes(l==='en'?'unauthenticated':'pas authentifié'),lang)&&await page.$eval('#paea-download',e=>e.disabled));
  await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click('nav a[href="/"]')]);await page.waitForSelector('a[href="/paea.html"]');check(lang+'-live-cockpit-menu',true);
  if(!await page.$eval('.rail-settings',e=>e.open))await page.click('.rail-settings > summary');await page.waitForFunction(()=>!document.querySelector('.rail-settings').getAnimations({subtree:true}).some(a=>a.playState==='running'&&Number.isFinite(a.effect?.getComputedTiming().endTime)));
  const theme=lang==='fr'?'etat':'sombre';if(await page.$eval('html',e=>e.dataset.theme)!==theme)await page.click('#theme');check(lang+'-dialog-theme',await page.$eval('html',(e,t)=>e.dataset.theme===t,theme));
  await page.click('#version-open');await page.waitForSelector('#version-dialog[open]');await page.waitForFunction(v=>document.getElementById('version-dialog').textContent.includes(v),{},cfg.binary.version);
  check(lang+'-live-dialog-current-version',await page.$eval('#version-dialog',(e,v)=>e.textContent.includes(v)&&e.textContent.includes('PAEA'),cfg.binary.version));
  check(lang+'-dialog-current-PAEA-evolutions',await page.$eval('#version-dialog',(e,data)=>data.items.filter(v=>v.fr.startsWith('PAEA :')).every(v=>e.textContent.includes(v[data.lang])),{items:notes.items,lang}));
  check(lang+'-live-dialog-commit-link',await page.$$eval('#version-dialog a',(els,c)=>els.some(e=>e.href.endsWith('/commit/'+c)),cfg.binary.commit));
  check(lang+'-explicit-uncommitted-disclosure',!cfg.binary.modified||await page.$eval('#version-dialog',(e,l)=>e.textContent.toLowerCase().includes(l==='en'?'uncommitted':'non commit'),lang));
  await (await page.$('#version-dialog')).screenshot({path:path.join(out,lang+'-version-dialog.png')});
  await page.evaluate(()=>{globalThis.versionClosed=new Promise(resolve=>document.getElementById('version-dialog').addEventListener('close',resolve,{once:true}))});await page.keyboard.press('Escape');await page.evaluate(()=>globalThis.versionClosed);check(lang+'-live-dialog-focus',await page.evaluate(()=>document.activeElement.id==='version-open'));await context.close();
 }
 check('managed-process-stable',sha(fs.readFileSync('/proc/'+cfg.pid+'/exe'))===cfg.binary_sha256);check('no-browser-errors-or-writes',errors.length===0);
 })(),observedFailure]);}catch(e){failure=e;console.error(e.stack)}finally{try{if(browser)await browser.close()}catch(e){errors.push('browser-close:'+e.message);failure??=e}fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({scope:'read-only live native assets and Version dialog; no mission database used as fixture',version:cfg.binary,pid:cfg.pid,binary_sha256:cfg.binary_sha256,checks,errors,success:!failure},null,2))}
 if(failure)process.exitCode=1;else console.log('LIVE_PAEA_DELIVERY_VERIFIED '+checks.length);
})();
