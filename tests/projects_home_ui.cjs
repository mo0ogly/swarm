'use strict';
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]||'test-results/projects-home');fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-projects-home-'));
function cli(args,data){return JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(data?['--input','-']:[])],{input:data?JSON.stringify(data):undefined,encoding:'utf8'})||'null')}
cli(['init']);const w=cli(['work','create'],{schema_version:1,event_id:'home-create',expected_revision:0,title:'Mon projet français',objective:'Préserver le titre saisi par l’utilisateur.',scope:'UI fixture only',criteria:['Navigation'],next:'Examiner'}).work;
const app=spawn(binary,['--root',root,'web','127.0.0.1:0']);let browser;const errors=[],mutations=[],mutationDetails=[];
(async()=>{
 const session=await new Promise((resolve,reject)=>{let s='';let timer=setTimeout(()=>reject(Error('startup timeout')),20000);app.stdout.on('data',x=>{s+=x;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}});app.on('exit',c=>reject(Error('exit '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});const p=await browser.newPage();p.on('dialog',d=>d.accept());p.on('pageerror',e=>errors.push(e.message));p.on('request',r=>{if(r.method()!=='GET')(mutations.push(r.url()),mutationDetails.push({url:r.url(),document:r.frame()?.url()}))});
 await p.goto(session);const base=new URL(session).origin;await p.goto(base+'/projects.html?lang=fr',{waitUntil:'networkidle0'});assert.ok(mutations.every(url=>{const u=new URL(url);return u.pathname==='/api/v1/visit'&&u.searchParams.get('work')===w.id}),'only initial fixture departure visit');mutations.length=0;mutationDetails.length=0;
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  await p.setViewport({width:1440,height:1100});await p.goto(base+'/projects.html?lang='+lang);await p.waitForSelector('#project-list a');if(await p.$eval('html',e=>e.dataset.theme)!==theme)await p.click('#project-theme');assert.equal(await p.$eval('html',e=>e.dataset.theme),theme);assert.equal(await p.$eval('#project-theme',e=>e.getAttribute('aria-pressed')),String(theme==='sombre'));
  assert.equal(await p.$eval('html',e=>e.lang),lang);assert.equal(await p.$eval('#project-list strong',e=>e.textContent),w.title);
  assert.match(await p.$eval('#launch-title',e=>e.textContent),lang==='en'?/Verified results/:/résultats vérifiés/);
  await p.click('[name=intent][value=debug]');assert.match(await p.$eval('#project-start',e=>e.href),/intent=debug/);
  await p.type('#project-search','does-not-exist');assert.equal(await p.$$('#project-list a').then(a=>a.length),0);await p.$eval('#project-search',e=>{e.value='';e.dispatchEvent(new Event('input'))});
  await p.screenshot({path:path.join(out,lang+'-'+theme+'.png'),fullPage:true});
  for(const width of [1024,760,390,320]){await p.setViewport({width,height:900});assert.ok(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'overflow '+width)}
  await p.screenshot({path:path.join(out,lang+'-'+theme+'-mobile.png'),fullPage:true});
 }
 await p.setViewport({width:1440,height:1000});await p.goto(base+'/projects.html?lang=en');await p.waitForSelector('#project-list a');await p.focus('[name=intent][value=build]');await p.keyboard.press('ArrowDown');assert.equal(await p.$eval('[name=intent]:checked',e=>e.value),'improve');
 await p.click('#project-start');await p.waitForFunction(()=>document.querySelector('#new-title')?.value==='Project improvement');assert.match(await p.$eval('#new-need',e=>e.value),/Existing project/);
 await p.goto(base+'/prepare.html?start=template&lang=en');await p.waitForSelector('#template-dialog[open]');await p.keyboard.press('Escape');
 await p.setRequestInterception(true);let fault=true;const intercept=r=>{if(r.url().endsWith('/api/v1/works')&&fault)r.respond({status:fault==='empty'?200:503,contentType:'application/json',body:fault==='empty'?'[]':'{"error":"fixture"}'});else r.continue()};p.on('request',intercept);
 await p.goto(base+'/projects.html?lang=en');await p.waitForSelector('#project-retry:not([hidden])');assert.match(await p.$eval('#project-status',e=>e.textContent),/Unable to load/);assert.equal(await p.$eval('#project-search',e=>e.disabled),true);fault=false;await p.click('#project-retry');await p.waitForSelector('#project-list a');
 fault='empty';await p.goto(base+'/projects.html?lang=en');await p.waitForFunction(()=>document.getElementById('project-status').textContent.includes('first mission'));assert.equal(await p.$$('#project-list a').then(a=>a.length),0);
 if(mutations.length)console.error('Mutation diagnostic',JSON.stringify(mutationDetails));assert.deepEqual(mutations,[],'launcher/preparation must not dispatch or save without confirmation');
 p.off('request',intercept);await p.setRequestInterception(false);
 await p.goto(base+'/?work='+w.id+'&lang=en',{waitUntil:'domcontentloaded'});await p.waitForSelector('.project-home-link');await p.click('.project-home-link');await p.waitForSelector('#project-start');
 assert.deepEqual(errors,[]);assert.ok(mutations.every(url=>new URL(url).pathname==='/api/v1/visit'),'cockpit only records navigation');
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({result:'PASS',languages:['fr','en'],themes:['etat','sombre'],widths:[1440,1024,760,390,320],checks:['intent','theme-button','empty-state','keyboard','filter','preserved-user-title','preparation-prefill','template-dialog','error-retry','cockpit-link','no-mutations'],root},null,2));console.log('PASS projects home real-server journeys');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();app.kill('SIGTERM')});
