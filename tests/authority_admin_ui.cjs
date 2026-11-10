'use strict';
/** B8/B9 browser check of the "Comptes et accès" Admin card on the real path:
 * real `swarm-authority serve` in front of a real engine (`ENGINE_BIN`, built
 * with the embedded `web/native-authority-admin.js`) in `authority` mode.
 * Sign-in through the daemon `/login`, Admin view, then: account creation,
 * API key creation (secret shown once), permission matrix with a new custom role,
 * audit view; FR/EN × both themes, 390 px mobile overflow, focus restoration.
 * Screenshots and checks go to OUTPUT (fresh). Fixtures: protected roots under
 * ~/.local/state/swarm-protected-fixtures, engine Root under .swarm-fixtures;
 * never /tmp. No global watchdog: waits end on page errors.
 * Usage: node tests/authority_admin_ui.cjs ENGINE_BIN DAEMON_BIN OUTPUT */
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),net=require('node:net'),assert=require('node:assert/strict'),{spawn,spawnSync}=require('node:child_process'),puppeteer=require('puppeteer');
if(process.argv.length!==5)throw Error('Usage: authority_admin_ui.cjs ENGINE_BIN DAEMON_BIN OUTPUT');
const [engineBin,daemonBin,output]=process.argv.slice(2).map(p=>path.resolve(p));
if(fs.existsSync(output))throw Error('Fresh output required');fs.mkdirSync(output,{recursive:true});
const repo=path.resolve(__dirname,'..'),area=path.join(repo,'.swarm-fixtures');fs.mkdirSync(area,{recursive:true});
const scratch=fs.mkdtempSync(path.join(area,'authority-ui-'));fs.chmodSync(scratch,0o700);
const protectedArea=path.join(os.homedir(),'.local/state/swarm-protected-fixtures');fs.mkdirSync(protectedArea,{recursive:true,mode:0o700});fs.chmodSync(protectedArea,0o700);
const dir=fs.mkdtempSync(path.join(protectedArea,'ui-'));fs.chmodSync(dir,0o700);
const env={...process.env,TMPDIR:scratch},checks=[],errors=[],children=[];
const check=(name,ok,detail)=>{checks.push({name,ok:!!ok,detail});if(!ok)throw Error('check failed: '+name+(detail?' '+JSON.stringify(detail):''))};
function run(bin,args,input){const r=spawnSync(bin,args,{env,input,encoding:'utf8'});if(r.status!==0)throw Error(args.join(' ')+': '+r.stderr);return r.stdout?JSON.parse(r.stdout):null}
const free=()=>new Promise(res=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>res(p))})});
async function listening(port){for(;;){const ok=await new Promise(res=>{const c=net.connect(port,'127.0.0.1',()=>{c.end();res(true)});c.on('error',()=>res(false))});if(ok)return;await new Promise(r=>setTimeout(r,50))}}
/** Wait for the admin card to finish rendering (lazy group fetches complete).
 * Bound 60 s justified by observation: cold Chrome + loaded daemon renders in
 * ≤26 s; RAF default polling was flaky under load, 500 ms polling is stable.
 * On timeout the diagnostic keeps the rendered state (details/save/config) so
 * the failure names its cause instead of a bare "Waiting failed". */
