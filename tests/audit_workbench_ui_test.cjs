'use strict';
/** Render the real Trust page in audit view (web/trust-native.html with
 * trust-native.js + trust-workbench.js) against a local HTTP double and check
 * the "Audit et preuves" layout: counters computed from recorded states (0 on
 * an empty projection), Refresh refilling the list, transaction list and filter, selection without any request,
 * proof chain drawn only from the matching audit report, missing-evidence
 * notice, FR/EN, both themes, 1440/390 px without horizontal overflow.
 * Scope: UI behaviour on stub projections; no native service, provider, live
 * Root or real audit record. Usage: node tests/audit_workbench_ui_test.cjs
 * FRESH_OUTPUT (screenshots and Chrome profile go there; no watchdog).
 */
const fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const output=process.argv[2];if(!output||process.argv.length!==3)throw Error('Usage: node tests/audit_workbench_ui_test.cjs FRESH_OUTPUT');
fs.mkdirSync(output);const requests=[],errors=[];let count=0,server,browser;
const csrf='b'.repeat(64);
const full={schema_version:1,revision:1,policy:{mandate_ttl_seconds:3600,unknown_reconcile_seconds:0,evidence_retention_days:365,require_separation:true},
 actions:{'act-1':{id:'act-1',state:'reconciled',resource:'fournisseur',currency:'EUR'},'act-2':{id:'act-2',state:'pending_approval',resource:'depot-code',currency:'EUR'},
  'act-3':{id:'act-3',state:'denied',resource:'export-donnees',currency:'EUR'},'act-4':{id:'act-4',state:'unknown',resource:'paiement',currency:'EUR'}},
 amount_minor_text:{'act-1':'12.50','act-2':'0','act-3':'0','act-4':'99.00'}};
const empty={...full,actions:{},amount_minor_text:{}};let state=empty;
const audit={schema_version:1,kind:'trust-transaction-audit-v1',action_id:'act-2',state:'pending_approval',chain_verified:true,transaction_verified:false,complete:false,
 missing_evidence:['approval-proof'],access_recorded:true,audit_access:{hash:'f'.repeat(64)},
 events:[{sequence:'1',kind:'mandate-create',actor:'admin-a',integrity:3,result:'admitted',at:'2026-11-10T09:00:00Z',hash:'a'.repeat(64)},
  {sequence:'2',kind:'request',actor:'demandeur-b',integrity:1,result:'admitted',at:'2026-11-10T09:01:00Z',hash:'b'.repeat(64)}]};
const assets=new Map(['/trust-native.js','/trust-native.css','/trust-workbench.js','/trust-confidentiality.js','/swarm-workbench.css',
 '/i18n-en.js','/i18n.js','/swarm-design.css','/wattson_themes.css','/swarm-logo.png'].map(url=>[url,'web'+url]));
assets.set('/trust-actions.html','web/trust-native.html');
/** Record one named assertion. */
function check(name,value){assert(value,name);count++;console.log('PASS '+name)}
/** Serve fixed workspace files and read-only trust projections. Any other
 * operation is recorded as an error so a write cannot pass silently. */
