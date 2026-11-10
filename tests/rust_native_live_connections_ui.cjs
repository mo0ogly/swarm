'use strict';
/** Regression: many live windows in one browser profile must not exhaust the
 * HTTP/1.1 per-host connection pool (Chrome: 6). Opens the dashboard plus seven
 * harness windows, then requires a fresh JSON request from every window to
 * complete. Waits are on observable completion; the only bound is a measured
 * stall threshold (STALL_MS) far above normal local latency, used to report a
 * pending request as starved rather than to pass a slow one.
 */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),output=path.resolve(cfg.output);fs.mkdirSync(output,{recursive:false});
const base='http://'+cfg.address,STALL_MS=15000;
(async()=>{
 const browser=await puppeteer.launch({headless:true,timeout:0,protocolTimeout:0,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',args:['--no-sandbox']});
 const result={status:'FAIL',windows:0,latencies_ms:[],starved:0};
 try{
  const context=await browser.createBrowserContext(),login=await context.newPage();login.setDefaultTimeout(0);login.setDefaultNavigationTimeout(0);
  await login.goto(base+'/login',{waitUntil:'load'});await login.type('#access-key',cfg.key);await login.$eval('input[name=next]',e=>e.value='/api/v1/works');
  await Promise.all([login.waitForNavigation({waitUntil:'load'}),login.click('button[type=submit]')]);
  const pages=[];
  const open=async url=>{const p=await context.newPage();p.setDefaultTimeout(0);p.setDefaultNavigationTimeout(0);await p.goto(base+url,{waitUntil:'domcontentloaded'});pages.push(p);return p};
  await open('/live.html');
  for(let i=0;i<7;i++)await open('/live-session.html?agent=dev&work='+cfg.works[0]);
  result.windows=pages.length;
  for(const p of pages){
   const ms=await p.evaluate(async limit=>{const t=performance.now();const r=await Promise.race([fetch('/api/v1/live/fingerprint',{cache:'no-store'}).then(r=>r.ok?performance.now()-t:-1),new Promise(ok=>setTimeout(()=>ok(null),limit))]);return r},STALL_MS);
   if(ms===null)result.starved++;else result.latencies_ms.push(Math.round(ms));
  }
  assert.equal(result.starved,0,'requests starved by the connection pool');
  result.status='PASS';
 }catch(error){result.error=String(error.stack||error)}
 finally{fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({...result,checks:[result.status]},null,2)+'\n');await browser.close();process.exitCode=result.status==='PASS'?0:1}
})();
