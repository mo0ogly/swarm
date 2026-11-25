'use strict';
// Use the real served cockpit. Stdin carries the private fixture session only.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8'));
const base='http://'+cfg.address;

async function verify(){
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',
  dumpio:true,timeout:0,protocolTimeout:0,args:['--no-sandbox','--disable-dev-shm-usage']});
 try{
  const context=await browser.createBrowserContext(),page=await context.newPage();
  page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  const split=cfg.cookie.indexOf('=');
  await context.setCookie({name:cfg.cookie.slice(0,split),value:cfg.cookie.slice(split+1),url:base,httpOnly:true,sameSite:'Strict'});
  const errors=[],writes=[];
  page.on('pageerror',error=>errors.push(error.message));
  page.on('request',request=>{if(request.method()==='POST')writes.push(new URL(request.url()).pathname)});
  let held,heldCatalog,linkedReads=0,catalogReads=0,listReads=0,reportReads=0,supportReads=0;
  let catalogRequested;
  const initialCatalog=new Promise(resolve=>{catalogRequested=resolve});
  let failInterception;
  const interceptionFailure=new Promise((_,reject)=>{failInterception=reject});
  interceptionFailure.catch(()=>{});
  const observe=promise=>Promise.race([promise,interceptionFailure]);
  await page.setRequestInterception(true);
  // Provide a deterministic attachment to the real served browser in the
  // human-gate fixture. Approval, revision, session and readback stay native.
  page.on('request',async request=>{
   const uri=new URL(request.url());
   try{
    if(!cfg.eligible&&!held&&uri.pathname==='/api/v1/mission'&&!uri.searchParams.has('action')){held=request;return;}
    if(uri.pathname==='/api/v1/works'){
     assert.ok(linkedReads,'the linked mission must be fetched before the catalog');
     catalogReads++;
     if(cfg.eligible&&!heldCatalog){heldCatalog=request;catalogRequested();return;}
    }
    if(uri.pathname==='/api/v1/planning'&&uri.searchParams.get('work')===cfg.work&&linkedReads++===0){
     const response=await fetch(uri,{headers:{Cookie:cfg.cookie,Accept:'application/json',Origin:base}});
     assert.equal(response.status,200);
     const work=await response.json();
     const task=work.id===cfg.work&&work.tasks.find(task=>task.id===cfg.task);
     assert(task);
     if(task.status==='submitted')task.automatic_validation={receipt:'fixture-read-only'};
     await request.respond({contentType:'application/json',body:JSON.stringify(work)});return;
    }
    if(uri.pathname==='/api/v1/agent-logs/task-review-files'){
     const file=uri.searchParams.get('file');
     const timed=Boolean(file&&cfg.eligible);
     assert.equal(uri.searchParams.get('timing'),timed?'1':null);
     if(file==='report')reportReads++;
     else if(file==='support.md')supportReads++;
     else if(file)assert.fail('unexpected original file '+file);
     else assert.equal(uri.searchParams.get('include_report'),'1');
     if((!file&&++listReads===1&&!cfg.eligible)||(file==='support.md'&&supportReads===1)){
      await request.respond({status:503,contentType:'text/plain; charset=utf-8',body:'web-workers-saturated'});return;
     }
     if(file==='support.md'&&supportReads===2){
      await request.respond({status:502,contentType:'text/plain; charset=utf-8',body:'authority-engine-unavailable'});return;
     }
     const report={id:'report',text:'# Review proof\n\nReadable result review-task',language:'markdown'};
     const unavailable=!file&&!cfg.eligible&&listReads===3;
     const data=file==='support.md'?{id:file,text:'# Supporting evidence',language:'markdown'}
      :file?{...report,text:report.text+'\n\nRevalidated'}
      :{files:[{id:'report',path:'Report.md',kind:'markdown',readable:true},
       {id:'support.md',path:'support.md',kind:'markdown',readable:true}],
       report:unavailable?{id:'report',error:'task-review-file-digest-changed'}:report};
     await request.respond({contentType:'application/json',body:JSON.stringify({
      schema_version:1,work:cfg.work,task:cfg.task,revision:Number(uri.searchParams.get('revision')),data,
      ...(timed?{timing_ms:{engine:120,checks_and_preparation:116,work_snapshot:6,receipt:90,submission:2,git:4,
       receipt_phases:{receipt_decode:15,receipt_originals:50,receipt_publication:5,receipt_recheck:20}}}:{}),
     })});return;
    }
    await request.continue();
   }catch(error){
    errors.push(error.message);
    failInterception(error);
    await request.abort();
   }
  });
  await page.setViewport({width:cfg.eligible?1100:390,height:900});
  const query=new URLSearchParams({work:cfg.work,lang:cfg.lang,theme:cfg.eligible?'etat':'sombre'});
  if(cfg.eligible)query.set('fileTiming','1');
  await page.goto(base+'/?'+query);
  await observe(page.waitForSelector('[data-summary=warn] button',{visible:true}));
  assert.equal(await page.$eval('#work',node=>node.value),cfg.work);
  if(cfg.eligible)await observe(initialCatalog);
  else assert.equal(catalogReads,0,'the link can render before the catalog is requested');
  console.log('PASS '+cfg.lang+' linked mission renders independently of the catalog');
  const help='#native-cockpit-help';
  if(!cfg.eligible)await page.click('#nav-toggle');
  await page.click('#help');
  await page.waitForSelector(help+'[open]',{visible:true});
  assert.equal(await page.$eval(help+' h2',heading=>heading.textContent),cfg.eligible?'Aide du cockpit IA':'AI cockpit help');
  assert.equal(await page.$eval(help+' h2',heading=>heading===document.activeElement),true);
  assert.equal(await page.$$eval(help+' h3',headings=>headings.length),3);
  assert.equal(await page.$eval(help,node=>node.scrollWidth<=node.clientWidth),true);
  await page.screenshot({path:cfg.helpScreenshot});
  await page.keyboard.press('Escape');
  await page.waitForSelector(help+'[open]',{hidden:true});
  await page.waitForFunction(()=>document.activeElement?.id==='help');
  if(!cfg.eligible)await page.click('#nav-toggle');
  else{
   await page.click('#help');
   await page.click(help+' section button');
   await page.waitForSelector(help+'[open]',{hidden:true});
   await page.waitForFunction(()=>document.activeElement?.id==='main-content');
   assert.equal(await page.$eval('#tabs [data-view=conduite]',node=>node.getAttribute('aria-current')),'page');
  }
  assert.deepEqual(writes,[]);
  console.log('PASS '+cfg.lang+' cockpit help, keyboard return and read-only guidance');
  if(!cfg.eligible)assert.equal(await page.$eval('#main-content',node=>node.getAttribute('aria-busy')),'true');
  const see='[data-summary=warn] button',dialog='.workbench-task-dialog[open]',button=dialog+' .workbench-review button';
  await page.click(see);
  await page.waitForSelector(button);
  assert.equal(await page.$eval(button,node=>node.textContent),cfg.lang==='en'?'Approve':'Valider');
  assert.equal(await page.$eval(dialog,node=>node.scrollWidth<=node.clientWidth),true);
  assert.equal(await page.$eval(button,node=>!!node.closest('.task-detail-header')),true);
  const mobileFiles=dialog+' .task-detail-mobile-files',tree=dialog+' .task-detail-file-tree';
  if(!cfg.eligible){
   await observe(page.waitForSelector(dialog+' .task-detail-file-retry',{visible:true}));
   assert.match(await page.$eval(dialog+' .task-detail-file-viewer [role=status]',node=>node.textContent),/The server is already handling other reads/);
   assert.equal(await page.$eval(tree,node=>getComputedStyle(node).display),'none');
   await page.click(dialog+' .task-detail-file-retry');
  }
  await observe(page.waitForSelector(dialog+' .monaco-editor'));
  await page.waitForFunction(()=>[...document.querySelectorAll('.monaco-editor .view-lines')].some(node=>node.textContent.includes('review-task')));
  assert.equal(reportReads,0,'the first visible report must reuse the inventory response');
  assert.equal(listReads,cfg.eligible?1:2,'one inventory read per attempt');
  const find=await page.$(dialog+' .task-detail-file-toolbar [data-task-file-find]');
  assert(find,'visible search command');
  await find.click();
  await page.waitForSelector(dialog+' .find-widget.visible');
  await page.keyboard.type('review-task');
  await page.waitForFunction(()=>document.querySelector('.find-widget .matchesCount')?.textContent.includes('1'));
  await page.keyboard.press('Escape');
  if(!cfg.eligible)await page.click(mobileFiles);
  const filter=await page.$(dialog+' input[type=search]');
  await filter.type('no-such-file-fixture');
  assert.equal(await page.$$eval(dialog+' .task-detail-file-item:not([hidden])',rows=>rows.length),0);
  await filter.click({clickCount:3});
  await page.keyboard.press('Backspace');
  if(!cfg.eligible){
   await page.click(dialog+' [data-file-id="original:report"]');
   await page.waitForFunction(()=>document.querySelector('.monaco-editor .view-lines')?.textContent.includes('Revalidated'));
   assert.equal(reportReads,1,'a later explicit open revalidates the file');
   assert.equal(await page.$eval(mobileFiles,node=>node.getAttribute('aria-expanded')),'false');
  }
  await page.waitForFunction(()=>document.querySelector('.monaco-editor .view-lines')?.textContent.includes('Readable'));
  assert.match(await page.$eval(dialog+' .swarm-files-path',node=>node.textContent),cfg.eligible?/Rapport remis/:/Report handed off/);
  assert.equal(await page.$eval(dialog+' [data-directory]',node=>node.dataset.directory),cfg.eligible?'Livrables':'Deliverables');
  const bounds=await page.$eval(dialog,node=>{
   const editor=node.querySelector('.monaco-editor').getBoundingClientRect();
   const validation=node.querySelector('.workbench-review button').getBoundingClientRect();
   return {editor:Math.min(editor.bottom,node.getBoundingClientRect().bottom)-editor.top,validation:validation.top};
  });
  assert(bounds.editor>=350&&bounds.validation>=0,'report occupies usable height and approval is visible');
  const fits=await page.$eval(dialog,node=>{
   const rect=node.getBoundingClientRect();
   return rect.top>=0&&rect.left>=0&&rect.bottom<=innerHeight&&rect.right<=innerWidth;
  });
  assert(fits,'dialog stays within the viewport');
  await page.screenshot({path:cfg.screenshot});
  console.log('PASS '+cfg.lang+' submitted report opens first, Monaco search and responsive file tree');
  if(cfg.eligible){
   await page.click(dialog+' [data-file-id="original:support.md"]');
   await observe(page.waitForSelector(dialog+' .task-detail-file-retry',{visible:true}));
   assert.match(await page.$eval(dialog+' .task-detail-file-viewer [role=status]',node=>node.textContent),/Le serveur traite déjà d’autres lectures/);
   assert.equal(await page.$eval(dialog+' [aria-current="true"]',node=>node.dataset.fileId),'original:support.md');
   assert.equal(await page.$$eval(dialog+' .task-detail-file-row',rows=>rows.filter(row=>row.dataset.fileId.startsWith('original:')).length),2);
   assert.equal(supportReads,1,'saturation never starts an automatic retry');
   await page.locator(dialog+' .task-detail-file-retry').click();
   await observe(page.waitForFunction(selector=>document.querySelector(selector)?.textContent.includes('authority-engine-unavailable'),
    {},dialog+' .task-detail-file-viewer [role=status]'));
   assert.match(await page.$eval(dialog+' .task-detail-file-viewer [role=status]',node=>node.textContent),/Le moteur Swarm est indisponible/);
   assert.equal(supportReads,2,'authority failure never starts an automatic retry');
   await page.locator(dialog+' .task-detail-file-retry').click();
   await observe(page.waitForFunction(()=>/Supporting\s+evidence/.test(document.querySelector('.monaco-editor .view-lines')?.textContent||'')));
   const readTiming=await observe(page.waitForSelector(dialog+' [data-review-timing]'));
   const timing=await readTiming.evaluate(node=>JSON.parse(node.dataset.reviewTiming));
   assert.equal(timing.receipt,90);
   assert.equal(timing.receipt_phases.receipt_originals,50);
   assert.equal(timing.git,4);
   assert.equal(timing.mode,'monaco');
   assert(timing.total>=timing.request&&timing.render>=0);
   console.log('PASS named read distinguishes verification, Git and Monaco timing');
   assert.equal(supportReads,3,'each retry reads the same original once');
   assert.equal(listReads,1,'switching files does not reload the inventory');
   assert.equal(reportReads,0,'switching files does not read the submitted report again');
   console.log('PASS selecting another deliverable reads only that original');
   console.log('PASS saturation preserves the file tree and retries the selected original');
   console.log('PASS plain authority outage explains the error and retries the same file');
   await heldCatalog.continue();
   await page.waitForFunction(selector=>document.querySelector(selector)?.disabled===false,{},button);
   await page.click(button);
   await page.waitForSelector(dialog+' .task-detail-header [data-task-state=accepted]');
   assert.match(await page.$eval('.workbench-ratio',node=>node.textContent),/1 \/ 1/);
   assert.deepEqual(writes,['/api/v1/action']);
  }else{
   assert.match(await page.$eval(dialog+' .workbench-review [role=status]',node=>node.textContent),/Verified delivery evidence/);
   assert.equal(await page.$eval(dialog+' .task-detail-diagnostic',node=>node.open),false);
   await page.click(dialog+' .task-detail-diagnostic summary');
   assert.equal(await page.$eval(dialog+' .task-detail-diagnostic code',node=>node.textContent),'gate-required');
   assert.equal(await page.$eval(button,node=>node.disabled),true);
   await page.keyboard.press('Escape');
   await page.waitForSelector(dialog,{hidden:true});
   assert.equal(await page.$eval(see,node=>node===document.activeElement),true);
   assert.deepEqual(writes,[]);
   await page.click(see);
   await page.waitForSelector(dialog+' .task-detail-file-retry',{visible:true});
   assert.match(await page.$eval(dialog+' .task-detail-file-viewer [role=status]',node=>node.textContent),/task-review-file-digest-changed/);
   assert.equal(await page.$$eval(dialog+' [data-file-id="original:support.md"]',nodes=>nodes.length),1);
   assert.equal(reportReads,1,'the initial failure must not trigger a second automatic read');
   await page.locator(dialog+' .task-detail-file-retry').click();
   await page.waitForSelector(dialog+' .monaco-editor .view-lines');
   await page.waitForFunction(()=>document.querySelector('.monaco-editor .view-lines')?.textContent.includes('review-task'));
   assert.equal(reportReads,2,'the retry must perform a fresh checked read');
   console.log('PASS failed initial report keeps other files and retries with a fresh read');
   await page.keyboard.press('Escape');
   await page.waitForSelector(dialog,{hidden:true});
   await held.continue();
  }
  assert.deepEqual(errors,[]);
  console.log('PASS '+cfg.lang+' exact task detail, button, outcome and writes');
 }finally{await browser.close()}
}
verify().catch(error=>{console.error(error);process.exitCode=1});
