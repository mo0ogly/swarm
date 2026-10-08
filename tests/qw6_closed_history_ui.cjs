'use strict';
// Real rendering from an isolated Go-engine fixture, without provider calls.
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),puppeteer=require('puppeteer');
const out=path.resolve(process.argv[2]||'/tmp/qw6-evidence');
(async()=>{const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});try{
 const p=await browser.newPage(),errors=[],network=[];p.on('pageerror',e=>errors.push(e.message));p.on('console',e=>{if(e.type()==='error')errors.push(e.text())});p.on('requestfailed',r=>network.push(r.url()));await p.setViewport({width:1400,height:1000});
 await p.setContent('<html><body><main id="panel"></main></body></html>');
 for(const name of ['wattson_themes','cockpit','pilotage','mission'])await p.addStyleTag({path:path.resolve('web/'+name+'.css')});
 for(const name of ['i18n-en','status-contract'])await p.addScriptTag({path:path.resolve('web/'+name+'.js')});
 await p.evaluate(()=>{globalThis.lang='fr';globalThis.SwarmI18n={t:s=>lang==='en'?(SwarmEnglish[s]||s):s};globalThis.node=(tag,text,cls)=>{const e=document.createElement(tag);if(text)e.textContent=text;if(cls)e.className=cls;return e};globalThis.Pilot={command:(text,fn)=>{const e=node('button',text);e.onclick=fn;return e}};globalThis.RoleModels={open(){}};globalThis.Quotas={open(){throw Error('closed mission offered quota recovery')}};});
 await p.addScriptTag({path:path.resolve('web/planning.js')});const fixture=JSON.parse(fs.readFileSync(path.join(out,'closed-fixture.json')));
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
 await p.evaluate(({fixture,lang,theme})=>{globalThis.lang=lang;document.documentElement.dataset.theme=theme;globalThis.snapshot={...fixture,paused:false,independent_reviews:{}};document.getElementById('panel').replaceChildren();Planning.render(document.getElementById('panel'))}, {fixture,lang,theme});
 assert.equal(await p.$eval('#planning-ownership',e=>e.dataset.planningState),'completed');assert.match(await p.$eval('#planning-ownership > p.notice',e=>e.textContent),lang==='fr'?/Tous les périmètres sont terminés/:/All scopes are complete/);
 const history='[data-planning-history]';assert.equal(await p.$eval(history,e=>e.open),false);await p.focus(history+' summary');await p.keyboard.press('Enter');assert.equal(await p.$eval(history,e=>e.open),true);assert.match(await p.$eval(history,e=>e.textContent),/old timeout/);assert.equal(await p.$eval(history+' summary',e=>e.textContent),lang==='fr'?'Diagnostic conservé dans l’historique':'Diagnostic retained in history');
 assert.equal(await p.evaluate(()=>Array.from(document.querySelectorAll('button')).some(b=>/Reprendre la planification|Resume planning/.test(b.textContent))),false);
 await p.screenshot({path:path.join(out,`history-${lang}-${theme}.png`)});await p.keyboard.press('Enter');assert.equal(await p.$eval(history,e=>e.open),false);assert.equal(await p.evaluate(()=>document.activeElement.tagName),'SUMMARY');
 // Current failure still has an alert and a recovery action.
 await p.evaluate(()=>{snapshot.work.planning.scopes[0].state='waiting';snapshot.work.planning.paused=false;document.getElementById('panel').replaceChildren();Planning.render(document.getElementById('panel'))});assert.equal(await p.$eval('#planning-ownership',e=>e.dataset.planningState),'error');assert.match(await p.$eval('#planning-ownership > p.notice.alert',e=>e.textContent),/old timeout/);
 await p.screenshot({path:path.join(out,`current-${lang}-${theme}.png`)});console.log(JSON.stringify({lang,theme,keyboard_open:'Enter',keyboard_close:'Enter',closed_after_second_enter:true,focus_after_close:'SUMMARY',historical_failure_retained:'old timeout',current_state:await p.$eval('#planning-ownership',e=>e.dataset.planningState),current_error:await p.$eval('#planning-ownership > p.notice.alert',e=>e.textContent),isolated_render:true,real_provider:false}));
 }
 assert.deepEqual(errors,[]);assert.deepEqual(network,[]);fs.writeFileSync(path.join(out,'browser-result.json'),JSON.stringify({status:'PASS',checks:['closed planning with retained failure','current error visible','history keyboard open close focus','FR EN both themes'],console_errors:errors,failed_requests:network},null,2));console.log('PASS QW6 history UI');
 }finally{await browser.close()}})().catch(e=>{console.error(e);process.exitCode=1});
