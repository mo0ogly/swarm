/** Native manual-admission Admin acceptance on four private FR/EN × theme roots.
 * Uses the actual default-profile CLI for settings and launch previews, no probe,
 * mock API authorization or paid call. Saves/reopens eleven admission controls and
 * nine v18 supervision and three v19 model-rule controls; runs a local cat child, checks frozen policies,
 * actual capture, guard consumers, refusal/recovery, null and keyboard/mobile
 * states. Zero global/CDP/page watchdog; progress and original failures retained.
 * PASS is bounded candidate evidence, not deployment or a full native mission.
 */
'use strict';
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),http=require('node:http'),crypto=require('node:crypto'),{spawn}=require('node:child_process'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
if(process.argv.length!==4)throw Error('Usage: rust_agent_launch_admin_ui.cjs NATIVE_CLI FRESH_OUTPUT');
const repo=path.resolve(__dirname,'..'),binary=path.resolve(process.argv[2]),output=path.resolve(process.argv[3]);
if(fs.existsSync(output))throw Error('Fresh evidence required');fs.mkdirSync(output,{recursive:true});
const scratch=fs.mkdtempSync(path.join(os.homedir(),'.swarm-launch-admin-'));fs.chmodSync(scratch,0o700);
const adminState=require('./helpers/engine_settings_admin_state.cjs');
const helper=require('./helpers/browser_verification_runtime.cjs'),runtime=helper.begin(repo,output);
const assets={'/themes.css':'web/wattson_themes.css','/base.css':'web/processes.css','/design.css':'web/swarm-design.css','/admin.css':'frontend/engine-settings/admin.css','/admin.js':'frontend/engine-settings/admin.js','/english.js':'web/i18n-en.js','/swarm-logo.png':'web/swarm-logo.png'};
const inputs=[...Object.values(assets),'locales/en.json','rust/crates/swarm-engine/config/engine-settings-catalog.json','rust/crates/swarm-engine/config/engine-settings.json','tests/rust_agent_launch_admin_ui.cjs','tests/helpers/browser_verification_runtime.cjs','tests/helpers/engine_settings_admin_state.cjs',binary];
const digest=p=>crypto.createHash('sha256').update(fs.readFileSync(path.isAbsolute(p)?p:path.join(repo,p))).digest('hex');
const snapshot=()=>Object.fromEntries(inputs.map(p=>[p,digest(p)])),before=snapshot();
const english=JSON.parse(fs.readFileSync(path.join(repo,'locales/en.json'))),catalog=JSON.parse(fs.readFileSync(path.join(repo,'rust/crates/swarm-engine/config/engine-settings-catalog.json')));
for(const field of catalog.fields)for(const key of ['label','help'])assert(Object.hasOwn(english,field[key]));
const commands=[],checks=[],errors=[];let browser,server,sequence=0;
/** Run one real CLI operation, recording argv, status and original streams.
 * Fresh per-command input files are fixture-owned; no inherited test timeout. */
async function cli(root,args,value){
 const extra=[];if(value!==undefined){const input=path.join(scratch,'input-'+(++sequence)+'.json');fs.writeFileSync(input,JSON.stringify(value),{mode:0o600});extra.push('--input',input);}
 const argv=[...args,'--root',root,'--json',...extra];
 const r=await new Promise((resolve,reject)=>{const child=spawn(binary,argv,{cwd:repo});let stdout=[],stderr=[];child.stdout.on('data',b=>stdout.push(b));child.stderr.on('data',b=>stderr.push(b));child.once('error',reject);child.once('close',code=>resolve({code,stdout:Buffer.concat(stdout).toString(),stderr:Buffer.concat(stderr).toString()}));});
 commands.push({argv,...r});fs.writeFileSync(path.join(output,'commands.json'),JSON.stringify(commands,null,2)+'\n');
 if(r.code){const wire=JSON.parse(r.stderr);const e=Error(wire.failure.message);e.code=wire.failure.code;throw e;}assert.equal(r.stderr,'');return JSON.parse(r.stdout);
}
/** Record progress around actual UI assertions; failure retains its original. */
async function observed(label,promise){console.log(label);try{return await promise}catch(e){checks.push({label,status:'FAIL',message:e.message});throw e}}
const req=(event,revision,fields)=>({schema_version:1,event_id:event,expected_revision:revision,...fields});
(async()=>{try{
 server=http.createServer((request,response)=>{const pathname=new URL(request.url,'http://localhost').pathname;if(pathname==='/'){response.writeHead(200,{'Content-Type':'text/html; charset=utf-8','Content-Security-Policy':"default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'none'"});response.end('<!doctype html><html lang="fr"><head><meta name="viewport" content="width=device-width,initial-scale=1"><title>Agent admission Admin candidate</title><link rel="icon" href="/swarm-logo.png"><link rel="stylesheet" href="/themes.css"><link rel="stylesheet" href="/base.css"><link rel="stylesheet" href="/design.css"><link rel="stylesheet" href="/admin.css"><script src="/english.js" defer></script><script src="/admin.js" defer></script></head><body><main id="admin"><h1>Administration</h1></main></body></html>');return;}const file=assets[pathname];if(!file){response.writeHead(404);response.end();return;}response.writeHead(200,{'Content-Type':pathname.endsWith('.css')?'text/css':pathname.endsWith('.js')?'text/javascript':'image/png'});response.end(fs.readFileSync(path.join(repo,file)));});
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 browser=await puppeteer.launch({...helper.launchOptions,headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',args:['--no-sandbox']});await helper.recordBrowser(runtime,browser);
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const label=lang+'-'+theme,root=path.join(scratch,label);fs.mkdirSync(root,{mode:0o700});await cli(root,['init']);
  const work=(await cli(root,['work','create'],req('create',0,{title:'Manual admission',objective:'Observe saved policy',scope:'Fixture',criteria:['No provider child']}))).work.id;
  await cli(root,['task','add',work],req('add',1,{id:'task',title:'Prepare',deliverable:'Intention',criteria:['No provider child']}));
  const providers=path.join(root,'.swarm/providers.json');const provider=extra=>({schema_version:1,providers:{fixture:{command:'/bin/cat',args:[],...extra}}});fs.writeFileSync(providers,JSON.stringify(provider({})),{mode:0o600});
  const frame=req('session',2,{task_id:'task',provider:'fixture',instruction:'hello'});
  const preview=extra=>cli(root,['agent','preview',work],{...frame,...extra});await preview({});
  const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  page.on('pageerror',e=>errors.push({label,type:'page',message:e.message}));page.on('console',m=>{if(m.type()==='error')errors.push({label,type:'console',message:m.text()});});page.on('requestfailed',r=>errors.push({label,type:'request',url:r.url(),failure:r.failure()}));page.on('response',r=>{if(r.status()>=400)errors.push({label,type:'response',url:r.url(),status:r.status()});});
  await page.exposeFunction('nativeSettings',async(op,r)=>{try{return {value:await cli(root,['engine-settings',op],r)}}catch(e){return {failure:{code:e.code,message:e.message}}}});
  await observed(label+' load',page.goto('http://127.0.0.1:'+server.address().port));
  await page.evaluate(({lang,theme})=>{document.documentElement.lang=lang;document.documentElement.dataset.theme=theme;const translate=s=>{if(lang==='fr')return s;if(!Object.hasOwn(SwarmEnglish,s))throw Error('Missing English: '+s);return SwarmEnglish[s];};const call=async(op,r)=>{const result=await(r===undefined?nativeSettings(op):nativeSettings(op,r));if(result.failure){const e=Error(result.failure.message);e.code=result.failure.code;throw e;}return result.value;};window.component=SwarmEngineSettingsAdmin.mount(document.querySelector('#admin'),{describe:()=>call('describe'),show:()=>call('show'),preview:r=>call('preview',r),apply:r=>call('apply',r)},{translate});},{lang,theme});
  const open='[data-action="edit"]',modal='dialog[data-engine-settings]',confirm=modal+' [data-action="confirm"]',settings=path.join(root,'.swarm/engine-settings.json');
  await observed(label+' loaded',adminState.waitForSettingsState(page,'loaded'));
  await page.focus(open);await page.keyboard.press('Enter');await page.waitForSelector(modal);await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]'));assert(await page.$eval(open,e=>document.activeElement===e));
  /** Save coherent settings via actual preview/apply, assert preview is pure,
   * reopen all changed fields. Backend errors remain visible and fail the check. */
  async function save(changes){
   console.log(label+' save '+changes.map(c=>c.field).join(','));await page.click(open);await page.waitForSelector(modal);
   for(const {field,value,unlimited=false} of changes){const c=await page.$(modal+' [data-unlimited="'+field+'"]');if(c&&(await c.evaluate(e=>e.checked))!==unlimited)await c.click();if(!unlimited)await page.$eval(modal+' [data-setting="'+field+'"]',(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}));},value);}
   await page.$eval(modal+' [data-setting-reason]',e=>{e.value='Observe actual native preparation consumer';e.dispatchEvent(new Event('input',{bubbles:true}));});const original=fs.readFileSync(settings);await page.click(confirm);await adminState.waitForSettingsState(page,'preview');assert(fs.readFileSync(settings).equals(original));await page.click(confirm);await adminState.waitForSettingsState(page,'saved');
   await page.click(open);await page.waitForSelector(modal);for(const {field,value,unlimited=false} of changes){if(unlimited)assert(await page.$eval(modal+' [data-unlimited="'+field+'"]',e=>e.checked));else assert.equal(await page.$eval(modal+' [data-setting="'+field+'"]',e=>e.value),value);}await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]'));
  }
  const values={agent_interleaved_failure_lookback_multiplier:'1',agent_launch_timeout_seconds_default:'17',agent_launch_timeout_seconds_ceiling:'18',agent_launch_instruction_max_bytes:'64',agent_provider_config_max_bytes:'1024',agent_launch_context_max_bytes:'64000',agent_preflight_ttl_seconds_default:'60',agent_preflight_ttl_seconds_ceiling:'120',agent_preflight_min_free_bytes:'0',agent_environment_allow:'',agent_allow_unverified_provider_context:'1'};
  await save(Object.entries(values).map(([field,value])=>({field,value})));
  assert((await page.$eval('.engine-settings-admin',e=>e.textContent)).includes(lang==='en'?'No inherited variables':'Aucune variable héritée'));
  // Model rules are saved operator data. Public checks observe their effect;
  // they never grant provider execution, model availability or a price.
  const routing={agent_model_automatic_levels:'exigeant',agent_model_restricted_levels:'simple',agent_model_restriction_rules:'exact:fixture-sensitive\nprefix:fixture/prefix\ncontains:fixture-fragment'};
  const modelCheck=(level,model,automatic)=>cli(root,['engine-settings','model-check'],{schema_version:1,level,model,automatic});
  await assert.rejects(()=>modelCheck('exigeant','ordinary',true),e=>e.code==='settings-model-automatic-level-denied');
  await save(Object.entries(routing).map(([field,value])=>({field,value})));
  const routingBefore=fs.readFileSync(settings),selection=await modelCheck('exigeant','ordinary',true);
  assert.equal(selection.status,'allowed_by_saved_rules');assert.equal(selection.scope,'candidate_rules_only');
  assert.equal(selection.settings_revision,(await cli(root,['engine-settings','show'])).revision);
  await modelCheck('simple','fixture-sensitive',false);
  await assert.rejects(()=>modelCheck('standard','fixture-sensitive',false),e=>e.code==='settings-model-restricted-level-denied');
  assert(fs.readFileSync(settings).equals(routingBefore));
  // Invalid preview stays visible and cannot publish or erase the valid rules.
  await page.click(open);await page.waitForSelector(modal);
  await page.$eval(modal+' [data-setting="agent_model_restriction_rules"]',e=>{e.value='contains:';e.dispatchEvent(new Event('input',{bubbles:true}));});
  await page.$eval(modal+' [data-setting-reason]',e=>{e.value='Reject an invalid model rule';e.dispatchEvent(new Event('input',{bubbles:true}));});
  await page.click(confirm);await assert.rejects(()=>adminState.waitForSettingsState(page,'preview'),e=>e.code==='admin-state-failure');
  const ruleError=await page.$eval(modal+' [role="alert"]',e=>e.textContent);
  assert(ruleError.includes(lang==='en'?'Invalid model rule':'Règle de modèle invalide'));
  assert(fs.readFileSync(settings).equals(routingBefore));await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]'));
  await modelCheck('exigeant','ordinary',true);
  await save(Object.keys(routing).map(field=>({field,value:''})));
  assert((await page.$eval('.engine-settings-admin',e=>e.textContent)).includes(lang==='en'?'Empty list':'Liste vide'));
  await assert.rejects(()=>modelCheck('standard','ordinary',true),e=>e.code==='settings-model-automatic-level-denied');
  await modelCheck('standard','fixture-sensitive',false);
  await save(Object.entries(routing).map(([field,value])=>({field,value})));
  const prepared=await preview({});assert.equal(prepared.preview_only,true);assert.equal(prepared.process_started,false);assert.equal(prepared.agent.timeout_seconds,17);assert.deepEqual(prepared.agent.env_allow,[]);assert(prepared.agent.prompt.includes('mission '+work+' ; tâche task ; agent session ; tentative '+prepared.agent.attempt_id));assert(prepared.agent.prompt.includes('révision du travail 3'));assert.equal(Date.parse(prepared.agent.preflight.valid_until)-Date.parse(prepared.agent.preflight.checked_at),60000);
  await assert.rejects(()=>preview({timeout_seconds:19}),e=>e.code==='agent-launch-timeout-outside-saved-policy');
  for(const [field,value,code] of [['agent_launch_instruction_max_bytes','0','agent-launch-instruction-outside-saved-policy'],['agent_provider_config_max_bytes','0','agent-provider-catalogue-read'],['agent_launch_context_max_bytes','0','agent-launch-context-outside-saved-policy'],['agent_preflight_min_free_bytes','9223372036854775807','agent-workspace-free-space-below-policy'],['agent_allow_unverified_provider_context','0','agent-provider-verified-context-required']]){
   await preview({});const beforeWork=await cli(root,['work','show',work]);await save([{field,value}]);await assert.rejects(()=>preview({}),e=>e.code===code);assert.equal((await cli(root,['work','show',work])).work.revision,beforeWork.work.revision);await save([{field,value:values[field]}]);await preview({});
  }
  await save([{field:'agent_launch_timeout_seconds_ceiling',value:'0',unlimited:true}]);assert.equal((await preview({timeout_seconds:19})).agent.timeout_seconds,19);await save([{field:'agent_launch_timeout_seconds_ceiling',value:'18'}]);
  fs.writeFileSync(providers,JSON.stringify(provider({preflight_ttl_seconds:121})));await assert.rejects(()=>preview({}),e=>e.code==='agent-preflight-ttl-outside-saved-policy');fs.writeFileSync(providers,JSON.stringify(provider({})));await preview({});
  // Backend refusal is visible, preserves settings and can be corrected in-place.
  await page.click(open);await page.waitForSelector(modal);const unchanged=fs.readFileSync(settings);await page.$eval(modal+' [data-setting="agent_environment_allow"]',e=>{e.value='BAD=NAME';e.dispatchEvent(new Event('input',{bubbles:true}));});await page.$eval(modal+' [data-setting-reason]',e=>{e.value='Invalid fixture name';e.dispatchEvent(new Event('input',{bubbles:true}));});await page.click(confirm);await page.waitForFunction(()=>{const e=document.querySelector('dialog [role="alert"]');return e&&!e.hidden&&e.textContent.length>0;});assert(fs.readFileSync(settings).equals(unchanged));assert((await page.$eval(modal+' [role="alert"]',e=>e.textContent)).includes(lang==='en'?'valid, unique':'valides et uniques'));await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]'));
  // One frozen intention per policy, via supported native mutations only.
  // Old intention replay remains on its original factor after later Admin saves.
  const windowField='agent_interleaved_failure_lookback_multiplier';
  const trace={schema_version:1,events:[],final_at_ms:9};
  for(const [i,n] of ['A','B','C','D','A'].entries()){
   const id='event-'+i;trace.events.push({call:{at_ms:2*i,id,name:'Read',input:{file:n}}},{result:{at_ms:2*i+1,id,failed:n==='A',message:n==='A'?'permission denied':null}});
  }
  /** Prepare a new private Work so policy changes never reset/reuse an attempt.
   * Public cancellation releases the queued workspace without task acceptance.
   * No child starts; retained metadata then feeds declared diagnostic replay. */
  async function prepareGuard(suffix){
   const gw=(await cli(root,['work','create'],req('guard-create-'+suffix,0,{title:'Guard '+suffix,objective:'Frozen window witness',scope:'Private fixture',criteria:['No tools executed']}))).work.id;
   await cli(root,['task','add',gw],req('guard-add-'+suffix,1,{id:'task',title:'Observe',deliverable:'Diagnostic',criteria:['No tools executed']}));
   const id='guard-'+suffix;
   const admission=await cli(root,['agent','prepare',gw],req(id,2,{task_id:'task',provider:'fixture',limits:{max_repeated_calls:2}}));
   assert.equal(admission.process_started,false);assert.equal(admission.agent.child_pid,undefined);
   const cancelled=await cli(root,['agent','reconcile',id]);assert.equal(cancelled.cancelled_before_start,true);assert.equal(cancelled.accepted_by_cancellation,false);assert.equal(cancelled.agent.status,'interrupted');assert.deepEqual(cancelled.agent.native_tool_guard_policy,admission.agent.native_tool_guard_policy);
   assert.equal((await cli(root,['work','show',gw])).work.tasks[0].status,'blocked');
   return {id,snapshot:admission.agent.native_tool_guard_policy};
  }
  const first=await prepareGuard('one');assert.equal(first.snapshot.lookback_multiplier,'1');
  let replay=await cli(root,['agent','guard',first.id],trace);assert.equal(replay.observation.total_errors,2);assert.equal(replay.observation.stop_reason,null);assert.equal(replay.observation.calls,5);assert.equal(replay.process_started,false);
  await save([{field:windowField,value:'4'}]);const second=await prepareGuard('four');assert.equal(second.snapshot.lookback_multiplier,'4');
  replay=await cli(root,['agent','guard',second.id],trace);assert.equal(replay.observation.stop_reason,'interleaved_failures');assert.equal(replay.event_source,'caller_declared_unverified');
  replay=await cli(root,['agent','guard',first.id],trace);assert.equal(replay.lookback_multiplier,'1');assert.equal(replay.settings_revision,first.snapshot.settings_revision);assert.equal(replay.observation.stop_reason,null);
  await save([{field:windowField,value:'0',unlimited:true}]);const third=await prepareGuard('unlimited');assert.equal(third.snapshot.lookback_multiplier,null);
  replay=await cli(root,['agent','guard',third.id],trace);assert.equal(replay.observation.stop_reason,'interleaved_failures');
  await page.click(open);await page.waitForSelector(modal);const beforeInvalidWindow=fs.readFileSync(settings);
  await page.click(modal+' [data-unlimited="'+windowField+'"]');await page.$eval(modal+' [data-setting="'+windowField+'"]',e=>{e.value='0';e.dispatchEvent(new Event('input',{bubbles:true}));});await page.$eval(modal+' [data-setting-reason]',e=>{e.value='Invalid zero fixture';e.dispatchEvent(new Event('input',{bubbles:true}));});await page.click(confirm);
  await page.waitForFunction(()=>{const e=document.querySelector('dialog [role="alert"]');return e&&!e.hidden&&e.textContent.length>0;});assert(fs.readFileSync(settings).equals(beforeInvalidWindow));assert((await page.$eval(modal+' [role="alert"]',e=>e.textContent)).includes(lang==='en'?'positive':'positif'));await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]'));
  await save([{field:windowField,value:'1'}]);
  // All nine saved supervision settings are consumed by a new actual run.
  // These fixture values are witnesses, never product defaults or test deadlines.
  const supervision={agent_supervision_poll_interval_ms:'5',agent_supervision_heartbeat_stale_ms:'12345',agent_supervision_term_grace_ms:'0',agent_supervision_output_drain_ms:'1000',agent_provider_event_max_bytes:'100000',agent_capture_max_bytes:'7',agent_capture_max_lines:'1',agent_public_message_max_bytes:'0',agent_log_message_max_bytes:'0'};
  await save(Object.entries(supervision).map(([field,value])=>({field,value})));
  const liveWork=(await cli(root,['work','create'],req('native-create',0,{title:'Native supervision',objective:'Saved policy consumer',scope:'Local cat fixture',criteria:['Observed, not accepted']}))).work.id;
  await cli(root,['task','add',liveWork],req('native-add',1,{id:'task',title:'Run',deliverable:'Output',criteria:['Observed, not accepted']}));
  await cli(root,['agent','prepare',liveWork],req('native-run',2,{task_id:'task',provider:'fixture',capture_output:true}));
  const outcome=await cli(root,['agent','run','native-run']);assert.equal(outcome.task_accepted,false);assert.equal(outcome.agent.status,'completed');
  assert.deepEqual(outcome.agent.native_execution.policy,{poll_interval_ms:'5',heartbeat_stale_ms:'12345',term_grace_ms:'0',output_drain_ms:'1000',event_max_bytes:'100000',capture_max_bytes:'7',capture_max_lines:'1',public_message_max_bytes:'0',log_message_max_bytes:'0'});
  const facts=outcome.agent.native_execution_result;assert.equal(facts.kernel_reaped,true);assert.equal(facts.output_complete,true);assert.equal(facts.capture_truncated,true);assert.equal(Buffer.byteLength(facts.capture),7);assert(Number(facts.output_bytes)>7);
  assert.equal((await cli(root,['work','show',liveWork])).work.tasks[0].status,'blocked');
  // Explicit null survives save/reopen; it is not a missing/defaulted field.
  const nullable=['agent_supervision_output_drain_ms','agent_provider_event_max_bytes','agent_capture_max_bytes','agent_capture_max_lines','agent_public_message_max_bytes','agent_log_message_max_bytes'];
  await save(nullable.map(field=>({field,value:'0',unlimited:true})));
  const savedSupervision=await cli(root,['engine-settings','show']);for(const field of nullable)assert.equal(savedSupervision.values[field],null);
  await save(Object.entries(supervision).map(([field,value])=>({field,value})));
  for(const width of [320,375,768,1024,1440]){await page.setViewport({width,height:1000});await page.click(open);await page.waitForSelector(modal);await page.$eval(modal+' [data-setting="agent_interleaved_failure_lookback_multiplier"]',e=>e.scrollIntoView({block:'center'}));const layout=await page.$eval(modal,e=>({scroll:e.scrollWidth,width:e.clientWidth}));assert(layout.scroll<=layout.width+1);if([320,1440].includes(width))await page.screenshot({path:path.join(output,label+'-'+width+'.png')});await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]'));}
  assert.equal((await cli(root,['work','show',work])).work.revision,2);assert.equal(errors.length,0,JSON.stringify(errors));checks.push({label,status:'PASS',fields:Object.keys(values).length+Object.keys(supervision).length+Object.keys(routing).length,widths:[320,375,768,1024,1440],nativeConsumer:true,modelPolicyCheck:true,providerStarted:true});await page.close();
 }
 assert.deepEqual(snapshot(),before);const environment=helper.finish(runtime,repo);fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:'PASS',scope:'Bounded v19 admission/supervision/model-rule Admin with local cat, no HTTP authorization, actual agent tools or full native mission',inputs:before,checks,errors,runtime:environment,scratch},null,2)+'\n');console.log('PASS four native manual-admission Admin journeys');
}catch(e){fs.writeFileSync(path.join(output,'failure.json'),JSON.stringify({status:'FAIL',message:e.message,stack:e.stack,checks,errors,scratch},null,2)+'\n');console.error(e);process.exitCode=1;}finally{if(browser)await browser.close();if(server)await new Promise(resolve=>server.close(resolve));}})();
