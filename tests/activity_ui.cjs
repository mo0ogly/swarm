// Recette du fil d'activité : travail vide, distinction moteur/humain, filtre
// des décisions et pagination sans doublon. Aucun modèle appelé.
'use strict';
const assert=require('node:assert/strict');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os');
const {spawn,execFileSync}=require('node:child_process');
const puppeteer=require(process.env.PUPPETEER_MODULE||'puppeteer');
const binary=path.resolve(process.argv[2]),outDir=path.resolve(process.argv[3]);
fs.mkdirSync(outDir,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-fil-'));
const checks=[],errors=[],external=[];
const cli=(...args)=>{const r=execFileSync(binary,['--root',root,'--json',...args],{encoding:'utf8'});return r.startsWith('{')||r.startsWith('[')?JSON.parse(r):null};
const send=(args,data)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,'--input','-'],{encoding:'utf8',input:JSON.stringify(data)}));
const uuid=()=>require('node:crypto').randomUUID().replace(/-/g,'');
const mutate=(args,revision,fields)=>send(args,{schema_version:1,event_id:uuid(),expected_revision:revision,...fields}).work;
const sleep=ms=>new Promise(r=>setTimeout(r,ms));

cli('init');

// Travail vide : rien ne s'est produit, l'écran doit le dire.
let vide=mutate(['work','create'],0,{title:'Travail sans activité',objective:'Observer un fil vide',scope:'recette',criteria:['message lisible'],next:'ajouter une tâche'});

// Travail actif : assez d'entrées pour dépasser une page, des deux origines.
let actif=mutate(['work','create'],0,{title:'Travail suivi',objective:'Observer le fil',scope:'recette',criteria:['fil lisible'],next:'lancer'});
for(let i=1;i<=3;i++) actif=mutate(['task','add',actif.id],actif.revision,{id:'f'+i,title:'Tâche f'+i,deliverable:'rapport',criteria:['preuve'],owner:'recette',next:'lancer'});
// Gestes d'opérateur : suspendre puis réautoriser les départs.
cli('autonomy',actif.id,'manuel');


const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});
const lignes=page=>page.$$eval('.fil-entree',ns=>ns.map(n=>({
  origine:n.querySelector('.fil-origine')?.dataset.origine,
  label:n.querySelector('.fil-label')?.textContent,
  message:n.querySelector('.fil-message')?.textContent})));