function serve(request,response){
 const url=new URL(request.url,'http://localhost');
 if(url.pathname==='/favicon.ico'){response.statusCode=204;response.end();return}
 if(url.pathname==='/api/v1/native-session'){response.setHeader('Content-Type','application/json');response.end(JSON.stringify({schema_version:1,csrf_token:csrf}));return}
 if(url.pathname==='/api/v1/trust-actions'){const chunks=[];request.on('data',c=>chunks.push(c));request.on('end',()=>{const op=url.searchParams.get('action')||'show';requests.push({op,auth:request.headers.authorization||''});response.setHeader('Content-Type','application/json');
  if(request.method==='GET'&&op==='show'){response.end(JSON.stringify(state));return}
  if(op==='audit-transaction'){response.end(JSON.stringify(audit));return}
  errors.push('unexpected operation '+op);response.statusCode=500;response.end('{"failure":{"code":"fixture-refused"}}')});return}
 if(assets.has(url.pathname)){response.setHeader('Content-Type',{'.html':'text/html; charset=utf-8','.css':'text/css','.js':'application/javascript','.png':'image/png'}[path.extname(url.pathname)]);response.end(fs.readFileSync(assets.get(url.pathname)));return}
 errors.push('unknown fixture path '+url.pathname);response.statusCode=404;response.end();
}
(async()=>{try{
 server=http.createServer(serve);await new Promise(r=>server.listen(0,'127.0.0.1',r));const base='http://127.0.0.1:'+server.address().port;
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(path.resolve(output),'browser')});
 const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
 const text=sel=>page.$eval(sel,e=>e.textContent.trim());
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,390]){
  const label=lang+'-'+theme+'-'+width;requests.length=0;state=empty;await page.setViewport({width,height:1000});
  await page.goto(base+'/trust-actions.html?view=audit&lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'});
  check(label+'-layout-mounted',await page.$eval('body',b=>b.classList.contains('audit-workbench')&&!!document.querySelector('.audit-grid .audit-transactions')&&!!document.querySelector('.audit-grid .audit-chain')&&!!document.querySelector('.audit-grid .audit-verify')));
  check(label+'-header-actions-are-real-buttons',await page.evaluate(()=>{const a=document.querySelector('.audit-actions');return a.contains(document.getElementById('export'))&&a.contains(document.getElementById('anchor'))}));
  await page.waitForFunction(()=>document.getElementById('trust-audit-action').dataset.loaded==='true');
  check(label+'-empty-projection-counts-zero',JSON.stringify(await page.$$eval('.audit-card .audit-badge',b=>b.map(x=>x.textContent)))==='["0","0","0"]');
  check(label+'-empty-list-message',/Aucune transaction enregistrée|No recorded transaction/.test(await text('.audit-list')));
  state=full;await page.click('#refresh');
  await page.waitForFunction(()=>document.querySelectorAll('.audit-tx').length===4);
  check(label+'-counters-from-recorded-states',JSON.stringify(await page.$$eval('.audit-card .audit-badge',b=>b.map(x=>x.textContent)))==='["4","1","1"]');
  check(label+'-badges-carry-state-tone',JSON.stringify(await page.$$eval('.audit-tx .audit-badge',b=>b.map(x=>x.dataset.tone)))==='["ok","wait","bad","run"]');
  check(label+'-title-from-resource-and-amount',/fournisseur · 12\.50 EUR/.test(await text('.audit-tx[data-tx="act-1"]')));
  const before=requests.length;await page.click('.audit-tx[data-tx="act-2"]');
  check(label+'-select-does-not-call-api',requests.length===before&&await page.$eval('#trust-audit-action',s=>s.value==='act-2'));
  check(label+'-chain-empty-until-read',(await page.$$('.audit-step')).length===0&&/depot-code/.test(await text('#audit-chain-title')));
  await page.type('#trust-admin-token','synthetic-admin-token');await page.click('#trust-audit-read');
  await page.waitForFunction(()=>document.querySelectorAll('.audit-step').length===2);
  check(label+'-audit-uses-admin-token',requests.some(r=>r.op==='audit-transaction'&&r.auth==='Bearer synthetic-admin-token'));
  check(label+'-chain-steps-from-events',/Mandat admis|Mandate admitted/.test(await text('.audit-steps'))&&JSON.stringify(await page.$$eval('.audit-dot',d=>d.map(x=>x.dataset.tone)))==='["ok","ok"]');
  check(label+'-missing-evidence-shown',await page.$eval('.audit-missing',m=>!m.hidden&&m.textContent.includes('approval-proof')));
  await page.click('.audit-tx[data-tx="act-1"]');
  check(label+'-other-transaction-hides-foreign-chain',(await page.$$('.audit-step')).length===0);
  await page.type('.audit-filter input','export');
  check(label+'-filter-narrows-list',JSON.stringify(await page.$$eval('.audit-tx',b=>b.map(x=>x.dataset.tx)))==='["act-3"]');
  check(label+'-no-horizontal-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.$eval('.audit-filter input',i=>{i.value='';i.dispatchEvent(new Event('input'))});await page.click('.audit-tx[data-tx="act-2"]');
  await page.screenshot({path:path.join(output,label+'.png'),fullPage:true});
 }
 assert.deepEqual(errors,[]);console.log(count+' audit workbench checks passed');
}catch(error){console.error(error);console.error('errors:',errors);process.exitCode=1}finally{await browser?.close();server?.close()}})();
