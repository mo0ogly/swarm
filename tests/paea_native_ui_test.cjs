'use strict';
/** Exercise the actual default-profile native executable and embedded PAEA UI.
 * Fresh isolated Root only. Native success/refusal controls; browser-only corrupt
 * reply injections are separately named. No provider/live mission mutation.
 * The caller supplies an exact existing binary/hash and a fresh evidence path.
 * No build, retry or watchdog; readiness is observed from the real listener/UI.
 */
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {spawn}=require('node:child_process'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const output=process.argv[2]||process.env.SWARM_PAEA_UI_EVIDENCE,binary=process.env.SWARM_PAEA_TEST_BINARY,expected=process.env.SWARM_PAEA_TEST_BINARY_SHA256;
const faultMode=process.env.SWARM_PAEA_NATIVE_FAULT;assert([undefined,'script'].includes(faultMode));
assert(output&&process.argv.length<=3&&binary&&/^[a-f0-9]{64}$/.test(expected||''),'explicit binary/hash/fresh output required');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
assert.equal(sha(fs.readFileSync(binary)),expected);fs.mkdirSync(output);
const root=path.resolve(output,'root');fs.mkdirSync(root);
const vectors=JSON.parse(fs.readFileSync('tests/fixtures/paea/contract-v1.json')).vectors;
const checks=[],errors=[],responses=[],cases=[],paymentCases=[],serverErrors=[];let browser,server,closed,token='',address='',expectRefusal=false,abort,stopping=false;
const observedFailure=new Promise((resolve,reject)=>{abort=reject});
/** Preserve unexpected browser failures and reject pending DOM waits without a
 * deadline. Expected native 400 refusals remain separately recorded. */
function browserFailure(value){errors.push(value);abort(Error('unexpected-native-browser-diagnostic'))}
/** Retain uniquely named observable assertions; zero-case runs cannot pass. */
function check(name,value){assert(value,name);assert(!checks.includes(name));checks.push(name);console.log('PASS '+name)}
/** Capture a finite supported CLI operation without shell, truncation or timeout. */
async function command(args){
 const child=spawn(binary,args,{stdio:['ignore','pipe','pipe']}),out=[],err=[];
 child.stdout.on('data',b=>out.push(b));child.stderr.on('data',b=>err.push(b));
 const result=await new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',(code,signal)=>resolve({code,signal}))});
 assert.equal(result.signal,null);assert.equal(result.code,0,Buffer.concat(err).toString());return Buffer.concat(out);
}
/** Start only the owned fixture server; early exit rejects listener readiness. */
async function start(){
 await command(['--root',root,'init']);
 server=spawn(binary,['--root',root,'--json','web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
 closed=new Promise(resolve=>server.once('close',(code,signal)=>{resolve({code,signal});if(!stopping)browserFailure({kind:'owned-server-exit',code,signal})}));
 let buffer='';server.stderr.on('data',b=>serverErrors.push(b));
 const line=await new Promise((resolve,reject)=>{
  server.once('error',reject);server.once('close',()=>reject(Error('native listener exited before announcement')));
  server.stdout.on('data',b=>{buffer+=b.toString();const n=buffer.indexOf('\n');if(n>=0)resolve(buffer.slice(0,n))});
 });
 address=JSON.parse(line).address;assert(/^127\.0\.0\.1:[1-9][0-9]*$/.test(address));
 assert.equal(sha(fs.readFileSync('/proc/'+server.pid+'/exe')),expected);
 const key=path.join(root,'.swarm','web-session-'+sha(address).slice(0,16));
 const fd=fs.openSync(key,fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW);
 try{const info=fs.fstatSync(fd);assert(info.isFile()&&info.uid===process.getuid()&&(info.mode&0o077)===0);token=fs.readFileSync(fd,'utf8').trim()}finally{fs.closeSync(fd)}
 assert(/^session-[A-Za-z0-9_-]{24}$/.test(token));
}
/** Upload exact fixture bytes and choose explicit sink/state via normal controls. */
async function fill(page,document,receipt){
 await (await page.$('#paea-document')).uploadFile(document);await (await page.$('#paea-receipt')).uploadFile(receipt);
 await page.$eval('#paea-sink',e=>{e.value='sink-native-fixture';e.dispatchEvent(new Event('input',{bubbles:true}))});
 await page.select('#paea-state','COMMIT_REQUESTED');
}
/** Observe a real native response and resulting DOM; expected refusal is narrowly scoped. */
async function submit(page,status){
 expectRefusal=status===400;
 const response=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/receipt-report'&&r.request().method()==='POST');
 await page.click('#paea-check');const actual=await response;assert.equal(actual.status(),status);
 const value=await actual.json();await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 if(status===200)await page.waitForFunction(()=>!document.getElementById('paea-download').disabled);
 else await page.waitForFunction(()=>document.getElementById('paea-status').dataset.state==='error');
 expectRefusal=false;return value;
}
/** Upload only named exact UTF-8 registry/request fixture files. */
async function fillPayment(page,registry,request){
 await (await page.$('#paea-registry')).uploadFile(registry);await (await page.$('#paea-request')).uploadFile(request);
}
/** Exercise the real native resolver, retaining raw i64 JSON report bytes. The
 * response body is never decoded with a lossy test-side JSON number conversion. */
async function submitPayment(page,status){
 expectRefusal=status===400;
 const pending=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/resolve-payment'&&r.request().method()==='POST');
 await page.click('#paea-check');const response=await pending;assert.equal(response.status(),status);
 const raw=await response.text();await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 await page.waitForFunction(expected=>document.getElementById('paea-status').dataset.state===expected,{},status===200?'success':'error');
 expectRefusal=false;return raw;
}
/** Complete menu-to-resolution journey with exact numbers, native rejection,
 * recovery, keyboard and a real completed download. Data remain unauthenticated. */
async function paymentCase(page,context,label,lang,theme){
 await page.goto('http://'+address+'/paea.html?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
 const link='a[data-paea-link]';await page.waitForSelector(link);
 await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click(link)]);
 await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 check(label+'-payment-menu-preserves-presentation',await page.evaluate((l,t)=>location.pathname==='/paea-payment.html'&&document.documentElement.lang===l&&document.documentElement.dataset.theme===t,lang,theme));
 check(label+'-payment-primary-translated',await page.$eval('#paea-check',(e,l)=>e.textContent.trim()===(l==='en'?'Resolve reference':'Résoudre la référence'),lang));
 check(label+'-payment-limits-and-empty',await page.$eval('#paea-limits',(e,l)=>e.textContent.includes(l==='en'?'unauthenticated':'pas authentifié'),lang)&&await page.$eval('#paea-download',e=>e.disabled));
 const embedded=await page.evaluate(async()=>{const r=await fetch('/paea-payment.html');if(!r.ok)throw Error('payment asset unavailable');return Array.from(new Uint8Array(await r.arrayBuffer()))});check(label+'-payment-embedded-page-bytes',sha(Buffer.from(embedded))===sha(fs.readFileSync('web/paea-payment.html')));
 check(label+'-payment-help-collapsed',await page.$eval('#paea-format-help',e=>!e.open));
 await page.focus('#paea-format-help summary');await page.keyboard.press('Enter');check(label+'-payment-help-keyboard-translated',await page.$eval('#paea-format-help',(e,l)=>e.open&&e.textContent.includes(l==='en'?'same project':'même projet'),lang));await page.keyboard.press('Enter');
 const max='9223372036854775807',directory=path.resolve(output,label+'-resolution');fs.mkdirSync(directory);
 const registry=path.join(directory,'registry.json'),request=path.join(directory,'request.json'),bad=path.join(directory,'wrong-project.json');
 const reg=JSON.parse(fs.readFileSync('tests/fixtures/paea/payment-registry-v1.json','utf8')),req=JSON.parse(fs.readFileSync('tests/fixtures/paea/payment-request-v1.json','utf8'));
 reg.debts[0].amount_minor=max;
 const rawRegistry=JSON.stringify(reg).replace('"revision":1','"revision":'+max),rawRequest=JSON.stringify(req).replace('"expected_registry_revision":1','"expected_registry_revision":'+max);
 assert(rawRegistry.includes('"revision":'+max)&&rawRequest.includes('"expected_registry_revision":'+max));
 fs.writeFileSync(registry,rawRegistry);fs.writeFileSync(request,rawRequest);fs.writeFileSync(bad,rawRequest.replace('project-alpha','wrong-project'));
 await fillPayment(page,registry,request);const report=await submitPayment(page,200);fs.writeFileSync(path.join(directory,'native-report.json'),report);
 check(label+'-payment-exact-revision-amount',report.includes('"registry_revision":'+max)&&report.includes('"amount_minor":"'+max+'"')&&await page.$eval('#paea-identities',(e,n)=>e.textContent.split(n).length===3,max));
 check(label+'-payment-input-hashes',report.includes(sha(rawRegistry))&&report.includes(sha(rawRequest))&&await page.$eval('#paea-report',(e,r)=>e.textContent===r,report));
 check(label+'-payment-debt-and-no-authority',report.includes('"invoice_id":"debt-17"')&&report.includes('"registry_authenticity":"not-established"')&&report.includes('"effect":"not-established"'));
 await page.$eval('#paea-check',e=>e.focus());await page.keyboard.press('Tab');check(label+'-payment-download-keyboard',await page.evaluate(()=>document.activeElement.id==='paea-download'));
 const cdp=await page.createCDPSession();assert.equal(typeof context.id,'string');
 await cdp.send('Browser.setDownloadBehavior',{behavior:'allow',downloadPath:directory,browserContextId:context.id,eventsEnabled:true});
 const downloaded=new Promise((resolve,reject)=>cdp.on('Browser.downloadProgress',event=>{if(event.state==='completed')resolve();if(event.state==='canceled')reject(Error('payment-download-canceled'))}));
 await page.click('#paea-download');await downloaded;await cdp.detach();
 check(label+'-payment-real-download-exact',fs.readFileSync(path.join(directory,'paea-payment-resolution.json'),'utf8')===report);
 await fillPayment(page,registry,bad);check(label+'-payment-edit-clears-stale',await page.$eval('#paea-report',e=>e.textContent==='')&&await page.$eval('#paea-download',e=>e.disabled));
 const refused=await submitPayment(page,400);check(label+'-payment-native-refusal',JSON.parse(refused).failure.code==='protection-registry-project-mismatch'&&await page.$eval('#paea-download',e=>e.disabled));
 await fillPayment(page,registry,request);check(label+'-payment-native-recovery',(await submitPayment(page,200))===report);
 check(label+'-payment-no-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
 const colors=await page.$eval('#paea-check',e=>{const s=getComputedStyle(e);return [s.color,s.backgroundColor]});
 const luminance=color=>color.match(/[\d.]+/g).slice(0,3).map(Number).map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((v,c,i)=>v+c*[.2126,.7152,.0722][i],0);
 const colorsL=colors.map(luminance);check(label+'-payment-primary-contrast-AA',(Math.max(...colorsL)+.05)/(Math.min(...colorsL)+.05)>=4.5);
 await page.screenshot({path:path.join(directory,'result.png'),fullPage:true});paymentCases.push(label);
}
/** Browser-only faults follow real native controls. Response substitution tests
 * the client boundary only; these synthetic reports never count as sink evidence.
 * Hold/release uses an observed request event, no sleep/watchdog or retry. */
async function paymentClientFaults(page,label){
 const directory=path.resolve(output,label+'-resolution'),registry=path.join(directory,'registry.json'),request=path.join(directory,'request.json');
 const good=fs.readFileSync(path.join(directory,'native-report.json'),'utf8'),max='9223372036854775807';
 let replacement=null,held,arrived,hold=false,posts=0;
 await page.setRequestInterception(true);
 const intercept=r=>{
  if(new URL(r.url()).pathname!=='/api/v1/protection/resolve-payment'||r.method()!=='POST'){r.continue().catch(e=>browserFailure({kind:'interception',message:e.message}));return;}
  posts++;
  if(hold){held=()=>r.continue().catch(e=>browserFailure({kind:'hold-release',message:e.message}));arrived();return;}
  if(replacement!==null)r.respond({status:200,contentType:'application/json',body:replacement}).catch(e=>browserFailure({kind:'fixture-response',message:e.message}));
  else r.continue().catch(e=>browserFailure({kind:'interception',message:e.message}));
 };
 page.on('request',intercept);
 for(const [name,raw] of [
  ['hash',good.replace(/"request_sha256":"[a-f0-9]{64}"/,'"request_sha256":"'+'0'.repeat(64)+'"')],
  ['authority',good.replace('"authority":"not-established"','"authority":"established"')],
  ['revision-string',good.replace('"registry_revision":'+max,'"registry_revision":"'+max+'"')],
  ['revision-exponent',good.replace('"registry_revision":'+max,'"registry_revision":1e3')],
  ['numeric-amount',good.replace('"amount_minor":"'+max+'"','"amount_minor":'+max)],
  ['extra',good.slice(0,-1)+',"extra":true}'],['malformed','{']]){
  replacement=null;check('payment-client-'+name+'-valid-control',(await submitPayment(page,200))===good);
  replacement=raw;fs.writeFileSync(path.join(directory,'synthetic-'+name+'.json'),raw);
  const response=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/resolve-payment'&&r.request().method()==='POST');
  await page.click('#paea-check');assert.equal((await response).status(),200);
  await page.waitForFunction(()=>document.getElementById('paea-status').dataset.state==='error'&&!document.getElementById('paea-check').disabled);
  check('payment-client-rejects-'+name,await page.$eval('#paea-download',e=>e.disabled)&&await page.$eval('#paea-report',e=>e.textContent===''));
 }
 replacement=null;check('payment-client-final-native-recovery',(await submitPayment(page,200))===good);
 const waiting=new Promise(resolve=>{arrived=resolve});hold=true;const before=posts,pending=submitPayment(page,200);await waiting;
 check('payment-loading-blocks-inputs',await page.$eval('#paea-inputs',e=>e.disabled)&&await page.$eval('#paea-check',e=>e.disabled)&&await page.$eval('#paea-download',e=>e.disabled));
 await page.$eval('#paea-form',e=>e.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})));check('payment-no-duplicate-gesture',posts===before+1);hold=false;await held();check('payment-held-native-recovery',(await pending)===good);
 await page.evaluate(()=>{globalThis.originalPaeaParse=JSON.parse;JSON.parse=(raw,reviver)=>originalPaeaParse(raw,typeof reviver==='function'?function(key,value){return reviver.call(this,key,value)}:reviver)});
 const noLexeme=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/resolve-payment'&&r.request().method()==='POST');
 await page.click('#paea-check');assert.equal((await noLexeme).status(),200);await page.waitForFunction(()=>document.getElementById('paea-status').dataset.state==='error');
 check('payment-unsupported-number-reader-refuses',await page.$eval('#paea-status',e=>e.textContent.includes('CLI'))&&await page.$eval('#paea-download',e=>e.disabled));
 await page.evaluate(()=>{JSON.parse=originalPaeaParse;delete globalThis.originalPaeaParse});check('payment-number-reader-recovery',(await submitPayment(page,200))===good);
 const invalid=path.join(directory,'invalid-utf8.json');fs.writeFileSync(invalid,Buffer.from([255]));await fillPayment(page,invalid,request);const prior=posts;await page.click('#paea-check');
 await page.waitForFunction(()=>document.getElementById('paea-status').dataset.state==='error'&&!document.getElementById('paea-check').disabled);
 check('payment-invalid-utf8-never-sent',posts===prior&&await page.$eval('#paea-download',e=>e.disabled));
 await fillPayment(page,registry,request);check('payment-utf8-recovery',(await submitPayment(page,200))===good);
 page.off('request',intercept);await page.setRequestInterception(false);
}
(async()=>{let failed;
 try{await Promise.race([(async()=>{
  await start();check('actual-default-profile-executable',true);
  browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.resolve(output,'browser')});
  for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,390,320]){
   const label=lang+'-'+theme+'-'+width,context=await browser.createBrowserContext(),page=await context.newPage();
   page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width,height:1000});
   await context.setCookie({name:'swarm_session_'+sha(address).slice(0,12),value:token,url:'http://'+address,httpOnly:true,sameSite:'Strict'});
   page.on('pageerror',e=>browserFailure({kind:'pageerror',message:e.message}));
   page.on('requestfailed',r=>browserFailure({kind:'requestfailed',path:new URL(r.url()).pathname}));
   page.on('response',r=>{if(r.status()>=400){const p=new URL(r.url()).pathname;responses.push({path:p,status:r.status()});if(!(expectRefusal&&['/api/v1/protection/receipt-report','/api/v1/protection/resolve-payment'].includes(p)&&r.status()===400))browserFailure({kind:'http',path:p,status:r.status()})}});
   page.on('console',m=>{if(m.type()==='error'){const p=m.location().url?new URL(m.location().url).pathname:'';if(!(expectRefusal&&['/api/v1/protection/receipt-report','/api/v1/protection/resolve-payment'].includes(p)&&m.text().includes('400')))browserFailure({kind:'console',message:m.text()})}});
   await page.goto('http://'+address+'/paea.html?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
   await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
   check(label+'-explicit-empty-inputs',await page.$eval('#paea-state',e=>e.value==='')&&await page.$eval('#paea-download',e=>e.disabled));
   check(label+'-theme-language',await page.evaluate((l,t)=>document.documentElement.lang===l&&document.documentElement.dataset.theme===t,lang,theme));
   check(label+'-translated-primary',await page.$eval('#paea-check',(e,l)=>e.textContent.trim()===(l==='en'?'Check correspondence':'Vérifier la correspondance'),lang));
   check(label+'-independent-cockpit-link',await page.$eval('nav a',e=>e.getAttribute('href')==='/'));
   const embedded=await page.evaluate(async()=>{const result={};for(const p of ['/paea.html','/paea.js','/paea.css','/i18n-en.js','/i18n.js','/swarm-design.css','/wattson_themes.css','/swarm-logo.png']){const r=await fetch(p);if(!r.ok)throw Error('asset unavailable');result[p]=Array.from(new Uint8Array(await r.arrayBuffer()))}return result});
   check(label+'-actual-embedded-assets',Object.entries(embedded).every(([p,bytes])=>sha(Buffer.from(bytes))===sha(fs.readFileSync('web'+p))));
   for(const kind of ['payment.settle','repository.promote']){
    const v=vectors.find(x=>x.proposal.action.kind===kind),raw=JSON.stringify({proposal:v.proposal,context:v.context});
    const receipt=JSON.stringify({schema_version:1,action_digest:v.action_digest,intent_id:v.intent_id,sink_id:'sink-native-fixture',outcome:'COMMITTED',observation_digest:'d'.repeat(64)});
    const name=label+'-'+kind,doc=path.resolve(output,name+'-document.json'),rec=path.resolve(output,name+'-receipt.json');fs.writeFileSync(doc,raw);fs.writeFileSync(rec,receipt);
    await fill(page,doc,rec);const report=await submit(page,200);
    check(name+'-real-native-report',report.schema_version===2&&report.document.document_sha256===sha(raw)&&report.receipt_sha256===sha(receipt)&&report.document.action_digest===v.action_digest&&report.document.intent_id===v.intent_id);
    check(name+'-no-authority-or-effect',report.authority==='not-established'&&report.receipt_authenticity==='not-established'&&report.effect==='not-established');
    check(name+'-rendered-download',await page.$eval('#paea-report',(e,r)=>JSON.stringify(JSON.parse(e.textContent))===JSON.stringify(r),report));
    await page.$eval('#paea-sink',e=>{e.value='different-sink';e.dispatchEvent(new Event('input',{bubbles:true}))});
    check(name+'-edit-clears-old-report',await page.$eval('#paea-download',e=>e.disabled));
    const refusal=await submit(page,400);check(name+'-actual-refusal',!!refusal.failure?.code&&await page.$eval('#paea-report',e=>e.textContent===''));
    await page.$eval('#paea-sink',e=>{e.value='sink-native-fixture';e.dispatchEvent(new Event('input',{bubbles:true}))});
    check(name+'-recovery',(await submit(page,200)).document.action_digest===v.action_digest);
   }
   if(faultMode==='script'){
    await page.setCacheEnabled(false);await page.setRequestInterception(true);
    page.on('request',r=>{if(new URL(r.url()).pathname==='/paea.js'){console.log('INJECTED_NATIVE_PAEA_SCRIPT_FAILURE');r.abort('failed').catch(e=>browserFailure({kind:'script-injection',message:e.message}))}else r.continue().catch(e=>browserFailure({kind:'script-injection-continue',message:e.message}))});
    await page.goto('http://'+address+'/paea-payment.html?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
    await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);throw Error('script-fault-not-observed');
   }
   check(label+'-no-horizontal-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   await page.screenshot({path:path.join(output,label+'.png'),fullPage:true});cases.push(label);
   if(width===1440){
    await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click('nav a[href="/"]')]);
    await page.waitForSelector('a[href="/paea.html"]');check(label+'-native-menu-entry',true);
    const version=await page.evaluate(async()=>{const r=await fetch('/api/v1/native-version');if(!r.ok)throw Error('version unavailable');return r.json()});
    if(!await page.$eval('.rail-settings',e=>e.open))await page.click('.rail-settings > summary');
    await page.click('#version-open');await page.waitForSelector('#version-dialog[open]');
    await page.waitForFunction(v=>document.getElementById('version-dialog').textContent.includes(v),{},version.binary.version);
    check(label+'-version-and-PAEA-notes',await page.$eval('#version-dialog',(e,v)=>e.textContent.includes(v)&&e.textContent.includes('PAEA'),version.binary.version));
    check(label+'-actual-commit-link',await page.$$eval('#version-dialog a',(els,c)=>els.some(e=>e.href.endsWith('/commit/'+c)),version.binary.commit));
    await page.screenshot({path:path.join(output,label+'-version.png'),fullPage:true});
    await page.evaluate(()=>{globalThis.closedVersion=new Promise(resolve=>document.getElementById('version-dialog').addEventListener('close',resolve,{once:true}))});
    await page.keyboard.press('Escape');await page.evaluate(()=>globalThis.closedVersion);
    check(label+'-version-focus-restored',await page.evaluate(()=>document.activeElement.id==='version-open'));
   }
   await paymentCase(page,context,label,lang,theme);if(label==='fr-etat-1440')await paymentClientFaults(page,label);await context.close();
  }
  check('all-twelve-native-browser-cases',cases.length===12);check('all-twelve-payment-browser-cases',paymentCases.length===12);check('exactly-thirty-six-native-refusals',responses.length===36);check('no-unexpected-browser-diagnostics',errors.length===0);check('binary-stable',sha(fs.readFileSync(binary))===expected);
 })(),observedFailure]);}catch(e){failed=e;console.error(e.stack)}
 finally{
  try{if(browser)await browser.close()}catch(e){errors.push({kind:'browser-close',message:e.message});failed??=e}
  if(server){stopping=true;server.kill('SIGINT');const stopped=await closed;if(!failed&&stopped.code!==0)failed=Error('owned native fixture did not stop cleanly')}
  fs.writeFileSync(path.join(root,'server.stderr'),Buffer.concat(serverErrors),{mode:0o600});
  fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({scope:'real default-profile binary and embedded native UI, isolated fixture; no protected effect/independent acceptance',binary,binary_sha256:expected,cases,paymentCases,checks,errors,responses,success:!failed},null,2));
 }
 if(failed)process.exitCode=1;else console.log('PAEA_NATIVE_UI_PASS '+checks.length+' assertions');
})();
