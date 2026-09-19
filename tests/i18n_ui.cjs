'use strict';
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn,execFile}=require('child_process'),{promisify}=require('util'),puppeteer=require('puppeteer');
const exec=promisify(execFile),binary=path.resolve(process.argv[2]||'bin/swarm'),out=path.resolve(process.argv[3]||'test-results/i18n');fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-readme-i18n-'));let app,browser;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',code=>code?reject(Error(err||text)):resolve(text?JSON.parse(text):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);let w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'À vérifier',objective:'Ne pas traduire mon besoin français.',scope:'UI fixture only',criteria:['Conserver mes données'],next:'Enregistrer'})).work;
 for(const [id,depends] of [['source',[]],['verification',['source']]])w=(await cli(['task','add',w.id],{schema_version:1,event_id:id,expected_revision:w.revision,id,title:id==='source'?'Enregistrer':'Résultat français',owner:'fixture',deliverable:'docs/'+id+'.md',criteria:['À vérifier'],depends,next:'Enregistrer'})).work;
 const fr=await exec(binary,['--root',root,'--lang','fr','--json','work','show',w.id]);
 const en=await exec(binary,['--root',root,'--lang','en','--json','work','show',w.id]);
 assert.deepEqual(JSON.parse(en.stdout),JSON.parse(fr.stdout),'CLI JSON is language-independent');
 assert.match((await exec(binary,['--lang','en','help','management'])).stdout,/mission/i);
 await exec('python3',['scripts/readme-team-fixture.py',root]);
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',code=>reject(Error('server '+code)))});
 browser=await puppeteer.launch({executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});const p=await browser.newPage(),errors=[];p.on('pageerror',e=>errors.push(e.message));await p.setViewport({width:1440,height:1050});await p.goto(url);await p.waitForSelector('#swarm-language');await p.waitForFunction(()=>document.querySelector('#connection').textContent.includes('Connecté'));
 for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'graph-fr-'+theme+'.png')})}
 const before=await p.evaluate(()=>JSON.stringify(snapshot.work));
 await Promise.all([p.waitForNavigation(),p.select('#swarm-language','en')]);await p.waitForFunction(()=>document.querySelector('#connection').textContent.includes('Connected'));
 assert.equal(await p.$eval('html',e=>e.lang),'en');
 await p.setRequestInterception(true);p.on('request',r=>{if(r.url().endsWith('/api/v1/i18n-error-fixture'))return r.respond({status:400,contentType:'application/json',body:JSON.stringify({error:'Le serveur a répondu HTTP 401. Vérifiez la clé, le modèle et l’adresse.',failure:{code:'validation'}})});r.continue()});
 const displayedError=await p.evaluate(async()=>{try{await api('/api/v1/i18n-error-fixture')}catch(e){return {message:e.message,code:e.code,status:e.status}}});
 assert.deepEqual(displayedError,{message:'The server returned HTTP 401. Check the key, model and URL.',code:'validation',status:400});
 await p.setRequestInterception(false);p.removeAllListeners('request');
 assert.equal(await p.$eval('[data-view="providers"]',e=>e.textContent),'AI and connections');
 assert.equal(await p.evaluate(()=>JSON.stringify(snapshot.work)),before,'language must not rewrite mission data');
 const titles=await p.evaluate(()=>snapshot.work.tasks.map(t=>t.title));assert.deepEqual(titles,['Enregistrer','Résultat français']);
 assert.match(await p.$eval('.team-role-flow',e=>e.textContent),/production tasks/);assert.doesNotMatch(await p.$eval('.team-role-flow',e=>e.textContent),/décision|Réalise|Examine chaque|appels utilisés/);
 assert.match(await p.$eval('#pilot-canvas',e=>e.textContent),/Orchestrator/);assert.match(await p.$eval('#pilot-canvas',e=>e.textContent),/AI reviewer/);
 for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'graph-en-'+theme+'.png')});await p.$eval('#pilot-canvas',e=>e.scrollIntoView({block:'center'}));await p.screenshot({path:path.join(out,'roles-en-'+theme+'.png')});await p.evaluate(()=>scrollTo(0,0))}
 await p.click('[data-view="providers"]');await p.waitForSelector('#connections-add');await p.click('#connections-add');await p.waitForSelector('#modal[open]');assert.equal(await p.$eval('#modal-title',e=>e.textContent),'Add an AI connection');
 await p.type('#field-connection_label','À vérifier');assert.equal(await p.$eval('#field-connection_label',e=>e.value),'À vérifier');
 for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'connection-en-'+theme+'.png')})}
 await p.keyboard.press('Escape');await p.click('#help');await p.waitForSelector('#modal[open]');assert.match(await p.$eval('#modal',e=>e.textContent),/Add your model and test the connection/);await p.keyboard.press('Escape');
 await p.goto(new URL('/prepare.html',url).href);await p.waitForFunction(()=>document.documentElement.lang==='en');assert.equal(await p.$eval('#title',e=>e.textContent),'Prepare a project');
 await p.type('#new-need','Mon besoin français reste intact.');await p.select('#swarm-language','fr');assert.equal(await p.$eval('html',e=>e.lang),'en','dirty preparation blocks language navigation');assert.equal(await p.$eval('#new-need',e=>e.value),'Mon besoin français reste intact.');assert.match(await p.$eval('#error',e=>e.textContent),/before changing language/);
 await p.$eval('#new-need',e=>e.value='');
 for(const theme of ['etat','sombre']){await p.evaluate(t=>document.documentElement.dataset.theme=t,theme);await p.screenshot({path:path.join(out,'prepare-en-'+theme+'.png')})}
 await Promise.all([p.waitForNavigation(),p.select('#swarm-language','fr')]);assert.equal(await p.$eval('html',e=>e.lang),'fr');assert.equal(await p.$eval('#title',e=>e.textContent),'Préparer un projet');
 await p.goto(new URL('/?lang=en',url).href);await p.waitForSelector('#swarm-language');assert.equal(await p.$eval('html',e=>e.lang),'en');
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['language switch and persistence','mission titles and payload unchanged','planner worker reviewer graph','English connection modal','help dialog','preparation draft navigation guard','both themes','French restored','no JS errors']},null,2));console.log('PASS bilingual web UI');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(app)app.kill()});
