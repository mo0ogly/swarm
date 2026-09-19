// Captures réelles de l'interface, données fictives ; aucun appel à une IA.
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawn, execFileSync} = require('node:child_process');
const {randomUUID} = require('node:crypto');
const puppeteer = require('puppeteer');
const repo = path.resolve(__dirname, '..');
const binary = path.join(repo, 'bin/swarm');
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'swarm-readme-'));
const output = path.join(repo, 'docs/screenshots');
let server, browser;
function cli(args, data) {
  return JSON.parse(execFileSync(binary, ['--root', root, '--json', ...args, ...(data ? ['--input', '-'] : [])], {input:data ? JSON.stringify(data) : undefined, encoding:'utf8'}));
}
function change(args, work, fields) {
  return cli(args, {schema_version:1,event_id:randomUUID(),expected_revision:work?.revision||0,...fields}).work;
}
(async()=>{
  fs.mkdirSync(output,{recursive:true});
  cli(['init']);
  let w=change(['work','create'],null,{title:'Démo — Une recherche accessible',objective:'Ajouter une recherche au catalogue et vérifier son utilisation au clavier.',scope:'Projet fictif de démonstration. Aucun agent lancé.',criteria:['Trouver un article par son titre','Parcours clavier vérifié'],next:'Préparer les missions puis choisir les agents.'});
  for(const t of [
    {id:'interface',title:'Créer la recherche',deliverable:'Champ de recherche et liste filtrée',criteria:['Filtrage par titre']},
    {id:'verification',title:'Tester le parcours clavier',deliverable:'Tests et rapport de vérification',criteria:['Résultats accessibles au clavier'],depends:['interface']},
    {id:'livraison',title:'Préparer la livraison',deliverable:'Résumé des changements et guide utilisateur',criteria:['Limites documentées'],depends:['verification']}
  ])w=change(['task','add',w.id],w,t);
  cli(['autonomy',w.id,'manuel']);
  server=spawn(binary,['--root',root,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
  const url=await new Promise((resolve,reject)=>{let s='';server.stdout.on('data',d=>{s+=d;const m=s.match(/http:\/\/[^\s]+/);if(m)resolve(m[0]);});server.once('error',reject);server.once('exit',c=>reject(new Error('serveur arrêté : '+c)));});
  browser=await puppeteer.launch({executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
  const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.setViewport({width:1600,height:1050,deviceScaleFactor:1});
  await page.goto(url+'?work='+w.id);await page.waitForSelector('#pilot-canvas svg');
  for(const theme of ['etat','sombre']){
    await page.evaluate(t=>setTheme(t),theme);
    await page.$eval('#pilot-toolbar',e=>e.scrollIntoView({block:'start'}));
    const clip=await page.evaluate(()=>{const a=document.getElementById('pilot-toolbar').getBoundingClientRect(),b=document.getElementById('pilot-canvas').getBoundingClientRect();return {x:a.x+scrollX,y:a.y+scrollY,width:a.width,height:b.bottom-a.top};});
    await page.screenshot({path:path.join(output,'pilotage-'+theme+'.png'),clip});
  }
  await page.click('[data-view="providers"]');await page.waitForSelector('#connections-add');await page.click('#connections-add');await page.waitForSelector('#modal[open]');
  for(const [id,value] of Object.entries({connection_id:'modele-local',connection_label:'Mon modèle local',connection_url:'http://localhost:11434/v1',connection_model:'mon-modele'}))await page.type('#field-'+id,value);
  await page.evaluate(()=>setTheme('etat'));
  await page.setViewport({width:1600,height:1200,deviceScaleFactor:1});
  await page.screenshot({path:path.join(output,'connexion.png')});
  await page.setViewport({width:1600,height:1050,deviceScaleFactor:1});
  await page.goto(new URL('/prepare.html',url).href);await page.waitForSelector('#new-title');
  await page.type('#new-title','Une recherche accessible dans le catalogue');
  await page.$eval('#new-need',e=>e.spellcheck=false);
  await page.type('#new-need','Je souhaite retrouver un article en saisissant quelques mots de son titre.\n\nLa recherche doit être utilisable au clavier et présenter clairement les résultats.\n\nRésultat attendu : une interface simple, des tests du filtrage et du parcours clavier, puis un court guide utilisateur.\n\nContrainte : conserver le fonctionnement actuel du catalogue.');
  await page.evaluate(()=>document.documentElement.dataset.theme='etat');
  await page.screenshot({path:path.join(output,'preparation.png')});
  if(errors.length)throw new Error(errors.join('\n'));
  console.log('4 captures générées ; aucun appel IA, aucune erreur JavaScript.');
})().catch(e=>{console.error(e);process.exitCode=1;}).finally(async()=>{
  if(browser)await browser.close();
  if(server){server.kill();await new Promise(r=>server.exitCode!==null?r():server.once('exit',r));}
  fs.rmSync(root,{recursive:true,force:true});
});
