'use strict';
/** Exercise the actual default-profile native executable and embedded PAEA UI.
 * Fresh isolated Root only. Native success/refusal controls; browser-only corrupt
 * reply injections are separately named. No provider/live mission mutation.
 * The caller supplies an exact existing binary/hash and a fresh evidence path.
 * No build, retry or watchdog; readiness is observed from the real listener/UI.
 * Default full scope retains 12 presentation cases per view and real example
 * downloads/uploads on all five views in each configuration. Explicit encoding
 * scope exercises one real invoice/BOM journey and exact current PAEA script;
 * it does not qualify receipt/presentation matrices or unrelated changed assets.
 */
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {spawn}=require('node:child_process'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const output=process.argv[2]||process.env.SWARM_PAEA_UI_EVIDENCE,binary=process.env.SWARM_PAEA_TEST_BINARY,expected=process.env.SWARM_PAEA_TEST_BINARY_SHA256;
const faultMode=process.env.SWARM_PAEA_NATIVE_FAULT;assert([undefined,'script'].includes(faultMode));
const scope=process.env.SWARM_PAEA_NATIVE_SCOPE||'full';assert(['full','payment-encoding'].includes(scope));assert(scope==='full'||faultMode===undefined);
assert(output&&process.argv.length<=3&&binary&&/^[a-f0-9]{64}$/.test(expected||''),'explicit binary/hash/fresh output required');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
assert.equal(sha(fs.readFileSync(binary)),expected);fs.mkdirSync(output);
const root=path.resolve(output,'root');fs.mkdirSync(root);
const vectors=JSON.parse(fs.readFileSync('tests/fixtures/paea/contract-v1.json')).vectors;
const checks=[],errors=[],responses=[],cases=[],paymentCases=[],comparisonCases=[],inspectionCases=[],approvalCases=[],primaryContrastSamples=[],exampleCases=[],serverErrors=[];let browser,server,closed,token='',address='',expectRefusal=false,abort,stopping=false;
const observedFailure=new Promise((resolve,reject)=>{abort=reject});
/** Preserve unexpected browser failures and reject pending DOM waits without a
 * deadline. Expected native 400 refusals remain separately recorded. */
function browserFailure(value){errors.push(value);abort(Error('unexpected-native-browser-diagnostic'))}
/** Retain uniquely named observable assertions; zero-case runs cannot pass. */
function check(name,value){assert(value,name);assert(!checks.includes(name));checks.push(name);console.log('PASS '+name)}
/** Capture one supported CLI operation without shell, truncation or timeout.
 * Optional internal evidenceName retains argv/both streams; cwd, when supplied,
 * is an owned fixture directory. Failures propagate; start callers stay unchanged. */
async function command(args,evidenceName,cwd){
 if(evidenceName)assert(/^[A-Za-z0-9_.-]+$/.test(evidenceName));
 const child=spawn(binary,args,{cwd,stdio:['ignore','pipe','pipe']}),out=[],err=[];
 child.stdout.on('data',b=>out.push(b));child.stderr.on('data',b=>err.push(b));
 const result=await new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',(code,signal)=>resolve({code,signal}))});
 if(evidenceName){fs.writeFileSync(path.join(output,evidenceName+'.argv.json'),JSON.stringify([binary,...args]));fs.writeFileSync(path.join(output,evidenceName+'.stdout'),Buffer.concat(out));fs.writeFileSync(path.join(output,evidenceName+'.stderr'),Buffer.concat(err))}
 assert.equal(result.signal,null);assert.equal(result.code,0,Buffer.concat(err).toString());return Buffer.concat(out);
}
/** Start only the owned fixture server; early exit rejects listener readiness. */
async function start(){
 await command(['--root',root,'init']);
 server=spawn(binary,['--root',root,'--json','web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
 closed=new Promise(resolve=>server.once('close',(code,signal)=>{resolve({code,signal});if(!stopping)browserFailure({kind:'owned-server-exit',code,signal})}));
 let buffer='';server.stderr.on('data',b=>serverErrors.push(b));
 const line=await new Promise((resolve,reject)=>{
  server.once('error',reject);server.once('close',()=>reject(Error('native listener exited before announcement')));
  server.stdout.on('data',b=>{buffer+=b.toString();const n=buffer.indexOf('\n');if(n>=0)resolve(buffer.slice(0,n))});
 });
 address=JSON.parse(line).address;assert(/^127\.0\.0\.1:[1-9][0-9]*$/.test(address));
 assert.equal(sha(fs.readFileSync('/proc/'+server.pid+'/exe')),expected);
 const key=path.join(root,'.swarm','web-session-'+sha(address).slice(0,16));
 const fd=fs.openSync(key,fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW);
 try{const info=fs.fstatSync(fd);assert(info.isFile()&&info.uid===process.getuid()&&(info.mode&0o077)===0);token=fs.readFileSync(fd,'utf8').trim()}finally{fs.closeSync(fd)}
 assert(/^session-[A-Za-z0-9_-]{24}$/.test(token));
}
/** Upload exact fixture bytes and choose explicit sink/state via normal controls. */
async function fill(page,document,receipt){
 await (await page.$('#paea-document')).uploadFile(document);await (await page.$('#paea-receipt')).uploadFile(receipt);
 await page.$eval('#paea-sink',e=>{e.value='sink-native-fixture';e.dispatchEvent(new Event('input',{bubbles:true}))});
 await page.select('#paea-state','COMMIT_REQUESTED');
}
/** Observe a real native response and resulting DOM; expected refusal is narrowly scoped. */
async function submit(page,status){
 expectRefusal=status===400;
 const response=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/receipt-report'&&r.request().method()==='POST');
 await page.click('#paea-check');const actual=await response;assert.equal(actual.status(),status);
 const value=await actual.json();await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 if(status===200)await page.waitForFunction(()=>!document.getElementById('paea-download').disabled);
 else await page.waitForFunction(()=>document.getElementById('paea-status').dataset.state==='error');
 expectRefusal=false;return value;
}
/** Upload only named exact UTF-8 registry/request fixture files. */
async function fillPayment(page,registry,request){
 await (await page.$('#paea-registry')).uploadFile(registry);await (await page.$('#paea-request')).uploadFile(request);
}
/** Exercise the real native resolver, retaining raw i64 JSON report bytes. The
 * response body is never decoded with a lossy test-side JSON number conversion. */
async function submitPayment(page,status){
 expectRefusal=status===400;
 const pending=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/resolve-payment'&&r.request().method()==='POST');
 await page.click('#paea-check');const response=await pending;assert.equal(response.status(),status);
 const raw=await response.text();await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 await page.waitForFunction(expected=>document.getElementById('paea-status').dataset.state===expected,{},status===200?'success':'error');
 expectRefusal=false;return raw;
}
/** Complete menu-to-resolution journey with exact numbers, native rejection,
 * recovery, keyboard and a real completed download. Data remain unauthenticated. */
async function paymentCase(page,context,label,lang,theme){
 await page.goto('http://'+address+'/paea.html?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
 const link='a[data-paea-link]';await page.waitForSelector(link);
 await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click(link)]);
 await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 check(label+'-payment-menu-preserves-presentation',await page.evaluate((l,t)=>location.pathname==='/paea-payment.html'&&document.documentElement.lang===l&&document.documentElement.dataset.theme===t,lang,theme));
 check(label+'-payment-primary-translated',await page.$eval('#paea-check',(e,l)=>e.textContent.trim()===(l==='en'?'Resolve reference':'Résoudre la référence'),lang));
 check(label+'-payment-limits-and-empty',await page.$eval('#paea-limits',(e,l)=>e.textContent.includes(l==='en'?'unauthenticated':'pas authentifié'),lang)&&await page.$eval('#paea-download',e=>e.disabled));
 const embedded=await page.evaluate(async()=>{const r=await fetch('/paea-payment.html');if(!r.ok)throw Error('payment asset unavailable');return Array.from(new Uint8Array(await r.arrayBuffer()))});check(label+'-payment-embedded-page-bytes',sha(Buffer.from(embedded))===sha(fs.readFileSync('web/paea-payment.html')));
 check(label+'-payment-help-collapsed',await page.$eval('#paea-format-help',e=>!e.open));
 await page.focus('#paea-format-help summary');await page.keyboard.press('Enter');check(label+'-payment-help-keyboard-translated',await page.$eval('#paea-format-help',(e,l)=>e.open&&e.textContent.includes(l==='en'?'same project':'même projet'),lang));await page.keyboard.press('Enter');
 const max='9223372036854775807',directory=path.resolve(output,label+'-resolution');fs.mkdirSync(directory);
 const registry=path.join(directory,'registry.json'),request=path.join(directory,'request.json'),bad=path.join(directory,'wrong-project.json');
 const reg=JSON.parse(fs.readFileSync('tests/fixtures/paea/payment-registry-v1.json','utf8')),req=JSON.parse(fs.readFileSync('tests/fixtures/paea/payment-request-v1.json','utf8'));
 reg.debts[0].amount_minor=max;
 const rawRegistry=JSON.stringify(reg).replace('"revision":1','"revision":'+max),rawRequest=JSON.stringify(req).replace('"expected_registry_revision":1','"expected_registry_revision":'+max);
 assert(rawRegistry.includes('"revision":'+max)&&rawRequest.includes('"expected_registry_revision":'+max));
 fs.writeFileSync(registry,rawRegistry);fs.writeFileSync(request,rawRequest);fs.writeFileSync(bad,rawRequest.replace('project-alpha','wrong-project'));
 await fillPayment(page,registry,request);const report=await submitPayment(page,200);fs.writeFileSync(path.join(directory,'native-report.json'),report);
 check(label+'-payment-exact-revision-amount',report.includes('"registry_revision":'+max)&&report.includes('"amount_minor":"'+max+'"')&&await page.$eval('#paea-identities',(e,n)=>e.textContent.split(n).length===3,max));
 check(label+'-payment-input-hashes',report.includes(sha(rawRegistry))&&report.includes(sha(rawRequest))&&await page.$eval('#paea-report',(e,r)=>e.textContent===r,report));
 check(label+'-payment-debt-and-no-authority',report.includes('"invoice_id":"debt-17"')&&report.includes('"registry_authenticity":"not-established"')&&report.includes('"effect":"not-established"'));
 await page.$eval('#paea-check',e=>e.focus());await page.keyboard.press('Tab');check(label+'-payment-download-keyboard',await page.evaluate(()=>document.activeElement.id==='paea-download'));
 const cdp=await page.createCDPSession();assert.equal(typeof context.id,'string');
 await cdp.send('Browser.setDownloadBehavior',{behavior:'allow',downloadPath:directory,browserContextId:context.id,eventsEnabled:true});
 const downloaded=new Promise((resolve,reject)=>cdp.on('Browser.downloadProgress',event=>{if(event.state==='completed')resolve();if(event.state==='canceled')reject(Error('payment-download-canceled'))}));
 await page.click('#paea-download');await downloaded;await cdp.detach();
 check(label+'-payment-real-download-exact',fs.readFileSync(path.join(directory,'paea-payment-resolution.json'),'utf8')===report);
 await fillPayment(page,registry,bad);check(label+'-payment-edit-clears-stale',await page.$eval('#paea-report',e=>e.textContent==='')&&await page.$eval('#paea-download',e=>e.disabled));
 const refused=await submitPayment(page,400);check(label+'-payment-native-refusal',JSON.parse(refused).failure.code==='protection-registry-project-mismatch'&&await page.$eval('#paea-download',e=>e.disabled));
 await fillPayment(page,registry,request);check(label+'-payment-native-recovery',(await submitPayment(page,200))===report);
 check(label+'-payment-no-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
 const colors=await page.$eval('#paea-check',e=>{const s=getComputedStyle(e);return [s.color,s.backgroundColor]});
 const luminance=color=>color.match(/[\d.]+/g).slice(0,3).map(Number).map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((v,c,i)=>v+c*[.2126,.7152,.0722][i],0);
 const colorsL=colors.map(luminance);check(label+'-payment-primary-contrast-AA',(Math.max(...colorsL)+.05)/(Math.min(...colorsL)+.05)>=4.5);
 await page.screenshot({path:path.join(directory,'result.png'),fullPage:true});paymentCases.push(label);
}
/** Exercise original BOM bytes through the actual protected file-upload route.
 * Each side has a valid control, typed native refusal, empty stale result and
 * exact recovery. No substituted response, live Root or effect is involved.
 * Requires paymentCase fixtures for label; writes only owned BOM copies.
 * Returns after recovery; IO, DOM and assertion failures propagate to cleanup. */
async function paymentBomFiles(page,label){
 const directory=path.resolve(output,label+'-resolution'),registry=path.join(directory,'registry.json'),request=path.join(directory,'request.json'),good=fs.readFileSync(path.join(directory,'native-report.json'),'utf8');
 for(const side of ['registry','request']){
  check('payment-'+side+'-bom-valid-control',(await submitPayment(page,200))===good);
  const file=path.join(directory,side+'-bom.json');fs.writeFileSync(file,Buffer.concat([Buffer.from([239,187,191]),fs.readFileSync(side==='registry'?registry:request)]));
  await fillPayment(page,side==='registry'?file:registry,side==='request'?file:request);
  const sent=page.waitForRequest(r=>new URL(r.url()).pathname==='/api/v1/protection/resolve-payment'&&r.method()==='POST');
  const refused=await submitPayment(page,400),body=JSON.parse((await sent).postData());
  check('payment-'+side+'-bom-preserved-and-native-refused',body[side].codePointAt(0)===0xfeff&&JSON.parse(refused).failure.code==='protection-document-invalid'&&await page.$eval('#paea-download',e=>e.disabled)&&await page.$eval('#paea-report',e=>e.textContent===''));
  await fillPayment(page,registry,request);check('payment-'+side+'-bom-native-recovery',(await submitPayment(page,200))===good);
 }
}
/** Browser-only faults follow real native controls. Response substitution tests
 * the client boundary only; these synthetic reports never count as sink evidence.
 * Hold/release uses an observed request event, no sleep/watchdog or retry.
 * BOM cases retain original upload bytes through the real API and then recover. */
async function paymentClientFaults(page,label){
 const directory=path.resolve(output,label+'-resolution'),registry=path.join(directory,'registry.json'),request=path.join(directory,'request.json');
 const good=fs.readFileSync(path.join(directory,'native-report.json'),'utf8'),max='9223372036854775807';
 let replacement=null,held,arrived,hold=false,posts=0;
 await page.setRequestInterception(true);
 const intercept=r=>{
  if(new URL(r.url()).pathname!=='/api/v1/protection/resolve-payment'||r.method()!=='POST'){r.continue().catch(e=>browserFailure({kind:'interception',message:e.message}));return;}
  posts++;
  if(hold){held=()=>r.continue().catch(e=>browserFailure({kind:'hold-release',message:e.message}));arrived();return;}
  if(replacement!==null)r.respond({status:200,contentType:'application/json',body:replacement}).catch(e=>browserFailure({kind:'fixture-response',message:e.message}));
  else r.continue().catch(e=>browserFailure({kind:'interception',message:e.message}));
 };
 page.on('request',intercept);
 for(const [name,raw] of [
  ['hash',good.replace(/"request_sha256":"[a-f0-9]{64}"/,'"request_sha256":"'+'0'.repeat(64)+'"')],
  ['authority',good.replace('"authority":"not-established"','"authority":"established"')],
  ['revision-string',good.replace('"registry_revision":'+max,'"registry_revision":"'+max+'"')],
  ['revision-exponent',good.replace('"registry_revision":'+max,'"registry_revision":1e3')],
  ['numeric-amount',good.replace('"amount_minor":"'+max+'"','"amount_minor":'+max)],
  ['extra',good.slice(0,-1)+',"extra":true}'],['malformed','{']]){
  replacement=null;check('payment-client-'+name+'-valid-control',(await submitPayment(page,200))===good);
  replacement=raw;fs.writeFileSync(path.join(directory,'synthetic-'+name+'.json'),raw);
  const response=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/resolve-payment'&&r.request().method()==='POST');
  await page.click('#paea-check');assert.equal((await response).status(),200);
  await page.waitForFunction(()=>document.getElementById('paea-status').dataset.state==='error'&&!document.getElementById('paea-check').disabled);
  check('payment-client-rejects-'+name,await page.$eval('#paea-download',e=>e.disabled)&&await page.$eval('#paea-report',e=>e.textContent===''));
 }
 replacement=null;check('payment-client-final-native-recovery',(await submitPayment(page,200))===good);
 const waiting=new Promise(resolve=>{arrived=resolve});hold=true;const before=posts,pending=submitPayment(page,200);await waiting;
 check('payment-loading-blocks-inputs',await page.$eval('#paea-inputs',e=>e.disabled)&&await page.$eval('#paea-check',e=>e.disabled)&&await page.$eval('#paea-download',e=>e.disabled));
 await page.$eval('#paea-form',e=>e.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})));check('payment-no-duplicate-gesture',posts===before+1);hold=false;await held();check('payment-held-native-recovery',(await pending)===good);
 await page.evaluate(()=>{globalThis.originalPaeaParse=JSON.parse;JSON.parse=(raw,reviver)=>originalPaeaParse(raw,typeof reviver==='function'?function(key,value){return reviver.call(this,key,value)}:reviver)});
 const noLexeme=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/resolve-payment'&&r.request().method()==='POST');
 await page.click('#paea-check');assert.equal((await noLexeme).status(),200);await page.waitForFunction(()=>document.getElementById('paea-status').dataset.state==='error');
 check('payment-unsupported-number-reader-refuses',await page.$eval('#paea-status',e=>e.textContent.includes('CLI'))&&await page.$eval('#paea-download',e=>e.disabled));
 await page.evaluate(()=>{JSON.parse=originalPaeaParse;delete globalThis.originalPaeaParse});check('payment-number-reader-recovery',(await submitPayment(page,200))===good);
 await paymentBomFiles(page,label);
 const invalid=path.join(directory,'invalid-utf8.json');fs.writeFileSync(invalid,Buffer.from([255]));await fillPayment(page,invalid,request);const prior=posts;await page.click('#paea-check');
 await page.waitForFunction(()=>document.getElementById('paea-status').dataset.state==='error'&&!document.getElementById('paea-check').disabled);
 check('payment-invalid-utf8-never-sent',posts===prior&&await page.$eval('#paea-download',e=>e.disabled));
 await fillPayment(page,registry,request);check('payment-utf8-recovery',(await submitPayment(page,200))===good);
 page.off('request',intercept);await page.setRequestInterception(false);
}
/** Upload named original action documents via actual controls; no JS number parsing. */
async function fillComparison(page,before,after){await(await page.$('#paea-before')).uploadFile(before);await(await page.$('#paea-after')).uploadFile(after)}
/** Observe native completion then assert its terminal DOM state; a wrong state
 * fails immediately after completion instead of waiting forever for expected text. */
