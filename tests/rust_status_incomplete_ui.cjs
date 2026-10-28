'use strict';
/** Read-only existing cockpit on an owned native incomplete-delivery or abandoned fixture.
 * Four language/theme cases, real HTTP; no provider, POST or watchdog.
 * Retain named assertions and screenshots; propagate errors and close browser. */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[],requests=[];let browser;
/** Record meaningful uniquely named assertions; a failure stops the journey. */
function check(name,value){assert(value,name);assert(!checks.includes(name));checks.push(name);console.log('PASS '+name)}
(async()=>{try{
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(cfg.root,'chrome-decision-'+cfg.resultState)});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const label=lang+'-'+theme,context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width:390,height:844});
  const digest=require('node:crypto').createHash('sha256').update(cfg.address).digest('hex');await context.setCookie({name:'swarm_session_'+digest.slice(0,12),value:cfg.key,url:'http://'+cfg.address,httpOnly:true,sameSite:'Strict'});
  page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});page.on('requestfailed',r=>errors.push(new URL(r.url()).pathname));page.on('request',r=>{if(!['GET','HEAD'].includes(r.method()))requests.push(r.method())});page.on('response',r=>{if(r.status()>=400)errors.push(new URL(r.url()).pathname+':'+r.status())});
  const response=page.waitForResponse(r=>{const u=new URL(r.url());return u.pathname==='/api/v1/mission'&&u.searchParams.get('action')==='status'});
  await page.goto('http://'+cfg.address+'/?'+new URLSearchParams({work:cfg.work,view:'conduite',lang,theme}),{waitUntil:'networkidle0'});
  const actual=await response,value=await actual.json();await page.waitForFunction(()=>!document.querySelector('#main-content').hasAttribute('aria-busy'));
  check(label+'-real-'+cfg.resultState,actual.status()===200&&value.tasks[0].result.state===cfg.resultState&&value.validated===0);
  check(label+'-visible-cause',await page.$eval('#conduite-plan',(e,marker)=>e.textContent.includes(marker),cfg.markers[lang]));
  check(label+'-no-global-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.screenshot({path:path.join(cfg.output,label+'-'+cfg.resultState+'-390.png'),fullPage:true});await context.close();
 }
 check('no-browser-errors',errors.length===0);check('no-mutations',requests.length===0);
 console.log('DECISION_BROWSER_PASS '+checks.length+' assertions');
 }finally{fs.writeFileSync(path.join(cfg.output,cfg.resultState+'-browser-result.json'),JSON.stringify({checks,errors,requests},null,2));if(browser)await browser.close()}
})().catch(e=>{console.error(e.stack);process.exitCode=1});
