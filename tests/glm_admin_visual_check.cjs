'use strict';
// Bounded visual inspection of the Admin engine-settings modal on the REAL
// installed local service (v0.1.1-dev.250). Real login (POST /login with the
// owned session credential), real Admin modal, assertions on the v49 field
// rendering, and a PNG evidence capture. Fixture transport only: CDP drives
// the local browser; no acceptance authority, no global verification.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os');
const {spawn}=require('node:child_process'),assert=require('node:assert/strict');
const puppeteer=require(path.join(process.cwd(),'node_modules','puppeteer'));
const ADDR='127.0.0.1:38433', ROOT=process.cwd();
const OUT=process.argv[2]||path.join(ROOT,'docs/plans/actions-confiance/paea/evidence/20261029-fixture-regen-mvfdts57/admin-visual-86');
if(fs.existsSync(OUT))throw Error('Fresh evidence directory required');
fs.mkdirSync(OUT,{recursive:true});
const TOKEN=fs.readFileSync(path.join(ROOT,'.swarm','web-session-e6ed55c9891a1038'),'utf8').trim();
(async()=>{
 const browser=await puppeteer.launch({headless:'new',executablePath:'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
 try{
  const page=await browser.newPage();
  await page.setViewport({width:1280,height:900});
  // 1. Real login with the owned token
  const resp=await page.goto(`http://${ADDR}/`,{waitUntil:'networkidle2'});
  assert.equal(resp.status(),200,'login page reachable');
  await page.type('input[name="token"]',TOKEN);
  await Promise.all([page.waitForNavigation({waitUntil:'networkidle2'}),page.click('button[type="submit"]')]);
  const after=page.url();
  assert.ok(!after.includes('login'),'login redirected, url='+after);
  // 2. Real path: cockpit Administration view → "Ouvrir les réglages Rust" →
  //    /native-runtime.html?view=admin → engine-settings card dialog.
  await page.evaluate(()=>{document.querySelector('button[data-view="admin"]')
    .dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true}))});
  await page.waitForFunction(()=>document.getElementById('admin')?.hidden===false,{timeout:10000});
  const rustLink=await page.evaluate(()=>{
    const a=[...document.querySelectorAll('a')].find(a=>/native-runtime\.html/.test(a.getAttribute('href')||'')&&/Ouvrir les réglages/.test(a.textContent||''));
    if(!a)return null;return a.getAttribute('href');
  });
  assert.equal(rustLink,'/native-runtime.html?view=admin&lang=fr&theme=sombre','rust settings link href, got: '+rustLink);
  await Promise.all([page.waitForNavigation({waitUntil:'networkidle2'}),page.evaluate(href=>{location.href=href},rustLink)]);
  await page.waitForSelector('#native-settings-admin .engine-settings-admin',{timeout:15000});
  const opened=await page.evaluate(()=>{
    const edit=[...document.querySelectorAll('.engine-settings-admin button')]
      .find(b=>(b.textContent||'').trim()==='Régler le moteur');
    if(!edit)return null;edit.click();return true;
  });
  console.log('settings trigger clicked:',opened);
  assert.equal(opened,true,'"Régler le moteur" button must exist and be clickable');
  await page.waitForSelector('dialog[data-engine-settings][open]',{timeout:10000});
  // 3. Assert the v49 field is rendered in the Stockage et Git group
  const found=await page.evaluate(()=>{
    const field=document.querySelector('dialog[data-engine-settings] [data-setting="paea_settlement_ledger_root"]');
    if(!field)return {found:false};
    const group=field.closest('details.engine-settings-group');
    const label=[...document.querySelectorAll('label')].find(l=>l.htmlFor===field.id||l.contains(field));
    return {found:true,group:group?group.querySelector('summary').textContent.trim():null,
            labelText:label?label.textContent.trim():null,
            visible:!!(field.offsetWidth||field.offsetHeight)};
  });
  console.log('v49 field render:',JSON.stringify(found));
  assert.equal(found.found,true,'v49 field must render in the Admin modal');
  if(found.group)assert.ok(found.group.toLowerCase().includes('stockage'),'group is Stockage et Git, got: '+found.group);
  // 4. Open the group so the field is visible in the capture; help disclosure check.
  await page.evaluate(()=>{const g=document.querySelector('dialog[data-engine-settings] details.engine-settings-group');if(g)g.open=true;});
  await new Promise(r=>setTimeout(r,300));
  const helpBtn=await page.$('dialog[data-engine-settings] [data-settings-help="paea_settlement_ledger_root"]');
  console.log('help trigger present:',!!helpBtn);
  await page.screenshot({path:path.join(OUT,'admin-v49-field.png'),fullPage:false});
  fs.writeFileSync(path.join(OUT,'result.json'),JSON.stringify({
    schema_version:1,kind:'admin-visual-check-v1',service:ADDR,
    checked_at:new Date().toISOString(),
    field_rendered:found.found,group:found.group,label:found.labelText,
    help_trigger:!!helpBtn,
    limitations:'pixel rendering inspected via this capture only; no acceptance authority'
  },null,2)+'\n');
  console.log('EVIDENCE OK');
 }finally{await browser.close();}
})().catch(e=>{console.error('FAIL:',e.message);process.exit(1);});
