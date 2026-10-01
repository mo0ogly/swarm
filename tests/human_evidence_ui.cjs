'use strict';
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{execSync}=require('child_process'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);
fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-human-evidence-ui-'));
let app,browser;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',c=>c?reject(Error(err||text)):resolve(text?JSON.parse(text):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);
 let w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Human evidence',objective:'Test policy',scope:'isolated',criteria:['Behavior'],next:'Configure'})).work;
 w=(await cli(['task','add',w.id],{schema_version:1,event_id:'task',expected_revision:w.revision,id:'t1',title:'Human evidence task',owner:'worker',deliverable:'docs/t1.md',criteria:['Automated behavior','Human judgment'],next:'Configure'})).work;
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const errors=[];
 for(const lang of ['fr','en']){
 const p=await browser.newPage();p.on('pageerror',e=>errors.push(e.message));await p.setViewport({width:1440,height:1050});
 const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang',lang);await p.goto(u.href);
 await p.waitForFunction(()=>typeof snapshot!=='undefined'&&snapshot?.work);
 if(!await p.$eval('[data-view="tasks"]',e=>e.getClientRects().length))await p.click('#mode');await p.waitForSelector('[data-view="tasks"]',{visible:true});await p.click('[data-view="tasks"]');await p.waitForSelector('#tasks-body [data-task="t1"] button:nth-child(2)',{visible:true});await p.click('#tasks-body [data-task="t1"] button:nth-child(2)');
 await p.waitForSelector('#validation-mode');assert.equal(await p.$eval('#validation-mode',e=>e.value),'human');
 for(const b of await p.$$('#validation-controls .validation-control > button'))await b.click();
 await p.click('#validation-add-control');
 for(const [field,value] of Object.entries({inputs:'go.mod',id:'check',args:'version',justification:'Checks the first criterion only',dir:'.'})){
 await p.$eval('[data-validation-field="'+field+'"]',(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},value);
 }
 await p.click('[data-criterion="1"]');
 const text=await p.$eval('#modal',e=>e.textContent);
 assert.match(text,lang==='fr'?/Fichiers examinés/:/Examined files/);
 await p.$eval('[data-validation-field="inputs"]',e=>e.scrollIntoView({block:'center'}));
 for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'human-'+lang+'-'+theme+'.png')})}
 await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#preview').hidden);
 const preview=await p.$eval('#preview',e=>e.textContent);assert.match(preview,/go.mod/);assert.match(preview,lang==='fr'?/aucune acceptation automatique/:/no automatic acceptance/i);
 await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open);
 const saved=await cli(['work','show',w.id]);const task=(saved.work||saved).tasks[0];assert.equal(task.validation_policy.mode,'human');assert.deepEqual(task.validation_policy.controls[0].inputs,['go.mod']);assert.equal(task.status,'todo');
 await p.close();
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',errors,checks:['FR/EN human control form','both themes','preview and apply','input paths persisted','no automatic acceptance']},null,2));console.log('PASS human evidence UI');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(app)app.kill()});
