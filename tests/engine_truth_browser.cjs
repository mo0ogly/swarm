'use strict';
// Mandatory browser check under the authorized truth runner, isolated root only.
const assert=require('node:assert/strict'),{spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const [binary,root,work,candidate]=process.argv.slice(2);
let app,browser;const errors=[];
(async()=>{
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const session=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',b=>{text+=b;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});app.once('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));page.on('response',r=>{if(r.status()>=400)errors.push(r.status()+' '+r.url())});
 const url=new URL(session);url.searchParams.set('work',work);
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,390]){
  await page.setViewport({width,height:900});url.searchParams.set('lang',lang);await page.goto(url.href);await page.waitForSelector('#review-read-first');
  if(await page.$eval('html',e=>e.dataset.theme)!==theme)await page.click('#theme');
  const roles=await page.$eval('#planning-ownership',e=>e.textContent);assert.match(roles,/organization-fixture/);assert.match(roles,/managed-review-fixture/);
  await page.click('#review-read-first');await page.waitForSelector('#modal[open] #review-evidence');
  const proof=await page.$eval('#review-evidence',e=>e.textContent);assert.ok(proof.includes(candidate));assert.match(proof,/git diff --exit-code/);assert.match(proof,/changes_requested/);assert.match(proof,/not_accepted/);
  assert.match(proof,lang==='en'?/exit code 0/:/code de sortie 0/);assert.match(proof,lang==='en'?/Read revision/:/Révision lue/);
  const geometry=await page.$eval('#modal',e=>({width:e.clientWidth,scroll:e.scrollWidth}));assert.ok(geometry.scroll<=geometry.width+1,'horizontal overflow');
  await page.keyboard.press('Tab');assert.ok(await page.evaluate(()=>!!document.activeElement.closest('#modal')));
  await page.keyboard.press('Escape');assert.equal(await page.$eval('#modal',e=>e.open),false);assert.equal(await page.evaluate(()=>document.activeElement.id),'review-read-first');
  console.log(JSON.stringify({check:'browser-proof',lang,theme,width,geometry,candidate,roles:true,command:true,exitCode:0,decision:'not_accepted',keyboard:true}));
 }
 assert.deepEqual(errors,[]);console.log('PASS actual browser: FR/EN, light/dark, desktop/mobile, roles, proof, refusal, keyboard, zero HTTP/JS errors; isolated fixture, not real-provider autonomy.');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{await browser?.close();app?.kill()});
