'use strict';
/** Laboratory report reading through real protected embedded assets on an isolated
 * native server. Reports come from the real maintained campaign; corrupt files
 * are browser-only negative cases, not independent security evaluation.
 * No build/retry/watchdog; early process/browser faults interrupt pending waits.
 */
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const output=process.env.SWARM_PAEA_LAB_UI_EVIDENCE+(process.env.SWARM_PAEA_LAB_UI_FAULT?'-fault':''),binary=process.env.SWARM_PAEA_TEST_BINARY,expected=process.env.SWARM_PAEA_TEST_BINARY_SHA256;
const scope=process.env.SWARM_PAEA_LAB_UI_SCOPE||'native';assert(['native','workspace-overlay'].includes(scope));
const fault=process.env.SWARM_PAEA_LAB_UI_FAULT;assert([undefined,'script'].includes(fault));
const source=path.join(process.env.SWARM_PAEA_LAB_EVIDENCE,'paea-lab-report.json'),raw=fs.readFileSync(source,'utf8'),report=JSON.parse(raw);
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');assert.equal(sha(fs.readFileSync(binary)),expected);assert.equal(report.binary_sha256,expected);fs.mkdirSync(output);
const root=path.resolve(output,'root');fs.mkdirSync(root);const checks=[],errors=[],serverErrors=[];let server,closed,browser,address,token,stopping=false,abort;
const observedFailure=new Promise((resolve,reject)=>{abort=reject});
/** Record real unique assertions; unexpected diagnostics stop waits immediately. */
function check(name,value){assert(value,name);assert(!checks.includes(name));checks.push(name);console.log('PASS '+name)}
/** Propagate an observed error without a temporal watchdog or unchanged retry. */
function browserFailure(value){errors.push(value);abort(Error('unexpected-lab-browser-diagnostic'))}
/** Capture one owned CLI process; actual failure propagates with its stderr. */
async function command(args){const c=spawn(binary,args),o=[],e=[];c.stdout.on('data',b=>o.push(b));c.stderr.on('data',b=>e.push(b));const exit=await new Promise((resolve,reject)=>{c.on('error',reject);c.on('close',(code,signal)=>resolve({code,signal}))});assert.equal(exit.signal,null);assert.equal(exit.code,0,Buffer.concat(e).toString());return Buffer.concat(o)}
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