async function submitComparison(page,status){
 expectRefusal=status===400;
 const pending=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/compare'&&r.request().method()==='POST');
 await page.click('#paea-check');const response=await pending;assert.equal(response.status(),status);const raw=await response.text();
 await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 assert.equal(await page.$eval('#paea-status',e=>e.dataset.state),status===200?'success':'error');expectRefusal=false;return raw;
}
/** Exercise the new menu on both domains, exact i64 input strings, CLI/API parity,
 * changes/equality, real downloads, refusal/recovery and existing presentation
 * matrix. All generated documents/reports stay in owned fixture directories. */
async function comparisonCase(page,context,label,lang,theme){
 await page.goto('http://'+address+'/paea.html?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
 await page.waitForSelector('a[href^="/paea-compare.html"]');
 await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click('a[href^="/paea-compare.html"]')]);
 await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 check(label+'-comparison-menu-presentation',await page.evaluate((l,t)=>location.pathname==='/paea-compare.html'&&document.documentElement.lang===l&&document.documentElement.dataset.theme===t,lang,theme));
 check(label+'-comparison-primary-translated',await page.$eval('#paea-check',(e,l)=>e.textContent.trim()===(l==='en'?'Compare actions':'Comparer les actions'),lang));
 check(label+'-comparison-empty-and-limits',await page.$eval('#paea-before',e=>e.files.length===0)&&await page.$eval('#paea-after',e=>e.files.length===0)&&await page.$eval('#paea-download',e=>e.disabled)&&await page.$eval('#paea-limits',(e,l)=>e.textContent.includes(l==='en'?'validates no approval':'ne valide aucun accord'),lang));
 const embedded=await page.evaluate(async()=>{const r=await fetch('/paea-compare.html');if(!r.ok)throw Error('comparison page unavailable');return Array.from(new Uint8Array(await r.arrayBuffer()))});check(label+'-comparison-embedded-page',sha(Buffer.from(embedded))===sha(fs.readFileSync('web/paea-compare.html')));
 await page.focus('#paea-format-help summary');await page.keyboard.press('Enter');check(label+'-comparison-help-keyboard',await page.$eval('#paea-format-help',(e,l)=>e.open&&e.textContent.includes(l==='en'?'sources remain ordered':'sources restent ordonnées'),lang));await page.keyboard.press('Enter');
 for(const kind of ['payment.settle','repository.promote']){
  const prefix=label+'-comparison-'+kind,directory=path.resolve(output,prefix);fs.mkdirSync(directory);
  const vector=vectors.find(v=>v.proposal.action.kind===kind),beforeDoc=structuredClone({proposal:vector.proposal,context:vector.context});beforeDoc.context.mandate_revision=1;
  const exact=(value,pretty=false)=>JSON.stringify(value,null,pretty?2:undefined).replace(/"mandate_revision":\s*1([,}])/,'"mandate_revision":9223372036854775807$1');
  const rawBefore=exact(beforeDoc);assert(rawBefore.includes('9223372036854775807'));
  const before=path.join(directory,'before.json');fs.writeFileSync(before,rawBefore);
  let goodTerms;
  for(const variant of ['same','request','terms','intent','source','formatting']){
   const afterDoc=structuredClone(beforeDoc);
   if(variant==='request')afterDoc.proposal.request_id='comparison-attempt-two';
   if(variant==='terms'){
    if(kind==='payment.settle'){afterDoc.proposal.action.amount_minor='9223372036854775807';afterDoc.proposal.action.beneficiary_account_id='comparison-account'}
    else{const a=afterDoc.proposal.action;const candidate=['e','d','c'].map(c=>c.repeat(a.candidate_oid.length)).find(v=>v!==a.candidate_oid&&v!==a.expected_old_oid);assert(candidate);a.candidate_oid=candidate}
   }
   if(variant==='intent'){if(kind==='payment.settle')afterDoc.proposal.action.invoice_id='comparison-other-debt';else afterDoc.proposal.action.target_ref='refs/heads/comparison-other'}
   if(variant==='source'){assert(!afterDoc.context.sources.some(v=>v.source_id==='comparison-source'));afterDoc.context.sources.push({source_id:'comparison-source',version:'v1',digest:'e'.repeat(64),integrity:1,confidentiality_labels:[]})}
   const rawAfter=exact(afterDoc,variant==='formatting'),after=path.join(directory,'after-'+variant+'.json');fs.writeFileSync(after,rawAfter);
   await fillComparison(page,before,after);check(prefix+'-'+variant+'-edit-clears',await page.$eval('#paea-download',e=>e.disabled)&&await page.$eval('#paea-report',e=>e.textContent===''));
   const raw=await submitComparison(page,200),report=JSON.parse(raw),c=report.comparison;
   fs.writeFileSync(path.join(directory,'native-'+variant+'-report.json'),raw);
   const cli=(await command(['protection','compare',before,after,'--json'],prefix+'-'+variant+'-cli',directory)).toString();
   check(prefix+'-'+variant+'-exact-cli-api',raw===cli&&c.before.document_sha256===sha(rawBefore)&&c.after.document_sha256===sha(rawAfter));
   check(prefix+'-'+variant+'-flags',c.canonical_content_changed===['terms','intent','source'].includes(variant)&&c.intent_changed===(variant==='intent')&&c.transport_request_changed===(variant==='request'));
   check(prefix+'-'+variant+'-non-authority',report.scope==='offline-data-only'&&['authority','effect','approval_validity'].every(k=>c[k]==='not-established')&&await page.$eval('#paea-report',(e,r)=>e.textContent===r,raw));
   if(variant==='formatting')check(prefix+'-formatting-raw-identity',c.before.document_sha256!==c.after.document_sha256&&c.before.action_digest===c.after.action_digest);
   if(variant==='terms'){
    goodTerms=raw;check(prefix+'-changed-field-names',c.changed_paths.includes(kind==='payment.settle'?'/action/amount_minor':'/action/candidate_oid')&&await page.$eval('#paea-identities',(e,l)=>e.textContent.includes(l),kind==='payment.settle'?(lang==='en'?'Integer amount':'Montant entier'):(lang==='en'?'Candidate Git object':'Objet Git candidat')));
    await page.focus('#paea-check');await page.keyboard.press('Tab');check(prefix+'-download-keyboard',await page.evaluate(()=>document.activeElement.id==='paea-download'));
    const cdp=await page.createCDPSession();await cdp.send('Browser.setDownloadBehavior',{behavior:'allow',downloadPath:directory,browserContextId:context.id,eventsEnabled:true});
    const completed=new Promise((resolve,reject)=>cdp.on('Browser.downloadProgress',e=>{if(e.state==='completed')resolve();if(e.state==='canceled')reject(Error('comparison-download-canceled'))}));
    await page.click('#paea-download');await completed;await cdp.detach();check(prefix+'-real-download-exact',fs.readFileSync(path.join(directory,'paea-comparison.json'),'utf8')===raw);
   }
  }
  const terms=path.join(directory,'after-terms.json'),invalid=path.join(directory,'invalid.json');fs.writeFileSync(invalid,'PRIVATE_COMPARISON_BROWSER_CANARY');
  await fillComparison(page,before,terms);check(prefix+'-refusal-valid-control',(await submitComparison(page,200))===goodTerms);
  await fillComparison(page,before,invalid);const refused=await submitComparison(page,400);check(prefix+'-native-refusal-clears',JSON.parse(refused).failure.code==='protection-document-invalid'&&await page.$eval('#paea-download',e=>e.disabled)&&await page.$eval('#paea-report',e=>e.textContent===''));
  await fillComparison(page,before,terms);check(prefix+'-native-recovery',(await submitComparison(page,200))===goodTerms);
  check(prefix+'-no-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.screenshot({path:path.join(directory,'result.png'),fullPage:true});
 }
 comparisonCases.push(label);
}
/** Alter only browser responses after actual native controls; never count these
 * synthetic reports as server/sink evidence. Wrong closed shapes/flags/hashes
 * must invalidate output immediately; recovery uses the real native response. */
