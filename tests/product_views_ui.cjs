'use strict';
// CI browser recipe; uses the same public CLI fixture as the manual UI recipe.
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict');
const {spawn,execFileSync}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);
fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-product-ui-'));
const fixture=JSON.parse(execFileSync('python3',['tests/product_recipe.py',binary,root],{encoding:'utf8'}));
const app=spawn(binary,['--root',root,'web','127.0.0.1:0']);let browser;const errors=[];
(async()=>{
 const session=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',v=>{text+=v;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('fixture server exited '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));
 await page.setViewport({width:1440,height:1000});await page.goto(session);
 await page.waitForSelector('#product-heading');
 const button=async text=>{
  const buttons=await page.$$('#product-views button');
  for(const b of buttons)if(await b.evaluate((e,wanted)=>e.textContent===wanted,text)){await b.click();return}
  throw Error('Missing product button '+text);
 };
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  await page.goto(new URL('/?lang='+lang,session).href);
  await page.waitForSelector('#product-heading');
  if(await page.$eval('html',e=>e.dataset.theme)!==theme)await page.click('#theme');
  await button('Application');
  assert.match(await page.$eval('.product-count',e=>e.textContent),/1–20\/25/);
  assert.equal(await page.$$('.product-map-node').then(es=>es.length),20);
  await button(lang==='fr'?'20 éléments suivants':'Next 20 items');
  assert.match(await page.$eval('.product-count',e=>e.textContent),/21–25\/25/);
  assert.equal(await page.$$('.product-map-node').then(es=>es.length),5);
  await button('later-26 · Parcours futur 26');await button('s26-later · Fonction future 26');
  assert.match(await page.$eval('#product-views',e=>e.textContent),lang==='fr'?/Non planifiée : aucune tâche liée/:/Unplanned: no linked tasks/);
  assert.equal(await page.$$('.graph-noeud').then(es=>es.length),0);
  await button('Application');await page.type('#pilot-search','futur 26');
  await page.waitForFunction(()=>document.querySelectorAll('.product-open').length===1);
  assert.match(await page.$eval('.product-count',e=>e.textContent),/1–1\/1/);
  await button('Application');await button('account · Suivre mes commandes');
  await button('s02-order · Commander une pizza');
  assert.equal(await page.$eval('.product-breadcrumb',e=>e.textContent.includes('Suivre mes commandes')),true);
  await button('Application');await button('buy · Acheter une pizza');
  const story=await page.$('.product-open');await story.focus();await page.keyboard.press('Enter');
  await page.waitForFunction(()=>document.querySelectorAll('.graph-noeud').length===1);
  await button('Acheter une pizza');await button('s02-order · Commander une pizza');
  assert.equal(await page.$$('.graph-noeud').then(es=>es.length),3);
  assert.match(await page.$eval('#product-views',e=>e.textContent),/2/);
  await page.screenshot({path:path.join(out,`story-${lang}-${theme}.png`)});
  await page.reload();await page.waitForSelector('#product-heading');
  assert.equal(await page.$$('.graph-noeud').then(es=>es.length),3);
  assert.match(await page.$eval('#product-heading',e=>e.textContent),/Commander une pizza/);
 }
 assert.deepEqual(errors,[]);
 const agents=JSON.parse(execFileSync(binary,['--root',root,'--json','agent','list',fixture.work],{encoding:'utf8'}));
 assert.deepEqual(agents.agents,[]);
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',pagination:20,journeys:25,stories:26,keyboard:true,shared_tasks:true,unplanned:true,persistence:true,languages:['fr','en'],themes:['etat','sombre'],agents:0,errors},null,2));
 console.log('PASS product navigation, pagination, search, keyboard, persistence, FR/EN and themes');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{await browser?.close();app.kill()});
