'use strict';
// B03 product journey: real candidate binary, isolated roots, HTTP service and
// Chromium. It never opens the store directly and never starts a provider.
const assert=require('node:assert/strict');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto');
const {spawn,execFileSync}=require('node:child_process');
const puppeteer=require(process.env.PUPPETEER_MODULE||'puppeteer');
const binary=path.resolve(process.argv[2]),outDir=path.resolve(process.argv[3]);
if(!process.argv[2]||!process.argv[3])throw Error('usage: node tests/graph_draft_ui.cjs BINAIRE DOSSIER_SORTIE');
fs.mkdirSync(outDir,{recursive:true});
const temporary=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-b03-')),uuid=()=>crypto.randomUUID().replaceAll('-','');
const cli=(root,args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',maxBuffer:16*1024*1024,input:input?JSON.stringify(input):undefined}));
const mutate=(root,args,revision,fields)=>cli(root,args,{schema_version:1,event_id:uuid(),expected_revision:revision,...fields}).work;
function fixture(root,count=6){
 fs.mkdirSync(root,{recursive:true});cli(root,['init']);let work=mutate(root,['work','create'],0,{title:'Graphe B03',objective:'Éditer les dépendances dans le produit',scope:'recette isolée',criteria:['édition vérifiée']});
 for(let index=0;index<count;index++){const id='t'+String(index).padStart(3,'0'),depends=index?[index<6?'t'+String(index-1).padStart(3,'0'):'t000']:[];work=mutate(root,['task','add',work.id],work.revision,{id,title:'Tâche '+index,deliverable:'docs/'+id+'.md',criteria:['effet observable'],owner:index%3===0?'planner':'worker',next:'préparer',depends})}
 return work;
}
function start(root){
 const child=spawn(binary,['--root',root,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
 return new Promise((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(Error('serveur non démarré: '+output)),15000);const read=data=>{output+=data;const match=output.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(match){clearTimeout(timer);resolve({child,url:match[0]})}};child.stdout.on('data',read);child.stderr.on('data',data=>output+=data);child.on('exit',code=>reject(Error('serveur arrêté '+code+': '+output)))})
}
const waitForRevision=(page,revision)=>page.waitForFunction(value=>snapshot?.work?.revision>value,{},revision);
const p95=values=>values.slice().sort((a,b)=>a-b)[Math.ceil(values.length*.95)-1];
async function choose(page,selector,value){await page.$eval(selector,e=>{for(let a=e.parentElement;a;a=a.parentElement)if(a.tagName==='DETAILS')a.open=true});const index=await page.$eval(selector,(element,wanted)=>[...element.options].findIndex(option=>option.value===wanted),value);assert.ok(index>=0,`option ${value} absente`);await page.focus(selector);await page.keyboard.press('Home');for(let step=0;step<index;step++)await page.keyboard.press('ArrowDown');await page.keyboard.press('Enter')}
async function openProduct(browser,root,work,lang,theme,diagnostics){
 const server=await start(root),page=await browser.newPage();page.on('close',()=>server.child.kill('SIGTERM'));page.setDefaultTimeout(25000);
 await page.setViewport({width:1440,height:1000});
 page.on('pageerror',error=>diagnostics.console_errors.push(String(error)));
 page.on('console',message=>{
  if(message.type()!=='error')return;
  const text=message.text(),status=text.match(/^Failed to load resource: the server responded with a status of (\d+) /),url=message.location().url;
  if(status&&url&&diagnostics.expected_http.has(status[1]+':'+new URL(url).pathname)){(diagnostics.expected_console_errors??=[]).push(text);return}
  diagnostics.console_errors.push(text);
 });
 diagnostics.inflightRequests=new Set();diagnostics.reloadAbortRequests=new Set();diagnostics.expected_network_aborts=[];
 page.on('request',request=>diagnostics.inflightRequests.add(request));
 page.on('requestfinished',request=>diagnostics.inflightRequests.delete(request));
 page.on('requestfailed',request=>{
  diagnostics.inflightRequests.delete(request);
  if(request.failure()?.errorText==='net::ERR_ABORTED'&&diagnostics.reloadAbortRequests.has(request)){diagnostics.expected_network_aborts.push(new URL(request.url()).pathname);return}
  diagnostics.network_errors.push('failed '+new URL(request.url()).pathname+' '+request.failure()?.errorText);
 });
 page.on('response',response=>{if(response.status()>=400&&!diagnostics.expected_http.has(response.status()+':'+new URL(response.url()).pathname))diagnostics.network_errors.push(response.status()+' '+response.url())});
 const target=new URL(server.url);target.searchParams.set('work',work.id);target.searchParams.set('lang',lang);const firstOpenStarted=Date.now();await page.goto(target.href,{waitUntil:'domcontentloaded'});await page.waitForFunction(id=>snapshot?.work?.id===id&&document.querySelectorAll('.graph-noeud').length>0,{},work.id);const firstOpenMS=Date.now()-firstOpenStarted;
 if(await page.$eval('html',element=>element.dataset.theme)!==theme){await page.$eval('#theme',e=>e.closest('details').open=true);await page.click('#theme');}assert.equal(await page.$eval('html',element=>element.dataset.theme),theme);
 return {server,page,firstOpenMS};
}
async function selectNode(page,id,key='Enter'){
 const selector=`.graph-noeud[data-task="${id}"]`;
 await page.$eval(selector,element=>element.focus());
 assert.equal(await page.$eval(selector,element=>document.activeElement===element),true,'SVG node did not receive keyboard focus');
 await page.keyboard.press(key);
}
async function variant(browser,base,work,name,lang,theme){
 const root=path.join(temporary,'variant-'+name);fs.cpSync(base,root,{recursive:true});const diagnostics={console_errors:[],network_errors:[],expected_http:new Set()};let server,page;
 try{
  ({server,page}=await openProduct(browser,root,work,lang,theme,diagnostics));
  const before=cli(root,['work','show',work.id]).work,agentsBefore=cli(root,['agent','list',work.id]).agents.length;
  assert.ok(await page.$$eval('.graph-arete',nodes=>nodes.length)>=5,'flèches initiales absentes');
  await choose(page,'#pilot-orientation','TB');await page.waitForFunction(()=>document.querySelector('.graph-svg')?.dataset.height);assert.ok(await page.$$eval('.graph-arete',nodes=>nodes.length)>=5,'flèches perdues après orientation');
  await page.click('#pilot-collapse');await page.waitForFunction(()=>document.querySelector('#pilot-status').textContent.includes('masqué')||document.querySelector('#pilot-status').textContent.includes('hidden'));assert.match(await page.$eval('#pilot-status',node=>node.textContent),/masqué|hidden/);await page.click('#pilot-expand');
  assert.ok(await page.$('#graph-minimap'),'mini-carte absente');
  await page.click('#pilot-zoom-in');await page.click('#pilot-zoom-in');const canvasBefore=await page.$eval('#pilot-canvas',node=>{node.scrollLeft=35;node.scrollTop=45;node.dispatchEvent(new Event('scroll'));return {x:node.scrollLeft,y:node.scrollTop}});
  await page.focus('#pilot-edit-graph');await page.keyboard.press('Enter');await page.waitForSelector('#graph-draft-panel:not([hidden])');
  await selectNode(page,'t001');await selectNode(page,'t003','Space');assert.equal(await page.$eval('#graph-draft-stage',node=>node.disabled),false);await page.keyboard.press('Enter');await page.waitForFunction(()=>document.querySelectorAll('#graph-draft-operations li').length===1&&!document.querySelector('#graph-draft-preview').disabled);
  await page.click('#graph-draft-undo');await page.waitForFunction(()=>document.querySelector('#graph-draft-status').textContent.startsWith('0'));assert.equal(await page.$eval('#graph-draft-error',node=>node.hidden),true,'undo empty proposal failed');assert.equal(await page.$eval('#graph-draft-apply',node=>node.disabled),true,'undo retained an applicable preview');assert.equal(cli(root,['work','show',work.id]).work.revision,before.revision,'undo changed the applied plan');await page.click('#graph-draft-redo');await page.waitForFunction(()=>!document.querySelector('#graph-draft-preview').disabled);
  await page.click('#graph-draft-preview');await page.waitForFunction(()=>!document.querySelector('#graph-draft-apply').disabled);assert.match(await page.$eval('#graph-draft-status',node=>node.textContent),/Aucun départ implicite|No implicit start/);
  const firstViewport=await page.$eval('#pilot-canvas',node=>({x:node.scrollLeft,y:node.scrollTop}));
  const revision=before.revision;await page.click('#graph-draft-apply');await waitForRevision(page,revision);await page.waitForFunction(()=>[...document.querySelectorAll('.graph-arete')].some(edge=>edge.dataset.from==='t001'&&edge.dataset.to==='t003'));
  const firstAfter=await page.$eval('#pilot-canvas',node=>({x:node.scrollLeft,y:node.scrollTop}));assert.ok(Math.abs(firstAfter.x-firstViewport.x)<=2&&Math.abs(firstAfter.y-firstViewport.y)<=2,'apply moved the graph viewport');
  const applied=cli(root,['work','show',work.id]).work;assert.ok(applied.tasks.find(task=>task.id==='t003').depends.includes('t001'));assert.equal(cli(root,['agent','list',work.id]).agents.length,agentsBefore,'départ implicite');
  // Create a stale preview with a public concurrent CLI mutation, then recover
  // without discarding the browser operations.
  await page.click('.graph-noeud[data-task="t000"] .graph-titre');await page.click('.graph-noeud[data-task="t004"] .graph-titre');assert.equal(await page.$eval('#graph-draft-stage',node=>node.disabled),false,'mouse selection did not choose two tasks');await page.click('#graph-draft-stage');await page.waitForFunction(()=>!document.querySelector('#graph-draft-preview').disabled);
  const current=cli(root,['work','show',work.id]).work;mutate(root,['task','add',work.id],current.revision,{id:'concurrent',title:'Ajout concurrent',deliverable:'docs/concurrent.md',criteria:['conservé'],owner:'worker',next:'attendre'});
  diagnostics.expected_http.add('409:/api/v1/graph-drafts/preview');await page.click('#graph-draft-preview');await page.waitForSelector('#graph-draft-error:not([hidden])');assert.match(await page.$eval('#graph-draft-error',node=>node.textContent),/conserv|preserv/);assert.equal(await page.$$eval('#graph-draft-operations li',nodes=>nodes.length),1);
  await page.click('#graph-draft-recover');await page.waitForFunction(()=>!document.querySelector('#graph-draft-preview').disabled&&document.querySelector('#graph-draft-error').hidden);await page.click('#graph-draft-preview');await page.waitForFunction(()=>!document.querySelector('#graph-draft-apply').disabled);
  const recoveredViewport=await page.$eval('#pilot-canvas',node=>({x:node.scrollLeft,y:node.scrollTop}));
  const recoveredRevision=cli(root,['work','show',work.id]).work.revision;await page.click('#graph-draft-apply');await waitForRevision(page,recoveredRevision);
  const after=cli(root,['work','show',work.id]).work;assert.ok(after.tasks.find(task=>task.id==='t004').depends.includes('t000'));assert.equal(cli(root,['agent','list',work.id]).agents.length,agentsBefore);
  const canvasAfter=await page.$eval('#pilot-canvas',node=>({x:node.scrollLeft,y:node.scrollTop}));assert.ok(Math.abs(canvasAfter.x-recoveredViewport.x)<=2&&Math.abs(canvasAfter.y-recoveredViewport.y)<=2,`viewport déplacé ${JSON.stringify({recoveredViewport,canvasAfter})}`);
  const screenshot=name+'.png';await page.screenshot({path:path.join(outDir,screenshot),fullPage:true});
  await page.click('#graph-draft-close');assert.equal(await page.evaluate(()=>document.activeElement.id),'pilot-edit-graph','focus non rendu');
  await page.screenshot({path:path.join(outDir,name+'-closed.png'),fullPage:true});
  let attemptWork=cli(root,['work','show',work.id]).work;attemptWork=mutate(root,['task','update',work.id],attemptWork.revision,{id:'t000',status:'running'});attemptWork=mutate(root,['task','update',work.id],attemptWork.revision,{id:'t000',status:'blocked',outcome:'completed',blocker:'Rapport attendu'});await page.waitForFunction(revision=>snapshot?.work?.revision>=revision,{},attemptWork.revision);
  await selectNode(page,'t000');await page.waitForSelector('#pilot-inspector[open]');
  const selectedAttempt=await page.evaluate(()=>({selection:Pilot.state.selection,projection:snapshot.pilotage.tasks.t000.attempt,validation:document.querySelector('#pilot-inspector .pilot-validation')?.textContent,technical:document.querySelector('#pilot-inspector details:last-of-type')?.textContent,reports:document.querySelector('#pilot-reports')?.dataset.review}));
  assert.equal(selectedAttempt.selection.id,'t000');assert.ok(selectedAttempt.projection.attempt_id,'tentative sélectionnée absente');assert.equal(selectedAttempt.projection.cost_state,'unknown','coût absent transformé en valeur rapportée');assert.match(selectedAttempt.technical,new RegExp(selectedAttempt.projection.attempt_id));assert.doesNotMatch(selectedAttempt.validation,/validée|validated/i,'activité simple affichée comme validation');assert.equal(selectedAttempt.reports,'false');
  const activityScreenshot=name+'-attempt.png';await page.screenshot({path:path.join(outDir,activityScreenshot),fullPage:true});
  const administrative=mutate(root,['checkpoint',work.id],attemptWork.revision,{summary:'Affichage actualisé',next:'Consulter la tentative'});await page.waitForFunction(revision=>snapshot?.work?.revision>=revision,{},administrative.revision);assert.deepEqual(await page.evaluate(()=>({id:Pilot.state.selection.id,attempt:snapshot.pilotage.tasks.t000.attempt.attempt_id})),{id:'t000',attempt:selectedAttempt.projection.attempt_id},'sélection ou tentative déplacée par une mise à jour administrative');
  await page.$eval('#mode',e=>e.closest('details').open=true);await page.click('#mode');await page.click('#tabs [data-view=tasks]');
  await page.waitForFunction(count=>document.querySelectorAll('#tasks-body tr').length===count,{},after.tasks.length);
  for(const task of after.tasks)assert.equal(await page.$eval('#tasks-body tr[data-task="'+task.id+'"] td:nth-child(4)',element=>element.textContent),(task.depends||[]).join(', ')||'Aucune','task table did not use current snapshot');
  await page.click('#tabs [data-view=conduite]');assert.deepEqual(diagnostics.console_errors,[]);assert.deepEqual(diagnostics.network_errors,[]);
  return {variant:name,activity_screenshot:activityScreenshot,assertions:{arrows:true,space_connect:true,keyboard_connect:true,mouse_connect:true,preview_apply:true,no_implicit_launch:true,conflict:true,undo_redo:true,fold_counts:true,viewport:true,focus_restore:true,task_table_fresh:true,selection_attempt_stable:true,activity_not_validation:true,unknown_cost_projected:true},console_errors:diagnostics.console_errors,network_errors:diagnostics.network_errors,expected_console_errors:diagnostics.expected_console_errors||[],screenshot,viewport_observations:[{before:firstViewport,after:firstAfter},{before:recoveredViewport,after:canvasAfter}],revision_before:before.revision,revision_after:administrative.revision};
 }finally{await page?.close();server?.child.kill('SIGTERM')}
}
async function loadMeasurement(browser,cards){
 const root=path.join(temporary,'load-'+cards),work=fixture(root,cards),diagnostics={console_errors:[],network_errors:[],expected_http:new Set()},initial=[],keyboard=[];let server,page,firstOpenMS;
 try{
  ({server,page,firstOpenMS}=await openProduct(browser,root,work,'fr','etat',diagnostics));
  const sampleCount=5;
  for(let sample=0;sample<sampleCount;sample++){
   diagnostics.reloadAbortRequests=new Set(diagnostics.inflightRequests);const startAt=Date.now();await page.reload({waitUntil:'domcontentloaded'});await page.waitForFunction(count=>document.querySelectorAll('.graph-noeud').length===count,{},cards);initial.push(Date.now()-startAt);
   await page.focus('#pilot-edit-graph');await page.keyboard.press('Enter');await page.waitForSelector('#graph-draft-panel:not([hidden])');
   await page.$eval('.graph-noeud[data-task="t000"]',async element=>{
    element.focus();
    await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
    window.b03KeyboardMeasurement=new Promise(resolve=>element.addEventListener('keydown',event=>{
     if(event.key!=='Enter'||!event.isTrusted)return;
     const start=event.timeStamp;
     requestAnimationFrame(()=>requestAnimationFrame(()=>resolve({milliseconds:performance.now()-start,selected:element.dataset.draftSource==='true',focused:document.activeElement===element})));
    },{capture:true,once:true}));
   });
   await page.keyboard.press('Enter');
   const observation=await page.evaluate(()=>window.b03KeyboardMeasurement);
   assert.equal(observation.selected,true,'keyboard input did not select the graph node');assert.equal(observation.focused,true,'keyboard focus lost');keyboard.push(observation.milliseconds);await page.click('#graph-draft-close');assert.equal(await page.evaluate(()=>document.activeElement.id),'pilot-edit-graph','load journey lost focus');
  }
  assert.deepEqual(diagnostics.console_errors,[]);assert.deepEqual(diagnostics.network_errors,[]);const initialP95=p95(initial),keyboardP95=p95(keyboard);fs.writeFileSync(path.join(outDir,'load-'+cards+'-measurements.json'),JSON.stringify({cards,initial,keyboard,keyboard_method:'trusted keydown event timestamp through two animation frames, browser clock; focus setup and automation round trips excluded',initialP95,keyboardP95},null,2));
  return {cards,samples:sampleCount,first_open_ms:firstOpenMS,first_open_method:'one cold geometry observation including session-link redirect; excluded from the historical five reload p95 samples',initial_method:'five page reloads in the same tab; optional geometry cache can be warm',initial_p95_ms:initialP95,keyboard_p95_ms:keyboardP95,initial_samples_ms:initial,keyboard_samples_ms:keyboard,keyboard_method:'trusted keydown timestamp to second animation frame',expected_reload_aborts:diagnostics.expected_network_aborts,arrows:await page.$$eval('.graph-arete',nodes=>nodes.length>0),focus_recovered:true};
 }finally{await page?.close();server?.child.kill('SIGTERM')}
}
(async()=>{
 const base=path.join(temporary,'base'),work=fixture(base),browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',userDataDir:path.join(temporary,'chrome'),args:['--no-sandbox','--disable-dev-shm-usage','--disable-crash-reporter','--disable-breakpad']});
 try{
  if(process.env.SWARM_B03_LOAD_ONLY){const cards=Number(process.env.SWARM_B03_LOAD_ONLY);assert.ok([50,200,500].includes(cards));const row=await loadMeasurement(browser,cards);fs.writeFileSync(path.join(outDir,'diagnostic-load-'+cards+'.json'),JSON.stringify(row,null,2));console.log(JSON.stringify({diagnostic_only:true,row}));assert.ok(row.initial_p95_ms<=(cards===500?2000:1000),`rendu p95 ${cards}: ${row.initial_p95_ms} ms`);assert.ok(row.keyboard_p95_ms<=100,`clavier p95 ${cards}: ${row.keyboard_p95_ms} ms`);return}
  const variants=[];for(const [name,lang,theme]of[['fr-sombre','fr','sombre'],['fr-etat','fr','etat'],['en-sombre','en','sombre'],['en-etat','en','etat']])variants.push(await variant(browser,base,work,name,lang,theme));
  const load_measurements=[];for(const cards of [50,200,500])load_measurements.push(await loadMeasurement(browser,cards));
  const results={surface:'swarm-product',environment:{host:`${os.platform()} ${os.release()} ${os.arch()} ${os.cpus()[0]?.model||'unknown'}`,browser:await browser.version(),viewport:'1440x1000',binary},variants,load_measurements};fs.writeFileSync(path.join(outDir,'results.json'),JSON.stringify(results,null,2));console.log(JSON.stringify(results,null,2));
  const failures=load_measurements.flatMap(row=>[...(row.initial_p95_ms>(row.cards===500?2000:1000)?[`rendu p95 ${row.cards}: ${row.initial_p95_ms} ms`]:[]),...(row.keyboard_p95_ms>100?[`clavier p95 ${row.cards}: ${row.keyboard_p95_ms} ms`]:[])]);assert.deepEqual(failures,[],'performance budgets; all measurements retained');
 }finally{await browser.close()}
})().catch(error=>{fs.writeFileSync(path.join(outDir,'failure.json'),JSON.stringify({error:error.stack,temporary_root:temporary},null,2));console.error(error);process.exitCode=1});