async function comparisonClientFaults(page,label){
 const directory=path.resolve(output,label+'-comparison-payment.settle'),before=path.join(directory,'before.json'),after=path.join(directory,'after-terms.json'),good=fs.readFileSync(path.join(directory,'native-terms-report.json'),'utf8');
 await fillComparison(page,before,after);let replacement=null;
 await page.setRequestInterception(true);
 const intercept=r=>{if(new URL(r.url()).pathname==='/api/v1/protection/compare'&&r.method()==='POST'&&replacement!==null)r.respond({status:200,contentType:'application/json',body:replacement}).catch(e=>browserFailure({kind:'comparison-response',message:e.message}));else r.continue().catch(e=>browserFailure({kind:'comparison-interception',message:e.message}))};page.on('request',intercept);
 const mutate=fn=>{const v=JSON.parse(good);fn(v.comparison,v);return JSON.stringify(v)};
 for(const [name,raw]of [
  ['before-hash',mutate(c=>c.before.document_sha256='0'.repeat(64))],['after-hash',mutate(c=>c.after.document_sha256='0'.repeat(64))],
  ['approval',mutate(c=>c.approval_validity='established')],['unknown-path',mutate(c=>c.changed_paths=['/action/PRIVATE_COMPARISON_BROWSER_CANARY'])],
  ['duplicate-path',mutate(c=>c.changed_paths.push(c.changed_paths[0]))],['unordered-path',mutate(c=>c.changed_paths.reverse())],
  ['wrong-flag',mutate(c=>c.canonical_content_changed=false)],['wrong-intent',mutate(c=>c.intent_changed=true)],
  ['flag-type',mutate(c=>c.transport_request_changed='false')],['extra',mutate((c,v)=>v.extra=true)],['malformed','{']]){
  replacement=null;check('comparison-client-'+name+'-native-control',(await submitComparison(page,200))===good);
  replacement=raw;fs.writeFileSync(path.join(directory,'synthetic-'+name+'.json'),raw);
  const pending=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/compare'&&r.request().method()==='POST');await page.click('#paea-check');assert.equal((await pending).status(),200);
  await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
  check('comparison-client-rejects-'+name,await page.$eval('#paea-status',e=>e.dataset.state==='error')&&await page.$eval('#paea-download',e=>e.disabled)&&await page.$eval('#paea-report',e=>e.textContent===''));
 }
 replacement=null;check('comparison-client-native-recovery',(await submitComparison(page,200))===good);page.off('request',intercept);await page.setRequestInterception(false);
}
/** Submit one exact selected file through the real native inspection route.
 * Completion/state mismatches fail immediately after the controller is idle;
 * refusal scope is narrow and no watchdog, retry or report substitution is used. */
