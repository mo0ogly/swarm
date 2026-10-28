'use strict';
/** Real native Admin journey: no intercepted response, provider or global timer.
 * Own browser context only; retain named checks and safe diagnostics. */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(process.argv[2],'utf8')),checks=[],errors=[];let browser,expectedDenial=false;
function check(name,value){assert(value,name);checks.push(name);console.log('PASS '+name);}
(async()=>{try{
 fs.mkdirSync(cfg.output,{recursive:true});browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
 const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 const at=cfg.cookie.indexOf('=');await context.setCookie({name:cfg.cookie.slice(0,at),value:cfg.cookie.slice(at+1),url:cfg.base_url,httpOnly:true,sameSite:'Strict'});
 page.on('pageerror',e=>errors.push(e.message));page.on('requestfailed',r=>errors.push(r.failure()?.errorText));page.on('console',m=>{if(m.type()==='error'&&!(expectedDenial&&m.text().includes('403')))errors.push(m.text());});
 page.on('response',r=>{if(r.status()>=400&&!(expectedDenial&&r.status()===403&&r.url().endsWith('action=evidence-key-activate')))errors.push(r.url()+':'+r.status());});
 async function fill(id,value){await page.$eval('#'+id,(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}));},String(value));}
 async function ready(){await page.waitForFunction(()=>document.querySelector('#message').textContent.includes('Prêt')||document.querySelector('#message').textContent.includes('Ready'));}
 async function click(id,operation){const response=page.waitForResponse(r=>r.request().method()==='POST'&&r.url().endsWith('action='+operation));await page.click('#'+id);const r=await response;await page.waitForFunction(()=>!document.querySelector('#close').disabled);return {status:r.status(),body:await r.json()};}
 await page.setViewport({width:1440,height:1100});await page.goto(cfg.base_url+'/trust-actions.html?lang=en',{waitUntil:'networkidle0'});await ready();await page.click('#admin');await page.waitForFunction(()=>document.querySelector('#settings').open);
 check('Admin-role-matrix-visible',await page.$$eval('#trust-role-pairs input',e=>e.length===6));
 check('Admin-principal-declarations-visible',await page.$$eval('#trust-role-principals input',e=>e.length===5&&e.every(v=>v.value)));
 await fill('trust-admin-token',cfg.tokens.admin);await fill('identities',JSON.stringify(cfg.configuration.identities));await fill('reason','Browser persists explicit governance policy');
 await fill('trust-signing-ttl','173');
 check('Admin-preview', (await click('preview','config-preview')).body.applied===false);
 check('Absent-enrollment-stays-disabled-after-preview',await page.$eval('#trust-enrolled-execute',e=>e.disabled));
 check('Admin-save',(await click('save','config-apply')).status===200);
 await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#settings').open);await page.click('#admin');await page.waitForFunction(()=>document.querySelector('#settings').open);
 check('Admin-signing-policy-reopened',await page.$eval('#trust-signing-ttl',e=>e.value)==='173');
 check('Admin-private-fields-cleared',await page.$eval('#identities',e=>e.value)===''&&await page.$eval('#trust-admin-token',e=>e.value)==='');
 await fill('trust-admin-token',cfg.tokens.admin);await fill('reason','Browser prepares attributed key decision');
 const prepared=await click('trust-key-prepare','evidence-key-prepare');check('UI-key-prepared',prepared.status===200);const id=prepared.body.key_id;await page.select('#trust-key-id',id);
 expectedDenial=true;const self=await click('trust-key-activate','evidence-key-activate');check('UI-self-activation-denied',self.status===403);check('Absent-enrollment-stays-disabled-after-denial',await page.$eval('#trust-enrolled-execute',e=>e.disabled));expectedDenial=false;
 await fill('trust-admin-token',cfg.tokens['second-admin']);const activated=await click('trust-key-activate','evidence-key-activate');check('UI-second-Admin-activation',activated.body.registry.keys[id].status==='active');
 await page.select('#trust-key-id',id);await click('trust-public-anchor','public-anchor');
 check('UI-independent-pin-not-invented',await page.$eval('#trust-public-pin',e=>e.value)==='');
 const pin=activated.body.registry.keys[id];await fill('trust-public-pin',JSON.stringify(pin));
 const verifier=await page.$('details:has(#trust-public-verify)');await verifier.evaluate(e=>e.open=true);
 check('UI-public-signature-verifies',(await click('trust-public-verify','public-verify')).body.signature_verified===true);
 check('UI-checkpoint-absence-explicit',await page.$eval('#trust-public-verdict',e=>e.textContent).then(t=>t.includes('without independent checkpoint')));
 const anchor=JSON.parse(await page.$eval('#trust-public-anchor-input',e=>e.value));await fill('trust-public-checkpoint',JSON.stringify({anchor,trusted_key:pin}));
 check('UI-checkpoint-verifies',(await click('trust-public-verify','public-verify')).body.checkpoint_verified===true);
 await fill('trust-public-checkpoint','null');check('UI-old-verdict-invalidated',await page.$eval('#trust-public-verdict',e=>e.textContent)==='');
 await page.select('#trust-audit-action',cfg.action_id);const audit=await click('trust-audit-read','audit-transaction');
 check('UI-audit-transaction',audit.body.action_id===cfg.action_id&&audit.body.chain_verified&&audit.body.access_recorded);
 check('UI-audit-summary-rendered',await page.$eval('#trust-audit-summary',e=>e.textContent).then(v=>v.includes('Transaction chain verified')&&v.includes('Events')));
 check('UI-explicit-revocation',(await click('trust-key-revoke','evidence-key-revoke')).body.registry.keys[id].status==='revoked');
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,390]){
  await page.setViewport({width,height:1100});await page.goto(cfg.base_url+'/trust-actions.html?lang='+lang,{waitUntil:'networkidle0'});await ready();await page.$eval('html',(e,v)=>e.dataset.theme=v,theme);await page.click('#admin');await page.waitForFunction(()=>document.querySelector('#settings').open);
  check(lang+'-'+theme+'-'+width+'-translated-Admin',await page.$eval('#trust-audit-title',e=>e.textContent)===(lang==='fr'?'Audit d’une transaction':'Transaction audit'));
  check(lang+'-'+theme+'-'+width+'-identity-source-limit',await page.$eval('#trust-identity-availability',e=>e.textContent).then(v=>v.includes(lang==='fr'?'raccord HTTP non actif':'HTTP integration inactive')));
  check(lang+'-'+theme+'-'+width+'-no-overflow',await page.$eval('#settings',e=>e.scrollWidth<=e.clientWidth));
  await page.$eval('#trust-role-settings',e=>e.scrollIntoView());await page.screenshot({path:path.join(cfg.output,lang+'-'+theme+'-'+width+'.png')});
  await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#settings').open&&document.activeElement.id==='admin');
 }
 check('no-unexpected-browser-diagnostics',errors.length===0);fs.writeFileSync(path.join(cfg.output,'result.json'),JSON.stringify({assertions:checks,errors,intercepted_business_responses:false,browser:await browser.version()},null,2));await context.close();
}finally{if(browser)await browser.close();}})().catch(e=>{console.error(e.stack);process.exitCode=1;});
