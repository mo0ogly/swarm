'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),os=require('node:os');
const {spawn,execFileSync}=require('node:child_process');
const puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-term-'));
const cli=(...args)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args],{encoding:'utf8'}));
const mutate=(args,revision,fields)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,'--input','-'],{encoding:'utf8',input:JSON.stringify({schema_version:1,event_id:crypto.randomUUID(),expected_revision:revision,...fields})})).work;
cli('init');
const provider=path.join(root,'codex');fs.writeFileSync(provider,`#!/usr/bin/python3
import sys,json
text=sys.stdin.read()
print(json.dumps({'type':'thread.started','thread_id':'fixture-session'}))
print(json.dumps({'type':'item.completed','item':{'id':'message','type':'agent_message','text':'SESSION CONTROLEE — '+('PREMIER ECHANGE' if 'resume' not in sys.argv else text)}}))
print(json.dumps({'type':'turn.completed','usage':{'input_tokens':10,'output_tokens':5}}))
`,{mode:0o700});
fs.writeFileSync(path.join(root,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{recette:{command:provider,args:['exec','--json','--sandbox','workspace-write','-']}}}));
let w=mutate(['work','create'],0,{title:'Recette — session interactive',objective:'Répondre à un agent depuis le graphe',scope:'Terminal supervisé, aucune IA appelée',criteria:['saisie et reprise'],next:'Ouvrir la session'});
w=mutate(['task','add',w.id],w.revision,{id:'terminal-demo',title:'Échanger avec l’agent',deliverable:'Session de démonstration',criteria:['aller-retour terminal'],next:'Démarrer en mode interactif'});
cli('autonomy',w.id,'manuel');
let server,browser,url;
async function startServer(){server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});return await new Promise((resolve,reject)=>{let text='';const timer=setTimeout(()=>reject(Error(text)),10000);server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}});server.stderr.on('data',d=>text+=d)})}
const checks=[],errors=[],csp=[],external=[];
(async()=>{
 url=await startServer();browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage();page.setDefaultTimeout(10000);await page.setViewport({width:1450,height:1050});
 page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(/Content Security Policy|Refused to/.test(m.text()))csp.push(m.text())});page.on('request',r=>{if(!['127.0.0.1','localhost','data:'].includes(new URL(r.url()).hostname)&&!r.url().startsWith('data:'))external.push(r.url())});
 await page.goto(url);await page.waitForSelector('.graph-noeud');await page.click('.graph-noeud');await page.waitForSelector('#pilot-inspector[open]');
 const all=await page.$('[data-inspector-action="actions"]');await all.click();await page.waitForSelector('#field-mode');await page.focus('#field-mode');await page.keyboard.press('Home');await page.keyboard.press('ArrowDown');await page.keyboard.press('Enter');await page.click('#confirm');
 await page.waitForSelector('#agent-terminal-dialog[open]');
 let frame=await (await page.$('#agent-terminal-dialog iframe')).contentFrame();
 await frame.waitForFunction(()=>document.querySelector('#terminal-screen').textContent.includes('SESSION CONTROLEE'));
 assert.match(await frame.$eval('#terminal-status',e=>e.textContent),/Lecture seule/);
 await frame.click('#terminal-claim');await frame.waitForFunction(()=>document.querySelector('#terminal-status').textContent.includes('Saisie active'));
 await frame.focus('.xterm-helper-textarea');await page.keyboard.type('Bonjour depuis la modale');await page.keyboard.press('Enter');
 await frame.waitForFunction(()=>document.querySelector('#terminal-screen').textContent.includes('SESSION CONTROLEE — Bonjour depuis la modale'));
 checks.push('lancement via formulaire, vrai PTY, saisie aller-retour');
 let dropInput=true;await page.setRequestInterception(true);
 page.on('request',async request=>{
  if(dropInput&&request.url().endsWith('/api/v1/terminal/control')&&JSON.parse(request.postData()||'{}').kind==='input'){
   dropInput=false;await fetch(request.url(),{method:'POST',headers:request.headers(),body:request.postData()});await request.abort('failed');
  }else await request.continue();
 });
 await frame.focus('.xterm-helper-textarea');await page.keyboard.type('Z');
 await frame.waitForFunction(()=>document.querySelector('#terminal-notice').textContent.includes('Réception de la dernière saisie non confirmée'));
 await frame.click('#terminal-claim');await frame.focus('.xterm-helper-textarea');await page.keyboard.press('Enter');
 await frame.waitForFunction(()=>document.querySelector('#terminal-screen').textContent.includes('SESSION CONTROLEE — Z'));
 assert.equal(await frame.$eval('.xterm-rows',e=>(e.textContent.match(/SESSION CONTROLEE — Z/g)||[]).length),1);
 checks.push('réponse réseau perdue après écriture : saisie suspendue, aucune retransmission automatique');
 await page.click('#agent-terminal-details');await page.waitForSelector('#pilot-inspector[open]');await page.locator('[data-inspector-action=terminal]').click();await page.waitForSelector('#agent-terminal-dialog[open]');frame=await(await page.$('#agent-terminal-dialog iframe')).contentFrame();await frame.waitForFunction(()=>!document.querySelector('#terminal-claim').disabled);await frame.click('#terminal-claim');await frame.waitForFunction(()=>document.querySelector('#terminal-status').textContent.includes('Saisie active'));checks.push('détails et validation accessibles depuis le terminal');

 const agent=cli('agent','list',w.id).agents[0].agent;assert.equal(agent.mode,'dialogue');
 // A second tab can view output but cannot steal the first writer.
 const second=await browser.newPage();await second.goto(new URL('/terminal.html?'+new URLSearchParams({agent:agent.id,work:w.id}),url).href);await second.waitForFunction(()=>!document.querySelector('#terminal-claim').disabled);await second.click('#terminal-claim');await second.waitForFunction(()=>document.querySelector('#terminal-notice').textContent.includes('autre vue'));await second.close();checks.push('deuxième vue en lecture seule, aucun vol de saisie');
 for(const theme of ['etat','sombre']){
  await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await frame.waitForFunction(t=>document.documentElement.dataset.theme===t,{},theme);
  const roles=await frame.evaluate(()=>{const css=getComputedStyle(document.documentElement);return ['fond','carte','carte-appuyee','champ','texte','titre','texte-discret','lien','action-fond','action-encre','action-bordure','ligne','alerte-encre','alerte-fond','info-encre','info-fond','succes-encre','attention-encre'].filter(n=>!css.getPropertyValue('--wattson-'+n).trim())});assert.deepEqual(roles,[]);
  await page.screenshot({path:path.join(out,theme+'.png')});
 }
 checks.push('DOM rendu et jetons présents dans les deux thèmes');
 await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#agent-terminal-dialog'));assert.equal(cli('agent','show',agent.id).agent.status,'running');
 await page.click('.graph-noeud');await page.waitForSelector('#agent-terminal-dialog[open]');frame=await(await page.$('#agent-terminal-dialog iframe')).contentFrame();await frame.waitForFunction(()=>document.querySelector('#terminal-screen').textContent.includes('SESSION CONTROLEE — Bonjour'));
 checks.push('Échap ferme la vue, session vivante, clic graphe rouvre le même historique');
 // Replacing only the web process must not kill the detached terminal supervisor.
 server.kill('SIGTERM');await new Promise(resolve=>server.once('exit',resolve));url=await startServer();await page.goto(url);await page.waitForSelector('.graph-noeud');await page.click('.graph-noeud');await page.waitForSelector('#agent-terminal-dialog[open]');frame=await(await page.$('#agent-terminal-dialog iframe')).contentFrame();await frame.waitForFunction(()=>document.querySelector('#terminal-screen').textContent.includes('SESSION CONTROLEE — Bonjour'));
 assert.equal(cli('agent','list',w.id).agents.length,1);checks.push('redémarrage serveur web : même agent, même tentative, historique conservé');
 await page.setViewport({width:390,height:844});await page.screenshot({path:path.join(out,'mobile.png')});
 const fits=await page.$eval('#agent-terminal-dialog',e=>{const r=e.getBoundingClientRect();return r.width<=innerWidth&&r.height<=innerHeight});assert.ok(fits);checks.push('modale mobile dans le viewport');
 await frame.click('#terminal-stop');
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await frame.waitForFunction(t=>document.documentElement.dataset.theme===t,{},theme);await page.screenshot({path:path.join(out,'arret-'+theme+'.png')});}
 await frame.click('#terminal-stop-confirm');await frame.waitForFunction(()=>/interrompue|terminée/.test(document.querySelector('#terminal-status').textContent));assert.equal(cli('agent','show',agent.id).agent.status,'interrupted');assert.notEqual(cli('work','show',w.id).work.tasks[0].status,'accepted');checks.push('arrêt confirmé, saisie désactivée, aucune acceptation automatique');
 assert.deepEqual(errors,[]);assert.deepEqual(csp,[]);assert.deepEqual(external,[]);
 fs.writeFileSync(path.join(out,'ui.json'),JSON.stringify({status:'PASS',checks,errors,csp,external,root},null,2));
})().catch(e=>{console.error(e);fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({error:e.stack,errors,csp,root},null,2));process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(server)server.kill('SIGTERM');try{for(const a of cli('agent','list',w.id).agents||[])if(['running','starting','queued'].includes(a.agent.status))cli('agent','stop',a.agent.id)}catch{}});
