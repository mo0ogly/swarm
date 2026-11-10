'use strict';
/** Real Admin editing/reopening of the served quarantine policy on an owned
 * fixture. Private session/profile stay under ignored evidence. No provider,
 * live project, watchdog, request retry or fabricated frontend response. */
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
assert.equal(process.argv.length,3);const cfg=JSON.parse(fs.readFileSync(process.argv[2])),out=cfg.output,checks=[],errors=[];
const url=new URL(cfg.url);assert.equal(url.hostname,'127.0.0.1');fs.mkdirSync(out,{recursive:true,mode:0o700});
const inputs=JSON.parse(fs.readFileSync(cfg.compiled_inputs)),sha=raw=>crypto.createHash('sha256').update(raw).digest('hex');
/** Keep named assertions separate from credentials and raw request headers. */
function check(name,value){assert(value,name);assert(!checks.includes(name));checks.push(name);console.log('PASS '+name);}
(async()=>{
 let browser,failure,closing=false,abort;const unexpected=new Promise((_,reject)=>{abort=reject});
 /** Unexpected diagnostics interrupt pending DOM waits without a time cutoff. */
 function bad(message){errors.push(message);abort(new Error('unexpected-browser-diagnostic'));}
 try {
  browser=await puppeteer.launch({headless:true,executablePath:'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.resolve(out,'profile')});
  const journey=async()=>{
   let assetsChecked=false,invalidChecked=false;
   for(const lang of ['fr','en'])for(const theme of ['sombre','etat'])for(const width of [1280,390]){
    const prefix=[lang,theme,width].join('-'),context=await browser.createBrowserContext(),page=await context.newPage();let expectedInvalid=false,expectedInvalidConsole=false;
    page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width,height:900});
    page.on('pageerror',()=>bad('pageerror-'+prefix));page.on('console',message=>{if(message.type()==='error'){const location=message.location().url||'';if(expectedInvalidConsole&&location.includes('action=blp-policy-preview')&&/status of 400/.test(message.text())){expectedInvalidConsole=false;return;}bad('console-error-'+prefix)}});
    page.on('requestfailed',()=>{if(!closing)bad('requestfailed-'+prefix)});
    page.on('response',r=>{if(r.status()>=400&&!(expectedInvalid&&r.status()===400&&new URL(r.url()).searchParams.get('action')==='blp-policy-preview'))bad('http-'+r.status()+'-'+new URL(r.url()).pathname+'-'+prefix)});
    const separator=cfg.cookie.indexOf('=');await context.setCookie({name:cfg.cookie.slice(0,separator),value:cfg.cookie.slice(separator+1),domain:'127.0.0.1',path:'/',httpOnly:true,sameSite:'Strict'});
    await page.goto(cfg.url+'/trust-actions.html?lang='+lang,{waitUntil:'networkidle0'});await page.waitForFunction(()=>!document.getElementById('admin').disabled);
    if(theme==='etat')await page.click('#theme');
    check(prefix+'-surface',await page.$eval('html',(e,p)=>e.lang===p.lang&&e.dataset.theme===p.theme,{lang,theme}));
    /** Observe persisted policy through the real readonly API, independently of
     * the edit form; no request is intercepted or successful response invented. */
    const read=()=>page.evaluate(async token=>{
     const session=await(await fetch('/api/v1/native-session')).json();
     const response=await fetch('/api/v1/trust-actions?action=blp-show',{method:'POST',headers:{'Content-Type':'application/json','X-Swarm-CSRF':session.csrf_token,Authorization:'Bearer '+token},body:'{}'});
     if(!response.ok)throw Error('policy-read-refused');const data=await response.json();return {revision:data.capability.policy_revision,enabled:data.capability.preparation_quarantined};
    },cfg.token);
    /** Use actual Admin gestures; every open requires a new ephemeral token. */
    const open=async()=>{
     await page.click('#admin');await page.waitForFunction(()=>document.getElementById('settings').open);
     assert.equal(await page.$eval('#trust-admin-token',e=>e.value),'');await page.type('#trust-admin-token',cfg.token);
     await page.click('#blp-load');await page.waitForFunction(()=>document.getElementById('blp-policy').value.length>0&&!document.getElementById('blp-load').disabled);
     if(!await page.$eval('#blp-settings details',e=>e.open))await page.click('#blp-settings details > summary');
    };
    /** Change the actual JSON editor and dispatch its ordinary input event. */
    const edit=async value=>{
     const raw=await page.$eval('#blp-policy',(e,v)=>{const p=JSON.parse(e.value);p.preparation_quarantine=v;return JSON.stringify(p,null,2)},value);
     await page.click('#blp-policy');await page.keyboard.down('Control');await page.keyboard.press('A');await page.keyboard.up('Control');await page.type('#blp-policy',raw);
     await page.click('#blp-reason');await page.keyboard.down('Control');await page.keyboard.press('A');await page.keyboard.up('Control');await page.type('#blp-reason','Owned Admin quarantine fixture');
    };
    const preview=async()=>{
     const response=page.waitForResponse(r=>new URL(r.url()).searchParams.get('action')==='blp-policy-preview');await page.click('#blp-preview');const r=await response;assert.equal(r.status(),200);
     await page.waitForFunction(()=>!document.getElementById('blp-save').disabled);
    };
    const save=async()=>{await page.click('#blp-save');await page.waitForFunction(()=>!document.getElementById('blp-load').disabled&&document.getElementById('blp-save').disabled)};
    await open();check(prefix+'-loaded-disabled',(await read()).enabled===false&&await page.$eval('#blp-save',e=>e.disabled));
    if(!assetsChecked){
     const assets=await page.evaluate(async()=>{const out={};for(const name of ['trust-native.html','trust-confidentiality.js','i18n-en.js']){const r=await fetch('/'+(name==='trust-native.html'?'trust-actions.html':name));if(!r.ok)throw Error('asset-refused');out['web/'+name]=Array.from(new Uint8Array(await r.arrayBuffer()));}return out;});
     check('compiled-quarantine-assets',Object.entries(assets).every(([name,bytes])=>sha(Buffer.from(bytes))===inputs[name]));assetsChecked=true;
    }
    if(!invalidChecked){
     const before=await read();await edit(null);expectedInvalid=true;expectedInvalidConsole=true;
     const response=page.waitForResponse(r=>new URL(r.url()).searchParams.get('action')==='blp-policy-preview');await page.click('#blp-preview');assert.equal((await response).status(),400);
     await page.waitForFunction(()=>document.getElementById('admin-message').textContent.includes('format')&&!document.getElementById('blp-preview').disabled);expectedInvalid=false;
     check('invalid-quarantine-type-refuses-without-mutation',JSON.stringify(await read())===JSON.stringify(before)&&await page.$eval('#blp-save',e=>e.disabled));
     await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.getElementById('settings').open);await open();invalidChecked=true;
    }
    const before=await read();await edit(true);await preview();check(prefix+'-preview-readonly',JSON.stringify(await read())===JSON.stringify(before));
    await save();const expected=lang==='en'?'Preparation sends quarantined':'Envois de préparation en quarantaine';
    check(prefix+'-saved-enabled',(await read()).enabled===true&&await page.$eval('#blp-status',(e,text)=>e.textContent.includes(text),expected));
    await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.getElementById('settings').open);await open();
    check(prefix+'-reopened-enabled',(await read()).enabled===true&&await page.$eval('#blp-policy',e=>JSON.parse(e.value).preparation_quarantine===true));
    await edit(false);await preview();await save();check(prefix+'-saved-disabled',(await read()).enabled===false);
    await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.getElementById('settings').open&&document.activeElement.id==='admin');
    check(prefix+'-keyboard-close-and-no-overflow',await page.$eval('html',e=>e.scrollWidth<=innerWidth+1));
    await context.close();
   }
  };
  await Promise.race([journey(),unexpected]);check('no-browser-errors',errors.length===0);
 } catch(error){failure=error;}
 finally{closing=true;if(browser)try{await browser.close()}catch(error){failure=failure?new AggregateError([failure,error],'browser and cleanup failed'):error}}
 fs.writeFileSync(cfg.public_output,JSON.stringify({status:failure?'FAIL':'PASS',assertions:checks.length,checks,errors},null,2)+'\n');if(failure)throw failure;
 console.log('QUARANTINE_UI_PASS '+checks.length);
})().catch(error=>{console.error(error.stack);process.exitCode=1});
