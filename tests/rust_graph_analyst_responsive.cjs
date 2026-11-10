'use strict';
/** Verify native analyst controls at the five documented design widths.
 * Credentials arrive only on stdin. Every page uses a private context; no
 * action POST, provider, retry, global watchdog or workspace overlay is used.
 */
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[];
fs.mkdirSync(cfg.output,{mode:0o700});
function check(name,value){assert(value,name);checks.push(name);console.log('PASS '+name)}
(async()=>{
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
 try{for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const context=await browser.createBrowserContext(),page=await context.newPage(),address='http://'+cfg.address,label=lang+'-'+theme;
  page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});page.on('requestfailed',r=>errors.push(r.url()));page.on('response',r=>{if(r.status()>=400)errors.push(r.status()+' '+r.url())});page.on('request',r=>{if(r.method()==='POST')errors.push('unexpected-post')});
  await context.setCookie({name:'swarm_session_'+crypto.createHash('sha256').update(cfg.address).digest('hex').slice(0,12),value:cfg.key,url:address,httpOnly:true,sameSite:'Strict'});
  await page.goto(address+'/?'+new URLSearchParams({work:cfg.work,lang,theme,view:'conduite'}),{waitUntil:'networkidle0'});
  check(label+'-responsive-native-control',await page.$eval('.graph-task-open',e=>e.dataset.task==='a'));
  await page.click('#pilot-fullscreen');
  for(const width of [1440,1024,760,390,320]){
   await page.setViewport({width,height:844});await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
   if(!await page.$eval('#pilot-analysis',e=>e.open))await page.click('#pilot-analysis-toggle');
   for(const selector of ['#pilot-branch','#pilot-path-from','#pilot-selection-mode']){
    await page.$eval(selector,e=>{const d=e.closest('details');if(!d.open)d.querySelector('summary').click()});
    check(label+'-responsive-'+width+'-'+selector.slice(1),await page.$eval(selector,e=>{const r=e.getBoundingClientRect();return r.width>0&&r.left>=0&&r.right<=innerWidth+1})&&await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    check(label+'-responsive-exclusive-'+width+'-'+selector.slice(1),await page.$$eval('.native-graph-analysis-section[open]',nodes=>nodes.length===1));
   }
   await page.focus('#pilot-selection-mode');await page.keyboard.press('Escape');check(label+'-responsive-escape-'+width,await page.$eval('#pilot-analysis',e=>!e.open&&document.activeElement.id==='pilot-analysis-toggle'));
   await page.screenshot({path:path.join(cfg.output,label+'-'+width+'.png')});
  }
  await context.close();
 }
 check('responsive-no-browser-errors-or-posts',errors.length===0);
 fs.writeFileSync(path.join(cfg.output,'results.json'),JSON.stringify({checks,errors},null,2)+'\n');
 }finally{await browser.close()}
})().catch(error=>{console.error(error);process.exitCode=1});
