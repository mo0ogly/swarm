'use strict';
// Candidate shared Admin + real native CLI. CDP is a fixture transport, not HTTP authorization.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),http=require('node:http'),crypto=require('node:crypto'),{spawn}=require('node:child_process'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
if(![9,10,11].includes(process.argv.length))throw Error('Usage: rust_engine_settings_admin_ui.cjs NATIVE_CLI ROOT_PROBE OUTPUT CONTEXT_DOCUMENT_PROBE GIT_REPORT_PROBE BASELINE_FILES_PROBE REVIEW_REPLY_PROBE [WORK_PROJECTION_PROBE [MANAGED_RECEIPT_PROBE]]');
const repo=path.resolve(__dirname,'..'),binary=path.resolve(process.argv[2]),probe=path.resolve(process.argv[3]),output=path.resolve(process.argv[4]),deliveryProbe=path.resolve(process.argv[5]),gitProbe=path.resolve(process.argv[6]),baselineProbe=path.resolve(process.argv[7]),replyProbe=path.resolve(process.argv[8]);
const workProbe=process.argv[9]?path.resolve(process.argv[9]):null;
const receiptProbe=process.argv[10]?path.resolve(process.argv[10]):null;
if(fs.existsSync(output))throw Error('Fresh evidence directory required');fs.mkdirSync(output,{recursive:true});
const scratch=fs.mkdtempSync(path.join(os.homedir(),'.swarm-rust-admin-'));fs.chmodSync(scratch,0o700);
const commands=[],checks=[],errors=[];let sequence=0,server,browser;
const adminState=require('./helpers/engine_settings_admin_state.cjs');
const verificationRuntime=require('./helpers/browser_verification_runtime.cjs'),runtime=verificationRuntime.begin(repo,output);
const files={
 '/themes.css':'web/wattson_themes.css','/base.css':'web/processes.css','/design.css':'web/swarm-design.css',
 '/admin.css':'frontend/engine-settings/admin.css','/admin.js':'frontend/engine-settings/admin.js','/english.js':'web/i18n-en.js','/swarm-logo.png':'web/swarm-logo.png'};
