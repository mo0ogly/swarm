'use strict';
/** Read-only installed Journaux checks. Cookie and actual log samples arrive
 * only on private stdin, are never printed or persisted. Retain digests/counts,
 * not raw messages. Optional transcript samples check real absence or metadata
 * counts, no capture content retained. No POST/provider/export/download/watchdog. */
const fs=require('node:fs'),assert=require('node:assert/strict'),crypto=require('node:crypto'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],observations=[],errors=[],failed=[],httpErrors=[],writes=[];let browser;
/** Unique assertions describe installed behavior, not observations/digests. */
function check(name,value){assert(value,name);assert(!checks.includes(name),'duplicate-assertion');checks.push(name)}
(async()=>{try{browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
 for(const sample of cfg.samples){const label=sample.work_id+'-'+sample.language,context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});page.on('requestfailed',r=>failed.push(new URL(r.url()).pathname));page.on('response',r=>{if(r.status()>=400)httpErrors.push({path:new URL(r.url()).pathname,status:r.status()})});page.on('request',r=>{if(!['GET','HEAD'].includes(r.method()))writes.push(r.method())});
  const split=cfg.cookie.indexOf('=');await context.setCookie({name:cfg.cookie.slice(0,split),value:cfg.cookie.slice(split+1),url:cfg.base,httpOnly:true,sameSite:'Strict'});
  await page.goto(cfg.base+'/?'+new URLSearchParams({work:sample.work_id,lang:sample.language,theme:sample.language==='en'?'sombre':'etat',view:'logs'}),{waitUntil:'networkidle0'});await page.waitForFunction(()=>!document.querySelector('#logs').hasAttribute('aria-busy')&&!document.querySelector('#log-agent').disabled);
  check(label+'-explicit-agent-required',await page.$eval('#log-agent',e=>e.value==='')&&await page.$eval('#log-lines',e=>e.textContent===''));
  await page.select('#log-agent',sample.agent_id);await page.waitForFunction(()=>!document.querySelector('#logs').hasAttribute('aria-busy')&&!document.querySelector('#log-prev').disabled);
  const text=await page.$eval('#log-lines',e=>e.textContent),expected=sample.rows.map(r=>'#'+r.seq+' '+r.at+' ['+r.kind+'] '+r.message).join('\n');
  check(label+'-exact-installed-history',text===expected);check(label+'-literal-text',await page.$eval('#log-lines',e=>!e.querySelector('*')));await page.focus('#log-lines');check(label+'-keyboard-focus',await page.$eval('#log-lines',e=>e===document.activeElement&&!!e.getAttribute('aria-label')));check(label+'-live-disabled',await page.$eval('#log-live',e=>e.disabled));
  if(sample.transcript){
   await page.$eval('#native-agent-transcript',e=>e.open=true);check(label+'-transcript-explicitly-empty-before-read',await page.$eval('#native-transcript-text',e=>e.textContent===''));
   await page.click('#native-transcript-read');await page.waitForFunction(()=>!document.querySelector('#native-agent-transcript').hasAttribute('aria-busy')&&!document.querySelector('#native-transcript-read').disabled);
   const actual=await page.$eval('#native-transcript-text',e=>e.textContent),status=await page.$eval('#native-transcript-status',e=>e.textContent);
   if(sample.transcript.state==='unavailable')check(label+'-transcript-actual-absence',actual===''&&status.includes(sample.language==='en'?'No transcript retained':'Aucun transcript retenu'));
   else check(label+'-transcript-actual-event-count',actual.split('\n').filter(Boolean).length===sample.transcript.transcript.entries.length);
   check(label+'-transcript-no-markup',await page.$eval('#native-transcript-text',e=>!e.querySelector('*')));await page.focus('#native-transcript-text');check(label+'-transcript-keyboard',await page.$eval('#native-transcript-text',e=>e===document.activeElement&&!!e.getAttribute('aria-label')));
  }
  observations.push({work_id:sample.work_id,language:sample.language,rows:sample.rows.length,text_sha256:crypto.createHash('sha256').update(text).digest('hex')});await context.close();
 }
 check('no-read-HTTP-errors',httpErrors.length===0);check('no-browser-errors',errors.length===0);check('no-failed-requests',failed.length===0);check('no-writes',writes.length===0);fs.writeFileSync(cfg.result,JSON.stringify({status:'PASS',checks,observations,errors,failed,httpErrors,writes},null,2));console.log('INSTALLED_AGENT_LOGS_PASS '+checks.length+' assertions');
 }finally{if(browser)await browser.close()}
})().catch(e=>{console.error(e.stack);process.exitCode=1});
