'use strict';
// Read-only browser recipe over an explicitly supplied isolated, paused store.
// No provider calls and no private mission database access.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const [binaryArg,rootArg,workID,outArg]=process.argv.slice(2);
assert(binaryArg&&rootArg&&workID&&outArg,'BINARY ISOLATED_ROOT WORK OUTPUT required');
const binary=path.resolve(binaryArg),root=path.resolve(rootArg),out=path.resolve(outArg);assert.notEqual(root,process.cwd(),'live source root cannot be a fixture');fs.mkdirSync(out,{recursive:true});
let app,browser,activePage;const failures=[],checks=[];
const safeURL=value=>{const u=new URL(value);return u.pathname.startsWith('/session/')?'/session/[private]':u.pathname};
async function cli(args){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args]);let text='',error='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>error+=x);p.on('exit',code=>code?reject(Error('public CLI refused '+args.join(' '))):resolve(JSON.parse(text)));p.stdin.end()})}
(async()=>{
 const before=await cli(['work','show',workID]);const status=await cli(['mission','status',workID]);assert.equal(status.paused,true,'fixture must already be paused');assert.equal(status.active_agents,0);
 const spending=await cli(['mission','spending',workID]);assert.ok(spending.calls.length>=3);assert.ok(spending.calls.some(c=>c.usage===null));assert.ok(spending.calls.some(c=>c.usage));
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const url=await new Promise((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(Error('isolated web startup not observed')),30000);app.stdout.on('data',x=>{output+=x;const m=output.match(/http:\/\/[^\s]+/);if(m){clearTimeout(timer);resolve(m[0])}});app.on('exit',()=>{clearTimeout(timer);reject(Error('isolated server ended'))})});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome'});
 for(const language of ['fr','en'])for(const theme of ['etat','sombre']){
  const page=await browser.newPage();activePage=page;page.setDefaultTimeout(30000);
  page.on('console',m=>{if(m.type()==='error')failures.push({language,theme,kind:'console',message:m.text()})});page.on('pageerror',e=>failures.push({language,theme,kind:'exception',message:e.message}));page.on('requestfailed',r=>failures.push({language,theme,kind:'request',path:safeURL(r.url()),message:r.failure()?.errorText}));page.on('response',r=>{if(r.status()>=400)failures.push({language,theme,kind:'http',path:safeURL(r.url()),status:r.status()})});
  await page.setViewport({width:1400,height:1050});const u=new URL(url);u.searchParams.set('work',workID);u.searchParams.set('view','conduite');u.searchParams.set('lang',language);await page.goto(u.href);
  await page.waitForFunction(()=>typeof snapshot!=='undefined'&&snapshot?.mission?.spending?.calls);await page.evaluate(t=>setTheme(t),theme);
  const actions=await page.waitForSelector('details[data-mission-detail="actions"] > summary');if(!await actions.evaluate(e=>e.parentElement.open))await actions.click();await page.waitForFunction(()=>document.querySelector('details[data-mission-detail="actions"]').open);
  const triggers=await page.$$('[data-mission-action="spending"]');let button;
  for(const handle of triggers)if(await handle.evaluate(e=>!!e.getClientRects().length&&!e.disabled)){button=handle;break}
  assert.ok(button,'visible spending trigger required');await button.focus();await page.keyboard.press('Enter');await page.waitForSelector('#modal[open]');
  const details=await page.waitForSelector('#modal details[data-call-ledger="calls"] summary');await details.focus();await page.keyboard.press('Enter');await page.waitForFunction(()=>document.querySelectorAll('#modal [data-provider-call]').length>0);
  const count=await page.$$eval('#modal [data-provider-call]',nodes=>nodes.length);assert.equal(count,spending.calls.length);
  const text=await page.$eval('#modal',e=>e.textContent);assert.match(text,language==='fr'?/Usage non rapporté/:/Usage not reported/);assert.match(text,language==='fr'?/Coût non rapporté/:/Cost not reported/);assert.doesNotMatch(text,/NaN|undefined|\[object Object\]/);
  const unknown=spending.calls.find(c=>c.usage===null);const unknownText=await page.evaluate(id=>[...document.querySelectorAll('[data-provider-call]')].find(e=>e.dataset.providerCall===id).textContent,unknown.id);assert.doesNotMatch(unknownText,/0.*USD|0 \/ 0/);
  await page.screenshot({path:path.join(out,`calls-${language}-${theme}.png`)});
  await page.setViewport({width:390,height:844});await page.$eval('#modal [data-provider-call]',e=>e.scrollIntoView({block:'start'}));const overflow=await page.$eval('#modal',e=>e.scrollWidth>e.clientWidth+1);assert.equal(overflow,false,'modal must fit mobile width');await page.screenshot({path:path.join(out,`calls-mobile-${language}-${theme}.png`)});
  await page.keyboard.press('Escape');await page.waitForSelector('#modal:not([open])');await page.waitForFunction(()=>document.activeElement?.dataset.missionAction==='spending');
  checks.push({language,theme,calls:count,keyboard:true,escape:true,focus_restored:true,mobile_no_overflow:true});await page.close();
 }
 assert.deepEqual(failures,[]);
 const after=await cli(['work','show',workID]);assert.deepEqual(after.work,before.work,'read-only UI changed mission');
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',root,work:workID,checks,failures,calls:spending.calls,binary},null,2));console.log('PASS actual FR/EN and both themes, ledger, keyboard, focus, mobile, diagnostics and unchanged mission');
})().catch(async e=>{if(activePage&&!activePage.isClosed()){await activePage.screenshot({path:path.join(out,'failure.png')});const diagnostics=await activePage.evaluate(()=>({focus:{tag:document.activeElement?.tagName,id:document.activeElement?.id,action:document.activeElement?.dataset.missionAction},spending:[...document.querySelectorAll('[data-mission-action="spending"]')].map(e=>({tag:e.tagName,visible:!!e.getClientRects().length,disabled:e.disabled,parent:e.parentElement?.tagName,text:e.textContent})),view:typeof view==='undefined'?null:view,folds:[...document.querySelectorAll('details[data-mission-detail]')].map(e=>({key:e.dataset.missionDetail,open:e.open})),modal:document.querySelector('#modal')?.open}));fs.writeFileSync(path.join(out,'diagnostics.json'),JSON.stringify(diagnostics,null,2))}fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({error:e.message,stack:e.stack,failures,checks},null,2));console.error(e.message);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(app){app.kill('SIGTERM');await new Promise(resolve=>app.once('exit',resolve))}});
