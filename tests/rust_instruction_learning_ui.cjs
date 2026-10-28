'use strict';
/** Real protected Rust UI, four locale/theme combinations and actual refusal /
 * lost-response recovery. Inert fixtures only. No global browser watchdog. */
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[],httpErrors=[],consoleErrors=[];let browser;
/** Record meaningful named checks, never just HTTP success or a screenshot. */
function check(name,value){assert(value,name);checks.push(name);console.log('PASS '+name)}
(async()=>{try{
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(cfg.root,'chrome-instruction-learning')});
 const context=await browser.createBrowserContext(),address='http://'+cfg.address;
 await context.setCookie({name:'swarm_session_'+crypto.createHash('sha256').update(cfg.address).digest('hex').slice(0,12),value:cfg.key,url:address,httpOnly:true,sameSite:'Strict'});
 let index=0;
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const label=lang+'-'+theme,page=await context.newPage(),posts=[];page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);page.on('response',r=>{if(r.status()>=400)httpErrors.push({path:new URL(r.url()).pathname,status:r.status()})});page.on('console',m=>{if(m.type()==='error')consoleErrors.push(m.text())});page.on('pageerror',e=>errors.push(e.message));page.on('requestfailed',r=>errors.push(new URL(r.url()).pathname));page.on('request',r=>{if(r.method()==='POST')posts.push(r.url())});
  await page.goto(address+'/?'+new URLSearchParams({lang,theme,work:cfg.works[index++]}),{waitUntil:'networkidle0'});
  await page.waitForFunction(()=>document.querySelector('#native-instruction-history button:not(:disabled)'));
  check(label+'-history-language',await page.$eval('#native-instruction-history',e=>e.textContent.includes(document.documentElement.lang==='en'?'Improved instructions':'Consignes améliorées')));
  check(label+'-navigation-no-write',posts.length===0);
  await page.click('#native-instruction-history summary');
  check(label+'-detail-content',await page.$eval('#native-instruction-history details',e=>e.open&&e.textContent.includes('Remember the verified control')));
  await page.screenshot({path:path.join(cfg.output,'instruction-'+label+'.png'),fullPage:true});
  await page.focus('#native-instruction-history button');await page.keyboard.press('Enter');
  await page.waitForFunction(()=>document.querySelector('#native-instruction-history [data-state="rejected"]'));
  check(label+'-rejection-readback',await page.$eval('#native-instruction-history',e=>e.textContent.includes(document.documentElement.lang==='en'?'Rejected':'Rejetée'))&&posts.length===1);
  check(label+'-keyboard-focus-restored',await page.evaluate(()=>document.activeElement?.dataset.state==='rejected'));
  check(label+'-no-horizontal-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.setViewport({width:390,height:844});check(label+'-mobile-width',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.close();
 }
 const page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);page.on('pageerror',e=>errors.push(e.message));
 await page.goto(address+'/?'+new URLSearchParams({lang:'fr',work:cfg.works[4]}),{waitUntil:'networkidle0'});await page.waitForFunction(()=>document.querySelector('#native-instruction-history button:not(:disabled)'));
 // Forward a real write then lose only its client response; no mocked engine result.
 await page.evaluate(()=>{window.realFetch=window.fetch;window.fetch=async(...args)=>{const r=await window.realFetch(...args);if(new URL(args[0],location.href).searchParams.get('action')==='reject-instructions'&&r.ok)throw new TypeError('fixture-lost-response');return r}});
 await page.click('#native-instruction-history button');await page.waitForFunction(()=>document.querySelector('#native-instruction-history [role="alert"]'));
 check('lost-response-explicit-uncertainty',await page.$eval('#native-instruction-history',e=>e.textContent.includes('peut être enregistrée'))&&await page.$eval('#native-instruction-history button',e=>e.disabled));
 check('lost-response-actual-write',await page.evaluate(async id=>(await(await window.realFetch('/api/v1/planning?work='+id)).json()).tasks[0].instruction_revisions[0].state==='rejected',cfg.works[4]));
 await page.evaluate(()=>{window.fetch=window.realFetch});await page.click('#refresh');await page.waitForFunction(()=>document.querySelector('#native-instruction-history [data-state="rejected"]'));
 check('lost-response-manual-readback-recovered',await page.$eval('#native-instruction-history',e=>!e.querySelector('[role="alert"]')));
 await page.goto(address+'/?'+new URLSearchParams({lang:'en',work:cfg.works[5]}),{waitUntil:'networkidle0'});await page.waitForFunction(()=>document.querySelector('#native-instruction-history button:not(:disabled)'));
 // A legitimate concurrent operator update makes the displayed revision stale.
 await page.evaluate(async id=>{const session=await(await fetch('/api/v1/native-session')).json(),w=await(await fetch('/api/v1/planning?work='+id)).json();const r=await fetch('/api/v1/planning?work='+id+'&action=pause',{method:'POST',headers:{'Content-Type':'application/json','X-Swarm-CSRF':session.csrf_token},body:JSON.stringify({schema_version:1,event_id:'concurrent-'+crypto.randomUUID(),expected_revision:w.revision})});if(!r.ok)throw Error('concurrent witness failed')},cfg.works[5]);
 await page.click('#native-instruction-history button');await page.waitForFunction(()=>document.querySelector('#native-instruction-history [role="alert"]'));
 check('stale-refusal-readable',await page.$eval('#native-instruction-history',e=>e.textContent.includes('Rejection refused.')&&e.textContent.includes('revision_conflict')));
 await page.click('#refresh');await page.waitForFunction(()=>document.querySelector('#native-instruction-history button:not(:disabled)'));
 await page.click('#native-instruction-history button');await page.waitForFunction(()=>document.querySelector('#native-instruction-history [data-state="rejected"]'));check('stale-refusal-fresh-read-recovery',true);
 await require('./helpers/instruction_recovery_race.cjs')(page,address,cfg.works[6]);check('concurrent-refresh-rejection-readback-keeps-new-revision',true);
 check('no-browser-exceptions',errors.length===0);check('healthy-pages-no-HTTP-or-console-errors',httpErrors.length===0&&consoleErrors.length===0);fs.writeFileSync(path.join(cfg.output,'browser-result.json'),JSON.stringify({status:'PASS',checks,errors,httpErrors,consoleErrors},null,2));
 }catch(error){console.error(error.stack);process.exitCode=1}finally{if(browser)await browser.close()}})();
