'use strict';
// T8: real browser capture of a SAVED run-limits configuration (FR/EN, 2 themes)
// in an isolated temporary root. Usage: node t8_saved_config_shots.cjs BINARY OUTDIR
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-t8-'));fs.mkdirSync(out,{recursive:true});
function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let t='',e='';p.stdout.on('data',x=>t+=x);p.stderr.on('data',x=>e+=x);p.on('exit',c=>c?reject(Error(e||t)):resolve(t?JSON.parse(t):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);
 const w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Install guide capture',objective:'Capture saved limits',scope:'isolated',criteria:['No secret shown'],next:'Configure'})).work;
 const app=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const url=await new Promise((res,rej)=>{let t='';app.stdout.on('data',x=>{t+=x;const m=t.match(/http:\/\/[^\s]+/);if(m)res(m[0])});app.on('exit',c=>rej(Error('server '+c)))});
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const errors=[];
 try{
  for(const lang of ['fr','en']){
   const key='qa-'+lang;
   const p=await browser.newPage();p.setDefaultTimeout(30000);
   p.on('pageerror',e=>errors.push(lang+' pageerror: '+e.message));
   p.on('console',m=>{if(m.type()==='error')errors.push(lang+' console: '+m.text())});
   p.on('requestfailed',r=>errors.push(lang+' requestfailed: '+r.url()));
   await p.setViewport({width:1400,height:1000});
   const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang',lang);await p.goto(u.href);
   await p.waitForFunction(()=>typeof snapshot!=='undefined'&&snapshot?.work);
   if(await p.$eval('[data-view="admin"]',e=>e.getClientRects().length===0))await p.click('#mode');
   await p.click('[data-view="admin"]');
   await p.waitForFunction(()=>document.querySelector('#admin-current').textContent.length>0);
   await p.select('#admin-scope','role');
   await p.$eval('#admin-mission',(e,v)=>{e.value=v},w.id);
   await p.$eval('#admin-key',(e,v)=>{e.value=v},key);
   await p.click('#admin-load');
   await p.waitForFunction(()=>document.querySelector('#admin-current').textContent.length>0);
   await p.click('#admin-current button');
   await p.waitForSelector('#field-silence_seconds');
   for(const [k,v] of Object.entries({silence_seconds:'30',tool_seconds:'120',max_tool_calls:'50',max_repeated_calls:'3',max_consecutive_errors:'2',reason:'install guide'}))
    await p.$eval('#field-'+k,(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},v);
   await p.click('#confirm');
   await p.waitForFunction(()=>!document.querySelector('#preview').hidden);
   await p.click('#confirm');
   await p.waitForFunction(()=>!document.querySelector('#modal').open);
   const saved=await cli(['run-limits','show','role',w.id,key]);
   assert.equal(saved.revision,1);assert.equal(saved.values.silence_seconds,30);
   await p.waitForFunction(()=>/(Révision|Revision) 1/.test(document.querySelector('#admin-current').textContent));
   const txt=await p.$eval('#admin',e=>e.textContent.replace(/Aucun secret n.est affiché sur cet écran\.?|No secret is shown on this screen\.?/gi,''));
   assert(!/api[_-]?key|bearer |secret|password|mot de passe/i.test(txt),'secret-like text');
   await p.$eval('#admin-history',e=>e.scrollIntoView({block:'center'}));
   for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'admin-saved-'+lang+'-'+theme+'.png')})}
   console.log('PASS',lang,'saved revision',saved.revision);
  }
  assert.deepEqual(errors,[]);
 }finally{await browser.close();app.kill()}
})().catch(e=>{console.error('FAIL',e.message);process.exit(1)});
