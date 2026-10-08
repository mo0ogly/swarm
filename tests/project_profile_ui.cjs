'use strict';
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn,execFileSync}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-profile-ui-'));fs.mkdirSync(out,{recursive:true});let app,browser;
const cli=(args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{input:input?JSON.stringify(input):undefined,encoding:'utf8'}));
(async()=>{try{
 cli(['init']);fs.writeFileSync(path.join(root,'AGENTS.md'),'PRIVATE_INSTRUCTION_CONTENT');fs.mkdirSync(path.join(root,'.claude'),{recursive:true});fs.writeFileSync(path.join(root,'.claude/settings.json'),'{"test":"PRIVATE_SETTINGS_CONTENT"}');
 cli(['project-profile','apply'],{profile:{version:1,name:'Project test',instructions:[{path:'AGENTS.md',roles:['preparation','planner','subplanner','worker','reviewer']}]},expected_sha256:''});
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
 fs.writeFileSync(path.join(out,'fixture.json'),JSON.stringify({root,url}));
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});const errors=[],checks=[];
 for(const lang of ['fr','en']){
 const p=await browser.newPage();p.on('pageerror',e=>errors.push(e.message));await p.setViewport({width:1400,height:1000});const u=new URL('/prepare.html',url);u.searchParams.set('lang',lang);await p.goto(url);await p.goto(u.href);await p.waitForFunction(()=>!document.querySelector('#project-profile-open').disabled);
 assert.match(await p.$eval('#project-profile-heading',e=>e.textContent),lang==='fr'?/Consignes du projet/:/Project instructions/);
 for(const theme of ['etat','sombre']){
 await p.evaluate(t=>document.documentElement.dataset.theme=t,theme);await p.click('#project-profile-open');assert.equal(await p.$eval('#project-profile-dialog',e=>e.open),true);
 const text=await p.$eval('#project-profile-dialog',e=>e.textContent);assert(!text.includes('PRIVATE_'),text);assert.equal(await p.$$eval('#project-profile-roles details',e=>e.length),5);assert(text.includes('AGENTS.md'));assert(text.includes('.claude/settings.json'));assert.match(text,/SHA-256/);
 await p.$$eval('#project-profile-roles details',els=>els.forEach(e=>e.open=true));await p.screenshot({path:path.join(out,`profile-${lang}-${theme}.png`)});await p.keyboard.press('Escape');await p.waitForFunction(()=>!document.querySelector('#project-profile-dialog').open);assert.equal(await p.evaluate(()=>document.activeElement.id),'project-profile-open');checks.push(`${lang}/${theme}: roles, hashes, no source content, Escape and focus`);
 }
 await p.close();
 }
 fs.unlinkSync(path.join(root,'swarm.project.json'));const p=await browser.newPage();await p.goto(new URL('/prepare.html?lang=en',url).href);await p.waitForFunction(()=>document.querySelector('#project-profile-status').textContent.includes('No project profile'));await p.click('#project-profile-open');assert.equal(await p.$$eval('#project-profile-roles details',e=>e.length),0);await p.keyboard.press('Escape');
 fs.writeFileSync(path.join(root,'swarm.project.json'),JSON.stringify({version:1,name:'Broken',instructions:[{path:'missing.md',roles:['preparation','planner','subplanner','worker','reviewer']}]}));await p.click('#project-profile-refresh');await p.waitForFunction(()=>document.querySelector('#project-profile-status').textContent.includes('unavailable'));assert.equal(await p.$eval('#project-profile-open',e=>e.disabled),true);checks.push('No profile and missing source states');assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'checks.json'),JSON.stringify(checks,null,2));console.log(checks.join('\n'));
 }finally{if(browser)await browser.close();if(app)app.kill('SIGTERM')}})().catch(e=>{console.error(e);process.exitCode=1});