async function cardReady(pg){try{await pg.waitForFunction(()=>[...document.querySelectorAll('#native-authority-admin details')].every(d=>!d.open||d.querySelector('table,.notice.alert,div > p'))&&document.querySelector('[data-authority-policy-save]')&&document.querySelector('[data-authority-config]'),{timeout:60000,polling:500})}catch(e){throw Error('card-wait url='+pg.url()+' state='+JSON.stringify(await pg.evaluate(()=>({details:[...document.querySelectorAll('#native-authority-admin details')].map(d=>({t:d.querySelector('summary')?.textContent,open:d.open,kids:d.querySelector('table')?'table':(d.querySelector('.notice.alert')?'alert':(d.querySelector('div > p')?'p':'none'))})),save:!!document.querySelector('[data-authority-policy-save]'),cfg:!!document.querySelector('[data-authority-config]')}))))}}
(async()=>{let browser,success=false;try{
 const prep=run(daemonBin,['prepare','--dir',dir,'--controller-uid','4242','--admin','ana','--json']);
 fs.writeFileSync(path.join(dir,'configuration/authority.json'),JSON.stringify({schema_version:1,session_absolute_seconds:3600,session_idle_seconds:1800,argon2:{memory_kib:8,passes:1,parallelism:1},relay_body_max_bytes:16777216}));
 const base=['--declaration',prep.declaration,'--pin',prep.pin];
 run(daemonBin,['init','--admin','ana','--display','Ana Admin',...base,'--json'],'ana pass\n');
 const root=path.join(scratch,'root');fs.mkdirSync(root,{mode:0o700});
 const cli=(args,input)=>run(engineBin,['--root',root,...args],input);
 cli(['init','--json']);const shown=cli(['engine-settings','show','--json']);const values={...shown.values,web_access_mode:'authority',web_authority_public_key:prep.public_key};
 const change={schema_version:shown.schema_version,event_id:'authority-ui',expected_revision:shown.revision,values,reason:'Authority Admin UI fixture',preview_token:''};const input=path.join(root,'change.json');
 fs.writeFileSync(input,JSON.stringify(change));change.preview_token=cli(['engine-settings','preview','--input',input,'--json']).preview_token;fs.writeFileSync(input,JSON.stringify(change));cli(['engine-settings','apply','--input',input,'--json']);
 const [enginePort,frontPort]=[await free(),await free()];
 children.push(spawn(engineBin,['--root',root,'web','127.0.0.1:'+enginePort],{env,stdio:'ignore'}));await listening(enginePort);
 children.push(spawn(daemonBin,['serve',...base,'--listen','127.0.0.1:'+frontPort,'--engine','127.0.0.1:'+enginePort],{env,stdio:['ignore','ignore','pipe']}));await listening(frontPort);
 const origin='http://127.0.0.1:'+frontPort;
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',userDataDir:path.join(scratch,'chrome'),args:['--no-sandbox']});
 const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 page.on('pageerror',e=>errors.push('pageerror: '+e.message));page.on('console',m=>{if(m.type()==='error')errors.push('console: '+m.text())});page.on('response',r=>{if(r.status()>=400)errors.push('http: '+r.status()+' '+r.request().method()+' '+r.url())});
 const failOnError=()=>{if(errors.some(e=>e.startsWith('pageerror')))throw Error(errors.join('\n'))};
 await page.setViewport({width:1440,height:960});
 // Public pages: logo, forgotten password; FR/EN, light/dark, mobile.
 for(const scheme of ['light','dark'])for(const lang of ['fr','en']){await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:scheme}]);
  await page.goto(origin+'/login?lang='+lang);check('logo-'+scheme+'-'+lang,await page.$eval('header img',i=>i.complete&&i.naturalWidth>0));await page.screenshot({path:path.join(output,'login-'+lang+'-'+scheme+'.png')});
  await Promise.all([page.waitForNavigation(),page.click('main > p a')]);await page.screenshot({path:path.join(output,'forgot-'+lang+'-'+scheme+'.png')});}
 await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:'light'}]);
 await page.goto(origin+'/login/forgot?lang=fr');await page.type('input[name=subject]','ana');await Promise.all([page.waitForNavigation(),page.click('button[type=submit]')]);
 check('forgot-confirmation',await page.$eval('[role=status]',e=>e.textContent.includes('Si ce compte existe')));await page.screenshot({path:path.join(output,'forgot-sent-fr.png')});
 await page.setViewport({width:390,height:844});await page.goto(origin+'/login');check('login-mobile-no-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:path.join(output,'login-mobile.png')});await page.setViewport({width:1440,height:960});
 await page.goto(origin+'/login');await page.type('input[name=subject]','ana');await page.type('input[name=password]','ana pass');
 await Promise.all([page.waitForNavigation(),page.click('button[type=submit]')]);check('signed-in',!page.url().includes('/login'),page.url());
 const open=async(lang)=>{await page.goto(origin+'/native-runtime.html?lang='+lang);await page.waitForFunction(()=>!document.getElementById('native-authority-admin').hidden&&document.querySelector('#native-authority-admin table'));failOnError();await page.click('[data-native-view="admin"]');await page.waitForSelector('#native-admin-view:not([hidden])')};
 // Main cockpit menu: "Utilisateurs et accès" view (Wattson-style admin).
 await page.goto(origin+'/?lang=fr');await page.waitForSelector('#tabs [data-view="authority"]');
 await page.waitForFunction(()=>location.search.includes('view='));await page.$eval('#tabs [data-view="authority"]',b=>b.closest('details').open=true);await page.click('#tabs [data-view="authority"]');await page.waitForFunction(()=>!document.getElementById('authority').hidden&&document.querySelector('#authority table'));
 await page.goto(origin+'/?lang=fr&view=authority');await page.waitForFunction(()=>!document.getElementById('authority').hidden&&document.querySelector('#authority table'));check('cockpit-direct-link',true);check('no-unconnected-note',await page.$$eval('#authority > p.notice',n=>n.length===0));
 check('cockpit-menu-entry',await page.$eval('#tabs [data-view="authority"]',b=>!b.hidden&&b.textContent==='Utilisateurs et accès'));
 await page.waitForSelector('#authority [data-authority-add]');await page.waitForSelector('#authority [data-authority-key-create]');
 const jumps=await page.$$('#authority [data-authority-jump]');check('cockpit-jump-bar',jumps.length>=5,jumps.length);
 await jumps[1].click();check('jump-opens-one',await page.$$eval('#authority details.engine-settings-group',ds=>ds.filter(d=>d.open).length===1));
 await page.screenshot({path:path.join(output,'fr-cockpit-users-access.png'),fullPage:false});
 await open('fr');
 await page.waitForSelector('[data-authority-add]');check('reset-request-shown',await page.$eval('#native-authority-admin',e=>e.textContent.includes('réinitialisation demandée')));
 const who=await page.$eval('.native-session-user',e=>e.textContent);check('header-user',who==='Ana Admin (ana)',who);
 // Account creation through the dialog, focus back on the opener.
 await page.$$eval('#native-authority-admin details',ds=>ds.forEach(d=>d.open=true));await page.waitForSelector('[data-authority-add]');
 await page.focus('[data-authority-add]');await page.keyboard.press('Enter');await page.waitForSelector('dialog[open][data-authority-dialog]');
 const dlg='dialog[open][data-authority-dialog] ';
 const inputs=await page.$$(dlg+'input[type=text]');await inputs[0].type('bob');await inputs[1].type('Bob Viewer');
 await page.click(dlg+'input[value=viewer]');const pw=await page.$$(dlg+'input[type=password]');await pw[0].type('bob pass');await pw[1].type('bob pass');
 await page.screenshot({path:path.join(output,'fr-create-account.png')});
 await page.click(dlg+'button[type=submit]');await page.waitForFunction(()=>!document.querySelector('dialog[open]')&&[...document.querySelectorAll('#native-authority-admin td')].some(td=>td.textContent==='bob'));
 check('account-created',true);
 const focused=await page.evaluate(()=>document.activeElement?.dataset?.authorityAdd!==undefined||document.activeElement?.closest('#native-authority-admin')!==null);check('focus-restored',focused);
 // Refusal stays visible with the input kept: duplicate account.
 await page.click('[data-authority-add]');await page.waitForSelector(dlg);const again=await page.$$(dlg+'input[type=text]');await again[0].type('bob');await again[1].type('Bob');await page.click(dlg+'input[value=viewer]');const pw2=await page.$$(dlg+'input[type=password]');await pw2[0].type('x');await pw2[1].type('x');
 await page.click(dlg+'button[type=submit]');await page.waitForFunction(()=>document.querySelector('dialog[open] .notice.alert')?.textContent.includes('account-exists'));
 check('refusal-visible',await page.$eval(dlg+'input[type=text]',e=>e.value==='bob'));
 await page.screenshot({path:path.join(output,'fr-refusal.png')});await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('dialog[open]'));
 // API key: secret shown once in a follow-up dialog.
 await page.click('[data-authority-key-create]');await page.waitForSelector(dlg);const kin=await page.$$(dlg+'input[type=text]');await kin[0].type('ci script');await kin[1].type('30');
 await page.click(dlg+'button[type=submit]');await page.waitForFunction(()=>document.querySelector('dialog[open] input[readonly]')?.value.startsWith('sak1-'));
 await page.screenshot({path:path.join(output,'fr-key-secret.png')});const secret=await page.$eval('dialog[open] input[readonly]',e=>e.value);
 await page.click('dialog[open] button[type=submit]');await page.waitForFunction(()=>!document.querySelector('dialog[open]'));
 const listed=await page.$eval('#native-authority-admin',e=>e.textContent);check('key-listed-without-secret',listed.includes('ci script')&&!listed.includes(secret.split('.')[1]));
 const viaKey=await fetch(origin+'/api/v1/native-version',{headers:{Authorization:'Bearer '+secret}});check('key-works',viaKey.status===200,viaKey.status);
 // Habilitations: matrix (admin locked), new custom role in the draft, saved,
 // then offered when creating an account.
 await page.$$eval('#native-authority-admin details',ds=>ds.forEach(d=>d.open=true));await page.waitForSelector('[data-authority-matrix]');
 // Schema 3 matrix: one row per catalog family permission (14) + per menu
 // view key (16) = 30 locked admin checkboxes.
 check('admin-column-locked',await page.$$eval('[data-authority-matrix] input[data-role="admin"]',cs=>cs.length===30&&cs.every(c=>c.checked&&c.disabled)),{count:await page.$$eval('[data-authority-matrix] input[data-role="admin"]',cs=>cs.length),bad:await page.$$eval('[data-authority-matrix] input[data-role="admin"]',cs=>cs.filter(c=>!c.checked||!c.disabled).map(c=>c.dataset.permission))});
 await page.click('[data-authority-role-new]');await page.waitForSelector(dlg);const rin=await page.$$(dlg+'input[type=text]');await rin[0].type('auditor');await rin[1].type('Auditeur');
 await page.click(dlg+'button[type=submit]');await page.waitForFunction(()=>!document.querySelector('dialog[open]')&&document.querySelector('[data-authority-matrix] input[data-role="auditor"][data-permission="audit.read"]'));
 await page.click('[data-authority-matrix] input[data-role="auditor"][data-permission="audit.read"]');await page.click('[data-authority-matrix] input[data-role="auditor"][data-permission="cockpit.read"]');
 await page.screenshot({path:path.join(output,'fr-habilitations-draft.png'),fullPage:true});
 await page.click('[data-authority-policy-save]');await page.waitForFunction(()=>document.querySelector('#native-authority-admin')?.textContent.includes('Habilitations enregistrées'));
 const pol=await (await fetch(origin+'/api/v1/authority/policy',{headers:{Cookie:(await page.cookies()).map(c=>c.name+'='+c.value).join('; ')}})).json();
 check('role-saved',pol.policy.roles.some(r=>r.key==='auditor')&&pol.policy.grants.some(g=>g.role==='auditor'&&g.permissions.includes('audit.read')),pol.policy.grants);
 await page.click('[data-authority-add]');await page.waitForSelector(dlg);check('role-offered-for-accounts',await page.$eval(dlg,d=>d.textContent.includes('Auditeur')));await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('dialog[open]'));
 // Audit group lazily built on open.
 await page.$$eval('#native-authority-admin details',ds=>ds.forEach(d=>d.open=true));await page.waitForSelector('[data-audit-tab="connections"]');
 await page.$$eval('#native-authority-admin button',bs=>bs.find(b=>b.textContent==='Filtrer').click());
 await page.waitForSelector('[data-audit-session]');check('audit-connections',await page.$$eval('[data-audit-session]',b=>b.length>=1));
 await page.click('[data-audit-session]');await page.waitForFunction(()=>document.querySelector('#native-authority-admin')?.textContent.includes('/api/v1/authority/accounts'));check('audit-shows-actions',true);
 await page.screenshot({path:path.join(output,'fr-audit-detail.png')});
 await page.click('[data-audit-tab="dashboard"]');await page.waitForFunction(()=>document.querySelector('#native-authority-admin')?.textContent.includes('Activité par jour'));check('audit-dashboard',true);
 await page.screenshot({path:path.join(output,'fr-audit-dashboard.png')});
 await page.click('[data-audit-tab="purge"]');await page.waitForSelector('[data-audit-purge]');await page.click('[data-audit-purge]');await page.waitForFunction(()=>document.querySelector('#native-authority-admin .notice.alert')?.textContent.includes('authority-purge-days-invalid'));check('purge-needs-days',true);
 await page.click('[data-audit-tab="connections"]');
 // Languages × themes, full card screenshots; English must translate all fixed text.
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  await open(lang);await page.click('[data-theme-choice="'+theme+'"]');await page.$$eval('#native-authority-admin details',ds=>ds.forEach(d=>d.open=true));
  await cardReady(page);
  const card=await page.$('#native-authority-admin');await card.screenshot({path:path.join(output,lang+'-'+theme+'-card.png')});
  if(lang==='en'){const text=await page.$eval('#native-authority-admin',e=>e.innerText);for(const fr of ['Comptes et accès','Journal d’accès','Habilitations','Réglages du démon','Ajouter un compte','Créer une clé','Révoquer'])check('en-no-french:'+fr,!text.includes(fr))}
 }
 // Mobile: no global horizontal overflow (tables scroll locally).
 await page.setViewport({width:390,height:844});await open('fr');await page.$$eval('#native-authority-admin details',ds=>ds.forEach(d=>d.open=true));await cardReady(page);
 const overflow=await page.evaluate(()=>document.documentElement.scrollWidth-window.innerWidth);check('mobile-no-overflow',overflow<=0,overflow);
 await page.screenshot({path:path.join(output,'fr-mobile.png'),fullPage:true});
 failOnError();success=true;
}catch(e){checks.push({name:'error',ok:false,detail:String(e.stack||e)});process.exitCode=1}finally{
 if(browser)await browser.close().catch(()=>{});for(const c of children)c.kill('SIGINT');
 fs.writeFileSync(path.join(output,'checks.json'),JSON.stringify({success,checks,errors},null,2)+'\n');
 if(success){fs.rmSync(scratch,{recursive:true,force:true});fs.rmSync(dir,{recursive:true,force:true})}
 console.log(JSON.stringify({success,checks:checks.length,failed:checks.filter(c=>!c.ok).map(c=>c.name),output}));
}})();