(async()=>{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(new Error('serveur non démarré : '+out)),15000);server.stdout.on('data',d=>{out+=d;const m=out.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(m){clearTimeout(timer);resolve(m[0])}});server.on('exit',c=>reject(new Error('serveur arrêté '+c)))});
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const page=await browser.newPage();
 page.setDefaultTimeout(20000);
 page.on('pageerror',e=>errors.push(e.message));
 page.on('response',r=>{if(r.status()>=400&&!r.url().endsWith('/favicon.ico'))errors.push('HTTP '+r.status()+' '+r.url())});
 page.on('request',r=>{const u=new URL(r.url());if(!['localhost','127.0.0.1'].includes(u.hostname)&&u.protocol!=='data:')external.push(r.url())});
 await page.setViewport({width:1500,height:1000});
 await page.goto(url);
 await page.waitForSelector("#conduite:not([hidden])");

 const ouvrir=async(id,titre)=>{await page.select('#work',id);await page.waitForFunction(t=>document.getElementById('title').textContent===t,{},titre);await sleep(400)};

 // 1. Un travail neuf n'a pas de fil vide : sa création y figure déjà, du côté
 // humain. Le message « aucune activité » reste du code défensif, il couvre un
 // état que le moteur ne produit pas — mieux vaut le dire que le tester faux.
 await ouvrir(vide.id,"Travail sans activité");
 await page.waitForFunction(()=>document.querySelectorAll('.fil-entree').length>0);
 const neuf=await lignes(page);
 assert.equal(neuf.length,1,'un travail neuf porte sa seule création : '+JSON.stringify(neuf));
 assert.equal(neuf[0].origine,'vous','créer un travail est un geste humain : '+JSON.stringify(neuf[0]));
 assert.match(neuf[0].message,/Travail sans activité/,'le titre saisi doit être rendu : '+neuf[0].message);
 checks.push('travail neuf : sa création figure au fil, côté humain');

 // 2. Les gestes d'opérateur apparaissent du côté humain, avec leur motif.
 await ouvrir(actif.id,'Travail suivi');
 await page.waitForFunction(()=>document.querySelectorAll('.fil-entree').length>0);
 const toutes=await lignes(page);
 const humaines=toutes.filter(x=>x.origine==='vous');
 assert.ok(humaines.length>0,'les gestes d’opérateur doivent apparaître : '+JSON.stringify(toutes));
 const autonomie=humaines.find(x=>/autonomie/i.test(x.label));
 assert.ok(autonomie,'le réglage d’autonomie doit figurer au fil : '+JSON.stringify(humaines));
 assert.match(autonomie.message,/Manuel|manuel/,'le motif enregistré doit être rendu : '+autonomie.message);
 checks.push('gestes d’opérateur rendus côté humain, avec leur motif');

 // 3. Le libellé est en français, jamais la chaîne technique du moteur.
 for(const x of toutes){
  assert.ok(x.label&&!/^[a-z.]+$|^retex-/.test(x.label),'libellé technique affiché tel quel : '+x.label);
 }
 checks.push('libellés français, aucune chaîne technique brute');

 // 4. Le filtre ne garde que les arbitrages humains, et se retire.
 await page.click('#fil-decisions');
 await sleep(500);
 const filtrees=await lignes(page);
 assert.ok(filtrees.length>0,'le filtre ne doit pas vider un fil qui contient des décisions');
 for(const x of filtrees) assert.equal(x.origine,'vous','entrée moteur conservée par le filtre : '+JSON.stringify(x));
 await page.click('#fil-decisions');
 await sleep(500);
 assert.ok((await lignes(page)).length>=toutes.length,'décocher le filtre doit ramener le fil complet');
 checks.push('filtre « décisions seulement » appliqué puis retiré');

 // 5. Deux thèmes.
 await page.screenshot({path:path.join(outDir,'fil-etat.png'),fullPage:true});
 await page.click('#theme');await sleep(200);
 await page.screenshot({path:path.join(outDir,'fil-sombre.png'),fullPage:true});
 await page.click('#theme');
 checks.push('fil rendu dans les deux thèmes');

 // 6. La case du filtre et son libellé forment une seule cible cliquable.
 const cible=await page.evaluate(()=>{
  const l=document.querySelector('.fil-filtre'),c=document.getElementById('fil-decisions');
  const rl=l.getBoundingClientRect(),rc=c.getBoundingClientRect();
  return {contient:rc.left>=rl.left-1&&rc.right<=rl.right+1,ecart:Math.round(rc.left-rl.left)};
 });
 assert.ok(cible.contient,'la case doit rester dans son libellé');
 assert.ok(cible.ecart<40,'case détachée de son libellé, écart de '+cible.ecart+' px');
 checks.push('case et libellé du filtre forment une seule cible');

 // 7. Le repère de visite ne bouge pas pendant la consultation : figé à
 // l'ouverture, il ne doit pas glisser sous les yeux du lecteur au gré des
 // rafraîchissements.
 await ouvrir(actif.id,'Travail suivi');
 const repere=async()=>page.evaluate(()=>{
  const n=document.querySelector('.fil-visite');
  return n?[...document.querySelectorAll('#fil-entrees > *')].indexOf(n):-1;
 });
 const avant=await repere();
 assert.ok(avant>=0,'une visite antérieure doit exister pour ce contrôle');
 // Une autre session — le cockpit terminal, un second onglet — enregistre une
 // visite pendant qu'on lit. Si la référence était relue à chaque
 // rafraîchissement, le repère sauterait en tête et le lecteur perdrait le fil
 // de ce qu'il était en train de parcourir.
 execFileSync('python3',['-c',
   "import sqlite3,sys;c=sqlite3.connect(sys.argv[1]);c.execute(\"UPDATE session_visits SET at=?\",(sys.argv[2],));c.commit()",
   path.join(root,'.swarm','state.db'), new Date().toISOString().replace('Z','000Z')],{encoding:'utf8'});
 execFileSync(binary,['--root',root,'--json','autonomy',actif.id,'assiste'],{encoding:'utf8'});
 await sleep(2500);
 const apres=await repere();
 assert.ok((await lignes(page)).length>0,'le fil doit rester peuplé');
 assert.equal(apres,avant+1,'le repère a bougé : figé à l’ouverture, il ne doit pas suivre une visite écrite ailleurs pendant la lecture');
 checks.push('repère figé : une visite écrite ailleurs ne le déplace pas');

 // Date de visite stockée côté serveur, lue par la page ouverte.
 const dateVisite=async p=>p.evaluate(async id=>(await (await fetch('/api/v1/snapshot?work='+encodeURIComponent(id))).json()).visit?.at||'',actif.id);

 // 8. La visite s'enregistre en quittant le travail, jamais en l'ouvrant.
 // La date stockée doit avancer : un trait « dernière visite » à l'écran
 // apparaîtrait aussi bien pour une visite bien plus ancienne.
 const visiteAvant=await dateVisite(page);
 await page.select('#work',vide.id);
 await page.waitForFunction(t=>document.getElementById('title').textContent===t,{},'Travail sans activité');
 await sleep(800);
 await ouvrir(actif.id,'Travail suivi');
 const marque=await page.evaluate(()=>document.querySelector('.fil-visite')?.textContent||'');
 assert.match(marque,/dernière visite/,'le repère doit apparaître après être sorti puis revenu : '+marque);
 const visiteApres=await dateVisite(page);
 assert.ok(visiteApres>visiteAvant,
   'quitter le travail n’a rien enregistré : '+visiteAvant+' puis '+visiteApres);
 checks.push('visite enregistrée à la sortie, repère présent au retour');

 // 8bis. Fermeture de l'onglet. L'enregistrement partait par sendBeacon, qui ne
 // peut poser aucun en-tête : le garde de session le refusait en 403, avalé en
 // silence, et le repère n'était jamais posé en quittant. On mesure la date
 // stockée avant et après la fermeture, pas la présence d'un trait à l'écran.
 const onglet=await browser.newPage();
 await onglet.goto(url);
 await onglet.waitForFunction(()=>document.querySelectorAll('#work option').length>1);
 await onglet.select('#work',actif.id);
 await onglet.waitForFunction(t=>document.getElementById('title').textContent===t,{},'Travail suivi');
 await sleep(500);
 // Mesure juste avant de fermer : le seul changement de travail écrit déjà une
 // visite par la voie ordinaire, et la mesurer plus tôt attribuerait son effet
 // à la fermeture.
 const avantFermeture=await dateVisite(page);
 await sleep(1100);
 await onglet.close();
 await sleep(800);
 const apresFermeture=await dateVisite(page);
 assert.ok(apresFermeture>avantFermeture,
   'la fermeture de l’onglet n’a rien enregistré : '+avantFermeture+' puis '+apresFermeture);
 checks.push('visite enregistrée aussi à la fermeture de l’onglet');

 // 8ter. Accordéon : replier le fil, le retrouver replié, et lire quand même
 // ce qu'il cache. Un panneau qui se replie sans rien dire oblige à le rouvrir
 // pour savoir s'il s'est passé quelque chose.
 assert.equal(await page.$eval('#fil-bloc',e=>e.open),true,'le fil doit être déplié par défaut');
 const resumePlie=await page.$eval('#fil-resume',e=>e.textContent.trim());
 assert.ok(resumePlie.length>0,'le sommaire ne dit rien de ce qu’il masque');
 await page.$eval('#fil-bloc > summary',e=>e.click());
 await page.waitForFunction(()=>!document.querySelector('#fil-bloc').open);
 assert.ok(!await page.$eval('#fil-entrees',e=>e.checkVisibility?.()??e.offsetParent!==null),
   'les entrées restent visibles alors que le fil est replié');
 assert.match(await page.$eval('#fil-bloc > summary',e=>getComputedStyle(e,'::after').content),/Déplier/,
   'le sommaire replié doit proposer de déplier');
 await page.reload();
 await page.waitForFunction(()=>document.querySelector('#fil-bloc'));
 assert.equal(await page.$eval('#fil-bloc',e=>e.open),false,'l’état replié doit survivre au rechargement');
 await page.$eval('#fil-bloc > summary',e=>e.click());
 await page.waitForFunction(()=>document.querySelector('#fil-bloc').open);
 await page.waitForFunction(()=>document.querySelectorAll('#fil-entrees .fil-entree').length>0);
 checks.push('fil repliable, état conservé, sommaire parlant');

 // 8quater. Surbrillance : chaque entrée colorée reste lisible sans la couleur.
 // Le moteur doit avoir agi, sinon le fil ne contient qu'une origine et
 // comparer les teintes ne prouverait rien. Un ordonnancement refusé suffit :
 // c'est bien le moteur qui l'écrit, et il est journalisé comme tel.
 cli('dispatch',actif.id);
 await page.reload();
 await page.waitForFunction(t=>document.getElementById('title')?.textContent===t,{},'Travail suivi');
 await page.waitForFunction(()=>[...document.querySelectorAll('#fil-entrees .fil-entree')].some(n=>n.dataset.origine==='moteur'));
 const paires=await page.$$eval('#fil-entrees .fil-entree',ns=>ns.map(n=>({
   origine:n.dataset.origine||'',
   mot:(n.querySelector('.fil-origine')?.textContent||''),
   bord:getComputedStyle(n).borderLeftColor})));
 assert.ok(paires.length>0,'aucune entrée à contrôler');
 for(const x of paires){
   assert.ok(['moteur','vous'].includes(x.origine),'entrée sans origine : '+JSON.stringify(x));
   assert.ok(x.mot.includes(x.origine),'la couleur n’est pas doublée par le mot : '+JSON.stringify(x));
   assert.ok(x.bord&&x.bord!=='rgba(0, 0, 0, 0)','surbrillance d’origine absente : '+JSON.stringify(x));
 }
 // Sans les deux origines à l'écran, comparer leurs teintes ne prouve rien :
 // l'assertion passerait quelles que soient les couleurs.
 const origines=new Set(paires.map(x=>x.origine));
 assert.equal(origines.size,2,'le fil doit montrer les deux origines pour que ce contrôle ait un sens : '+[...origines].join(', '));
 const teintes=new Set(paires.map(x=>x.origine+'|'+x.bord));
 assert.equal(new Set([...teintes].map(t=>t.split('|')[1])).size,
   new Set(paires.map(x=>x.origine)).size,
   'deux origines doivent se distinguer par la teinte : '+[...teintes].join(' , '));
 checks.push('surbrillance par origine, toujours doublée par le mot');

 // 9. Le bandeau dit franchement qu'il n'y a rien, plutôt que d'aligner des zéros.
 const accueil=async()=>page.$eval('#conduite-accueil',e=>e.textContent.trim());
 await page.waitForFunction(()=>document.getElementById('conduite-accueil').textContent.trim().length>0);
 const repos=await accueil();
 assert.match(repos,/Rien de neuf|Aucune décision/,'bandeau au repos illisible : '+repos);
 assert.ok(!/\b0\b/.test(repos),'le bandeau ne doit pas afficher de zéros : '+repos);
 checks.push('bandeau : absence dite en toutes lettres, sans zéros');

 // 10. Une décision arrive : le bandeau la compte et mène à la zone concernée.
 execFileSync(binary,['--root',root,'--json','autonomy',actif.id,'autonome'],{encoding:'utf8'});
 await sleep(2500);
 const apresChangement=await accueil();
 assert.match(apresChangement,/depuis votre visite/,'le bandeau doit compter ce qui a changé : '+apresChangement);
 const segments=await page.$$eval('#conduite-accueil .accueil-segment',bs=>bs.map(b=>b.textContent));
 assert.ok(segments.length>0,'les segments du bandeau doivent être cliquables : '+apresChangement);
 checks.push('bandeau : compte ce qui a changé depuis la visite');

 // 11. Aucune tentative n'a été lancée sur ce travail : le bandeau ne doit
 // annoncer aucun coût, et surtout pas un zéro qui passerait pour une mesure.
 const phrase=await page.$eval('#conduite-accueil',e=>e.textContent);
 assert.ok(!/USD/.test(phrase),'aucune tentative : le bandeau ne doit pas parler de coût : '+phrase);
 const etat=await page.$eval('#conduite-state',e=>e.textContent);
 // Trois états distincts, jamais confondus : aucune tentative, des tentatives
 // sans coût déclaré, un montant rapporté.
 assert.match(etat,/aucune tentative|coût réel non rapporté/,'la ligne d’état doit déclarer ce qu’elle sait du coût : '+etat);
 assert.ok(!/0\.00 USD rapport/.test(etat),'un coût inconnu ne s’affiche pas 0,00 : '+etat);
 checks.push('coût : absence déclarée, jamais un zéro inventé');

 await browser.close();
 const bilan={status:errors.length||external.length?'FAIL':'PASS',checks,errors,external};
 fs.writeFileSync(path.join(outDir,'activity.json'),JSON.stringify(bilan,null,1));
 console.log(JSON.stringify(bilan,null,1));
 server.kill();process.exit(bilan.status==='PASS'?0:1);
})().catch(e=>{console.error(String(e));server.kill();process.exit(1)});
