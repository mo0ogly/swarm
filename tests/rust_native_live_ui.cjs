'use strict';
/** Live dashboard + harness window in Chrome over the real native server.
 * Login uses the real /login form; credentials arrive on stdin only and never
 * reach evidence. Checks FR/EN, both themes, 1440/390/320 widths (no horizontal
 * overflow), keyboard opening of the harness, cursor paging (103 rows > one page),
 * ANSI stripping and live following of a row appended during the run. No CSP or
 * console error is tolerated. Screenshots are retained for visual inspection.
 */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer'),{spawnSync}=require('node:child_process');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),output=path.resolve(cfg.output);fs.mkdirSync(output,{recursive:false});
// waitForFunction defaults to requestAnimationFrame polling, which never runs in
// background tabs; every wait polls on an interval instead (not a timeout).
const checks=[],problems=[],saturated=[];
// Refusals a step provokes on purpose (label|path|status), each consumed once:
// they are asserted by that step, not counted as problems.
const expected=[];
const base='http://'+cfg.address;
// Explicit 503 web-workers-saturated replies on live reads are the server's
// designed no-queue refusal under burst; they are counted separately and the
// windows must keep following. Every other error, console or CSP issue fails.
function watch(page,label){page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);page.on('console',m=>{if(m.type()!=='error'||/status of 503/.test(m.text()))return;const s=/status of (\d{3})/.exec(m.text());const i=s?expected.findIndex(e=>e.label===label&&e.status===+s[1]&&!e.console):-1;if(i>=0){expected[i].console=true;return}problems.push({label,console:m.text()})});page.on('pageerror',e=>problems.push({label,pageerror:String(e)}));page.on('response',async r=>{const p=new URL(r.url()).pathname;if(r.status()===503&&p.startsWith('/api/v1/live/')){let body='';try{body=await r.text()}catch{}if(body.trim()==='web-workers-saturated'){saturated.push({label,path:p});return}}if(r.status()>=400&&!p.startsWith('/login')){const i=expected.findIndex(e=>e.label===label&&e.path===p&&e.status===r.status()&&!e.response);if(i>=0){expected[i].response=true;return}problems.push({label,path:p,status:r.status()})}})}
/** Fixture-only: record a new failed agent so a fresh attention signal appears. */
function addFailedAgent(id,work){const body=JSON.stringify({progress:{},diagnostic:{agent_id:'',attempt_id:'',observed_errors:0,consecutive_error_limit:0,limit_reached:false,consecutive_error_limit_reached:false},recovery:{},limits:{},id,work_id:work,task_id:'t9-'+id,attempt_id:'',provider:'fixture',role:'nouveau',workspace:'/isolated/'+id,status:'failed',activity:'Échec simulé',started:'2026-10-21T10:00:00Z'});
 const r=spawnSync('python3',['-c','import sqlite3,sys;d=sqlite3.connect(sys.argv[1]);d.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,\'failed\',?,\'{}\')",(sys.argv[2],sys.argv[3],"t9-"+sys.argv[2],"/isolated/"+sys.argv[2],sys.argv[4].encode()));d.commit();d.close()',cfg.db,id,work,body]);assert.equal(r.status,0,String(r.stderr))}
/** Fixture-only: change one agent's recorded input tokens (a provider progress). */
function bumpUsage(id,delta){const r=spawnSync('python3',['-c','import sqlite3,sys,json;d=sqlite3.connect(sys.argv[1]);b=json.loads(d.execute("SELECT body FROM agents WHERE id=?",(sys.argv[2],)).fetchone()[0]);b["usage"]["input_tokens"]+=int(sys.argv[3]);d.execute("UPDATE agents SET body=? WHERE id=?",(json.dumps(b).encode(),sys.argv[2]));d.commit();d.close()',cfg.db,id,String(delta)]);assert.equal(r.status,0,String(r.stderr))}
function removeAgent(id){const r=spawnSync('python3',['-c','import sqlite3,sys;d=sqlite3.connect(sys.argv[1]);d.execute("DELETE FROM agents WHERE id=?",(sys.argv[2],));d.commit();d.close()',cfg.db,id]);assert.equal(r.status,0,String(r.stderr))}
function appendLog(message){const r=spawnSync('python3',['-c','import sqlite3,sys;d=sqlite3.connect(sys.argv[1]);d.execute("INSERT INTO agent_logs(agent_id,at,kind,message) VALUES(\'dev\',\'2026-10-21T10:05:00Z\',\'message\',?)",(sys.argv[2],));d.commit();d.close()',cfg.db,message]);assert.equal(r.status,0,String(r.stderr))}
/** Hover a node of the live overview; a periodic re-render may replace it
 * between lookup and hover (detached node), so look it up again — only that
 * error is retried. */
