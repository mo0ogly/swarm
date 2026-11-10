 'use strict';
/** Real embedded scope modal in FR/EN and both themes; own inert Root only. */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const rt=require('./helpers/browser_verification_runtime.cjs'),{finishOwnedResources}=require('./helpers/verification_cleanup.cjs');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),output=path.resolve(cfg.output),repo=path.resolve(__dirname,'..');fs.mkdirSync(output);let browser,primary,runtime;const checks=[],errors=[];
(async()=>{try{
 runtime=rt.begin(repo,output);browser=await puppeteer.launch({...rt.launchOptions,headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',userDataDir:path.join(cfg.root,'chrome-limits'),args:['--no-sandbox']});await rt.recordBrowser(runtime,browser);
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  console.log('RUN run-limits',lang,theme);const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);let reject;const failure=new Promise((_,r)=>reject=r),guard=p=>Promise.race([p,failure]);
  page.on('pageerror',e=>{errors.push(e.message);reject(e)});page.on('requestfailed',r=>{errors.push(new URL(r.url()).pathname);reject(Error('resource failed'))});page.on('console',m=>{if(m.type()==='error'){errors.push(m.text());reject(Error('console error'))}});page.on('response',r=>{if(r.status()>=400){errors.push({path:new URL(r.url()).pathname,status:r.status()});reject(Error('HTTP refused'))}});
  await page.setViewport({width:1440,height:1000});await guard(page.goto('http://'+cfg.address+'/?lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'}));await page.type('#access-key',cfg.key);await guard(Promise.all([page.waitForNavigation({waitUntil:'networkidle0'}),page.click('button[type=submit]')]));await guard(page.goto('http://'+cfg.address+'/native-runtime.html?view=admin&lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'}));await guard(page.waitForFunction(()=>document.querySelector('[data-limits-open]')?.disabled===false));
  await page.click('[data-limits-open]');await guard(page.waitForFunction(()=>document.querySelector('[data-limits-preview]')?.disabled===false));assert.equal(await page.$eval('[data-run-limits] h2',e=>e.textContent),lang==='en'?'Limits for future starts':'Limites des prochains départs');
  const contrast=await page.$eval('[data-run-limits]',dialog=>{
   const rgba=raw=>{const n=raw.match(/[\d.]+/g)?.map(Number);if(!n||n.length<3)throw Error('Unsupported computed color');return[n[0],n[1],n[2],n[3]??1]};
   const blend=(fg,bg)=>fg.slice(0,3).map((v,i)=>v*fg[3]+bg[i]*(1-fg[3]));
   const background=node=>{const chain=[];for(let n=node;n;n=n.parentElement)chain.push(rgba(getComputedStyle(n).backgroundColor));let c=[255,255,255];for(const b of chain.reverse())c=blend(b,c);return c};
   const luminance=c=>c.map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((sum,v,i)=>sum+v*[.2126,.7152,.0722][i],0);
   return [...dialog.querySelectorAll('h2,label,input,textarea,button:not(:disabled)')].map(node=>{const style=getComputedStyle(node),bg=background(node),fg=blend(rgba(style.color),bg),a=luminance(fg),b=luminance(bg),large=parseFloat(style.fontSize)>=24||parseFloat(style.fontSize)>=18.66&&Number(style.fontWeight)>=700;return {tag:node.tagName,ratio:(Math.max(a,b)+.05)/(Math.min(a,b)+.05),minimum:large?3:4.5}})
  });assert(contrast.length>0);for(const row of contrast)assert(row.ratio>=row.minimum,JSON.stringify(row));

  for(const width of [1440,320]){
   await page.setViewport({width,height:900});await page.$eval('[data-limits-field=silence_seconds]',e=>{e.value=String(Number(e.value)+1);e.dispatchEvent(new Event('input',{bubbles:true}))});
   await page.$eval('[data-limits-reason]',e=>{e.value='Owned modal witness';e.dispatchEvent(new Event('input',{bubbles:true}))});
   await page.click('[data-limits-preview]');await guard(page.waitForFunction(()=>document.querySelector('[data-limits-apply]')?.disabled===false));assert(await page.$eval('[data-limits-diff]',e=>!e.hidden&&e.textContent.includes('silence_seconds')));
   await page.click('[data-limits-apply]');await guard(page.waitForFunction(()=>document.querySelector('[data-limits-status]')?.classList.contains('success')));
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));assert(await page.$eval('[data-run-limits]',e=>e.scrollWidth<=e.clientWidth));
   const geometry=await page.$eval('[data-limits-status]',e=>({status:e.getBoundingClientRect().toJSON(),footer:e.closest('.modal-footer').getBoundingClientRect().toJSON(),height:innerHeight}));assert(geometry.status.top>=0&&geometry.status.bottom<=geometry.height,JSON.stringify(geometry));
   assert(await page.$eval('[data-limits-apply]',e=>{const r=e.getBoundingClientRect();return r.top>=0&&r.bottom<=innerHeight}));
   await page.screenshot({path:path.join(output,lang+'-'+theme+'-'+width+'.png')});
  }
  // Frontend-only transport fault after a REAL successful SQL publication.
  // Actual GET recovery proves the recorded revision; no fake backend success.
  await page.setViewport({width:1440,height:1000});
  await page.evaluate(()=>{const original=window.fetch.bind(window);let fail=false;window.fetch=async(input,options)=>{const url=new URL(String(input),location.href);if(url.pathname==='/api/v1/run-limits'&&fail){fail=false;return new Response(JSON.stringify({failure:{code:'owned-readback-unavailable'}}),{status:500,headers:{'Content-Type':'application/json'}})}const response=await original(input,options);if(url.pathname==='/api/v1/run-limits/apply'&&response.ok)fail=true;return response;};window.g9OriginalFetch=original;});
  await page.$eval('[data-limits-field=silence_seconds]',e=>{e.value=String(Number(e.value)+1);e.dispatchEvent(new Event('input',{bubbles:true}))});await page.click('[data-limits-preview]');await guard(page.waitForFunction(()=>document.querySelector('[data-limits-apply]')?.disabled===false));await page.click('[data-limits-apply]');
  await guard(page.waitForFunction(()=>document.querySelector('[data-limits-status]')?.textContent.includes('run-limits-readback-failed')));
  assert(await page.$eval('[data-limits-status]',e=>e.textContent.includes(document.documentElement.lang==='en'?'Recorded; readback unavailable':'Enregistré ; relecture indisponible')));assert(await page.$eval('[data-limits-apply]',e=>e.disabled));
  await page.evaluate(()=>window.fetch=window.g9OriginalFetch);await page.click('[data-limits-reload]');await guard(page.waitForFunction(()=>document.querySelector('[data-limits-preview]')?.disabled===false));
  const postsBefore=await page.evaluate(()=>performance.getEntriesByType('resource').filter(e=>new URL(e.name).pathname==='/api/v1/run-limits/preview').length);
  await page.$eval('[data-limits-field=silence_seconds]',e=>{e.value='invalid';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.click('[data-limits-preview]');await guard(page.waitForFunction(()=>document.querySelector('[data-limits-status]')?.textContent.includes('run-limits-integer-invalid')));assert.equal(await page.evaluate(()=>performance.getEntriesByType('resource').filter(e=>new URL(e.name).pathname==='/api/v1/run-limits/preview').length),postsBefore);await page.click('[data-limits-reload]');await guard(page.waitForFunction(()=>document.querySelector('[data-limits-preview]')?.disabled===false));

  for(const ackMode of ['empty','fabricated']){
   const beforeScope=await page.evaluate(async()=>{const r=await fetch('/api/v1/run-limits?scope=project');const v=await r.json();return {entry:v.exact.entry,history:v.exact.history}});
   await page.evaluate(mode=>{const original=window.fetch.bind(window);let preview;window.g9AckOriginalFetch=original;window.fetch=async(input,options)=>{const url=new URL(String(input),location.href);if(url.pathname==='/api/v1/run-limits/apply'){return new Response(JSON.stringify(mode==='empty'?{applied:true,preview:false}:{...preview,applied:true,preview:false,replayed:false}),{status:200,headers:{'Content-Type':'application/json'}})}const response=await original(input,options);if(url.pathname==='/api/v1/run-limits/preview'&&response.ok)preview=await response.clone().json();return response}},ackMode);
   await page.click('[data-limits-preview]');await guard(page.waitForFunction(()=>document.querySelector('[data-limits-apply]')?.disabled===false));await page.click('[data-limits-apply]');await guard(page.waitForFunction(()=>{const e=document.querySelector('[data-limits-status]');return e?.classList.contains('alert')||e?.classList.contains('success')}));
   assert(await page.$eval('[data-limits-status]',e=>e.classList.contains('alert')),ackMode+' SQL ACK cannot confirm saving');const afterScope=await page.evaluate(async()=>{const r=await fetch('/api/v1/run-limits?scope=project');const v=await r.json();return {entry:v.exact.entry,history:v.exact.history}});assert.deepEqual(afterScope,beforeScope);await page.evaluate(()=>window.fetch=window.g9AckOriginalFetch);await page.click('[data-limits-reload]');await guard(page.waitForFunction(()=>document.querySelector('[data-limits-preview]')?.disabled===false));
  }

  // Source rows remain literal text; rollback is preview followed by confirmation.
  await page.setViewport({width:1440,height:1000});await page.$eval('[data-run-limits] details:last-of-type',e=>e.open=true);const restores=await page.$$('[data-limits-restore]');assert(restores.length>=2);await restores[restores.length-1].click();await guard(page.waitForFunction(()=>document.querySelector('[data-limits-apply]')?.disabled===false));await page.click('[data-limits-apply]');await guard(page.waitForFunction(()=>document.querySelector('[data-limits-status]')?.classList.contains('success')));
  await page.keyboard.press('Escape');await guard(page.waitForFunction(()=>!document.querySelector('[data-run-limits]')&&document.querySelector('[data-limits-open]')===document.activeElement));assert(await page.$eval('[data-limits-open]',e=>e===document.activeElement));checks.push({lang,theme,desktop_mobile:true,save_readback:true,rollback:true,escape_focus:true,contrast,readback_transport_fault_recovery:true,invalid_integer_no_contact:true});await context.close();
 }
 assert.equal(errors.length,0);const proof=rt.finish(runtime,repo);fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:'PASS',checks,errors,runtime:proof},null,2)+'\n');console.log('RUN_LIMITS_UI_VERIFIED 4');
}catch(error){primary={error};fs.writeFileSync(path.join(output,'failure.json'),JSON.stringify({status:'FAIL',message:error.message,stack:error.stack,errors},null,2))}finally{await finishOwnedResources(primary,[()=>browser?.close()])}})();
