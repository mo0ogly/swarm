'use strict';
/** Exercise the actual default-profile native executable and embedded PAEA UI.
 * Fresh isolated Root only. No HTTP doubles/provider/live mission mutation.
 * The caller supplies an exact existing binary/hash and a fresh evidence path.
 * No build, retry or watchdog; readiness is observed from the real listener/UI.
 */
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {spawn}=require('node:child_process'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const output=process.argv[2]||process.env.SWARM_PAEA_UI_EVIDENCE,binary=process.env.SWARM_PAEA_TEST_BINARY,expected=process.env.SWARM_PAEA_TEST_BINARY_SHA256;
assert(output&&process.argv.length<=3&&binary&&/^[a-f0-9]{64}$/.test(expected||''),'explicit binary/hash/fresh output required');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
assert.equal(sha(fs.readFileSync(binary)),expected);fs.mkdirSync(output);
const root=path.resolve(output,'root');fs.mkdirSync(root);
const vectors=JSON.parse(fs.readFileSync('tests/fixtures/paea/contract-v1.json')).vectors;
const checks=[],errors=[],responses=[],cases=[],serverErrors=[];let browser,server,closed,token='',address='',expectRefusal=false;
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
 closed=new Promise(resolve=>server.once('close',(code,signal)=>resolve({code,signal})));
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
(async()=>{let failed;
 try{
  await start();check('actual-default-profile-executable',true);
  browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.resolve(output,'browser')});
  for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,390,320]){
   const label=lang+'-'+theme+'-'+width,context=await browser.createBrowserContext(),page=await context.newPage();
   page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width,height:1000});
   await context.setCookie({name:'swarm_session_'+sha(address).slice(0,12),value:token,url:'http://'+address,httpOnly:true,sameSite:'Strict'});
   page.on('pageerror',e=>errors.push({kind:'pageerror',message:e.message}));
   page.on('requestfailed',r=>errors.push({kind:'requestfailed',path:new URL(r.url()).pathname}));
   page.on('response',r=>{if(r.status()>=400){const p=new URL(r.url()).pathname;responses.push({path:p,status:r.status()});if(!(expectRefusal&&p==='/api/v1/protection/receipt-report'&&r.status()===400))errors.push({kind:'http',path:p,status:r.status()})}});
   page.on('console',m=>{if(m.type()==='error'){const p=m.location().url?new URL(m.location().url).pathname:'';if(!(expectRefusal&&p==='/api/v1/protection/receipt-report'&&m.text().includes('400')))errors.push({kind:'console',message:m.text()})}});
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
   await context.close();
  }
  check('all-twelve-native-browser-cases',cases.length===12);check('exactly-twenty-four-native-refusals',responses.length===24);check('no-unexpected-browser-diagnostics',errors.length===0);check('binary-stable',sha(fs.readFileSync(binary))===expected);
 }catch(e){failed=e;console.error(e.stack)}
 finally{
  if(browser)await browser.close();
  if(server){server.kill('SIGINT');const stopped=await closed;if(!failed&&stopped.code!==0)failed=Error('owned native fixture did not stop cleanly')}
  fs.writeFileSync(path.join(root,'server.stderr'),Buffer.concat(serverErrors),{mode:0o600});
  fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({scope:'real default-profile binary and embedded native UI, isolated fixture; no protected effect/independent acceptance',binary,binary_sha256:expected,cases,checks,errors,responses,success:!failed},null,2));
 }
 if(failed)process.exitCode=1;else console.log('PAEA_NATIVE_UI_PASS '+checks.length+' assertions');
})();
