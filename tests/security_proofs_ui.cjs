'use strict';
/** Actual protected HTTP page on an owned fixture. No route doubles on the
 * successful path; one explicit malformed-response fault follows that control.
 * Synthetic credentials stay in memory and out of screenshots/results. */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const runtimeTools=require('./helpers/browser_verification_runtime.cjs');
(async()=>{
 const cfg=JSON.parse(fs.readFileSync(process.env.SWARM_SECURITY_UI_CONFIG,'utf8')),out=cfg.output;
 const checks=[],errors=[];let browser,failure,runtime;
 let diagnosticReject;const diagnosticFailure=new Promise((_,reject)=>{diagnosticReject=reject});
 function check(name,value){checks.push({name,passed:!!value});assert(value,name);}
 try{await Promise.race([(async()=>{
  runtime=runtimeTools.begin(process.cwd(),out);
  browser=await puppeteer.launch({...runtimeTools.launchOptions,headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',userDataDir:cfg.profile,args:['--no-sandbox']});
  await runtimeTools.recordBrowser(runtime,browser);
  for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
   const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
   await page.setCookie({name:cfg.cookie_name,value:cfg.cookie_value,url:cfg.url});
   let expectedDenial=false;
   const diagnostic=message=>{errors.push(message);diagnosticReject(Error(message))};
   page.on('pageerror',e=>diagnostic(e.message));page.on('console',m=>{if(m.type()==='error'&&!(expectedDenial&&m.location().url?.includes('action=security-show')&&m.text().includes('403')))diagnostic(m.text())});
   page.on('requestfailed',r=>diagnostic('request-failed:'+r.url()));
   page.on('response',r=>{if(r.status()>=400&&!(expectedDenial&&r.url().includes('action=security-show')&&r.status()===403))diagnostic('http-'+r.status()+':'+r.url())});
   await page.goto(cfg.url+'security-proofs.html?lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'});
   check(lang+theme+'-initial-empty',await page.$eval('#security-report',e=>e.hidden));
   check(lang+theme+'-language',await page.$eval('html',(e,l)=>e.lang===l,lang));
   await page.type('#security-token','fixture-admin-token');await page.click('#security-refresh');
   await page.waitForFunction(()=>!document.getElementById('security-report').hidden||(!document.getElementById('security-refresh').disabled&&document.getElementById('security-message').textContent.includes('invalide')));
   check(lang+theme+'-report-visible',await page.$eval('#security-report',e=>!e.hidden));
   check(lang+theme+'-25-boundaries',await page.$$eval('#security-flows details',e=>e.length===25));
   check(lang+theme+'-no-false-qualification',await page.$eval('#security-json',e=>{const v=JSON.parse(e.textContent);return !v.domains[0].runtime_enforced&&v.domains[0].verification.result==='NOT_TESTED'&&v.domains[0].qualification.state==='not_qualified'}));
   check(lang+theme+'-translated-summary',await page.$eval('#security-runtime',(e,l)=>e.textContent.includes(l==='en'?'Partial protection':'Protection partielle'),lang));
   check(lang+theme+'-six-real-recipes',await page.$$eval('#security-recipes li',e=>e.length===6));
   await page.evaluate(async()=>{await Promise.all(document.getAnimations().filter(a=>a.playState==='running'&&Number.isFinite(a.effect?.getComputedTiming().endTime)).map(a=>a.finished))});
   const contrasts=await page.evaluate(()=>{
    const rgb=s=>s.match(/[\d.]+/g).slice(0,3).map(Number);
    const lum=c=>c.map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);
    return ['body','#security-token','#security-refresh'].map(selector=>{const style=getComputedStyle(document.querySelector(selector));const a=lum(rgb(style.color)),b=lum(rgb(style.backgroundColor));return {selector,ratio:(Math.max(a,b)+.05)/(Math.min(a,b)+.05)}});
   });
   check(lang+theme+'-normal-text-AA',contrasts.every(c=>c.ratio>=4.5));
   await page.click('#security-flows-disclosure > summary');await page.focus('#security-flows summary');await page.keyboard.press('Enter');
   check(lang+theme+'-keyboard-disclosure',await page.$eval('#security-flows details',e=>e.open));
   await page.click('#security-flows-disclosure > summary');
   check(lang+theme+'-progressive-coverage',await page.$eval('#security-flows-disclosure',e=>!e.open));
   for(const width of [1440,1024,760,390,320]){
    await page.setViewport({width,height:900});check(lang+theme+'-width-'+width,await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   }
   await page.screenshot({path:path.join(out,lang+'-'+theme+'-320.png'),fullPage:true});
   await page.setViewport({width:1440,height:1000});await page.screenshot({path:path.join(out,lang+'-'+theme+'-1440.png'),fullPage:true});
   await page.click('#security-clear');
   check(lang+theme+'-clear-and-focus',await page.evaluate(()=>document.getElementById('security-token').value===''&&document.getElementById('security-report').hidden&&document.activeElement.id==='security-token'&&document.getElementById('security-json').textContent===''));
   // Same successful preconditions, but current caller is not an administrator.
   expectedDenial=true;await page.type('#security-token','fixture-worker-token');await page.click('#security-refresh');
   await page.waitForFunction(()=>!document.getElementById('security-refresh').disabled);
   check(lang+theme+'-weak-refused',await page.$eval('#security-report',e=>e.hidden));
   check(lang+theme+'-visible-denial',await page.$eval('#security-message',(e,l)=>e.textContent.includes(l==='en'?'I3 administrator':'administrateur I3'),lang));
   expectedDenial=false;
   // Restore the exact valid control before altering only the response body.
   await page.click('#security-clear');await page.type('#security-token','fixture-admin-token');await page.click('#security-refresh');
   await page.waitForFunction(()=>!document.getElementById('security-report').hidden);
   await page.setRequestInterception(true);page.on('request',request=>request.url().includes('action=security-show')?request.respond({status:200,contentType:'application/json',body:'{"schema_version":1,"accepted":true}'}):request.continue());
   await page.click('#security-refresh');await page.waitForFunction(()=>!document.getElementById('security-refresh').disabled);
   check(lang+theme+'-malformed-fails-closed',await page.evaluate(()=>document.getElementById('security-report').hidden&&document.getElementById('security-json').textContent===''));
   await context.close();
  }
  check('no-browser-errors',errors.length===0);
  runtimeTools.finish(runtime,process.cwd());
 })(),diagnosticFailure]);
 }catch(e){failure=e;console.error(e.stack)}finally{if(browser)await browser.close();fs.writeFileSync(path.join(out,'browser-result.json'),JSON.stringify({checks,errors,success:!failure,scope:'owned fixture, actual HTTP image assets; no real Authority-session acceptance'},null,2));}
 if(failure)process.exitCode=1;else console.log('SECURITY_PROOFS_BROWSER_PASS '+checks.length);
})();