async function hoverStable(page,sel){for(;;){try{await page.hover(sel);return}catch(e){if(!/detached/i.test(String(e)))throw e}}}
/** Wait for the next overview re-render (the works container is rebuilt). */
async function nextRender(page){await page.$eval('#live-works .live-work',e=>{e.dataset.stale='1'});await page.waitForFunction(()=>!document.querySelector('#live-works .live-work[data-stale]'),{polling:100})}
(async()=>{
 const browser=await puppeteer.launch({headless:true,timeout:0,protocolTimeout:0,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',args:['--no-sandbox']});
 try{
  let appended=0;
  for(const [lang,theme] of [['fr','etat'],['en','sombre']]){
   const label=lang+'-'+theme,context=await browser.createBrowserContext(),page=await context.newPage();watch(page,label);
   await context.overridePermissions(base,['clipboard-read','clipboard-write','notifications']);
   await page.goto(base+'/login?lang='+lang,{waitUntil:'load'});
   await page.type('#access-key',cfg.key);await page.$eval('input[name=next]',e=>e.value='/api/v1/works');
   await Promise.all([page.waitForNavigation({waitUntil:'load'}),page.click('button[type=submit]')]);
   await page.goto(base+'/live.html?lang='+lang+'&theme='+theme,{waitUntil:'load'});
   await page.waitForFunction(()=>document.querySelectorAll('.live-node').length===4,{polling:250});
   assert.equal(await page.$eval('html',e=>e.dataset.theme),theme);
   assert.equal(await page.$eval('#live-active',e=>e.textContent),'2');
   assert.equal(await page.$$eval('.live-work',e=>e.length),2);
   // Connectors are drawn on the frame after each render (requestAnimationFrame).
   await page.waitForFunction(()=>document.querySelectorAll('.live-link').length===2,{polling:100});
   assert.equal(await page.$eval('#live-tokens',e=>e.textContent.replace(/\D/g,'')),'9000');
   assert(await page.$eval('#live-unknown',e=>!e.hidden&&/2/.test(e.textContent)),'agents without observed usage are flagged');
   assert(await page.$eval('.live-node[data-agent="audit"]',e=>e.querySelector('.live-warn')?.textContent.length>0),'invalid usage shown on node');
   await page.waitForFunction(()=>/direct poussé|live push/.test(document.getElementById('live-status').textContent),{polling:250});
   // LIVE-UX-02 indicators: attention list, per-model and top bars, real RAM/CPU of
   // the verified fixture processes, live duration, animated links and title count.
   assert(await page.$$eval('#live-attention .live-attn',e=>e.length)>=2,'attention list: failed + over-threshold');
   assert(await page.$$eval('#live-by-model .live-bars li',e=>e.length)>=1&&await page.$$eval('#live-top .live-bars li',e=>e.length)>=2);
   await page.waitForFunction(()=>/[KMG]i?[Bo]|io/.test(document.getElementById('live-ram').textContent),{polling:250});
   await page.waitForFunction(()=>/%/.test(document.getElementById('live-cpu').textContent),{polling:250});
   assert(await page.$eval('.live-node[data-agent="dev"]',e=>/RAM/.test(e.textContent)&&/12\/40/.test(e.textContent)&&e.querySelector('[data-since]')),'node shows RAM, tool meter and live duration');
   assert(await page.$$eval('.live-link-flow',e=>e.length)>=1,'flowing link for active child');
   assert(await page.$$eval('.live-particle',e=>e.length)>=1,'travelling particles when motion allowed');
   assert(/^\(2\)/.test(await page.title()),'tab title carries the active count');
   // Plan graph (initial phase) linked to live agents: tasks, dependencies, idle task, click → agent.
   assert.equal(await page.$$eval('.live-plan .live-task',e=>e.length),5,'5 planned tasks across both Works');
   assert(await page.$eval('.live-task[data-task="t5"]',e=>e.classList.contains('is-idle')),'never-started task flagged');
   await page.waitForFunction(()=>document.querySelectorAll('.live-depends').length>=3,{polling:250});
   await page.click('.live-task[data-task="t2"]');assert.equal(await page.evaluate(()=>document.activeElement?.dataset?.agent),'dev','task click focuses its agent');
   // Action feedback: EKG on active nodes, flowing arrow into the active task, and a
   // floating +700 tokens chip with a ripple when the agent's recorded usage grows.
   assert(await page.$$eval('.live-plan marker',e=>e.length)>=2&&await page.$$eval('.live-depends[marker-end]',e=>e.length)>=3,'dependency arrowheads');
   await hoverStable(page,'.live-task[data-task="t5"]');
   assert(await page.$eval('.live-task[data-task="t3"]',e=>e.classList.contains('is-dim'))&&!await page.$eval('.live-task[data-task="t1"]',e=>e.classList.contains('is-dim')),'hover highlights the t5 chain (t2, t1) and dims t3');
   // The overview re-renders every few seconds while agents are active: the
   // chain under the pointer survives it (regression of tel25).
   await nextRender(page);
   assert(await page.$eval('.live-task[data-task="t3"]',e=>e.classList.contains('is-dim'))&&await page.$$eval('.live-depends.is-hl',e=>e.length)>=1,'chain highlight kept across a re-render');
   await page.mouse.move(5,5);assert.equal(await page.$$eval('.live-task.is-dim',e=>e.length),0,'leaving clears the highlight');
   assert(await page.$$eval('.live-node.is-active .live-ekg',e=>e.length)>=2,'EKG on active nodes');
   assert(await page.$$eval('.live-depends.is-flow',e=>e.length)>=1,'flowing arrow into an active task');
   // L9 delivery ledger: tasks with attempts, observed tokens summed, no candidate commit recorded yet.
   {const rows=await page.$$eval('.live-delivery-table:not(.live-runs-table) tbody tr',r=>r.map(x=>({id:x.dataset.delivery,cells:[...x.children].map(c=>c.textContent)})));
    assert(rows.some(r=>r.id==='t1')&&rows.some(r=>r.id==='t2'),'delivery rows for tasks with attempts: '+JSON.stringify(rows));
    assert(rows.every(r=>r.cells[5].startsWith('—')),'no reviewed candidate in the fixture');
    assert((await page.$eval('.live-delivery-table:not(.live-runs-table) thead',e=>e.textContent)).includes(lang==='en'?'Candidate commit':'Commit candidat'),'ledger heading localized');
    assert(await page.$eval('.live-delivery-table:not(.live-runs-table)',t=>[...t.querySelectorAll('thead th')].every((h,i)=>{const c=t.querySelector('tbody tr').children[i];return getComputedStyle(h).textAlign===getComputedStyle(c).textAlign})),'headers aligned like their values');
    assert(await page.$$eval('.live-deliveries',els=>els.every(e=>e.getBoundingClientRect().right<=e.closest('.live-work').getBoundingClientRect().right+0.5)),'ledger stays inside its mission')}
   // L7: budget summary in the Work header (limit 10 USD, nothing declared yet:
   // the fixture agents declare no cost, so the figure says it is partial).
   {const chips=await page.$$eval('.live-work-budget',e=>e.map(x=>({text:x.textContent,state:x.dataset.budget})));
    assert(chips.length===1&&chips[0].state==='ok'&&/10[.,]00/.test(chips[0].text)&&/(partiel|partial)/.test(chips[0].text)&&!/—/.test(chips[0].text),'budget chip shows zero amounts, not a dash: '+JSON.stringify(chips))}
   // L11: the failed 'audit' attempt is grouped by its recorded usage error code.
   {await page.waitForSelector('#live-causes li[data-cause="usage:agent-usage-fixture"]');
    const g=await page.$eval('#live-causes li[data-cause="usage:agent-usage-fixture"]',e=>{e.querySelector('details').open=true;return {sum:e.querySelector('summary').textContent,items:[...e.querySelectorAll('ul li')].map(x=>x.textContent)}});
    assert(/agent-usage-fixture/.test(g.sum)&&/(1 tentative|1 attempt)/.test(g.sum)&&g.items.length===1&&/auditeur/.test(g.items[0]),'failure group: '+JSON.stringify(g))
    // Opened above (the toggle event records it); the periodic re-render keeps it open.
    await nextRender(page);
    assert(await page.$eval('#live-causes li[data-cause="usage:agent-usage-fixture"] details',e=>e.open),'opened cause group stays open across a re-render')}
   // L8: export the failures of the second Work. The page builds a JSONL Blob
   // and clicks a download link: the click is captured in the page and the
   // Blob read back, so the check does not depend on a download directory.
   if(lang==='fr'){await page.evaluate(()=>{window.__downloads=[];window.__blobs=new Map();const make=URL.createObjectURL;URL.createObjectURL=b=>{const u=make(b);window.__blobs.set(u,b);return u};const click=HTMLAnchorElement.prototype.click;HTMLAnchorElement.prototype.click=function(){if(this.download){window.__downloads.push({name:this.download,href:this.href});return}return click.call(this)}});
    const box=`.live-work[data-work="${cfg.works[1]}"] .live-export`;await page.$eval(box+' button',b=>b.click());
    await page.waitForFunction(b=>/(entrée|Export impossible)/.test(document.querySelector(b+' .live-export-state')?.textContent||''),{polling:100},box);
    assert(/1 entrée exportée/.test(await page.$eval(box+' .live-export-state',e=>e.textContent)),'export state');
    const got=await page.evaluate(async()=>{const d=window.__downloads[0];return {name:d.name,text:await window.__blobs.get(d.href).text()}});
    const lines=got.text.trim().split('\n').map(JSON.parse);
    assert(got.name==='swarm-dataset-'+cfg.works[1]+'-failed.jsonl'&&lines.length===2,'dataset lines: '+got.text.slice(0,300));
    assert(lines[0].kind==='header'&&lines[0].provenance==='recorded-facts-no-content'&&lines[0].select==='failed'&&lines[0].entries===1,'dataset header: '+JSON.stringify(lines[0]));
    assert(lines[1].agent_id==='audit'&&lines[1].cause.key==='usage:agent-usage-fixture'&&lines[1].task.title==='Analyse des accès','dataset entry: '+JSON.stringify(lines[1]));
    fs.writeFileSync(path.join(output,got.name),got.text)}
   // v35 server loop: this server runs it at the 60 s installation cadence.
   await page.waitForFunction(()=>/(60 s)/.test(document.getElementById('live-server-eval').textContent),{polling:250});
   assert(/(Évaluation serveur|Server evaluation)/.test(await page.$eval('#live-server-eval',e=>e.textContent)),'server evaluation line localized');
   // Review jobs and planning runs of the Work, each opening its telemetry.
   {await page.waitForFunction(()=>document.querySelectorAll('.live-runs tbody tr').length===2,{polling:250});
    const rows=await page.$$eval('.live-runs tbody tr',r=>r.map(x=>({run:x.dataset.run,active:x.classList.contains('is-active'),cells:[...x.children].map(c=>c.textContent)})));
    assert.deepEqual(rows.map(r=>[r.run,r.active]),[['review:j-review',true],['planning:c-plan',false]],'runs listed newest first per kind');
    assert(rows[0].cells[1]==='Revue du module'&&/(en cours|in progress)/.test(rows[0].cells[2])&&/(appliquée|applied)/.test(rows[1].cells[2]),'run rows: '+JSON.stringify(rows));
    assert((await page.$eval('.live-runs h3',e=>e.textContent)).startsWith(lang==='en'?'Reviews and planning':'Relectures et planification'),'runs heading localized');
    await page.focus('.live-runs tr[data-run="review:j-review"] .live-run-open');await nextRender(page);
    assert.equal(await page.evaluate(()=>document.activeElement?.closest('tr')?.dataset.run),'review:j-review','keyboard focus kept across a re-render');
    if(cfg.telemetry_token){const target=new Promise(r=>browser.once('targetcreated',r));await page.keyboard.press('Enter');
     const run=await (await target).page();watch(run,label+'-run');
     await run.waitForFunction(()=>/reçue|received/.test(document.getElementById('h-tel-state').textContent),{polling:250});
     assert(/run=review%3Aj-review|run=review:j-review/.test(run.url()),'run window url: '+run.url());
     const calls=await run.$$eval('#h-tel table tbody tr',r=>r.map(x=>x.textContent));assert(calls.length===1&&/90/.test(calls[0])&&/640/.test(calls[0]),'review call only: '+JSON.stringify(calls));
     assert(!/FIXTURE-REVIEW-PROMPT/.test(await run.content()),'no prompt content');
     assert(await run.evaluate(()=>[document.querySelector('.live-panels'),...['h-plan','h-tr','h-log'].map(id=>document.getElementById(id).closest('section'))].every(e=>e.getClientRects().length===0)),'agent-only panels not rendered');
     assert(/(Relecture|Review) · j-review/.test(await run.$eval('#h-title',e=>e.textContent))&&/t3/.test(await run.$eval('#h-facts',e=>e.textContent)),'run facts');
     for(const width of [980,390]){await run.setViewport({width,height:900});await run.screenshot({path:path.join(output,'run-'+label+'-'+width+'.png'),fullPage:true});assert(await run.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'run overflow at '+width)}
     await run.close()}
    await page.focus('body')}
   {const b=await page.$eval('.live-blockers',e=>e.innerText);assert(/Documentation/.test(b)&&/(attend|waits for)/.test(b)&&/Découper les lots/.test(b),'blockers list waiting tasks and their chain head: '+b.slice(0,200))}
   bumpUsage('dev',700);
   await page.waitForFunction(()=>{const f=document.querySelector('.live-node[data-agent="dev"] .live-float');return f&&/\+700/.test(f.textContent)&&f.parentElement.classList.contains('is-pulse')},{polling:100});
   await page.waitForFunction(()=>parseInt(document.getElementById('live-pulse').textContent,10)>0,{polling:250});
   bumpUsage('dev',-700);await page.waitForFunction(()=>document.getElementById('live-tokens').textContent.replace(/\D/g,'')==='9000',{polling:250});
   // LIVE-UX-04 quick wins: dynamic tab icon, export, keyboard, deep link, notifications.
   assert(/^data:image\/svg\+xml/.test(await page.evaluate(()=>SwarmLiveDashboard.favicon())),'favicon carries the attention badge');
   assert.equal(JSON.parse(await page.evaluate(()=>SwarmLiveDashboard.exportJSON())).overview.works.length,2,'export holds the served overview');
   await page.focus('body');await page.keyboard.press('a');assert(await page.$eval('#live-active-only',e=>e.checked)&&/active=1/.test(page.url()),'a toggles active-only and the URL');
   await page.keyboard.press('a');assert(!/active=1/.test(page.url()));
   const deep=await context.newPage();watch(deep,label+'-deep');await deep.goto(base+'/live.html?lang='+lang+'&mission='+cfg.works[1],{waitUntil:'load'});
   await deep.waitForFunction(w=>document.getElementById('live-filter').value===w&&document.querySelectorAll('.live-node').length===1,{polling:250},cfg.works[1]);await deep.close();
   await page.click('#live-notify');await page.waitForFunction(()=>document.getElementById('live-notify').getAttribute('aria-pressed')==='true',{polling:250});
   // L5: notifications come from server-fired alerts; the first evaluation is baseline.
   // Ends on success or on a shown refusal, so a broken evaluation fails instead of waiting forever.
   await page.waitForFunction(()=>SwarmLiveDashboard.evaluations()>=1||document.getElementById('live-alerts-state').classList.contains('is-error'),{polling:250});
   assert.equal(await page.$eval('#live-alerts-state',e=>e.textContent),'','alert evaluation succeeded');
   assert(await page.$('#live-alerts [data-alert="agent-failed|audit"]'),'seeded failed agent is an open alert');
   assert.equal(await page.evaluate(()=>SwarmLiveDashboard.notified()),0,'first evaluation is baseline');
   assert.equal(await page.$eval('#live-retention',e=>e.textContent),lang==='en'?'Retention: 30 d · no per-attempt cap':'Conservation : 30 j · sans plafond par tentative','saved v33 bounds shown');
   addFailedAgent('nouveau-'+lang,cfg.works[0]);
   await page.waitForFunction(()=>SwarmLiveDashboard.notified()>=1,{polling:250});
   await page.waitForSelector('#live-alerts [data-alert="agent-failed|nouveau-'+lang+'"]');
   await page.click('#live-notify');removeAgent('nouveau-'+lang);
   await page.waitForFunction(()=>document.querySelectorAll('.live-node').length===4,{polling:250});
   await page.waitForFunction(l=>!document.querySelector('#live-alerts [data-alert="agent-failed|nouveau-'+l+'"]')&&document.querySelector('#live-alerts-history [data-alert="agent-failed|nouveau-'+l+'"].is-resolved'),{polling:250},lang);
   assert.equal(await page.$eval('#live-alerts-title',e=>e.firstChild.textContent.trim()),lang==='en'?'Alerts':'Alertes');
   {const h=await page.$eval('#live-alerts-history [data-alert="agent-failed|nouveau-'+lang+'"]',e=>e.textContent);assert(lang==='en'?/Agent failed.*subject gone after/.test(h):/Agent en échec.*sujet disparu après/.test(h),h);assert.equal(await page.$eval('#live-alerts-history [data-alert="agent-failed|nouveau-'+lang+'"]',e=>e.dataset.resolution),'gone','deleted agent resolves as gone, not as fixed')}
   await page.$eval('.live-alerts-history',e=>{e.open=true});await page.$eval('#live-alerts-test',e=>e.scrollIntoView({block:'center'}));await page.screenshot({path:path.join(output,'live-alerts-'+label+'.png')});
   assert(await page.$$eval('.live-alert',els=>els.every(e=>e.getBoundingClientRect().right<=e.closest('.live-panel').getBoundingClientRect().right+0.5)),'alert items stay inside their panel');
   {const before=await page.evaluate(()=>SwarmLiveDashboard.notified());await page.click('#live-alerts-test');assert.equal(await page.evaluate(()=>SwarmLiveDashboard.notified()),before,'test notification is local, not an alert')}
   const heading=await page.$eval('#live-load-title',e=>e.textContent);assert.equal(heading,lang==='en'?'Load':'Charge');
   for(const width of [1440,390,320]){await page.setViewport({width,height:900});await new Promise(r=>setTimeout(r,150));
    await page.screenshot({path:path.join(output,'live-'+label+'-'+width+'.png'),fullPage:true});
    assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'horizontal overflow at '+width)}
   await page.setViewport({width:1440,height:900});
   // Regression: a resize used to erase the plan's dependency arrows.
   await page.waitForFunction(()=>document.querySelectorAll('.live-depends').length>=3,{polling:250});
   // Filter by mission and active-only.
   await page.select('#live-filter',cfg.works[1]);assert.equal(await page.$$eval('.live-node',e=>e.length),1);
   await page.click('#live-active-only');assert(await page.$eval('#live-empty',e=>!e.hidden));
   await page.click('#live-active-only');await page.select('#live-filter','');
   // Keyboard: focus the "dev" node, Enter opens the harness popup.
   await page.focus('.live-node[data-agent="lead"]');await page.keyboard.press('Tab');
   assert.equal(await page.evaluate(()=>document.activeElement?.dataset?.agent),'dev','Tab reaches the child node');
   assert(await page.$eval('.live-node[data-agent="dev"]',e=>e.matches(':focus-visible')&&parseFloat(getComputedStyle(e).outlineWidth)>0),'visible keyboard focus');
   const popupTarget=new Promise(r=>browser.once('targetcreated',r));await page.keyboard.press('Enter');
   const popup=await (await popupTarget).page();watch(popup,label+'-harness');await popup.waitForSelector('#h-log li');
   await popup.waitForFunction(()=>document.querySelectorAll('#h-log li').length>=103,{polling:250});
   assert.equal(await popup.$eval('#h-title',e=>e.textContent),(lang==='en'?'développeur':'développeur')+' · dev');
   assert.equal(await popup.evaluate(()=>SwarmLiveHarness.renderer()),'xterm','xterm renderer loaded under CSP nonce');
   await popup.waitForFunction(()=>SwarmLiveHarness.terminalText().includes('running 4 tests'),{polling:250});
   await popup.waitForFunction(()=>/RAM/.test(document.getElementById('h-res').textContent),{polling:250});
   await popup.waitForFunction(()=>/\d+(\.\d)? %/.test(document.getElementById('h-res').textContent),{polling:250});
   assert(/12\/40/.test(await popup.$eval('#h-tools',e=>e.textContent))&&/mem-architecture v2/.test(await popup.$eval('#h-tools',e=>e.textContent)),'tools meter and memory ids');
   assert(await popup.$$eval('#h-frise rect',e=>e.length)>=100,'activity timeline ticks');
   assert(/(Disponible quand|Available once)/.test(await popup.$eval('#h-tr-state',e=>e.textContent)),'running attempt: transcript pending');
   {const rev=await context.newPage();watch(rev,label+'-transcript');await rev.goto(base+'/live-session.html?lang='+lang+'&theme='+theme+'&agent=rev&work='+cfg.works[0],{waitUntil:'load'});
    await rev.waitForFunction(()=>document.querySelectorAll('#h-tr .live-tr-item').length>=7,{polling:250});
    const text=await rev.$eval('#h-tr',e=>e.innerText);assert(!/FIXTURE-(ASSISTANT|PATH|OUTPUT|CMD|ERR)/.test(text),'no content in transcript');
    assert(await rev.$$eval('#h-tr .live-tr-item.is-error',e=>e.length)===1&&await rev.$$eval('#h-tr .live-tr-item.is-tool_rejected',e=>e.length)===1,'error and rejected entries flagged');
    await rev.click('.live-tr-chip.is-tool');assert.equal(await rev.$$eval('#h-tr .live-tr-item.is-tool',e=>e.length),0,'filter hides tool entries');
    assert.equal(await rev.$eval('.live-tr-chip.is-tool',e=>e.getAttribute('aria-pressed')),'false');
    for(const width of [980,390]){await rev.setViewport({width,height:900});await rev.screenshot({path:path.join(output,'transcript-'+label+'-'+width+'.png'),fullPage:true});assert(await rev.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'transcript overflow at '+width)}
    // L4: the overview's alert evaluation evaluates this finished attempt once,
    // possibly after this window read it; the pushed change makes the
    // finished window re-read it (regression guard of tel27), no reload.
    await rev.waitForSelector('#h-eval table');
    {const rows=await rev.$$eval('#h-eval tbody tr',r=>r.map(x=>[...x.children].map(c=>c.textContent)));
     assert(rows.some(r=>/(Issue de la tentative|Attempt outcome)/.test(r[0])&&/(réussi|passed)/.test(r[2]))&&rows.some(r=>/(Limites|Run limits)/.test(r[0])),'evaluations: '+JSON.stringify(rows))}
    // L6: task scenario. FR saves "outcome completed" + "tool used Read" (no
    // telemetry for rev: not judgeable); EN finds both rows saved.
    await rev.waitForFunction(()=>document.querySelectorAll('#h-scn-rows li').length>0,{polling:250});
    if(lang==='fr'){assert(/Aucun scénario/.test(await rev.$eval('#h-scn-results',e=>e.textContent)),'no scenario yet');
     // An empty numeric value is refused before sending (never saved as 0).
     await rev.click('#h-scn-add');{const sel=(await rev.$$('#h-scn-rows li select'))[1];await sel.select('max-cost-usd')}await rev.click('#h-scn-save');
     await rev.waitForFunction(()=>/scenario-value-missing/.test(document.getElementById('h-scn-state').textContent),{polling:100});
     {const sel=(await rev.$$('#h-scn-rows li select'))[1];await sel.select('tool-used')}
     const inputs=await rev.$$('#h-scn-rows li input');await inputs[1].type('Read');await rev.click('#h-scn-save');
     await rev.waitForFunction(()=>/révision 1/.test(document.getElementById('h-scn-state').textContent)&&!!document.querySelector('#h-scn-results table'),{polling:100})}
    else await rev.waitForFunction(()=>document.querySelectorAll('#h-scn-rows li').length===2&&!!document.querySelector('#h-scn-results table'),{polling:250});
    {const rows=await rev.$$eval('#h-scn-results tbody tr',r=>r.map(x=>[...x.children].map(c=>c.textContent)));
     assert(rows.length===2&&/(réussi|passed)/.test(rows[0][1])&&/(non jugeable|not judgeable)/.test(rows[1][1])&&/no telemetry/.test(rows[1][2]),'scenario results: '+JSON.stringify(rows))}
    // L10: annotate the finished attempt (FR creates revision 1; EN finds it
    // prefilled and changes it to revision 2). The note is operator text.
    await rev.waitForFunction(()=>!document.getElementById('h-ann-save').disabled,{polling:250});
    if(lang==='fr'){assert.equal(await rev.$eval('#h-ann-state',e=>e.textContent),'Pas encore annotée.');
     await rev.click('input[name=verdict][value=bad]');await rev.type('#h-ann-note','Mauvais fichier modifié');
     assert(/23 \/ 2000/.test(await rev.$eval('#h-ann-count',e=>e.textContent)),'character counter');
     await rev.click('#h-ann-save');await rev.waitForFunction(()=>/révision 1/.test(document.getElementById('h-ann-state').textContent),{polling:100})}
    else{await rev.waitForFunction(()=>document.querySelector('input[name=verdict][value=bad]').checked&&document.getElementById('h-ann-note').value==='Mauvais fichier modifié',{polling:250});
     // Concurrent change (revision 1 → 2) from another window, then this
     // form saves with revision 1: conflict, the typed text is kept, and the
     // next save replaces revision 2 knowingly (revision 3).
     await rev.evaluate(async w=>{const s=await (await fetch('/api/v1/native-session')).json();await fetch('/api/v1/live/annotations',{method:'POST',headers:{'Content-Type':'application/json','X-Swarm-CSRF':s.csrf_token},body:JSON.stringify({schema_version:1,work:w,agent:'rev',verdict:'good',note:'other window',expected_revision:1})})},cfg.works[0]);
     expected.push({label:label+'-transcript',path:'/api/v1/live/annotations',status:409});
     await rev.click('input[name=verdict][value=unsure]');await rev.$eval('#h-ann-note',e=>{e.value='';});await rev.type('#h-ann-note','typed here');await rev.click('#h-ann-save');
     await rev.waitForFunction(()=>/revision 2\): your text is kept/.test(document.getElementById('h-ann-state').textContent),{polling:100});
     assert.equal(await rev.$eval('#h-ann-note',e=>e.value),'typed here','typed text kept on conflict');
     await rev.click('#h-ann-save');await rev.waitForFunction(()=>/revision 3/.test(document.getElementById('h-ann-state').textContent),{polling:100})}
    for(const width of [980,390]){await rev.setViewport({width,height:900});await rev.$eval('#h-ann-panel',e=>e.scrollIntoView());await rev.screenshot({path:path.join(output,'annotation-'+label+'-'+width+'.png')})}
    await rev.close()
    // The plan graph shows the scenario result on task t3.
    await page.waitForFunction(()=>/(scénario 1\/1|scenario 1\/1)/.test(document.querySelector('.live-task[data-task="t3"] .live-scenario')?.textContent||''),{polling:250});
    // The overview queue no longer lists the annotated attempt.
    await page.waitForFunction(()=>!document.querySelector('#live-queue li[data-queue="rev"]')&&!!document.querySelector('#live-queue li[data-queue="audit"]'),{polling:250})}
   {const plan=await popup.$eval('#h-plan',e=>e.innerText);assert(/Livrable t2/.test(plan)&&/(Prévu|Planned)/.test(plan)&&/(Réel|Actual)/.test(plan),'planned task panel: '+plan.slice(0,200))}
   if(cfg.telemetry_token){
    // Claude Code trace of 'lead': labelled tree, tool name, no private path.
    {const lead=await context.newPage();watch(lead,label+'-claude-trace');await lead.goto(base+'/live-session.html?lang='+lang+'&theme='+theme+'&agent=lead&work='+cfg.works[0],{waitUntil:'load'});
     await lead.waitForSelector('#h-tel .live-trace');const names=await lead.$$eval('#h-tel .live-trace .live-wf-name',e=>e.map(x=>x.textContent));
     assert.deepEqual(names,lang==='en'?['Interaction','Model call','Bash','Waiting for permission']:['Interaction','Appel modèle','Bash','Attente d’autorisation'],'claude trace: '+JSON.stringify(names));
     assert(!/PRIVATE|REDACTED/.test(await lead.content()),'no prompt or program path');
     await lead.$eval('#h-tel .live-trace',e=>e.scrollIntoView());await lead.screenshot({path:path.join(output,'claude-trace-'+label+'.png')});await lead.close()}
    // Provider OTLP telemetry seeded over HTTP by the fixture; a new call posted
    // now must reach the open window through the pushed fingerprint change.
    await popup.waitForFunction(()=>/reçue|received/.test(document.getElementById('h-tel-state').textContent),{polling:250});
    assert(/non authentifiée|unauthenticated/.test(await popup.$eval('#h-tel-state',e=>e.textContent)),'telemetry provenance label');
    assert(await popup.$$eval('#h-tel tr.is-alert th',e=>e.some(x=>x.textContent==='Bash')),'failed tool row flagged');
    {const c=await popup.$eval('.live-cache',e=>e.innerText);assert(/1 (relu sur|re-read of) 2/.test(c),'cache health: '+c)}
    assert(await popup.$$eval('#h-tel tr.is-warn',e=>e.length)===1,'the 0 % cache call is flagged re-read');
    assert(await popup.$$eval('.live-wf-row',e=>e.length)>=2&&await popup.$$eval('.live-wf-bar.is-fail',e=>e.length)>=1,'tool-call waterfall with the failed call marked');
    assert(!/FIXTURE-PROMPT-CONTENT/.test(await popup.$eval('#h-tel',e=>e.textContent)),'no prompt content shown');
    {const rows=await popup.$$eval('#h-tel table',ts=>ts.filter(t=>/Latences déclarées|Declared latencies/.test(t.caption?.textContent||'')).flatMap(t=>[...t.tBodies[0].rows].map(r=>[...r.cells].map(c=>c.textContent))));
     assert(rows.length===1&&rows[0][1]==='gpt-fixture'&&rows[0][2]==='4'&&rows[0][4]==='550 ms'&&/^2[,.]6 s$/.test(rows[0][5]),'latency histogram merged with quantiles: '+JSON.stringify(rows))}
    assert(/sqlite\.init\.duration_ms ×1/.test(await popup.$eval('#h-tel .live-facts',e=>e.textContent)),'unrecognized names listed by name');
    {const t=await popup.$eval('#h-tel',e=>e.textContent);
     // Burn rate needs two calls within 5 minutes of the newest: true for the two
     // seeded calls (first pass); after the FR pass posts a call dated now, the
     // newest is alone in its window and no rate is shown (correct behaviour).
     {const rows=await popup.$$eval('#h-tel .live-trace .live-wf-row',r=>r.map(x=>[x.querySelector('.live-wf-name').textContent,parseInt(x.querySelector('.live-wf-name').style.paddingLeft)||0]));
      assert(rows.length===3&&rows[2][1]===24&&/exec_command/.test(rows[2][0]),'trace tree with depth: '+JSON.stringify(rows));
      assert(!/PRIVATE/.test(t),'span code location not shown')}
     if(lang==='fr')assert(/Débit/.test(t),'burn rate shown');else assert(!/Burn rate/.test(t),'no rate from a lone recent call');
     assert(/(Réserve de lancement|Launch reserve).*186 %.*(dépassée|exceeded)/.test(t),'launch reserve use: '+t.slice(0,400))}
    assert(/lines_of_code\.count \(added\)\s*37/.test(await popup.$eval('#h-tel',e=>e.innerText)),'metric total shown');
    // Quick wins: model calls get their own timeline lane, the declared/telemetry
    // gap is shown, and `m` moves focus to the telemetry panel.
    {const d=await popup.evaluate(()=>({status:document.getElementById('h-status').textContent,classes:[...document.querySelectorAll('#h-frise rect')].reduce((m,r)=>(m[r.getAttribute('class')]=(m[r.getAttribute('class')]||0)+1,m),{})}));assert((d.classes['live-tick is-model']||0)>=2,'seeded model calls (2, plus earlier live posts) in their lane: '+JSON.stringify(d))}
    assert(/appels modèle|model calls/.test(await popup.$eval('#h-frise-legend',e=>e.textContent)),'model lane in legend');
    const lanes=await popup.$$eval('#h-frise rect',r=>[...new Set(r.map(x=>x.getAttribute('y')))].length);assert(lanes>=3,'one lane per group: '+lanes);
    assert(/(Écart déclaré|Declared).*\s*(\+|−)\s*[0-9]/s.test(await popup.$eval('#h-tel',e=>e.innerText)),'declared/telemetry gap row');
    await popup.evaluate(()=>document.activeElement?.blur());await popup.keyboard.press('m');
    assert.equal(await popup.evaluate(()=>document.activeElement.id),'h-tel-panel','m focuses the telemetry panel');
    const calls=()=>popup.$eval('#h-tel .live-mini strong',e=>parseInt(e.textContent,10));const before=await calls();
    const stamp=String(BigInt(Date.now())*1000000n+BigInt(checks.length));
    const sent=await fetch('http://'+cfg.address+'/api/otel/v1/logs',{method:'POST',headers:{'Content-Type':'application/json','Authorization':'Bearer '+cfg.telemetry_token},
     body:JSON.stringify({resourceLogs:[{scopeLogs:[{logRecords:[{timeUnixNano:stamp,body:{stringValue:'claude_code.api_request'},attributes:[{key:'model',value:{stringValue:'claude-fixture'}},{key:'input_tokens',value:{stringValue:'700'}}]}]}]}]})});
    assert.equal(sent.status,200,'live OTLP post accepted');
    await popup.waitForFunction(n=>parseInt(document.querySelector('#h-tel .live-mini strong').textContent,10)===n+1,{polling:250},before);
   }
   await popup.keyboard.press('/');assert.equal(await popup.evaluate(()=>document.activeElement.id),'h-filter','slash focuses the log filter');
   await popup.keyboard.type('étape 10');assert(await popup.$$eval('#h-log li:not([hidden])',e=>e.length)<=4,'filter narrows the log');
   await popup.click('#h-copy');await popup.waitForFunction(()=>/\d/.test(document.getElementById('h-copy-status').textContent),{polling:250});
   await popup.focus('#h-filter');await popup.keyboard.press('Escape');assert(await popup.$$eval('#h-log li:not([hidden])',e=>e.length)>=103,'Escape clears the filter');
   assert(/^▶/.test(await popup.title()),'harness title shows live state');
   assert(await popup.$eval('#h-last-error',e=>!e.disabled),'latest-error button enabled when an error line exists');
   await popup.click('#h-last-error');assert(await popup.evaluate(()=>document.activeElement?.classList.contains('kind-output-limit')),'focus moves to the latest error line');
   const term=await popup.evaluate(()=>SwarmLiveHarness.terminalText());assert(term.includes('$ cargo test')&&!term.includes('\x1b'),'ANSI interpreted');
   assert(await popup.$$eval('style',s=>s.length>0&&s.every(x=>x.nonce&&x.nonce.length>0)),'xterm styles carry the nonce');
   assert(await popup.$eval('#h-facts',e=>/fixture/.test(e.textContent)));
   await popup.waitForFunction(()=>/direct poussé|live push/.test(document.getElementById('h-status').textContent),{polling:250});
   // Wait on the appended message itself: a row count can jump past n+1 while
   // earlier cursor pages are still arriving.
   const pushed='ligne en direct '+(++appended);appendLog(pushed);
   await popup.waitForFunction(m=>document.querySelector('#h-log li:last-child .live-log-msg')?.textContent===m,{polling:250},pushed);
   assert.equal(await popup.$$eval('#h-log li .live-log-msg',(e,m)=>e.filter(x=>x.textContent===m).length,pushed),1,'appended row shown once');
   assert(await popup.$eval('#h-log li:last-child',e=>e.classList.contains('is-new')),'live-appended row is highlighted as new');
   assert.equal(await popup.$eval('html',e=>e.dataset.theme),theme);
   for(const width of [980,390]){await popup.setViewport({width,height:760});await new Promise(r=>setTimeout(r,150));
    await popup.screenshot({path:path.join(output,'harness-'+label+'-'+width+'.png'),fullPage:true});
    assert(await popup.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'harness overflow at '+width)}
   // Reduced motion: no particles, no running animations.
   const calm=await context.newPage();watch(calm,label+'-calm');await calm.emulateMediaFeatures([{name:'prefers-reduced-motion',value:'reduce'}]);
   await calm.goto(base+'/live.html?lang='+lang+'&theme='+theme,{waitUntil:'load'});await calm.waitForFunction(()=>document.querySelectorAll('.live-node').length===4,{polling:250});
   await calm.waitForFunction(()=>document.querySelectorAll('.live-link-flow').length>=1,{polling:250});
   assert.equal(await calm.$$eval('.live-particle',e=>e.length),0,'no particles with reduced motion');
   assert.equal(await calm.$eval('.live-link-flow',e=>getComputedStyle(e).animationName),'none','flow animation off with reduced motion');
   await calm.close();
   // Connection pool: dashboard + popup + 7 more harness windows in one profile
   // (> Chrome's 6 per host). One shared stream must keep every window live.
   const extra=[];for(let i=0;i<7;i++){const w=await context.newPage();watch(w,label+'-pool'+i);await w.goto(base+'/live-session.html?agent=dev&work='+cfg.works[0]+'&lang='+lang,{waitUntil:'load'});extra.push(w)}
   const pooled='ligne partagée '+(++appended);appendLog(pooled);
   for(const w of extra)await w.waitForFunction(m=>document.querySelector('#h-log li:last-child .live-log-msg')?.textContent===m,{polling:250},pooled);
   await popup.waitForFunction(m=>document.querySelector('#h-log li:last-child .live-log-msg')?.textContent===m,{polling:250},pooled);
   for(const w of extra)await w.close();
   // Forced fallback: without EventSource the harness polls and still follows.
   const polled=await context.newPage();watch(polled,label+'-polling');await polled.evaluateOnNewDocument(()=>{delete window.EventSource});
   await polled.goto(base+'/live-session.html?agent=dev&work='+cfg.works[0]+'&lang='+lang,{waitUntil:'load'});
   await polled.waitForFunction(()=>/interrogation périodique|periodic polling/.test(document.getElementById('h-status').textContent),{polling:250});
   const polledMessage='ligne interrogée '+(++appended);appendLog(polledMessage);
   await polled.waitForFunction(m=>document.querySelector('#h-log li:last-child .live-log-msg')?.textContent===m,{polling:250},polledMessage);
   assert.equal(await polled.$$eval('#h-log li',e=>e.length),await polled.evaluate(()=>document.querySelectorAll('#h-log li').length));
   await polled.close();
   // Finished agent: following stops and offers resume.
   const done=await context.newPage();watch(done,label+'-done');
   await done.goto(base+'/live-session.html?agent=rev&work='+cfg.works[0]+'&lang='+lang,{waitUntil:'load'});
   // Finished agent with push available: no polling, re-read on each pushed change, no resume button.
   await done.waitForFunction(()=>/(relu à chaque changement|re-read on each change)/.test(document.getElementById('h-status').textContent)&&document.getElementById('h-resume').hidden,{polling:250});
   // Unknown agent: explicit refusal code shown, no empty success.
   const ghost=await context.newPage();ghost.setDefaultTimeout(0);ghost.setDefaultNavigationTimeout(0);
   await ghost.goto(base+'/live-session.html?agent=ghost&work='+cfg.works[0],{waitUntil:'load'});
   await ghost.waitForFunction(()=>document.getElementById('h-status').textContent.includes('live-agent-unknown'),{polling:250});
   checks.push({case:label,status:'PASS',widths:[1440,390,320],harness_widths:[980,390],keyboard_open:true,cursor_pages:true,live_append_push:true,live_append_polling_fallback:true,nine_windows_one_profile:true,indicators:true,real_process_resources:true,reduced_motion:true,log_filter_copy:true,quick_wins_04:true,xterm_nonce:true,usage_state_flagged:true,telemetry_panel:!!cfg.telemetry_token,telemetry_quick_wins:!!cfg.telemetry_token});
   await context.close();
  }
  assert.deepEqual(problems,[],'console/CSP/HTTP problems');
  fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:'PASS',scope:'Local native live dashboard + harness UI on a private fixture Root; not installation, independent review or cutover',checks,problems,saturated_reads:saturated.length},null,2)+'\n');
 }catch(error){fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:'FAIL',error:String(error.stack||error),checks,problems},null,2)+'\n');process.exitCode=1}
 finally{await browser.close()}
})();
