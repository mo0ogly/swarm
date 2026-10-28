'use strict';
// Isolated historical-agent fixture: no provider, launch or live database.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-task-reading-'));
const cli=(args,data)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(data?['--input','-']:[])],{encoding:'utf8',input:data?JSON.stringify(data):undefined}));
const req=(revision,fields)=>({schema_version:1,event_id:crypto.randomUUID(),expected_revision:revision,...fields});
cli(['init']);let w=cli(['work','create'],req(0,{title:'Comprendre une tâche',objective:'Comprendre le blocage et les tâches liées.',scope:'Fixture locale',criteria:['Lecture claire'],next:'Examiner'})).work;
for(const [id,title,depends]of [['before','Vérifier les accès',[]],['current','Préparer la livraison',['before']],['after','Contrôler la livraison',['current']]])w=cli(['task','add',w.id],req(w.revision,{id,title,depends,deliverable:'rapport.md',criteria:['Conserver les preuves'],next:'Examiner le résultat.'})).work;
cli(['autonomy',w.id,'manuel']);
const original='git cat-file : error: object file ./objects/ab/0123456789 is empty\nerror: object file ./objects/ab/0123456789 is empty\nfatal: Not a valid object name 0123456789^{commit}\n<script>fixture</script>';
w=cli(['task','update',w.id],req(w.revision,{id:'current',status:'blocked',blocker:original,next:'Faire vérifier le dépôt avant de relancer.'})).work;
execFileSync('python3',['-c',`import json,sqlite3,sys
from pathlib import Path
r=Path(sys.argv[1]);assert r.name.startswith('swarm-task-reading-')
c=sqlite3.connect(r/'.swarm/state.db');wid=sys.argv[2]
a=dict(id='historic-agent',work_id=wid,task_id='current',attempt_id='historic-attempt',provider='fixture',role='worker',status='completed',workspace=str(r),host='fixture',started='2026-10-17T09:00:00Z',ended='2026-10-17T09:05:00Z',model_route={'model':'fixture-model'},progress={'action':'Exécution d’une commande'})
c.execute('INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)',(a['id'],wid,'current',str(r),'completed',json.dumps(a),'{}'));c.commit()`,root,w.id,original]);
const initial=cli(['work','show',w.id]).work.revision;
let browser,page;const errors=[],failed=[],mutations=[],visits=[],checks=[];
const app=spawn(binary,['--root',root,'web','127.0.0.1:0']);
(async()=>{
 const session=await new Promise((resolve,reject)=>{let s='';app.stdout.on('data',b=>{s+=b;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});app.on('error',reject);app.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{const u=new URL(r.url());if(r.method()==='POST'&&u.pathname==='/api/v1/visit'&&u.searchParams.get('work')===w.id&&r.postData()==='{}')visits.push('navigation receipt');else if(!['GET','HEAD'].includes(r.method()))mutations.push(u.pathname)});page.on('requestfailed',r=>{if(r.failure()?.errorText!=='net::ERR_ABORTED')failed.push(new URL(r.url()).pathname)});
 await page.goto(session,{waitUntil:'domcontentloaded'});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  console.log('CASE task reading '+lang+'/'+theme);await page.setViewport({width:1440,height:1000});
  const u=new URL('/',session);u.searchParams.set('work',w.id);u.searchParams.set('view','conduite');u.searchParams.set('lang',lang);await page.goto(u.href,{waitUntil:'domcontentloaded'});await page.waitForFunction(id=>typeof snapshot!=='undefined'&&snapshot?.work.id===id,{},w.id);
  const renderError=await page.evaluate(()=>{try{render();return ''}catch(e){return e.stack}});assert.equal(renderError,'','complete cockpit rendering');await page.evaluate(()=>refresh(true));assert.equal(await page.evaluate(()=>connectionFailure),false,'snapshot refresh recovers before reading');
  await page.evaluate(t=>{setTheme(t);Pilot.inspect('task','current')},theme);await page.waitForSelector('#pilot-current-activity');
  assert.equal(await page.$$eval('#pilot-inspector-commands',es=>es.length),1);
  assert.equal(await page.$$eval('#pilot-current-activity .pilot-info-row strong',es=>es.length)>=4,true,'facts have bold labels');
  assert.doesNotMatch(await page.$eval('#pilot-current-activity',e=>e.innerText),/Rôle déclaré|worker|Processus :|Prochaine action/);
  assert.match(await page.$eval('#pilot-inspector-commands',e=>e.innerText),lang==='fr'?/Lecture du dépôt Git impossible/:/Cannot read the Git repository/);
  assert.doesNotMatch(await page.$eval('#pilot-inspector-body',e=>e.innerText),/git cat-file|objects\/ab|Profil non enregistré|Modèle demandé/);
  assert.equal(await page.$eval('#pilot-blocker-technical pre',e=>e.textContent),original,'technical original remains exact and literal');
  assert.equal(await page.$('#pilot-blocker-technical script'),null);
  assert.equal(await page.$eval('#pilot-inspector-body',e=>e.children[1].id),'pilot-inspector-commands','next action precedes activity details');
  const weights=await page.$$eval('#pilot-current-activity strong, #pilot-inspector h4',es=>es.map(e=>parseInt(getComputedStyle(e).fontWeight)));assert.ok(weights.every(n=>n>=700));
  assert.equal(await page.$$eval('#pilot-task-dependencies [data-related-task]',es=>es.length),2);
  await page.click('#pilot-downstream-tasks summary');assert.match(await page.$eval('#pilot-task-dependencies',e=>e.innerText),/Vérifier les accès/);assert.match(await page.$eval('#pilot-task-dependencies',e=>e.innerText),/Contrôler la livraison/);await page.click('#pilot-downstream-tasks summary');
  await page.click('#pilot-blocker-technical summary');await page.waitForFunction(()=>document.querySelector('#pilot-blocker-technical').open);assert.match(await page.$eval('#pilot-blocker-technical',e=>e.innerText),/git cat-file/);
  await page.focus('#pilot-blocker-technical summary');await page.evaluate(()=>{Pilot.inspectorKey='';return PilotInspector.render()});assert.equal(await page.$eval('#pilot-blocker-technical',e=>e.open),true);assert.equal(await page.evaluate(()=>document.activeElement.dataset.inspectorAction),'pilot-blocker-technical');
  await page.click('#pilot-blocker-technical summary');await page.click('#pilot-agent-settings summary');assert.match(await page.$eval('#pilot-agent-settings',e=>e.innerText),/fixture-model/);
  await page.click('#pilot-agent-settings summary');
  const links=await page.$$('#pilot-task-dependencies [data-related-task="before"]');await links[0].click();await page.waitForFunction(()=>document.querySelector('.pilot-mission h3')?.textContent==='Vérifier les accès');
  await page.evaluate(()=>Pilot.inspect('task','current'));
  for(const width of [1440,1024,760,390,320]){await page.setViewport({width,height:1000});await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));assert.ok(await page.$eval('#pilot-inspector',e=>e.scrollWidth<=e.clientWidth+1));if(width===1440||width===390)await page.screenshot({path:path.join(out,`task-${lang}-${theme}-${width}.png`)})}
  const variants=await page.evaluate(()=>['git cat-file : permission denied','git cat-file : object absent','Attendre la validation du client'].map(text=>({text,body:PilotInspector.blocker(text).textContent})));
  for(const variant of variants.slice(0,2))assert.match(variant.body,lang==='fr'?/Lecture du dépôt Git impossible/:/Cannot read the Git repository/);
  assert.ok(variants[2].body.includes(variants[2].text),'unrecognized business blockers remain visible');
  await page.keyboard.press('Escape');assert.equal(await page.$eval('#pilot-inspector',e=>e.open),false);
  checks.push({lang,theme,widths:5,labels:'bold',original:'exact',navigation:'task title',mutation:false});console.log('PASS task reading '+lang+'/'+theme);
 }
 assert.equal(cli(['work','show',w.id]).work.revision,initial);assert.deepEqual(mutations,[]);assert.deepEqual(errors,[]);assert.deepEqual(failed,[]);
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks,errors,failed,mutations,navigation_receipts:visits.length,scope:'isolated fixture; no provider or Git repair'},null,2));
})().catch(async e=>{fs.writeFileSync(path.join(out,'failure.txt'),e.stack);if(page)await page.screenshot({path:path.join(out,'failure.png')}).catch(()=>{});console.error(e.message);process.exitCode=1}).finally(async()=>{await browser?.close();app.kill()});
