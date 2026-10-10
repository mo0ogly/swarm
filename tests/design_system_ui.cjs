'use strict';
// Real server + isolated CLI fixture. Presentation checks never start an agent.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]||'bin/swarm'),out=path.resolve(process.argv[3]||'test-results/design-system');
fs.mkdirSync(out,{recursive:true});
const fixtureRoot=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-design-'));
function cli(args,data){const text=execFileSync(binary,['--root',fixtureRoot,'--json',...args,...(data?['--input','-']:[])],{input:data?JSON.stringify(data):undefined,encoding:'utf8'});return text?JSON.parse(text):null}
let sequence=0;const input=(revision,data)=>({schema_version:1,event_id:'design-'+(++sequence),expected_revision:revision,...data});
cli(['init']);let w=cli(['work','create'],input(0,{title:'Casa Pizza — Commandes et livraison',objective:'Relier la commande, la cuisine et la livraison dans un parcours accessible.',scope:'Fixture UI isolée. Aucun appel IA.',criteria:['Graphe et navigation lisibles'],next:'Vérifier les parcours.'})).work;
for(const [id,title,depends]of [['commande','Prendre la commande',[]],['cuisine','Préparer les pizzas',['commande']],['livraison','Organiser la livraison',['cuisine']],['verification','Vérifier le parcours clavier',['commande']]])w=cli(['task','add',w.id],input(w.revision,{id,title,depends,owner:'fixture',deliverable:'docs/'+id+'.md',criteria:['Vérification UI'],next:'Examiner le résultat.'})).work;
cli(['autonomy',w.id,'manuel']);
const app=spawn(binary,['--root',fixtureRoot,'web','127.0.0.1:0']);let browser,page;
const errors=[],networkErrors=[],checks=[],navigationAborts=[],inflight=new Set(),expectedNavigationAborts=new WeakSet();
function markNavigation(){for(const request of inflight)expectedNavigationAborts.add(request)}
async function navigateTo(url){markNavigation();return page.goto(url,{waitUntil:'domcontentloaded'})}
async function reloadPage(){markNavigation();return page.reload({waitUntil:'domcontentloaded'})}
async function destination(view){
 await page.$eval('[data-view="'+view+'"]',e=>{e.closest('details').open=true});
 await page.click('[data-view="'+view+'"]');
 await page.waitForFunction(v=>!document.getElementById(v).hidden,{},view);
 assert.equal(await page.$eval('[data-view="'+view+'"]',e=>e.getAttribute('aria-current')),'page');
 assert.equal(await page.$eval('[data-view="'+view+'"]',e=>e.closest('details').open),true);
}
async function fits(label){
 const geometry=await page.evaluate(()=>({viewport:innerWidth,width:document.documentElement.scrollWidth}));
 assert.ok(geometry.width<=geometry.viewport+1,label+': global overflow '+JSON.stringify(geometry));
}
async function colors(){
 return page.evaluate(()=>{
  function lum(hex){const c=hex.trim().replace('#','').match(/.{2}/g).map(v=>parseInt(v,16)/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4);return .2126*c[0]+.7152*c[1]+.0722*c[2]}
  function ratio(a,b){const x=lum(a),y=lum(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05)}
  const style=getComputedStyle(document.documentElement),pairs=[['texte','fond'],['titre','carte'],['texte-discret','fond'],['libelle','champ'],['info-encre','info-fond'],['succes-encre','succes-fond'],['attention-encre','attention-fond'],['alerte-encre','alerte-fond'],['action-encre','action-fond']];
  return pairs.map(([a,b])=>({pair:a+'/'+b,ratio:ratio(style.getPropertyValue('--wattson-'+a),style.getPropertyValue('--wattson-'+b))}));
 });
}
(async()=>{
 const session=await new Promise((resolve,reject)=>{let text='';const timer=setTimeout(()=>reject(Error('server startup timeout')),20000);app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}});app.on('exit',c=>reject(Error('server exited '+c)))});
 const base=new URL(session).origin;
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 page=await browser.newPage();page.on('dialog',dialog=>dialog.accept());page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
 page.on('request',r=>inflight.add(r));page.on('requestfinished',r=>inflight.delete(r));
 page.on('requestfailed',r=>{inflight.delete(r);if(r.failure()?.errorText==='net::ERR_ABORTED'&&expectedNavigationAborts.has(r))navigationAborts.push(new URL(r.url()).pathname);else networkErrors.push(r.url()+' '+r.failure()?.errorText)});
 page.on('response',r=>{if(r.status()>=400)networkErrors.push('HTTP '+r.status()+' '+r.url())});
 await page.setViewport({width:1440,height:1100});await navigateTo(session);await page.waitForSelector('#pilot-canvas');
 assert.equal(await page.$$('.nav-group').then(x=>x.length),5);
 assert.equal(await page.$eval('#graph',e=>e.firstElementChild.id),'mission-summary');
 assert.equal(await page.$eval('.graph-tools',e=>e.open),false);
 assert.equal(await page.$eval('.mission-reading',e=>e.open),false);
 assert.equal(await page.$$('#mission-results .mission-task').then(x=>x.length),0,'closed results avoid building the full task list');
 await page.focus('#mission-results>summary');await page.keyboard.press('Enter');
 await page.waitForFunction(()=>document.querySelectorAll('#mission-results .mission-task').length===4);
 await page.$eval('#mission-results [data-mission-detail="expected-commande"]',e=>e.open=true);
 await page.focus('#mission-results [data-mission-action="detail-commande"]');
 await page.evaluate(()=>{Mission.key='';Mission.render()});
 assert.equal(await page.$eval('#mission-results',e=>e.open),true);
 assert.equal(await page.$eval('#mission-results [data-mission-detail="expected-commande"]',e=>e.open),true);
 assert.equal(await page.evaluate(()=>document.activeElement.dataset.missionAction),'detail-commande');
 await page.focus('#mission-results>summary');await page.keyboard.press('Enter');
 await page.evaluate(()=>{Mission.key='';Mission.render()});
 assert.equal(await page.$$('#mission-results .mission-task').then(x=>x.length),0);
 await page.evaluate(()=>{Mission.key='';Mission.render()});
 await page.click('#mission-results>summary');
 await page.waitForFunction(()=>document.querySelectorAll('#mission-results .mission-task').length===4);
 assert.equal(await page.$eval('#mission-results [data-mission-detail="expected-commande"]',e=>e.open),true,'closed refreshes retain nested disclosures for reopening');
 await page.click('#mission-results>summary');
 checks.push('results populated at first opening; all tasks available; expanded results/nested details/action focus survive refresh; closed refresh defers construction');
 const routes=await page.$$eval('.graph-arete',es=>es.map(e=>e.getAttribute('d')));
 await reloadPage();await page.waitForSelector('.graph-noeud');
 assert.deepEqual(await page.$$eval('.graph-arete',es=>es.map(e=>e.getAttribute('d'))),routes,'tab cache preserves every routed dependency');
 await page.evaluate(()=>{const key='swarm-pilot-layout:v1',cache=JSON.parse(sessionStorage.getItem(key));cache.nodes[0].x=-1;sessionStorage.setItem(key,JSON.stringify(cache))});
 await reloadPage();await page.waitForSelector('.graph-noeud');
 assert.deepEqual(await page.$$eval('.graph-arete',es=>es.map(e=>e.getAttribute('d'))),routes,'corrupt cache recomputes the real graph');
 assert.ok(await page.evaluate(()=>JSON.parse(sessionStorage.getItem('swarm-pilot-layout:v1')).nodes[0].x>=0));
 checks.push('real tab geometry cache survives reload with exact routes; corrupted positions recompute without browser errors');
 for(const view of ['manage','brainstorm','agents','conduite','tasks','decisions','logs','budget','resume','automation','providers','admin'])await destination(view);
 checks.push('all 12 destinations accessible in simplified mode; active group and route identity');
 const connectionData=await page.evaluate(()=>api('/api/v1/providers/connections'));
 let releaseConnections;const connectionGate=new Promise(resolve=>releaseConnections=resolve);
 await page.setRequestInterception(true);
 const loadingInterceptor=async r=>{if(r.interceptResolutionState().action==='disabled')return;if(r.url().endsWith('/api/v1/providers/connections')){await connectionGate;await r.respond({status:200,contentType:'application/json',body:JSON.stringify(connectionData)})}else await r.continue()};
 page.on('request',loadingInterceptor);await destination('providers');
 assert.equal(await page.$eval('#connections-add',e=>e.disabled),true,'add action waits for connection data');
 await page.click('#connections-add');assert.equal(await page.$eval('#modal',e=>e.open),false);
 releaseConnections();await page.waitForFunction(()=>!document.getElementById('connections-add').disabled);
 await page.click('#connections-add');await page.waitForSelector('#modal[open]');await page.keyboard.press('Escape');
 page.off('request',loadingInterceptor);await page.setRequestInterception(false);
 checks.push('delayed connections loading disables the action, then enables a working dialog');
 await destination('conduite');
 // Keyboard disclosure and persistence over reload and periodic renderer updates.
 await page.focus('.graph-tools>summary');await page.keyboard.press('Enter');
 assert.equal(await page.$eval('.graph-tools',e=>e.open),true);
 await reloadPage();await page.waitForSelector('#pilot-canvas');
 assert.equal(await page.$eval('.graph-tools',e=>e.open),true);
 await page.click('.graph-tools>summary');
 await page.focus('[data-nav-group="configuration"]>summary');
 const wasOpen=await page.$eval('[data-nav-group="configuration"]',e=>e.open);await page.keyboard.press('Enter');
 assert.equal(await page.$eval('[data-nav-group="configuration"]',e=>e.open),!wasOpen);
 checks.push('keyboard accordions; graph disclosure persisted across reload');
 assert.equal(await page.$$eval('.graph-arete',es=>es.every(e=>e.tagName==='path'&&e.getAttribute('d').includes('Q'))),true);
 const contrasts={};
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  await navigateTo(base+'/?lang='+lang+'&view=conduite');await page.waitForSelector('#pilot-canvas');
  await page.evaluate(t=>{setTheme(t);for(const group of document.querySelectorAll('[data-nav-group]'))group.open=group.dataset.navGroup==='conduite'},theme);
  const pairs=await colors();for(const pair of pairs)assert.ok(pair.ratio>=4.5,theme+' '+pair.pair+': '+pair.ratio);
  contrasts[lang+'-'+theme]=pairs;
  assert.match(await page.$eval('.graph-tools>summary',e=>e.textContent),lang==='fr'?/Affichage/:/Graph display/);
  assert.equal(await page.$eval('#title',e=>e.textContent),w.title);
  const expanded=await page.$eval('#pilot-canvas',e=>e.getBoundingClientRect().width);
  const graphState=await page.evaluate(()=>{globalThis.designGraph=document.querySelector('.graph-svg');return {zoom:Pilot.state.zoom,selection:Pilot.state.selection,revision:snapshot.work.revision}});
  await page.focus('#nav-collapse');await page.keyboard.press('Enter');
  await page.waitForFunction(()=>document.querySelector('#pilot-canvas').getBoundingClientRect().width>0);
  assert.equal(await page.$eval('#nav-collapse',e=>e.getAttribute('aria-expanded')),'false');
  assert.equal(await page.$eval('#nav-collapse',e=>e.textContent.trim()),lang==='fr'?'→Ouvrir le menu':'→Open menu');
  assert.equal(await page.$eval('.rail',e=>getComputedStyle(e).display),'none');
  assert.ok(await page.$eval('#pilot-canvas',e=>e.getBoundingClientRect().width)>expanded+200,'collapsing the rail gives its width to the graph');
  assert.equal(await page.evaluate(()=>document.activeElement.id),'nav-collapse');
  assert.deepEqual(await page.evaluate(()=>({zoom:Pilot.state.zoom,selection:Pilot.state.selection,revision:snapshot.work.revision})),graphState);
  assert.equal(await page.evaluate(()=>document.querySelector('.graph-svg')===globalThis.designGraph),true,'collapse keeps the mounted graph');
  await fits('collapsed cockpit '+lang+' '+theme);
  await page.screenshot({path:path.join(out,'cockpit-collapsed-'+lang+'-'+theme+'.png'),fullPage:true});
  await reloadPage();await page.waitForSelector('.graph-noeud');
  assert.equal(await page.$eval('.rail',e=>getComputedStyle(e).display),'none','desktop preference survives reload');
  await page.setViewport({width:390,height:1100});
  await page.click('#nav-toggle');
  assert.notEqual(await page.$eval('.rail',e=>getComputedStyle(e).display),'none','desktop collapse does not hide the mobile menu');
  await page.keyboard.press('Escape');
  await page.setViewport({width:1440,height:1100});
  assert.equal(await page.$eval('.rail',e=>getComputedStyle(e).display),'none','returning to desktop restores the collapsed rail');
  await page.focus('#nav-collapse');await page.keyboard.press('Space');
  assert.equal(await page.$eval('#nav-collapse',e=>e.getAttribute('aria-expanded')),'true');
  assert.equal(await page.$eval('#nav-collapse',e=>e.textContent.trim()),lang==='fr'?'←Replier le menu':'←Collapse menu');
  assert.notEqual(await page.$eval('.rail',e=>getComputedStyle(e).display),'none');
  assert.equal(await page.$eval('[data-nav-group="conduite"]',e=>e.open),true);
  await page.screenshot({path:path.join(out,'cockpit-'+lang+'-'+theme+'.png'),fullPage:true});
  for(const width of [1024,760,390,320]){await page.setViewport({width,height:1100});await fits('cockpit '+lang+' '+theme+' '+width)}
  await page.click('#nav-toggle');await page.focus('[data-nav-group="missions"]>summary');await page.keyboard.press('Escape');
  assert.equal(await page.$eval('#nav-toggle',e=>e.getAttribute('aria-expanded')),'false');
  assert.equal(await page.evaluate(()=>document.activeElement.id),'nav-toggle');
  await page.click('#nav-toggle');await destination('providers');
  assert.equal(await page.$eval('#nav-toggle',e=>e.getAttribute('aria-expanded')),'false');
  await fits('providers mobile');
  await page.setViewport({width:1440,height:1100});
  await page.waitForFunction(()=>AIConnections.state);await page.click('#connections-add');await page.waitForSelector('#modal[open]');
  await page.screenshot({path:path.join(out,'connection-'+lang+'-'+theme+'.png')});await page.keyboard.press('Escape');
  assert.equal(await page.evaluate(()=>document.activeElement.id),'connections-add');
  await navigateTo(base+'/prepare.html?lang='+lang);await page.waitForSelector('#new-need');
  await page.waitForFunction(()=>!document.querySelector('#project-profile-open').disabled);
  assert.equal(await page.$eval('.project-profile-fold',e=>e.open),false);
  await page.screenshot({path:path.join(out,'prepare-'+lang+'-'+theme+'.png'),fullPage:true});
  await page.type('#new-need','Mon besoin conserve son texte français.');
  await page.select('#swarm-language',lang==='fr'?'en':'fr');
  assert.equal(await page.$eval('html',e=>e.lang),lang,'unsaved draft guards language navigation');
  assert.equal(await page.$eval('#new-need',e=>e.value),'Mon besoin conserve son texte français.');
  assert.equal(await page.$eval('#error',e=>e.hidden),false);
  for(const width of [760,390,320]){await page.setViewport({width,height:1100});await fits('preparation '+lang+' '+theme+' '+width)}
  await page.$eval('#new-need',e=>e.value='');
  await page.setViewport({width:1440,height:1100});
 }
 checks.push('FR/EN; themes; semantic contrast AA; 1440/1024/760/390/320px; mobile menu and escape; modal return focus; draft guard');
 checks.push('desktop rail collapse gives >200px to graph; keyboard and focus; graph identity/zoom/selection/revision retained; preference persists; mobile menu independent; FR/EN and both themes');
 await navigateTo(base+'/prepare.html?lang=fr');await page.waitForFunction(()=>!document.querySelector('#project-profile-open').disabled);
 await page.type('#new-title','Préparation de fixture');await page.type('#new-need','Rendre le parcours de commande accessible.');await page.click('#create');
 await page.waitForFunction(()=>!document.querySelector('#documents').hidden&&!document.querySelector('#chat').hidden);
 assert.equal(await page.$eval('.chat-settings',e=>e.open),false);
 await page.focus('.chat-settings>summary');await page.keyboard.press('Enter');await page.select('#chat-context','recent');
 assert.equal(await page.$eval('#chat-context',e=>e.value),'recent');
 await page.screenshot({path:path.join(out,'preparation-dialogue-fr.png'),fullPage:true});
 await reloadPage();await page.waitForFunction(()=>!document.querySelector('#documents').hidden);
 assert.equal(await page.$eval('.chat-settings',e=>e.open),true);
 assert.match(await page.$eval('#plain-editor',e=>e.value),/parcours de commande accessible/);
 checks.push('real preparation creation and saved document; AI/context accordion via keyboard and persisted disclosure; no AI request');
 await navigateTo(base+'/?view=providers&lang=en');await page.waitForFunction(()=>!document.querySelector('#providers').hidden);
 assert.equal(await page.$eval('[data-nav-group="configuration"]',e=>e.open),true,'direct link opens active group');
 // Incoming pending-decision fixture at the read-only snapshot boundary.
 const incoming=await page.evaluate(()=>api('/api/v1/snapshot?work='+encodeURIComponent(work)));
 incoming.decisions=[{id:'fixture-decision',kind:'execution',task_id:'commande',summary:'Fixture : résultat à examiner',evidence:'Preuve de fixture uniquement',resolved_at:''}];
 await page.setRequestInterception(true);
 const snapshotInterceptor=r=>{if(r.interceptResolutionState().action==='disabled')return;return r.url().includes('/api/v1/snapshot?')?r.respond({status:200,contentType:'application/json',body:JSON.stringify(incoming)}):r.continue()};
 page.on('request',snapshotInterceptor);
 await page.evaluate(()=>refresh(true));
 await page.$eval('[data-nav-group="conduite"]',e=>e.open=false);
 assert.equal(await page.$eval('#nav-interventions',e=>e.hidden),false);
 assert.equal(await page.$eval('#nav-decision-count',e=>e.textContent),'1');
 await page.click('#nav-interventions');
 assert.equal(await page.$eval('#decisions',e=>e.hidden),false);
 assert.equal(await page.$eval('[data-nav-group="conduite"]',e=>e.open),true);
 assert.match(await page.$eval('#decisions-list',e=>e.textContent),/résultat à examiner/);
 page.off('request',snapshotInterceptor);await page.setRequestInterception(false);
 // Actual refresh failure and recovery, with network double and unchanged saved data.
 await page.evaluate(async()=>{const original=globalThis.fetch;globalThis.fetch=async()=>{throw Error('Fixture offline')};try{await refresh(true)}finally{globalThis.fetch=original}});
 assert.equal(await page.$eval('#message',e=>e.hidden),false);
 assert.equal(await page.$eval('#title',e=>e.textContent),w.title);
 await page.evaluate(()=>refresh(true));
 assert.equal(await page.$eval('#message',e=>e.hidden),true);
 assert.equal(await page.$eval('#nav-interventions',e=>e.hidden),true);
 checks.push('direct-link group opening; pending decision reachable with closed group; failed refresh preserves data and recovers');
 const accessContext=await browser.createBrowserContext(),accessPage=await accessContext.newPage();
 accessPage.on('pageerror',e=>errors.push(e.message));accessPage.on('console',m=>{if(m.type()==='error')errors.push(m.text())});accessPage.on('response',r=>{if(r.status()>=400)networkErrors.push('login HTTP '+r.status()+' '+new URL(r.url()).pathname)});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  await accessPage.setViewport({width:1000,height:850});await accessPage.goto(base+'/login?lang='+lang+'&theme='+theme);
  await accessPage.waitForFunction(()=>document.querySelector('.access-logo')?.naturalWidth>0);
  assert.equal(await accessPage.$eval('html',e=>e.dataset.theme),theme);
  await accessPage.screenshot({path:path.join(out,'login-'+lang+'-'+theme+'.png')});
  await accessPage.setViewport({width:320,height:850});assert.equal(await accessPage.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 }
 await accessContext.close();checks.push('login before authentication: logo and design assets; FR/EN; themes; 320px; private API tested in Go');
 assert.deepEqual(errors,[]);assert.deepEqual(networkErrors,[]);
 const agents=cli(['agent','list',w.id]);assert.deepEqual(agents.agents,[]);
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks,contrasts,errors,networkErrors,navigationAborts,fixtureRoot,agents:0},null,2));
 console.log('PASS Swarm design system: '+checks.join('; '));
})().catch(async e=>{console.error(e);fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({error:e.stack,errors,networkErrors,fixtureRoot},null,2));if(page){await page.screenshot({path:path.join(out,'failure.png'),fullPage:true}).catch(()=>{});console.error(await page.evaluate(()=>document.body.innerText).catch(()=>''))}process.exitCode=1}).finally(async()=>{await browser?.close();app.kill()});
