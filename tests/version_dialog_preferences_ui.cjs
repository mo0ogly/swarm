'use strict';
/** Targeted real Rust HTTP fixture for Version wrapping and presentation URL
 * persistence. CLI init creates only an isolated Root; no provider/mission runs.
 * Optional --source-assets overlays exactly the three declared candidate assets,
 * explicitly separating source regression from a compiled/installed proof.
 * No global, navigation or protocol watchdog; named failures are retained. */
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),overlay=process.argv.includes('--source-assets');fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.resolve('.swarm-fixtures/version-ui-'));
execFileSync(binary,['--root',root,'init'],{stdio:'pipe'});
const app=spawn(binary,['web','--root',root,'--json','--lang','en'],{stdio:['ignore','pipe','pipe']});app.stderr.pipe(fs.createWriteStream(path.join(out,'server.stderr'),{flags:'wx'}));
let browser;let injectingHistoryRefusal=false;const checks=[],errors=[],faultErrors=[],consoleErrors=[],requestFailures=[],posts=[],inputs={};
for(const file of ['web/swarm-design.css','web/cockpit-native.js','web/i18n.js','web/i18n-en.js','web/runtime-health.js','web/swarm-shell.js','tests/version_dialog_preferences_ui.cjs'])inputs[file]=crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
/** Record all observable failures instead of abandoning the other preconditions. */
function check(name,ok,detail){checks.push({name,ok:!!ok,detail});console.log((ok?'PASS ':'FAIL ')+name)}
/** Wait for the real controlled read, including its DOM completion, before
 * navigating; mutation polling avoids reliance on offscreen animation frames. */
