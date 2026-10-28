'use strict';
/** Actual native landing/shared Admin over protected HTTP, no CDP-to-CLI bridge.
 * Empty acknowledgement and response loss are explicit client-side fixtures after
 * healthy saves; Root is never edited directly. No time/retry watchdog is added.
 * Saved mission slots17/33 and explicit null are checked in FR/EN and both
 * themes, with reopened controls and persistence; no automatic launch is inferred.
 * New reviewer defaults are saved as7/0/3072/8192 and consumed through separate
 * real CLI bootstraps on the same Root; existing reviewer snapshots stay intact.
 * Scope policy2/30/1/8192 is saved/reopened and consumed by five real claims.
 * Eight decision settings are saved/reopened; four real decisions create workers
 * using the saved initial attempt policy, without executing provider processes.
 * Requested leases use the saved finite maximum; existing deadlines stay intact.
 * This is not a CDP-to-CLI settings bridge and executes no provider process.
 * Key/CSRF/preview data stay out of proof records. Browser profile stays private.
 */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer'),{spawnSync}=require('node:child_process');
const runtimeTools=require('./helpers/browser_verification_runtime.cjs'),state=require('./helpers/engine_settings_admin_state.cjs'),{finishOwnedResources}=require('./helpers/verification_cleanup.cjs');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),repo=path.resolve(__dirname,'..'),output=path.resolve(cfg.output);fs.mkdirSync(output,{recursive:false});
let browser,primary,success=false;const checks=[],errors=[],responses=[],consoleErrors=[];let cap=123,configurationOutcomes=0,expectedCookie;
const reviewerDefaults=[['reviewer_default_delivery_files','7'],['reviewer_default_context_files','0'],['reviewer_default_source_bytes','3072'],['reviewer_default_context_bytes','8192']];
const scopeDefaults=[['planning_scope_lease_min_seconds','2'],['planning_scope_lease_max_seconds','30'],['planning_context_max_events','1'],['planning_context_max_bytes','8192']];
const decisionDefaults=[['planning_decision_max_inputs','2'],['planning_decision_max_operations','2'],['planning_decision_max_reason_bytes','1024'],['planning_decision_max_title_characters','200'],['planning_decision_max_instruction_bytes','8192'],['planning_decision_max_dependencies','5'],['planning_decision_max_criterion_bytes','4096'],['planning_worker_default_attempts','3']];
const responseDefaults=[['planning_response_max_bytes','2048']];
const planningWaitDefaults=[['planning_provider_silence_seconds','0'],['planning_provider_max_duration_seconds','0'],['planning_default_lease_seconds','10'],['planning_lease_renew_after_percent','25']];
let consumerSequence=0;const consumers=[];let originalReviewer;
const saved=()=>fs.readFileSync(path.join(cfg.root,'.swarm','engine-settings.json'));
const doc=()=>JSON.parse(saved());
/** Run a supported native command with original stdout/stderr retained on files.
 * No shell, output-volume cap or temporal watchdog; close/flush causes survive.
 * Optional expectedFailure requires an actual safe refusal from the consumer.
 * These fixture inputs contain no session/CSRF credentials or real provider data.
 */
async function consumerCli(args,value,expectedFailure){
 const label='consumer-'+String(++consumerSequence).padStart(3,'0'),stdout=path.join(output,label+'.stdout'),stderr=path.join(output,label+'.stderr');let out,err,result,primary;
 const argv=[...args,'--root',cfg.root,'--json'];if(value!==undefined)argv.push('--input','-');
 fs.writeFileSync(path.join(output,label+'.input.json'),JSON.stringify({argv,input:value??null})+'\n',{flag:'wx',mode:0o600});
 try{
  out=fs.openSync(stdout,'wx',0o600);err=fs.openSync(stderr,'wx',0o600);
  const child=spawnSync(cfg.binary,argv,{input:value===undefined?undefined:JSON.stringify(value),stdio:['pipe',out,err],timeout:0});
  fs.fsyncSync(out);fs.fsyncSync(err);if(child.error)throw child.error;if(expectedFailure){assert.notEqual(child.status,0);result=JSON.parse(fs.readFileSync(stderr,'utf8'));assert.equal(result.failure.code,expectedFailure)}else{assert.equal(child.status,0,'Native consumer refused; inspect retained '+label+' stdout/stderr');result=JSON.parse(fs.readFileSync(stdout,'utf8'))}
 }catch(error){primary={error}}
 await finishOwnedResources(primary,[()=>{if(err!==undefined)fs.closeSync(err)},()=>{if(out!==undefined)fs.closeSync(out)}]);return result;
}
/** Prove saved defaults affect new reviewer/claim snapshots, preserving old ones.
 * This configures only an inert source; no planner/worker/reviewer process runs.
 */
