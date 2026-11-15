'use strict';
/** Render the real Trust page in security view (web/trust-native.html with
 * trust-native.js + trust-workbench.js) against a local HTTP double and check
 * the "Sécurité" layout: configuration form placed in the page (not in the
 * dialog) and filled at load, I3 token banner, five domain tabs with keyboard
 * selection, integrity levels and identity table drawn from the declared
 * identities of the `show` projection (empty state, then "Relire la
 * configuration" refill without opening the dialog), key steps, save bar with a
 * real config-preview round trip (admin bearer, no apply), audit-only controls
 * hidden, FR/EN, both themes, 1440/390 px without horizontal overflow.
 * Scope: UI behaviour on stub projections; no native service, provider, live
 * Root or real configuration write. Usage:
 * node tests/security_workbench_ui_test.cjs FRESH_OUTPUT
 * (screenshots and Chrome profile go there; no watchdog).
 */
const fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const output=process.argv[2];if(!output||process.argv.length!==3)throw Error('Usage: node tests/security_workbench_ui_test.cjs FRESH_OUTPUT');
fs.mkdirSync(output);const requests=[],errors=[];let count=0,server,browser;
const csrf='c'.repeat(64);
const base={schema_version:1,revision:1,policy:{mandate_ttl_seconds:3600,unknown_reconcile_seconds:0,evidence_retention_days:365,require_separation:true},actions:{},amount_minor_text:{},
 evidence_signing:{enabled:false,key_ttl_seconds:86400},public_evidence_registry:{revision:0,keys:{}},public_evidence_runtime:{private_available:{}}};
const empty={...base,identities:[]};
const full={...base,identities:[{subject:'admin-a',integrity:3,roles:['admin']},{subject:'approbateur-c',integrity:2,roles:['approver']},
 {subject:'compte-d',integrity:2,roles:['effect'],enrollment_principal:{principal_id:'p-1',issuer:'local',subject:'d'}},{subject:'demandeur-b',integrity:1,roles:['requester']}]};
let state=empty;
const assets=new Map(['/trust-native.js','/trust-native.css','/trust-workbench.js','/trust-confidentiality.js','/swarm-workbench.css',
 '/i18n-en.js','/i18n.js','/swarm-design.css','/wattson_themes.css','/swarm-logo.png'].map(url=>[url,'web'+url]));
assets.set('/trust-actions.html','web/trust-native.html');
/** Record one named assertion. */
function check(name,value){assert(value,name);count++;console.log('PASS '+name)}
/** Serve fixed workspace files, the read-only `show` projection and a
 * config-preview that never applies. Any other operation is recorded as an
 * error so a write cannot pass silently. */