(async()=>{let failure;
try{await Promise.race([(async()=>{
 await start();browser=await puppeteer.launch({headless:true,executablePath:'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.resolve(output,'browser')});
 const base='http://'+address+'/';
 const unauth=await fetch(base+'paea-lab.html');check('laboratory-private-asset-refuses-anonymous',unauth.status!==200);
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,390,320]){
  const label=[lang,theme,width].join('-'),context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width,height:1000});
  await context.setCookie({name:'swarm_session_'+sha(address).slice(0,12),value:token,url:base,httpOnly:true,sameSite:'Strict'});
  page.on('pageerror',e=>browserFailure(e.message));page.on('console',m=>{if(m.type()==='error')browserFailure(m.text())});page.on('requestfailed',r=>browserFailure('failed:'+new URL(r.url()).pathname));page.on('response',r=>{if(r.status()>=400)browserFailure('HTTP'+r.status())});
  await page.setRequestInterception(true);page.on('request',r=>{if(!['GET','HEAD'].includes(r.method())){browserFailure('unexpected-write');r.abort().catch(browserFailure)}else if(fault&&new URL(r.url()).pathname==='/paea-lab.js'){console.log('INJECTED_LAB_SCRIPT_FAILURE');r.abort().catch(browserFailure)}else if(scope==='workspace-overlay'&&['/paea.html','/paea-lab.html','/paea-lab.js','/i18n-en.js'].includes(new URL(r.url()).pathname)){const p=new URL(r.url()).pathname;r.respond({status:200,contentType:p.endsWith('.js')?'application/javascript':'text/html',body:fs.readFileSync('web'+p)}).catch(browserFailure)}else r.continue().catch(browserFailure)});
  await page.goto(base+'paea.html?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
  await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click('a[href^="/paea-lab.html"]')]);await page.waitForFunction(()=>!document.getElementById('lab-read').disabled);
  check(label+'-menu-presentation',await page.evaluate((l,t)=>location.pathname==='/paea-lab.html'&&document.documentElement.lang===l&&document.documentElement.dataset.theme===t,lang,theme));
  check(label+'-translated-title-limits',await page.$eval('h1',(e,l)=>e.textContent===(l==='en'?'PAEA laboratory campaigns':'Campagnes de laboratoire PAEA'),lang)&&await page.$eval('#lab-limits',(e,l)=>e.textContent.includes(l==='en'?'simulated authority':'autorité simulée'),lang));
  const bytes=await page.evaluate(async()=>{const o={};for(const p of ['/paea-lab.html','/paea-lab.js']){const r=await fetch(p);o[p]=Array.from(new Uint8Array(await r.arrayBuffer()))}return o});check(label+(scope==='native'?'-exact-protected-assets':'-workspace-overlay-bytes'),Object.entries(bytes).every(([p,b])=>sha(Buffer.from(b))===sha(fs.readFileSync('web'+p))));
  check(label+'-empty-no-success',await page.$eval('#lab-results',e=>e.hidden)&&await page.$eval('#lab-download',e=>e.disabled));
  await page.focus('#lab-read');await page.keyboard.press('Enter');await page.waitForFunction(()=>document.getElementById('lab-status').dataset.state==='error');check(label+'-missing-file-refused',await page.$eval('#lab-results',e=>e.hidden));
  await (await page.$('#lab-file')).uploadFile(source);await page.click('#lab-read');await page.waitForFunction(()=>document.getElementById('lab-status').dataset.state==='read');
  check(label+'-eight-real-cases-and-counterexample',await page.$$eval('#lab-tests li',e=>e.length===8)&&await page.$eval('#lab-counterexample',(e,l)=>e.textContent.includes(l==='en'?'counterexample':'contre-exemple'),lang));
  check(label+'-exact-binary-provenance',await page.$eval('#lab-identities',(e,h)=>e.textContent.includes(h),expected));
  await page.evaluate(()=>{const original=File.prototype.arrayBuffer;globalThis.labReads=0;File.prototype.arrayBuffer=function(){globalThis.labReads++;return new Promise(resolve=>{globalThis.releaseLabRead=()=>{File.prototype.arrayBuffer=original;resolve(original.call(this))}})}});
  await page.click('#lab-read');check(label+'-loading-clears-and-blocks',await page.$eval('#lab-status',e=>e.dataset.state==='loading')&&await page.$eval('#lab-inputs',e=>e.disabled)&&await page.$eval('#lab-download',e=>e.disabled)&&await page.$eval('#lab-results',e=>e.hidden));
  await page.$eval('#lab-form',e=>e.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})));check(label+'-duplicate-gesture-refused',await page.evaluate(()=>globalThis.labReads===1));
  await page.evaluate(()=>globalThis.releaseLabRead());await page.waitForFunction(()=>document.getElementById('lab-status').dataset.state==='read');check(label+'-loading-recovery',await page.$eval('#lab-tests',e=>e.children.length===8));
  const directory=path.resolve(output,label);fs.mkdirSync(directory);const cdp=await page.createCDPSession();await cdp.send('Browser.setDownloadBehavior',{behavior:'allow',downloadPath:directory,browserContextId:context.id,eventsEnabled:true});const downloaded=new Promise((resolve,reject)=>cdp.on('Browser.downloadProgress',e=>{if(e.state==='completed')resolve();if(e.state==='canceled')reject(Error('download-canceled'))}));await page.click('#lab-download');await downloaded;await cdp.detach();check(label+'-exact-completed-download',fs.readFileSync(path.join(directory,'paea-lab-report.json'),'utf8')===raw);
  await page.focus('#lab-scenarios summary');await page.keyboard.press('Enter');check(label+'-keyboard-scenarios',await page.$eval('#lab-scenarios',e=>e.open));await page.keyboard.press('Enter');
  for(const [name,change] of Object.entries({empty:v=>({}),duplicate:v=>{v.tests[1]=v.tests[0];return v},missing:v=>{v.tests.pop();return v},failed:v=>{v.tests[0].status='FAIL';return v},authority:v=>{v.authority='protected';return v},hash:v=>{v.calls_sha256='';return v},extra:v=>{v.html='<img src=x onerror=alert(1)>';return v}})){
   await (await page.$('#lab-file')).uploadFile(source);await page.click('#lab-read');await page.waitForFunction(()=>document.getElementById('lab-status').dataset.state==='read');check(label+'-'+name+'-valid-control',await page.$eval('#lab-tests',e=>e.children.length===8));
   const file=path.join(directory,name+'.json');fs.writeFileSync(file,JSON.stringify(change(structuredClone(report))));await (await page.$('#lab-file')).uploadFile(file);check(label+'-'+name+'-clears-stale',await page.$eval('#lab-download',e=>e.disabled));await page.click('#lab-read');await page.waitForFunction(()=>document.getElementById('lab-status').dataset.state==='error');check(label+'-'+name+'-refused',await page.$eval('#lab-results',e=>e.hidden));
  }
  await (await page.$('#lab-file')).uploadFile(source);await page.click('#lab-read');await page.waitForFunction(()=>document.getElementById('lab-status').dataset.state==='read');check(label+'-utf8-valid-control',await page.$eval('#lab-tests',e=>e.children.length===8));
  const invalid=path.join(directory,'invalid-utf8.json');fs.writeFileSync(invalid,Buffer.from([0xc3,0x28]));await (await page.$('#lab-file')).uploadFile(invalid);await page.click('#lab-read');await page.waitForFunction(()=>document.getElementById('lab-status').dataset.state==='error');check(label+'-invalid-utf8-refused',await page.$eval('#lab-status',(e,l)=>e.textContent===(l==='en'?'Invalid UTF-8 file; choose a valid file.':'Fichier UTF-8 invalide ; choisir un fichier valide.'),lang));
  for(const [name,bytes] of [['malformed-json',Buffer.from('{')],['bom',Buffer.concat([Buffer.from([0xef,0xbb,0xbf]),Buffer.from(raw)])]]){
   await (await page.$('#lab-file')).uploadFile(source);await page.click('#lab-read');await page.waitForFunction(()=>document.getElementById('lab-status').dataset.state==='read');check(label+'-'+name+'-valid-control',await page.$eval('#lab-tests',e=>e.children.length===8));
   const file=path.join(directory,name+'.json');fs.writeFileSync(file,bytes);await (await page.$('#lab-file')).uploadFile(file);await page.click('#lab-read');await page.waitForFunction(()=>document.getElementById('lab-status').dataset.state==='error');check(label+'-'+name+'-refused',await page.$eval('#lab-results',e=>e.hidden)&&await page.$eval('#lab-download',e=>e.disabled));
  }
  await (await page.$('#lab-file')).uploadFile(source);await page.click('#lab-read');await page.waitForFunction(()=>document.getElementById('lab-status').dataset.state==='read');check(label+'-valid-recovery',await page.$eval('#lab-tests',e=>e.children.length===8));
  check(label+'-no-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  check(label+'-primary-contrast-AA',await page.$eval('#lab-read',e=>{const s=getComputedStyle(e),l=c=>c.match(/[\d.]+/g).slice(0,3).map(Number).map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((a,c,i)=>a+c*[.2126,.7152,.0722][i],0),a=l(s.color),b=l(s.backgroundColor);return (Math.max(a,b)+.05)/(Math.min(a,b)+.05)>=4.5}));
  await page.screenshot({path:path.join(directory,'result.png'),fullPage:true});await context.close();
 }
 check('no-browser-errors-or-business-writes',errors.length===0);console.log((scope==='native'?'PAEA_LAB_NATIVE_UI_PASS ':'PAEA_LAB_COMPONENT_UI_PASS ')+checks.length);
})(),observedFailure]);}catch(error){failure=error;console.error(error.stack);process.exitCode=1}
finally{if(browser)await browser.close();if(server){stopping=true;server.kill('SIGINT');const exit=await closed;if(exit.code!==0&&!failure){process.exitCode=1;failure=Error('owned-server-stop-failed')}}fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:failure?'FAIL':'PASS',scope,overlay_assets:scope==='native'?[]:['/paea.html','/paea-lab.html','/paea-lab.js','/i18n-en.js'],binary_sha256:expected,checks,errors,global_watchdog:null,failure:failure?.message||null},null,2)+'\n');fs.writeFileSync(path.join(output,'server.stderr'),Buffer.concat(serverErrors))}
})();
