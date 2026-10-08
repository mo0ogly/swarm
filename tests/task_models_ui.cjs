'use strict';
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-budget-ui-'));fs.mkdirSync(out,{recursive:true});let app,browser;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',c=>c?reject(Error(err||text)):resolve(text?JSON.parse(text):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);let w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Budget test',objective:'Verify budget changes',scope:'isolated',criteria:['No lost updates'],next:'Configure'})).work;
 const provider=path.join(root,'claude');fs.writeFileSync(provider,'#!/bin/sh\ncat >/dev/null\n',{mode:0o700});fs.writeFileSync(path.join(root,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{claude:{command:provider,args:['-p','--output-format','stream-json','--verbose']}}}));
 w=await cli(['task','add',w.id],{schema_version:1,event_id:'task',expected_revision:w.revision,id:'t1',title:'Task model test',deliverable:'report',criteria:['proof'],next:'Implement'});w=w.work||w;
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,args:['--no-sandbox']});const p=await browser.newPage();await p.setViewport({width:1400,height:1000});const errors=[];p.on('pageerror',e=>errors.push(String(e)));
 for(const lang of ['fr','en']){
  const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang',lang);await p.goto(u.href);await p.waitForFunction(()=>typeof snapshot!=='undefined' && snapshot?.work);
  await p.evaluate(()=>showView('tasks'));await p.click('#tasks-body button:last-child');await p.waitForSelector('#modal[open]');
  await p.select('#field-inherit','false');await p.select('#field-provider','claude');await p.select('#field-level','exigeant');await p.waitForFunction(()=>modalContext.modelReady && !document.querySelector('#confirm').disabled);
  const before=await cli(['task-model','show',w.id]);await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#preview').hidden && !document.querySelector('#confirm').disabled);assert.equal((await cli(['task-model','show',w.id])).revision,before.revision);
  for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'task-model-'+lang+'-'+theme+'.png')})}
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open);
  const after=await cli(['task-model','show',w.id]);assert.equal(after.tasks[0].model_selection.route.model,'opus');assert.deepEqual(after.tasks[0].attempts,before.tasks[0].attempts);
  await p.evaluate(()=>TaskModels.open('t1'));await p.select('#field-level','standard');await p.waitForFunction(()=>modalContext.modelReady);await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#preview').hidden && !document.querySelector('#confirm').disabled);assert.match(await p.$eval('#preview',e=>e.textContent),/sonnet/);await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open);assert.equal((await cli(['task-model','show',w.id])).tasks[0].model_selection.route.model,'sonnet');
  await p.click('#tasks-body button:last-child');
  const concurrent=await cli(['task-model','show',w.id]);await cli(['task-model','apply',w.id],{schema_version:1,event_id:'external-'+lang,expected_revision:concurrent.revision,task_id:'t1',inherit:true});await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal-error').hidden);assert.equal((await cli(['task-model','show',w.id])).tasks[0].model_selection,undefined);await p.keyboard.press('Escape');assert.match(await p.evaluate(()=>document.activeElement.textContent),/Modèle de la tâche|Task model/);
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['read-only preview','web CLI persistence','Opus then Sonnet','attempts preserved','FR EN both themes'],root},null,2));console.log('PASS task models UI');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(app)app.kill()});
