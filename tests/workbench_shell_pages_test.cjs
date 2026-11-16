'use strict';
/** Render the real PAEA hub (web/paea.html) and security proofs page
 * (web/security-proofs.html) with web/workbench-shell.js against a local HTTP
 * double and check the shared workbench shell: rail with the current entry,
 * links keeping language/theme and carrying no token, header moved into main
 * with eyebrow and lead, original theme button hidden and replaced by a
 * Dark/Light switch that sets data-theme and the URL, PAEA view links gathered
 * in a card grid (same hrefs, data-paea-link kept), security sections laid out
 * as cards, FR/EN, both themes, 1440/390 px without horizontal overflow, and no
 * request other than the native session read.
 * Scope: UI behaviour on stub responses; no native service, provider, PAEA
 * report or security reading. Usage:
 * node tests/workbench_shell_pages_test.cjs FRESH_OUTPUT (no watchdog).
 */
const fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const output=process.argv[2];if(!output||process.argv.length!==3)throw Error('Usage: node tests/workbench_shell_pages_test.cjs FRESH_OUTPUT');
fs.mkdirSync(output);const requests=[],errors=[];let count=0,server,browser;
const assets=new Map(['/paea.html','/paea.js','/paea.css','/security-proofs.html','/security-proofs.js','/security-proofs.css','/trust-native.css',
 '/workbench-shell.js','/swarm-workbench.css','/i18n-en.js','/i18n.js','/swarm-design.css','/wattson_themes.css','/swarm-logo.png'].map(url=>[url,'web'+url]));
/** Record one named assertion. */
function check(name,value){assert(value,name);count++;console.log('PASS '+name)}
/** Serve named workspace files and the native session; record every API call.
 * Any unknown path is an error so a missing shell asset cannot pass. */
function serve(request,response){
 const url=new URL(request.url,'http://localhost');
 if(url.pathname==='/favicon.ico'){response.statusCode=204;response.end();return}
 if(url.pathname.startsWith('/api/')){requests.push(request.method+' '+url.pathname);if(url.pathname==='/api/v1/native-session'){response.setHeader('Content-Type','application/json');response.end(JSON.stringify({schema_version:1,csrf_token:'d'.repeat(64)}));return}errors.push('unexpected api '+url.pathname);response.statusCode=500;response.end('{}');return}
 if(assets.has(url.pathname)){response.setHeader('Content-Type',{'.html':'text/html; charset=utf-8','.css':'text/css','.js':'application/javascript','.png':'image/png'}[path.extname(url.pathname)]);response.end(fs.readFileSync(assets.get(url.pathname)));return}
 errors.push('unknown fixture path '+url.pathname);response.statusCode=404;response.end();
}
const pages=[{file:'paea.html',key:'paea',theme:'paea-theme',eyebrow:/04 · (Suivi|Monitoring)/},{file:'security-proofs.html',key:'security-proofs',theme:'security-theme',eyebrow:/05 · Configuration · (Sécurité|Security)/}];
(async()=>{try{
 server=http.createServer(serve);await new Promise(r=>server.listen(0,'127.0.0.1',r));const origin='http://127.0.0.1:'+server.address().port;
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(path.resolve(output),'browser')});
 const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
 for(const p of pages)for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,390]){
  const label=p.key+'-'+lang+'-'+theme+'-'+width;requests.length=0;await page.setViewport({width,height:1000});
  await page.goto(origin+'/'+p.file+'?lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'});
  check(label+'-shell-mounted',await page.$eval('body',b=>b.classList.contains('workbench-shell-page')&&!!document.querySelector('body>.workbench-rail')&&document.querySelector('main').contains(document.querySelector('.workbench-page-header'))));
  check(label+'-skip-link-first',await page.$eval('body',b=>b.firstElementChild.tagName==='A'&&b.firstElementChild.getAttribute('href').startsWith('#')));
  check(label+'-rail-current',await page.$eval('.workbench-rail [aria-current=page]',(a,f)=>new URL(a.href).pathname==='/'+f,p.file));
  check(label+'-rail-links-keep-lang-theme-no-token',await page.$$eval('.workbench-rail nav a',(as,x)=>as.length===6&&as.every(a=>{const u=new URL(a.href);return u.searchParams.get('lang')===x.lang&&u.searchParams.get('theme')===x.theme&&[...u.searchParams.keys()].every(k=>['lang','theme','view'].includes(k))}),{lang,theme}));
  check(label+'-eyebrow-and-lead',p.eyebrow.test(await page.$eval('.audit-eyebrow',e=>e.textContent))&&await page.$eval('.audit-heading .workbench-lead',e=>e.textContent.trim().length>20));
  check(label+'-original-theme-hidden',await page.$eval('#'+p.theme,b=>b.hidden));
  check(label+'-switch-reflects-theme',await page.$eval('.workbench-theme-switch [aria-pressed=true]',(b,t)=>b.dataset.themeChoice===t,theme));
  const other=theme==='etat'?'sombre':'etat';await page.click('.workbench-theme-switch [data-theme-choice='+other+']');
  check(label+'-switch-changes-theme-and-url',await page.evaluate(t=>document.documentElement.dataset.theme===t&&new URL(location.href).searchParams.get('theme')===t,other));
  check(label+'-rail-link-follows-new-theme',await page.$eval('.workbench-rail nav a',(a,t)=>{a.addEventListener('click',e=>e.preventDefault(),{once:true});a.dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true}));return new URL(a.href).searchParams.get('theme')===t},other));
  await page.click('.workbench-theme-switch [data-theme-choice='+theme+']');
  if(p.key==='paea'){
   check(label+'-paea-links-grid',JSON.stringify(await page.$$eval('.shell-links a',as=>as.map(a=>new URL(a.href).pathname)))===JSON.stringify(['/paea-payment.html','/paea-compare.html','/paea-inspect.html','/paea-approval.html','/paea-lab.html','/paea-effects.html','/paea-consent.html','/paea-linux.html']));
   check(label+'-paea-links-keep-page-sync',await page.$$eval('.shell-links a',as=>as.every(a=>a.hasAttribute('data-paea-link'))));
   check(label+'-paea-form-is-card',await page.$eval('#paea-form',f=>getComputedStyle(f).borderTopStyle==='solid'));
  }else{
   check(label+'-security-form-card',await page.$eval('#security-access',f=>getComputedStyle(f).borderTopStyle==='solid'));
   check(label+'-security-report-still-hidden',await page.$eval('#security-report',r=>r.hidden&&getComputedStyle(r).display==='none'));
  }
  check(label+'-only-session-read',requests.every(r=>r==='GET /api/v1/native-session'));
  check(label+'-no-horizontal-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.screenshot({path:path.join(output,label+'.png'),fullPage:true});
 }
 assert.deepEqual(errors,[]);console.log(count+' workbench shell checks passed');
}catch(error){console.error(error);console.error('errors:',errors);process.exitCode=1}finally{await browser?.close();server?.close()}})();