async function ready(page){await page.waitForFunction(()=>document.querySelector('#work')&&!document.querySelector('#main-content').hasAttribute('aria-busy'),{polling:'mutation'});await page.evaluate(async()=>{await VersionHistoryPanel.pending});}
/** Reveal existing mobile/tools controls through their actual interactions. */
async function tools(page){if(await page.$eval('#nav-toggle',e=>e.getClientRects().length>0&&e.getAttribute('aria-expanded')!=='true'))await page.click('#nav-toggle');if(!await page.$eval('.rail-settings',e=>e.open))await page.click('.rail-settings summary');}
(async()=>{try{
 const address=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',b=>{text+=b;let i;while((i=text.indexOf('\n'))>=0){const line=text.slice(0,i);text=text.slice(i+1);try{const frame=JSON.parse(line);if(frame.event==='web-bound')resolve(frame.address)}catch(e){reject(e)}}});app.on('error',reject);app.on('exit',c=>reject(Error('fixture server exited '+c)))});
 const base='http://'+address,digest=crypto.createHash('sha256').update(address).digest('hex'),key=fs.readFileSync(path.join(root,'.swarm','web-session-'+digest.slice(0,16)),'utf8');
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',timeout:0,protocolTimeout:0,userDataDir:path.join(root,'chrome'),args:['--no-sandbox']});
 const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);page.on('pageerror',e=>(injectingHistoryRefusal?faultErrors:errors).push(e.message));page.on('console',m=>{if(['warn','error'].includes(m.type()))consoleErrors.push(m.text())});page.on('requestfailed',r=>requestFailures.push({path:new URL(r.url()).pathname,error:r.failure()?.errorText}));page.on('request',r=>{if(r.method()==='POST')posts.push(new URL(r.url()).pathname)});
 if(overlay){await page.setRequestInterception(true);page.on('request',r=>{const pathname=new URL(r.url()).pathname;const file={'/swarm-design.css':'web/swarm-design.css','/cockpit-native.js':'web/cockpit-native.js','/i18n-en.js':'web/i18n-en.js'}[pathname];if(file)r.respond({status:200,contentType:file.endsWith('.css')?'text/css':'application/javascript',body:fs.readFileSync(file)}).catch(e=>errors.push(e.message));else r.continue().catch(e=>errors.push(e.message))})}
 await context.setCookie({name:'swarm_session_'+digest.slice(0,12),value:key,url:base,httpOnly:true,sameSite:'Strict'});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [624,390]){
  const label=lang+'-'+theme+'-'+width;await page.setViewport({width,height:900});await page.goto(base+'/?'+new URLSearchParams({view:'conduite',lang,theme}),{waitUntil:'load'});await ready(page);await tools(page);await page.click('#version-open');await page.waitForFunction(()=>document.querySelector('#version-dialog')?.open&&document.querySelector('.version-local-changes li'),{polling:'mutation'});
  const measure=await page.evaluate(()=>{const content=document.querySelector('#version-dialog-content');return {client:content.clientWidth,scroll:content.scrollWidth,document:document.documentElement.scrollWidth,viewport:innerWidth,titleFocused:document.activeElement.id==='version-dialog-title',notes:[...content.querySelectorAll('li')].map(e=>e.textContent),overflow:[...content.querySelectorAll('li')].filter(e=>e.scrollWidth>e.clientWidth+1).map(e=>({text:e.textContent,client:e.clientWidth,scroll:e.scrollWidth,wrap:getComputedStyle(e).overflowWrap}))}});
  check(label+'-internal-content-fits',measure.scroll<=measure.client+1,measure);check(label+'-document-fits',measure.document<=measure.viewport+1);check(label+'-title-focus',measure.titleFocused);check(label+'-all-local-notes-retained',measure.notes.length>0&&JSON.stringify(measure.notes)===JSON.stringify(await page.evaluate(()=>VersionHistoryPanel.value.history.local_changes.items.map(item=>item[document.documentElement.lang]))));
  if(width===624)await (await page.$('#version-dialog')).screenshot({path:path.join(out,label+'.png')});
  await page.keyboard.press('Escape');check(label+'-escape-restores-opener',await page.evaluate(()=>!document.querySelector('#version-dialog').open&&document.activeElement.id==='version-open'));
  await page.click('#version-open');await page.evaluate(async()=>{await VersionHistoryPanel.pending});check(label+'-reopens',await page.$eval('#version-dialog',e=>e.open));await page.keyboard.press('Escape');
 }
 // Exact reported precondition: URL has no theme; theme changed by its button.
 await page.goto(base+'/?view=conduite&lang=en',{waitUntil:'load'});await ready(page);await tools(page);if(await page.$eval('html',e=>e.dataset.theme!=='etat'))await page.click('#theme');await page.click('#theme');check('button-selects-dark',await page.$eval('html',e=>e.dataset.theme==='sombre'));await Promise.all([page.waitForNavigation({waitUntil:'load'}),page.select('#swarm-language','fr')]);await ready(page);check('EN-dark-to-FR-retains-dark',await page.$eval('html',e=>e.lang==='fr'&&e.dataset.theme==='sombre'),page.url());await page.reload({waitUntil:'load'});await ready(page);check('reload-retains-language-and-dark',await page.$eval('html',e=>e.lang==='fr'&&e.dataset.theme==='sombre'));
 await tools(page);if(await page.$eval('html',e=>e.dataset.theme!=='etat'))await page.click('#theme');check('button-selects-light',await page.$eval('html',e=>e.dataset.theme==='etat'));await Promise.all([page.waitForNavigation({waitUntil:'load'}),page.select('#swarm-language','en')]);await ready(page);check('FR-light-to-EN-retains-light',await page.$eval('html',e=>e.lang==='en'&&e.dataset.theme==='etat'));
 await page.goto(base+'/?view=conduite&lang=en&theme=invalid',{waitUntil:'load'});await ready(page);check('invalid-theme-keeps-native-light-default',await page.$eval('html',e=>e.dataset.theme==='etat'));
 // Marked client fault: a history refusal must leave both DOM and URL old.
 await tools(page);const oldPresentation=await page.evaluate(()=>({theme:document.documentElement.dataset.theme,url:location.href}));injectingHistoryRefusal=true;
 await page.evaluate(()=>{window.fixtureHistory={original:history.replaceState,cause:null};window.fixtureHistory.listener=e=>{window.fixtureHistory.cause={message:e.error?.message,cause:e.error?.cause?.message}};addEventListener('error',window.fixtureHistory.listener);history.replaceState=()=>{throw new DOMException('fixture-history-refused','SecurityError')}});
 await page.click('#theme');const refused=await page.evaluate(()=>({theme:document.documentElement.dataset.theme,url:location.href,message:document.querySelector('#message').textContent,visible:!document.querySelector('#message').hidden,cause:window.fixtureHistory.cause}));
 check('history-refusal-keeps-old-presentation',refused.theme===oldPresentation.theme&&refused.url===oldPresentation.url,refused);
 check('history-refusal-visible',refused.visible&&refused.message.includes('theme'),refused.message);
 check('history-refusal-preserves-cause',refused.cause?.message==='web-native-theme-update-refused'&&refused.cause?.cause==='fixture-history-refused',refused.cause);
 await page.evaluate(()=>{history.replaceState=window.fixtureHistory.original;removeEventListener('error',window.fixtureHistory.listener);delete window.fixtureHistory});injectingHistoryRefusal=false;
 check('only-marked-history-error',faultErrors.length===1&&faultErrors[0].includes('web-native-theme-update-refused'),faultErrors);
 await page.click('#theme');await page.reload({waitUntil:'load'});await ready(page);check('history-refusal-recovery-persists',await page.$eval('html',(e,oldTheme)=>e.dataset.theme!==oldTheme,oldPresentation.theme),null);
 check('no-JS-errors',errors.length===0,errors);check('no-console-warnings-errors',consoleErrors.length===0,consoleErrors);check('no-failed-requests',requestFailures.length===0,requestFailures);check('no-POST',posts.length===0,posts);await context.close();
 }catch(error){errors.push(error.stack);throw error}finally{if(browser)await browser.close();if(app.exitCode===null&&app.signalCode===null){const stopped=new Promise(resolve=>app.once('exit',resolve));if(!app.kill('SIGTERM'))throw Error('fixture-server-stop-refused');await stopped}const result={status:checks.length>0&&errors.length===0&&checks.every(c=>c.ok)?'PASS':'FAIL',checks,errors,marked_fault_errors:faultErrors,consoleErrors,requestFailures,posts,overlay,inputs,binary_sha256:crypto.createHash('sha256').update(fs.readFileSync(binary)).digest('hex'),root,global_watchdog:null};fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));}
 if(checks.some(c=>!c.ok))process.exitCode=1;
})().catch(e=>{console.error(e.stack);process.exitCode=1});