async function submitInspection(page,status){
 expectRefusal=status===400;
 const pending=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/inspect'&&r.request().method()==='POST');
 await page.click('#paea-check');const response=await pending;assert.equal(response.status(),status);const raw=await response.text();
 await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 assert.equal(await page.$eval('#paea-status',e=>e.dataset.state),status===200?'success':'error');expectRefusal=false;return raw;
}
/** Inspect both domains through menu/file/native CLI/API/download. Revisions and
 * amounts use exact i64 text; sources are ordered and empty-source state is
 * explicit. Invalid data clears every term/canonical result and recovers exactly.
 * Writes only owned fictional files/screenshots and records native diagnostics. */
async function inspectionCase(page,context,label,lang,theme){
 await page.goto('http://'+address+'/paea.html?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
 await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click('a[href^="/paea-inspect.html"]')]);await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 check(label+'-inspection-menu-presentation',await page.evaluate((l,t)=>location.pathname==='/paea-inspect.html'&&document.documentElement.lang===l&&document.documentElement.dataset.theme===t,lang,theme));
 check(label+'-inspection-primary-translated',await page.$eval('#paea-check',(e,l)=>e.textContent.trim()===(l==='en'?'Inspect action':'Examiner l’action'),lang));
 check(label+'-inspection-empty-limits',await page.$eval('#paea-document',e=>e.files.length===0)&&await page.$eval('#paea-inspected-content',e=>e.hidden)&&await page.$eval('#paea-limits',(e,l)=>e.textContent.includes(l==='en'?'validates no approval':'ne valide aucun accord'),lang));
 const asset=await page.evaluate(async()=>Array.from(new Uint8Array(await(await fetch('/paea-inspect.html')).arrayBuffer())));check(label+'-inspection-exact-embedded-page',sha(Buffer.from(asset))===sha(fs.readFileSync('web/paea-inspect.html')));
 const max='9223372036854775807';
 const encode=value=>JSON.stringify(value).replace(/"(mandate_revision|policy_revision)":1([,}])/g,'"$1":'+max+'$2');
 for(const kind of ['payment.settle','repository.promote']){
  const tag=label+'-inspection-'+kind,directory=path.resolve(output,tag);fs.mkdirSync(directory);
  const vector=vectors.find(v=>v.proposal.action.kind===kind),value=structuredClone({proposal:vector.proposal,context:vector.context});value.context.mandate_revision=1;value.context.policy_revision=1;
  const first=value.context.sources[0].source_id,second=structuredClone(value.context.sources[0]);second.source_id='inspection-second-source';value.context.sources.push(second);
  if(kind==='payment.settle')value.proposal.action.amount_minor=max;
  const file=path.join(directory,'document.json'),rawInput=encode(value);assert(rawInput.includes('"mandate_revision":'+max)&&rawInput.includes('"policy_revision":'+max));fs.writeFileSync(file,rawInput);
  await(await page.$('#paea-document')).uploadFile(file);check(tag+'-edit-clears',await page.$eval('#paea-inspected-content',e=>e.hidden)&&await page.$eval('#paea-report',e=>e.textContent===''));
  const raw=await submitInspection(page,200),report=JSON.parse(raw);fs.writeFileSync(path.join(directory,'native-report.json'),raw);
  const cli=(await command(['protection','inspect','--input',file,'--json'],tag+'-cli',directory)).toString();
  check(tag+'-exact-cli-api',raw===cli&&report.document.document_sha256===sha(rawInput)&&report.document.action_digest===sha(report.canonical));
  check(tag+'-exact-i64-context',await page.$$eval('#paea-context dd',(els,n)=>els.filter(e=>e.textContent===n).length===2,max));
  check(tag+'-exact-terms',await page.$eval('#paea-terms',(e,data)=>e.textContent.includes(data.term),{term:kind==='payment.settle'?max:value.proposal.action.candidate_oid}));
  check(tag+'-source-order',await page.$$eval('#paea-sources li',(els,ids)=>els.length===2&&els.every((e,i)=>e.textContent.startsWith(ids[i]+' · ')),[first,second.source_id]));
  check(tag+'-non-authority-and-canonical',report.scope==='offline-data-only'&&['authority','effect','approval_validity'].every(k=>report[k]==='not-established')&&await page.$eval('#paea-canonical',(e,text)=>e.textContent===text,report.canonical)&&await page.$eval('#paea-report',(e,text)=>e.textContent===text,raw));
  await page.focus('#paea-check');await page.keyboard.press('Tab');check(tag+'-download-keyboard',await page.evaluate(()=>document.activeElement.id==='paea-download'));
  const cdp=await page.createCDPSession();await cdp.send('Browser.setDownloadBehavior',{behavior:'allow',downloadPath:directory,browserContextId:context.id,eventsEnabled:true});const completed=new Promise((resolve,reject)=>cdp.on('Browser.downloadProgress',e=>{if(e.state==='completed')resolve();if(e.state==='canceled')reject(Error('inspection-download-canceled'))}));await page.keyboard.press('Enter');await completed;await cdp.detach();check(tag+'-real-download-exact',fs.readFileSync(path.join(directory,'paea-inspection.json'),'utf8')===raw);
  value.context.sources=[];const empty=path.join(directory,'empty-sources.json');fs.writeFileSync(empty,encode(value));await(await page.$('#paea-document')).uploadFile(empty);await submitInspection(page,200);
  check(tag+'-explicit-empty-sources',await page.$eval('#paea-source-empty',(e,l)=>!e.hidden&&e.textContent===(l==='en'?'No declared sources.':'Aucune source déclarée.'),lang)&&await page.$eval('#paea-sources',e=>e.children.length===0));
  await(await page.$('#paea-document')).uploadFile(file);check(tag+'-refusal-control',(await submitInspection(page,200))===raw);
  const invalid=path.join(directory,'invalid.json');fs.writeFileSync(invalid,'INSPECTION_PRIVATE_BROWSER_CANARY');await(await page.$('#paea-document')).uploadFile(invalid);const refused=JSON.parse(await submitInspection(page,400));
  check(tag+'-refusal-clears-all',refused.failure.code==='protection-document-invalid'&&await page.$eval('#paea-inspected-content',e=>e.hidden)&&await page.$eval('#paea-terms',e=>e.textContent==='')&&await page.$eval('#paea-canonical',e=>e.textContent==='')&&await page.$eval('#paea-download',e=>e.disabled));
  await(await page.$('#paea-document')).uploadFile(file);check(tag+'-native-recovery',(await submitInspection(page,200))===raw);
  check(tag+'-no-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:path.join(directory,'result.png'),fullPage:true});
 }
 inspectionCases.push(label);
}
/** Client-only corruptions follow actual native controls. Canonical mutations
 * retain original numeric text and recompute their own hash so the targeted
 * shape/type failure cannot be masked by a shared hash failure. Real BOM/native
 * and local invalid UTF8/unsupported-number cases remain separately attributed. */
