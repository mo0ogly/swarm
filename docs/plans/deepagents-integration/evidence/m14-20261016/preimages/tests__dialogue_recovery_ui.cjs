'use strict';
// Real CLI, supervisor, API and browser against an owned fixture project.
const fs=require('node:fs'),os=require('node:os'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-dr-ui-'));
const cli=(args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',input:input?JSON.stringify(input):undefined}));
const stop=id=>execFileSync(binary,['--root',root,'agent','stop',id],{stdio:'pipe'});
const workOf=value=>value.work||value;
const request=(w,extra)=>({schema_version:1,event_id:crypto.randomUUID(),expected_revision:w.revision,...extra});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const redact=value=>String(value).replace(/\/session\/[^\s/?]+/g,'/session/[private]');
async function waitFor(check,description,timeout=15000){const until=Date.now()+timeout;while(Date.now()<until){const value=check();if(value)return value;await sleep(100)}throw Error(description)}
let server,browser,page,agent,work,integrityFailure=false,navigating=false;const errors=[],failures=[],checks=[],expectedFailures=[],navigationAborts=[];
(async()=>{try{
 cli(['init']);
 const script=path.join(root,'codex');
 fs.writeFileSync(script,"#!/bin/sh\ncat >/dev/null\nprintf 'call\\n' >>\"$0.calls\"\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"browser-fixture-session\"}' '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":10,\"output_tokens\":2}}'\n",{mode:0o700});
 fs.writeFileSync(path.join(root,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{fixture:{command:script,args:['exec','--json','--sandbox','workspace-write','-']}}}));
 work=workOf(cli(['work','create'],request({revision:0},{title:'Dialogue recovery fixture',objective:'Inspect retained boundaries',scope:'Owned local fixture',criteria:['Qualified recovery'],next:'Inspect'})));
 work=workOf(cli(['task','add',work.id],request(work,{id:'t1',title:'Inspect dialogue boundary',deliverable:'fixture.txt',criteria:['No implicit provider restart'],next:'Examine the retained boundary'})));
 agent=cli(['agent','start',work.id],request(work,{task_id:'t1',provider:'fixture',role:'worker',mode:'dialogue',workspace:'.',instruction:'Local fixture without a model',capture_output:true,timeout_seconds:30})).agent;
 await waitFor(()=>cli(['agent','show',agent.id]).dialogue?.boundary_confirmed,'No confirmed provider boundary');
 stop(agent.id);
 await waitFor(()=>{const a=cli(['agent','show',agent.id]).agent;return !['queued','starting','running','stopping'].includes(a.status)},'Owned dialogue did not stop');
 const baseline=JSON.stringify(cli(['work','show',work.id]).work),calls=fs.readFileSync(script+'.calls','utf8');assert.equal(calls,'call\n');
 server=spawn(binary,['--root',root,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
 const address=await new Promise((resolve,reject)=>{let text='';server.stdout.on('data',chunk=>{text+=chunk;const match=text.match(/http:\/\/\S+\/session\/\S+/);if(match)resolve(match[0])});server.once('exit',code=>reject(Error('Fixture web server exited: '+code)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});page=await browser.newPage();
 const expectedFailure=(url,status)=>integrityFailure&&new URL(url).pathname==='/api/v1/recovery-preview'&&status===400;
 page.on('pageerror',e=>errors.push(redact(e.message)));page.on('console',message=>{if(message.type()==='error'){if(expectedFailure(message.location().url||address,400)&&/400/.test(message.text()))expectedFailures.push({console:redact(message.text())});else errors.push(redact(message.text()))}});
 page.on('requestfailed',r=>{const item={url:redact(r.url()),error:r.failure()?.errorText},u=new URL(r.url());if(navigating&&item.error==='net::ERR_ABORTED'&&u.pathname==='/api/v1/snapshot'&&u.searchParams.get('work')===work.id)navigationAborts.push(item);else failures.push(item)});page.on('response',r=>{if(r.status()>=400)(expectedFailure(r.url(),r.status())?expectedFailures:failures).push({url:redact(r.url()),status:r.status()})});
 const navigate=async(target,options)=>{navigating=true;try{return await page.goto(target,options)}finally{navigating=false}};
 const url=new URL(address);url.searchParams.set('work',work.id);
 for(const lang of ['fr','en']){
  url.searchParams.set('lang',lang);await page.setViewport({width:1440,height:1100});await navigate(url.href);await page.waitForFunction(()=>snapshot?.work?.tasks.some(t=>t.id==='t1'));
  for(const theme of ['etat','sombre']){
   await page.evaluate(t=>setTheme(t),theme);
   await page.waitForSelector('#mission-results > summary');if(!await page.$eval('#mission-results',n=>n.open))await page.click('#mission-results > summary');
   await page.waitForSelector('[data-mission-action="recovery-preview-t1"]');await page.click('[data-mission-action="recovery-preview-t1"]');
   await page.waitForSelector('[data-dialogue-recovery="review_boundary"]');
   const text=await page.$eval('[data-dialogue-recovery]',n=>n.textContent);
   assert.match(text,lang==='fr'?/Reprise de conversation/:/Conversation recovery/);assert.match(text,lang==='fr'?/Non vérifié/:/Not verified/);assert.match(text,lang==='fr'?/ne lance aucun agent/:/starts no agent/);
   assert.equal(await page.$$eval('[data-dialogue-check][data-check-status="unknown"]',nodes=>nodes.length),2);
   assert.equal(await page.$eval('#confirm',n=>n.hidden),true);
   assert.equal(await page.$eval('[data-dialogue-verified]',n=>n.open),false);await page.click('[data-dialogue-verified] summary');assert.equal(await page.$eval('[data-dialogue-verified]',n=>n.open),true);await page.click('[data-dialogue-verified] summary');
   const contrasts=await page.evaluate(()=>{
    const rgb=s=>s.match(/[\d.]+/g)?.map(Number),lum=c=>c.slice(0,3).map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4}).reduce((sum,v,i)=>sum+v*[.2126,.7152,.0722][i],0),values=[];
    for(const n of document.querySelectorAll('[data-dialogue-recovery] h4,[data-dialogue-recovery] p,[data-dialogue-recovery] li,[data-dialogue-recovery] summary')){
     if(!n.getClientRects().length)continue;let b=n,bg;
     while(b){const c=rgb(getComputedStyle(b).backgroundColor);if(c&&(c.length===3||c[3]>0)){bg=c;break}b=b.parentElement}
     if(!bg)continue;const a=lum(rgb(getComputedStyle(n).color)),c=lum(bg);values.push((Math.max(a,c)+.05)/(Math.min(a,c)+.05));
    }return values;
   });assert(contrasts.length>=5);assert(contrasts.every(value=>value>=4.5),`${lang}/${theme}: contrast ${contrasts}`);
   for(const width of [1440,760,390,320]){await page.setViewport({width,height:1100});assert(await page.$eval('#modal',n=>n.scrollWidth<=n.clientWidth+1),`${lang}/${theme}/${width}: modal overflow`)}
   await page.screenshot({path:path.join(out,`dialogue-${lang}-${theme}-mobile.png`)});
   await page.setViewport({width:1440,height:1100});await page.screenshot({path:path.join(out,`dialogue-${lang}-${theme}.png`)});
   await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#modal').open);assert.equal(await page.evaluate(()=>document.activeElement?.dataset.missionAction),'recovery-preview-t1','Escape restores action focus after refresh');
  }
 }
 checks.push('Actual provider boundary and stopped process inspected through recovery-preview; no provider restart','FR/EN, both themes, 1440/760/390/320 widths, Escape and semantic action focus restoration','Two unknown checks remain visible, verified checks expand on demand and confirmation is hidden','Rendered diagnostic text contrast is at least 4.5:1');
 const xss=await page.evaluate(()=>{const p={task:'t1',kept:[],redone:[],criteria:[],dialogue:{recommended_path:'reconcile_effects',recommendation:'<img src=x onerror=alert(1)>',checks:[{code:'untrusted',status:'unknown',message:'<script>window.injected=true</script>'}]}};const view=Mission.recoveryPreviewView(p);return {text:view.textContent,executable:view.querySelectorAll('img,script,button').length,executed:!!window.injected}});
 assert.equal(xss.executable,0);assert.equal(xss.executed,false);assert(xss.text.includes('<img src=x onerror=alert(1)>'));checks.push('Untrusted diagnostic text is rendered as text without executable nodes');
 if(process.argv.includes('--integrity')){
  // This database belongs only to the isolated fixture created above.
  const fixtureState=session=>execFileSync('python3',['-c',
   "import sqlite3,sys,json; c=sqlite3.connect(sys.argv[1]); session=sys.argv[3]; c.execute('UPDATE agent_dialogue_turns SET session_id=? WHERE agent_id=?',(session,sys.argv[2])) if session else None; c.commit(); tables=['works','agents','agent_dialogue_turns','reservations','events','agent_logs','cockpit_events']; print(json.dumps({t:c.execute('SELECT * FROM '+t+' ORDER BY rowid').fetchall() for t in tables},default=lambda b:b.hex(),sort_keys=True))",
   path.join(root,'.swarm/state.db'),agent.id,session||''],{encoding:'utf8'}).trim();
  const corrupted=fixtureState('foreign-fixture-session');integrityFailure=true;
  for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
   url.searchParams.set('lang',lang);await navigate(url.href,{waitUntil:'networkidle0'});await page.waitForFunction(()=>snapshot?.work?.tasks.some(t=>t.id==='t1'));await page.evaluate(t=>setTheme(t),theme);
   await page.waitForSelector('#mission-results > summary');if(!await page.$eval('#mission-results',n=>n.open))await page.click('#mission-results > summary');
   const response=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/recovery-preview');await page.click('[data-mission-action="recovery-preview-t1"]');const refused=await response;assert.equal(refused.status(),400);
   const body=await refused.text();assert(!body.includes('foreign-fixture-session'));assert(!body.includes('browser-fixture-session'));
   await page.waitForFunction(()=>document.querySelector('#modal-fields')?.textContent.includes(document.documentElement.lang==='fr'?'Aperçu de reprise indisponible':'Recovery preview unavailable'));
   const text=await page.$eval('#modal-fields',n=>n.textContent);assert.match(text,lang==='fr'?/Session du tour différente/:/Turn session differs/);assert.equal(await page.$eval('#confirm',n=>n.hidden),true);assert.equal(await page.$$eval('[data-dialogue-recovery]',nodes=>nodes.length),0);
   for(const width of [1440,320]){await page.setViewport({width,height:1100});assert(await page.$eval('#modal',n=>n.scrollWidth<=n.clientWidth+1));await page.screenshot({path:path.join(out,`integrity-${lang}-${theme}-${width}.png`)})}
   await page.setViewport({width:1440,height:1100});await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#modal').open);assert.equal(await page.evaluate(()=>document.activeElement?.dataset.missionAction),'recovery-preview-t1');
  }
  assert.equal(fixtureState(),corrupted,'Refusing corrupt state modified the isolated database');integrityFailure=false;fixtureState('browser-fixture-session');assert(cli(['agent','show',agent.id]).dialogue.boundary_confirmed);
  assert.equal(expectedFailures.filter(f=>f.status===400).length,4);checks.push('Inconsistent answered session rejected through the actual button: HTTP 400, FR/EN error, both themes, desktop/mobile, Escape/focus, no confirmation, no read mutation; exact isolated fixture repaired for positive reread');
 }
 assert.equal(JSON.stringify(cli(['work','show',work.id]).work),baseline,'Preview changed work or attempts');assert.equal(fs.readFileSync(script+'.calls','utf8'),calls,'Preview invoked provider');
 assert.deepEqual(errors,[]);assert.deepEqual(failures,[]);
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',root,checks,errors,failures,expectedFailures,navigationAborts,provider_invocations:1,model_calls:0},null,2));console.log(JSON.stringify({status:'PASS',checks:checks.length,provider_invocations:1,model_calls:0}));
}catch(error){const focus=page?await page.evaluate(()=>({active:{id:document.activeElement?.id,action:document.activeElement?.dataset.missionAction},returnTarget:typeof returnFocus!=='undefined'?{id:returnFocus?.id,connected:returnFocus?.isConnected,action:returnFocus?.dataset.missionAction}:null})).catch(()=>null):null;fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({error:redact(error.stack||error),errors,failures,focus},null,2));if(page)await page.screenshot({path:path.join(out,'failure.png')}).catch(()=>{});throw Error(redact(error.message))
}finally{await browser?.close();if(server){server.kill('SIGTERM');await Promise.race([new Promise(resolve=>server.once('exit',resolve)),sleep(5000)])}if(agent&&!integrityFailure){try{const a=cli(['agent','show',agent.id]).agent;if(['queued','starting','running','stopping'].includes(a.status)){stop(agent.id);await waitFor(()=>!['queued','starting','running','stopping'].includes(cli(['agent','show',agent.id]).agent.status),'Fixture supervisor cleanup failed',6000)}}catch(error){console.error(redact(error.message));process.exitCode=1}}}
})().catch(error=>{console.error(redact(error.stack||error));process.exitCode=1});
