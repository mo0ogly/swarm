'use strict';
/** Exercise the actual native login over TCP in isolated Chrome contexts.
 * Sensitive config arrives only on stdin; never log the key/cookie/input value.
 * Uses no page/CDP/global watchdog. Failed fixture ownership stays with Python.
 * Canonical assets and actual npm/Node/Chrome inputs are retained by existing tools.
 */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const runtimeTools=require('./helpers/browser_verification_runtime.cjs'),{finishOwnedResources}=require('./helpers/verification_cleanup.cjs');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),repo=path.resolve(__dirname,'..'),output=path.resolve(cfg.output);
fs.mkdirSync(output,{recursive:false});let browser,primary,success=false;const checks=[],errors=[],consoleErrors=[],responses=[];
/** Compute WCAG relative luminance/contrast for the rendered opaque semantic pairs. */
function contrast(a,b){const lum=s=>{const v=s.match(/[\d.]+/g).slice(0,3).map(Number).map(n=>{n/=255;return n<=0.04045?n/12.92:((n+.055)/1.055)**2.4});return .2126*v[0]+.7152*v[1]+.0722*v[2]};const x=lum(a),y=lum(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05)}
(async()=>{
 try{
 const runtime=runtimeTools.begin(repo,output);
 browser=await puppeteer.launch({...runtimeTools.launchOptions,headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',userDataDir:path.join(cfg.root,'chrome'),args:['--no-sandbox']});await runtimeTools.recordBrowser(runtime,browser);
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const label=lang+'-'+theme;console.log('RUN native-login '+label);
  const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  page.on('pageerror',e=>errors.push({case:label,message:e.message}));page.on('requestfailed',r=>errors.push({case:label,path:new URL(r.url()).pathname,failure:r.failure().errorText}));
  page.on('console',m=>{if(m.type()==='error')consoleErrors.push({case:label,message:m.text()})});
  page.on('response',r=>{if(r.status()>=400){const pathname=new URL(r.url()).pathname;responses.push({case:label,path:pathname,status:r.status()});if(!((pathname==='/login'&&r.status()===401)||(pathname==='/api/v1/works'&&r.status()===403)))errors.push({case:label,path:pathname,status:r.status()})}});
  await page.setViewport({width:1440,height:1000});const loaded=await page.goto('http://'+cfg.address+'/login?lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'});assert(loaded.status()===200,'Login document must load');assert.deepEqual(errors,[],'Resource load must succeed before interaction');
  assert(await page.$eval('html',e=>e.lang)===lang);assert(await page.$eval('html',e=>e.dataset.theme)===theme);assert(await page.$eval('h1',e=>e.textContent)===(lang==='en'?'Sign in to Swarm':'Connexion à Swarm'));
  const denied=await page.evaluate(async()=>{const r=await fetch('/api/v1/works');return {status:r.status,body:await r.text()}});assert(denied.status===403&&denied.body==='web-access-refused','Private API must refuse before login');
  const pairs=[];
  for(const width of [1440,390,320]){
   await page.setViewport({width,height:1000});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'Login viewport overflow');
   const colors=await page.evaluate(()=>['body','h1','label','button','footer p'].map(selector=>{const element=document.querySelector(selector),s=getComputedStyle(element);let parent=element,background;while(parent){background=getComputedStyle(parent).backgroundColor;if(background!=='rgba(0, 0, 0, 0)'&&background!=='transparent')break;parent=parent.parentElement}return{selector,color:s.color,background,fontSize:parseFloat(s.fontSize),weight:parseInt(s.fontWeight,10)||400}}));
   for(const c of colors){const ratio=contrast(c.color,c.background),large=c.fontSize>=24||(c.fontSize>=18.66&&c.weight>=700);assert(ratio>=(large?3:4.5),'Rendered contrast below AA for '+label+' '+c.selector);pairs.push({width,...c,ratio})}
   if(width!==390)await page.screenshot({path:path.join(output,label+'-'+width+'.png'),fullPage:true});
  }
  await page.keyboard.press('Tab');assert(await page.$eval('#access-key',e=>document.activeElement===e),'First form control keyboard focus');assert(await page.$eval('#access-key',e=>parseFloat(getComputedStyle(e).outlineWidth)>0),'Visible keyboard focus');
  await page.type('#access-key','wrong-key');await page.click('button[type=submit]');await page.waitForFunction(()=>!document.querySelector('#access-error').hidden&&!document.querySelector('button').disabled);
  assert(await page.$eval('#access-error',e=>e.textContent)===(lang==='en'?'Access key rejected. Try again.':'Clé d’accès refusée. Réessayez.'));assert(await page.$eval('#access-key',e=>document.activeElement===e),'Rejected key focus restored');assert((await page.cookies()).length===0,'Refusal must not create cookie');
  await page.$eval('#access-key',e=>e.value='');await page.type('#access-key',cfg.key);await page.$eval('input[name=next]',e=>e.value='/api/v1/works');
  await page.$eval('#access-error',e=>e.hidden=true);
  const terminal=page.waitForFunction(()=>{const e=document.querySelector('#access-error'),b=document.querySelector('button');return e&&b&&!e.hidden&&!b.disabled}).then(()=>{throw Error('Valid login ended in a visible form error')});
  await Promise.all([Promise.race([page.waitForNavigation({waitUntil:'networkidle0'}),terminal]),page.click('button[type=submit]')]);assert(await page.$eval('body',e=>e.textContent.trim())==='[]','Post-login protected Work consumer');
  const cookies=await page.cookies();assert(cookies.length===1,'One scoped cookie required');const cookie=cookies[0];assert(cookie.value===cfg.key,'Scoped cookie content');assert(cookie.name.startsWith('swarm_session_')&&cookie.httpOnly&&cookie.sameSite==='Strict'&&cookie.path==='/','Scoped browser cookie protections');assert(cookie.expires>0,'Persisted cookie lifetime consumed');
  checks.push({case:label,status:'PASS',viewports:[1440,390,320],contrast:pairs,form_rejection:true,keyboard_focus:true,actual_http_cookie:true,protected_consumer:[],cookie:{httpOnly:cookie.httpOnly,sameSite:cookie.sameSite,path:cookie.path,persistent:cookie.expires>0}});console.log('PASS native-login '+label);await context.close();
 }
 assert.deepEqual(errors,[]);assert(responses.filter(r=>r.status===401).length===4&&responses.filter(r=>r.status===403).length===4,'Expected rejection witnesses');
 assert(consoleErrors.every(e=>/Failed to load resource: the server responded with a status of (401|403)/.test(e.message)),'Unexpected console error');
 const actual=runtimeTools.finish(runtime,repo);fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:'PASS',scope:'Actual native TCP login/browser, shared assets, four isolated FR/EN/theme contexts; full cockpit/Admin/agent identity unproven',checks,errors,expected_http_refusals:responses,console_errors:consoleErrors,runtime:actual},null,2)+'\n');success=true;
 }catch(error){primary={error}}
 await finishOwnedResources(primary,[async()=>{if(browser)await browser.close()}],async failures=>{if(!success||failures.length)fs.writeFileSync(path.join(output,'failure.json'),JSON.stringify({status:'FAIL',errors,checks,causes:failures.map(e=>({name:e?.name,message:e?.message}))},null,2)+'\n')});
})().catch(e=>{console.error(e);process.exitCode=1});