async function inspectionClientFaults(page,label){
 const directory=path.resolve(output,label+'-inspection-payment.settle'),file=path.join(directory,'document.json'),good=fs.readFileSync(path.join(directory,'native-report.json'),'utf8');await(await page.$('#paea-document')).uploadFile(file);
 let replacement=null;await page.setRequestInterception(true);const intercept=r=>{if(new URL(r.url()).pathname==='/api/v1/protection/inspect'&&r.method()==='POST'&&replacement!==null)r.respond({status:200,contentType:'application/json',body:replacement}).catch(e=>browserFailure({kind:'inspection-intercept',message:e.message}));else r.continue().catch(e=>browserFailure({kind:'inspection-continue',message:e.message}))};page.on('request',intercept);
 const mutate=fn=>{const r=JSON.parse(good);fn(r);return JSON.stringify(r)},canonical=fn=>mutate(r=>{const before=r.canonical;r.canonical=fn(before);assert.notEqual(r.canonical,before);r.document.action_digest=sha(r.canonical)});
 for(const [name,raw]of [
  ['input-hash',mutate(r=>r.document.document_sha256='0'.repeat(64))],['canonical-hash',mutate(r=>r.canonical+=' ')],['approval',mutate(r=>r.approval_validity='established')],['extra',mutate(r=>r.extra=true)],['identity-extra',mutate(r=>r.document.extra=true)],
  ['canonical-extra',canonical(text=>'{"extra":true,'+text.slice(1))],['revision-float',canonical(text=>text.replace(/("mandate_revision":)[0-9]+/,(all,prefix)=>prefix+'1.5'))],['revision-string',canonical(text=>text.replace(/("mandate_revision":)[0-9]+/,'$1"1"'))],['unknown-kind',canonical(text=>text.replace('"kind":"payment.settle"','"kind":"unknown"'))],['duplicate-labels',canonical(text=>text.replace(/"confidentiality_labels":\[[^\]]*\]/,'"confidentiality_labels":["duplicate","duplicate"]'))],['malformed','{']]){
  replacement=null;check('inspection-client-'+name+'-native-control',(await submitInspection(page,200))===good);replacement=raw;fs.writeFileSync(path.join(directory,'synthetic-'+name+'.json'),raw);
  const pending=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/inspect'&&r.request().method()==='POST');await page.click('#paea-check');assert.equal((await pending).status(),200);await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
  check('inspection-client-rejects-'+name,await page.$eval('#paea-status',e=>e.dataset.state==='error')&&await page.$eval('#paea-inspected-content',e=>e.hidden)&&await page.$eval('#paea-download',e=>e.disabled));
 }
 replacement=null;check('inspection-client-native-recovery',(await submitInspection(page,200))===good);page.off('request',intercept);await page.setRequestInterception(false);
 await page.evaluate(()=>{globalThis.inspectionOriginalParse=JSON.parse;JSON.parse=(text,reviver)=>globalThis.inspectionOriginalParse(text,reviver?function(k,v){return reviver.call(this,k,v,undefined)}:undefined)});
 const pending=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/inspect'&&r.request().method()==='POST');await page.click('#paea-check');assert.equal((await pending).status(),200);await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);check('inspection-unsupported-number-reader-refuses',await page.$eval('#paea-status',e=>e.dataset.state==='error'&&e.textContent.includes('CLI'))&&await page.$eval('#paea-download',e=>e.disabled));await page.evaluate(()=>{JSON.parse=globalThis.inspectionOriginalParse;delete globalThis.inspectionOriginalParse});check('inspection-number-reader-native-recovery',(await submitInspection(page,200))===good);
 const invalid=path.join(directory,'invalid-utf8.json');fs.writeFileSync(invalid,Buffer.from([0xc3,0x28]));let sends=0;const count=r=>{if(new URL(r.url()).pathname==='/api/v1/protection/inspect'&&r.method()==='POST')sends++};page.on('request',count);await(await page.$('#paea-document')).uploadFile(invalid);await page.click('#paea-check');await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);check('inspection-invalid-utf8-never-sent',sends===0&&await page.$eval('#paea-status',e=>e.dataset.state==='error'));page.off('request',count);await(await page.$('#paea-document')).uploadFile(file);check('inspection-utf8-native-recovery',(await submitInspection(page,200))===good);
 const bom=path.join(directory,'bom.json');fs.writeFileSync(bom,Buffer.concat([Buffer.from([0xef,0xbb,0xbf]),fs.readFileSync(file)]));await(await page.$('#paea-document')).uploadFile(bom);const request=page.waitForRequest(r=>new URL(r.url()).pathname==='/api/v1/protection/inspect'&&r.method()==='POST');const refused=await submitInspection(page,400);check('inspection-bom-preserved-and-native-refused',JSON.parse((await request).postData()).document.startsWith('\ufeff')&&JSON.parse(refused).failure.code==='protection-document-invalid');await(await page.$('#paea-document')).uploadFile(file);check('inspection-bom-native-recovery',(await submitInspection(page,200))===good);
}

/** Submit two original files to the real approval correspondence route. A valid
 * mismatch is200/successful data validation, invalid data400/error. Completion
 * checks actual response plus idle DOM without watchdog or automatic retry. */
async function submitApproval(page,status){
 expectRefusal=status===400;const pending=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/check-approval'&&r.request().method()==='POST');
 await page.click('#paea-check');const response=await pending;assert.equal(response.status(),status);const raw=await response.text();await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);assert.equal(await page.$eval('#paea-status',e=>e.dataset.state),status===200?'success':'error');expectRefusal=false;return raw;
}
/** Observe every animation frame of the primary control’s foreground/background
 * transition, without a sleep/deadline. Record actual colors/theme; return only
 * when those CSS transitions end. All measured frames must meet normative AA,
 * including a real theme toggle. No animation is disabled by the test. */
async function primaryContrastFrames(page,name){
 const samples=await page.$eval('#paea-check',e=>new Promise(resolve=>{const rows=[];function frame(){const s=getComputedStyle(e),transitions=e.getAnimations().filter(a=>['color','background-color'].includes(a.transitionProperty));rows.push({color:s.color,background:s.backgroundColor,theme:document.documentElement.dataset.theme,transitions:transitions.length});if(transitions.some(a=>a.playState==='running'))requestAnimationFrame(frame);else resolve(rows)}frame()}));
 const lum=color=>color.match(/[\d.]+/g).slice(0,3).map(Number).map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((v,c,i)=>v+c*[.2126,.7152,.0722][i],0);
 for(const row of samples){const values=[lum(row.color),lum(row.background)];row.contrast=(Math.max(...values)+.05)/(Math.min(...values)+.05)}primaryContrastSamples.push({name,samples});return samples;
}
/** Compare both domains through menu, two files, actual CLI/API and real download.
 * Max revisions remain original integer text; match/mismatch/changed attempt and
 * unverified approver are distinct. Invalid data clears stale export and recovers;
 * all writes/screenshots stay in the owned native fixture, never a live Root. */
