'use strict';
// Real browser recipe for REQ-4: FR/EN x etat/sombre, validation errors,
// keyboard submission/focus, persisted readback and secret-shaped marker scan.
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-sqlite-config-ui-'));fs.mkdirSync(out,{recursive:true});
let app,browser;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let stdout='',stderr='';p.stdout.on('data',x=>stdout+=x);p.stderr.on('data',x=>stderr+=x);p.on('exit',code=>code?reject(Error(`exit=${code} ${stderr||stdout}`)):resolve(stdout?JSON.parse(stdout):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
async function setNumber(page,selector,value){await page.$eval(selector,(input,next)=>{input.value=next;input.dispatchEvent(new Event('input',{bubbles:true}))},String(value))}
(async()=>{
 await cli(['init']);
 const work=(await cli(['work','create'],{schema_version:1,event_id:'sqlite-ui-create',expected_revision:0,title:'SQLite configuration UI',objective:'Verify persisted SQLite wait settings',scope:'isolated browser fixture',criteria:['REQ-4'],next:'Open Administration'})).work;
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const url=await new Promise((resolve,reject)=>{let text='',diagnostic='';app.stdout.on('data',chunk=>{text+=chunk;const match=text.match(/http:\/\/[^\s]+/);if(match)resolve(match[0])});app.stderr.on('data',chunk=>diagnostic+=chunk);app.on('exit',code=>reject(Error(`server exit=${code} ${diagnostic||text}`)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const checks=[],diagnostics=[];
 try{
  for(const [index,lang] of ['fr','en'].entries()){
   let navigating=false;const page=await browser.newPage();page.setDefaultTimeout(30000);await page.setViewport({width:1400,height:1000});
   page.on('pageerror',error=>diagnostics.push(`${lang}: pageerror: ${error.message}`));
   page.on('console',message=>{if(message.type()==='error')diagnostics.push(`${lang}: console: ${message.text()}`)});
   page.on('requestfailed',request=>{const error=request.failure()?.errorText||'';if(navigating&&error==='net::ERR_ABORTED'&&new URL(request.url()).pathname==='/api/v1/snapshot')return;diagnostics.push(`${lang}: requestfailed: ${request.url()} ${error}`)});
   const target=new URL(url);target.searchParams.set('work',work.id);target.searchParams.set('lang',lang);target.searchParams.set('view','admin');
   await page.goto(target.href,{waitUntil:'networkidle0'});
   if(await page.$eval('[data-view="admin"]',element=>element.getClientRects().length===0))await page.click('#mode');
   await page.click('[data-view="admin"]');
   await page.waitForFunction(()=>!document.querySelector('#storage-retry-effective').hidden);
   const title=await page.$eval('#storage-retry-title',element=>element.textContent);
   assert.match(title,lang==='fr'?/Attente du stockage/:/SQLite storage wait/);
   const visible=await page.$eval('#storage-retry-admin',element=>element.textContent);
   assert(!/PRIVATE_MARKER|api[_-]?key|password|bearer\s+[a-z0-9]/i.test(visible),visible);
   checks.push(`${lang}: administration translated; no secret-shaped value visible`);

   await setNumber(page,'#storage-busy-timeout',0);
   await page.focus('#storage-retry-save');await page.keyboard.press('Enter');
   assert.equal(await page.$eval('#storage-busy-timeout',element=>element.matches(':invalid')),true);
   assert.equal(await page.evaluate(()=>document.activeElement===document.querySelector('#storage-busy-timeout')),true);
   checks.push(`${lang}: invalid busy_timeout blocked by browser and focus moved to invalid field`);

   const values={retries:index+2,delay:11+index,timeout:220+index};
   await setNumber(page,'#storage-busy-retries',values.retries);
   await setNumber(page,'#storage-retry-delay',values.delay);
   await setNumber(page,'#storage-busy-timeout',values.timeout);
   await page.focus('#storage-retry-save');await page.keyboard.press('Enter');
   await page.waitForFunction(()=>/enregistrés et relus|saved and read back/.test(document.querySelector('#storage-retry-state').textContent));
   assert.equal(await page.evaluate(()=>document.activeElement===document.querySelector('#storage-retry-state')),true);
   const expected=(values.retries+1)*values.timeout+values.retries*values.delay;
   const effective=await page.$eval('#storage-retry-effective',element=>element.textContent);
   assert(effective.includes('sqlite_busy')&&effective.includes(String(expected)),effective);
   const persisted=await cli(['storage-retry','show']);
   assert.equal(persisted.configured.busy_timeout_ms,values.timeout);assert.equal(persisted.maximum_total_wait_ms,expected);
   checks.push(`${lang}: keyboard save, focus feedback, storage cause and total wait visible; CLI readback persisted`);

   navigating=true;await page.reload({waitUntil:'networkidle0'});navigating=false;await page.waitForFunction(()=>!document.querySelector('#storage-retry-effective').hidden);
   assert.equal(await page.$eval('#storage-busy-timeout',element=>Number(element.value)),values.timeout);
   checks.push(`${lang}: persisted value survives browser reload`);
   for(const theme of ['etat','sombre']){
    await page.evaluate(selected=>setTheme(selected),theme);
    assert.equal(await page.$eval('html',element=>element.dataset.theme),theme);
    await page.screenshot({path:path.join(out,`sqlite-config-${lang}-${theme}.png`),fullPage:true});
    checks.push(`${lang}/${theme}: real browser capture`);
   }
   navigating=true;await page.close();
  }
  assert.deepEqual(diagnostics,[]);
  fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks,diagnostics,root,at:new Date().toISOString()},null,2));
  console.log(checks.join('\n'));
 }finally{if(browser)await browser.close();if(app)app.kill()}
})().catch(error=>{console.error(error);process.exitCode=1}).finally(()=>{try{if(app)app.kill()}catch{}});
