'use strict';
/** Served explicit visit/readback; client-only injected replies are labelled.
 * The actual server writes only an owned inactive fixture bookmark. No provider,
 * global watchdog, polling, automatic retry or live Root write. */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer'),rt=require('./helpers/browser_verification_runtime.cjs'),{finishOwnedResources}=require('./helpers/verification_cleanup.cjs');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),out=path.resolve(cfg.output),repo=path.resolve(__dirname,'..');fs.mkdirSync(out);let browser,runtime,primary;const checks=[],errors=[];
(async()=>{try{
 runtime=rt.begin(repo,out);browser=await puppeteer.launch({...rt.launchOptions,headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',userDataDir:path.join(cfg.root,'chrome-visit'),args:['--no-sandbox']});await rt.recordBrowser(runtime,browser);
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);let reject;const failure=new Promise((_,r)=>reject=r),guard=p=>Promise.race([p,failure]);page.on('pageerror',e=>{errors.push(e.message);reject(e)});page.on('requestfailed',r=>{const e=Error('Resource failed: '+new URL(r.url()).pathname);errors.push(e.message);reject(e)});
  let posts=0;page.on('request',r=>{if(r.method()==='POST'&&new URL(r.url()).pathname==='/api/v1/visit')posts++});
  await page.setViewport({width:1440,height:1000});await guard(page.goto('http://'+cfg.address+'/?lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'}));await page.type('#access-key',cfg.key);await guard(Promise.all([page.waitForNavigation({waitUntil:'networkidle0'}),page.click('button[type=submit]')]));await guard(page.goto('http://'+cfg.address+'/native-runtime.html?'+new URLSearchParams({lang,theme,work:cfg.work}),{waitUntil:'networkidle0'}));
  await guard(page.waitForSelector('[data-session-visit]'));assert.equal(posts,0);assert.equal(await page.$eval('[data-visit-mark]',e=>e.disabled),true);await page.locator('[data-session-visit] > summary').click();
  const action=async selector=>{await page.locator(selector).click();await guard(page.waitForFunction(()=>!document.querySelector('[data-visit-read]').disabled))};
  await action('[data-visit-read]');assert.equal(await page.$eval('[data-visit-mark]',e=>e.disabled),false);assert.equal(posts,0);await action('[data-visit-mark]');assert.equal(posts,1);assert.equal(await page.$eval('[data-visit-status]',e=>e.textContent),lang==='fr'?'Consultation enregistrée. Aucune tâche n’est validée.':'Consultation recorded. No task has been accepted.');
  const prior=await page.$eval('[data-session-visit] dl',e=>e.textContent);
  // Actual write executes, then the client alone loses its valid response.
  await page.evaluate(()=>{window.visitOriginalFetch=window.fetch.bind(window);window.fetch=async(i,o)=>{const r=await window.visitOriginalFetch(i,o);return new URL(String(i),location.href).pathname==='/api/v1/visit'?new Response('{}',{status:200,headers:{'Content-Type':'application/json'}}):r}});
  await action('[data-visit-mark]');assert.equal(posts,2);assert.equal(await page.$eval('[data-visit-mark]',e=>e.disabled),true);assert.equal(await page.$eval('[data-session-visit] dl',e=>e.textContent),prior);assert(await page.$eval('[data-visit-status]',e=>e.classList.contains('alert')));await page.evaluate(()=>window.fetch=window.visitOriginalFetch);await action('[data-visit-read]');assert.equal(posts,2);assert.equal(await page.$eval('[data-visit-mark]',e=>e.disabled),false);
  // Client-only component fixture: high exact wire revision and literal hostile
  // actor; intercepted hooks never write these synthetic values to the server.
  const exact=await page.evaluate(async()=>{
   const host=document.createElement('section');document.body.append(host);const C=SwarmRecordedCursor,work=C.parse('{"id":"fixture-high","revision":9007199254740993}');let intent;
   const view=()=>C.parse('{"schema_version":1,"work_id":"fixture-high","work_revision":"9007199254740993","exists":true,"exact_visit_revision":"9007199254740993","visit":{"revision":9007199254740993,"at":"fixture","operator":"<img src=x onerror=window.visitInjected=true>"}}');
   SwarmSessionVisit.mount(host,work,{show:async()=>view(),mark:async(_,value)=>{intent=value;return {saved:true,state:view()}}});
   host.querySelector('details').open=true;host.querySelector('[data-visit-read]').click();await new Promise(resolve=>queueMicrotask(resolve));host.querySelector('[data-visit-mark]').click();await new Promise(resolve=>queueMicrotask(resolve));
   const result={intent,html:host.querySelectorAll('img').length,text:host.textContent,injected:window.visitInjected===true};host.remove();return result;
  });assert.equal(exact.intent.expected_revision,'9007199254740993');assert.equal(exact.html,0);assert(!exact.injected);assert(exact.text.includes('<img'));
  const contrast=await page.$eval('[data-session-visit]',section=>{
   const rgba=raw=>{const n=raw.match(/[\d.]+/g)?.map(Number);if(!n||n.length<3)throw Error('Unsupported computed color');return[n[0],n[1],n[2],n[3]??1]},blend=(fg,bg)=>fg.slice(0,3).map((v,i)=>v*fg[3]+bg[i]*(1-fg[3])),background=node=>{const chain=[];for(let n=node;n;n=n.parentElement)chain.push(rgba(getComputedStyle(n).backgroundColor));let c=[255,255,255];for(const b of chain.reverse())c=blend(b,c);return c},luminance=c=>c.map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);
   return [...section.querySelectorAll('summary,button,a,p,dt,dd,h4')].filter(node=>node.getClientRects().length&&!node.disabled).map(node=>{const style=getComputedStyle(node),bg=background(node),fg=blend(rgba(style.color),bg),a=luminance(fg),b=luminance(bg);return {tag:node.tagName,ratio:(Math.max(a,b)+.05)/(Math.min(a,b)+.05),minimum:4.5}})
  });assert(contrast.length>0);for(const row of contrast)assert(row.ratio>=row.minimum,JSON.stringify(row));
  for(const width of [1440,320]){await page.setViewport({width,height:1000});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:path.join(out,lang+'-'+theme+'-'+width+'.png'),fullPage:true})}
  checks.push({lang,theme,explicit_only:true,real_mark_readback:true,actual_write_lost_client_reply_readback:true,client_exact_high_integer_safe_text:true,desktop_mobile:true,contrast});await context.close();console.log('PASS visit',lang,theme);
 }
 assert.deepEqual(errors,[]);const proof=rt.finish(runtime,repo);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks,errors,runtime:proof},null,2)+'\n');console.log('SESSION_VISIT_UI_VERIFIED 4');
}catch(error){primary={error};fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({status:'FAIL',message:error.message,stack:error.stack,errors},null,2))}
await finishOwnedResources(primary,[()=>browser?.close()]);
})().catch(e=>{console.error(e.message);process.exitCode=1});
