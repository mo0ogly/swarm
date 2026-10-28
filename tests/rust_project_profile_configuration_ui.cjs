'use strict';
/** Actual served profile form, trusted keyboard/clicks, FR/EN and both themes.
 * No direct browser writes replace the product flow. Readonly fetches observe
 * state; one owned forwarding interceptor retains a real write behind a malformed
 * acknowledgement. No global watchdog; Python owns the inert server and Root.
 */
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),http=require('http'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[],failed=[],posts=[],consoleErrors=[];
function check(name,value){assert(value,name);checks.push(name);fs.writeFileSync(path.join(cfg.output,'checks.json'),JSON.stringify(checks,null,2)+'\n');console.log('PASS '+name);}

/** Observe the public current digest only, without changing editor/CAS state. */
async function current(page){return page.evaluate(async()=>{const response=await fetch('/api/v1/project-profile/catalog');if(!response.ok)throw Error('Catalogue observation failed');const value=await response.json();return value.active_sha256;});}
/** Select the first actual available suggestion using trusted native keys. */
async function choose(page){await page.focus('#project-profile-choice');await page.keyboard.press('Home');await page.keyboard.press('ArrowDown');await page.keyboard.press('Enter');await page.waitForFunction(()=>!document.querySelector('#project-profile-preview-button').disabled||document.querySelector('#project-profile-config-status').getAttribute('role')==='alert');assert.equal(await page.$eval('#project-profile-choice',e=>e.value),'claude-project');}
/** Finish on success OR a visible refusal, then assert the actual preview state. */
async function preview(page){const waiting=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/project-profile/preview');await page.click('#project-profile-preview-button');const response=await waiting;await page.waitForFunction(()=>!document.querySelector('#project-profile-preview-button').disabled);return response;}
/** A rendered semantic text/background pair, composited through its ancestors.
 * Canvas handles computed CSS color syntax; WCAG luminance uses actual sRGB.
 * This checks these pairs, not all controls, gradients or complete accessibility.
 */
async function contrast(page,selector){return page.$eval(selector,element=>{
 const canvas=document.createElement('canvas');canvas.width=canvas.height=1;const context=canvas.getContext('2d');context.fillStyle='#fff';context.fillRect(0,0,1,1);const chain=[];for(let node=element;node;node=node.parentElement)chain.unshift(node);for(const node of chain){context.fillStyle=getComputedStyle(node).backgroundColor;context.fillRect(0,0,1,1);}const background=[...context.getImageData(0,0,1,1).data].slice(0,3);context.fillStyle=getComputedStyle(element).color;context.fillRect(0,0,1,1);const foreground=[...context.getImageData(0,0,1,1).data].slice(0,3);const luminance=rgb=>rgb.map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;}).reduce((sum,v,index)=>sum+v*[.2126,.7152,.0722][index],0);const a=luminance(background),b=luminance(foreground);return(Math.max(a,b)+.05)/(Math.min(a,b)+.05);
 });}

