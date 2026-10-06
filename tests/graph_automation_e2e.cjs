'use strict';
// D01 product receipt.  It composes the native graph and Programs journeys on
// isolated roots, adds a public-CLI rejection/replay probe, and records which
// engine D01 test supplies each non-browser guarantee.  No provider is started.
const assert=require('node:assert/strict');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto');
const {spawn,spawnSync}=require('node:child_process');
if(!process.argv[2]||!process.argv[3])throw Error('usage: node tests/graph_automation_e2e.cjs BINARY OUTPUT');
const binary=path.resolve(process.argv[2]),output=path.resolve(process.argv[3]);
fs.mkdirSync(output,{recursive:true});
const scratch=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-d01-'));
const graphOutput=path.join(output,'graph'),automationOutput=path.join(output,'programs');

function executeAsync(command,args,options={}){
 return new Promise((resolve,reject)=>{
  const child=spawn(command,args,{env:options.env,stdio:['ignore','pipe','pipe']});let stdout='',stderr='',settled=false;
  const timer=setTimeout(()=>{if(!settled){child.kill('SIGTERM');reject(Error(`${command} ${args.join(' ')} timed out`))}},options.timeout||110000);
  child.stdout.on('data',chunk=>stdout+=chunk);child.stderr.on('data',chunk=>stderr+=chunk);
  child.on('error',error=>{if(!settled){settled=true;clearTimeout(timer);reject(error)}});
  child.on('close',status=>{if(settled)return;settled=true;clearTimeout(timer);if(status!==0)reject(Error(`${command} ${args.join(' ')} failed (${status}):\n${stdout}\n${stderr}`));else resolve({stdout,stderr,status})});
 });
}
function publicCLI(root,args,input,expected=0){
 const result=spawnSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',input:input?JSON.stringify(input):undefined,maxBuffer:16*1024*1024,timeout:15000});
 if(result.error)throw Error(`CLI ${args.join(' ')} did not complete: ${result.error.message}`);
 assert.equal(result.status,expected,`CLI ${args.join(' ')}: ${result.stderr}`);
 const text=(expected===0?result.stdout:result.stderr).trim();
 return text?JSON.parse(text):{};
}
function mutation(root,args,revision,body){
 return publicCLI(root,args,{schema_version:1,event_id:crypto.randomUUID(),expected_revision:revision,...body}).work;
}
function engineProbe(){
 const pattern='^(TestGraphDeliveryD01WorkspaceWait|TestGraphDeliveryD01RestartRetention|TestGraphDeliveryD01ProofFreshness)$';
 const parent=path.dirname(output);
 const receipt=fs.readdirSync(parent).filter(name=>name.endsWith('.json')).map(name=>({name,value:JSON.parse(fs.readFileSync(path.join(parent,name),'utf8'))})).find(row=>row.value.command?.[0]==='go'&&row.value.command.includes('-race')&&row.value.command.some(arg=>arg.startsWith('^TestGraphDeliveryD01')));
 assert.ok(receipt,'supervisor D01 race receipt missing');assert.equal(receipt.value.exit_code,0,'supervisor D01 race failed');
 const log=fs.readFileSync(receipt.value.log,'utf8'),digest=crypto.createHash('sha256').update(log).digest('hex');assert.equal(digest,receipt.value.sha256,'supervisor D01 race log digest mismatch');
 const tests=log.split('\n').filter(line=>line.startsWith('{')).map(line=>JSON.parse(line)).filter(event=>event.Action==='pass'&&event.Test&&new RegExp(pattern).test(event.Test)).map(event=>event.Test);
 const expected=['TestGraphDeliveryD01WorkspaceWait','TestGraphDeliveryD01RestartRetention','TestGraphDeliveryD01ProofFreshness'];
 assert.deepEqual(tests.sort(),expected.sort());
 return {tests,command:receipt.value.command,receipt:receipt.name,log_sha256:digest};
}
function cliProbe(){
 const root=path.join(scratch,'cli');fs.mkdirSync(root,{recursive:true});publicCLI(root,['init']);
 let work=mutation(root,['work','create'],0,{title:'D01 public probe',objective:'Traverse product boundaries',scope:'isolated fixture',criteria:['observable effects']});
 work=mutation(root,['task','add',work.id],work.revision,{id:'prepare',title:'Prepare',owner:'planner',deliverable:'docs/prepare.md',criteria:['roles visible'],next:'prepare'});
 work=mutation(root,['task','add',work.id],work.revision,{id:'execute',title:'Execute',owner:'worker',deliverable:'docs/execute.md',criteria:['effect visible'],depends:['prepare'],next:'wait'});
 const shown=publicCLI(root,['work','show',work.id]).work;
 assert.deepEqual(shown.tasks.map(task=>task.owner),['planner','worker']);
 const cycle=publicCLI(root,['plan','draft','import',work.id],{schema_version:1,work_id:work.id,expected_revision:work.revision,operations:[{kind:'add_dependency',prerequisite:'execute',dependent:'prepare'}]});
 const rejected=publicCLI(root,['plan','draft','preview',work.id],{schema_version:1,work_id:work.id,draft_id:cycle.draft_id,expected_revision:work.revision},2);
 assert.equal(rejected.failure.code,'dependency_cycle');
 const afterCycle=publicCLI(root,['work','show',work.id]).work;assert.equal(afterCycle.revision,work.revision);assert.deepEqual(afterCycle.tasks.find(task=>task.id==='execute').depends,['prepare']);
 work=mutation(root,['task','add',work.id],work.revision,{id:'review',title:'Review',owner:'reviewer',deliverable:'docs/review.md',criteria:['independent'],next:'review'});
 const draft=publicCLI(root,['plan','draft','import',work.id],{schema_version:1,work_id:work.id,expected_revision:work.revision,operations:[{kind:'add_dependency',prerequisite:'execute',dependent:'review'}]});
 const preview=publicCLI(root,['plan','draft','preview',work.id],{schema_version:1,work_id:work.id,draft_id:draft.draft_id,expected_revision:work.revision});
 const apply={schema_version:1,work_id:work.id,draft_id:draft.draft_id,event_id:'d01-public-replay',expected_revision:work.revision,preview_token:preview.preview_token,content_digest:preview.content_digest};
 const first=publicCLI(root,['plan','draft','apply',work.id],apply),again=publicCLI(root,['plan','draft','apply',work.id],apply);
 assert.equal(again.revision,first.revision);assert.equal(again.applied_at,first.applied_at);
 const conflicted=publicCLI(root,['plan','draft','apply',work.id],{...apply,content_digest:'different-content'},3);assert.equal(conflicted.failure.code,'event_conflict');
 const finalWork=publicCLI(root,['work','show',work.id]).work;assert.equal(finalWork.revision,first.revision);assert.ok(finalWork.tasks.find(task=>task.id==='review').depends.includes('execute'));
 return {root_kind:'temporary',work_id:work.id,prepare_roles:{owners:shown.tasks.map(task=>task.owner),agent_count:publicCLI(root,['agent','list',work.id]).agents.length},cycle_rejected:{code:rejected.failure.code,revision_preserved:afterCycle.revision},dependency_apply:{revision:first.revision,dependency:'execute -> review'},content_conflict:{code:conflicted.failure.code,replay_revision:again.revision,rows_preserved:true}};
}

