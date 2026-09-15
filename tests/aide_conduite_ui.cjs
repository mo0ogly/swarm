// Recette de l'aide du cockpit. L'aide n'est pas de la decoration : sur un
// systeme qui agit seul, elle est le seul endroit qui dit ce que le moteur
// fait sans demander. Aucun modele appele.
'use strict';
const assert=require('node:assert/strict');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os');
const {spawn,execFileSync}=require('node:child_process');
const puppeteer=require(process.env.PUPPETEER_MODULE||'puppeteer');
const binary=path.resolve(process.argv[2]),outDir=path.resolve(process.argv[3]);
fs.mkdirSync(outDir,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-aide-'));
const checks=[],errors=[],external=[];
const cli=(...a)=>{const r=execFileSync(binary,['--root',root,'--json',...a],{encoding:'utf8'});return r.startsWith('{')||r.startsWith('[')?JSON.parse(r):null};
const send=(a,d)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...a,'--input','-'],{encoding:'utf8',input:JSON.stringify(d)}));
const uuid=()=>require('node:crypto').randomUUID().replace(/-/g,'');
cli('init');
const w=send(['work','create'],{schema_version:1,event_id:uuid(),expected_revision:0,title:'Aide',objective:'Verifier l aide',scope:'recette',criteria:['aide lisible']}).work;
send(['task','add',w.id],{schema_version:1,event_id:uuid(),expected_revision:w.revision,id:'a1',title:'Tache',deliverable:'livrable',criteria:['c'],owner:'session',next:'lancer'});

const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});
let out='';server.stdout.on('data',d=>out+=d);
(async()=>{
 const deadline=Date.now()+10000;
 while(!/http:\/\/\S+/.test(out)){if(Date.now()>deadline)throw new Error('serveur non demarre');await new Promise(r=>setTimeout(r,80))}
 const url=out.match(/http:\/\/\S+/)[0];
 const b=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const p=await b.newPage();p.setDefaultTimeout(20000);
 p.on('pageerror',e=>errors.push(e.message));
 // Les erreurs de console comptent aussi : une ressource absente n'interrompt
 // rien mais salit chaque chargement, et masque les vraies erreurs quand on
 // vient chercher un incident.
 p.on('console',m=>{if(m.type()==='error')errors.push('console: '+m.text())});
 // Le flux d'evenements est une connexion longue : le navigateur l'interrompt
 // en se fermant, ce qui n'est pas une panne. Tout le reste compte.
 p.on('requestfailed',r=>{if(!r.url().includes('/api/v1/events'))errors.push('requete echouee : '+r.url())});
 p.on('response',r=>{if(r.status()>=400)errors.push(r.status()+' sur '+r.url())});
 p.on('request',r=>{const u=r.url();if(!u.startsWith(url.split('/session/')[0])&&!u.startsWith('data:')&&!u.startsWith('blob:'))external.push(u)});
 await p.setViewport({width:1500,height:1100});
 await p.goto(url);
 await p.waitForFunction(()=>document.querySelectorAll('#work option').length>0);
 await p.select('#work',w.id);
 await p.waitForFunction(t=>document.getElementById('title').textContent===t,{},'Aide');

 // 1. L'aide reste atteignable dans le mode qui masque les onglets experts.
 assert.equal(await p.evaluate(()=>document.body.dataset.mode),'conduite');
 assert.ok(!await p.$eval('#help',e=>e.hidden||e.offsetParent===null),'le bouton d aide est masque en mode conduite');
 checks.push('aide atteignable depuis le mode conduite');

 await p.click('#help');
 await p.waitForSelector('#modal[open] #help-search',{visible:true});
 const rubriques=await p.$$eval('#modal-rich .help-card h3',es=>es.map(e=>e.textContent));
 assert.ok(rubriques.length>=15,'rubriques attendues, obtenu '+rubriques.length);
 checks.push(rubriques.length+' rubriques presentes');

 // 2. Le systeme autonome est explique : ces sujets n'existaient pas.
 for(const sujet of ['conduite','autonomie','traiter','fil','graphe']){
   const present=await p.$$eval('#modal-rich .help-card',(es,s)=>es.some(e=>e.dataset.topic===s),sujet);
   assert.ok(present,'sujet d aide absent : '+sujet);
 }
 checks.push('cinq sujets couvrent le fonctionnement autonome');

 // 3. L'aide dit ce que le moteur NE fait pas. C'est la promesse du produit.
 const texte=await p.$eval('#modal-rich',e=>e.textContent);
 for(const phrase of ['n’accepte jamais','ne déroge jamais','Suspendre les départs prime',
                      'Soumis n’est pas accepté','sous-déclare l’autonomie','jamais 0,00']){
   assert.ok(texte.includes(phrase),'garantie absente de l aide : '+phrase);
 }
 checks.push('limites du moteur enoncees, pas seulement ses pouvoirs');

 // 4. La recherche trouve ces sujets par les mots qu'un lecteur emploiera.
 for(const [mot,attendu] of [['autonomie','autonomie'],['conducteur','conduite'],['créneaux','autonomie'],['signal','graphe']]){
   await p.$eval('#help-search',e=>{e.value=''});
   await p.type('#help-search',mot);
   await p.waitForFunction(()=>document.querySelectorAll('#modal-rich .help-card').length>0||document.querySelector('#help-results-state').textContent.includes('Aucune'));
   const ids=await p.$$eval('#modal-rich .help-card',es=>es.map(e=>e.dataset.topic));
   assert.ok(ids.includes(attendu),'recherche « '+mot+' » ne trouve pas '+attendu+' : '+ids.join(', '));
 }
 checks.push('recherche par les mots du lecteur');

 await p.screenshot({path:path.join(outDir,'aide-etat.png'),fullPage:true});
 await p.click('#theme');await new Promise(r=>setTimeout(r,200));
 await p.screenshot({path:path.join(outDir,'aide-sombre.png'),fullPage:true});
 const contraste=await p.$eval('#modal-rich .help-card',e=>getComputedStyle(e).color);
 assert.ok(contraste&&contraste!=='rgba(0, 0, 0, 0)','texte d aide sans couleur resolue en sombre');
 checks.push('aide rendue dans les deux themes');
 await p.click('#theme');

 // 5. Echap ferme et rend le focus : contrat commun a toutes les modales.
 await p.keyboard.press('Escape');
 await p.waitForFunction(()=>!document.querySelector('#modal').open);
 assert.equal(await p.$eval(':focus',e=>e.id),'help','le focus ne revient pas sur le declencheur');
 checks.push('Echap ferme l aide et rend le focus');

 // 6. Aucune ressource manquante : l'icone d'onglet est servie par le binaire.
 const icone=await p.evaluate(async()=>{const r=await fetch('/favicon.svg');return r.status});
 assert.equal(icone,200,'icone d onglet absente : le navigateur retombe sur /favicon.ico et journalise un 404');
 checks.push('aucune ressource manquante au chargement');

 await b.close();
 console.log(JSON.stringify({status:errors.length||external.length?'FAIL':'PASS',checks,errors,external:[...new Set(external)]},null,1));
 server.kill();
 if(errors.length||external.length)process.exit(1);
})().catch(e=>{console.error('FAIL '+e.message);server.kill();process.exit(1)});
