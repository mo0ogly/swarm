'use strict';
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-budget-ui-'));fs.mkdirSync(out,{recursive:true});let app,browser;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',c=>c?reject(Error(err||text)):resolve(text?JSON.parse(text):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);const w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Budget test',objective:'Verify budget changes',scope:'isolated',criteria:['No lost updates'],next:'Configure'})).work;
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,args:['--no-sandbox']});const p=await browser.newPage();await p.setViewport({width:1400,height:1000});const errors=[];p.on('pageerror',e=>errors.push(String(e)));
 for(const lang of ['fr','en']){
  const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang',lang);await p.goto(u.href);await p.waitForFunction(()=>typeof PilotActions!=='undefined' && typeof snapshot!=='undefined' && snapshot?.work);
  if(await p.$eval('[data-view="budget"]',e=>e.getClientRects().length===0))await p.click('#mode');await p.click('[data-view="budget"]');await p.click('#budget-edit');await p.waitForSelector('#modal[open]');
  for(const [key,value] of Object.entries({limit:'10',reserve:'2',source:'Test fixture',date:'2026-09-23'}))await p.$eval('#field-'+key,(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},value);
  const before=await cli(['budget','show',w.id]);await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#preview').hidden && !document.querySelector('#confirm').disabled);
  assert.equal((await cli(['budget','show',w.id])).revision,before.revision,'preview must not write');
  for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,lang+'-'+theme+'.png')});assert.equal(await p.$eval('#confirm',e=>getComputedStyle(e).visibility),'visible')}
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open);
  const after=await cli(['budget','show',w.id]);assert.equal(after.revision,before.revision+1);assert.equal(after.budget.budget.limit_usd,10);
  // A stale modal must fail after a separate CLI update.
  await p.click('#budget-edit');const r=after.revision;await cli(['budget','apply',w.id],{schema_version:1,event_id:'external-'+lang,expected_revision:r,budget:{limit_usd:12,reserve_per_launch_usd:2,estimate_source:'fixture',reference_date:'2026-09-23'}});
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal-error').hidden);assert.equal((await cli(['budget','show',w.id])).budget.budget.limit_usd,12);
  await p.keyboard.press('Escape');assert.equal(await p.evaluate(()=>document.activeElement.id),'budget-edit');
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['preview is read-only','web and CLI parity','stale modal rejected','Escape restores focus','French and English','both themes'],root},null,2));console.log('PASS budget UI');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(app)app.kill()});