async function approvalCase(page,context,label,lang,theme){
 await page.goto('http://'+address+'/paea.html?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
 await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click('a[href^="/paea-approval.html"]')]);await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 check(label+'-approval-menu-title',await page.evaluate((l,t)=>location.pathname==='/paea-approval.html'&&document.documentElement.lang===l&&document.documentElement.dataset.theme===t&&document.querySelector('h1').textContent===(l==='en'?'Supplied approval correspondence':'Correspondance d’un accord'),lang,theme));
 check(label+'-approval-primary-translated',await page.$eval('#paea-check',(e,l)=>e.textContent.trim()===(l==='en'?'Compare approval and action':'Rapprocher l’accord et l’action'),lang));
 check(label+'-approval-empty-limits',await page.$eval('#paea-inputs',e=>Array.from(e.querySelectorAll('input')).every(f=>f.files.length===0))&&await page.$eval('#paea-limits',(e,l)=>e.textContent.includes(l==='en'?'validates no approval':'ne valide aucun accord'),lang)&&await page.$eval('#paea-download',e=>e.disabled));
 const embedded=await page.evaluate(async()=>Array.from(new Uint8Array(await(await fetch('/paea-approval.html')).arrayBuffer())));check(label+'-approval-embedded-exact',sha(Buffer.from(embedded))===sha(fs.readFileSync('web/paea-approval.html')));
 await page.focus('#paea-format-help summary');await page.keyboard.press('Enter');check(label+'-approval-help-keyboard',await page.$eval('#paea-format-help',(e,l)=>e.open&&e.textContent.includes(l==='en'?'revocation':'révocation'),lang));await page.keyboard.press('Enter');
 const colors=await primaryContrastFrames(page,label+'-initial');check(label+'-approval-primary-contrast-AA',colors.every(s=>s.contrast>=4.5));
 for(const expectedTheme of [theme==='etat'?'sombre':'etat',theme]){await page.click('#paea-theme');const samples=await primaryContrastFrames(page,label+'-'+expectedTheme);check(label+'-approval-theme-'+expectedTheme+'-all-frames-AA',samples.length>0&&samples.every(s=>s.theme===expectedTheme&&s.contrast>=4.5&&s.transitions===0))}
 const encode=value=>JSON.stringify(value).replace(/"(mandate_revision|policy_revision)":1([,}])/g,'"$1":9223372036854775807$2');
 for(const kind of ['payment.settle','repository.promote']){
  const tag=label+'-approval-'+kind,directory=path.resolve(output,tag);fs.mkdirSync(directory);const vector=vectors.find(v=>v.proposal.action.kind===kind),value=structuredClone({proposal:vector.proposal,context:vector.context});value.context.mandate_revision=1;value.context.policy_revision=1;
  const file=path.join(directory,'document.json'),approval=path.join(directory,'approval.json'),original=encode(value);fs.writeFileSync(file,original);
  const canonical=JSON.parse((await command(['protection','canonicalize','--input',file,'--json'],tag+'-canonical',directory)).toString());
  const binding={schema_version:1,action_digest:canonical.action_digest,policy_digest:value.context.policy_digest,approver_id:'fictional-approver'},signed=JSON.stringify(binding);fs.writeFileSync(approval,signed);
  await(await page.$('#paea-document')).uploadFile(file);await(await page.$('#paea-approval')).uploadFile(approval);const raw=await submitApproval(page,200),report=JSON.parse(raw);fs.writeFileSync(path.join(directory,'native-report.json'),raw);
  const cli=(await command(['protection','check-approval',file,approval,'--json'],tag+'-cli',directory)).toString();
  check(tag+'-exact-cli-api',raw===cli&&report.binding_matches&&report.mismatched_fields.length===0&&report.document.document_sha256===sha(original)&&report.approval_sha256===sha(signed));
  check(tag+'-no-authenticity-authority-or-effect',['authority','approval_authenticity','approval_validity','effect'].every(k=>report[k]==='not-established')&&await page.$eval('#paea-summary',(e,l)=>e.textContent===(l==='en'?'Hashes match. No approval validated.':'Empreintes correspondantes. Aucun accord validé.'),lang));
  await page.focus('#paea-check');await page.keyboard.press('Tab');check(tag+'-download-keyboard',await page.evaluate(()=>document.activeElement.id==='paea-download'));
  const cdp=await page.createCDPSession();await cdp.send('Browser.setDownloadBehavior',{behavior:'allow',downloadPath:directory,browserContextId:context.id,eventsEnabled:true});const complete=new Promise((resolve,reject)=>cdp.on('Browser.downloadProgress',e=>{if(e.state==='completed')resolve();if(e.state==='canceled')reject(Error('approval-download-canceled'))}));await page.keyboard.press('Enter');await complete;await cdp.detach();check(tag+'-real-download-exact',fs.readFileSync(path.join(directory,'paea-approval-report.json'),'utf8')===raw);
  for(const variant of ['policy','terms','attempt','approver']){
   const changed=structuredClone(value),other=structuredClone(binding);
   if(variant==='policy')other.policy_digest='f'.repeat(64);
   if(variant==='terms'){if(kind==='payment.settle')changed.proposal.action.amount_minor='420001';else changed.proposal.action.candidate_oid='e'.repeat(changed.proposal.action.candidate_oid.length)}
   if(variant==='attempt')changed.proposal.request_id='approval-new-attempt';if(variant==='approver')other.approver_id='unverified-alternative';
   const doc=path.join(directory,variant+'-document.json'),app=path.join(directory,variant+'-approval.json');fs.writeFileSync(doc,encode(changed));fs.writeFileSync(app,JSON.stringify(other));await(await page.$('#paea-document')).uploadFile(doc);await(await page.$('#paea-approval')).uploadFile(app);
   check(tag+'-'+variant+'-edit-clears',await page.$eval('#paea-summary',e=>e.textContent==='')&&await page.$eval('#paea-download',e=>e.disabled));
   const observedRaw=await submitApproval(page,200),r=JSON.parse(observedRaw);const expectedFields=variant==='policy'?['policy_digest']:variant==='terms'?['action_digest']:[];
   check(tag+'-'+variant+'-explicit-data-result',r.binding_matches===(expectedFields.length===0)&&JSON.stringify(r.mismatched_fields)===JSON.stringify(expectedFields)&&r.approval_validity==='not-established'&&r.approval_authenticity==='not-established'&&await page.$eval('#paea-download',e=>!e.disabled));
   check(tag+'-'+variant+'-native-cli-parity',observedRaw===(await command(['protection','check-approval',doc,app,'--json'],tag+'-'+variant+'-cli',directory)).toString());
   if(expectedFields.length)check(tag+'-'+variant+'-visible-negative',await page.$eval('#paea-summary',(e,l)=>e.textContent===(l==='en'?'Hashes differ. No approval validated.':'Empreintes différentes. Aucun accord validé.'),lang));
  }
  await(await page.$('#paea-document')).uploadFile(file);await(await page.$('#paea-approval')).uploadFile(approval);check(tag+'-refusal-control',(await submitApproval(page,200))===raw);
  const invalid=path.join(directory,'invalid.json');fs.writeFileSync(invalid,'APPROVAL_PRIVATE_BROWSER_CANARY');await(await page.$('#paea-approval')).uploadFile(invalid);const refused=JSON.parse(await submitApproval(page,400));
  check(tag+'-invalid-clears-report-export',refused.failure.code==='protection-document-invalid'&&await page.$eval('#paea-identities',e=>e.children.length===0)&&await page.$eval('#paea-summary',e=>e.textContent==='')&&await page.$eval('#paea-report',e=>e.textContent==='')&&await page.$eval('#paea-download',e=>e.disabled));
  await(await page.$('#paea-approval')).uploadFile(approval);check(tag+'-native-recovery',(await submitApproval(page,200))===raw);check(tag+'-no-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:path.join(directory,'result.png'),fullPage:true});
 }
 approvalCases.push(label);
}
/** Browser-only bad responses and loading hold follow actual native controls;
 * malformed UTF8 never leaves the browser, original BOM is refused by native.
 * All faults restore actual input/transport, with exact real recovery. */
async function approvalClientFaults(page,label){
 const directory=path.resolve(output,label+'-approval-payment.settle'),doc=path.join(directory,'document.json'),app=path.join(directory,'approval.json'),good=fs.readFileSync(path.join(directory,'native-report.json'),'utf8');await(await page.$('#paea-document')).uploadFile(doc);await(await page.$('#paea-approval')).uploadFile(app);
 let replacement=null,hold=null,posts=0;await page.setRequestInterception(true);const intercept=r=>{if(new URL(r.url()).pathname==='/api/v1/protection/check-approval'&&r.method()==='POST'){posts++;if(replacement==='hold'){hold=r;return}if(replacement!==null){r.respond({status:200,contentType:'application/json',body:replacement}).catch(e=>browserFailure({kind:'approval-response-injection',message:e.message}));return}}r.continue().catch(e=>browserFailure({kind:'approval-continue',message:e.message}))};page.on('request',intercept);
 const mutate=fn=>{const r=JSON.parse(good);fn(r);return JSON.stringify(r)};
 for(const [name,raw]of [['document-hash',mutate(r=>r.document.document_sha256='0'.repeat(64))],['approval-hash',mutate(r=>r.approval_sha256='0'.repeat(64))],['authority',mutate(r=>r.authority='established')],['authenticity',mutate(r=>r.approval_authenticity='established')],['validity',mutate(r=>r.approval_validity='established')],['effect',mutate(r=>r.effect='established')],['extra',mutate(r=>r.extra=true)],['unknown-field',mutate(r=>{r.binding_matches=false;r.mismatched_fields=['unknown']})],['duplicate-field',mutate(r=>{r.binding_matches=false;r.mismatched_fields=['action_digest','action_digest']})],['unordered-fields',mutate(r=>{r.binding_matches=false;r.mismatched_fields=['policy_digest','action_digest']})],['inconsistent-match',mutate(r=>r.binding_matches=false)],['wrong-type',mutate(r=>r.binding_matches='true')],['malformed','{']]){
  replacement=null;check('approval-client-'+name+'-native-control',(await submitApproval(page,200))===good);replacement=raw;fs.writeFileSync(path.join(directory,'synthetic-'+name+'.json'),raw);const pending=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/check-approval'&&r.request().method()==='POST');await page.click('#paea-check');assert.equal((await pending).status(),200);await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);check('approval-client-rejects-'+name,await page.$eval('#paea-status',e=>e.dataset.state==='error')&&await page.$eval('#paea-download',e=>e.disabled)&&await page.$eval('#paea-identities',e=>e.children.length===0));
 }
 replacement=null;check('approval-client-native-recovery',(await submitApproval(page,200))===good);
 replacement='hold';const before=posts,requested=page.waitForRequest(r=>new URL(r.url()).pathname==='/api/v1/protection/check-approval'&&r.method()==='POST');await page.click('#paea-check');await requested;assert(hold);check('approval-loading-clears-and-blocks',await page.$eval('#paea-inputs',e=>e.disabled)&&await page.$eval('#paea-download',e=>e.disabled)&&await page.$eval('#paea-report',e=>e.textContent==='')&&await page.$eval('#paea-status',e=>e.dataset.state==='loading'));await page.$eval('#paea-form',e=>e.requestSubmit());check('approval-no-duplicate-gesture',posts===before+1);
 replacement=null;const response=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v1/protection/check-approval'&&r.request().method()==='POST');await hold.continue();hold=null;assert.equal((await response).status(),200);await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);check('approval-loading-native-recovery',await page.$eval('#paea-report',(e,text)=>e.textContent===text,good));page.off('request',intercept);await page.setRequestInterception(false);
 for(const [id,file]of [['paea-document',doc],['paea-approval',app]]){
  check('approval-'+id+'-encoding-control',(await submitApproval(page,200))===good);const invalid=path.join(directory,id+'-invalid-utf8.json');fs.writeFileSync(invalid,Buffer.from([0xc3,0x28]));let sends=0;const count=r=>{if(new URL(r.url()).pathname==='/api/v1/protection/check-approval'&&r.method()==='POST')sends++};page.on('request',count);await(await page.$('#'+id)).uploadFile(invalid);await page.click('#paea-check');await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);check('approval-'+id+'-invalid-utf8-never-sent',sends===0&&await page.$eval('#paea-status',e=>e.dataset.state==='error')&&await page.$eval('#paea-download',e=>e.disabled));page.off('request',count);await(await page.$('#'+id)).uploadFile(file);check('approval-'+id+'-utf8-native-recovery',(await submitApproval(page,200))===good);
  const bom=path.join(directory,id+'-bom.json');fs.writeFileSync(bom,Buffer.concat([Buffer.from([0xef,0xbb,0xbf]),fs.readFileSync(file)]));await(await page.$('#'+id)).uploadFile(bom);const requested=page.waitForRequest(r=>new URL(r.url()).pathname==='/api/v1/protection/check-approval'&&r.method()==='POST');const denied=await submitApproval(page,400);const field=id==='paea-document'?'document':'approval';check('approval-'+id+'-bom-preserved-native-refused',JSON.parse((await requested).postData())[field].startsWith('\ufeff')&&JSON.parse(denied).failure.code==='protection-document-invalid');await(await page.$('#'+id)).uploadFile(file);check('approval-'+id+'-bom-native-recovery',(await submitApproval(page,200))===good);
 }
}

