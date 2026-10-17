'use strict';
// Browser verification of the isolated, generated component: no Swarm server/provider.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),http=require('node:http');
const assert=require('node:assert/strict'),crypto=require('node:crypto');
const {execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const repo=path.resolve(__dirname,'..');let S;
const out=path.resolve(process.argv[2]||fs.mkdtempSync(path.join(os.tmpdir(),'swarm-signals-evidence-')));
if(process.argv[2])fs.mkdirSync(out,{recursive:false});
const temporary=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-signals-build-')),build=path.join(temporary,'catalogue');
const checks=[],errors=[],failed=[],requests=[];let server,browser;
const hash=file=>crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const inputPaths=['frontend/build-signals.mjs','frontend/signals/signals.js','frontend/signals/signals.css','frontend/signals/catalogue.html','frontend/signals/catalogue.js','frontend/signals/catalogue.css','web/wattson_themes.css','web/swarm-design.css','web/swarm-logo.png','tests/semantic_signals_ui.cjs'];
const inputs=Object.fromEntries(inputPaths.map(p=>[p,hash(path.join(repo,p))]));
const save=(name,value)=>fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2)+'\n');
(async()=>{
 try{
  const result=execFileSync(process.execPath,['frontend/build-signals.mjs',build],{cwd:repo,encoding:'utf8'});save('build.json',JSON.parse(result));
  S=require(path.join(build,'signals.js'));
  assert.equal(Object.keys(S.definitions).length,29);
  for(const item of Object.values(S.definitions)){assert(item.label.fr&&item.label.en&&item.description.fr&&item.description.en);assert(Object.isFrozen(item));}
  assert.equal(JSON.parse(result).authority,false);
  const manifest=JSON.parse(fs.readFileSync(path.join(build,'build-manifest.json'),'utf8'));
  for(const [name,digest]of Object.entries(manifest.assets))assert.equal(hash(path.join(build,name)),digest);
  assert.throws(()=>execFileSync(process.execPath,['frontend/build-signals.mjs',build],{cwd:repo,stdio:'pipe'}));
  assert.throws(()=>execFileSync(process.execPath,['frontend/build-signals.mjs',path.join(repo,'web','signals-proof')],{cwd:repo,stdio:'pipe'}));
  fs.symlinkSync(path.join(repo,'web'),path.join(temporary,'web-alias'),'dir');
  assert.throws(()=>execFileSync(process.execPath,['frontend/build-signals.mjs',path.join(temporary,'web-alias','signals-proof')],{cwd:repo,stdio:'pipe'}));
  assert(!fs.existsSync(path.join(repo,'web','signals-proof')));checks.push('Explicit fresh build preserves inputs and refuses replacement or embedded-web output');
  const allowed=new Set(Object.keys(manifest.assets));allowed.add('build-manifest.json');
  server=http.createServer((req,res)=>{const name=new URL(req.url,'http://localhost').pathname.slice(1)||'catalogue.html';if(!allowed.has(name)){res.writeHead(404);res.end();return;}const ext=path.extname(name);res.setHeader('Content-Type',({'.html':'text/html','.css':'text/css','.js':'application/javascript','.png':'image/png','.json':'application/json'})[ext]);res.end(fs.readFileSync(path.join(build,name)));});
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const origin='http://127.0.0.1:'+server.address().port;
  browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage();
  page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});page.on('requestfailed',r=>failed.push(r.url()));page.on('request',r=>requests.push(r.url()));
  await page.setViewport({width:1440,height:1050});await page.goto(origin+'/catalogue.html',{waitUntil:'networkidle0'});
  assert.equal(await page.$$eval('#legend [data-signal]',nodes=>nodes.length),29);
  assert.equal(await page.$eval('#seal-preview [data-signal^="protection-"]',n=>n.dataset.signal),'protection-unknown');
  assert.equal(await page.$eval('#seal-preview [data-signal^="identity-"]',n=>n.dataset.signal),'identity-unknown');
  assert.equal(await page.$$eval('#seal-preview .swarm-signal--success',nodes=>nodes.length),0);
  assert.equal(await page.$eval('.swarm-context-seal__wax',n=>n.getAttribute('aria-hidden')),'true');
  checks.push('All bilingual examples render; the default wax seal shows unknown identity/rights and no positive status');
  const safety=await page.evaluate(()=>{
   const host=document.createElement('div');document.body.append(host);const malicious='<img src=x onerror=alert(1)>';
   host.append(SwarmSignals.seal({owner:malicious,author:malicious,reference:malicious}));
   const result={images:host.querySelectorAll('img').length,text:host.textContent.includes(malicious),positive:host.querySelectorAll('.swarm-signal--success').length};
   const only=SwarmSignals.badge('actor-agent',{iconOnly:true});result.aria=only.getAttribute('aria-label');result.role=only.getAttribute('role');
   result.unknown=SwarmSignals.memory('future-type').dataset.signal;
   result.refused=0;for(const fn of [()=>SwarmSignals.badge('invented'),()=>SwarmSignals.state('protection',true),()=>SwarmSignals.state('identity','admin'),()=>SwarmSignals.badge('actor-agent',{lang:'xx'}),...['constructor','__proto__','toString'].map(id=>()=>SwarmSignals.badge(id)),...['constructor','__proto__','toString'].map(lang=>()=>SwarmSignals.badge('actor-agent',{lang}))])try{fn()}catch(e){if(e instanceof RangeError)result.refused++;}
   host.remove();return result;
  });assert.deepEqual(safety,{images:0,text:true,positive:0,aria:'Agent',role:'img',unknown:'memory-unknown',refused:10});
  checks.push('Names and references remain text; IDs/roles/booleans cannot imply verification; icon-only output is named');
  await page.select('#memory','procedure');await page.select('#protection','denied');await page.select('#identity','suspended');
  assert.equal(await page.$eval('#seal-preview [data-signal^="memory-"]',n=>n.dataset.signal),'memory-procedure');
  assert.equal(await page.$eval('#seal-preview [data-signal^="protection-"]',n=>n.dataset.signal),'protection-denied');
  assert.equal(await page.$eval('#seal-preview [data-signal^="identity-"]',n=>n.dataset.signal),'identity-suspended');
  for(const lang of ['fr','en']){
   await page.select('#language',lang);assert.equal(await page.$eval('html',n=>n.lang),lang);
   assert.equal(await page.$eval('#protection',n=>n.value),'denied');assert.equal(await page.$eval('#identity',n=>n.value),'suspended');
   for(const theme of ['etat','sombre']){
    if(await page.$eval('html',n=>n.dataset.theme)!==theme)await page.click('#theme');
    for(const width of [1440,1024,760,390,320]){await page.setViewport({width,height:1050});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),`${lang}/${theme}/${width}: overflow`);}
    const contrast=await page.evaluate(()=>{
     const rgb=x=>x.match(/[\d.]+/g).slice(0,3).map(Number),lum=x=>x.map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4}).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);
     return Array.from(document.querySelectorAll('.swarm-signal')).map(n=>{const s=getComputedStyle(n),a=lum(rgb(s.color)),b=lum(rgb(s.backgroundColor));return{id:n.dataset.signal,ratio:(Math.max(a,b)+.05)/(Math.min(a,b)+.05)}});
    });assert(contrast.every(c=>c.ratio>=4.5),JSON.stringify({lang,theme,contrast}));save(`contrast-${lang}-${theme}.json`,contrast);
    await page.screenshot({path:path.join(out,`${lang}-${theme}-mobile.png`)});
    await page.setViewport({width:1440,height:1050});await page.screenshot({path:path.join(out,`${lang}-${theme}-desktop.png`)});
   }
  }
  checks.push('FR/EN selections persist; both themes render at 1440/1024/760/390/320px without overflow; every badge reaches 4.5:1');
  await page.select('#language','fr');if(await page.$eval('html',n=>n.dataset.theme)!=='etat')await page.click('#theme');
  await page.select('#memory','experience');await page.click('#reset');assert.equal(await page.$eval('#reset',n=>document.activeElement===n),true);
  assert.equal(await page.$$eval('#seal-preview .swarm-signal--success',n=>n.length),0);
  await page.keyboard.press('Tab');await page.focus('#theme');await page.keyboard.press('Tab');
  assert(await page.evaluate(()=>{const el=document.activeElement,s=getComputedStyle(el);return el.matches(':focus-visible')&&s.outlineStyle!=='none'}));
  await page.focus('summary');await page.keyboard.press('Enter');assert(await page.$eval('details',n=>n.open));await page.keyboard.press('Enter');assert.equal(await page.$eval('details',n=>n.open),false);
  const geometry=await page.$$eval('button,select',nodes=>nodes.map(n=>({id:n.id,width:n.getBoundingClientRect().width,height:n.getBoundingClientRect().height})));assert(geometry.every(n=>n.height>=44));save('controls.json',geometry);
  for(const state of ['unknown','pending','allowed','denied','revoked']){await page.select('#protection',state);assert.equal(await page.$eval('#seal-preview [data-signal^="protection-"]',n=>n.dataset.signal),'protection-'+state);}
  for(const state of ['unknown','pending','verified','suspended','revoked']){await page.select('#identity',state);assert.equal(await page.$eval('#seal-preview [data-signal^="identity-"]',n=>n.dataset.signal),'identity-'+state);}
  await page.click('#reset');await page.screenshot({path:path.join(out,'seal-default.png')});
  await(await page.$('.catalogue-parchment')).screenshot({path:path.join(out,'seal-paper.png')});
  for(const family of ['protection','identity','version'])await(await page.$(`[data-family="${family}"]`)).screenshot({path:path.join(out,`legend-${family}.png`)});
  checks.push('Every protection/identity state updates the seal; reset preserves focus and restores unknown; keyboard disclosure and visible focus work');
  assert.equal(errors.length,0,errors.join('\n'));assert.equal(failed.length,0,failed.join('\n'));assert(requests.every(url=>url.startsWith(origin+'/')));
  assert(inputPaths.every(p=>hash(path.join(repo,p))===inputs[p]));
  save('result.json',{status:'PASS',checks,definitions:29,viewports:20,inputs,input_unchanged:true,requests:requests.map(u=>new URL(u).pathname),console_errors:errors,failed_requests:failed,application_api_calls:0,model_calls:0,build_directory:build});
  console.log(JSON.stringify({status:'PASS',checks:checks.length,definitions:29,viewports:20,output:out}));
 }catch(error){save('failure.json',{error:String(error.stack),checks,errors,failed,inputs});throw error;}
 finally{if(browser)await browser.close();if(server)await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error.message);process.exitCode=1});
