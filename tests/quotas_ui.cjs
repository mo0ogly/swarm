'use strict';
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-budget-ui-'));fs.mkdirSync(out,{recursive:true});let app,browser;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',c=>c?reject(Error(err||text)):resolve(text?JSON.parse(text):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);let w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Budget test',objective:'Verify budget changes',scope:'isolated',criteria:['No lost updates'],next:'Configure'})).work;
 await cli(['planning','enable',w.id],{schema_version:1,event_id:'planning',expected_revision:w.revision,max_tasks:10,max_decisions:10,max_activations:15});
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,args:['--no-sandbox']});const p=await browser.newPage();await p.setViewport({width:1400,height:1000});const errors=[];p.on('pageerror',e=>errors.push(String(e)));
 for(const lang of ['fr','en']){
  const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang',lang);await p.goto(u.href);await p.waitForFunction(()=>typeof snapshot!=='undefined' && snapshot?.work);
  if(await p.$eval('[data-view="budget"]',e=>e.getClientRects().length===0))await p.click('#mode');await p.click('[data-view="budget"]');await p.click('#quotas-open');await p.waitForSelector('#modal[open]');
  for(const [k,v]of Object.entries({planning_activations:'20',planning_decisions:'20',quota_reason:'Explicit isolated test authorization'}))await p.$eval('#field-'+k,(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},v);
  let before=await cli(['quotas','show',w.id]);await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#preview').hidden && !document.querySelector('#confirm').disabled);assert.equal((await cli(['quotas','show',w.id])).revision,before.revision);
  for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'quotas-'+lang+'-'+theme+'.png')})}
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open);
  let after=await cli(['quotas','show',w.id]);assert.equal(after.limits.planning_activations,20);assert.deepEqual(after.consumed,before.consumed);assert.equal(after.limits.review_calls,null);
  await p.click('#quotas-open');await p.type('#field-quota_reason','Another isolated authorization');await cli(['quotas','apply',w.id],{schema_version:1,event_id:'outside-'+lang,expected_revision:after.revision,limits:{planning_activations:21,planning_decisions:21,review_calls:null},reason:'Concurrent authorization test'});
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal-error').hidden);assert.equal((await cli(['quotas','show',w.id])).limits.planning_activations,21);
  await p.keyboard.press('Escape');assert.equal(await p.evaluate(()=>document.activeElement.id),'quotas-open');
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['quota preview read-only','web CLI parity','counts preserved','missing reviewer not invented','stale form refused','keyboard focus','FR EN both themes'],root},null,2));console.log('PASS quotas UI');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(app)app.kill()});
