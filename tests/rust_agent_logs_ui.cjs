'use strict';
/** Existing Journaux page on real Rust HTTP, plus explicitly marked malformed
 * and delayed client responses after valid witnesses. No global/CDP watchdog,
 * real provider, mutation or credential persistence. Outputs are owned fixtures. */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[],httpErrors=[],failed=[],posts=[],expectedStatusFailures=[],expectedConsoleErrors=[];let browser,passed=false;
/** Named observable assertions, no zero-test success or repeated labels. */
function check(name,value){assert(value,name);assert(!checks.includes(name),'duplicate '+name);checks.push(name);console.log('PASS '+name)}
(async()=>{try{
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(cfg.root,'chrome-logs')});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  let expectedStatus=false;const label=lang+'-'+theme,context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  const digest=require('node:crypto').createHash('sha256').update(cfg.address).digest('hex');await context.setCookie({name:'swarm_session_'+digest.slice(0,12),value:cfg.key,url:'http://'+cfg.address,httpOnly:true,sameSite:'Strict'});
  page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')(expectedStatus?expectedConsoleErrors:errors).push(m.text())});page.on('requestfailed',r=>failed.push(new URL(r.url()).pathname));page.on('request',r=>{if(r.method()!=='GET'&&r.method()!=='HEAD')posts.push(r.method())});page.on('response',r=>{if(r.status()>=400)(expectedStatus?expectedStatusFailures:httpErrors).push({path:new URL(r.url()).pathname,status:r.status()})});
  await page.goto('http://'+cfg.address+'/?'+new URLSearchParams({work:cfg.work,view:'logs',lang,theme}),{waitUntil:'networkidle0'});await page.waitForFunction(()=>!document.querySelector('#logs').hasAttribute('aria-busy')&&!document.querySelector('#log-agent').disabled);
  check(label+'-explicit-agent-required',await page.$eval('#log-agent',e=>e.value==='')&&await page.$eval('#log-lines',e=>e.textContent===''));
  await page.select('#log-agent','session');await page.waitForFunction(()=>!document.querySelector('#logs').hasAttribute('aria-busy')&&!document.querySelector('#log-prev').disabled);
  const expected=cfg.latest.rows.map(r=>'#'+r.seq+' '+r.at+' ['+r.kind+'] '+r.message).join('\n');
  check(label+'-exact-latest-CLI-text',await page.$eval('#log-lines',(e,v)=>e.textContent===v,expected));check(label+'-literal-reader-no-markup',await page.$eval('#log-lines',e=>!e.querySelector('*')));
  await require('./rust_agent_transcript_flow.cjs')({page,cfg,label,check});
  await page.focus('#log-lines');check(label+'-keyboard-named-focus',await page.$eval('#log-lines',e=>e===document.activeElement&&!!e.getAttribute('aria-label')));
  check(label+'-no-automatic-live',await page.$eval('#log-live',e=>e.disabled&&e.getAttribute('aria-pressed')==='false'));
  await page.type('#log-search','Processus récolté');check(label+'-filter-loaded-page',await page.$eval('#log-lines',e=>e.textContent.includes('Processus récolté')&&!e.textContent.includes('Départ autorisé')));
  await page.evaluate(()=>{document.querySelector('#log-search').value='';document.querySelector('#log-search').dispatchEvent(new Event('input',{bubbles:true}))});
  await page.click('#log-prev');await page.waitForFunction(seq=>!document.querySelector('#logs').hasAttribute('aria-busy')&&document.querySelector('#log-lines').textContent.startsWith('#'+seq+' '),{},cfg.first.rows[0].seq);
  check(label+'-first-page-is-CLI-first',await page.$eval('#log-lines',(e,v)=>e.textContent===v,cfg.first.rows.map(r=>'#'+r.seq+' '+r.at+' ['+r.kind+'] '+r.message).join('\n')));
  const firstEnd=cfg.first.rows.at(-1).seq;await page.click('#log-next');await page.waitForFunction(()=>!document.querySelector('#logs').hasAttribute('aria-busy')&&!document.querySelector('#log-prev').disabled);
  check(label+'-forward-empty-page-not-stale',await page.$eval('#log-next',e=>e.disabled)&&await page.$eval('#log-export',e=>e.disabled)&&await page.$eval('#journal-console-state',e=>e.textContent.includes(document.documentElement.lang==='en'?'No recorded rows':'Aucune ligne enregistrée')));
  await page.click('#log-prev');await page.waitForFunction(()=>!document.querySelector('#logs').hasAttribute('aria-busy')&&!document.querySelector('#log-export').disabled);
  await page.evaluate(()=>{window.logExportOriginal=URL.createObjectURL;URL.createObjectURL=b=>{window.logExportBlob=b;return window.logExportOriginal(b)}});await page.click('#log-export');const exported=await page.evaluate(async()=>window.logExportBlob.text());
  check(label+'-export-exact-visible-NDJSON',exported===cfg.first.rows.map(r=>JSON.stringify(r)).join('\n')+'\n');await page.evaluate(()=>{URL.createObjectURL=window.logExportOriginal;delete window.logExportOriginal;delete window.logExportBlob});
  for(const width of [1440,390]){await page.setViewport({width,height:1000});check(label+'-no-global-overflow-'+width,await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));if(width===390)await page.screenshot({path:path.join(cfg.output,label+'-logs-390.png'),fullPage:true})}
  await page.select('#work',cfg.other);await page.waitForFunction(()=>!document.querySelector('#main-content').hasAttribute('aria-busy')&&!document.querySelector('#logs').hasAttribute('aria-busy'));
  check(label+'-empty-other-mission-clears-export',await page.$eval('#log-agent',e=>e.disabled)&&await page.$eval('#log-export',e=>e.disabled)&&await page.$eval('#log-lines',e=>e.textContent===''));
  if(label==='fr-etat'){
   await page.select('#work',cfg.work);await page.waitForFunction(()=>!document.querySelector('#log-agent').disabled&&!document.querySelector('#logs').hasAttribute('aria-busy'));
   await page.evaluate(()=>{window.logsFetch=window.fetch;window.fetch=async(...args)=>{const r=await window.logsFetch(...args);if(new URL(args[0],location.href).pathname!=='/api/v1/agent-logs')return r;const v=await r.json();v.rows[0].message='Client fixture <img src=x onerror=alert(1)> & <b>literal</b>';return new Response(JSON.stringify(v),{status:200,headers:{'Content-Type':'application/json'}})}});
   await page.select('#log-agent','session');await page.waitForFunction(()=>!document.querySelector('#logs').hasAttribute('aria-busy')&&document.querySelector('#log-lines').textContent.includes('Client fixture'));
   check('client-HTML-text-not-executed',await page.$eval('#log-lines',e=>e.textContent.includes('<img')&&!e.querySelector('*')));
   await page.evaluate(()=>window.fetch=window.logsFetch);
   await page.evaluate(()=>{window.logsFetch=window.fetch;window.fetch=(...args)=>new URL(args[0],location.href).pathname==='/api/v1/agent-logs'?Promise.resolve(new Response('{}',{status:200,headers:{'Content-Type':'application/json'}})):window.logsFetch(...args)});
   await page.select('#log-agent','session');await page.waitForFunction(()=>document.querySelector('#journal-console-state').classList.contains('alert'));
   check('malformed-not-empty-success-or-stale-export',await page.$eval('#log-lines',e=>e.textContent==='')&&await page.$eval('#log-export',e=>e.disabled)&&await page.$eval('#journal-console-state',e=>e.textContent.includes('agent-logs-view-invalid')));
   await page.evaluate(()=>window.fetch=window.logsFetch);await page.click('#log-prev');await page.waitForFunction(()=>!document.querySelector('#logs').hasAttribute('aria-busy')&&!document.querySelector('#log-next').disabled);check('explicit-recovery-after-malformed',await page.$eval('#journal-console-state',e=>!e.classList.contains('alert')));
   await page.evaluate(()=>{window.fetch=(...args)=>new URL(args[0],location.href).pathname==='/api/v1/agent-logs'?new Promise(resolve=>{window.releaseLogs=()=>window.logsFetch(...args).then(r=>{const json=r.json.bind(r);r.json=async()=>{const v=await json();window.logsBodyRead=true;return v};resolve(r)})}):window.logsFetch(...args)});
   await page.click('#log-prev');await page.waitForFunction(()=>typeof window.releaseLogs==='function');check('pending-clears-old-text',await page.$eval('#log-lines',e=>e.textContent===''));
   await page.select('#work',cfg.other);await page.waitForFunction(()=>!document.querySelector('#logs').hasAttribute('aria-busy'));await page.evaluate(()=>window.releaseLogs());await page.waitForFunction(()=>window.logsBodyRead===true);
   check('late-old-agent-response-ignored',await page.$eval('#log-lines',e=>e.textContent==='')&&await page.$eval('#log-export',e=>e.disabled));await page.evaluate(()=>{window.fetch=window.logsFetch;delete window.logsFetch;delete window.releaseLogs;delete window.logsBodyRead});
  }
  if(label==='fr-etat'){
   await page.select('#work',cfg.work);await page.waitForFunction(()=>!document.querySelector('#main-content').hasAttribute('aria-busy')&&!document.querySelector('#logs').hasAttribute('aria-busy'));
   const statusReply=page.waitForResponse(r=>{const u=new URL(r.url());return u.pathname==='/api/v1/mission'&&u.searchParams.get('action')==='status'});
   await page.$eval('[data-view="conduite"]',e=>e.click());const r=await statusReply,v=await r.json();await page.waitForFunction(()=>!document.querySelector('#main-content').hasAttribute('aria-busy'));
   check('leaving-logs-observes-completed-status',r.status()===200&&v.validated===0&&v.tasks[0].result.state==='completed_unproven');
   check('completion-not-validated',v.tasks[0].result.process_state==='completed'&&v.tasks[0].result.validation_state==='not_started');
  }
  await context.close();
 }
 check('logs-no-browser-errors',errors.length===0);check('no-failed-requests',failed.length===0);check('logs-no-HTTP-errors',httpErrors.length===0);check('no-POST',posts.length===0);check('completed-status-no-errors',expectedStatusFailures.length===0&&expectedConsoleErrors.length===0);
 fs.writeFileSync(path.join(cfg.output,'browser-result.json'),JSON.stringify({status:'PASS',checks,errors,httpErrors,failed,posts,expectedStatusFailures,expectedConsoleErrors},null,2));passed=true;console.log('AGENT_LOGS_BROWSER_PASS '+checks.length+' assertions');
 }finally{fs.writeFileSync(path.join(cfg.output,'browser-diagnostics.json'),JSON.stringify({status:passed?'PASS':'FAIL',checks,errors,httpErrors,failed,posts,expectedStatusFailures,expectedConsoleErrors},null,2));if(browser)await browser.close()}
})().catch(e=>{console.error(e.stack);process.exitCode=1});
