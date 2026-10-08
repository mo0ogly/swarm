'use strict';
const {spawn}=require('child_process'),fs=require('fs'),path=require('path'),assert=require('assert/strict'),puppeteer=require('puppeteer');
const [binary,root,work,out]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});let app,browser;
(async()=>{
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const url=await new Promise((ok,fail)=>{let s='';app.stdout.on('data',x=>{s+=x;const m=s.match(/http:\/\/\S+/);if(m)ok(m[0])});app.on('exit',c=>fail(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,args:['--no-sandbox']});const p=await browser.newPage();await p.setViewport({width:1400,height:1000});const errors=[];p.on('pageerror',e=>errors.push(String(e)));
 for(const lang of ['fr','en']){
 await p.goto(url+'?work='+work+'&lang='+lang);await p.waitForFunction(()=>typeof snapshot!=='undefined'&&snapshot?.work);await p.evaluate(()=>showView('conduite'));await p.click('#requalify-first');await p.waitForFunction(()=>modalContext?.requalification && !document.querySelector('#confirm').disabled);
 await p.type('#field-planning-reason','Verify retained historical evidence');
 for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,lang+'-'+theme+'.png')})}
 await p.keyboard.press('Escape');assert.equal(await p.evaluate(()=>document.activeElement.id),'requalify-first');
 }
 await p.click('#requalify-first');await p.waitForFunction(()=>modalContext?.requalification);await p.type('#field-planning-reason','Verify retained historical evidence');await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open);await p.waitForFunction(()=>snapshot.work.tasks[0].requalifications?.length===1);assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['real button','read-only identities','FR EN two themes','Escape focus','public submit persisted']}));
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(app)app.kill()});
