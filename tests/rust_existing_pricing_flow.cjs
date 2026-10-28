'use strict';
/** Exercise the existing pricing renderer on served Rust routes in an owned Root.
 * Shared with the canonical cockpit recipe: all writes are explicit fixture
 * append operations. Named checks assert amounts, history, refusal and recovery;
 * response-loss injection follows a valid witnessed real save. No watchdog. */
module.exports=async function pricingFlow({page,cfg,label,lang,theme,check}){
 const dialog='#pricing-modal',field=name=>'#pricing-field-'+name,model='ui-'+label;
 /** Fill explicit user intent, leaving unknown rates blank. No source lookup. */
 async function fill(){const fields={provider:'fixture',model,currency:'USD',source:'Inert <b>literal</b> pricing fixture',reference_date:'2026-10-24',input_per_million:'2',output_per_million:'3',cache_read_per_million:'',cache_write_per_million:'0'};for(const [key,value]of Object.entries(fields))await page.$eval(field(key),(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},value)}
 /** Explicit list action scoped to one matching immutable model card. */
 async function cardAction(index){await page.$$eval('#pricing-modal-fields section',(cards,{model,index})=>{const card=cards.find(c=>c.querySelector('h3').textContent.includes(model));if(!card)throw Error('pricing-test-card-missing');card.querySelectorAll('button')[index].click()},{model,index})}
 await page.click('#pricing-open');await page.waitForFunction(()=>document.querySelector('#pricing-modal').open&&document.querySelector('#pricing-modal-fields section')&&!document.querySelector('#pricing-open').disabled);
 check(label+'-pricing-existing-list',await page.$eval('#pricing-modal-fields',e=>e.textContent.includes('http-witness')));
 await page.$eval('#pricing-modal-fields > button',e=>e.click());await fill();
 check(label+'-pricing-blank-unknown-not-zero',await page.$eval(field('cache_read_per_million'),e=>e.value===''));
 for(const width of [1440,390]){await page.setViewport({width,height:1000});check(label+'-pricing-dialog-no-overflow-'+width,await page.evaluate(()=>{const d=document.querySelector('#pricing-modal');return d.scrollWidth<=d.clientWidth&&document.documentElement.scrollWidth<=innerWidth}));}
 await page.click('#pricing-confirm');await page.waitForFunction(()=>!document.querySelector('#pricing-modal').open&&!document.querySelector('#pricing-open').disabled);
 check(label+'-pricing-save-reread-message',await page.$eval('#message',e=>e.textContent.includes(document.documentElement.lang==='en'?'Price saved':'Tarif enregistré')));
 await page.click('#pricing-open');await page.waitForFunction(()=>document.querySelector('#pricing-modal').open&&!document.querySelector('#pricing-open').disabled);
 check(label+'-pricing-saved-null-and-free',await page.$$eval('#pricing-modal-fields section',(cards,model)=>{const text=cards.find(c=>c.querySelector('h3').textContent.includes(model)).textContent;return text.includes('0 USD')&&text.includes(document.documentElement.lang==='en'?'Unknown':'Inconnu')&&text.includes('<b>literal</b>')},model));
 check(label+'-pricing-source-no-html',!(await page.$('#pricing-modal-fields b')));
 await cardAction(1);
 for(const [key,value]of Object.entries({non_cached_input_tokens:'1000000',output_tokens:'2000000',cache_read_tokens:'0',cache_write_tokens:'0'}))await page.$eval(field(key),(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},value);
 await page.click('#pricing-confirm');await page.waitForFunction(()=>!document.querySelector('#pricing-preview').hidden&&!document.querySelector('#pricing-confirm').disabled);
 check(label+'-pricing-actual-eight-USD-estimate',await page.$eval('#pricing-preview',e=>e.textContent.includes('8 USD')));
 await page.$eval(field('cache_read_tokens'),e=>{e.value='1';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.click('#pricing-confirm');await page.waitForFunction(()=>!document.querySelector('#pricing-preview').hidden&&!document.querySelector('#pricing-confirm').disabled);
 check(label+'-pricing-unknown-rate-no-zero-estimate',await page.$eval('#pricing-preview',e=>e.textContent.includes(document.documentElement.lang==='en'?'Estimate unknown':'Estimation inconnue')&&!e.textContent.includes('8 USD')));
 await page.screenshot({path:require('node:path').join(cfg.output,label+'-pricing-390.png'),fullPage:true});await page.keyboard.press('Escape');check(label+'-pricing-escape-focus',await page.$eval('#pricing-open',e=>e===document.activeElement));
 // Admin entry opens the same renderer and restores its own focus.
 await page.$eval('[data-view="admin"]',e=>{e.closest('details').open=true;e.click()});await page.click('#admin-pricing-open');await page.waitForFunction(()=>document.querySelector('#pricing-modal').open&&!document.querySelector('#pricing-open').disabled);await page.keyboard.press('Escape');check(label+'-pricing-admin-opener-focus',await page.$eval('#admin-pricing-open',e=>e===document.activeElement));
 await page.$eval('[data-view="budget"]',e=>{e.closest('details').open=true;e.click()});
 if(lang==='fr'&&theme==='etat'){
  // Witnessed real list/save above precedes malformed-success client injection.
  await page.evaluate(()=>{window.pricingFetch=window.fetch;window.fetch=async(...args)=>new URL(args[0],location.href).pathname==='/api/v1/pricing'?new Response('{}',{status:200,headers:{'Content-Type':'application/json'}}):window.pricingFetch(...args)});
  await page.click('#pricing-open');await page.waitForFunction(()=>document.querySelector('#pricing-modal').open&&!document.querySelector('#pricing-modal-error').hidden&&!document.querySelector('#pricing-open').disabled);check('pricing-malformed-not-empty-success',await page.$eval('#pricing-modal-error',e=>e.textContent.includes('pricing-native-catalogue-invalid'))&&await page.$eval('#pricing-confirm',e=>e.hidden));await page.keyboard.press('Escape');
  await page.evaluate(()=>{window.fetch=window.pricingFetch;delete window.pricingFetch});await page.click('#pricing-open');await page.waitForFunction(()=>document.querySelector('#pricing-modal-fields section')&&!document.querySelector('#pricing-open').disabled);check('pricing-recovery-real-versions',await page.$eval('#pricing-modal-fields',e=>e.textContent.includes('http-witness')));
  await cardAction(0);await page.$eval(field('source'),e=>{e.value='Lost-response fixture <literal>';e.dispatchEvent(new Event('input',{bubbles:true}))});
  await page.evaluate(()=>{window.pricingFetch=window.fetch;window.fetch=async(...args)=>{const r=await window.pricingFetch(...args);if(new URL(args[0],location.href).pathname==='/api/v1/pricing'&&args[1]?.method==='POST'&&r.ok)throw Error('client-fixture-lost-after-save');return r}});
  await page.click('#pricing-confirm');await page.waitForFunction(()=>!document.querySelector('#pricing-modal-error').hidden&&!document.querySelector('#pricing-open').disabled);check('pricing-lost-save-no-retry',await page.$eval('#pricing-confirm',e=>e.disabled)&&await page.$eval('#pricing-modal-error',e=>e.textContent.includes('peut déjà être enregistrée')));await page.keyboard.press('Escape');await page.evaluate(()=>{window.fetch=window.pricingFetch;delete window.pricingFetch});
  await page.click('#pricing-open');await page.waitForFunction(()=>document.querySelector('#pricing-modal-fields section')&&!document.querySelector('#pricing-open').disabled);check('pricing-lost-save-reread-once-history-kept',await page.$$eval('#pricing-modal-fields section',(cards,model)=>cards.filter(c=>c.querySelector('h3').textContent.includes(model)).length===2&&cards.some(c=>c.textContent.includes('Lost-response fixture')),model));await page.keyboard.press('Escape');
 }
};