async function graphJourney(){
 let source=fs.readFileSync(path.join('tests','graph_draft_ui.cjs'),'utf8');
 const loads='const load_measurements=[];for(const cards of [50,200,500])load_measurements.push(await loadMeasurement(browser,cards));';
 assert.equal(source.split(loads).length,2,'graph recipe load anchor changed');source=source.replace(loads,'const load_measurements=[];');
 const anchor="  const activityScreenshot=name+'-attempt.png';";assert.equal(source.split(anchor).length,2,'graph recipe journal anchor changed');
 const journal=String.raw`
  await page.locator('[data-mission-action="journal"]').click();
  await page.waitForFunction(()=>document.querySelector('#fil-bloc')?.open&&document.querySelectorAll('#fil-entrees .fil-entree').length);
  const d01Journal=await page.evaluate(()=>[...document.querySelectorAll('#fil-entrees .fil-entree')].map(row=>({origin:row.dataset.origine,actor:row.querySelector('.fil-origine')?.textContent,action:row.querySelector('.fil-label')?.textContent,reason:row.querySelector('.fil-message')?.textContent,visible:!!row.getClientRects().length})));
  assert.ok(d01Journal.some(row=>row.visible&&row.actor&&row.action&&row.reason&&/t000|Tâche 0/.test(row.reason)),'selected agent activity missing from journal');
  fs.writeFileSync(path.join(outDir,name+'-d01-journal.json'),JSON.stringify(d01Journal,null,2));
`;
 source=source.replace(anchor,journal+'\n'+anchor);
 const generated=path.join(scratch,'graph-d01.cjs');fs.writeFileSync(generated,source);
 const result=await executeAsync(process.execPath,[generated,binary,graphOutput],{env:{...process.env,PUPPETEER_MODULE:require.resolve('puppeteer')},timeout:110000});
 fs.writeFileSync(path.join(output,'graph-run.log'),result.stdout+result.stderr);
 return JSON.parse(fs.readFileSync(path.join(graphOutput,'results.json')));
}
async function programsJourney(){
 const result=await executeAsync(process.execPath,[path.join('tests','automation_ui.cjs'),binary,automationOutput],{env:{...process.env,PUPPETEER_MODULE:require.resolve('puppeteer')},timeout:110000});
 fs.writeFileSync(path.join(output,'programs-run.log'),result.stdout+result.stderr);
 return JSON.parse(fs.readFileSync(path.join(automationOutput,'results.json')));
}

