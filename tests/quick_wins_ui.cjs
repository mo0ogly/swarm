'use strict';
// End-to-end recette for plan-843bb3ce22-T6 quick wins, FR/EN x 2 themes:
// QW1 (REQ-VER-01/02): server version shown or explicitly unknown; comparison
//   against the local git HEAD is functional in degraded mode (no git source,
//   or a source that does not match) without any network request.
// QW2 (REQ-LINK-01): a link to a deleted/purged mission shows a clear message
//   and a "Choisir une mission" CTA instead of a silent or generic failure.
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{execSync}=require('child_process'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);
fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-quick-wins-ui-'));
let app,browser;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',c=>c?reject(Error(err||text)):resolve(text?JSON.parse(text):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);
 // A git checkout at root, with a HEAD that cannot match the tested binary's
 // baked-in revision, exercises the "comparison functional but not current"
 // degraded branch without contacting any network.
 execSync('git init -q && git -c user.email=t@t -c user.name=t commit -q --allow-empty -m init',{cwd:root});
 const w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Quick wins test',objective:'Verify QW1/QW2',scope:'isolated',criteria:['No network needed for version comparison','Clear message on deleted mission link'],next:'Load the cockpit'})).work;
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
 const loopbackHost=new URL(url).host;
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const checks=[],errors=[],offHost=[];
 try{
  for(const lang of ['fr','en']){
   const page=await browser.newPage();page.setDefaultTimeout(30000);
   page.on('pageerror',e=>errors.push(lang+': '+e.message));
   page.on('request',r=>{const h=new URL(r.url()).host;if(h&&h!==loopbackHost)offHost.push(lang+': '+r.url())});
   await page.setViewport({width:1400,height:1000});

   // QW1 — version display + degraded, network-free comparison.
   const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang',lang);
   await page.goto(u.href);
   await page.waitForFunction(()=>typeof snapshot!=='undefined'&&snapshot?.work);
   await page.waitForFunction(()=>document.getElementById('server-version').textContent.trim().length>0);
   const versionText=await page.$eval('#server-version',e=>e.textContent);
   const versionTitle=await page.$eval('#server-version',e=>e.title);
   assert(!/undefined|NaN|\[object/.test(versionText),lang+': malformed version text: '+versionText);
   assert(lang==='fr'?/^Version/.test(versionText):/^Version/.test(versionText),lang+': unexpected version text: '+versionText);
   assert(versionTitle.length>0,lang+': version badge missing explanatory title');
   checks.push(lang+': version badge shows a non-empty, explicit state ("'+versionText+'")');

   for(const theme of ['etat','sombre']){
    await page.evaluate(t=>setTheme(t),theme);
    await page.screenshot({path:path.join(out,'version-'+lang+'-'+theme+'.png')});
   }
   checks.push(lang+': version badge captured in both themes without a network request beyond the local server');

   // QW2 — link to a deleted/purged mission.
   const gone=new URL(url);gone.searchParams.set('work','w-does-not-exist-'+lang);gone.searchParams.set('lang',lang);
   await page.goto(gone.href);
   await page.waitForFunction(()=>typeof view!=='undefined'&&view==='manage');
   await page.waitForFunction(()=>!document.getElementById('message').hidden&&document.getElementById('message').textContent.length>0);
   const noticeText=await page.$eval('#message',e=>e.textContent);
   assert.match(noticeText,lang==='fr'?/mission ouverte par ce lien n.existe plus/:/mission opened by this link no longer exists/,lang+': unexpected notice text: '+noticeText);
   assert.match(noticeText,lang==='fr'?/Choisir une mission/:/Choose a mission/,lang+': missing CTA label in notice: '+noticeText);
   assert.equal(await page.$eval('#manage',e=>e.hidden),false,lang+': manage view not shown for a deleted-mission link');
   checks.push(lang+': deleted-mission link shows a clear notice ("'+noticeText+'")');

   for(const theme of ['etat','sombre']){
    await page.evaluate(t=>setTheme(t),theme);
    await page.screenshot({path:path.join(out,'deleted-mission-'+lang+'-'+theme+'.png')});
   }

   const ctaButtons=await page.$$('#message button');
   assert.equal(ctaButtons.length,1,lang+': expected exactly one CTA button in the notice');
   await ctaButtons[0].click();
   await page.waitForFunction(()=>document.activeElement&&document.activeElement.id==='manage-search');
   assert.equal(await page.$eval('#manage',e=>e.hidden),false,lang+': CTA did not keep the manage view open');
   checks.push(lang+': "Choisir une mission" CTA focuses the mission search field');

   await page.close();
  }
  assert.deepEqual(errors,[],'page errors: '+JSON.stringify(errors));
  assert.deepEqual(offHost,[],'unexpected non-loopback network request(s): '+JSON.stringify(offHost));
  checks.push('no request left the local server for version display or the deleted-mission notice (REQ-VER-02: no imposed network access)');
  fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks,errors,offHost,at:new Date().toISOString(),root},null,2));
  console.log(checks.join('\n'));
 }finally{await browser.close();app.kill()}
})().catch(e=>{console.error(e);process.exitCode=1}).finally(()=>{try{app&&app.kill()}catch{}});