/** Download the actual private examples, then choose those exact files through
 * normal controls and run the native check. This owned fixture-only journey
 * asserts each completed download's bytes, translated keyboard disclosure,
 * expected data/limits and cleared stale output. No automatic product load,
 * response substitution, live Root, provider or effect is involved. */
async function exampleJourney(page,context,label,lang,theme,operation){
 const routes={'receipt-report':'paea.html','resolve-payment':'paea-payment.html','compare':'paea-compare.html','inspect':'paea-inspect.html','check-approval':'paea-approval.html'},tag=label+'-examples-'+operation;
 await page.goto('http://'+address+'/'+routes[operation]+'?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
 await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
 check(tag+'-explicit-empty',await page.$eval('#paea-inputs',e=>Array.from(e.querySelectorAll('input[type=file]')).every(f=>f.files.length===0))&&await page.$eval('#paea-download',e=>e.disabled));
 check(tag+'-collapsed-translated',await page.$eval('#paea-example-help',(e,l)=>!e.open&&e.querySelector('summary').textContent===(l==='en'?'Download fictional examples':'Exemples fictifs à télécharger'),lang));
 await page.focus('#paea-example-help summary');await page.keyboard.press('Enter');
 check(tag+'-keyboard-help',await page.$eval('#paea-example-help',(e,l)=>e.open&&e.textContent.includes(l==='en'?'No file is loaded':'Aucun fichier n’est chargé'),lang));
 const directory=path.resolve(output,tag);fs.mkdirSync(directory);
 const links=await page.$$eval('#paea-example-help a[download]',els=>els.map(e=>({href:new URL(e.href).pathname,name:e.download})));
 assert.equal(links.length,['resolve-payment','inspect'].includes(operation)?2:4);
 const cdp=await page.createCDPSession();await cdp.send('Browser.setDownloadBehavior',{behavior:'allow',downloadPath:directory,browserContextId:context.id,eventsEnabled:true});
 for(const link of links){
  assert.equal(link.href,'/paea-examples/v1/'+link.name);assert.equal(path.basename(link.name),link.name);
  const completed=new Promise((resolve,reject)=>{const listener=e=>{if(e.state==='completed'){cdp.off('Browser.downloadProgress',listener);resolve()}if(e.state==='canceled'){cdp.off('Browser.downloadProgress',listener);reject(Error('example-download-canceled'))}};cdp.on('Browser.downloadProgress',listener)});
  const selector='#paea-example-help a[download="'+link.name+'"]';
  if(link===links[0]){await page.focus(selector);check(tag+'-download-keyboard-focus',await page.$eval(selector,e=>document.activeElement===e&&getComputedStyle(e).outlineStyle!=='none'));await page.keyboard.press('Enter')}else await page.click(selector);await completed;
  check(tag+'-download-'+link.name,fs.readFileSync(path.join(directory,link.name)).equals(fs.readFileSync('web'+link.href)));
 }
 await cdp.detach();
 const colors=await page.$eval('#paea-example-help a',e=>[getComputedStyle(e).color,getComputedStyle(document.body).backgroundColor]);
 const luminance=color=>color.match(/[\d.]+/g).slice(0,3).map(Number).map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((v,c,i)=>v+c*[.2126,.7152,.0722][i],0),levels=colors.map(luminance);
 check(tag+'-link-contrast-AA',(Math.max(...levels)+.05)/(Math.min(...levels)+.05)>=4.5);
 check(tag+'-download-keeps-inputs-empty',await page.$eval('#paea-inputs',e=>Array.from(e.querySelectorAll('input[type=file]')).every(f=>f.files.length===0))&&await page.$eval('#paea-report',e=>e.textContent===''));
 if(operation==='check-approval'){
  for(const prefix of ['payment','git']){const doc=path.join(directory,prefix+'-document.json'),app=path.join(directory,prefix+'-approval.json');await(await page.$('#paea-document')).uploadFile(doc);await(await page.$('#paea-approval')).uploadFile(app);const r=JSON.parse(await submitApproval(page,200));check(tag+'-'+prefix+'-native-example-result',r.binding_matches&&r.document.document_sha256===sha(fs.readFileSync(doc))&&r.approval_sha256===sha(fs.readFileSync(app))&&r.approval_authenticity==='not-established'&&r.approval_validity==='not-established')}
 }else if(operation==='inspect'){
  for(const prefix of ['payment','git']){const file=path.join(directory,prefix+'-document.json');await(await page.$('#paea-document')).uploadFile(file);const r=JSON.parse(await submitInspection(page,200));check(tag+'-'+prefix+'-native-example-result',r.document.document_sha256===sha(fs.readFileSync(file))&&r.effect==='not-established'&&r.approval_validity==='not-established')}
 }else if(operation==='resolve-payment'){
  const registry=path.join(directory,'payment-registry.json'),request=path.join(directory,'payment-request.json');await fillPayment(page,registry,request);const raw=await submitPayment(page,200),report=JSON.parse(raw);
  check(tag+'-native-example-result',report.proposal.action.invoice_id==='debt-17'&&report.registry_sha256===sha(fs.readFileSync(registry))&&report.request_sha256===sha(fs.readFileSync(request))&&report.effect==='not-established');
 }else for(const prefix of ['payment','git']){
  const document=path.join(directory,prefix+'-document.json');
  if(operation==='receipt-report'){
   const receipt=path.join(directory,prefix+'-receipt.json');await fill(page,document,receipt);await page.$eval('#paea-sink',e=>{e.value='sink-paea-example';e.dispatchEvent(new Event('input',{bubbles:true}))});
   const report=await submit(page,200);check(tag+'-'+prefix+'-native-example-result',report.document.document_sha256===sha(fs.readFileSync(document))&&report.receipt_sha256===sha(fs.readFileSync(receipt))&&report.proposed_state==='COMMITTED'&&report.effect==='not-established'&&report.receipt_authenticity==='not-established');
  }else{
   const after=path.join(directory,prefix+'-after.json');await fillComparison(page,document,after);const report=JSON.parse(await submitComparison(page,200)),c=report.comparison;
   check(tag+'-'+prefix+'-native-example-result',c.before.document_sha256===sha(fs.readFileSync(document))&&c.after.document_sha256===sha(fs.readFileSync(after))&&c.canonical_content_changed&&!c.intent_changed&&!c.transport_request_changed&&c.approval_validity==='not-established');
  }
 }
 check(tag+'-no-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
 await page.screenshot({path:path.join(directory,'result.png'),fullPage:true});exampleCases.push(tag);
}

(async()=>{let failed;
 try{await Promise.race([(async()=>{
  await start();check('actual-default-profile-executable',true);
  browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.resolve(output,'browser')});
  const matrix=scope==='payment-encoding'?[['fr','etat',1440]]:['fr','en'].flatMap(lang=>['etat','sombre'].flatMap(theme=>[1440,390,320].map(width=>[lang,theme,width])));
  for(const [lang,theme,width]of matrix){
   const label=lang+'-'+theme+'-'+width,context=await browser.createBrowserContext(),page=await context.newPage();
   page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width,height:1000});
   await context.setCookie({name:'swarm_session_'+sha(address).slice(0,12),value:token,url:'http://'+address,httpOnly:true,sameSite:'Strict'});
   page.on('pageerror',e=>browserFailure({kind:'pageerror',message:e.message}));
   page.on('requestfailed',r=>browserFailure({kind:'requestfailed',path:new URL(r.url()).pathname}));
   page.on('response',r=>{if(r.status()>=400){const p=new URL(r.url()).pathname;responses.push({path:p,status:r.status()});if(!(expectRefusal&&['/api/v1/protection/receipt-report','/api/v1/protection/resolve-payment','/api/v1/protection/compare','/api/v1/protection/inspect','/api/v1/protection/check-approval'].includes(p)&&r.status()===400))browserFailure({kind:'http',path:p,status:r.status()})}});
   page.on('console',m=>{if(m.type()==='error'){const p=m.location().url?new URL(m.location().url).pathname:'';if(!(expectRefusal&&['/api/v1/protection/receipt-report','/api/v1/protection/resolve-payment','/api/v1/protection/compare','/api/v1/protection/inspect','/api/v1/protection/check-approval'].includes(p)&&m.text().includes('400')))browserFailure({kind:'console',message:m.text()})}});
   if(scope==='payment-encoding'){
    await paymentCase(page,context,label,lang,theme);
    const script=await page.evaluate(async()=>{const r=await fetch('/paea.js');if(!r.ok)throw Error('PAEA script unavailable');return Array.from(new Uint8Array(await r.arrayBuffer()))});
    check('payment-encoding-exact-current-script',sha(Buffer.from(script))===sha(fs.readFileSync('web/paea.js')));
    await paymentBomFiles(page,label);await context.close();continue;
   }
   await page.goto('http://'+address+'/paea.html?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
   await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);
   check(label+'-explicit-empty-inputs',await page.$eval('#paea-state',e=>e.value==='')&&await page.$eval('#paea-download',e=>e.disabled));
   check(label+'-theme-language',await page.evaluate((l,t)=>document.documentElement.lang===l&&document.documentElement.dataset.theme===t,lang,theme));
   check(label+'-translated-primary',await page.$eval('#paea-check',(e,l)=>e.textContent.trim()===(l==='en'?'Check correspondence':'Vérifier la correspondance'),lang));
   check(label+'-independent-cockpit-link',await page.$eval('nav a',e=>e.getAttribute('href')==='/'));
   const embedded=await page.evaluate(async()=>{const result={};for(const p of ['/paea.html','/paea.js','/paea.css','/i18n-en.js','/i18n.js','/swarm-design.css','/wattson_themes.css','/swarm-logo.png']){const r=await fetch(p);if(!r.ok)throw Error('asset unavailable');result[p]=Array.from(new Uint8Array(await r.arrayBuffer()))}return result});
   check(label+'-actual-embedded-assets',Object.entries(embedded).every(([p,bytes])=>sha(Buffer.from(bytes))===sha(fs.readFileSync('web'+p))));
   for(const kind of ['payment.settle','repository.promote']){
    const v=vectors.find(x=>x.proposal.action.kind===kind),raw=JSON.stringify({proposal:v.proposal,context:v.context});
    const receipt=JSON.stringify({schema_version:1,action_digest:v.action_digest,intent_id:v.intent_id,sink_id:'sink-native-fixture',outcome:'COMMITTED',observation_digest:'d'.repeat(64)});
    const name=label+'-'+kind,doc=path.resolve(output,name+'-document.json'),rec=path.resolve(output,name+'-receipt.json');fs.writeFileSync(doc,raw);fs.writeFileSync(rec,receipt);
    await fill(page,doc,rec);const report=await submit(page,200);
    check(name+'-real-native-report',report.schema_version===2&&report.document.document_sha256===sha(raw)&&report.receipt_sha256===sha(receipt)&&report.document.action_digest===v.action_digest&&report.document.intent_id===v.intent_id);
    check(name+'-no-authority-or-effect',report.authority==='not-established'&&report.receipt_authenticity==='not-established'&&report.effect==='not-established');
    check(name+'-rendered-download',await page.$eval('#paea-report',(e,r)=>JSON.stringify(JSON.parse(e.textContent))===JSON.stringify(r),report));
    await page.$eval('#paea-sink',e=>{e.value='different-sink';e.dispatchEvent(new Event('input',{bubbles:true}))});
    check(name+'-edit-clears-old-report',await page.$eval('#paea-download',e=>e.disabled));
    const refusal=await submit(page,400);check(name+'-actual-refusal',!!refusal.failure?.code&&await page.$eval('#paea-report',e=>e.textContent===''));
    await page.$eval('#paea-sink',e=>{e.value='sink-native-fixture';e.dispatchEvent(new Event('input',{bubbles:true}))});
    check(name+'-recovery',(await submit(page,200)).document.action_digest===v.action_digest);
   }
   if(faultMode==='script'){
    await page.setCacheEnabled(false);await page.setRequestInterception(true);
    page.on('request',r=>{if(new URL(r.url()).pathname==='/paea.js'){console.log('INJECTED_NATIVE_PAEA_SCRIPT_FAILURE');r.abort('failed').catch(e=>browserFailure({kind:'script-injection',message:e.message}))}else r.continue().catch(e=>browserFailure({kind:'script-injection-continue',message:e.message}))});
    await page.goto('http://'+address+'/paea-payment.html?'+new URLSearchParams({lang,theme}),{waitUntil:'domcontentloaded'});
    await page.waitForFunction(()=>!document.getElementById('paea-check').disabled);throw Error('script-fault-not-observed');
   }
   check(label+'-no-horizontal-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   await page.screenshot({path:path.join(output,label+'.png'),fullPage:true});cases.push(label);
   if(width===1440){
    await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),page.click('nav a[href="/"]')]);
    await page.waitForSelector('a[href="/paea.html"]');check(label+'-native-menu-entry',true);
    const version=await page.evaluate(async()=>{const r=await fetch('/api/v1/native-version');if(!r.ok)throw Error('version unavailable');return r.json()});
    if(!await page.$eval('.rail-settings',e=>e.open))await page.click('.rail-settings > summary');
    await page.click('#version-open');await page.waitForSelector('#version-dialog[open]');
    await page.waitForFunction(v=>document.getElementById('version-dialog').textContent.includes(v),{},version.binary.version);
    check(label+'-version-and-PAEA-notes',await page.$eval('#version-dialog',(e,v)=>e.textContent.includes(v)&&e.textContent.includes('PAEA'),version.binary.version));
    check(label+'-actual-commit-link',await page.$$eval('#version-dialog a',(els,c)=>els.some(e=>e.href.endsWith('/commit/'+c)),version.binary.commit));
    await page.screenshot({path:path.join(output,label+'-version.png'),fullPage:true});
    await page.evaluate(()=>{globalThis.closedVersion=new Promise(resolve=>document.getElementById('version-dialog').addEventListener('close',resolve,{once:true}))});
    await page.keyboard.press('Escape');await page.evaluate(()=>globalThis.closedVersion);
    check(label+'-version-focus-restored',await page.evaluate(()=>document.activeElement.id==='version-open'));
   }
   await paymentCase(page,context,label,lang,theme);if(label==='fr-etat-1440')await paymentClientFaults(page,label);
   await comparisonCase(page,context,label,lang,theme);if(label==='fr-etat-1440')await comparisonClientFaults(page,label);
   await inspectionCase(page,context,label,lang,theme);if(label==='fr-etat-1440')await inspectionClientFaults(page,label);
   await approvalCase(page,context,label,lang,theme);if(label==='fr-etat-1440')await approvalClientFaults(page,label);
   for(const operation of ['receipt-report','resolve-payment','compare','inspect','check-approval'])await exampleJourney(page,context,label,lang,theme,operation);await context.close();
  }
  if(scope==='payment-encoding'){check('one-native-payment-encoding-case',cases.length===0&&paymentCases.length===1);check('exactly-three-native-encoding-refusals',responses.length===3)}
  else{check('all-twelve-native-browser-cases',cases.length===12);check('all-twelve-payment-browser-cases',paymentCases.length===12);check('all-twelve-comparison-browser-cases',comparisonCases.length===12);check('all-twelve-inspection-browser-cases',inspectionCases.length===12);check('all-twelve-approval-browser-cases',approvalCases.length===12);check('exactly-one-hundred-thirteen-native-refusals',responses.length===113);check('all-sixty-example-journeys',exampleCases.length===60)}
  check('no-unexpected-browser-diagnostics',errors.length===0);check('binary-stable',sha(fs.readFileSync(binary))===expected);
 })(),observedFailure]);}catch(e){failed=e;console.error(e.stack)}
 finally{
  try{if(browser)await browser.close()}catch(e){errors.push({kind:'browser-close',message:e.message});failed??=e}
  if(server){stopping=true;server.kill('SIGINT');const stopped=await closed;if(!failed&&stopped.code!==0)failed=Error('owned native fixture did not stop cleanly')}
  fs.writeFileSync(path.join(root,'server.stderr'),Buffer.concat(serverErrors),{mode:0o600});
  fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({scope:'real default-profile binary and embedded native UI, isolated fixture; no protected effect/independent acceptance',test_scope:scope,binary,binary_sha256:expected,cases,paymentCases,comparisonCases,inspectionCases,approvalCases,primaryContrastSamples,exampleCases,checks,errors,responses,success:!failed},null,2));
 }
 if(failed)process.exitCode=1;else console.log('PAEA_NATIVE_UI_PASS '+checks.length+' assertions');
})();
