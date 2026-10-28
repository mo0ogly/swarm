'use strict';
// Real server, isolated fixture; no agents or provider calls. No global watchdog.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]||'bin/swarm'),out=path.resolve(process.argv[3]||'test-results/window-identity');
fs.mkdirSync(out,{recursive:true});
const fixture=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-windows-'));
const cli=args=>JSON.parse(execFileSync(binary,['--root',fixture,'--json',...args],{encoding:'utf8'}));
cli(['init']);
const app=spawn(binary,['--root',fixture,'web','127.0.0.1:0']);
let browser;const errors=[],failures=[],checks=[];
async function headingBrand(page,selector){
 const result=await page.$eval(selector,e=>{
  const s=getComputedStyle(e,'::before');return {image:s.backgroundImage,width:parseFloat(s.width),height:parseFloat(s.height)};
 });
 assert.match(result.image,/\/swarm-logo\.png/,selector+' has the canonical logo');
 assert.ok(result.width>=28&&result.height>=28,selector+' has a visible logo');
}
async function fits(page,label){
 const overflow=await page.evaluate(()=>document.documentElement.scrollWidth-innerWidth);
 assert.ok(overflow<=1,label+': global overflow '+overflow);
}
async function allDialogs(page,label){
 const ids=await page.$$eval('dialog',es=>es.map(e=>e.id));
 for(const id of ids){
  const title=await page.$eval('#'+id,e=>e.getAttribute('aria-labelledby'));
  assert.ok(title,id+' has an accessible title');
  await headingBrand(page,'#'+title);
  await page.$eval('#'+id,e=>e.showModal());
  const geometry=await page.$eval('#'+title,e=>{const r=e.getBoundingClientRect();return {width:r.width,height:r.height,scroll:e.scrollWidth}});
  assert.ok(geometry.width>0&&geometry.height>0,label+' '+id+' title visible');
  assert.ok(geometry.scroll<=geometry.width+1,label+' '+id+' title fits');
  await fits(page,label+' '+id);await page.keyboard.press('Escape');
  assert.equal(await page.$eval('#'+id,e=>e.open),false,id+' Escape closes');
 }
 checks.push({label,dialogs:ids.length});
}
(async()=>{
 const session=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});app.on('error',reject);app.on('exit',c=>reject(Error('server exited '+c)))});
 const base=new URL(session).origin;
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
 page.on('response',r=>{if(r.status()>=400)failures.push(r.status()+' '+new URL(r.url()).pathname)});
 await page.goto(session,{waitUntil:'networkidle0'});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  console.log('Checking windows '+lang+' '+theme);
  await page.evaluate(t=>localStorage.setItem('swarm-theme',t),theme);
  for(const [route,brand]of [['/',''],['/prepare.html','.prep-header img'],['/projects.html','.project-brand img'],['/project-memory.html','.memory-header img'],['/processes.html','.pm-header img'],['/trust-actions.html','.trust-header img']]){
   await page.setViewport({width:1440,height:1000});
   await page.goto(base+route+'?lang='+lang,{waitUntil:'networkidle0'});
   await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);
   if(brand)assert.equal(await page.$eval(brand,e=>e.complete&&e.naturalWidth>0),true,route+' logo loads');
   if(route==='/'){
    // Exercise public controls for both a reused form and a dynamic dialog.
    await page.$eval('[data-nav-group="configuration"]',e=>e.open=true);
    await page.click('[data-view="providers"]');
    await page.waitForFunction(()=>!document.getElementById('connections-add').disabled);
    await page.click('#connections-add');await page.waitForSelector('#modal[open]');
    await headingBrand(page,'#modal-title');
    await page.screenshot({path:path.join(out,'connection-'+lang+'-'+theme+'.png')});
    await page.keyboard.press('Escape');
    assert.equal(await page.evaluate(()=>document.activeElement.id),'connections-add');
    await page.evaluate(()=>document.querySelector('[data-version-open]').click());
    await page.waitForSelector('#version-dialog[open]');
    await headingBrand(page,'#version-dialog-title');await page.keyboard.press('Escape');
    await page.click('#nav-collapse');
    assert.equal(await page.$eval('.workspace-logo',e=>getComputedStyle(e).display!=='none'&&e.complete&&e.naturalWidth>0),true,'collapsed desktop brand');
    await page.click('#nav-collapse');
   }
   assert.deepEqual(await page.$$eval('link[rel=icon]',es=>es.map(e=>new URL(e.href).pathname)),['/swarm-logo.png'],route+' favicon');
   if(route==='/projects.html'){
    await page.click('#project-method-open');await page.waitForSelector('#project-method-dialog[open]');
    await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>document.activeElement.id),'project-method-open');
   }
   for(const width of [1440,1024,760,390,320]){
    await page.setViewport({width,height:1000});await fits(page,route+' '+width);
    if(width===1440||width===320)await allDialogs(page,route+' '+lang+' '+theme+' '+width);
    if(route==='/'&&width===320)assert.equal(await page.$eval('.mobile-header img',e=>e.complete&&e.naturalWidth>0&&e.getBoundingClientRect().width>=28),true,'mobile logo');
   }
   if(route==='/prepare.html'){
    await page.click('#help');await page.waitForSelector('#help-dialog[open]');
    assert.equal(await page.evaluate(()=>document.activeElement.id),'help-title','long help starts at its title');
    assert.ok(await page.$eval('#help-title',e=>{const r=e.getBoundingClientRect();return r.top>=0&&r.bottom<=innerHeight}),'help title and logo visible at opening');
    await page.screenshot({path:path.join(out,'help-mobile-'+lang+'-'+theme+'.png')});await page.keyboard.press('Escape');
    assert.equal(await page.evaluate(()=>document.activeElement.id),'help');
   }
  }
 }
 // The OIDC page immediately redirects in normal use. Inspect its transient
 // loading markup without executing that redirect; authentication is unchanged.
 const callback=await browser.newPage();await callback.setJavaScriptEnabled(false);
 await callback.setViewport({width:320,height:680});
 await callback.goto(base+'/trust-oidc-callback.html',{waitUntil:'networkidle0'});
 assert.equal(await callback.$eval('.window-brand img',e=>e.complete&&e.naturalWidth>0),true,'OIDC loading logo');
 assert.deepEqual(await callback.$$eval('link[rel=icon]',es=>es.map(e=>new URL(e.href).pathname)),['/swarm-logo.png']);
 await fits(callback,'OIDC loading page');await callback.close();
 assert.deepEqual(errors,[],'browser errors');assert.deepEqual(failures,[],'HTTP errors');
 const result={checks,errors,failures,fixture,binary,limits:'Geometry checks open static dialogs without submitting; public controls cover connection, version, help and method. No real providers or live deployment.'};
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2)+'\n');
 console.log('PASS '+checks.length+' page/theme/width groups');
})().catch(e=>{fs.writeFileSync(path.join(out,'failure.txt'),e.stack+'\n');console.error(e.stack);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();app.kill()});