function serve(request,response){
 const url=new URL(request.url,'http://localhost');
 if(url.pathname==='/favicon.ico'){response.statusCode=204;response.end();return}
 if(url.pathname==='/api/v1/native-session'){response.setHeader('Content-Type','application/json');response.end(JSON.stringify({schema_version:1,csrf_token:csrf}));return}
 if(url.pathname==='/api/v1/trust-actions'){const chunks=[];request.on('data',c=>chunks.push(c));request.on('end',()=>{const op=url.searchParams.get('action')||'show';requests.push({op,method:request.method,auth:request.headers.authorization||'',body:Buffer.concat(chunks).toString()});response.setHeader('Content-Type','application/json');
  if(request.method==='GET'&&op==='show'){response.end(JSON.stringify(state));return}
  if(request.method==='POST'&&op==='config-preview'){response.end(JSON.stringify({applied:false,current_revision:state.revision}));return}
  errors.push('unexpected operation '+request.method+' '+op);response.statusCode=500;response.end('{"failure":{"code":"fixture-refused"}}')});return}
 if(assets.has(url.pathname)){response.setHeader('Content-Type',{'.html':'text/html; charset=utf-8','.css':'text/css','.js':'application/javascript','.png':'image/png'}[path.extname(url.pathname)]);response.end(fs.readFileSync(assets.get(url.pathname)));return}
 errors.push('unknown fixture path '+url.pathname);response.statusCode=404;response.end();
}
(async()=>{try{
 server=http.createServer(serve);await new Promise(r=>server.listen(0,'127.0.0.1',r));const origin='http://127.0.0.1:'+server.address().port;
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(path.resolve(output),'browser')});
 const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
 const text=sel=>page.$eval(sel,e=>e.textContent.trim());
 const levels=()=>page.$$eval('.security-level',rows=>rows.map(r=>[r.querySelector('.audit-badge').textContent,r.querySelector('.security-level-who strong').textContent,r.querySelector('.security-level-count').textContent]));
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,390]){
  const label=lang+'-'+theme+'-'+width;requests.length=0;state=empty;await page.setViewport({width,height:1000});
  await page.goto(origin+'/trust-actions.html?view=security&lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'});
  check(label+'-form-in-page-not-dialog',await page.$eval('body',b=>b.classList.contains('security-workbench')&&!document.getElementById('settings').contains(document.getElementById('configuration'))&&document.querySelector('main').contains(document.getElementById('configuration'))));
  check(label+'-token-banner',await page.$eval('.security-token',b=>b.contains(document.getElementById('trust-admin-token'))));
  check(label+'-five-tabs-with-subtitles',JSON.stringify(await page.$$eval('[role=tab] small',s=>s.map(x=>x.textContent)))===JSON.stringify(['Biba','Bell-LaPadula',lang==='en'?'separation':'séparation',lang==='en'?'subjects':'sujets','signature']));
  await page.waitForFunction(()=>document.getElementById('trust-identity-sources').dataset.loaded==='true');
  check(label+'-form-filled-at-load',await page.$eval('#p-mandate_ttl_seconds',i=>i.value==='3600')&&await page.$eval('#separation',c=>c.checked));
  check(label+'-empty-identities-stated',/Aucune identité déclarée|No declared identity/.test(await text('.security-table tbody')));
  check(label+'-empty-levels-count-zero',JSON.stringify((await levels()).map(r=>[r[0],r[1]]))==='[["I3","—"],["I2","—"],["I1","—"]]'&&(await levels()).every(r=>/^0 /.test(r[2])));
  state=full;const before=requests.length;await page.click('#admin');
  await page.waitForFunction(()=>document.querySelectorAll('.security-table tbody tr').length===4);
  check(label+'-reread-is-one-get-and-no-dialog',requests.slice(before).every(r=>r.method==='GET'&&r.op==='show')&&requests.length===before+1&&await page.$eval('#settings',d=>!d.open));
  check(label+'-levels-from-declared-identities',JSON.stringify((await levels()).map(r=>[r[0],r[1]]))==='[["I3","admin-a"],["I2","approbateur-c, compte-d"],["I1","demandeur-b"]]');
  check(label+'-level-counts',JSON.stringify((await levels()).map(r=>r[2].split(' ')[0]))==='["1","2","1"]');
  check(label+'-identity-table-auth',JSON.stringify(await page.$$eval('.security-table tbody tr',rows=>rows.map(r=>r.cells[3].textContent)))===JSON.stringify(lang==='en'?['Local token','Local token','Enrolled account (enrollment_principal)','Local token']:['Jeton local','Jeton local','Compte enrôlé (enrollment_principal)','Jeton local']));
  check(label+'-identity-badge-tones',JSON.stringify(await page.$$eval('.security-table .audit-badge',b=>b.map(x=>x.dataset.tone)))==='["bad","wait","wait","ok"]');
  await page.focus('#security-tab-biba');await page.keyboard.press('ArrowRight');
  check(label+'-tab-keyboard',await page.$eval('#security-tab-blp',e=>e===document.activeElement&&e.getAttribute('aria-selected')==='true')&&await page.$eval('#security-panel-biba',p=>p.hidden));
  await page.keyboard.press('End');
  check(label+'-key-steps',await page.$eval('#security-panel-keys',p=>!p.hidden&&p.querySelectorAll('.security-steps li').length===3&&p.contains(document.getElementById('trust-key-prepare'))));
  check(label+'-audit-controls-hidden',await page.$eval('#trust-audit-title',e=>!!e.closest('[hidden]'))&&await page.$eval('#trust-public-verify',e=>!!e.closest('[hidden]')));
  check(label+'-savebar',await page.$eval('.security-savebar',b=>['reason','admin-message','preview','save'].every(id=>b.contains(document.getElementById(id))))&&await page.$eval('#close',c=>c.hidden)&&await page.$eval('#save',s=>s.disabled));
  check(label+'-savebar-visible-on-every-tab',await page.$eval('.security-savebar',b=>b.getClientRects().length>0));
  await page.keyboard.press('Home');await page.click('#security-tab-identities');
  await page.type('#trust-admin-token','synthetic-admin-token');
  await page.$eval('#identities',t=>{t.value='[{"subject":"admin-a","integrity":3,"roles":["admin"],"token":"t"}]';t.dispatchEvent(new Event('input',{bubbles:true}))});
  await page.type('#reason','fixture preview');await page.click('#preview');
  await page.waitForFunction(()=>!document.getElementById('save').disabled);
  const preview=requests.find(r=>r.op==='config-preview');
  check(label+'-preview-uses-admin-bearer-and-does-not-apply',preview&&preview.auth==='Bearer synthetic-admin-token'&&JSON.parse(preview.body).reason==='fixture preview'&&!requests.some(r=>r.op==='config-apply'));
  check(label+'-preview-status-in-savebar',/Prévisualisation validée|Preview validated/.test(await text('.security-savebar #admin-message')));
  check(label+'-no-horizontal-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.click('#security-tab-biba');await page.screenshot({path:path.join(output,label+'.png'),fullPage:true});
 }
 assert.deepEqual(errors,[]);console.log(count+' security workbench checks passed');
}catch(error){console.error(error);console.error('errors:',errors);process.exitCode=1}finally{await browser?.close();server?.close()}})();
