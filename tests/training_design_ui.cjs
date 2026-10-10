'use strict';
// Verify embedded refreshed media and the same reader extracted from each kit.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const {execFileSync,spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);
fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-training-reader-'));
execFileSync(binary,['--root',root,'init']);
const server=spawn(binary,['--root',root,'web','127.0.0.1:0']);let browser,page;
const errors=[],failures=[],checks=[];
async function metadata(selector,duration){
 await page.$eval(selector,e=>e.load());
 await page.waitForFunction((s)=>document.querySelector(s).readyState>=1,{},selector);
 const result=await page.$eval(selector,e=>({duration:e.duration,width:e.videoWidth,height:e.videoHeight,paused:e.paused,autoplay:e.autoplay}));
 assert(Math.abs(result.duration-duration)<.1);assert(result.width>0&&result.height>0);
 assert(result.paused&&!result.autoplay);return result;
}
async function reader(prefix,offline){
 for(const lang of ['fr','en']){
  await page.goto(prefix+'?lang='+lang,{waitUntil:'load'});
  await page.waitForSelector('#chapters button');assert.equal(await page.$eval('html',e=>e.lang),lang);
  assert((await page.$eval('#scope',e=>e.textContent)).includes('10'));
  assert.equal(await page.$eval('#graph-intro',e=>e.hidden),lang==='en');
  assert.equal(await page.$eval('#kit-downloads',e=>e.hidden),offline);
  for(let i=0;i<4;i++){
   await page.click('#chapters button:nth-child('+(i+1)+')');
   const count=[4,4,5,7][i];
   for(let n=0;n<count;n++){
    await page.waitForFunction(()=>{const e=document.querySelector('#step-image');return e.complete&&e.naturalWidth>0});
    if(n<count-1)await page.click('#next');
   }
   await page.click('#video-mode');
   const result=await metadata('#video',[24,24,30,42][i]);
   if(i<2)assert.equal(result.width,1280);
   // Real decoder/playback and seeking, followed by an explicit pause.
   await page.$eval('#video',async e=>{await e.play();e.pause();e.currentTime=12});
   await page.waitForFunction(()=>Math.abs(document.querySelector('#video').currentTime-12)<.1);
   assert((await page.$eval('#download-video',e=>e.getAttribute('href'))).endsWith('-'+lang+'.mp4'));
   checks.push({offline,lang,module:i+1,...result});await page.click('#step-mode');
  }
  if(lang==='fr'){const r=await metadata('#graph-intro-video',63);assert.equal(r.width,1280);checks.push({offline,lang,module:'graph-intro',...r})}
  for(const theme of ['etat','sombre']){
   if(await page.$eval('html',e=>e.dataset.theme)!==theme)await page.click('#theme');
   await page.setViewport({width:320,height:900});
   assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
   await page.setViewport({width:1440,height:1100});
  }
 }
}
(async()=>{
 const session=await new Promise((resolve,reject)=>{let s='';const t=setTimeout(()=>reject(Error('startup timeout')),15000);server.stdout.on('data',x=>{s+=x;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(t);resolve(m[0])}})});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});page=await browser.newPage();await page.setViewport({width:1440,height:1100});
 page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
 page.on('response',r=>{if(r.status()>=400)failures.push(r.status()+' '+new URL(r.url()).pathname)});
 // Browser cancellation of media reads during load()/module changes is expected.
 page.on('requestfailed',r=>{if(r.failure()?.errorText==='net::ERR_ABORTED'&&r.resourceType()==='media')return;failures.push(r.failure()?.errorText+' '+new URL(r.url()).pathname)});
 await page.goto(session,{waitUntil:'networkidle0'});
 await reader(new URL(session).origin+'/training/casa-pizza/tutoriels/',false);
 for(const kit of ['Kit_Formation_Swarm_Casa_Pizza.zip','Swarm_Casa_Pizza_Training_Kit_EN.zip']){
  const dir=path.join(root,kit.slice(0,-4));fs.mkdirSync(dir);
  execFileSync('python3',['-c','import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])',path.resolve('docs/training/casa-pizza',kit),dir]);
  await reader('file://'+path.join(dir,'tutoriels/index.html'),true);
 }
 await page.goto(new URL(session).origin+'/training/casa-pizza/tutoriels/?lang=fr',{waitUntil:'load'});
 await page.click('#next');await page.waitForFunction(()=>document.querySelector('#step-image').complete);
 await page.screenshot({path:path.join(out,'training-refreshed-fr.png'),fullPage:true});
 assert.deepEqual(errors,[]);assert.deepEqual(failures,[]);
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',binary,checks,errors,failures},null,2));console.log('PASS embedded and both offline kits: FR/EN, 4 modules, video decoding/seek, 63s introduction, themes/mobile');
})().catch(async e=>{console.error(e);if(page)await page.screenshot({path:path.join(out,'failure.png'),fullPage:true});fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({error:e.stack,errors,failures},null,2));process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