(async()=>{
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
 try{
  for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
   const context=await browser.createBrowserContext(),page=await context.newPage(),label=lang+'-'+theme;page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width:1440,height:1000});page.on('pageerror',error=>errors.push(error.message));page.on('requestfailed',request=>failed.push(new URL(request.url()).pathname));page.on('console',message=>{if(message.type()==='error')consoleErrors.push({text:message.text(),url:message.location().url});});page.on('request',request=>{if(request.method()==='POST')posts.push(new URL(request.url()).pathname);});
   const split=cfg.cookie.indexOf('=');await context.setCookie({name:cfg.cookie.slice(0,split),value:cfg.cookie.slice(split+1),url:cfg.base,httpOnly:true,sameSite:'Strict'});
   let badPreview=label==='fr-etat',badApply=label==='fr-etat';
   if(badPreview){await page.setRequestInterception(true);page.on('request',async request=>{
    const target=new URL(request.url()).pathname;
    if(request.method()==='POST'&&target==='/api/v1/project-profile/preview'&&badPreview){badPreview=false;await request.respond({status:200,contentType:'application/json',body:'{}'});return;}
    if(request.method()==='POST'&&target==='/api/v1/project-profile/apply'&&badApply){badApply=false;try{
     const body=request.postData();const status=await new Promise((resolve,reject)=>{const call=http.request(request.url(),{method:'POST',headers:{...request.headers(),'content-length':Buffer.byteLength(body)}},response=>{response.resume();response.on('end',()=>resolve(response.statusCode));});call.on('error',reject);call.end(body);});assert.equal(status,200);await request.respond({status:200,contentType:'application/json',body:'{}'});return;
    }catch(error){errors.push('owned-apply-forward-failed');await request.abort();return;}}
    await request.continue();
   });}
   await page.goto(cfg.base+'/prepare.html?'+new URLSearchParams({lang,theme}),{waitUntil:'networkidle0'});
   await page.click('[data-fold-key="project-profile"] > summary');await page.waitForFunction(()=>!document.querySelector('#project-profile-open').disabled);await page.click('#project-profile-open');await page.waitForFunction(()=>!document.querySelector('#project-profile-choice').disabled||document.querySelector('#project-profile-config-status').getAttribute('role')==='alert');
   check(label+'-dialog-title-and-close',await page.$eval('#project-profile-dialog',dialog=>dialog.open&&dialog.getAttribute('aria-labelledby')==='project-profile-dialog-title'&&!!dialog.querySelector('#project-profile-close')));
   check(label+'-unavailable-choice-visible',await page.$eval('#project-profile-choice option[value="gemini-root"]',option=>option.disabled)&&await page.$eval('#project-profile-unavailable',node=>!node.hidden&&node.textContent.includes('GEMINI.md')));
   await choose(page);const name=label+' profile <b>literal</b>';await page.click('#project-profile-name');await page.keyboard.down('Control');await page.keyboard.press('KeyA');await page.keyboard.up('Control');await page.type('#project-profile-name',name);
   check(label+'-explicit-paths-and-roles',await page.$$eval('#project-profile-sources .profile-source',rows=>rows.length===2&&rows[0].querySelectorAll('input:checked').length===5&&rows[1].querySelectorAll('input:checked').length===1));
   const before=await current(page);
   if(label==='fr-etat'){
    const response=await preview(page);check('profile-malformed-preview-cannot-enable-apply',response.status()===200&&await page.$eval('#project-profile-apply',button=>button.disabled)&&await page.$eval('#project-profile-name',(input,expected)=>input.value===expected,name));check('profile-malformed-preview-keeps-current-file',await current(page)===before);
   }
   let response=await preview(page);check(label+'-preview-real-metadata',response.status()===200&&!await page.$eval('#project-profile-preview',node=>node.hidden)&&!await page.$eval('#project-profile-apply',button=>button.disabled));check(label+'-preview-does-not-write',await current(page)===before);
   if(label==='fr-etat'){
    const writing=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/project-profile/apply');await page.click('#project-profile-apply');await writing;await page.waitForFunction(()=>!document.querySelector('#project-profile-preview-button').disabled);check('profile-malformed-apply-preserves-draft',await page.$eval('#project-profile-name',(input,expected)=>input.value===expected,name)&&await page.$eval('#project-profile-apply',button=>button.disabled)&&await page.$eval('#project-profile-config-status',node=>node.getAttribute('role')==='alert'));check('profile-malformed-apply-may-have-written',await current(page)!==before);
    response=await preview(page);check('profile-original-cas-still-refuses',response.status()===409);
    const fresh=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/project-profile/catalog');await page.click('#project-profile-catalog-refresh');await fresh;await page.waitForFunction(()=>!document.querySelector('#project-profile-catalog-refresh').disabled);response=await preview(page);check('profile-refresh-does-not-rebase-draft',response.status()===409&&await page.$eval('#project-profile-name',(input,expected)=>input.value===expected,name));await choose(page);response=await preview(page);check('profile-explicit-reselection-recovers',response.status()===200&&!await page.$eval('#project-profile-apply',button=>button.disabled));
   }
   check(label+'-label-and-field-contrast',await contrast(page,'label[for="project-profile-name"]')>=4.5&&await contrast(page,'#project-profile-name')>=4.5);
   // From Preview, four native reverse tabs traverse add + two collapsed source
   // summaries to the editable name. This also checks visible keyboard focus.
   await page.focus('#project-profile-preview-button');for(let index=0;index<4;index++){await page.keyboard.down('Shift');await page.keyboard.press('Tab');await page.keyboard.up('Shift');}
   check(label+'-keyboard-field-focus',await page.evaluate(()=>{const element=document.activeElement,style=getComputedStyle(element);return element.id==='project-profile-name'&&(style.outlineStyle!=='none'||style.boxShadow!=='none');}));
   for(const width of [1440,390]){await page.setViewport({width,height:1000});check(label+'-no-overflow-'+width,await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth&&document.querySelector('#project-profile-dialog').getBoundingClientRect().width<=innerWidth));await page.screenshot({path:path.join(cfg.output,label+'-'+width+'.png'),fullPage:true});}
   const writing=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/project-profile/apply');await page.click('#project-profile-apply');const result=await writing,value=await result.json();await page.waitForFunction(()=>!document.querySelector('#project-profile-catalog-refresh').disabled);check(label+'-explicit-save-real-context',result.status()===200&&value.enabled===true&&value.roles.length===5&&value.roles.every(role=>role.sources.length));check(label+'-saved-label-localized',await page.$eval('#project-profile-config-status',(node,lang)=>node.textContent===(lang==='en'?'Project profile saved. No agent started.':'Profil du projet enregistré. Aucun agent lancé.'),lang));check(label+'-metadata-does-not-render-html-or-settings',await page.$eval('#project-profile-status',node=>!node.querySelector('b'))&&!await page.evaluate(()=>document.body.textContent.includes('NEVER_IMPORT_NATIVE_PERMISSION')));
   await page.keyboard.press('Escape');check(label+'-escape-restores-focus',await page.evaluate(()=>!document.querySelector('#project-profile-dialog').open&&document.activeElement.id==='project-profile-open'));
   await context.close();
  }
  check('profile-no-unhandled-javascript',errors.length===0);check('profile-no-failed-network-request',failed.length===0);check('profile-only-explicit-configuration-posts',posts.length>0&&posts.every(target=>['/api/v1/project-profile/preview','/api/v1/project-profile/apply'].includes(target)));
  fs.writeFileSync(path.join(cfg.output,'console-errors.json'),JSON.stringify(consoleErrors,null,2)+'\n');fs.writeFileSync(path.join(cfg.output,'requests.json'),JSON.stringify(posts,null,2)+'\n');console.log('PROJECT_PROFILE_CONFIGURATION_BROWSER_PASS '+checks.length);
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
