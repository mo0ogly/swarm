'use strict';
/** Real Admin object-attribution form on the served Security page and Trust dialog, owned fixture
 * only. One gesture attributes one exact object: digest, absolute project path,
 * a level/compartments pair chosen from the re-read policy (never invented by
 * the page), and a non-empty reason. Explicit preview then apply; refusals and
 * UNKNOWN are visible; FR/EN, both themes, narrow viewport and keyboard path
 * are exercised. No classified content, live project, provider, watchdog,
 * request retry or fabricated frontend response. Migration of existing objects
 * stays an explicit operator decision per object, never a bulk or implicit scan. */
const runtime=require('./helpers/browser_verification_runtime.cjs');
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
assert.equal(process.argv.length,3);const cfg=JSON.parse(fs.readFileSync(process.argv[2])),out=cfg.output,checks=[],errors=[];
const url=new URL(cfg.url);assert.equal(url.hostname,'127.0.0.1');fs.mkdirSync(out,{recursive:true,mode:0o700});
const inputs=JSON.parse(fs.readFileSync(cfg.compiled_inputs)),sha=raw=>crypto.createHash('sha256').update(raw).digest('hex');
/** Keep named assertions separate from credentials and raw request headers. */
function check(name,value){assert(value,name);assert(!checks.includes(name));checks.push(name);console.log('PASS '+name);}
(async()=>{
 const runtimeRecord=runtime.begin(path.resolve(__dirname,'..'),out);
 let browser,failure,closing=false,abort,expectedDenial=false,expectedDenialConsole=false;const unexpected=new Promise((_,reject)=>{abort=reject});
 /** Unexpected diagnostics interrupt pending DOM waits without a time cutoff. */
 function bad(message){errors.push(message);abort(new Error('unexpected-browser-diagnostic'));}
 try {
  browser=await puppeteer.launch({headless:true,executablePath:'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.resolve(out,'profile')});
  await runtime.recordBrowser(runtimeRecord,browser);
  const journey=async()=>{
   let assetsChecked=false,comboIndex=0;
   for(const {lang,theme,width,view,initial_order} of cfg.journeys){
    const prefix=[view,lang,theme,width].join('-'),context=await browser.createBrowserContext(),page=await context.newPage();
    /** One exact object per combo: the producer refuses a second attribution
     * of the same (digest, project, revision) under another event id. */
    const objectDigest=cfg.digests[comboIndex++];
    page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width,height:900});
    page.on('pageerror',()=>bad('pageerror-'+prefix));page.on('console',message=>{if(message.type()==='error'){const location=message.location().url||'';if(expectedDenialConsole&&location.includes('action=blp-object-attribute')&&/status of 40[03]/.test(message.text())){expectedDenialConsole=false;return;}bad('console-error-'+prefix)}});
    page.on('requestfailed',()=>{if(!closing)bad('requestfailed-'+prefix)});
    // Count actual gestures before navigation; initial hydration must not need Admin.
    await page.evaluateOnNewDocument(()=>{globalThis.__blpAdminClicks=0;document.addEventListener('click',e=>{if(e.target?.id==='admin')globalThis.__blpAdminClicks++;},true)});
    let heldInitial,initialRequested;const initialRequest=new Promise(resolve=>{initialRequested=resolve});
    /** Hold only an original asset or initial readonly show request. Cookies,
     * body, destination and HTTP response remain the native fixture's own;
     * release follows observed state/layout, never a timer or fabricated JSON. */
    if(initial_order){await page.setRequestInterception(true);page.on('request',async request=>{
     const u=new URL(request.url());
     const selected=!heldInitial&&((initial_order==='data-before-layout'&&u.pathname==='/trust-workbench.js')||(initial_order==='layout-before-data'&&u.pathname==='/api/v1/trust-actions'&&u.search===''&&request.method()==='GET'));
     if(selected){heldInitial=()=>request.continue();initialRequested();return;}
     try{await request.continue()}catch(error){if(!closing)bad('initial-request-continue-'+prefix)}
    });}

    page.on('response',r=>{if(r.status()>=400&&!(expectedDenial&&r.status()===403&&new URL(r.url()).searchParams.get('action')==='blp-object-attribute-preview'))bad('http-'+r.status()+'-'+new URL(r.url()).pathname+'-'+prefix)});
    const separator=cfg.cookie.indexOf('=');await context.setCookie({name:cfg.cookie.slice(0,separator),value:cfg.cookie.slice(separator+1),domain:'127.0.0.1',path:'/',httpOnly:true,sameSite:'Strict'});
    await page.setViewport({width,height:900});
    let navigationError;const navigation=page.goto(cfg.url+'/trust-actions.html?view='+view+'&lang='+lang+'&theme='+theme,{waitUntil:'load'}).catch(error=>{navigationError=error});
    if(initial_order){
     if(initial_order==='data-before-layout')await page.waitForFunction(()=>/Prêt|Ready/.test(document.getElementById('message')?.textContent||''));
     else await page.waitForFunction(()=>document.body.classList.contains('security-workbench'));
     await initialRequest;await heldInitial();
    }
    await navigation;if(navigationError)throw navigationError;
    await page.evaluate(theme=>{document.documentElement.dataset.theme=theme},theme);
    await page.waitForFunction(()=>document.getElementById('message').textContent.includes('Prêt')||document.getElementById('message').textContent.includes('Ready'));
    if(view==='security'){
     check(prefix+'-initial-no-admin-click',await page.evaluate(()=>globalThis.__blpAdminClicks===0));
     const values=await page.$$eval('#policy input',inputs=>Object.fromEntries(inputs.map(input=>[input.id.slice(2),input.value])));
     check(prefix+'-initial-native-policy-fields',JSON.stringify(values)===JSON.stringify(cfg.initial_policy));
     check(prefix+'-initial-inline-save-disarmed',await page.evaluate(()=>!document.getElementById('settings').contains(document.getElementById('configuration'))&&document.getElementById('save').disabled));
    }
    /** Open the Admin dialog, enter the admin bearer, then re-read the policy. */
    await page.click('#admin');
    if(view==='actions')await page.waitForFunction(()=>document.querySelector('#settings').open);
    else await page.waitForFunction(()=>!document.querySelector('#settings').contains(document.querySelector('#configuration')));
    await page.waitForFunction(()=>!document.getElementById('admin').disabled);await page.click('#security-tab-blp');
    await page.type('#trust-admin-token',cfg.tokens.admin);
    await page.click('#blp-load');check(prefix+'-load-pointer-target',await page.$eval('#blp-load',e=>{const r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return hit===e||e.contains(hit)}));await page.waitForFunction(()=>document.getElementById('blp-status').textContent.length>0);
    /** Reveal the attribution controls: the sub-details is closed by default. */
    const attributionDetails=await page.$$('#blp-settings details');
    for(const d of attributionDetails){if(await page.evaluate(e=>e.querySelector('#blp-attribute-preview')&&e,d)){await page.evaluate(e=>{e.open=true},d);break;}}
    /** The object-attribution form exists with its exact controls. */
    check(prefix+'-attribution-form-present',await page.$('#blp-object-digest')&&await page.$('#blp-object-path')&&await page.$('#blp-object-level')&&await page.$('#blp-object-compartments')&&await page.$('#blp-object-reason')&&await page.$('#blp-attribute-preview')&&await page.$('#blp-attribute-apply'));
    /** Level options come exactly from the re-read policy, never invented. */
    const levels=await page.$$eval('#blp-object-level option',os=>os.map(o=>o.value));
    check(prefix+'-levels-from-policy',JSON.stringify(levels)===JSON.stringify(cfg.levels)&&await page.$eval('#blp-attribute-apply',e=>e.disabled));
    /** Fill the exact object: this combo's fixture digest from the driver. */
    const fill=async(id,text)=>{await page.$eval('#'+id,(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},String(text))};
    await fill('blp-object-digest',objectDigest);await fill('blp-object-path',cfg.project_path);
    await page.select('#blp-object-level','confidential');
    for(const compartment of cfg.compartments)await page.select('#blp-object-compartments',compartment);
    await fill('blp-object-reason','Owned fixture object migration');
    /** Observe the real protected producer before and after preview. */
    const readPolicy=()=>page.evaluate(async token=>{const session=await (await fetch('/api/v1/native-session')).json();const r=await fetch('/api/v1/trust-actions?action=blp-show',{method:'POST',headers:{'Content-Type':'application/json','X-Swarm-CSRF':session.csrf_token,Authorization:'Bearer '+token},body:'{}'});if(!r.ok)throw Error('blp-show-refused');return r.json()},cfg.tokens.admin);
    const beforePreview=JSON.stringify(await readPolicy());
    /** Preview is the first explicit gesture; apply is disabled until it passes. */
    const previewWait=page.waitForResponse(r=>new URL(r.url()).searchParams.get('action')==='blp-object-attribute-preview');
    await page.click('#blp-attribute-preview');const preview=await previewWait;assert.equal(preview.status(),200);
    const previewBody=await preview.json();check(prefix+'-preview-explicit',previewBody.preview===true&&previewBody.applied===false&&previewBody.attributed===false&&previewBody.would_attribute===true);
    check(prefix+'-preview-no-persisted-attribution',JSON.stringify(await readPolicy())===beforePreview);
    await page.waitForFunction(()=>!document.getElementById('blp-attribute-apply').disabled);
    /** Actual DOM edits disarm an admitted preview, including the relocated
     * Security token. Each next preview is an explicit new gesture. */
    await fill('blp-object-reason','Edited explicit fixture reason');
    check(prefix+'-field-edit-disarms',await page.$eval('#blp-attribute-apply',e=>e.disabled));
    let admittedBytes;const freshPreview=async()=>{const response=page.waitForResponse(r=>new URL(r.url()).searchParams.get('action')==='blp-object-attribute-preview');await page.click('#blp-attribute-preview');const r=await response;assert.equal(r.status(),200);assert.equal((await r.json()).would_attribute,true);admittedBytes=r.request().postData();await page.waitForFunction(()=>!document.getElementById('blp-attribute-apply').disabled)};
    await freshPreview();await fill('trust-admin-token',cfg.tokens.admin);
    check(prefix+'-token-edit-disarms',await page.$eval('#blp-attribute-apply',e=>e.disabled));
    if(view==='security')check(prefix+'-token-outside-form',await page.$eval('#configuration',e=>!e.contains(document.getElementById('trust-admin-token'))));
    await freshPreview();check(prefix+'-repreviews-no-persisted-attribution',JSON.stringify(await readPolicy())===beforePreview);
    /** Apply records the SAME event bytes validated without mutation. */
    const applyWait=page.waitForResponse(r=>new URL(r.url()).searchParams.get('action')==='blp-object-attribute');
    await page.click('#blp-attribute-apply');const apply=await applyWait;assert.equal(apply.status(),200);
    const applyBody=await apply.json();check(prefix+'-apply-identical-preview-bytes',apply.request().postData()===admittedBytes);check(prefix+'-apply-explicit',applyBody.runtime_enforced===false&&applyBody.attributed===true);
    /** The apply success text is set only after the post-apply re-read inside
     * the same guarded() turn, so it cannot match while the request is still
     * in flight (blp-status already holds earlier text, which raced here). */
    await page.waitForFunction(()=>/Attribution enregistrée|Attribution saved/.test(document.getElementById('blp-attribute-status').textContent));
    check(prefix+'-no-overflow',await page.$eval('html',e=>e.scrollWidth<=innerWidth+1));
    /** After apply completes, verify dialog Escape/focus or the inline tab
     * keyboard path, according to the actual view. */
    if(view==='actions'){await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#settings').open&&document.activeElement.id==='admin');}
    else{await page.focus('#security-tab-blp');await page.keyboard.press('ArrowLeft');await page.waitForFunction(()=>document.getElementById('security-tab-biba').getAttribute('aria-selected')==='true');await page.keyboard.press('ArrowRight');await page.waitForFunction(()=>document.getElementById('security-tab-blp').getAttribute('aria-selected')==='true');check(prefix+'-inline-keyboard-tabs',await page.$eval('#settings',e=>!e.open));}
    await page.screenshot({path:path.join(out,prefix+'.png'),fullPage:true});
    if(!assetsChecked){
     const assets=await page.evaluate(async()=>{const out={};for(const name of ['trust-native.html','trust-confidentiality.js','trust-native.js','trust-workbench.js','swarm-workbench.css','i18n-en.js']){const r=await fetch('/'+(name==='trust-native.html'?'trust-actions.html':name));if(!r.ok)throw Error('asset-refused');out['web/'+name]=Array.from(new Uint8Array(await r.arrayBuffer()));}return out;});
     check('compiled-attribution-assets',Object.entries(assets).every(([name,bytes])=>sha(Buffer.from(bytes))===inputs[name]));assetsChecked=true;
    }
    await context.close();
   }
   /** Refusal matrix on one context: session alone and a weak bearer each
    * refuse with the typed authorization code and no mutation. */
   const context=await browser.createBrowserContext(),page=await context.newPage();await page.setViewport({width:800,height:600});page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
   page.on('pageerror',()=>bad('pageerror-refusals'));page.on('requestfailed',()=>{if(!closing)bad('requestfailed-refusals')});
   page.on('response',r=>{if(r.status()>=400&&!(new URL(r.url()).searchParams.get('action')==='blp-object-attribute-preview'))bad('http-'+r.status()+'-refusals')});
   const separator=cfg.cookie.indexOf('=');await context.setCookie({name:cfg.cookie.slice(0,separator),value:cfg.cookie.slice(separator+1),domain:'127.0.0.1',path:'/',httpOnly:true,sameSite:'Strict'});
   await page.goto(cfg.url+'/trust-actions.html?view=security&lang=en',{waitUntil:'networkidle0'});
   await page.waitForFunction(()=>document.getElementById('message').textContent.includes('Ready'));
   await page.click('#admin');await page.waitForFunction(()=>!document.querySelector('#settings').contains(document.querySelector('#configuration')));await page.waitForFunction(()=>!document.getElementById('admin').disabled);await page.click('#security-tab-blp');
   await page.type('#trust-admin-token',cfg.tokens.admin);
   await page.click('#blp-load');check('refusal-800x600-load-not-occluded',await page.$eval('#blp-load',e=>{const r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return hit===e||e.contains(hit)}));await page.waitForFunction(()=>document.getElementById('blp-status').textContent.length>0);
   for(const d of await page.$$('#blp-settings details')){if(await page.evaluate(e=>e.querySelector('#blp-attribute-preview')&&e,d)){await page.evaluate(e=>{e.open=true},d);break;}}
   const fill=async(id,text)=>{await page.$eval('#'+id,(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},String(text))};
   await fill('blp-object-digest',cfg.digests[0]);await fill('blp-object-path',cfg.project_path);
   await page.select('#blp-object-level','confidential');
   for(const compartment of cfg.compartments)await page.select('#blp-object-compartments',compartment);
   await fill('blp-object-reason','Owned fixture refusal control');
   /** Session alone (empty admin token): the producer refuses authorization. */
   await fill('trust-admin-token','');expectedDenial=true;expectedDenialConsole=true;
   const bare=page.waitForResponse(r=>new URL(r.url()).searchParams.get('action')==='blp-object-attribute-preview');
   await page.click('#blp-attribute-preview');const bareReply=await bare;assert.equal(bareReply.status(),403);assert.equal((await bareReply.json()).failure.code,'authorization_required');
   await page.waitForFunction(()=>document.getElementById('blp-attribute-preview').disabled===false);
   expectedDenial=false;expectedDenialConsole=false;
   check('session-alone-refused',await page.$eval('#blp-attribute-apply',e=>e.disabled));
   /** Weak bearer: same typed refusal, apply still disabled. */
   await fill('trust-admin-token',cfg.tokens.worker);expectedDenial=true;expectedDenialConsole=true;
   const weak=page.waitForResponse(r=>new URL(r.url()).searchParams.get('action')==='blp-object-attribute-preview');
   await page.click('#blp-attribute-preview');const weakReply=await weak;assert.equal(weakReply.status(),403);assert.equal((await weakReply.json()).failure.code,'authorization_required');
   await page.waitForFunction(()=>document.getElementById('blp-attribute-preview').disabled===false);
   expectedDenial=false;expectedDenialConsole=false;
   check('weak-bearer-refused',await page.$eval('#blp-attribute-apply',e=>e.disabled));
   fs.writeFileSync(path.join(out,'refusal-result.json'),JSON.stringify({session_alone:'403-authorization_required',weak_bearer:'403-authorization_required'},null,2)+'\n');
   await context.close();
  };
  await Promise.race([journey(),unexpected]);check('no-browser-errors',errors.length===0);
 } catch(error){failure=error;}
 finally{closing=true;if(browser)try{await browser.close()}catch(error){failure=failure?new AggregateError([failure,error],'browser and cleanup failed'):error}}
 if(!failure)runtime.finish(runtimeRecord,path.resolve(__dirname,'..'));
 const summary=JSON.stringify({status:failure?'FAIL':'PASS',assertions:checks.length,checks,errors},null,2)+'\n';
 fs.writeFileSync(path.join(out,'result.json'),summary);
 /** The driver reads the journey verdict outside the 0700 evidence tree. */
 if(cfg.public_output)fs.writeFileSync(cfg.public_output,summary);if(failure)throw failure;
 console.log('OBJECT_ATTRIBUTE_UI_PASS '+checks.length);
})().catch(error=>{console.error(error.stack);process.exitCode=1});
