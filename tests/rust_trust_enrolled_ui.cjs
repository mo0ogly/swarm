'use strict';
/** Real single-intent server, no route interception or global watchdog.
 * Credentials arrive through private stdin, never result output. */
const fs=require('node:fs'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[];let browser;
function check(name,ok){assert(ok,name);checks.push(name);console.log('PASS '+name);}
(async()=>{try{
 fs.mkdirSync(cfg.output,{recursive:false});browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,390]){
  const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width,height:1100});
  const at=cfg.cookie.indexOf('=');await context.setCookie({name:cfg.cookie.slice(0,at),value:cfg.cookie.slice(at+1),url:cfg.base_url,httpOnly:true,sameSite:'Strict'});
  page.on('pageerror',e=>errors.push(e.message));page.on('requestfailed',r=>errors.push(r.failure()?.errorText));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});page.on('response',r=>{if(r.status()>=400)errors.push('HTTP '+r.status());});
  await page.evaluateOnNewDocument(t=>localStorage.setItem('theme',t),theme);
  await page.goto(cfg.base_url+'/trust-actions.html?lang='+lang,{waitUntil:'networkidle0'});await page.$eval('html',(e,t)=>e.dataset.theme=t,theme);await page.waitForFunction(()=>/Prêt|Ready/.test(document.querySelector('#message').textContent));await page.click('#admin');await page.waitForFunction(()=>document.querySelector('#settings').open);
  await page.$eval('details:has(#trust-enrolled-execute)',e=>e.open=true);
  check(lang+theme+width+'-source-bound',!await page.$eval('#trust-enrolled-execute',e=>e.disabled));
  await page.$eval('#trust-enrolled-request',(e,v)=>e.value=v,cfg.raw);await page.$eval('#trust-enrolled-password',(e,v)=>e.value=v,cfg.password);
  const pending=page.waitForResponse(r=>r.request().method()==='POST'&&r.url().endsWith('action=mandate-create'));await page.click('#trust-enrolled-execute');const response=await pending,body=await response.json();
  check(lang+theme+width+'-exact-body',response.request().postData()===cfg.raw);
  check(lang+theme+width+'-real-mandate',response.ok()&&body.object_id==='mandate'&&body.content_redacted===true);
  await page.waitForFunction(()=>document.querySelector('#trust-enrolled-password').value==='');check(lang+theme+width+'-password-cleared',true);
  const decision=await page.$eval('#trust-enrolled-message',e=>e.textContent);check(lang+theme+width+'-decision-language '+decision,decision===(lang==='fr'?'Intention admise — décision reçue · Admise':'Intent admitted — decision received · Admitted'));
  check(lang+theme+width+'-no-overflow',await page.$eval('#settings',e=>e.scrollWidth<=e.clientWidth));check(lang+theme+width+'-language',await page.$eval('#trust-enrolled-execute',(e,l)=>e.textContent.includes(l==='fr'?'Exécuter':'Execute'),lang));
  await page.screenshot({path:cfg.output+'/'+lang+'-'+theme+'-'+width+'.png'});await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#settings').open);await context.close();
 }
 check('no-unexpected-diagnostics',errors.length===0);fs.writeFileSync(cfg.output+'/result.json',JSON.stringify({status:'PASS',assertions:checks.length,checks,errors,intercepted:false,global_watchdog:null},null,2));
}catch(error){if(fs.existsSync(cfg.output))fs.writeFileSync(cfg.output+'/failure.json',JSON.stringify({error:String(error),checks,errors},null,2));throw error;}finally{if(browser)await browser.close();}})().catch(()=>process.exitCode=1);