(async()=>{try{
 const engine=engineProbe(),cli=cliProbe(),[graph,programs]=await Promise.all([graphJourney(),programsJourney()]);
 const names=['fr-sombre','fr-etat','en-sombre','en-etat'];
 assert.deepEqual(graph.variants.map(row=>row.variant),names);assert.deepEqual(programs.variants.map(row=>row.variant),names);
 const variants=names.map(name=>{
  const graphRow=graph.variants.find(row=>row.variant===name),programRow=programs.variants.find(row=>row.variant===name);
  const journal=JSON.parse(fs.readFileSync(path.join(graphOutput,name+'-d01-journal.json')));
  assert.ok(journal.some(row=>row.visible&&row.actor&&row.action&&row.reason));
  const screenshot=name+'.png';fs.copyFileSync(path.join(automationOutput,programRow.screenshot),path.join(output,screenshot));
  const assertions={
   prepare_roles:cli.prepare_roles.agent_count===0,
   dependency_apply:graphRow.assertions.preview_apply&&cli.dependency_apply.revision>0,
   cycle_rejected:cli.cycle_rejected.code==='dependency_cycle',
   agent_journal:journal.some(row=>row.visible&&row.actor&&row.action&&row.reason),
   program_replay:programRow.assertions.save_reload,
   content_conflict:graphRow.assertions.conflict&&cli.content_conflict.code==='event_conflict',
   workspace_wait:engine.tests.includes('TestGraphDeliveryD01WorkspaceWait'),
   restart_retention:engine.tests.includes('TestGraphDeliveryD01RestartRetention')&&programRow.assertions.save_reload,
   causal_recovery:engine.tests.includes('TestGraphDeliveryD01ProofFreshness'),
   unknown_cost:graphRow.assertions.unknown_cost_projected&&programRow.assertions.unknown_cost,
   focus_restore:graphRow.assertions.focus_restore&&programRow.assertions.focus_restore,
   graph_preserved:programRow.assertions.graph_preserved
  };
  assert.ok(Object.values(assertions).every(Boolean),`${name}: incomplete D01 assertions`);
  return {variant:name,assertions,screenshot,console_errors:[...graphRow.console_errors,...programRow.console_errors],network_errors:[...graphRow.network_errors,...programRow.network_errors],evidence:{prepare_roles:cli.prepare_roles,dependency_apply:cli.dependency_apply,cycle_rejected:cli.cycle_rejected,agent_journal:{rows:journal.filter(row=>row.visible).length,path:`graph/${name}-d01-journal.json`},program_replay:{schedule_id:programRow.schedule_id,save_reload:programRow.assertions.save_reload},content_conflict:cli.content_conflict,workspace_wait:{engine_test:'TestGraphDeliveryD01WorkspaceWait',observed:engine.tests.includes('TestGraphDeliveryD01WorkspaceWait')},restart_retention:{engine_test:'TestGraphDeliveryD01RestartRetention',observed:engine.tests.includes('TestGraphDeliveryD01RestartRetention'),program_reload:programRow.assertions.save_reload},causal_recovery:{engine_test:'TestGraphDeliveryD01ProofFreshness',observed:engine.tests.includes('TestGraphDeliveryD01ProofFreshness')},unknown_cost:{graph:graphRow.assertions.unknown_cost_projected,program:programRow.assertions.unknown_cost},focus_restore:{graph:graphRow.assertions.focus_restore,program:programRow.assertions.focus_restore},graph_preserved:{program:programRow.assertions.graph_preserved}}};
 });
 const receipt={schema_version:1,surface:'swarm-product',environment:{platform:`${os.platform()} ${os.release()} ${os.arch()}`,binary,browser:programs.environment.browser,isolated_roots:true,provider_started:false},engine_preconditions:engine,cli_probe:cli,variants,limits:['fixture provider only; no real provider autonomy','workspace wait and causal recovery are engine assertions re-executed in this combined receipt, not browser-triggerable operations']};
 fs.writeFileSync(path.join(output,'results.json'),JSON.stringify(receipt,null,2));console.log(JSON.stringify(receipt,null,2));
}catch(error){fs.writeFileSync(path.join(output,'failure.json'),JSON.stringify({error:error.stack,scratch},null,2));console.error(error);process.exitCode=1}})();
