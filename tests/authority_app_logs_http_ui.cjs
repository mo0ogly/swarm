'use strict';

/**
 * Check the app log console through a real authority + engine fixture, without
 * using a live Admin session. Arguments: ENGINE DAEMON FRESH_OUTPUT. TMPDIR and
 * the unique SWARM_LOGS_FIXTURE_TMP must be on the admitted fixture volume.
 * Compares protected asset bytes against source and checks routes/UI together.
 */
const fs=require('node:fs'),path=require('node:path'),net=require('node:net'),{spawn,spawnSync}=require('node:child_process'),puppeteer=require('puppeteer');
if(process.argv.length!==5)throw Error('Usage: authority_app_logs_http_ui.cjs ENGINE_BIN DAEMON_BIN OUTPUT');
const [engineBin,daemonBin,output]=process.argv.slice(2).map(p=>path.resolve(p));
if(fs.existsSync(output))throw Error('Fresh output required');fs.mkdirSync(output,{recursive:true});
const repo=process.cwd();
const scratch=path.resolve(process.env.SWARM_LOGS_FIXTURE_TMP);
if (!fs.statSync(scratch).isDirectory()) throw Error('An existing private fixture area is required');
const dir=fs.mkdtempSync(path.join(scratch,'authority-'));
const env={...process.env,TMPDIR:scratch},checks=[],errors=[],children=[];
const check=(name,ok,detail)=>{checks.push({name,ok:!!ok,detail});if(!ok)throw Error('check failed: '+name+(detail?' '+JSON.stringify(detail):''))};
function run(bin,args,input){const r=spawnSync(bin,args,{env,input,encoding:'utf8'});if(r.status!==0)throw Error(args.join(' ')+': '+r.stderr);return r.stdout?JSON.parse(r.stdout):null}
const free=()=>new Promise(res=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>res(p))})});
async function listening(port,child){for(;;){if(child.exitCode!==null||child.signalCode!==null)throw Error('fixture child exited before listen: '+child.exitCode);const ok=await new Promise(res=>{const c=net.connect(port,'127.0.0.1',()=>{c.end();res(true)});c.on('error',()=>res(false))});if(ok)return;await new Promise(r=>setTimeout(r,50))}}
(async()=>{let browser,success=false;try{
 const prep=run(daemonBin,['prepare','--dir',dir,'--controller-uid','4242','--admin','ana','--json']);
 fs.writeFileSync(path.join(dir,'configuration/authority.json'),JSON.stringify({schema_version:1,session_absolute_seconds:3600,session_idle_seconds:1800,argon2:{memory_kib:8,passes:1,parallelism:1},relay_body_max_bytes:16777216}));
 const base=['--declaration',prep.declaration,'--pin',prep.pin];
 run(daemonBin,['init','--admin','ana','--display','Ana Admin',...base,'--json'],'ana pass\n');
 const root=path.join(scratch,'root');fs.mkdirSync(root,{mode:0o700});
 const cli=(args,input)=>run(engineBin,['--root',root,...args],input);
 cli(['init','--json']);const shown=cli(['engine-settings','show','--json']);const values={...shown.values,runtime_temporary_directory:scratch,web_access_mode:'authority',web_authority_public_key:prep.public_key};
 const change={schema_version:shown.schema_version,event_id:'authority-ui',expected_revision:shown.revision,values,reason:'Authority Admin UI fixture',preview_token:''};const input=path.join(root,'change.json');
 fs.writeFileSync(input,JSON.stringify(change));change.preview_token=cli(['engine-settings','preview','--input',input,'--json']).preview_token;fs.writeFileSync(input,JSON.stringify(change));cli(['engine-settings','apply','--input',input,'--json']);
 const [enginePort,frontPort]=[await free(),await free()];
 children.push(spawn(engineBin,['--root',root,'web','127.0.0.1:'+enginePort],{env,stdio:['ignore',fs.openSync(path.join(output,'engine.stdout'),'w'),fs.openSync(path.join(output,'engine.stderr'),'w')]}));await listening(enginePort,children.at(-1));
 children.push(spawn(daemonBin,['serve',...base,'--listen','127.0.0.1:'+frontPort,'--engine','127.0.0.1:'+enginePort],{env,stdio:['ignore',fs.openSync(path.join(output,'daemon.stdout'),'w'),fs.openSync(path.join(output,'daemon.stderr'),'w')]}));await listening(frontPort,children.at(-1));
 const origin='http://127.0.0.1:'+frontPort;
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',args:['--no-sandbox']});
 const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 const expectedRefusals=new Map();
 page.on('pageerror',e=>errors.push('pageerror: '+e.message));
 page.on('console',m=>{
  if(m.type()!=='error')return;
  const expected=expectedRefusals.get(m.location().url);
  if(expected!==undefined&&m.text().startsWith('Failed to load resource:')&&m.text().includes('status of '+expected))return;
  errors.push('console: '+m.text()+' ('+m.location().url+')')
 });page.on('response',r=>{if(r.status()>=400)errors.push('http: '+r.status()+' '+r.request().method()+' '+r.url())});
 await page.setViewport({width:1440,height:960});

 const digest = b => require('node:crypto').createHash('sha256').update(b).digest('hex');
 const sources = [
  ['/native-runtime/authority-admin.js','web/native-authority-admin.js'],
  ['/native-runtime/admin.css','frontend/engine-settings/admin.css'],
  ['/i18n-en.js','web/i18n-en.js'],
 ];
 for (const [url] of sources) {
  const refusal = await fetch(origin+url, {redirect:'manual'});
  check('anonymous protected '+url, [302,303,401,403].includes(refusal.status),refusal.status);
 }
 const reject = await fetch(origin+'/api/v1/mission', {redirect:'manual'});
 check('real anonymous mission refusal', reject.status===401,reject.status);
 const rejectTask = await fetch(origin+'/api/v1/task', {redirect:'manual'});
 check('real anonymous task refusal', rejectTask.status===401,rejectTask.status);
 const deniedHistory = await fetch(origin+'/api/v1/authority/log?position=all', {redirect:'manual'});
 check('anonymous history access is refused', [401,403].includes(deniedHistory.status),deniedHistory.status);
 await page.goto(origin+'/login?lang=fr');
 await page.type('input[name=subject]','ana');await page.type('input[name=password]','ana pass');
 await Promise.all([page.waitForNavigation(),page.click('button[type=submit]')]);
 check('real sign-in',!page.url().includes('/login'));
 for (const [url,file] of sources) {
  const result=await page.evaluate(async url=>{const response=await fetch(url);return {status:response.status, bytes:Array.from(new Uint8Array(await response.arrayBuffer()))}},url);
  check('matched protected asset '+url,result.status===200&&digest(Buffer.from(result.bytes))===digest(fs.readFileSync(path.join(repo,file))));
 }
 /** Read one document using the real authenticated browser session. */
 const getLog = query => page.evaluate(async query=>{
  const response=await fetch('/api/v1/authority/log'+query);
  return {status:response.status, body:await response.json()};
 }, query);
 const historyQuery = '?position=all&order=oldest&limit=1&q=identity&minimum_level=warn';
 const firstPage=await getLog(historyQuery);
 const firstRead=firstPage.body;
 check('history enumerates two refused operations',firstPage.status===200&&firstRead.total===2&&firstRead.lines.length===1);
 const nextRead=await getLog(historyQuery+'&offset=1&snapshot='+firstRead.snapshot);
 check('real snapshot reads a distinct next page',nextRead.status===200&&nextRead.body.offset===1&&
  JSON.parse(nextRead.body.lines[0]).operation!==JSON.parse(firstRead.lines[0]).operation);
 const invalidQuery='?position=all&window_bytes=1&q=private-filter-sentinel';
 expectedRefusals.set(origin+'/api/v1/authority/log'+invalidQuery,400);
 const invalid=await getLog(invalidQuery);
 check('invalid window refuses',invalid.status===400&&invalid.body.error==='authority-log-filter-invalid');
 const sameSnapshot=await getLog(historyQuery+'&snapshot='+firstRead.snapshot);
 check('invalid read leaves the journal snapshot intact',sameSnapshot.status===200&&sameSnapshot.body.snapshot===firstRead.snapshot);
 const startup=await getLog('?position=all&q=authority-service');
 check('Rust daemon startup is visible in the application journal',startup.status===200&&startup.body.lines.some(raw=>{
  const row=JSON.parse(raw);
  return row.component==='swarm-authority'&&row.operation==='serve'&&row.phase==='listening'&&row.level==='info';
 }));
 await fetch(origin+'/api/v1/mission', {redirect:'manual'});
 const staleQuery=historyQuery+'&offset=1&snapshot='+firstRead.snapshot;
 expectedRefusals.set(origin+'/api/v1/authority/log'+staleQuery,409);
 const expired=await getLog(staleQuery);
 check('append invalidates a real snapshot',expired.status===409&&expired.body.error==='authority-log-snapshot-stale');
 const logStderr=fs.readFileSync(path.join(output,'daemon.stderr'),'utf8');
 const refusals=logStderr.split('\n').filter(line=>line.startsWith('{')).map(JSON.parse)
  .filter(row=>row.component==='swarm-authority'&&row.operation==='read-history');
 check('Rust reports structured invalid and stale reads out of band',[
  [400,'authority-log-filter-invalid'],[409,'authority-log-snapshot-stale'],
 ].every(([status,code])=>refusals.some(row=>row.status===status&&row.code===code&&row.level==='warn'&&row.phase==='refused')));
 check('diagnostics omit filter text and the snapshot value',!logStderr.includes('private-filter-sentinel')&&!logStderr.includes(firstRead.snapshot));
 await page.goto(origin+'/native-runtime.html?lang=fr');
 await page.waitForSelector('[data-native-view="admin"]');
 await page.click('[data-native-view="admin"]');
 await page.waitForSelector('[data-authority-log-jump]',{visible:true});
 await page.click('[data-authority-log-jump]');
 const query='[data-authority-log-search]';
 await page.waitForSelector(query,{visible:true});
 await page.waitForSelector('[data-authority-log-rotation] progress',{visible:true});
 check('rotation metrics appear in the authenticated console',await page.$eval('[data-authority-log-rotation] progress',bar=>bar.value>0&&bar.max>=bar.value));
 const logged = await page.evaluate(async () => {
  const response=await fetch('/api/v1/authority/log?level=warn&q=identity');
  return {status:response.status, rows:(await response.json()).lines.map(raw=>JSON.parse(raw))};
 });
 check('refusal was recorded by real daemon',logged.status===200&&logged.rows.some(row=>row.phase==='identity'&&row.status===401&&row.operation==='mission'));
 await page.type(query,'identity');
 await page.select('[data-authority-log-level]','warn');
 await page.focus(query);await page.keyboard.press('Enter');
 await page.waitForFunction(()=>[...document.querySelectorAll('[data-authority-log-results] tbody tr')].some(row=>row.cells[1].textContent==='warn'&&row.cells[3].textContent==='identity'&&row.cells[5].textContent==='401'));
 check('refusal visible through real authenticated UI',true);
 await page.$eval('[data-authority-log-options]',options=>{options.open=true});
 await page.select('[data-authority-log-position]','all');
 await page.$eval('[data-authority-log-rows]',field=>{field.value='1';field.form.requestSubmit()});
 await page.waitForFunction(()=>!document.querySelector('[data-authority-log-next-page]').disabled);
 await page.click('[data-authority-log-next-page]');
 await page.waitForFunction(()=>document.querySelector('[data-authority-log-status]').textContent.includes('2 à 2'));
 check('real browser pages retained events',true);
 await page.click('[data-authority-log-next-match]');
 check('real occurrence is focused',await page.evaluate(()=>document.activeElement.classList.contains('authority-log-match')));
 await page.select('[data-authority-log-source]','archives');
 await page.$eval(query,field=>field.form.requestSubmit());
 await page.waitForFunction(()=>document.querySelector('[data-authority-log-status]').textContent.includes('0 ligne'));
 check('empty archives are distinguished from a filtered-out result',await page.$eval('[data-authority-log-results]',e=>e.textContent.includes('Aucune archive conservée.')));
 await page.click('[data-authority-log-filter-source]');
 await page.waitForFunction(()=>document.querySelector('[data-authority-log-results] tbody tr'));
 check('removing the source chip retained the other filters',await page.$eval(query,field=>field.value==='identity')&&
  await page.$eval('[data-authority-log-level]',field=>field.value==='warn'));
 await page.select('[data-authority-log-source]','active');
 await page.$eval(query,field=>field.form.requestSubmit());
 await page.waitForFunction(()=>document.querySelector('[data-authority-log-results] tbody tr'));
 check('active log origin is visible',await page.$eval('[data-authority-log-results] tbody tr',row=>row.cells[7].textContent==='Journal actif'));
 const threshold='[data-authority-log-setting="max_bytes"]';
 await page.$eval(threshold,control=>{control.closest('details').open=true;control.value='256'});
 await page.$eval('[data-authority-log-save]',button=>button.click());
 await page.waitForFunction(()=>document.querySelector('[data-authority-log-dialog] details [role="status"]').textContent.includes('enregistrés'));
 const saved=await page.evaluate(async()=>(await fetch('/api/v1/authority/config')).json());
 check('rotation policy saved through the UI',saved.config.diagnostics.max_bytes===256);
 // Six synthetic, fixed-size relay events cross the fixture's 256-byte trigger.
 for(let index=0;index<6;index++){
  const refused=await fetch(origin+'/api/v1/mission',{redirect:'manual'});
  if(refused.status!==401)throw Error('fixture stopped producing authorization refusals');
 }
 const archive=await getLog('?position=all&source=archives&limit=1&minimum_level=warn');
 check('rotation retains archived bytes',archive.status===200&&archive.body.archives>0&&
  archive.body.rotation.archived_bytes>0&&archive.body.rotation.last_rotation_at>0);
 await page.select('[data-authority-log-source]','archives');
 await page.$eval(query,field=>field.form.requestSubmit());
 await page.waitForFunction(()=>document.querySelector('[data-authority-log-results] tbody tr')?.cells[7].textContent.startsWith('Archive du'));
 check('real archive origin and rotation are visible',true);
 await page.click('[data-authority-log-refresh]');
 await page.waitForFunction(()=>document.querySelector('[data-authority-log-results]').getAttribute('aria-busy')==='false');
 check('refresh retained criteria',await page.$eval(query,input=>input.value==='identity'));
 for (const lang of ['fr','en']) {
  if (lang==='en') {
   await page.goto(origin+'/native-runtime.html?lang=en');
   await page.click('[data-native-view="admin"]'); await page.waitForSelector('[data-authority-log-jump]',{visible:true}); await page.click('[data-authority-log-jump]');
  }
  const selector='[data-authority-log-refresh]';
  await page.waitForSelector(selector,{visible:true});
  check('render language '+lang,(await page.$eval(selector,element=>element.textContent))===(lang==='en'?'Refresh':'Actualiser'));
  for (const theme of ['etat','sombre']) {
   await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);
   await page.setViewport({width:390,height:844});
   check('no page overflow '+lang+' '+theme,await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   const view=await page.$('[data-authority-log-dialog]');
   await view.screenshot({path:path.join(output,lang+'-'+theme+'.png')});
  }
 }
 await page.keyboard.press('Escape');
 await page.waitForFunction(()=>!document.querySelector('[data-authority-log-dialog]'));
 check('Escape returns to Admin shortcut',await page.evaluate(()=>document.activeElement.hasAttribute('data-authority-log-jump')));
 check('no page errors',!errors.some(error=>error.startsWith('pageerror:')||error.startsWith('console:')));
 success=true;
}catch(error){console.error(error.stack||error);process.exitCode=1}
finally{
 if(browser)await browser.close();
 for(const child of children)child.kill('SIGINT');
 fs.writeFileSync(path.join(output,'checks.json'),JSON.stringify({success,checks,errors},null,2)+'\n');
 if(success)console.log('PASS AUTHORITY_APPLICATION_LOGS_AUTHENTICATED '+checks.length);
}})();
