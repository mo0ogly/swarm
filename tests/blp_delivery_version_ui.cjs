'use strict';
/** Check Version and news on the provided owned local endpoint, without source
 * overlay or project mutation. Private config supplies a session and expected
 * metadata/notes; reports contain no cookie. The caller must separately establish
 * managed installation/process provenance and human Authority session scope.
 * Explicit clean/modified boolean facts require matching notice absence/presence;
 * invalid provenance is refused by the shared assertion helper.
 * Reuses the runtime collector and explicit unbounded verification transport. */
const {versionProvenanceMatches}=require('./helpers/version_provenance_assertions.cjs');
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer'),runtime=require('./helpers/browser_verification_runtime.cjs');
assert.equal(process.argv.length,3);const cfg=JSON.parse(fs.readFileSync(process.argv[2])),base=new URL(cfg.url),out=path.resolve(cfg.output);assert.equal(base.hostname,'127.0.0.1');fs.mkdirSync(out,{mode:0o700});const checks=[],errors=[];
/** Retain a named behavioral assertion; raw session headers never enter it. */
function check(name,value){assert(value,name);checks.push(name);console.log('PASS '+name);}
(async()=>{
 const record=runtime.begin(path.resolve(__dirname,'..'),out);let browser,failure;
 try{
  browser=await puppeteer.launch({headless:true,executablePath:'/opt/google/chrome/chrome',...runtime.launchOptions,args:['--no-sandbox'],userDataDir:path.join(out,'profile')});await runtime.recordBrowser(record,browser);
  const context=await browser.createBrowserContext(),separator=cfg.cookie.indexOf('=');await context.setCookie({name:cfg.cookie.slice(0,separator),value:cfg.cookie.slice(separator+1),domain:'127.0.0.1',path:'/',httpOnly:true,sameSite:'Strict'});
  for(const lang of ['fr','en']){
   const page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);page.on('pageerror',()=>errors.push('pageerror-'+lang));page.on('requestfailed',()=>errors.push('requestfailed-'+lang));page.on('console',m=>{if(m.type()==='error')errors.push('console-error-'+lang)});
   await page.setViewport({width:1280,height:900});const navigation=await page.goto(new URL('/?view=conduite&lang='+lang+'&theme=sombre',base).href,{waitUntil:'load'});assert.equal(navigation.status(),200);assert.equal(new URL(page.url()).pathname,'/','provided local session must reach the cockpit');
   await page.waitForFunction(()=>document.querySelector('#work')&&!document.querySelector('#main-content').hasAttribute('aria-busy'));await page.evaluate(async()=>{await VersionHistoryPanel.pending});
   if(await page.$eval('#nav-toggle',e=>e.getClientRects().length>0&&e.getAttribute('aria-expanded')!=='true'))await page.click('#nav-toggle');if(!await page.$eval('.rail-settings',e=>e.open))await page.click('.rail-settings summary');
   await page.click('#version-open');await page.waitForFunction(()=>document.getElementById('version-dialog')?.open&&document.querySelector('.version-local-changes li'));
   const observed=await page.evaluate(()=>({binary:VersionHistoryPanel.value.binary,notes:VersionHistoryPanel.value.history.local_changes,text:document.getElementById('version-dialog-content').textContent,hrefs:[...document.querySelectorAll('#version-dialog a')].map(a=>a.href),rendered:[...document.querySelectorAll('.version-local-changes li')].map(e=>e.textContent),dirtyNoticeCount:document.querySelectorAll('.version-local-changes > p.notice.attention').length}));
   check(lang+'-running-metadata',observed.binary.version===cfg.binary.version&&observed.binary.commit===cfg.binary.commit&&observed.binary.modified===cfg.binary.modified);
   assert.deepEqual(observed.notes,cfg.notes);check(lang+'-embedded-notes-and-blp-note',observed.rendered.includes(cfg.blp_note[lang])&&observed.text.includes(cfg.binary.version));
   check(lang+'-git-link-and-dirty-disclosure',versionProvenanceMatches(cfg,observed));
   await page.screenshot({path:path.join(out,'version-'+lang+'.png'),fullPage:true});await page.keyboard.press('Escape');check(lang+'-escape-and-focus',await page.evaluate(()=>!document.getElementById('version-dialog').open&&document.activeElement.id==='version-open'));await page.close();
  }
  check('no-browser-errors',errors.length===0);await context.close();
 }catch(error){failure=error;}
 finally{if(browser)try{await browser.close()}catch(error){failure=failure?new AggregateError([failure,error],'verification and browser cleanup failed'):error;}}
 if(!failure)runtime.finish(record,path.resolve(__dirname,'..'));fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:failure?'FAIL':'PASS',checks,errors,scope:'versiondialogue-owned-endpoint',limits:'No managed installation/process identity or human Authority session is established by this script; caller provenance is separate'},null,2)+'\n');if(failure)throw failure;
})().catch(error=>{console.error(error);process.exitCode=1});
