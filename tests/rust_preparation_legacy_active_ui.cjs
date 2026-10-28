'use strict';
/** Protected installed-candidate history and explicit stop with no model authority.
 * Four real FR/EN×theme journeys, manual refresh only; no provider or watchdog. */
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[],posts=[];
function check(name,value){assert(value,name);checks.push(name);fs.writeFileSync(path.join(cfg.output,'active-browser-checks.json'),JSON.stringify(checks,null,2)+'\n');console.log('PASS '+name);}
(async()=>{const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
try{for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
 const context=await browser.createBrowserContext(),page=await context.newPage(),label=lang+'-'+theme;
 page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{if(r.method()==='POST')posts.push(new URL(r.url()).pathname)});
 const [name,value]=cfg.cookie.split('=');await context.setCookie({name,value,url:'http://'+cfg.address,httpOnly:true,sameSite:'Strict'});
 await page.goto('http://'+cfg.address+'/prepare.html?'+new URLSearchParams({id:cfg.preparation,lang,theme}),{waitUntil:'networkidle0'});
 await page.waitForSelector('#chat-recovery button');
 check(label+'-active-visible',await page.$eval('#chat-turns',e=>e.textContent.includes('Historical question')));
 check(label+'-private-omitted',await page.$eval('#chat',e=>!e.textContent.includes('PRIVATE_')));
 check(label+'-calls-remain-hidden',await page.$eval('#chat-form',e=>e.hidden));
 check(label+'-stop-labelled',await page.$eval('#chat-recovery button',e=>e.textContent.includes(e.ownerDocument.documentElement.lang==='en'?'Request or check stop':'Demander ou vérifier l’arrêt')));
 await page.focus('#chat-recovery button');check(label+'-keyboard-focus',await page.$eval('#chat-recovery button',e=>e===document.activeElement));
 check(label+'-stop-contrast',await page.$eval('#chat-recovery button',e=>{
  const rgb=c=>c.match(/[\d.]+/g).slice(0,3).map(Number),lum=c=>rgb(c).map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4}).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);
  let n=e,bg;while(n){bg=getComputedStyle(n).backgroundColor;if(bg!=='transparent'&&bg!=='rgba(0, 0, 0, 0)')break;n=n.parentElement;}
  const a=lum(getComputedStyle(e).color),b=lum(bg);return (Math.max(a,b)+.05)/(Math.min(a,b)+.05)>=4.5;
 }));
 await page.keyboard.press('Enter');await page.waitForFunction(()=>!!document.querySelector('#chat-recovery button:not(:disabled)')&&!document.querySelector('#chat-status').textContent.includes('Chargement'));
 check(label+'-stopping-preserved',await page.$eval('#chat-turns',e=>e.textContent.includes(document.documentElement.lang==='en'?'Stop requested':'Arrêt demandé')));
 for(const width of [1440,390]){await page.setViewport({width,height:1000});check(label+'-no-overflow-'+width,await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:path.join(cfg.output,'legacy-active-'+label+'-'+width+'.png'),fullPage:true});}
 if(label==='fr-etat'){
  await page.setRequestInterception(true);page.on('request',r=>new URL(r.url()).pathname==='/api/v1/preparations/stop'?r.respond({status:200,contentType:'application/json',body:'{}'}):r.continue());
  await page.click('#chat-recovery button');await page.waitForFunction(()=>document.querySelector('#chat-status').classList.contains('attention'));
  check('legacy-active-invalid-receipt-visible',await page.$eval('#chat-status',e=>e.textContent.includes('Arrêt non confirmé'))&&await page.$eval('#chat-recovery button',e=>!e.disabled));
  await page.setRequestInterception(false);page.removeAllListeners('request');page.on('request',r=>{if(r.method()==='POST')posts.push(new URL(r.url()).pathname)});
  await page.click('#chat-refresh');await page.waitForSelector('#chat-recovery button');check('legacy-active-error-recovery',await page.$eval('#chat-status',e=>!e.classList.contains('attention')));
 }
 await context.close();
}
check('legacy-active-no-page-errors',errors.length===0);check('legacy-active-only-explicit-stop-posts',posts.length===5&&posts.every(p=>p==='/api/v1/preparations/stop'));
fs.writeFileSync(path.join(cfg.output,'active-browser-result.json'),JSON.stringify({status:'PASS',checks,errors,posts},null,2)+'\n');console.log('LEGACY_ACTIVE_BROWSER_PASS '+checks.length);
}finally{await browser.close();}})().catch(e=>{console.error(e.stack);process.exitCode=1});
