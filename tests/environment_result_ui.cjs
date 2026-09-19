'use strict';
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process');const puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),recipe=JSON.parse(fs.readFileSync(process.argv[3],'utf8')),out=path.resolve(process.argv[4]);fs.mkdirSync(out,{recursive:true});
assert.ok(recipe.root.startsWith('/tmp/swarm-a9-'));
const server=spawn(binary,['--root',recipe.root,'web'],{stdio:['ignore','pipe','pipe']});let browser;
(async()=>{
 const url=await new Promise((resolve,reject)=>{let text='';const timer=setTimeout(()=>reject(Error('serveur muet')),10000);server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}});server.once('error',reject)});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome'});const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1440,height:1100});await page.goto(url+'?work='+recipe.work.id);await page.waitForSelector('#mission-summary');
 await page.waitForFunction(()=>document.querySelector('#mission-summary').textContent.includes('nouvelle vérification'));
 const text=await page.$eval('#mission-summary',e=>e.textContent);assert.match(text,/ne sera pas relancée automatiquement/);assert.match(text,/environnement/i);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await page.screenshot({path:path.join(out,'environment-'+theme+'.png'),fullPage:true})}
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['défaut environnement expliqué','nouvelle précondition explicite','aucune relance automatique annoncée','deux thèmes rendus'],errors},null,2));console.log('PASS environnement web');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();server.kill()});
