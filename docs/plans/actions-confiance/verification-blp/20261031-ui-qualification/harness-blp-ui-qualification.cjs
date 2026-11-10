'use strict';
// Bounded browser qualification of the BLP administration section on the real
// installed native server (v0.1.1-dev.282). Uses the ordinary persisted session
// credential as the ordinary cookie; no mock HTTP/DOM; no acceptance claim.
// Qualifies: real GET /trust-native.html, real asset /trust-confidentiality.js,
// dialog open, blp-load business refusal (no trust state / no I3 token) shown
// as a typed message, example button fills JSON, FR/EN both themes, no console
// errors, no page overflow. Evidence (screenshots, checks) stays in the owned
// fixture area, never /tmp.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const root=path.resolve(__dirname,'..');
const pup=require(path.join(root,'node_modules','puppeteer'));
const out=path.join(root,'.swarm-fixtures','blp-ui-evidence');
fs.mkdirSync(out,{recursive:true});
const serverRoot=path.join(root,'.swarm');
const sessionFile='.swarm/web-session-e6ed55c9891a1038';
const secret=fs.readFileSync(path.join(root,sessionFile),'utf8').trim();
const address='127.0.0.1:38433';
const origin=`http://${address}`;
const crypto=require('node:crypto');
const csrf=crypto.createHmac('sha256',secret).update(Buffer.concat([Buffer.from('swarm-local-csrf-v1\0'),Buffer.from(address)])).digest('hex');
const checks=[],errors=[],expectedHttp=[],expectedConsole=[];let browser;
(async()=>{
 browser=await pup.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
 const page=await browser.newPage();
 page.setDefaultTimeout(20000);
 page.on('pageerror',e=>errors.push('pageerror: '+e.message));
 page.on('console',m=>{
  if(m.type()!=='error')return;
  // Chrome omits the URL from fetch-failure console messages; classify by
  // consuming exactly one allowance per already-observed expected response.
  const i=expectedConsole.indexOf('403');
  if(i>=0&&/status of 403/.test(m.text())){expectedConsole.splice(i,1);return;}
  errors.push('console: '+m.text());
 });
 page.on('response',r=>{if(r.status()>=400)expectedHttp.push(r.status()+' '+r.url());});
 await page.setCookie({name:'swarm_session_e6ed55c9891a',value:secret,domain:'127.0.0.1',path:'/',httpOnly:true,sameSite:'Strict'});
 // 1. Real served page with embedded BLP asset
 const resp=await page.goto(origin+'/trust-actions.html',{waitUntil:'networkidle0'});
 assert.equal(resp.status(),200,'trust-actions.html must be served 200');
 const assetLoaded=await page.evaluate(()=>Array.from(document.scripts).some(s=>s.src.endsWith('/trust-confidentiality.js')));
 assert.ok(assetLoaded,'trust-confidentiality.js must be referenced');
 checks.push('real page served; BLP asset referenced');
 // 2. Open the Admin dialog (real click)
 await page.click('#admin');
 await page.waitForSelector('#settings[open]',{timeout:10000});
 checks.push('Admin dialog opens');
 // 3. blp-load: real API call with the shared protected API. No admin token is
 // set, so the server must refuse with the typed authorization message shown
 // by guarded() in #admin-message (dialog open) — one expected 403 response.
 await page.click('#blp-load');
 expectedConsole.push('403'); // armed for the typed blp-show refusal
 await page.waitForFunction(()=>document.getElementById('admin-message').textContent.length>0,{timeout:10000});
 // Let the response event settle before classifying it (event delivery races
 // the DOM update; the 403 always arrives with the typed message).
 await new Promise(r=>setTimeout(r,500));
 const status1=await page.$eval('#admin-message',e=>e.textContent);
 assert.match(status1,/insuffisante|Identité|autoris/i,'blp-load must show the typed authorization refusal, got: '+status1);
 assert.equal(expectedHttp.filter(x=>x.includes('/api/v1/trust-actions?action=blp-show')&&x.startsWith('403')).length,1,'exactly one typed 403 on blp-show');
 checks.push('blp-load business refusal displayed: '+status1+' (one 403 authorization_required)');
 await page.screenshot({path:path.join(out,'01-fr-dialog-blp-load-refusal.png')});
 // 4. Example button fills a no-clearance policy without saving. Its fieldset
 // section lives in a closed <details>; open it first (real user gesture too).
 await page.evaluate(()=>{const d=document.querySelector('#blp-settings details');if(d&&!d.open)d.open=true;});
 await page.click('#blp-example');
 const policyValue=await page.$eval('#blp-policy',e=>e.value);
 const policy=JSON.parse(policyValue);
 assert.equal(policy.schema_version,1);
 assert.ok(Array.isArray(policy.levels)&&policy.levels.length===4);
 assert.equal(Object.keys(policy.clearances||{}).length,0,'example must carry no clearance');
 const status2=await page.$eval('#blp-status',e=>e.textContent);
 assert.match(status2,/Exemple/i);
 checks.push('example fills 4-level no-clearance policy; status: '+status2);
 // Save stays disabled until a preview validates; verify it never enables here.
 assert.equal(await page.$eval('#blp-save',e=>e.disabled),true,'save must stay disabled without preview');
 checks.push('save disabled without validated preview');
 // 5. EN language switch: the button performs a real navigation with ?lang=en.
 // The modal dialog makes everything outside it inert, so close it first — the
 // ordinary user flow too. The reload closes the dialog; reopen it afterwards.
 await page.click('#close');
 await page.waitForSelector('#settings:not([open])',{timeout:10000});
 await page.click('#language');
 await page.waitForNavigation({waitUntil:'networkidle0',timeout:10000}).catch(()=>{});
 assert.ok(page.url().includes('lang=en'),'language button must navigate to ?lang=en, got: '+page.url());
 assert.equal(await page.$eval('html',e=>e.getAttribute('lang')),'en','document must be EN after navigation');
 // Reopen the Admin dialog after the reload and confirm the EN legend.
 await page.click('#admin');
 await page.waitForSelector('#settings[open]',{timeout:10000});
 const legendEn=await page.$eval('#blp-settings legend',e=>e.textContent);
 assert.match(legendEn,/Bell-LaPadula/i,'EN legend must show Bell-LaPadula');
 checks.push('EN legend: '+legendEn);
 // EN refusal message check with a fresh blp-load (second expected 403).
 await page.click('#blp-load');
 expectedConsole.push('403'); // armed for the second typed refusal
 await page.waitForFunction(()=>document.getElementById('admin-message').textContent.length>0,{timeout:10000});
 await new Promise(r=>setTimeout(r,500));
 const statusEn=await page.$eval('#admin-message',e=>e.textContent);
 assert.match(statusEn,/Insufficient|identity|authority|I3/i,'EN typed refusal expected, got: '+statusEn);
 checks.push('EN typed refusal: '+statusEn);
 await page.screenshot({path:path.join(out,'02-en-dialog.png')});
 // 6. Theme toggle: close the modal first (header is inert behind the dialog),
 // then toggle sombre→etat (light) and assert the real attribute change.
 await page.click('#close');
 await page.waitForSelector('#settings:not([open])',{timeout:10000});
 assert.equal(await page.$eval('html',e=>e.getAttribute('data-theme')),'sombre','initial theme must be sombre');
 await page.click('#theme');
 const theme=await page.$eval('html',e=>e.getAttribute('data-theme'));
 assert.equal(theme,'etat','toggle must switch to etat (light theme), got: '+theme);
 checks.push('theme toggled to: '+theme);
 await page.screenshot({path:path.join(out,'03-'+theme+'-en.png')});
 // 7. No page overflow at mobile width
 await page.setViewport({width:390,height:844});
 await new Promise(r=>setTimeout(r,300));
 const overflow=await page.evaluate(()=>document.documentElement.scrollWidth>document.documentElement.clientWidth);
 assert.ok(!overflow,'no horizontal overflow at mobile width');
 checks.push('no horizontal overflow at 390px');
 assert.deepEqual(errors,[],'no console/page errors expected beyond the two typed 403s; got: '+JSON.stringify(errors));
 fs.writeFileSync(path.join(out,'checks.json'),JSON.stringify({checks,errors,expectedHttp,sessionFile,server:'v0.1.1-dev.282',at:new Date().toISOString()},null,2));
 console.log('PASS',checks.length,'checks; evidence in',out);
})().catch(e=>{console.error('FAIL',e.message);console.error('errors:',JSON.stringify(errors,null,1));process.exitCode=1;}).finally(async()=>{if(browser)await browser.close();});
