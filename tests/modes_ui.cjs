// Recette des deux niveaux d'interface : conduite par défaut, expert complet.
// Tout passe par des contrôles visibles ; aucune requête hors du poste.
'use strict';
const fs=require('node:fs'),path=require('node:path'),{spawn,execFileSync}=require('node:child_process');
const puppeteer=require(process.env.PUPPETEER_MODULE||'puppeteer');
const binary=path.resolve(process.argv[2]), outDir=path.resolve(process.argv[3]);
fs.mkdirSync(outDir,{recursive:true});
const root=execFileSync('python3',[path.join(__dirname,'web_fixture.py'),binary],{encoding:'utf8'}).trim();
const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});
const errors=[],external=[],checks=[];
(async()=>{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(new Error('serveur non démarré : '+out)),15000);server.stdout.on('data',d=>{out+=d;const m=out.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(m){clearTimeout(timer);resolve(m[0])}});server.on('exit',c=>reject(new Error('serveur arrêté '+c)))});
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const page=await browser.newPage();
 page.on('pageerror',e=>errors.push(e.message));
 // Le favicon n'est pas servi par le cockpit ; tout autre échec est une erreur.
 page.on('response',r=>{if(r.status()>=400&&!r.url().endsWith('/favicon.ico'))errors.push('HTTP '+r.status()+' '+r.url())});
 page.on('request',r=>{const u=new URL(r.url());if(!['localhost','127.0.0.1'].includes(u.hostname)&&u.protocol!=='data:')external.push(r.url())});
 await page.setViewport({width:1400,height:950});
 await page.goto(url);
 await page.waitForSelector('#work');
 await new Promise(r=>setTimeout(r,1500));
 if(!await page.evaluate(()=>!!document.querySelector('#conduite:not([hidden])')))throw new Error('écran de conduite masqué');
 checks.push('mode conduite par défaut');
 const hidden=await page.$$eval('#tabs button',bs=>bs.filter(b=>!b.hidden).map(b=>b.dataset.view));
 if(hidden.join()!=='conduite')throw new Error('onglets visibles en conduite : '+hidden.join());
 checks.push('un seul onglet en conduite');
 if(await page.$eval('#assistant',e=>e.hidden)!==true)throw new Error('assistant visible en conduite');
 checks.push('assistant rangé en conduite');
 await page.screenshot({path:path.join(outDir,'conduite-etat.png'),fullPage:true});
 await page.click('#theme');await new Promise(r=>setTimeout(r,150));
 await page.screenshot({path:path.join(outDir,'conduite-sombre.png'),fullPage:true});
 checks.push('conduite rendue dans les deux thèmes');
 await page.click('#mode');
 await page.waitForFunction(()=>document.body.dataset.mode==='expert');
 const visible=await page.$$eval('#tabs button',bs=>bs.filter(b=>!b.hidden).length);
 if(visible<9)throw new Error('onglets manquants en expert : '+visible);
 checks.push('mode expert : '+visible+' vues disponibles');
 await page.screenshot({path:path.join(outDir,'expert-sombre.png'),fullPage:true});
 // persistance + URL : le jeton de session est déjà échangé contre un cookie.
 const base=new URL(url).origin;
 await page.goto(base);await page.waitForFunction(()=>document.body.dataset.mode==='expert');
 checks.push('mode persisté au rechargement');
 await page.goto(base+'/?mode=conduite');await page.waitForFunction(()=>document.body.dataset.mode==='conduite');
 checks.push('mode imposé par URL');
 // clavier : le premier bouton de la barre reçoit le focus
 await page.focus('#conduite-autonomy');
 const focused=await page.$eval(':focus',e=>e.id);
 if(focused!=='conduite-autonomy')throw new Error('focus clavier : '+focused);
 checks.push('navigation clavier sur la barre de conduite');
 const state=await page.$eval('#conduite-state',e=>e.textContent);
 if(!/créneau/.test(state))throw new Error('barre sans créneaux : '+state);
 checks.push('barre de conduite : '+state.slice(0,90));
 await browser.close();
 fs.writeFileSync(path.join(outDir,'modes.json'),JSON.stringify({status:errors.length||external.length?'FAIL':'PASS',checks,errors,external},null,1));
 console.log(JSON.stringify({status:errors.length||external.length?'FAIL':'PASS',checks,errors,external},null,1));
 server.kill();process.exit(errors.length||external.length?1:0);
})().catch(async e=>{console.error(String(e));server.kill();process.exit(1)});
