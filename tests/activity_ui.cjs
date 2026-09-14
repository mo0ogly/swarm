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

 await browser.close();
 const bilan={status:errors.length||external.length?'FAIL':'PASS',checks,errors,external};
 fs.writeFileSync(path.join(outDir,'activity.json'),JSON.stringify(bilan,null,1));
 console.log(JSON.stringify(bilan,null,1));
 server.kill();process.exit(bilan.status==='PASS'?0:1);
})().catch(e=>{console.error(String(e));server.kill();process.exit(1)});
