'use strict';
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn,execFileSync}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]||'bin/swarm'),out=path.resolve(process.argv[3]||'test-results/action-skills'),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-skills-ui-'));fs.mkdirSync(out,{recursive:true});let app,browser;
const cli=(args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{input:input?JSON.stringify(input):undefined,encoding:'utf8'}));
(async()=>{try{
cli(['init']);let w=cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Action skills test',objective:'Verify explicit task selection',scope:'isolated',criteria:['Selection remains explicit'],next:'Configure'}).work;
w=cli(['task','add',w.id],{schema_version:1,event_id:'task',expected_revision:w.revision,id:'t1',title:'Apply a selected method',deliverable:'report',criteria:['method transmitted'],next:'Check'}).work;
for(let i=0;i<9;i++){const dir=path.join(root,'.claude/skills','method-'+i);fs.mkdirSync(dir,{recursive:true});fs.writeFileSync(path.join(dir,'SKILL.md'),`---\nname: Method ${i}\ndescription: A bounded task method\n---\nSKILL_MARKER_${i}`)}
const bad=path.join(root,'.claude/skills/unavailable');fs.mkdirSync(bad);fs.writeFileSync(path.join(bad,'SKILL.md'),'x'.repeat(16001));
const provider=path.join(root,'fixture-agent');fs.writeFileSync(provider,`#!/bin/sh\ncat >'${path.join(root,'received-prompt.txt')}'\nprintf 'HANDOFF: fixture complete\\n'\n`,{mode:0o700});fs.writeFileSync(path.join(root,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{fixture:{command:provider}}}));
const items=cli(['skills','list']).skills;assert.equal(items.length,10);assert.equal(items.filter(x=>x.available).length,9);
app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});const errors=[],checks=[];
const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1440,height:1000});await page.goto(url);
for(const lang of ['fr','en']){
 const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang',lang);await page.goto(u.href);await page.waitForFunction(()=>snapshot?.work?.tasks?.length===1);
 if(await page.evaluate(()=>document.body.dataset.mode)==='conduite')await page.click('#mode');
 await page.click('[data-view="tasks"]');await page.waitForSelector('#tasks-body [data-task="t1"] button',{visible:true});await page.click('#tasks-body [data-task="t1"] button');await page.waitForSelector('#field-action');await page.select('#field-action','start');await page.waitForFunction(()=>document.querySelectorAll('.action-skill-option').length===10 && !modalContext.skillLoading);
 assert.match(await page.$eval('#action-skills legend',e=>e.textContent),lang==='fr'?/Skills pour cette tâche/:/Skills for this task/);
 assert.equal(await page.$$eval('.action-skill-option input:checked',a=>a.length),0);
 assert.equal(await page.$$eval('.action-skill-option input:disabled',a=>a.length),1);
 for(let i=0;i<8;i++)await page.click(`[data-skill-path=".claude/skills/method-${i}/SKILL.md"]`);
 assert.equal(await page.$$eval('.action-skill-option input:disabled',a=>a.length),2);
 const selection=await page.$eval('[name="skills"]',e=>JSON.parse(e.value));assert.equal(selection.length,8);assert.equal(selection[0].sha256,items[0].sha256);
 await page.click('[data-skill-path=".claude/skills/method-0/SKILL.md"]');assert.equal(await page.$$eval('.action-skill-option input:disabled',a=>a.length),1);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.$eval('#action-skills',e=>e.scrollIntoView());await page.screenshot({path:path.join(out,`${lang}-${theme}.png`)});const tokens=[...new Set(fs.readFileSync(path.resolve(__dirname,'../web/project-profile-settings.css'),'utf8').match(/--wattson-[a-z-]+/g))];const missing=await page.evaluate(tokens=>tokens.filter(t=>!getComputedStyle(document.documentElement).getPropertyValue(t).trim()),tokens);assert.deepEqual(missing,[]);const surface=await page.$eval('#action-skills',e=>({ink:getComputedStyle(e).color,bg:getComputedStyle(e).backgroundColor}));assert.notEqual(surface.ink,surface.bg);checks.push(`${lang}/${theme}: explicit choices, description, unavailable file, selection limit`)}
 await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#modal').open);
}
await page.waitForSelector('#tasks-body [data-task="t1"] button',{visible:true});await page.click('#tasks-body [data-task="t1"] button');await page.waitForSelector('#field-action');await page.select('#field-action','start');await page.waitForFunction(()=>document.querySelectorAll('.action-skill-option').length===10&&!modalContext.skillLoading);await page.select('#field-provider','fixture');await page.$eval('#field-workspace',e=>{e.value='.';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.click('[data-skill-path=".claude/skills/method-0/SKILL.md"]');await page.click('#confirm');await page.waitForFunction(()=>!document.querySelector('#modal').open);
 const deadline=Date.now()+10000;while(Date.now()<deadline&&(!fs.existsSync(path.join(root,'received-prompt.txt'))||!fs.readFileSync(path.join(root,'received-prompt.txt'),'utf8').includes('SKILL_MARKER_0')))await new Promise(r=>setTimeout(r,100));const prompt=fs.readFileSync(path.join(root,'received-prompt.txt'),'utf8');assert(prompt.includes('SKILL_MARKER_0'));assert(!prompt.includes('SKILL_MARKER_1'));checks.push('Real web launch transmits only selected skill to fixture provider');
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',root,checks,errors},null,2));console.log('PASS action skills UI: '+checks.length+' theme/language combinations');
}finally{await browser?.close();app?.kill('SIGTERM')}})().catch(e=>{console.error(e);process.exitCode=1});
