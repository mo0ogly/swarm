 'use strict';
/** Real Chrome, repository HTML and scripts, fully intercepted synthetic API.
 * Requires one private JSON config: fresh output, optional selected cases and
 * explicit asset_root for a private source candidate. Default assets are web/.
 * No listening server, backend, Authority, real credentials, recipe or apply.
 * Event/request promises control order; there is no sleep, retry or watchdog.
 * Reports bind source bytes, Node/npm/Chrome inputs and cleanup outcomes. Browser
 * profiles are private and must never be included in a handoff archive. */
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const root=fs.realpathSync(process.cwd()),runtime=require(path.join(root,'tests/helpers/browser_verification_runtime.cjs')),puppeteer=require(path.join(root,'node_modules/puppeteer'));
assert.equal(process.argv.length,3);const cfg=JSON.parse(fs.readFileSync(process.argv[2]));
assert(Object.keys(cfg).every(k=>['output','cases','asset_root'].includes(k)));assert.equal(typeof cfg.output,'string');
const available=['normal','data-before-layout','layout-before-data','guards','initial-refusal','early-edit','post-render-failure'];
const chosen=cfg.cases===undefined?available:cfg.cases;assert(Array.isArray(chosen)&&chosen.length&&chosen.every(x=>available.includes(x))&&new Set(chosen).size===chosen.length);
// Every selected fault/recovery run retains its valid normal control first.
const cases=['normal',...chosen.filter(x=>x!=='normal')],out=path.resolve(cfg.output),source=fs.realpathSync(cfg.asset_root||path.join(root,'web'));
assert(out!==root&&!out.startsWith(path.join(root,'.swarm')+path.sep));fs.mkdirSync(out,{mode:0o700});
const digest=raw=>crypto.createHash('sha256').update(raw).digest('hex'),assets=new Map(),checks=[],results=[];
const harnessInputs=[__filename,path.join(root,'tests/helpers/browser_verification_runtime.cjs')].map(file=>({file,sha256:digest(fs.readFileSync(file))}));
const report={status:'RUNNING',started_at:new Date().toISOString(),scope:'Real Chrome and repository scripts with intercepted synthetic GET/config-preview responses; no backend, Authority or installed-binary proof',source_root:source,harness_inputs:harnessInputs,selected_cases:cases,checks,results};
/** Publish named observed assertions; credentials and headers never enter them. */
function check(name,value){assert(value,name);assert(!checks.includes(name));checks.push(name);console.log('PASS '+name);}
/** Retain a failure as failure, including close errors, never a successful empty result. */
function publish(){fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({...report,assets:[...assets.values()].map(({file,sha256})=>({file,sha256}))},null,2)+'\n');}
/** Read once, hash and retain the exact bytes fulfilled to Chrome. Candidate
 * overrides are explicit; unprovided assets come from the repository web tree. */
function asset(name){
 assert(name&&!name.includes('/')&&!name.includes('..'));
 if(!assets.has(name)){
  const candidate=path.join(source,name),file=fs.existsSync(candidate)?candidate:path.join(root,'web',name),raw=fs.readFileSync(file);
  assets.set(name,{file,raw,sha256:digest(raw)});
 }
 return assets.get(name).raw;
}
const state={schema_version:1,revision:1,policy:{mandate_ttl_seconds:3600,unknown_reconcile_seconds:0,evidence_retention_days:365,require_separation:true},actions:{},identities:[],amount_minor_text:{},public_evidence_registry:{keys:{}}};
/** Exercise one deterministic ordering and its explicit gestures. All requests
 * are intercepted and admitted by exact origin/method/path. Any unexpected
 * diagnostic interrupts readiness waits without a time cutoff. Close always. */
