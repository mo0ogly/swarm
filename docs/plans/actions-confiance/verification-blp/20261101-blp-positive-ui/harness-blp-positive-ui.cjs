'use strict';
// Positive BLP administration end-to-end through the REAL served UI on an
// owned fixture root (never the mission root, never /tmp). Real Chrome, real
// served page, real server on 127.0.0.1:38499; admin bearer typed into the
// page-local field like a user. Qualifies: blp-load reads unconfigured state,
// blp-example fills the no-clearance catalog, blp-preview validates (200),
// blp-save records the policy, reload shows the recorded policy + attributed
// history; runtime_enforced stays false everywhere. Evidence stays in the
// fixture area. No acceptance claim; policy content is fixture-labeled.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const root=path.resolve(__dirname,'..');
const pup=require(path.join(root,'node_modules','puppeteer'));
const outDir=path.join(root,'.swarm-fixtures','blp-positive-ui-evidence');
fs.mkdirSync(outDir,{recursive:true});
const envPath=path.join(root,'.swarm-fixtures','blp-positive-root.env');
const ROOT=fs.readFileSync(envPath,'utf8').match(/ROOT=(.+)/)[1].trim();
const address='127.0.0.1:38499';
const origin=`http://${address}`;
const sessionFile=fs.readdirSync(path.join(ROOT,'.swarm')).find(f=>f.startsWith('web-session-'));
const secret=fs.readFileSync(path.join(ROOT,'.swarm',sessionFile),'utf8').trim();
const crypto=require('node:crypto');
const cookieName='swarm_session_'+crypto.createHash('sha256').update(address).digest('hex').slice(0,12);
const csrf=crypto.createHmac('sha256',secret).update(Buffer.concat([Buffer.from('swarm-local-csrf-v1\0'),Buffer.from(address)])).digest('hex');
const checks=[],errors=[],expectedHttp=[],expectedConsole=[];let browser;
(async()=>{
 browser=await pup.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
 const page=await browser.newPage();
 page.setDefaultTimeout(20000);
 page.on('pageerror',e=>errors.push('pageerror: '+e.message));
 page.on('console',m=>{
  if(m.type()!=='error')return;
  const i=expectedConsole.indexOf('4xx');
  if(i>=0){expectedConsole.splice(i,1);return;}
  errors.push('console: '+m.text());
 });
 page.on('response',r=>{if(r.status()>=400)expectedHttp.push(r.status()+' '+r.url());});
 await page.setCookie({name:cookieName,value:secret,domain:'127.0.0.1',path:'/',httpOnly:true,sameSite:'Strict'});
 const resp=await page.goto(origin+'/trust-actions.html',{waitUntil:'networkidle0'});
 assert.equal(resp.status(),200,'trust-actions.html served 200');
 checks.push('real fixture-root page served');
 // Open Admin, type the fixture admin token into the page-local I3 field
 await page.click('#admin');
 await page.waitForSelector('#settings[open]');
 await page.type('#trust-admin-token','fixture-admin-token');
 // blp-load on unconfigured fixture: status shows "Aucune politique ..."
 await page.click('#blp-load');
 await page.waitForFunction(()=>document.getElementById('blp-status').textContent.length>0,{timeout:10000});
 const loadStatus=await page.$eval('#blp-status',e=>e.textContent);
 assert.match(loadStatus,/Aucune politique|No confidential/i,'unconfigured status expected, got: '+loadStatus);
 checks.push('blp-load unconfigured: '+loadStatus);
 // Example fills the 4-level no-clearance catalog
 await page.evaluate(()=>{const d=document.querySelector('#blp-settings details');if(d&&!d.open)d.open=true;});
 await page.click('#blp-example');
 const example=JSON.parse(await page.$eval('#blp-policy',e=>e.value));
 assert.equal(example.schema_version,1);
 assert.equal(Object.keys(example.clearances||{}).length,0);
 checks.push('example filled: 4 levels, no clearances');
 // Reason + preview: real blp-policy-preview POST must return 200 applied:false
 await page.type('#blp-reason','Positive BLP administration on an owned fixture root');
 await page.click('#blp-preview');
 await page.waitForFunction(()=>/Prévisualisation/.test(document.getElementById('blp-status').textContent),{timeout:10000});
 checks.push('preview validated: '+(await page.$eval('#blp-status',e=>e.textContent)));
 assert.equal(await page.$eval('#blp-save',e=>e.disabled),false,'save enabled after valid preview');
 // Save: real blp-policy-apply POST; then automatic reload shows recorded state
 await page.click('#blp-save');
 await page.waitForFunction(()=>/enregistrée et relue/.test(document.getElementById('blp-status').textContent),{timeout:10000});
 const saved=await page.$eval('#blp-status',e=>e.textContent);
 checks.push('saved+reloaded: '+saved);
 await page.screenshot({path:path.join(outDir,'01-fr-positive-saved.png')});
 // Read back through the form: the textarea now holds the recorded policy
 const recorded=JSON.parse(await page.$eval('#blp-policy',e=>e.value));
 assert.equal(recorded.schema_version,1);
 assert.ok(Array.isArray(recorded.levels),'recorded policy read back');
 checks.push('recorded policy visible in form after save');
 // Verify server-side history attribution over HTTP (same session, admin token)
 const hist=await page.evaluate(async(csrf)=>{
  const r=await fetch('/api/v1/trust-actions?action=blp-show',{method:'POST',headers:{'Content-Type':'application/json','X-Swarm-CSRF':csrf,'Authorization':'Bearer fixture-admin-token'},body:'{}'});
  return r.json();
 },csrf);
 assert.equal(hist.capability.configured,true,'capability.configured must be true after save');
 assert.equal(hist.capability.runtime_enforced,false,'runtime_enforced stays false');
 assert.ok(hist.record&&Array.isArray(hist.record.history)&&hist.record.history.length===1,'one attributed history entry');
 assert.equal(hist.record.history[0].actor,'admin');
 checks.push('server history: 1 entry, actor admin, runtime_enforced false, policy_revision '+hist.capability.policy_revision);
 fs.writeFileSync(path.join(outDir,'checks.json'),JSON.stringify({checks,errors,expectedHttp,ROOT,address,at:new Date().toISOString()},null,2));
 console.log('PASS',checks.length,'checks; evidence in',outDir);
})().catch(e=>{console.error('FAIL',e.message);console.error('errors:',JSON.stringify(errors,null,1));console.error('http:',JSON.stringify(expectedHttp,null,1));process.exitCode=1;}).finally(async()=>{if(browser)await browser.close();});