const sourcePaths=[...Object.values(files),'locales/en.json','rust/crates/swarm-engine/config/engine-settings-catalog.json','rust/crates/swarm-engine/config/engine-settings.json','tests/rust_engine_settings_admin_ui.cjs','tests/helpers/browser_verification_runtime.cjs','tests/helpers/engine_settings_admin_state.cjs','package-lock.json','rust/crates/swarm-engine/tests/fixtures/legacy-v1-document.json'];
const snapshot=()=>Object.fromEntries([...sourcePaths,binary,probe,deliveryProbe,gitProbe,baselineProbe,replyProbe,...(workProbe?[workProbe]:[]),...(receiptProbe?[receiptProbe]:[])].map(p=>[p,crypto.createHash('sha256').update(fs.readFileSync(path.isAbsolute(p)?p:path.join(repo,p))).digest('hex')]));const before=snapshot();
// Preflight every literal and descriptor string before starting browser work.
const english=JSON.parse(fs.readFileSync(path.join(repo,'locales/en.json'))),catalogue=JSON.parse(fs.readFileSync(path.join(repo,'rust/crates/swarm-engine/config/engine-settings-catalog.json')));
for(const match of fs.readFileSync(path.join(repo,'frontend/engine-settings/admin.js'),'utf8').matchAll(/tr\('([^']*)'\)/g))assert(Object.hasOwn(english,match[1]),'missing literal translation: '+match[1]);
for(const field of catalogue.fields)for(const key of ['label','help'])assert(Object.hasOwn(english,field[key]),'missing catalogue translation: '+field[key]);
const vm=require('node:vm'),context={};vm.runInNewContext(fs.readFileSync(path.join(repo,'frontend/engine-settings/admin.js'),'utf8'),context);for(const label of Object.values(context.SwarmEngineSettingsAdmin.errorLabels))assert(Object.hasOwn(english,label),'missing error translation: '+label);

function processRun(label,args,input){return new Promise((resolve,reject)=>{const start=new Date().toISOString(),p=spawn(args[0],args.slice(1),{cwd:repo,env:{...process.env,TMPDIR:scratch}});let out=[],err=[];p.stdout.on('data',d=>out.push(d));p.stderr.on('data',d=>err.push(d));p.on('error',reject);p.on('close',code=>{const stdout=Buffer.concat(out),stderr=Buffer.concat(err);commands.push({label,args,start,code,stdout:stdout.toString(),stderr:stderr.toString()});fs.writeFileSync(path.join(output,'commands.json'),JSON.stringify(commands,null,2)+'\n');resolve({code,stdout,stderr})});p.stdin.end(input)})}
// Create a real bare Git report without a provider, shell or global watchdog.
async function prepareGit(root,label){
 const dir=path.join(root,'repository.git');let r=await processRun(label+'-git-init',['git','init','--bare',dir]);assert.equal(r.code,0);
 r=await processRun(label+'-git-blob',['git','--git-dir='+dir,'hash-object','-w','--stdin'],Buffer.from('Admin Git report\n'));assert.equal(r.code,0);const blob=r.stdout.toString().trim();
 r=await processRun(label+'-git-tree',['git','--git-dir='+dir,'mktree'],Buffer.from('100644 blob '+blob+'\treport.md\n'));assert.equal(r.code,0);const tree=r.stdout.toString().trim();
 r=await processRun(label+'-git-commit',['git','--git-dir='+dir,'-c','user.name=Fixture','-c','user.email=fixture@localhost','commit-tree',tree],Buffer.from('Fixture\n'));assert.equal(r.code,0);
 return {repository:'repository.git',candidate:r.stdout.toString().trim(),report:'report.md',version:2};
}
// Observe the actual Root consumer under saved settings, preserving configuration bytes.
async function readSavedGit(root,label,frame,reason){
 const file=path.join(root,'.swarm/engine-settings.json'),before=fs.readFileSync(file);
 const r=await processRun(label,[gitProbe,root],JSON.stringify(frame)+'\n');assert.equal(r.code,reason?2:0);assert.equal(r.stderr.length,0);const value=JSON.parse(r.stdout);
 if(reason)assert.equal(value.reason,'git-report: '+reason);else assert.equal(value.sha256,crypto.createHash('sha256').update('Admin Git report\n').digest('hex'));
 assert(fs.readFileSync(file).equals(before));return value;
}
// Supplied-review fixture with real local proof files, not a forged stored Work or identity.
function prepareBaseline(root){
 const hash=raw=>crypto.createHash('sha256').update(raw).digest('hex');
 const review=(name,candidate,previous,children)=>{const context={candidate_commit:candidate,previous_candidate:previous,receipt:null,tasks:[]};if(children.length)context.accepted_baseline={version:2,candidate_commit:previous,accepted_reviews:children};const raw=Buffer.from(JSON.stringify(context)),receipt=Buffer.from('Receipt'),report=Buffer.from('Report');fs.writeFileSync(path.join(root,name+'.context'),raw,{mode:0o600});fs.writeFileSync(path.join(root,name+'.receipt'),receipt,{mode:0o600});fs.writeFileSync(path.join(root,name+'.report'),report,{mode:0o600});return {id:name,state:'passed',candidate_commit:candidate,context:name+'.context',context_sha256:hash(raw),receipt:name+'.receipt',receipt_sha256:hash(receipt),report:name+'.report',sha256:hash(report)}};
 const leaf=review('baseline-leaf','old','',[]);return {tasks:[{independent_review:review('baseline-root','current','old',[leaf])}]};
}
// Read actual saved settings and the shared historical-files guard; no stored-Work/authority claim.
async function readSavedBaseline(root,label,frame,reason){
 const file=path.join(root,'.swarm/engine-settings.json'),before=fs.readFileSync(file),r=await processRun(label,[baselineProbe,root],JSON.stringify(frame)+'\n');assert.equal(r.code,reason?2:0);assert.equal(r.stderr.length,0);const value=JSON.parse(r.stdout);if(reason)assert.equal(value.reason,reason);else {assert.equal(value.files_consistent,true);assert.equal(value.max_depth,1);assert.equal(value.reviews_checked,2)}assert(fs.readFileSync(file).equals(before));return value;
}
async function cli(root,op,request){let input=[];if(request!==undefined){const file=path.join(scratch,'request-'+(++sequence)+'.json');fs.writeFileSync(file,JSON.stringify(request),{mode:0o600});input=['--input',file]}
 const result=await processRun(op.join('-'),[binary,'--root',root,'--json',...op,...input]);if(result.code){const raw=JSON.parse(result.stderr);assert.equal(raw.schema_version,1);assert.equal(typeof raw.failure?.code,'string');const e=Error(raw.failure.message);e.code=raw.failure.code;throw e}return JSON.parse(result.stdout)}
// Parse supplied originals against the actual stored task/current saved policy; never authenticate a verdict.
async function readSavedReply(root,work,label,code){
 const file=path.join(root,'.swarm/engine-settings.json'),before=fs.readFileSync(file);
 const frame={work,task:'task',raw:JSON.stringify({reason:'Valid rationale',criteria:[{index:1,verdict:'pass',evidence:'Original exact evidence'}]}),originals:[{name:'proof.md',kind:'source',content:'Original `exact` evidence'}]};
 const r=await processRun(label,[replyProbe,'root',root],JSON.stringify(frame));assert.equal(r.code,code?2:0);assert.equal(r.stderr.length,0);const value=JSON.parse(r.stdout);
 if(code)assert.equal(value.code,code);else {assert.equal(value.observation.parsed.state,'passed');assert.equal(value.observation.work_id,work);assert.equal(value.observation.task_id,'task')}
 assert(fs.readFileSync(file).equals(before));return value;
}
async function mutate(root,work,kind,request){const r=await processRun(kind,[probe,root],JSON.stringify({work,kind,request})+'\n');assert.equal(r.code,0);return JSON.parse(r.stdout)}
async function evaluateSavedQuality(root,label,expected){
 const proof=Buffer.from('Admin quality consumer proof');fs.writeFileSync(path.join(root,'quality-proof.txt'),proof,{mode:0o600});
 const document={method_version:'3',scope_id:'quality-consumer',artifacts:{'quality-proof.txt':crypto.createHash('sha256').update(proof).digest('hex')},domains:{quality:1},checks:[{id:'score',domain:'quality',mandatory:false,gate:'delivery',severity:'minor',penalty:28,max_penalty:100}],results:[{id:'score',status:'FAIL',count:1,evidence:['quality-proof.txt']}]};
 const input=path.join(scratch,'quality-'+(++sequence)+'.json');fs.writeFileSync(input,JSON.stringify(document),{mode:0o600});
 const settings=path.join(root,'.swarm/engine-settings.json'),before=fs.readFileSync(settings);
 const result=await processRun(label,[binary,'evaluate','--root',root,'--input',input,'--json']);assert.equal(result.code,expected);assert(fs.readFileSync(settings).equals(before));
 if(expected===2){assert.equal(result.stdout.length,0);assert.equal(JSON.parse(result.stderr).failure.code,'settings-quality-policy-not-configured');return null}
 assert.equal(result.stderr.length,0);const value=JSON.parse(result.stdout);assert.equal(value.evaluation.quality,72);assert.equal(value.evaluation.ship_allowed,expected===0);return value;
}
async function readSavedContext(root,label,expected,legacy=false){
 fs.writeFileSync(path.join(root,'context-guide.md'),'Project context rules',{mode:0o600});fs.writeFileSync(path.join(root,'swarm.project.json'),JSON.stringify({version:1,name:'Context consumer',instructions:[{path:'context-guide.md',roles:['reviewer']}]}),{mode:0o600});
 const file=path.join(root,'.swarm/engine-settings.json'),before=fs.readFileSync(file);const result=await processRun(label,[binary,'project-context','reviewer','--root',root,'--json']);assert.equal(result.code,expected);assert(fs.readFileSync(file).equals(before));
 if(expected===2){assert.equal(result.stdout.length,0);assert.equal(JSON.parse(result.stderr).failure.code,legacy?'settings-context-policy-not-configured':'project-context-invalid');return null}
 assert.equal(result.stderr.length,0);const observed=JSON.parse(result.stdout);assert.equal(observed.context.role,'reviewer');assert(observed.prompt.includes('Project context rules'));return observed;
}
async function readSavedDelivery(root,label,expected,legacy=false){
 const name='docs/review-delivery.md';fs.mkdirSync(path.join(root,'docs'),{recursive:true});fs.writeFileSync(path.join(root,name),'Review delivery rules',{mode:0o600});
 const file=path.join(root,'.swarm/engine-settings.json'),before=fs.readFileSync(file);const result=await processRun(label,[deliveryProbe,root],JSON.stringify({document:name}));assert.equal(result.code,expected);assert(fs.readFileSync(file).equals(before));
 const observed=JSON.parse(result.stdout);if(expected===2){assert.equal(observed.code,legacy?'settings-context-policy-not-configured':'project-context-invalid');return null}
 assert.equal(result.stderr.length,0);assert.equal(observed.document,'Review delivery rules');return observed;
}
const req=(event,revision,fields)=>({schema_version:1,event_id:event,expected_revision:revision,...fields});
(async()=>{
 try{
 server=http.createServer((request,response)=>{const pathname=new URL(request.url,'http://localhost').pathname;
  if(pathname==='/'){response.writeHead(200,{'Content-Type':'text/html; charset=utf-8','Content-Security-Policy':"default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'none'"});response.end('<!doctype html><html lang="fr"><head><meta name="viewport" content="width=device-width,initial-scale=1"><title>Admin candidate</title><link rel="icon" href="/swarm-logo.png"><link rel="stylesheet" href="/themes.css"><link rel="stylesheet" href="/base.css"><link rel="stylesheet" href="/design.css"><link rel="stylesheet" href="/admin.css"><script src="/english.js" defer></script><script src="/admin.js" defer></script></head><body><main id="admin"><h1>Administration</h1></main></body></html>');return}
  const file=files[pathname];if(!file){response.writeHead(404);response.end();return}response.writeHead(200,{'Content-Type':pathname.endsWith('.css')?'text/css':pathname.endsWith('.js')?'text/javascript':'image/png'});response.end(fs.readFileSync(path.join(repo,file)));
 });await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const url='http://127.0.0.1:'+server.address().port;
 browser=await puppeteer.launch({...verificationRuntime.launchOptions,headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});await verificationRuntime.recordBrowser(runtime,browser);
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const label=lang+'-'+theme,root=path.join(scratch,label);fs.mkdirSync(root,{mode:0o700});await cli(root,['init']);fs.writeFileSync(path.join(root,'.rust-migration-fixture'),'Owned Admin fixture.\n');
  const gitFrame=await prepareGit(root,label),baselineFrame=prepareBaseline(root);
  const currentCatalog=await cli(root,['engine-settings','describe']);
  // Existing task fixture is created under real configured defaults. A retained
  // legacy settings document is then injected only into this declared fixture;
  // it must block NEW creation until explicit Admin adoption, preserving history.
  const created=await mutate(root,'','work.create',req('create',0,{title:'Admin proof',objective:'Actual consumer',scope:'Isolated',criteria:['Proof']}));assert.equal(created.code,'ok');const wid=created.work.id;
  assert.equal((await mutate(root,wid,'task.add',req('task',1,{id:'task',title:'Task',deliverable:'report',criteria:['Proof']}))).code,'ok');
  const legacy=theme==='sombre',settingsPath=path.join(root,'.swarm/engine-settings.json');if(legacy)fs.writeFileSync(settingsPath,fs.readFileSync(path.join(repo,'rust/crates/swarm-engine/tests/fixtures/legacy-v1-document.json')));const baseline=await cli(root,['engine-settings','show']),savedHistory=JSON.stringify(baseline.history);
  if(legacy){const bytes=fs.readFileSync(path.join(root,'.swarm/state.db')),policy=fs.readFileSync(settingsPath);const denied=await mutate(root,wid,'task.add',req('legacy-new-phase-task',2,{id:'legacy-new',title:'New task',deliverable:'Report',criteria:['Proof']}));assert.equal(denied.code,'format');assert(fs.readFileSync(path.join(root,'.swarm/state.db')).equals(bytes));assert(fs.readFileSync(settingsPath).equals(policy))}
  await readSavedGit(root,label+'-git-before',gitFrame,legacy?'settings-git-policy-not-configured':null);
  await readSavedBaseline(root,label+'-baseline-before',baselineFrame,legacy?'settings-review-baseline-policy-not-configured':null);
  await evaluateSavedQuality(root,label+'-quality-before',legacy?2:0);
  await readSavedContext(root,label+'-context-before',legacy?2:0,legacy);await readSavedDelivery(root,label+'-delivery-before',legacy?2:0,legacy);
  await readSavedReply(root,wid,label+'-reply-before',legacy?'settings-review-reply-policy-not-configured':null);
  const editRequest=req('edit',2,{id:'task',max_attempts:4});assert.notEqual((await mutate(root,wid,'task.update',editRequest)).code,'ok');
  const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  let fatal;const rejected=new Promise((_,reject)=>fatal=reject);rejected.catch(()=>{});const observed=(name,promise)=>{console.log(label,name);return Promise.race([promise,rejected])};
  const fail=message=>{errors.push(label+': '+message);fatal(Error(message))};page.on('pageerror',e=>fail(e.message));page.on('requestfailed',r=>fail(r.url()+': '+r.failure()?.errorText));page.on('console',m=>{if(m.type()==='error')fail(m.text())});page.on('response',r=>{if(r.status()>=400)fail('HTTP '+r.status()+' '+r.url())});
  await page.exposeFunction('nativeSettings',async(op,request)=>{try{return {value:await cli(root,['engine-settings',op],request)}}catch(e){return {failure:{code:e.code,message:e.message}}}});
  await observed('load',page.goto(url));await page.evaluate(({lang,theme})=>{document.documentElement.lang=lang;document.documentElement.dataset.theme=theme;const translate=s=>{if(lang==='fr')return s;if(!Object.hasOwn(SwarmEnglish,s))throw Error('Missing English: '+s);return SwarmEnglish[s]};window.failureMode='';const call=async(op,r)=>{const result=await(r===undefined?nativeSettings(op):nativeSettings(op,r));if(result.failure){const e=Error(result.failure.message);e.code=result.failure.code;throw e}return result.value};window.component=SwarmEngineSettingsAdmin.mount(document.querySelector('#admin'),{
   describe:()=>call('describe'),show:async()=>{if(window.failureMode==='load')throw Error('fixture load refusal');const value=await call('show');if(window.failureMode==='phase-bool')value.values.task_manual_required_gate_phases=true;if(window.failureMode==='phase-unknown')value.values.task_manual_required_gate_phases=['unknown'];if(window.failureMode==='phase-null')value.values.task_manual_required_gate_phases=null;if(window.failureMode==='phase-duplicate')value.values.task_manual_required_gate_phases=['entry','entry'];return value},preview:r=>call('preview',r),apply:async r=>{const result=await call('apply',r);if(window.failureMode==='response-loss'){window.failureMode='';throw Error('fixture response lost after publication')}return result}
  },{translate})},{lang,theme});
  /** Reveal the logical group before a real pointer action on its control. */
  async function clickSetting(selector){await page.$eval(selector,e=>{const section=e.closest('[data-settings-group]');if(section)section.open=true});await page.click(selector)}
  const open='[data-action="edit"]',modal='dialog[data-engine-settings]',confirm=modal+' [data-action="confirm"]',input=modal+' [data-setting="task_max_attempts"]';
  await observed('loaded',adminState.waitForSettingsState(page,'loaded'));
  if(legacy)assert((await page.$eval('.engine-settings-admin',e=>e.textContent)).includes(lang==='en'?'Not configured in this version':'Non configuré dans cette version'));
  assert((await page.$eval('[data-input-transport]',e=>e.textContent)).includes('16777216'));assert.equal(await page.$$eval('[data-input-transport] input',e=>e.length),0);
  // Keyboard opening, Escape and focus restoration use real native dialog behavior.
  await page.focus(open);await page.keyboard.press('Enter');await observed('dialog',page.waitForSelector(modal));await page.keyboard.press('Escape');await observed('escape',page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]')));assert(await page.$eval(open,e=>document.activeElement===e));
  await page.click(open);await observed('reopen',page.waitForSelector(modal));assert.equal(await page.$eval(input,e=>e.value),'3');
  assert.equal(await page.$$eval('[data-historical-quality] input',e=>e.length),0);assert((await page.$eval('[data-historical-quality]',e=>e.textContent)).includes(lang==='en'?'Historical method 2':'Méthode historique 2'));
  const settingsFile=path.join(root,'.swarm/engine-settings.json'),initial=fs.readFileSync(settingsFile);
  assert.equal(await page.$eval(modal+' [data-setting="validation_max_output_bytes"]',e=>e.value),'65536');if(legacy)assert((await page.$eval(modal,e=>e.textContent)).includes(lang==='en'?'This journal uses an older version':'Ce journal utilise une ancienne version'));
  const retention=legacy?'0':lang==='en'?null:'5';if(retention===null)await clickSetting(modal+' [data-unlimited="validation_max_output_bytes"]');else await page.$eval(modal+' [data-setting="validation_max_output_bytes"]',(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},retention);
  await page.$eval(modal+' [data-setting="managed_review_max_baseline_depth"]',e=>{e.value='0';e.dispatchEvent(new Event('input',{bubbles:true}))});
  await page.$eval(modal+' [data-setting="git_command_timeout_seconds"]',e=>{e.value='0';e.dispatchEvent(new Event('input',{bubbles:true}))});
  await page.$eval(modal+' [data-setting="project_context_max_bytes"]',e=>{e.value='1';e.dispatchEvent(new Event('input',{bubbles:true}))});
  await page.$eval(modal+' [data-setting="review_delivery_max_bytes"]',e=>{e.value='1';e.dispatchEvent(new Event('input',{bubbles:true}))});
  await page.$eval(modal+' [data-setting="quality_delivery_min_percent"]',e=>{e.value='75';e.dispatchEvent(new Event('input',{bubbles:true}))});
  await page.$eval(modal+' [data-setting="validation_allowed_programs"]',e=>{e.value='cargo fmt';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.$eval(modal+' [data-setting-reason]',e=>{e.value='Refuse one invalid program line';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.click(confirm);await observed('program-line-refused',page.waitForFunction(()=>{const e=document.querySelector('dialog [role="alert"]');return e&&!e.hidden}));assert(fs.readFileSync(settingsFile).equals(initial));assert((await page.$eval(modal+' [role="alert"]',e=>e.textContent)).includes(lang==='en'?'Enter program names':'Saisissez des noms de programmes'));
  await page.$eval(modal+' [data-setting="validation_allowed_programs"]',e=>{e.value='cargo\nrustc';e.dispatchEvent(new Event('input',{bubbles:true}))});
  await page.$eval(modal+' [data-setting="quality_band_ok_min_percent"]',e=>{e.value='40';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.click(confirm);await observed('quality-band-refused',page.waitForFunction(()=>{const e=document.querySelector('dialog [role="alert"]');return e&&!e.hidden}));assert(fs.readFileSync(settingsFile).equals(initial));assert((await page.$eval(modal+' [role="alert"]',e=>e.textContent)).includes(lang==='en'?'Band thresholds must be ordered':'Les seuils des bandes doivent être ordonnés'));await page.$eval(modal+' [data-setting="quality_band_ok_min_percent"]',e=>{e.value='80';e.dispatchEvent(new Event('input',{bubbles:true}))});
  await page.$eval(input,e=>{e.value='9007199254740993';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.$eval(modal+' [data-setting-reason]',e=>{e.value='Exact browser integer and consumer proof';e.dispatchEvent(new Event('input',{bubbles:true}))});
  await clickSetting(modal+' [data-unlimited="task_max_criteria"]');assert(await page.$eval(modal+' [data-setting="task_max_criteria"]',e=>e.disabled));
  await page.click(confirm);await observed('preview',page.waitForFunction(()=>!document.querySelector('[data-setting-preview]').hidden));assert(fs.readFileSync(settingsFile).equals(initial));assert((await page.$eval('[data-setting-preview]',e=>e.textContent)).includes('9007199254740993'));
  for(const width of [1440,1024,760,390,320]){
   await page.setViewport({width,height:width<760?740:1000});const dims=await page.evaluate(()=>{const dialog=document.querySelector('dialog'),bounds=dialog.getBoundingClientRect(),title=dialog.querySelector('h2').getBoundingClientRect(),confirm=dialog.querySelector('[data-action="confirm"]').getBoundingClientRect();return {global:document.documentElement.scrollWidth<=innerWidth,dialog:dialog.scrollWidth<=dialog.clientWidth+1,title:title.top>=bounds.top&&title.bottom<=bounds.bottom,confirm:confirm.top>=bounds.top&&confirm.bottom<=bounds.bottom}});assert(dims.global&&dims.dialog&&dims.title&&dims.confirm,JSON.stringify({label,width,dims}));
   if(width===1440||width===320)await page.screenshot({path:path.join(output,label+'-'+width+'.png')});
  }
  await page.setViewport({width:1440,height:1000});await page.evaluate(()=>window.failureMode='response-loss');await page.click(confirm);await observed('lost-response-error',page.waitForFunction(()=>{const e=document.querySelector('dialog [role="alert"]');return e&&!e.hidden&&e.textContent.includes('fixture response lost')}));
  const published=await cli(root,['engine-settings','show']);assert.equal(published.values.task_max_attempts,'9007199254740993');assert.equal(published.history.length,baseline.history.length+1);assert.equal(published.schema_version,currentCatalog.document_schema_version);assert.equal(published.values.validation_max_output_bytes,retention);assert.equal(JSON.stringify(published.history.slice(0,baseline.history.length)),savedHistory);assert.equal(published.values.task_max_criteria,null);assert.deepEqual(published.values.validation_allowed_programs,['cargo','rustc']);
  assert.equal(published.values.managed_review_max_baseline_depth,'0');await readSavedBaseline(root,label+'-baseline-zero',baselineFrame,'managed-review-baseline-depth-exceeded');
  assert.equal(published.values.git_command_timeout_seconds,'0');await readSavedGit(root,label+'-git-zero',gitFrame,'git-command-timeout');
  assert.equal(published.values.quality_delivery_min_percent,'75');const firstPolicy=await cli(root,['engine-settings','quality-profile']);assert.equal(firstPolicy.generation,'1');assert.equal(firstPolicy.legacy_method2_thresholds_compatible,false);const savedCalculation=await evaluateSavedQuality(root,label+'-quality-saved',1);assert.equal(savedCalculation.policy.digest,firstPolicy.digest);assert.equal(published.values.project_context_max_bytes,'1');await readSavedContext(root,label+'-context-saved',2);assert.equal(published.values.review_delivery_max_bytes,'1');await readSavedDelivery(root,label+'-delivery-saved',2);
  await page.click(confirm);await observed('replay-close',page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]')));await observed('replay-reload',adminState.waitForSettingsState(page,'loaded'));
  const replay=await cli(root,['engine-settings','show']);assert.equal(replay.history.length,baseline.history.length+1);assert.equal(replay.revision,String(Number(baseline.revision)+1));assert.equal((await mutate(root,wid,'task.update',editRequest)).code,'ok');
  await page.click(open);await observed('persisted-dialog',page.waitForSelector(modal));assert.equal(await page.$eval(input,e=>e.value),'9007199254740993');assert(await page.$eval(modal+' [data-unlimited="task_max_criteria"]',e=>e.checked));
  assert.equal(await page.$eval(modal+' [data-setting="git_command_timeout_seconds"]',e=>e.value),'0');
  // Invalid input reaches authoritative validation; the saved values stay intact.
  await page.$eval(input,e=>{e.value='0';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.$eval(modal+' [data-setting-reason]',e=>{e.value='Rejected zero cap';e.dispatchEvent(new Event('input',{bubbles:true}))});const saved=fs.readFileSync(settingsFile);await page.click(confirm);await observed('rejected-zero',page.waitForFunction(()=>{const e=document.querySelector('dialog [role="alert"]');return e&&!e.hidden}));assert(fs.readFileSync(settingsFile).equals(saved));assert.equal(await page.$eval(input,e=>e.value),'0');
  const zeroMessage=await page.$eval(modal+' [role="alert"]',e=>e.textContent);assert(zeroMessage.includes(lang==='en'?'Task caps must':'Les plafonds des tâches'));
  // Concurrent operator change makes the open editor stale; no stale publication.
  const external=await cli(root,['engine-settings','show']),externalChange={schema_version:external.schema_version,event_id:'external-operator',expected_revision:external.revision,values:{...external.values,task_max_tool_calls:'101'},reason:'Another operator change'};
  externalChange.preview_token=(await cli(root,['engine-settings','preview'],externalChange)).preview_token;await cli(root,['engine-settings','apply'],externalChange);const concurrent=fs.readFileSync(settingsFile);
  await page.$eval(input,e=>{e.value='6';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.click(confirm);await observed('stale-editor',page.waitForFunction(()=>{const e=document.querySelector('dialog [role="alert"]');return e&&!e.hidden}));assert(fs.readFileSync(settingsFile).equals(concurrent));const conflictMessage=await page.$eval(modal+' [role="alert"]',e=>e.textContent);assert(conflictMessage.includes(lang==='en'?'Settings have changed':'La configuration a changé'));assert.equal(await page.$eval(input,e=>e.value),'6');

  await page.keyboard.press('Escape');await observed('close-invalid',page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]')));
  // Failed loading disables editing, then explicit reload recovers.
  await page.evaluate(()=>window.failureMode='load');await page.click('[data-action="reload"]');await observed('load-error',page.waitForFunction(()=>document.querySelector('.engine-settings-admin [role="status"]').textContent.includes('fixture load refusal')));assert(await page.$eval(open,e=>e.disabled));
  await page.evaluate(()=>window.failureMode='');await page.click('[data-action="reload"]');await observed('load-recovery',adminState.waitForSettingsState(page,'loaded'));
  // Await the public reload result directly: malformed transport data must
  // disable editing immediately, not produce a timed wait or uncaught open error.
  const transportPolicy=fs.readFileSync(settingsFile),transportState=fs.readFileSync(path.join(root,'.swarm/state.db'));
  for(const mode of ['phase-bool','phase-unknown','phase-null','phase-duplicate']){
   await page.evaluate(async()=>{window.failureMode='';await window.component.reload()});assert(!await page.$eval(open,e=>e.disabled));
   await page.evaluate(async mode=>{window.failureMode=mode;await window.component.reload()},mode);
   assert(await page.$eval('.engine-settings-admin [role="status"]',e=>e.classList.contains('alert')),'Malformed phase data should be refused: '+mode);assert(await page.$eval(open,e=>e.disabled));assert(fs.readFileSync(settingsFile).equals(transportPolicy));assert(fs.readFileSync(path.join(root,'.swarm/state.db')).equals(transportState));
  }
  await page.evaluate(async()=>{window.failureMode='';await window.component.reload()});assert(!await page.$eval(open,e=>e.disabled));
  // Explicit unlimited Git policy is saved through the same Admin, then reopened.
  await page.click(open);await observed('git-unlimited-dialog',page.waitForSelector(modal));await clickSetting(modal+' [data-unlimited="git_command_timeout_seconds"]');await clickSetting(modal+' [data-unlimited="managed_review_max_baseline_depth"]');
  await page.$eval(modal+' [data-setting-reason]',e=>{e.value='Explicit unlimited Git deadline';e.dispatchEvent(new Event('input',{bubbles:true}))});
  await page.click(confirm);await observed('git-unlimited-preview',page.waitForFunction(()=>!document.querySelector('[data-setting-preview]').hidden));await page.click(confirm);await observed('git-unlimited-save',page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]')));
  await observed('git-unlimited-reload',page.waitForFunction(()=>!document.querySelector('[data-action="edit"]').disabled));await readSavedGit(root,label+'-git-unlimited',gitFrame,null);
  assert.equal((await cli(root,['engine-settings','show'])).values.git_command_timeout_seconds,null);assert.equal((await cli(root,['engine-settings','show'])).values.managed_review_max_baseline_depth,null);await readSavedBaseline(root,label+'-baseline-unlimited',baselineFrame,null);
  await page.click(open);await observed('git-unlimited-reopen',page.waitForSelector(modal));assert(await page.$eval(modal+' [data-unlimited="git_command_timeout_seconds"]',e=>e.checked));assert(await page.$eval(modal+' [data-unlimited="managed_review_max_baseline_depth"]',e=>e.checked));await page.keyboard.press('Escape');await observed('git-unlimited-close',page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]')));
  const qualityDoc=await cli(root,['engine-settings','show']),revertQuality={schema_version:qualityDoc.schema_version,event_id:'revert-quality',expected_revision:qualityDoc.revision,values:{...qualityDoc.values,quality_delivery_min_percent:'70',project_context_max_bytes:null,review_delivery_max_bytes:null},reason:'Restore threshold without reviving an old receipt'};revertQuality.preview_token=(await cli(root,['engine-settings','preview'],revertQuality)).preview_token;await cli(root,['engine-settings','apply'],revertQuality);const revertedPolicy=await cli(root,['engine-settings','quality-profile']);assert.equal(revertedPolicy.generation,'2');assert.notEqual(revertedPolicy.digest,firstPolicy.digest);assert.equal(revertedPolicy.legacy_method2_thresholds_compatible,false);const restoredCalculation=await evaluateSavedQuality(root,label+'-quality-restored',0);assert.equal(restoredCalculation.policy.digest,revertedPolicy.digest);assert.notEqual(restoredCalculation.evaluation.config_digest,savedCalculation.evaluation.config_digest);await readSavedContext(root,label+'-context-restored',0);await readSavedDelivery(root,label+'-delivery-restored',0);
  // Each new policy is saved/reopened in Admin, then affects the actual stored-task parser.
  const replyQualityBefore=await cli(root,['engine-settings','quality-profile']);
  // The prior CLI restoration changed revision after the component's last load.
  await page.click('[data-action="reload"]');await observed('reply-policy-reloaded',adminState.waitForSettingsState(page,'loaded'));
  // Set the intended finite/null policy explicitly, including transitions from
  // an already checked Unlimited control. Save/reopen then verify the consumer.
  /** Save one explicit finite/null policy through the real settings CLI transport,
   * reopen Admin and assert persisted values. Uses the current isolated root/page;
   * assertion/transport errors reject. This fixture proves no HTTP authorization. */
  async function savePolicyField(field,value,unlimited=false){return savePolicyFields([{field,value,unlimited}]);}
  /** Save one coherent set through preview/apply, verify no preview write and
   * reopen every field. Uses real CLI in this isolated root; no HTTP authority. */
  async function savePolicyFields(changes){
   const tag=changes.map(c=>c.field).join('-');
   await page.click(open);await observed('policy-dialog-'+tag,page.waitForSelector(modal));
   for(const {field,value,unlimited=false} of changes){
    const control=await page.$(modal+' [data-unlimited="'+field+'"]');
    if(control&&(await control.evaluate(e=>e.checked))!==unlimited)await control.click();
    if(!unlimited)await page.$eval(modal+' [data-setting="'+field+'"]',(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},value);
   }
   await page.$eval(modal+' [data-setting-reason]',e=>{e.value='Observe saved policy at its actual consumer';e.dispatchEvent(new Event('input',{bubbles:true}))});
   const before=fs.readFileSync(settingsFile);await page.click(confirm);await observed('policy-preview-'+tag,adminState.waitForSettingsState(page,'preview'));assert(fs.readFileSync(settingsFile).equals(before));
   await page.click(confirm);await observed('policy-saved-'+tag,adminState.waitForSettingsState(page,'saved'));
   await page.click(open);await observed('policy-reopened-'+tag,page.waitForSelector(modal));
   for(const {field,value,unlimited=false} of changes){if(unlimited)assert(await page.$eval(modal+' [data-unlimited="'+field+'"]',e=>e.checked));else assert.equal(await page.$eval(modal+' [data-setting="'+field+'"]',e=>e.value),value);}
   if(changes.length>1){for(const width of [1440,320]){await page.setViewport({width,height:1000});await page.$eval(modal+' [data-setting="runtime_silence_seconds_default"]',e=>e.scrollIntoView({block:'center'}));const layout=await page.$eval(modal,e=>({scroll:e.scrollWidth,width:e.clientWidth}));assert(layout.scroll<=layout.width+1);await page.screenshot({path:path.join(output,label+'-runtime-'+(++sequence)+'-'+width+'.png')});}await page.setViewport({width:1440,height:1000});}
   await page.keyboard.press('Escape');await observed('policy-close-'+tag,page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]')));
  }
  const runtimeValues={silence_seconds:[181,86401],tool_seconds:[301,86401],max_tool_calls:[101,10001],max_repeated_calls:[5,101],max_consecutive_errors:[4,101]};
  const runtimeChanges=Object.entries(runtimeValues).flatMap(([name,values])=>values.map((value,index)=>({field:'runtime_'+name+'_'+(index?'ceiling':'default'),value:String(value)})));
  await savePolicyFields(runtimeChanges);
  const runtimeObserved=await cli(root,['agent','limits','work','worker','task'],{schema_version:1});
  assert.equal(runtimeObserved.preview_only,true);assert.equal(runtimeObserved.launch_authorized,false);
  for(const [name,[value]] of Object.entries(runtimeValues))assert.equal(runtimeObserved.limits[name],value);
  for(const [name,[,ceiling]] of Object.entries(runtimeValues)){
   await assert.rejects(()=>cli(root,['agent','limits','work','worker','task'],{schema_version:1,provider:{[name]:ceiling+1}}),e=>e.code==='runtime-limit-outside-saved-policy');
  }
  await savePolicyFields(Object.keys(runtimeValues).map(name=>({field:'runtime_'+name+'_ceiling',value:'0',unlimited:true})));
  for(const [name,[,ceiling]] of Object.entries(runtimeValues))assert.equal((await cli(root,['agent','limits','work','worker','task'],{schema_version:1,provider:{[name]:ceiling+1}})).limits[name],ceiling+1);
  await savePolicyFields(Object.entries(runtimeValues).map(([name,[,ceiling]])=>({field:'runtime_'+name+'_ceiling',value:String(ceiling)})));

  // V16 launch policies are saved/reopened in Admin, then exercised by the
  // actual default-profile preparation CLI. Local command observation only.
  const launchSaved=await cli(root,['engine-settings','show']);
  const launchChanges=[
   ['agent_launch_timeout_seconds_default','17'],['agent_launch_timeout_seconds_ceiling','18'],
   ['agent_launch_instruction_max_bytes','64'],['agent_provider_config_max_bytes','1024'],
   ['agent_launch_context_max_bytes','64000'],['agent_preflight_ttl_seconds_default','60'],
   ['agent_preflight_ttl_seconds_ceiling','120'],['agent_preflight_min_free_bytes','0'],
   ['agent_environment_allow',''],['agent_allow_unverified_provider_context','1']
  ];
  await savePolicyFields(launchChanges.map(([field,value])=>({field,value})));
  assert((await page.$eval('.engine-settings-admin',e=>e.textContent)).includes(lang==='en'?'No inherited variables':'Aucune variable héritée'));
  const launchWork=await cli(root,['work','create'],req(label+'-launch-create',0,{title:'Native preparation',objective:'Observe Admin admission policy',scope:'Fixture',criteria:['No child process']}));
  const launchWorkId=launchWork.work.id;
  await cli(root,['task','add',launchWorkId],req(label+'-launch-task',1,{id:'task',title:'Prepare',deliverable:'Intent',criteria:['No provider child']}));
  const providersFile=path.join(root,'.swarm/providers.json'),providersPrior=fs.existsSync(providersFile)?fs.readFileSync(providersFile):null;
  fs.writeFileSync(providersFile,JSON.stringify({schema_version:1,providers:{fixture:{command:'/bin/cat',args:[]}}}),{mode:0o600});
  const launchFrame={schema_version:1,event_id:label+'-launch',expected_revision:2,task_id:'task',provider:'fixture',instruction:'hello'};
  async function previewLaunch(extra={}){return cli(root,['agent','preview',launchWorkId],{...launchFrame,...extra});}
  const prepared=await previewLaunch();assert.equal(prepared.preview_only,true);assert.equal(prepared.process_started,false);assert.equal(prepared.agent.timeout_seconds,17);assert.deepEqual(prepared.agent.env_allow,[]);assert.equal(Date.parse(prepared.agent.preflight.valid_until)-Date.parse(prepared.agent.preflight.checked_at),60000);
  await assert.rejects(()=>previewLaunch({timeout_seconds:19}),e=>e.code==='agent-launch-timeout-outside-saved-policy');
  // Every refusal has a valid control with these same saved policy prerequisites.
  for(const [field,value,code] of [
   ['agent_launch_instruction_max_bytes','0','agent-launch-instruction-outside-saved-policy'],
   ['agent_provider_config_max_bytes','0','agent-provider-catalogue-read'],
   ['agent_launch_context_max_bytes','0','agent-launch-context-outside-saved-policy'],
   ['agent_preflight_min_free_bytes','9223372036854775807','agent-workspace-free-space-below-policy'],
   ['agent_allow_unverified_provider_context','0','agent-provider-verified-context-required']
  ]) {
   await previewLaunch();await savePolicyField(field,value);await assert.rejects(()=>previewLaunch(),e=>e.code===code);
   await savePolicyField(field,launchChanges.find(([name])=>name===field)[1]);await previewLaunch();
  }
  fs.writeFileSync(providersFile,JSON.stringify({schema_version:1,providers:{fixture:{command:'/bin/cat',args:[],preflight_ttl_seconds:121}}}),{mode:0o600});
  await assert.rejects(()=>previewLaunch(),e=>e.code==='agent-preflight-ttl-outside-saved-policy');
  if(providersPrior)fs.writeFileSync(providersFile,providersPrior);else fs.unlinkSync(providersFile);
  await savePolicyFields(launchChanges.map(([field])=>({field,value:Array.isArray(launchSaved.values[field])?launchSaved.values[field].join('\n'):String(launchSaved.values[field]??'0'),unlimited:launchSaved.values[field]===null})));
  checks.push({label:label+'-native-launch-policy',fields:launchChanges.length,previewOnly:true,providerStarted:false});
  for(const [field,value] of [['review_reply_min_text_bytes','16'],['review_reply_max_text_bytes','20'],['review_quote_min_normalized_bytes','24']]){
   await readSavedReply(root,wid,label+'-'+field+'-control');await savePolicyField(field,value);
   await readSavedReply(root,wid,label+'-'+field+'-refused','review_fragment_protocol');
   await savePolicyField(field,'0',field==='review_reply_max_text_bytes');await readSavedReply(root,wid,label+'-'+field+'-recovered');
  }
  // Complete these edits before the history fixture starts its running task.
  /** Save an explicit phase selection in Admin and reopen the same fieldset.
   * Uses fixture CLI transport; failures reject and no HTTP authority is claimed. */
  async function savePhaseDefaults(selected){
   await page.click(open);await observed('phase-dialog',page.waitForSelector(modal));
   for(const choice of currentCatalog.fields.find(f=>f.name==='task_manual_required_gate_phases').choices){const selector=modal+' [data-setting="task_manual_required_gate_phases"] [data-phase-choice="'+choice.value+'"]';if(await page.$eval(selector,e=>e.checked)!==selected.includes(choice.value))await clickSetting(selector)}
   await page.$eval(modal+' [data-setting-reason]',e=>{e.value='Explicit manual phase defaults';e.dispatchEvent(new Event('input',{bubbles:true}))});
   await page.click(confirm);await observed('phase-preview',adminState.waitForSettingsState(page,'preview'));
   await page.click(confirm);await observed('phase-saved',page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]')));await observed('phase-reloaded',page.waitForFunction(()=>!document.querySelector('[data-action="edit"]').disabled));
   assert.deepEqual((await cli(root,['engine-settings','show'])).values.task_manual_required_gate_phases,[...selected].sort());
   const phaseField=currentCatalog.fields.find(f=>f.name==='task_manual_required_gate_phases'),translated=source=>lang==='fr'?source:english[source];const expectedText=selected.length?phaseField.choices.filter(c=>selected.includes(c.value)).map(c=>translated(c.label)).join('\n'):translated('Aucun contrôle de phase supplémentaire');assert.equal(await page.$$eval('.engine-settings-admin tr',(rows,label)=>rows.find(r=>r.querySelector('th')?.textContent===label)?.querySelector('td')?.textContent,translated(phaseField.label)),expectedText);
   await page.click(open);await observed('phase-reopened',page.waitForSelector(modal));assert.deepEqual(await page.$$eval(modal+' [data-setting="task_manual_required_gate_phases"] input:checked',items=>items.map(e=>e.value).sort()),[...selected].sort());for(const width of [1440,320]){
    await page.setViewport({width,height:1000});await page.$eval(modal+' [data-setting="task_manual_required_gate_phases"]',e=>{e.scrollIntoView({block:'center'});e.querySelector('input').focus({preventScroll:true})});
    const visible=await page.$eval(modal+' [data-setting="task_manual_required_gate_phases"]',e=>{const r=e.getBoundingClientRect(),body=e.closest('.modal-body');return {top:r.top,bottom:r.bottom,height:innerHeight,overflow:body.scrollWidth-body.clientWidth}});assert(visible.top>=0&&visible.bottom<=visible.height&&visible.overflow<=1);
    await page.screenshot({path:path.join(output,label+'-manual-phases-'+selected.length+'-'+width+'.png')});
   }await page.setViewport({width:1440,height:1000});await page.keyboard.press('Escape');await observed('phase-close',page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]')));
  }
  const phaseQuality=await cli(root,['engine-settings','quality-profile']);
  const originalBeforePhases=(await cli(root,['work','list'])).find(w=>w.id===wid);
  const phaseBase=await mutate(root,'','work.create',req('phase-work-'+label,0,{title:'Isolated phase consumer',objective:'Configured defaults',scope:'Owned fixture',criteria:['Proof']}));assert.equal(phaseBase.code,'ok');const phaseWid=phaseBase.work.id;
  const phaseWitness=await mutate(root,phaseWid,'task.add',req('phase-witness-'+label,phaseBase.work.revision,{id:'phase-before',title:'Prior contract',deliverable:'Report',criteria:['Proof']}));assert.equal(phaseWitness.code,'ok');assert.equal(Object.keys(phaseWitness.work.tasks[0].plan_checks).length,4);
  const phaseInitial=phaseWitness.work;const oldMappings=phaseInitial.tasks.map(t=>({id:t.id,checks:t.plan_checks}));
  await savePhaseDefaults(['validation']);
  const phaseRequest=req('phase-created-'+label,phaseInitial.revision,{id:'phase-selected',title:'Configured phases',deliverable:'Report',criteria:['Proof']});const phaseCreated=await mutate(root,phaseWid,'task.add',phaseRequest);assert.equal(phaseCreated.code,'ok');assert.deepEqual(phaseCreated.work.tasks.find(t=>t.id==='phase-selected').plan_checks,{'plan-validation':'validation','plan-criterion-1':'validation'});
  await savePhaseDefaults([]);const noPhases=await mutate(root,phaseWid,'task.add',req('phase-empty-'+label,phaseCreated.work.revision,{id:'phase-empty',title:'No extra phases',deliverable:'Report',criteria:['Proof']}));assert.equal(noPhases.code,'ok');assert.deepEqual(noPhases.work.tasks.find(t=>t.id==='phase-empty').plan_checks,{'plan-criterion-1':'validation'});
  await savePhaseDefaults(['entry','validation','delivery']);const renamed=await mutate(root,phaseWid,'task.update',req('phase-title-'+label,noPhases.work.revision,{id:'phase-empty',title:'Existing contract retained'}));assert.equal(renamed.code,'ok');assert.deepEqual(renamed.work.tasks.find(t=>t.id==='phase-empty').plan_checks,{'plan-criterion-1':'validation'});for(const old of oldMappings)assert.deepEqual(renamed.work.tasks.find(t=>t.id===old.id).plan_checks,old.checks);
  const afterPhases=await cli(root,['engine-settings','quality-profile']);assert(BigInt(afterPhases.settings_revision)>BigInt(phaseQuality.settings_revision));assert.deepEqual(afterPhases,{...phaseQuality,settings_revision:afterPhases.settings_revision});
  assert.deepEqual((await cli(root,['work','list'])).find(w=>w.id===wid),originalBeforePhases);
  // Actual stored row + saved policy reader; supplied review is fixture data,
  // never an authenticated producer, execution or acceptance receipt.
  if(receiptProbe){
   assert(workProbe,'receipt fixture seeding requires Work probe');
   const rw=await mutate(root,'','work.create',req('receipt-work-'+label,0,{title:'Receipt policy',objective:'Read policy evidence',scope:'Private fixture',criteria:['Proof']}));assert.equal(rw.code,'ok');
   const receiptWork=rw.work||rw;assert.equal(typeof receiptWork.id,'string');
   const receipt=Buffer.from(JSON.stringify({candidate_commit:'candidate',attempt_id:'attempt',agent_id:'producer',controls:{task:[{id:'check',executed:true,passed:true,exit_code:0,output_sha256:'hash',summary:'Declared receipt'}]}}));
   fs.writeFileSync(path.join(root,'managed.receipt'),receipt,{mode:0o600});
   const declared={...receiptWork,planning:{repository:{}},tasks:[{id:'task',independent_review:{id:'review',attempt:'attempt',producer:'producer',candidate_commit:'candidate',receipt:'managed.receipt',receipt_sha256:crypto.createHash('sha256').update(receipt).digest('hex'),started:'date'}}]};
   const seeded=await processRun(label+'-receipt-seed',[workProbe,'seed-work',root],JSON.stringify(declared));assert.equal(seeded.code,0,seeded.stderr);assert.equal(JSON.parse(seeded.stdout).fixture_data_only,true);
   /** Observe real Root receipt consumer and immutable row/settings bytes; reject
    * safe coded errors, assert meaningful control content, retain original logs. */
   async function savedReceipt(name,reason){
    const stateFile=path.join(root,'.swarm/state.db'),state=fs.readFileSync(stateFile),settings=fs.readFileSync(settingsFile);
    const result=await processRun(label+'-'+name,[receiptProbe,root],JSON.stringify({work:receiptWork.id,task:'task'}));assert.equal(result.code,reason?2:0,result.stderr);assert.equal(result.stderr.length,0);
    const value=JSON.parse(result.stdout);if(reason)assert.equal(value.reason,'review-evidence-invalid: '+reason);else {assert.equal(value.observation.work_id,receiptWork.id);assert.equal(value.observation.task_id,'task');assert.equal(value.observation.receipt.controls[0].id,'check');assert.equal(value.observation.receipt.producer_agent_id,'producer')}
    assert(fs.readFileSync(stateFile).equals(state));assert(fs.readFileSync(settingsFile).equals(settings));return value;
   }
   await savedReceipt('receipt-control');
   await savePolicyField('managed_receipt_max_bytes',String(receipt.length));await savedReceipt('receipt-exact');
   await savePolicyField('managed_receipt_max_bytes',String(receipt.length-1));await savedReceipt('receipt-oversize','artifact-byte-limit-exceeded');
   await savePolicyField('managed_receipt_max_bytes','0');await savedReceipt('receipt-zero','artifact-byte-limit-exceeded');
   await savePolicyField('managed_receipt_max_bytes','0',true);await savedReceipt('receipt-unlimited');
   fs.writeFileSync(path.join(root,'managed.receipt'),'PRIVATE corruption',{mode:0o600});await savedReceipt('receipt-corrupt','managed-receipt-digest-stale');
   fs.writeFileSync(path.join(root,'managed.receipt'),receipt,{mode:0o600});await savedReceipt('receipt-restored');
  }
  if(workProbe){
   // Declared fixture receipts only; real start/FK/UDF and actual Root consumer.
   const frame=JSON.stringify({work:wid,language:lang});
   const seeded=await processRun(label+'-seed-history',[workProbe,'seed',root],frame);assert.equal(seeded.code,0);assert.equal(JSON.parse(seeded.stdout).fixture_data_only,true);
   async function savedProjection(name,mode,request){
    const before=fs.readFileSync(settingsFile),r=await processRun(label+'-'+name,[workProbe,mode,root],JSON.stringify(request));assert.equal(r.code,0,r.stderr);assert.equal(r.stderr.length,0);assert(fs.readFileSync(settingsFile).equals(before));return JSON.parse(r.stdout);
   }
   async function longest(name,count){const v=(await savedProjection(name,'root',{work:wid,language:lang})).observation.validation;assert.equal(v.tasks.task.evidence.controls.longest_measured.length,count);assert.equal(v.tasks.task.evidence.controls.history.length,6);assert.equal(v.validated,0)}
   await longest('longest-control',5);await savePolicyField('evidence_longest_measured_count','1');await longest('longest-one',1);
   await savePolicyField('evidence_longest_measured_count','0');await longest('longest-zero',0);
   await savePolicyField('evidence_longest_measured_count','0',true);await longest('longest-all',6);
   const decisions={events:['ooda','other','ooda','ooda']};
   assert.deepEqual((await savedProjection('decisions-control','decisions',decisions)).revisions,[0,2,3]);
   await savePolicyField('work_view_recent_decisions','1');assert.deepEqual((await savedProjection('decisions-one','decisions',decisions)).revisions,[3]);
   await savePolicyField('work_view_recent_decisions','0');assert.deepEqual((await savedProjection('decisions-zero','decisions',decisions)).revisions,[]);
   await savePolicyField('work_view_recent_decisions','0',true);assert.deepEqual((await savedProjection('decisions-all','decisions',decisions)).revisions,[0,2,3]);
   const text={text:'abc defghij'};assert.equal((await savedProjection('wrap-control','text',text)).text,text.text);
   await savePolicyField('work_view_wrap_characters','3');assert.equal((await savedProjection('wrap-three','text',text)).text,'abc\ndef\nghi\nj');
   await savePolicyField('work_view_wrap_characters','0',true);assert.equal((await savedProjection('wrap-unlimited','text',text)).text,text.text);
   await page.click(open);await observed('projection-capture',page.waitForSelector(modal));
   await page.$eval(modal+' [data-unlimited="work_view_wrap_characters"]',e=>{e.scrollIntoView({block:'center'});e.focus()});
   await page.screenshot({path:path.join(output,label+'-work-presentation.png'),fullPage:true});
   await page.keyboard.press('Escape');await observed('projection-capture-closed',page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]')));
  }
  // The existing Admin criterion-count policy must reach actual creation,
  // including null -> finite -> null; no direct journal edit or extra ceiling.
  await savePolicyField('task_max_criteria','2');
  let creationWork=(await cli(root,['work','list'])).find(w=>w.id===wid);assert(creationWork);
  const controlCreation=req('criteria-control-'+label,creationWork.revision,{id:'criteria-control',title:'Criterion control',deliverable:'Report',criteria:['A','B']});
  const createdControl=await mutate(root,wid,'task.add',controlCreation);assert.equal(createdControl.code,'ok');
  const rejectedCreation=req('criteria-three-'+label,createdControl.work.revision,{id:'criteria-three',title:'Criterion policy',deliverable:'Report',criteria:['A','B','C']});
  const priorCreationDB=fs.readFileSync(path.join(root,'.swarm/state.db')),priorCreationPolicy=fs.readFileSync(settingsFile);
  assert.equal((await mutate(root,wid,'task.add',rejectedCreation)).code,'format');assert(fs.readFileSync(path.join(root,'.swarm/state.db')).equals(priorCreationDB));assert(fs.readFileSync(settingsFile).equals(priorCreationPolicy));
  await savePolicyField('task_max_criteria','3');const recoveredCreation=await mutate(root,wid,'task.add',rejectedCreation);assert.equal(recoveredCreation.code,'ok');assert.equal(recoveredCreation.work.revision,createdControl.work.revision+1);
  assert.equal((await mutate(root,wid,'task.add',rejectedCreation)).replayed,true);
  await savePolicyField('task_max_criteria','0',true);
  const unlimitedCreation=req('criteria-unlimited-'+label,recoveredCreation.work.revision,{id:'criteria-unlimited',title:'Unlimited criteria',deliverable:'Report',criteria:Array.from({length:13},(_,i)=>'Criterion '+(i+1))});
  const unlimitedCreated=await mutate(root,wid,'task.add',unlimitedCreation);assert.equal(unlimitedCreated.code,'ok');assert.equal(Object.keys(unlimitedCreated.work.tasks.find(t=>t.id==='criteria-unlimited').plan_checks).length,16);
  const replyQualityAfter=await cli(root,['engine-settings','quality-profile']);assert.equal(replyQualityAfter.digest,replyQualityBefore.digest);assert.equal(replyQualityAfter.generation,replyQualityBefore.generation);assert.deepEqual(replyQualityAfter.values,replyQualityBefore.values);assert.notEqual(replyQualityAfter.settings_revision,replyQualityBefore.settings_revision);
  // New resume policy: real saved Admin values reach the production CLI.
  // Only the marked private fixture is seeded; no live database or actor exists.
  if(workProbe){
   const planningWork={schema_version:1,id:'planning-policy',revision:1,criteria:['Proof'],planning:{version:1,paused:false,failure:'Prior failure',max_tasks:10,max_decisions:10,max_activations:15,scopes:[{id:'root',objective:'Fixture',requirements:['req-1'],revision:1,state:'blocked',generation:2,holder:'fixture',lease_until:'2099-01-01T00:00:00Z'}]}};
   const seeded=await processRun(label+'-planning-seed',[workProbe,'seed-work',root],JSON.stringify(planningWork));assert.equal(seeded.code,0);
   const control=await cli(root,['planning','resume','planning-policy'],req('planning-control',1,{reason:'Valid'}));assert.equal(control.revision,2);assert.equal(control.planning.scopes[0].generation,3);
   await savePolicyField('planning_resume_max_reason_bytes','0');
   const rejected=req('planning-policy-recovery',2,{reason:'Reprise é'}),beforeDB=fs.readFileSync(path.join(root,'.swarm/state.db')),beforePolicy=fs.readFileSync(settingsFile);
   await assert.rejects(cli(root,['planning','resume','planning-policy'],rejected),{code:'planning-control-invalid'});
   assert(fs.readFileSync(path.join(root,'.swarm/state.db')).equals(beforeDB));assert(fs.readFileSync(settingsFile).equals(beforePolicy));
   await savePolicyField('planning_resume_max_reason_bytes','0',true);
   const resumed=await cli(root,['planning','resume','planning-policy'],rejected);assert.equal(resumed.revision,3);assert.equal(resumed.planning.scopes[0].generation,4);assert.equal(resumed.planning.inbox.at(-1).message,'Reprise é');
   const replay=await cli(root,['planning','resume','planning-policy'],rejected);assert.deepEqual(replay,resumed);
   await savePolicyField('planning_resume_max_reason_bytes','4');
   const exact=await cli(root,['planning','resume','planning-policy'],req('planning-exact',3,{reason:'éé'}));assert.equal(exact.revision,4);
   const exactDB=fs.readFileSync(path.join(root,'.swarm/state.db'));await assert.rejects(cli(root,['planning','resume','planning-policy'],req('planning-oversize',4,{reason:'ééx'})),{code:'planning-control-invalid'});assert(fs.readFileSync(path.join(root,'.swarm/state.db')).equals(exactDB));
   await savePolicyField('planning_resume_max_reason_bytes','0',true);
   const pausedRequest=req('canonical-pause',4,{reason:'é'});
   await savePolicyField('planning_request_max_bytes','0');
   const zeroDB=fs.readFileSync(path.join(root,'.swarm/state.db'));
   await assert.rejects(cli(root,['planning','pause','planning-policy'],pausedRequest),{code:'planning-control-invalid'});assert(fs.readFileSync(path.join(root,'.swarm/state.db')).equals(zeroDB));
   const canonicalBytes=Buffer.byteLength(JSON.stringify(pausedRequest));
   await savePolicyField('planning_request_max_bytes',String(canonicalBytes-1));
   await assert.rejects(cli(root,['planning','pause','planning-policy'],pausedRequest),{code:'planning-control-invalid'});assert(fs.readFileSync(path.join(root,'.swarm/state.db')).equals(zeroDB));
   await savePolicyField('planning_request_max_bytes',String(canonicalBytes));
   const paused=await cli(root,['planning','pause','planning-policy'],pausedRequest);assert.equal(paused.revision,5);
   await savePolicyField('planning_request_max_bytes','0');const pausedDB=fs.readFileSync(path.join(root,'.swarm/state.db'));assert.deepEqual(await cli(root,['planning','pause','planning-policy'],pausedRequest),paused);assert(fs.readFileSync(path.join(root,'.swarm/state.db')).equals(pausedDB));
   await savePolicyField('planning_request_max_bytes','0',true);
   // Real public creation and local enablement, without fixture hierarchy seeding.
   const makeWork=async name=>{const result=await mutate(root,'','work.create',req(name,0,{title:'Planning enable',objective:'Two requirements',scope:'Owned fixture',criteria:['A','B']}));assert.equal(result.code,'ok');return result.work};
   const enableRequest=event=>req(event,1,{max_tasks:10,max_decisions:12,max_activations:15});
   const witness=await makeWork('enable-witness');assert.equal((await cli(root,['planning','enable',witness.id],enableRequest('enable-witness-start'))).revision,2);
   const fresh=await makeWork('enable-policy');await savePolicyField('planning_max_requirements','0');
   const noEnableDB=fs.readFileSync(path.join(root,'.swarm/state.db'));await assert.rejects(cli(root,['planning','enable',fresh.id],enableRequest('enable-policy-start')),{code:'planning-control-invalid'});assert(fs.readFileSync(path.join(root,'.swarm/state.db')).equals(noEnableDB));
   await savePolicyField('planning_max_requirements','1');await assert.rejects(cli(root,['planning','enable',fresh.id],enableRequest('enable-policy-start')),{code:'planning-control-invalid'});assert(fs.readFileSync(path.join(root,'.swarm/state.db')).equals(noEnableDB));
   await savePolicyField('planning_max_requirements','2');const enabled=await cli(root,['planning','enable',fresh.id],enableRequest('enable-policy-start'));assert.equal(enabled.revision,2);assert.deepEqual(enabled.planning.scopes[0].requirements,['req-1','req-2']);
   await savePolicyField('planning_max_requirements','0',true);const unlimited=await makeWork('enable-unlimited');assert.equal((await cli(root,['planning','enable',unlimited.id],enableRequest('enable-unlimited-start'))).revision,2);
  }
  checks.push({lang,theme,saved_runtime_defaults_and_ceilings_reach_production_cli:true,runtime_all_ten_fields_saved_reopened:true,runtime_ceiling_null_refusal_and_recovery:true,saved_admin_canonical_request_policy_reaches_production_cli:!!workProbe,saved_admin_enable_requirement_policy_reaches_production_cli:!!workProbe,saved_admin_resume_reason_policy_reaches_production_cli:!!workProbe,resume_zero_null_utf8_exact_refusal_replay:!!workProbe,saved_admin_receipt_cap_reaches_actual_stored_reader:!!receiptProbe,receipt_exact_zero_null_refusal_and_recovery:!!receiptProbe,saved_admin_manual_phases_change_new_tasks:true,manual_phases_empty_and_existing_contract_preserved:true,malformed_phase_transport_refused_without_mutation:true,legacy_manual_phase_adoption:legacy,saved_admin_criterion_limit_changes_creation:true,criterion_creation_finite_null_and_recovery:true,saved_admin_work_projection_policies:!!workProbe,work_history_is_declared_fixture_data:!!workProbe,saved_admin_reply_policies_change_stored_task_parser:true,reply_three_fields_saved_reopened:true,reply_data_not_authenticated_verdict:true,saved_admin_baseline_depth_changes_real_guard:true,baseline_zero_null_saved_reopened:true,baseline_consumer_supplied_review_files_not_identity:true,saved_admin_git_deadline_changes_real_process:true,git_zero_null_saved_reopened:true,legacy_git_requires_adoption:true,saved_admin_review_cap_changes_real_reader:true,saved_admin_context_changes_real_reader:true,context_cap_null_and_legacy_adoption:true,saved_admin_quality_changes_real_calculation:true,legacy_quality_requires_explicit_adoption:true,quality_profile_readonly:true,quality_band_refused_without_mutation:true,quality_change_and_revert_generations:true,fields:Object.keys(published.values).length,legacy_upgraded:legacy,old_history_preserved:true,retention_saved:retention,exact_large_integer:true,preview_no_write:true,lost_response_replay_once:true,save_reopen_actual_consumer:true,zero_refused_without_mutation:true,unlimited_saved_reopened:true,stale_editor_refused_without_mutation:true,translated_errors:true,program_lines_not_split_on_spaces:true,transport_bound_visible_readonly:true,loading_error_recovery:true,keyboard_escape_focus:true,title_and_primary_action_visible:true,widths:[1440,1024,760,390,320]});await page.close();
 }
 assert.deepEqual(snapshot(),before,'Source or binary changed during browser checks');assert.equal(errors.length,0);const verifiedRuntime=verificationRuntime.finish(runtime,repo);
 fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:'PASS',scope:'Shared Admin candidate; real Rust CLI, CDP fixture transport; no native HTTP/session acceptance',checks,errors,inputs:before,scratch,verification_runtime:verifiedRuntime,global_watchdog:null,native_server:false,admin_installed:false},null,2)+'\n');console.log('PASS shared Admin candidate / native server integration remains open');
 }catch(error){fs.writeFileSync(path.join(output,'failure.json'),JSON.stringify({status:'FAIL',message:error.stack,checks,errors,scratch},null,2)+'\n');throw error}
 finally{if(browser)await browser.close();if(server)await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1});