async function journey(browser,scenario){
 const page=await browser.newPage(),requests=[],errors=[];let closing=false,heldScript,heldRead,resolveRead,showCount=0,rejectDiagnostic,primary;
 const readRequested=new Promise(resolve=>{resolveRead=resolve;}),diagnostic=new Promise((_,reject)=>{rejectDiagnostic=reject;});diagnostic.catch(()=>{});
 const bad=message=>{errors.push(message);rejectDiagnostic(Error(message));};
 page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 page.on('pageerror',()=>bad('unexpected-page-error'));page.on('requestfailed',()=>{if(!closing)bad('unexpected-request-failure');});
 page.on('console',message=>{if(message.type()==='error'&&!(scenario==='initial-refusal'&&message.location().url?.includes('/api/v1/trust-actions')&&/403/.test(message.text())))bad('unexpected-console-error');});
 try{
  await page.evaluateOnNewDocument(()=>{globalThis.__fixtureAdminClicks=0;document.addEventListener('click',e=>{if(e.target?.id==='admin')globalThis.__fixtureAdminClicks++;},true);});
  await page.setRequestInterception(true);
  page.on('request',async r=>{try{
   const u=new URL(r.url());assert.equal(u.origin,'http://127.0.0.1');requests.push({path:u.pathname,query:u.search,method:r.method()});
   const reply=(body,type='application/json',status=200)=>r.respond({status,contentType:type,body});
   if(r.method()==='POST'){
    assert.equal(u.pathname,'/api/v1/trust-actions');assert.equal(u.search,'?action=config-preview');
    return reply(JSON.stringify({applied:false,current_revision:1}));
   }
   assert.equal(r.method(),'GET');
   if(u.pathname==='/api/v1/native-session'){
    const body=JSON.stringify({schema_version:1,csrf_token:'a'.repeat(64)});
    if(scenario==='normal'){heldRead=()=>reply(body);resolveRead();return;}return reply(body);
   }
   if(u.pathname==='/api/v1/trust-actions'){
    assert.equal(u.search,'');showCount++;const fixture=scenario==='post-render-failure'&&showCount===1?{...state,actions:{synthetic_render_fault:null}}:state;const body=JSON.stringify(fixture);
    if(scenario!=='normal'&&scenario!=='data-before-layout'&&scenario!=='post-render-failure'&&!heldRead){
     heldRead=()=>scenario==='initial-refusal'?reply(JSON.stringify({failure:{code:'fixture-read-refused'}}),'application/json',403):reply(body);resolveRead();return;
    }return reply(body);
   }
   const name=u.pathname==='/trust-actions.html'?'trust-native.html':u.pathname.slice(1);
   if(name==='favicon.ico')return r.respond({status:204});
   const raw=asset(name),respond=()=>reply(raw,name.endsWith('.html')?'text/html':name.endsWith('.js')?'application/javascript':name.endsWith('.css')?'text/css':'image/png');
   if(name==='trust-workbench.js'&&['data-before-layout','post-render-failure'].includes(scenario)){heldScript=respond;return;}return respond();
  }catch(error){bad('intercepted-request-refused: '+error.message);try{await r.abort();}catch(secondary){if(!closing)bad('request-abort-failed: '+secondary.message);}}});
  await Promise.race([(async()=>{
   let navigationError;const navigation=page.goto('http://127.0.0.1/trust-actions.html?view=security',{waitUntil:'load'}).catch(e=>{navigationError=e;});
   if(scenario==='data-before-layout'||scenario==='post-render-failure'){
    if(scenario==='post-render-failure')await page.waitForFunction(()=>document.getElementById('message')?.textContent.includes('Vérifier l’état avant reprise'));else await page.waitForFunction(()=>document.getElementById('message')?.textContent.includes('Prêt'));assert(heldScript);await heldScript();
   }else{
    await page.waitForFunction(()=>document.body.classList.contains('security-workbench'));await readRequested;assert(heldRead);
    if(scenario==='early-edit')await page.evaluate(()=>{document.getElementById('identities').value='private-early-draft';document.getElementById('configuration').dispatchEvent(new Event('input',{bubbles:true}));});
    await heldRead();
   }
   await navigation;if(navigationError)throw navigationError;await page.waitForFunction(()=>!document.getElementById('admin').disabled);
   const initialAdminClicks=await page.evaluate(()=>globalThis.__fixtureAdminClicks);check(scenario+'-initial-without-admin-click',initialAdminClicks===0);
   if(['initial-refusal','early-edit','post-render-failure'].includes(scenario)){
    check(scenario+'-no-automatic-fields',await page.$$eval('#policy input',e=>e.length===0));
    if(scenario==='early-edit')check(scenario+'-draft-preserved',await page.$eval('#identities',e=>e.value==='private-early-draft'));
    await page.evaluate(()=>document.dispatchEvent(new Event('swarm-trust-layout-ready')));
    check(scenario+'-duplicate-event-no-promotion',await page.$$eval('#policy input',e=>e.length===0));
    await page.evaluate(()=>document.getElementById('admin').click());await page.waitForFunction(()=>!document.getElementById('admin').disabled&&document.querySelectorAll('#policy input').length===3);
    check(scenario+'-explicit-recovery',true);
   }else if(scenario==='guards'){
    await page.evaluate(()=>{document.getElementById('identities').value='[]';document.getElementById('reason').value='Synthetic draft';document.getElementById('trust-admin-token').value='private-synthetic';document.getElementById('p-mandate_ttl_seconds').value='7200';document.getElementById('configuration').dispatchEvent(new Event('input',{bubbles:true}));document.getElementById('configuration').requestSubmit();});
    await page.waitForFunction(()=>!document.getElementById('save').disabled);check(scenario+'-explicit-preview-control',true);
    await page.evaluate(()=>{document.dispatchEvent(new Event('swarm-trust-layout-ready'));document.dispatchEvent(new Event('swarm-trust-layout-ready'));document.getElementById('refresh').click();});await page.waitForFunction(()=>!document.getElementById('refresh').disabled);
    check(scenario+'-draft-token-preview-preserved',await page.evaluate(()=>document.getElementById('p-mandate_ttl_seconds').value==='7200'&&document.getElementById('identities').value==='[]'&&document.getElementById('trust-admin-token').value==='private-synthetic'&&!document.getElementById('save').disabled));
    await page.evaluate(()=>{document.getElementById('reason').value+=' edited';document.getElementById('configuration').dispatchEvent(new Event('input',{bubbles:true}));});check(scenario+'-input-disarms',await page.$eval('#save',e=>e.disabled));
    await page.evaluate(()=>document.getElementById('admin').click());await page.waitForFunction(()=>!document.getElementById('admin').disabled);
   }
   const observed=await page.evaluate(()=>({inline:!document.getElementById('settings').contains(document.getElementById('configuration')),fields:[...document.querySelectorAll('#policy input')].map(e=>({id:e.id,value:e.value})),saveDisabled:document.getElementById('save').disabled,adminClicks:globalThis.__fixtureAdminClicks}));
   check(scenario+'-measured-admin-clicks',observed.adminClicks===(['guards','initial-refusal','early-edit','post-render-failure'].includes(scenario)?1:0));
   check(scenario+'-exact-fields',JSON.stringify(observed.fields)===JSON.stringify([{id:'p-mandate_ttl_seconds',value:'3600'},{id:'p-unknown_reconcile_seconds',value:'0'},{id:'p-evidence_retention_days',value:'365'}]));
   check(scenario+'-inline-and-save-disarmed',observed.inline&&observed.saveDisabled);check(scenario+'-no-browser-errors',errors.length===0);
   results.push({scenario,status:'PASS',initialAdminClicks,observed,requests});
  })(),diagnostic]);
 }catch(error){primary=error;results.push({scenario,status:'FAIL',cause:error.message,requests,errors});}
 finally{closing=true;try{await page.close();}catch(error){primary=primary?new AggregateError([primary,error],'journey and page close failed'):error;}}
 publish();if(primary)throw primary;
}
(async()=>{let browser,primary,record;try{
 record=runtime.begin(root,out);browser=await puppeteer.launch({headless:true,executablePath:'/opt/google/chrome/chrome',...runtime.launchOptions,args:['--no-sandbox'],userDataDir:path.join(out,'profile')});await runtime.recordBrowser(record,browser);
 for(const scenario of cases)await journey(browser,scenario);
 for(const item of harnessInputs)check('harness-stable-'+path.basename(item.file),digest(fs.readFileSync(item.file))===item.sha256);
 for(const item of assets.values())check('source-stable-'+path.basename(item.file),digest(fs.readFileSync(item.file))===item.sha256);
 }catch(error){primary=error;}finally{if(browser)try{await browser.close();}catch(error){primary=primary?new AggregateError([primary,error],'verification and browser close failed'):error;}}
 if(!primary)try{report.runtime=runtime.finish(record,root);}catch(error){primary=error;}
 report.finished_at=new Date().toISOString();report.status=primary?'FAIL':'PASS';if(primary)report.failure=primary.message;publish();if(primary)throw primary;
 console.log('SECURITY_INITIALIZATION_PASS '+results.length);
})().catch(error=>{console.error(error);process.exitCode=1});
