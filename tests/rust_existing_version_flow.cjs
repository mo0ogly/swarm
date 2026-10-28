'use strict';
/** Real native Version/news journey, shared renderer and compiled producer.
 * Marked client-response fixtures follow an actual successful witness. No writes,
 * external navigation, watchdog, provider or unobserved CLI/source comparison. */
module.exports=async function versionFlow({page,cfg,label,lang,check}){
 const nativeRequests=[];const observe=r=>nativeRequests.push(new URL(r.url()).pathname);page.on('request',observe);
 await page.setViewport({width:1440,height:1000});await page.$eval('#version-open',e=>{e.closest('details').open=true});
 const actual=await page.evaluate(async()=>{const r=await fetch('/api/v1/native-version');if(!r.ok)throw Error('version-test-witness-failed');return r.json()});
 await page.click('#version-open');await page.waitForFunction(()=>document.querySelector('#version-dialog')?.open&&VersionHistoryPanel.value&&!VersionHistoryPanel.pending);
 const text=await page.$eval('#version-dialog-content',e=>e.textContent),binary=actual.binary;
 check(label+'-version-actual-compiled-number',text.includes(binary.version));check(label+'-version-actual-compiled-commit',text.includes(binary.commit.slice(0,12)));
 check(label+'-version-actual-commit-link',await page.$eval('#version-dialog-content a', (e,commit)=>e.href==='https://github.com/mo0ogly/swarm/commit/'+commit&&e.rel.includes('noopener'),binary.commit));
 check(label+'-version-dirty-disclosure',binary.modified===true&&text.includes(lang==='en'?'uncommitted changes':'modifications non commitées'));
 check(label+'-version-actual-local-notes',actual.history.local_changes.items.every(item=>text.includes(item[lang])));
 check(label+'-version-no-invented-source-comparison',text.includes(lang==='en'?'Local sources have not been examined':'Sources locales non examinées')&&!text.includes(lang==='en'?'Comparative diagnostic':'Diagnostic comparatif'));
 for(const width of [1440,390]){await page.setViewport({width,height:1000});check(label+'-version-no-overflow-'+width,await page.evaluate(()=>{const d=document.querySelector('#version-dialog');return d.scrollWidth<=d.clientWidth&&document.documentElement.scrollWidth<=innerWidth}));if(width===390)await page.screenshot({path:require('node:path').join(cfg.output,label+'-version-390.png'),fullPage:true})}
 // Restore the desktop opener visibility after viewport-only layout checks.
 await page.setViewport({width:1440,height:1000});await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#version-dialog').open&&document.activeElement.id==='version-open');check(label+'-version-escape-focus',await page.$eval('#version-open',e=>e===document.activeElement));
 await page.setViewport({width:1440,height:1000});
 if(lang==='fr'&&label==='fr-etat'){
  await page.evaluate(()=>{window.versionFetch=window.fetch;window.fetch=(...args)=>new URL(args[0],location.href).pathname==='/api/v1/native-version'?new Promise(resolve=>{window.releaseVersion=()=>window.versionFetch(...args).then(resolve)}):window.versionFetch(...args)});
  await page.click('#version-open');await page.waitForFunction(()=>document.querySelector('#version-dialog-content').textContent.includes('Chargement de la version et des nouveautés'));
  check('version-loading-clears-stale-identity',await page.$eval('#version-dialog-content',e=>!e.textContent.includes('Binaire lancé')));
  await page.evaluate(()=>window.releaseVersion());await page.waitForFunction(()=>VersionHistoryPanel.value&&!VersionHistoryPanel.pending);check('version-delayed-real-read-recovery',await page.$eval('#version-dialog-content',(e,v)=>e.textContent.includes(v),binary.version));await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#version-dialog').open);
  await page.evaluate(()=>{window.fetch=async(...args)=>new URL(args[0],location.href).pathname==='/api/v1/native-version'?new Response('{}',{status:200,headers:{'Content-Type':'application/json'}}):window.versionFetch(...args)});
  await page.click('#version-open');await page.waitForFunction(()=>VersionHistoryPanel.error&&!VersionHistoryPanel.pending);check('version-malformed-not-stale-success',await page.$eval('#version-dialog-content',e=>e.textContent.includes('indisponibles')&&!e.querySelector('a')));check('version-malformed-safe-cause',await page.evaluate(()=>VersionHistoryPanel.error.code==='version-schema-unsupported'||VersionHistoryPanel.error.code==='version-contract'));await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#version-dialog').open);
  await page.evaluate(()=>{window.fetch=window.versionFetch;delete window.versionFetch;delete window.releaseVersion});await page.click('#version-open');await page.waitForFunction(()=>VersionHistoryPanel.value&&!VersionHistoryPanel.pending);check('version-explicit-reopen-recovery',await page.$eval('#version-dialog-content',(e,v)=>e.textContent.includes(v),binary.version));await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#version-dialog').open);
 }
 check(label+'-version-no-legacy-health-probe',!nativeRequests.includes('/api/v1/runtime-health'));page.off('request',observe);
};