async function bootstrapConsumer(label){
 const created=await consumerCli(['work','create'],{schema_version:1,event_id:'admin-create-'+label,expected_revision:0,title:'Admin creation defaults',objective:'Observe saved reviewer configuration',scope:'Owned fixture',criteria:['Persisted snapshot']});
 const work=await consumerCli(['planning','enable',created.work.id],{schema_version:1,event_id:'admin-enable-'+label,expected_revision:1,provider:'admin-fixture',max_activations:3,max_tasks:10,max_decisions:10,max_review_calls:2});
 const fields=[['max_files','reviewer_default_delivery_files'],['max_context_files','reviewer_default_context_files'],['max_source_bytes','reviewer_default_source_bytes'],['max_context_bytes','reviewer_default_context_bytes']];
 for(const[field,name]of fields)assert.equal(work.planning.reviewer[field],Number(doc().values[name]));
 assert.equal(work.planning.reviewer.calls??0,0);assert(!fs.existsSync(path.join(cfg.root,'unexpected-provider-execution')),'Metadata bootstrap executed provider');
 const lease=doc().values.planning_scope_lease_max_seconds;assert(lease!==null&&Number.isSafeInteger(Number(lease))&&Number(lease)>0,'Fixture requires an explicit finite saved maximum');
 const claimed=await consumerCli(['planning','claim',created.work.id],{schema_version:1,event_id:'admin-claim-'+label,expected_revision:work.revision,scope:'root',scope_revision:1,holder:'admin-'+label,lease_seconds:Number(lease)});
 const scope=claimed.planning.scopes.find(s=>s.id==='root');assert.equal(claimed.planning.activations,1);assert.equal(scope.generation,1);assert(scope.workflow&&scope.delivery);assert.deepEqual(scope.delivery.events,['admin-enable-'+label]);assert(!fs.existsSync(path.join(cfg.root,'unexpected-provider-execution')),'Claim executed provider');
 let worker=null;if(label!=='before-admin'){
 const decided=await consumerCli(['planning','decide',created.work.id],{schema_version:1,event_id:'admin-decision-'+label,expected_revision:claimed.revision,scope:'root',scope_revision:scope.revision,holder:scope.holder,generation:scope.generation,input_events:scope.delivery.events,reason:'Observe saved decision policy',operations:[{kind:'task',id:'admin-worker',title:'Saved policy worker',requirements:['req-1'],deliverable:'Owned report',criteria:['Observe only']}]});
 worker=decided.tasks.find(t=>t.id==='admin-worker');assert.equal(worker.plan_max_attempts,Number(doc().values.planning_worker_default_attempts));assert.equal(decided.planning.decisions,1);assert(!fs.existsSync(path.join(cfg.root,'unexpected-provider-execution')),'Decision executed provider');
 }
 const proposal={type:'result',structured_output:{input_events:['admin-event'],reason:'Observe',operations:[]}};
 const checked=await consumerCli(['planning-output','check'],proposal);assert(checked.proposal_valid&&!checked.process_started&&!checked.decision_applied&&checked.input_count===1);
 const overflow={...proposal,structured_output:{...proposal.structured_output,reason:'r'.repeat(Number(doc().values.planning_response_max_bytes)+1)}};await consumerCli(['planning-output','check'],overflow,'planning-response-byte-limit-exceeded');
 const result={label,work:created.work.id,reviewer:work.planning.reviewer,scope,worker,requested_lease_seconds:Number(lease),scope_policy:Object.fromEntries(scopeDefaults.map(([name])=>[name,doc().values[name]])),provider_started:false};consumers.push(result);return result;
}
/** Read a terminal DOM value and dispose the handle without masking either cause. */
async function terminal(page,predicate){const h=await page.waitForFunction(predicate);let value,primary;try{value=await h.jsonValue()}catch(error){primary={error}}await finishOwnedResources(primary,[()=>h.dispose()]);return value}
(async()=>{
 try{
 originalReviewer=await bootstrapConsumer('before-admin');
 const runtime=runtimeTools.begin(repo,output);browser=await puppeteer.launch({...runtimeTools.launchOptions,headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',userDataDir:path.join(cfg.root,'chrome-admin'),args:['--no-sandbox']});await runtimeTools.recordBrowser(runtime,browser);
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const label=lang+'-'+theme;console.log('RUN native-admin '+label);const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  let expectedWaitRefusal=false,wait400seen=0;
  let rejectPage;const failure=new Promise((_,reject)=>{rejectPage=reject});const guard=p=>Promise.race([p,failure]);
  page.on('pageerror',e=>{errors.push({case:label,message:e.message});rejectPage(e)});
  page.on('requestfailed',r=>{const e={case:label,path:new URL(r.url()).pathname,failure:r.failure().errorText};errors.push(e);rejectPage(Error('Unexpected native resource failure: '+e.path))});
  page.on('console',m=>{if(m.type()==='error')consoleErrors.push({case:label,message:m.text()})});
  page.on('response',r=>{if(r.status()>=400){const pathname=new URL(r.url()).pathname;responses.push({case:label,path:pathname,status:r.status()});if(r.status()===400&&expectedWaitRefusal&&wait400seen===0&&pathname==='/api/v1/native-settings/preview'&&r.request().method()==='POST'){wait400seen++;return}if(!(r.status()===403&&['/api/v1/native-settings','/native-runtime/app.js'].includes(pathname))){errors.push({case:label,path:pathname,status:r.status()});rejectPage(Error('Unexpected HTTP '+r.status()+' '+pathname))}}});
  await page.setViewport({width:1440,height:1000});const response=await guard(page.goto('http://'+cfg.address+'/?lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'}));assert(response.status()===200);assert(await page.$('#access-key'),'Default root must offer login');
  const before=saved();assert.deepEqual(await page.evaluate(async()=>{const result=[];for(const path of ['/api/v1/native-settings','/native-runtime/app.js']){const r=await fetch(path);await r.text();result.push(r.status)}return result}),[403,403]);
  await page.type('#access-key',cfg.key);
  const loginError=page.waitForFunction(()=>{const e=document.querySelector('#access-error'),b=document.querySelector('button[type=submit]');return e&&b&&!e.hidden&&!b.disabled}).then(()=>{throw Error('Valid native login refused')});
  await guard(Promise.all([Promise.race([page.waitForNavigation({waitUntil:'networkidle0'}),loginError]),page.click('button[type=submit]')]));
  const startup=await guard(terminal(page,()=>{const status=document.querySelector('#native-startup-status');if(status&&!status.hidden&&status.classList.contains('alert'))return{status:'failed',message:status.textContent};const s=document.querySelector('.engine-settings-admin [role=status]');if(s?.classList.contains('alert'))return{status:'failed',message:s.textContent};const edit=document.querySelector('[data-action=edit]');return edit&&!edit.disabled?{status:'loaded'}:false}));assert.equal(startup.status,'loaded',startup.message);
  assert(await page.$eval('html',e=>e.lang)===lang);assert(await page.$eval('html',e=>e.dataset.theme)===theme);assert(await page.$eval('h1',e=>e.textContent)===(lang==='en'?'Rust engine — development':'Moteur Rust — développement'));assert(saved().equals(before),'Login/load must not adopt settings');
  /** Preview through the real form/API; inputs stay data and preview stays pure. */
  async function preview(){const original=saved();await page.click('[data-action=edit]');await guard(page.waitForSelector('dialog[open]'));const value=String(cap++),age=checks.length===3?null:String(3600+checks.length);expectedCookie=age;
   await page.$eval('[data-unlimited="web_cookie_max_age_seconds"]',(e,unlimited)=>{e.checked=unlimited;e.dispatchEvent(new Event('change',{bubbles:true}));e.dispatchEvent(new Event('input',{bubbles:true}))},age===null);
   // A repeated preview can retain an explicit unlimited value. Select the
   // finite control before filling it, then restore the requested null below.
   await page.$eval('[data-unlimited="mission_dispatch_max_slots"]',e=>{e.checked=false;e.dispatchEvent(new Event('change',{bubbles:true}));e.dispatchEvent(new Event('input',{bubbles:true}))});
   for(const [name,text]of [['review_delivery_max_bytes',value],['mission_supervision_poll_interval_ms','4000'],['mission_supervision_stale_after_ms','45000'],['mission_supervision_future_tolerance_ms','0'],['mission_dispatch_default_slots','17'],['mission_dispatch_max_slots','33'],...reviewerDefaults,...scopeDefaults,...decisionDefaults,...responseDefaults,...planningWaitDefaults,...(age===null?[]:[['web_cookie_max_age_seconds',age]])])await page.$eval('[data-setting="'+name+'"]',(e,value)=>{if(e.disabled)throw Error('Test field unexpectedly disabled');e.value=value;e.dispatchEvent(new Event('input',{bubbles:true}))},text);
   await page.$eval('[data-unlimited="mission_dispatch_max_slots"]',(e,unlimited)=>{e.checked=unlimited;e.dispatchEvent(new Event('change',{bubbles:true}));e.dispatchEvent(new Event('input',{bubbles:true}))},age===null);
   if(!checks.length){const programs=doc().values.validation_allowed_programs;assert(programs.length>1,'Canonical program-set witness needs distinct entries');await page.$eval('[data-setting="validation_allowed_programs"]',(e,programs)=>{e.value=[...programs].reverse().join('\n');e.dispatchEvent(new Event('input',{bubbles:true}))},programs)}
   await page.$eval('[data-setting-reason]',(e,label)=>{e.value='Actual native Admin proof '+label;e.dispatchEvent(new Event('input',{bubbles:true}))},label);
   await page.click('[data-action=confirm]');await guard(state.waitForSettingsState(page,'preview'));assert(saved().equals(original),'Preview mutated saved values');return{value,age};}
  async function confirm(){await page.click('[data-action=confirm]');await guard(state.waitForSettingsState(page,'saved'));configurationOutcomes++;}
  let chosen=await preview();
  // A valid preview is the control before invalidating just the renewal domain.
  const beforeInvalidWait=saved();expectedWaitRefusal=true;const rejectedWaitResponse=page.waitForResponse(r=>r.status()===400&&new URL(r.url()).pathname==='/api/v1/native-settings/preview'&&r.request().method()==='POST');await page.$eval('[data-setting="planning_lease_renew_after_percent"]',e=>{e.value='100';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.click('[data-action=confirm]');
  await guard(terminal(page,()=>{const m=document.querySelector('dialog[data-engine-settings]');const e=m?.querySelector('[role=alert]');return e&&!e.hidden?'refused':false}));
  const waitRefusal=await guard((await guard(rejectedWaitResponse)).json());expectedWaitRefusal=false;assert.equal(wait400seen,1);assert.equal(waitRefusal.failure.code,'settings-planning-wait-renewal-invalid');assert.notEqual(waitRefusal.configuration_may_be_recorded,true);
  assert(saved().equals(beforeInvalidWait),'Invalid renewal changed saved policy');
  const waitError=await page.$eval('dialog [role=alert]',e=>e.textContent);
  assert(waitError.includes(lang==='en'?'Lease renewal must be disabled (0)':'Le renouvellement du bail doit être désactivé (0)'),'Wait policy refusal is not actionable in the actual Admin');
  await page.$eval('[data-setting="planning_lease_renew_after_percent"]',e=>{e.value='25';e.dispatchEvent(new Event('input',{bubbles:true}))});chosen=await preview();

  const leaseHelp='dialog[data-engine-settings] [data-settings-help="planning_scope_lease_min_seconds"]';await page.$eval(leaseHelp,e=>{e.closest('[data-settings-group]').open=true});await page.click(leaseHelp);const help=await page.$eval('dialog.engine-settings-help',e=>e.textContent);assert(help.includes(lang==='en'?'cannot resurrect an expired lease':'ne recrée pas un bail expiré'),'Saved catalogue help falsely describes renewal');await page.keyboard.press('Escape');assert(await page.$eval(leaseHelp,e=>document.activeElement===e));
  const responseHelp='dialog[data-engine-settings] [data-settings-help="planning_response_max_bytes"]';await page.$eval(responseHelp,e=>{e.closest('[data-settings-group]').open=true});await page.click(responseHelp);const responseText=await page.$eval('dialog.engine-settings-help',e=>e.textContent);assert(responseText.includes(lang==='en'?'each candidate text':'chaque texte candidat'),'Structured response cap scope missing');await page.keyboard.press('Escape');
  const waitHelp='dialog[data-engine-settings] [data-settings-help="planning_default_lease_seconds"]';await page.$eval(waitHelp,e=>{e.closest('[data-settings-group]').open=true});await page.click(waitHelp);const waitText=await page.$eval('dialog.engine-settings-help',e=>e.textContent);assert(waitText.includes(lang==='en'?'not maximum model duration':'pas la durée maximale du modèle'),'Ownership lease is falsely presented as a model duration cutoff');await page.keyboard.press('Escape');
  for(const width of [1440,390,320]){await page.setViewport({width,height:1000});await page.$eval('[data-setting="planning_lease_renew_after_percent"]',e=>e.scrollIntoView({block:'center'}));assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'Admin overflow');if(width!==390)await page.screenshot({path:path.join(output,label+'-dialog-'+width+'.png')});}
  await confirm();assert.equal(doc().values.review_delivery_max_bytes,chosen.value);assert.equal(doc().values.web_cookie_max_age_seconds,chosen.age);await page.click('[data-action=edit]');await guard(page.waitForSelector('dialog[open]'));assert(chosen.age===null?await page.$eval('[data-unlimited="web_cookie_max_age_seconds"]',e=>e.checked):await page.$eval('[data-setting="web_cookie_max_age_seconds"]',e=>e.value)===chosen.age);for(const[name,value]of [...scopeDefaults,...decisionDefaults,...responseDefaults,...planningWaitDefaults])assert.equal(await page.$eval('[data-setting="'+name+'"]',e=>e.value),value,'Reopened scope policy differs');await page.keyboard.press('Escape');assert(!(await page.$('dialog[open]')));assert(await page.$eval('[data-action=edit]',e=>document.activeElement===e),'Escape must restore focus');
  if(!checks.length){
   // A genuine successful save is the positive witness before the empty ACK fault.
   chosen=await preview();const unchanged=saved();await page.setRequestInterception(true);let injected=0;
   const intercept=async request=>{if(new URL(request.url()).pathname==='/api/v1/native-settings/apply'&&!injected){injected++;await request.respond({status:200,contentType:'application/json',body:'{}'})}else await request.continue()};page.on('request',intercept);
   await page.click('[data-action=confirm]');const empty=await guard(terminal(page,()=>{const m=document.querySelector('dialog[data-engine-settings]');if(!m)return'closed';const e=m.querySelector('[role=alert]');return e&&!e.hidden?'refused':false}));
   page.off('request',intercept);await page.setRequestInterception(false);assert.equal(injected,1);assert(saved().equals(unchanged),'Empty fake acknowledgement must not mutate Root');assert.equal(empty,'refused','Unconfirmed200 response closed the dialog as if saved');
   await confirm();assert.equal(doc().values.review_delivery_max_bytes,chosen.value);
   // Real HTTP publication, then controlled client loss; explicit identical replay.
   chosen=await preview();const previous=saved();await page.evaluate(()=>{const original=window.fetch;let once=true;window.fetch=async(...args)=>{const response=await original(...args);if(once&&new URL(String(args[0]),location.href).pathname==='/api/v1/native-settings/apply'){once=false;await response.text();throw new TypeError('Controlled client response loss after actual HTTP publication')}return response}});
   await page.click('[data-action=confirm]');const lost=await guard(terminal(page,()=>{const m=document.querySelector('dialog[data-engine-settings]');if(!m)return'closed';const e=m.querySelector('[role=alert]');return e&&!e.hidden?'refused':false}));assert.equal(lost,'refused');configurationOutcomes++;const committed=saved();assert(!committed.equals(previous),'Lost response must follow real publication');const event=doc().history.at(-1).change.event_id;
   await confirm();assert(saved().equals(committed),'Identical recovery must replay without duplicate bytes');assert.equal(doc().history.filter(e=>e.change.event_id===event).length,1);
  }
  for(const[name,value]of [...reviewerDefaults,...scopeDefaults,...decisionDefaults,...responseDefaults,...planningWaitDefaults])assert.equal(doc().values[name],value);
  await bootstrapConsumer(label);const previous=await consumerCli(['planning','show',originalReviewer.work]);assert.deepEqual(previous.planning.reviewer,originalReviewer.reviewer,'Global defaults changed an existing reviewer');assert.deepEqual(previous.planning.scopes.find(s=>s.id==='root'),originalReviewer.scope,'New global bounds changed an existing claim');
  assert.equal(doc().values.mission_dispatch_default_slots,'17');assert.equal(doc().values.mission_dispatch_max_slots,expectedCookie===null?null:'33');
  checks.push({case:label,status:'PASS',viewports:[1440,390,320],actual_http:true,default_login_landing:true,pure_preview:true,saved_revision:doc().revision,review_cap:doc().values.review_delivery_max_bytes,cookie_age:doc().values.web_cookie_max_age_seconds,scope_renewal_help:true,escape_focus:true});console.log('PASS native-admin '+label);await context.close();
 }
 assert.deepEqual(errors,[]);assert.equal(responses.filter(e=>e.status===400).length,checks.length,'One specifically guarded invalid renewal response per case');assert.equal(consoleErrors.filter(e=>/Failed to load resource: the server responded with a status of 400/.test(e.message)).length,checks.length,'One browser diagnostic per injected renewal refusal');assert(consoleErrors.every(e=>/Failed to load resource: the server responded with a status of (403|400)/.test(e.message)),'Unexpected console error');const actual=runtimeTools.finish(runtime,repo);
 fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:'PASS',scope:'Actual native login/default landing/shared Admin HTTP; empty ACK and post-publication client loss are controlled fixtures, not real network/server faults',checks,errors,expected_http_refusals:responses,expected_rejected_requests:8,expected_configuration_outcomes:configurationOutcomes,expected_cookie_max_age:expectedCookie,expected_review_cap:doc().values.review_delivery_max_bytes,expected_mission_policy:Object.fromEntries(['mission_supervision_poll_interval_ms','mission_supervision_stale_after_ms','mission_supervision_future_tolerance_ms'].map(name=>[name,doc().values[name]])),reviewer_creation_consumers:consumers,expected_reviewer_defaults:Object.fromEntries(reviewerDefaults),expected_scope_policy:Object.fromEntries(scopeDefaults),expected_decision_policy:Object.fromEntries(decisionDefaults),expected_response_policy:Object.fromEntries(responseDefaults),expected_dispatch_policy:Object.fromEntries(['mission_dispatch_default_slots','mission_dispatch_max_slots'].map(name=>[name,doc().values[name]])),expected_revision:doc().revision,runtime:actual},null,2)+'\n');success=true;
 }catch(error){primary={error}}
 await finishOwnedResources(primary,[async()=>{if(browser)await browser.close()}],async failures=>{if(!success||failures.length)fs.writeFileSync(path.join(output,'failure.json'),JSON.stringify({status:'FAIL',checks,errors,causes:failures.map(e=>({name:e?.name,message:e?.message}))},null,2)+'\n')});
})().catch(e=>{console.error(e);process.exitCode=1});
