'use strict';
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-budget-ui-'));fs.mkdirSync(out,{recursive:true});let app,browser;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',c=>c?reject(Error(err||text)):resolve(text?JSON.parse(text):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);const w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Budget test',objective:'Verify budget changes',scope:'isolated',criteria:['No lost updates'],next:'Configure'})).work;
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,args:['--no-sandbox']});const p=await browser.newPage();await p.setViewport({width:1400,height:1000});const errors=[];p.on('pageerror',e=>errors.push(String(e)));
 for(const lang of ['fr','en']){
  const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang',lang);await p.goto(u.href);await p.waitForFunction(()=>typeof snapshot!=='undefined' && snapshot?.work);
  if(await p.$eval('[data-view="budget"]',e=>e.getClientRects().length===0))await p.click('#mode');await p.click('[data-view="budget"]');await p.click('#pricing-open');await p.waitForSelector('#modal[open]');
  await p.evaluate(()=>Pricing.edit());
  for(const [key,value]of Object.entries({provider:'fixture-'+lang,model:'example',currency:'USD',source:'Illustrative UI fixture',reference_date:'2026-09-23',input_per_million:'2',output_per_million:'5'}))await p.$eval('#field-'+key,(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},value);
  for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'pricing-'+lang+'-'+theme+'.png')})}
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open);
  let c=await cli(['pricing','list']);let rate=c.rates.at(-1);assert.equal(rate.provider,'fixture-'+lang);assert.equal(rate.cache_read_per_million,null);
  await p.click('#pricing-open');await p.waitForSelector('#modal[open]');await p.evaluate(()=>Pricing.estimate(Pricing.catalogue.rates.at(-1)));
  for(const k of ['non_cached_input_tokens','output_tokens','cache_read_tokens'])await p.$eval('#field-'+k,e=>{e.value='1000000'});
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#preview').hidden);assert.match(await p.$eval('#preview',e=>e.textContent),/inconnue|unknown/);
  await p.$eval('#field-cache_read_tokens',e=>{e.value='0';e.dispatchEvent(new Event('input',{bubbles:true}))});await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#preview').hidden);assert.match(await p.$eval('#preview',e=>e.textContent),/7 USD/);
  const q=await cli(['pricing','estimate'],{rate_version:rate.version,non_cached_input_tokens:1000000,output_tokens:1000000});assert.equal(q.estimated_amount,7);
  await p.keyboard.press('Escape');assert.equal(await p.evaluate(()=>document.activeElement.id),'pricing-open');
  // A stale price form must not overwrite a CLI catalogue change.
  await p.click('#pricing-open');await p.waitForSelector('#modal[open]');await p.evaluate(()=>Pricing.edit(Pricing.catalogue.rates.at(-1)));
  await cli(['pricing','save'],{schema_version:1,expected_digest:c.digest,rate:{...rate,input_per_million:3}});
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal-error').hidden);
  const old=await cli(['pricing','estimate'],{rate_version:rate.version,non_cached_input_tokens:1000000,output_tokens:1000000});assert.equal(old.estimated_amount,7,'old version changed');
  await p.keyboard.press('Escape');
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['price save','unknown cache cost stays unknown','CLI/web estimate parity','stale edit rejected','old price preserved','Escape focus','FR/EN two themes'],root},null,2));console.log('PASS pricing UI');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(app)app.kill()});
