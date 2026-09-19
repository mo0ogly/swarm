'use strict';
const fs=require('fs'),os=require('os'),path=require('path'),assert=require('assert/strict'),{spawn,execFileSync}=require('child_process'),p=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-help-'));fs.mkdirSync(out,{recursive:true});execFileSync(binary,['--root',root,'init']);
const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});let browser;
(async()=>{const url=await new Promise((resolve,reject)=>{let text='';server.stdout.on('data',d=>{text+=d;const match=text.match(/http:\/\/\S+\/session\/\S+/);if(match)resolve(match[0])});server.on('exit',()=>reject(Error('server stopped')))});browser=await p.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage(),errors=[],checks=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(url);
for(const route of ['/','/prepare.html','/terminal.html']){
 await page.goto(new URL(route,url).href);await page.waitForSelector('[data-context-help]');
 // Expose each static surface in this empty fixture; no business action is run.
 const keys=await page.$$eval('[data-context-help]',es=>es.map(e=>e.dataset.contextHelp));
 for(const key of [...new Set(keys)]){
 const selector='[data-context-help="'+key+'"]';await page.$eval(selector,e=>{for(let n=e;n;n=n.parentElement){n.hidden=false;if(n.tagName==='DETAILS')n.open=true}});
 await page.click(selector);await page.waitForSelector('#context-help[open]');assert.equal(await page.$$eval('.context-help-body section',es=>es.length),3);
 assert.equal(await page.$eval('#context-help',e=>e.getAttribute('aria-labelledby')),'context-help-title');await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.getElementById('context-help').open);assert.equal(await page.$eval(selector,e=>e===document.activeElement),true);
 await page.click(selector);await page.click('#context-help footer button');assert.equal(await page.$eval(selector,e=>e===document.activeElement),true);checks.push(route+': '+key);
 }
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await page.click('[data-context-help]');await page.setViewport({width:1100,height:850});await page.screenshot({path:path.join(out,route==='/'?'cockpit-'+theme+'.png':route.slice(1)+'-'+theme+'.png')});const missing=await page.evaluate(()=>{const css=[...document.styleSheets].flatMap(s=>[...s.cssRules].map(r=>r.cssText)).join(' '),style=getComputedStyle(document.documentElement);return [...new Set([...css.matchAll(/var\((--wattson-[\w-]+)/g)].map(m=>m[1]))].filter(t=>!style.getPropertyValue(t).trim())});assert.deepEqual(missing,[]);await page.keyboard.press('Escape')}
 await page.setViewport({width:390,height:844});await page.click('[data-context-help]');assert.equal(await page.$eval('#context-help',e=>e.getBoundingClientRect().right<=innerWidth),true);await page.screenshot({path:path.join(out,(route==='/'?'cockpit':route.slice(1))+'-mobile.png')});await page.keyboard.press('Escape');await page.setViewport({width:1100,height:850});
}
assert.deepEqual(errors,[]);const result={status:'PASS',checks,themes:['etat','sombre'],keyboard:true,focusRestored:true,mobile:true,errors};fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));console.log(result);
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();server.kill()});
