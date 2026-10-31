'use strict';
/** Verify the authenticated embedded graph on an isolated native server.
 * Credentials arrive on stdin only. No provider, retries or global watchdog.
 * Assertions record real interaction state; original failures remain on stderr.
 */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[],httpErrors=[],diagnostics=[],expectedHttpRefusals=[];let expectedTasks=3;
fs.mkdirSync(cfg.output,{mode:0o700});
function check(name,value){assert(value,name);checks.push(name);console.log('PASS '+name)}
(async()=>{
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
 try{for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const context=await browser.createBrowserContext(),page=await context.newPage(),label=lang+'-'+theme,address='http://'+cfg.address;
  page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  page.on('pageerror',e=>errors.push({type:'pageerror',text:e.message}));page.on('console',m=>{if(m.type()==='error')errors.push({type:'console',text:m.text(),url:m.location().url})});page.on('requestfailed',r=>errors.push({type:'requestfailed',url:r.url()}));
  page.on('response',r=>{if(r.status()>=400)diagnostics.push((async()=>{let failure;try{const body=await r.json();failure=body.failure}catch{failure={code:'error-body-unavailable'}}httpErrors.push({url:r.url(),status:r.status(),failure})})())});
  if(cfg.workspace_preview){await page.setRequestInterception(true);page.on('request',r=>{const source={'/cockpit-native-graph.js':'web/cockpit-native-graph.js','/cockpit-native.js':'web/cockpit-native.js','/i18n-en.js':'web/i18n-en.js','/graph.css':'web/graph.css'}[new URL(r.url()).pathname];if(source)r.respond({status:200,contentType:source.endsWith('.css')?'text/css':'application/javascript',body:fs.readFileSync(source)});else r.continue()})}
  await context.setCookie({name:'swarm_session_'+crypto.createHash('sha256').update(cfg.address).digest('hex').slice(0,12),value:cfg.key,url:address,httpOnly:true,sameSite:'Strict'});
  await page.setViewport({width:1440,height:1000});
  await page.goto(address+'/?'+new URLSearchParams({work:cfg.work,lang,theme,view:'conduite'}),{waitUntil:'networkidle0'});
  check(label+'-valid-native-control',await page.$$eval('.graph-noeud',nodes=>nodes.length)===expectedTasks);
  check(label+'-graph-editing-notice',await page.$eval('#conduite>.notice',e=>e.textContent.includes(document.documentElement.lang==='en'?'create tasks and edit their links':'créer des tâches et de modifier leurs liens')));
  check(label+'-zoom-control-present',!!await page.$('#pilot-zoom-in'));
  check(label+'-translated-create',await page.$eval('#pilot-add-task',e=>e.textContent)===(lang==='en'?'Add task':'Ajouter une tâche'));
  check(label+'-translated-zoom',await page.$eval('#pilot-zoom-in',e=>e.textContent)===(lang==='en'?'Zoom in':'Agrandir le zoom'));
  check(label+'-minimap-reflects-nodes',await page.$$eval('.graph-minimap-node',nodes=>nodes.length)===expectedTasks);
  for(const [route,expected]of Object.entries(cfg.assets)){
   const actual=await page.evaluate(async route=>{const r=await fetch(route);if(!r.ok)throw Error('protected-asset-refused');return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',await r.arrayBuffer())),v=>v.toString(16).padStart(2,'0')).join('')},route);
   check(label+'-embedded-'+route,actual===expected);
  }
  const width=()=>page.$eval('#pilot-canvas .graph-svg',e=>Number(e.getAttribute('width')));
  const initial=await width();await page.click('#pilot-zoom-in');check(label+'-zoom-in',await width()>initial);await page.click('#pilot-zoom-out');check(label+'-zoom-out',Math.abs(await width()-initial)<1);
  await page.click('#pilot-fullscreen');check(label+'-fullscreen',await page.$eval('#graph-studio',e=>e.classList.contains('is-fullscreen')));
  await page.focus('#pilot-orientation');await page.keyboard.press('Tab');check(label+'-modal-keyboard-contained',await page.$eval('#graph-studio',e=>e.contains(document.activeElement)));
  await page.$eval('.graph-drag[data-task="a"]',e=>e.focus());await page.keyboard.press('ArrowDown');
  const position=await page.$eval('.graph-unit[data-task="a"]',e=>e.getAttribute('transform'));
  check(label+'-keyboard-card-moved',position!=='translate(0 0)');
  check(label+'-links-follow-card',await page.$eval('.graph-arete',e=>e.getAttribute('d').includes('C')));
  await page.keyboard.press('Escape');await page.waitForFunction(()=>document.activeElement?.id==='pilot-fullscreen'&&!document.querySelector('.native-graph-dialog').open&&!document.querySelector('#graph-studio').classList.contains('is-fullscreen'));check(label+'-escape-restores-focus',await page.$eval('#pilot-fullscreen',e=>e===document.activeElement));
  await page.reload({waitUntil:'networkidle0'});check(label+'-card-position-persisted',await page.$eval('.graph-unit[data-task="a"]',e=>e.getAttribute('transform'))===position);
  await page.click('#pilot-fullscreen');await page.click('#pilot-fit');
  const handle=await(await page.$('.graph-drag[data-task="a"]')).boundingBox();
  await page.mouse.move(handle.x+handle.width/2,handle.y+handle.height/2);await page.mouse.down();await page.mouse.move(handle.x+handle.width/2+70,handle.y+handle.height/2+45,{steps:5});await page.keyboard.press('Escape');await page.mouse.up();
  check(label+'-pointer-cancel-restores-card',await page.$eval('.graph-unit[data-task="a"]',e=>e.getAttribute('transform'))===position);
  await page.mouse.move(handle.x+handle.width/2,handle.y+handle.height/2);await page.mouse.down();await page.mouse.move(handle.x+handle.width/2+70,handle.y+handle.height/2+45,{steps:5});await page.mouse.up();
  check(label+'-pointer-card-moved',await page.$eval('.graph-unit[data-task="a"]',e=>e.getAttribute('transform'))!==position);
  await page.setViewport({width:390,height:1000});const port=await(await page.$('#pilot-canvas')).boundingBox();await page.mouse.move(port.x+port.width/2,port.y+port.height-40);
  const oldZoom=await width();await page.keyboard.down('Control');await page.mouse.wheel({deltaY:-100});await page.keyboard.up('Control');await page.waitForFunction(old=>Number(document.querySelector('#pilot-canvas .graph-svg').getAttribute('width'))>old,{},oldZoom);check(label+'-ctrl-wheel-zoom',await width()>oldZoom);
  await page.$eval('#pilot-canvas',e=>{e.scrollLeft=80});const scroll=await page.$eval('#pilot-canvas',e=>e.scrollLeft);
  await page.mouse.down();await page.mouse.move(port.x+port.width/2-40,port.y+port.height-40,{steps:4});await page.mouse.up();check(label+'-background-panning',await page.$eval('#pilot-canvas',e=>e.scrollLeft)>scroll);
  await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('.native-graph-dialog').open&&!document.querySelector('#graph-studio').classList.contains('is-fullscreen'));
  for(const mobile of [390,320]){await page.setViewport({width:mobile,height:844});await page.click('#pilot-fullscreen');check(label+'-fullscreen-width-'+mobile,await page.$eval('#graph-studio',e=>Math.round(e.getBoundingClientRect().width))===mobile);await page.screenshot({path:path.join(cfg.output,label+'-'+mobile+'.png')});await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('.native-graph-dialog').open&&!document.querySelector('#graph-studio').classList.contains('is-fullscreen'));check(label+'-no-page-overflow-'+mobile,await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth))}
  await page.setViewport({width:1440,height:1000});await page.select('#pilot-orientation','TB');check(label+'-vertical-orientation',await page.$eval('#graph',e=>e.dataset.orientation)==='TB');await page.select('#pilot-orientation','LR');
  if(cfg.editing){
   await page.setViewport({width:1440,height:1000});const task='ui-'+label;
   await page.click('#pilot-add-task');await page.waitForSelector('.native-graph-editor[open]');
   for(const [name,value]of Object.entries({id:task,title:'Created '+label,deliverable:'docs/'+task+'.md',criteria:'Visible proof'}))await page.type('#native-graph-'+name,value);
   await page.select('#native-graph-depends','a');
   const added=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/action');await page.click('#native-graph-save');const reply=await added;check(label+'-task-create-acknowledged',reply.ok());
   await page.waitForFunction(task=>!document.querySelector('.native-graph-editor')&&[...document.querySelectorAll('.graph-noeud')].some(n=>n.dataset.task===task),{},task);expectedTasks++;
   check(label+'-created-task-linked',await page.$$eval('.graph-arete',(edges,task)=>edges.some(e=>e.dataset.from==='a'&&e.dataset.to===task),task));
   const proposedBefore=await page.evaluate(async work=>(await fetch('/api/v1/planning?'+new URLSearchParams({work}))).json(),cfg.work);
   await page.$eval('.graph-port[data-task="c"][data-side="out"]',e=>e.focus());await page.keyboard.press('Enter');
   await page.$eval('.graph-port[data-task="'+task+'"][data-side="in"]',e=>e.focus());await page.keyboard.press('Enter');await page.waitForSelector('.native-graph-editor[open]');
   check(label+'-keyboard-link-staged',await page.$eval('#native-graph-depends',e=>[...e.selectedOptions].map(o=>o.value).join('|'))==='a|c');
   const proposedAfter=await page.evaluate(async work=>(await fetch('/api/v1/planning?'+new URLSearchParams({work}))).json(),cfg.work);check(label+'-link-gesture-readonly',JSON.stringify(proposedBefore)===JSON.stringify(proposedAfter));
   const linkSave=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/action');await page.click('#native-graph-save');check(label+'-keyboard-link-confirmed',(await linkSave).ok());await page.waitForFunction(()=>!document.querySelector('.native-graph-editor'));await page.waitForFunction(task=>[...document.querySelectorAll('.graph-arete')].some(e=>e.dataset.from==='c'&&e.dataset.to===task),{},task);
   await page.click('#pilot-fullscreen');await page.click('#pilot-fit');
   const fromPort=await(await page.$('.graph-port[data-task="b"][data-side="out"]')).boundingBox(),toPort=await(await page.$('.graph-port[data-task="'+task+'"][data-side="in"]')).boundingBox();
   await page.mouse.move(fromPort.x+fromPort.width/2,fromPort.y+fromPort.height/2);await page.mouse.down();await page.mouse.move(toPort.x+toPort.width/2,toPort.y+toPort.height/2,{steps:6});check(label+'-pointer-link-ghost',!!await page.$('.graph-link-ghost'));await page.mouse.up();await page.waitForSelector('.native-graph-editor[open]');check(label+'-pointer-link-staged',await page.$eval('#native-graph-depends',e=>[...e.selectedOptions].map(o=>o.value).join('|'))==='a|b|c');await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('.native-graph-editor'));await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('.native-graph-dialog').open&&!document.querySelector('#graph-studio').classList.contains('is-fullscreen'));
   await page.$eval('.graph-arete[data-from="c"][data-to="'+task+'"]',e=>e.focus());await page.keyboard.press('Delete');await page.waitForSelector('.native-graph-editor[open]');check(label+'-link-removal-staged',await page.$eval('#native-graph-depends',e=>[...e.selectedOptions].map(o=>o.value).join('|'))==='a');await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('.native-graph-editor'));
   const before=await page.evaluate(async work=>(await fetch('/api/v1/planning?'+new URLSearchParams({work}))).json(),cfg.work);
   await page.$eval('.graph-task-open[data-task="a"]',e=>e.focus());await page.keyboard.press('Enter');await page.select('#native-graph-depends',task);
   const rejected=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/action');await page.click('#native-graph-save');const denied=await rejected;check(label+'-cycle-refused',!denied.ok());expectedHttpRefusals.push({url:denied.url(),status:denied.status()});await page.waitForSelector('.native-graph-editor .notice.alert:not([hidden])');
   const after=await page.evaluate(async work=>(await fetch('/api/v1/planning?'+new URLSearchParams({work}))).json(),cfg.work);check(label+'-cycle-refusal-preserves-work',JSON.stringify(after)===JSON.stringify(before));
   await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('.native-graph-editor'));
   await page.$eval('.graph-task-open[data-task="'+task+'"]',e=>e.focus());await page.keyboard.press('Enter');await page.select('#native-graph-depends');
   const removed=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/action');await page.click('#native-graph-save');check(label+'-dependency-clear-acknowledged',(await removed).ok());
   await page.waitForFunction(()=>!document.querySelector('.native-graph-editor'));await page.waitForFunction(task=>![...document.querySelectorAll('.graph-arete')].some(e=>e.dataset.to===task),{},task);
   await page.reload({waitUntil:'networkidle0'});check(label+'-creation-and-link-removal-persist',await page.$$eval('.graph-noeud',nodes=>nodes.length)===expectedTasks&&!await page.$$eval('.graph-arete',(edges,task)=>edges.some(e=>e.dataset.to===task),task));
   if(lang==='en'&&theme==='sombre'){
    // Real POST commits, then the browser-only fixture drops its acknowledgement.
    // This is an output-loss injection, not a fake successful engine response.
    await page.evaluate(()=>{const original=window.fetch;window.graphFixtureFetch=original;window.fetch=async(...args)=>{const r=await original(...args);if(new URL(args[0],location.href).pathname==='/api/v1/action'){await r.clone().arrayBuffer();throw Error('fixture-lost-acknowledgement')}return r}});
    await page.click('#pilot-add-task');for(const [name,value]of Object.entries({id:'uncertain',title:'Uncertain acknowledgement',deliverable:'docs/uncertain.md',criteria:'Retain recorded task'}))await page.type('#native-graph-'+name,value);
    await page.click('#native-graph-save');await page.waitForSelector('.native-graph-editor .notice.alert:not([hidden])');
    check('lost-acknowledgement-blocks-resubmit',await page.$eval('#native-graph-save',e=>e.disabled)&&await page.$eval('#native-graph-id',e=>e.value)==='uncertain');
    await page.evaluate(()=>{window.fetch=window.graphFixtureFetch;delete window.graphFixtureFetch});await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('.native-graph-editor'));await page.click('#refresh');await page.waitForFunction(()=>!document.querySelector('#refresh').disabled&&!document.querySelector('#main-content').hasAttribute('aria-busy')&&[...document.querySelectorAll('.graph-noeud')].some(n=>n.dataset.task==='uncertain'));expectedTasks++;
    check('lost-acknowledgement-explicit-refresh-observes-single-task',await page.$$eval('.graph-noeud',nodes=>nodes.filter(n=>n.dataset.task==='uncertain').length)===1);
   }
  }
  await context.close();
 }
 const emptyContext=await browser.createBrowserContext(),emptyPage=await emptyContext.newPage(),address='http://'+cfg.address;
 emptyPage.setDefaultTimeout(0);emptyPage.setDefaultNavigationTimeout(0);await emptyContext.setCookie({name:'swarm_session_'+crypto.createHash('sha256').update(cfg.address).digest('hex').slice(0,12),value:cfg.key,url:address,httpOnly:true,sameSite:'Strict'});
 emptyPage.on('pageerror',e=>errors.push({type:'pageerror',text:e.message}));emptyPage.on('requestfailed',r=>errors.push({type:'requestfailed',url:r.url()}));emptyPage.on('console',m=>{if(m.type()==='error')errors.push({type:'console',text:m.text(),url:m.location().url})});emptyPage.on('response',r=>{if(r.status()>=400)httpErrors.push({url:r.url(),status:r.status()})});
 if(cfg.workspace_preview){await emptyPage.setRequestInterception(true);emptyPage.on('request',r=>{const source={'/cockpit-native-graph.js':'web/cockpit-native-graph.js','/cockpit-native.js':'web/cockpit-native.js','/i18n-en.js':'web/i18n-en.js','/graph.css':'web/graph.css'}[new URL(r.url()).pathname];if(source)r.respond({status:200,contentType:source.endsWith('.css')?'text/css':'application/javascript',body:fs.readFileSync(source)});else r.continue()})}
 await emptyPage.goto(address+'/?'+new URLSearchParams({work:cfg.empty_work,view:'conduite'}),{waitUntil:'networkidle0'});check('empty-graph-has-create-control',!!await emptyPage.$('#pilot-add-task')&&(await emptyPage.$$('.graph-noeud')).length===0);await emptyPage.click('#pilot-add-task');
 if(cfg.editing){for(const [name,value]of Object.entries({id:'first',title:'First task',deliverable:'docs/first.md',criteria:'Created from empty graph'}))await emptyPage.type('#native-graph-'+name,value);const created=emptyPage.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/action');await emptyPage.click('#native-graph-save');check('empty-graph-first-task-acknowledged',(await created).ok());await emptyPage.waitForFunction(()=>!document.querySelector('.native-graph-editor'));check('empty-graph-first-task-observed',await emptyPage.$$eval('.graph-noeud',nodes=>nodes.length===1&&nodes[0].dataset.task==='first'))}
 else{await emptyPage.keyboard.press('Escape');await emptyPage.waitForFunction(()=>!document.querySelector('.native-graph-editor'))}
 await emptyContext.close();
 await Promise.all(diagnostics);
 const unexpected=errors.filter(e=>!(e.type==='console'&&expectedHttpRefusals.some(r=>r.url===e.url&&e.text===`Failed to load resource: the server responded with a status of ${r.status} (Bad Request)`)));
 unexpected.push(...httpErrors.filter(e=>!expectedHttpRefusals.some(r=>r.url===e.url&&r.status===e.status)));
 fs.writeFileSync(path.join(cfg.output,'results.json'),JSON.stringify({checks,errors,httpErrors,expectedHttpRefusals,unexpected},null,2));check('browser-no-unexpected-errors',unexpected.length===0);
 }finally{await browser.close()}
})().catch(e=>{console.error(e);process.exitCode=1});
