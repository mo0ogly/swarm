'use strict';
/** Read-only native Biba source disclosure. stdin carries the owned session,
 * never saved or logged. No POST, provider, interception or global watchdog. */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[],posts=[];
function check(name,value){assert(value,name);checks.push(name);console.log('PASS '+name);}
(async()=>{let browser;try{
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,390]){
  const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  page.on('pageerror',e=>errors.push(e.message));page.on('requestfailed',r=>errors.push(r.failure()?.errorText));page.on('request',r=>{if(r.method()==='POST')posts.push(new URL(r.url()).pathname)});
  const split=cfg.cookie.indexOf('=');await context.setCookie({name:cfg.cookie.slice(0,split),value:cfg.cookie.slice(split+1),url:cfg.base,httpOnly:true,sameSite:'Strict'});
  await page.setViewport({width,height:1050});await page.goto(cfg.base+'/trust-actions.html?lang='+lang,{waitUntil:'networkidle0'});
  await page.waitForFunction(()=>/Prêt|Ready/.test(document.querySelector('#message').textContent));await page.$eval('html',(e,v)=>e.dataset.theme=v,theme);
  await page.click('#admin');await page.waitForFunction(()=>document.querySelector('#settings').open);
  await page.$eval('#trust-identity-title',e=>e.scrollIntoView({block:'center'}));
  const prefix=lang+'-'+theme+'-'+width;
  check(prefix+'-current-source-limit',await page.$eval('#trust-identity-availability',e=>e.textContent).then(v=>v.includes(lang==='fr'?'raccord HTTP non actif':'HTTP integration inactive')));
  check(prefix+'-source-list',await page.$$eval('#trust-identity-sources li',e=>e.length)===cfg.source_count);
  check(prefix+'-visible-warning',await page.$eval('#trust-identity-availability',e=>{const r=e.getBoundingClientRect();return r.top>=0&&r.bottom<=innerHeight}));
  check(prefix+'-no-overflow',await page.$eval('#settings',e=>e.scrollWidth<=e.clientWidth));
  await page.screenshot({path:path.join(cfg.output,prefix+'-identity.png')});await page.keyboard.press('Escape');
  await page.waitForFunction(()=>!document.querySelector('#settings').open&&document.activeElement.id==='admin');await context.close();
 }
 check('no-unexpected-diagnostics',errors.length===0);check('no-POST',posts.length===0);
 fs.writeFileSync(path.join(cfg.output,'identity-read.json'),JSON.stringify({checks,errors,posts,intercepted:false},null,2)+'\n');
}finally{if(browser)await browser.close()}})().catch(e=>{console.error(e.stack);process.exitCode=1});
