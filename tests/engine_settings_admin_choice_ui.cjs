'use strict';
// Focused real-browser regression for the bounded scalar 'choice' catalogue unit
// in the shared Admin UI. Serves the actual frontend bundle over HTTP and drives
// the documented mount(describe/show/preview/apply) transport seam with an
// in-page stub that records every call and owns a mutable document. The backend
// ships independently, so no native CLI, live server, provider or paid call is
// involved; only stock puppeteer and the real DOM are used (no invented APIs).
// Catalogue labels go through the supplied translate map to prove tr() use;
// every other UI key intentionally stays French.
const fs=require('node:fs'),path=require('node:path'),http=require('node:http'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const {waitForSettingsState}=require('./helpers/engine_settings_admin_state.cjs');
const repo=path.resolve(__dirname,'..');
const adminJs=fs.readFileSync(path.join(repo,'frontend/engine-settings/admin.js'),'utf8');
const adminCss=fs.readFileSync(path.join(repo,'frontend/engine-settings/admin.css'),'utf8');
const choiceChoices=[{value:'0',label:'Manuel'},{value:'1',label:'Assisté'},{value:'2',label:'Autonome'}];
const en={
 'Manuel':'Manual',
 'Assisté':'Assisted',
 'Autonome':'Autonomous',
 'Non configuré dans cette version':'Not configured in this version',
 'Défaut initial : ':'Initial default: ',
 'Révision ':'Revision ',
 'Ce réglage choisit seulement le mode initial du poste de pilotage. Le mode enregistré dans la mission reste prioritaire ; le consentement au lancement de la mission reste obligatoire.':'Cockpit autonomy picks the initial mode only. The mode saved in the mission wins; explicit launch consent stays mandatory.'
};
const baseCatalog=()=>({
 schema_version:6,document_schema_version:48,supported_document_versions:[1,48],
 fields:[
  {name:'task_phase_gates',label:'Contrôles de phases',unit:'phases',introduced_schema_version:1,help:'Critères contrôlés par phase.',choices:[{value:'plan',label:'Planification'},{value:'run',label:'Exécution'}]},
  {name:'task_max_attempts',label:'Tentatives par tâche',unit:'count',introduced_schema_version:1,help:'Essais maximum par tâche.'},
  {name:'cockpit_default_autonomy',label:'Autonomie du poste de pilotage',unit:'choice',nullable:false,introduced_schema_version:48,help:'Mode du poste de pilotage.',choices:choiceChoices.map(c=>({...c}))}
 ],
 defaults:{task_phase_gates:['plan'],task_max_attempts:'3',cockpit_default_autonomy:'2'},
 network_transport:{max_header_fields:'100',minimum_connection_buffer_bytes:'1024',max_header_bytes:'1048576',native_body_max_bytes:'1048576',max_semaphore_permits:'64'}
});
const historicalDocument=()=>({schema_version:1,revision:'1',history:[],values:{task_phase_gates:['plan'],task_max_attempts:'3'}});
const select='select[data-setting="cockpit_default_autonomy"]';
const cases=[];let server,browser,page;const pageErrors=[];
async function mountFixture(fixture){
 await page.goto('http://127.0.0.1:'+server.address().port+'/',{waitUntil:'domcontentloaded'});
 await page.evaluate(fixture=>{
  window.__calls=[];
  window.__state={document:structuredClone(fixture.document)};
  const state=window.__state;
  const transport={
   describe:()=>structuredClone(fixture.catalog),
   show:()=>structuredClone(state.document),
   preview:request=>{
    window.__calls.push({op:'preview',request:structuredClone(request)});
    return {current_revision:state.document.revision,preview_token:'token-'+state.document.revision,values:structuredClone(request.values)};
   },
   apply:request=>{
    window.__calls.push({op:'apply',request:structuredClone(request)});
    const revision=(BigInt(request.expected_revision)+1n).toString();
    const entry={revision,at:'2026-01-01T00:00:00Z',actor:'fixture',change:{schema_version:request.schema_version,event_id:request.event_id,expected_revision:request.expected_revision,reason:request.reason,preview_token:request.preview_token,values:structuredClone(request.values)}};
    const document={schema_version:48,revision,history:[...state.document.history,entry],values:structuredClone(request.values)};
    state.document=document;
    return {replayed:false,document:structuredClone(document)};
   }
  };
  window.component=SwarmEngineSettingsAdmin.mount(document.querySelector('#admin'),transport,{translate:source=>Object.hasOwn(fixture.en,source)?fixture.en[source]:source});
 },fixture);
 await page.waitForFunction(()=>!document.querySelector('[data-action="edit"]').disabled||document.querySelector('.engine-settings-admin [role="status"]').classList.contains('alert'));
}
async function assertLoadRefusal(fixture,why){
 await mountFixture(fixture);
 assert.equal(await page.$eval('.engine-settings-admin [role="status"]',e=>e.classList.contains('alert')),true,why);
 assert.equal(await page.$eval('.engine-settings-admin [role="status"]',e=>e.textContent),'Format des réglages incompatible.');
 assert.equal(await page.$eval('[data-action="edit"]',e=>e.disabled),true,why);
}
(async()=>{try{
 server=http.createServer((req,res)=>{
  if(req.url==='/'){res.setHeader('Content-Type','text/html; charset=utf-8');res.end('<!doctype html><html lang="fr"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><link rel="stylesheet" href="/admin.css"><main id="admin"></main><script src="/admin.js"></script>')}
  else if(req.url==='/admin.js'){res.setHeader('Content-Type','text/javascript; charset=utf-8');res.end(adminJs)}
  else if(req.url==='/admin.css'){res.setHeader('Content-Type','text/css; charset=utf-8');res.end(adminCss)}
  else if(req.url==='/favicon.ico'){res.statusCode=204;res.end()}else{res.statusCode=404;res.end('Unknown fixture asset')}
 });
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
 page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 await page.setViewport({width:1440,height:1000});
 page.on('pageerror',e=>pageErrors.push(e.message));
 page.on('console',m=>{if(m.type()==='error')pageErrors.push(m.text())});
 page.on('requestfailed',r=>pageErrors.push('request failed: '+r.url()));
 // Historical document: explicit unconfigured display, translated default, native
 // select with exact options, then explicit preview/apply adoption of the value.
 await mountFixture({catalog:baseCatalog(),document:historicalDocument(),en});
 assert.equal(await page.$eval('.engine-settings-admin [role="status"]',e=>e.classList.contains('alert')),false,'choice fixture load refused');
 assert.equal(await page.$eval('.engine-settings-admin [role="status"]',e=>e.textContent),'Revision 1');
 const planningRead=await page.$eval('[data-settings-summary-group="planning"]',e=>e.textContent);
 assert(planningRead.includes('Not configured in this version'),'historical choice field must stay explicitly unconfigured');
 assert(planningRead.includes('Initial default: Autonomous'),'default label must resolve by exact membership');
 assert(!planningRead.includes('Autonome'),'no raw value may leak as a label');
 await page.click('[data-action="edit"]');
 assert.equal(await page.$eval(select,e=>e.tagName),'SELECT');
 assert.equal(await page.$eval(select,e=>e.required),true);
 assert.equal(await page.$eval(select,e=>e.value),'2');
 assert.deepEqual(await page.$$eval(select+' option',os=>os.map(o=>[o.value,o.textContent])),[['0','Manual'],['1','Assisted'],['2','Autonomous']]);
 assert(await page.evaluate(()=>{const s=document.querySelector('select[data-setting="cockpit_default_autonomy"]');const label=document.querySelector('label[for="'+CSS.escape(s.id)+'"]');return !!label&&label.contains(s)}),'select must keep its label association');
 assert.deepEqual(await page.$$eval('[data-settings-group]',gs=>gs.map(g=>g.dataset.settingsGroup)),['tasks','planning']);
 const modalText=await page.$eval('dialog[data-engine-settings]',e=>e.textContent);
 assert(modalText.includes('Cockpit autonomy picks the initial mode only'),'cockpit help note must be visible');
 assert.equal(await page.$eval('fieldset[data-setting="task_phase_gates"]',e=>e.tagName),'FIELDSET');
 assert.deepEqual(await page.$$eval('fieldset[data-setting="task_phase_gates"] input[type="checkbox"]',cs=>cs.map(c=>[c.value,c.checked])),[['plan',true],['run',false]]);
 assert.equal(await page.$eval('[data-setting="task_max_attempts"]',e=>e.tagName),'INPUT');
 assert.deepEqual(await page.evaluate(()=>({schema:window.__state.document.schema_version,revision:window.__state.document.revision,applies:window.__calls.filter(c=>c.op==='apply').length})),{schema:1,revision:'1',applies:0});
 await page.click('[data-settings-group="planning"] > summary');
 await page.select(select,'0');
 await page.$eval('[data-setting-reason]',e=>{e.value='ADM-CHOICE-01 explicit initial autonomy';e.dispatchEvent(new Event('input',{bubbles:true}))});
 await page.click('[data-action="confirm"]');
 await waitForSettingsState(page,'preview');
 await page.waitForFunction(()=>!document.querySelector('[data-action="confirm"]').disabled);
 const previewText=await page.$eval('[data-setting-preview]',e=>e.textContent);
 assert(previewText.includes('Manual'),'preview must show the selected catalog label');
 const previewRequest=await page.evaluate(()=>window.__calls.filter(c=>c.op==='preview').at(-1).request);
 assert.equal(typeof previewRequest.values.cockpit_default_autonomy,'string');
 assert.equal(previewRequest.values.cockpit_default_autonomy,'0');
 assert.deepEqual(previewRequest.values.task_phase_gates,['plan']);
 assert.equal(previewRequest.values.task_max_attempts,'3');
 assert.equal(previewRequest.expected_revision,'1');
 assert.equal(await page.evaluate(()=>window.__calls.filter(c=>c.op==='apply').length),0,'preview must not save');
 await page.click('[data-action="confirm"]');
 await waitForSettingsState(page,'saved');
 assert.equal(await page.$eval('.engine-settings-admin [role="status"]',e=>e.textContent),'Revision 2');
 const saved=await page.evaluate(()=>({applies:window.__calls.filter(c=>c.op==='apply').length,document:window.__state.document}));
 assert.equal(saved.applies,1);
 assert.equal(saved.document.schema_version,48);
 assert.equal(saved.document.values.cockpit_default_autonomy,'0');
 assert.equal(saved.document.history.length,1);
 assert.equal(saved.document.history[0].change.event_id,previewRequest.event_id);
 const savedPlanning=await page.$eval('[data-settings-summary-group="planning"]',e=>e.textContent);
 assert(savedPlanning.includes('Manual'),'saved choice must display its exact catalog label');
 assert(!savedPlanning.includes('Not configured in this version'));
 await page.click('[data-action="edit"]');
 await page.click('[data-settings-group="planning"] > summary');
 assert.equal(await page.$eval(select,e=>e.value),'0','reopen must select the saved value');
 await page.click('[data-settings-help="cockpit_default_autonomy"]');
 assert(await page.$eval('dialog.engine-settings-help',e=>e.textContent.includes('Manual')),'help must display the saved label');
 await page.keyboard.press('Escape');
 cases.push('historical display/explicit adoption/translated select/preview-apply string/reopen/help');
 // Descriptor and document refusals: exact membership, no numeric guessing.
 {
  const catalog=baseCatalog();catalog.fields[2].choices=[...choiceChoices.map(c=>({...c})),{value:'1',label:'Doublon'}];
  await assertLoadRefusal({catalog,document:historicalDocument(),en},'duplicate choice values must refuse the descriptor');
  cases.push('duplicate descriptor choices refusal');
 }
 {
  const catalog=baseCatalog();catalog.defaults.cockpit_default_autonomy='9';
  await assertLoadRefusal({catalog,document:historicalDocument(),en},'non-member default must refuse the descriptor');
  cases.push('non-member descriptor default refusal');
 }
 {
  const catalog=baseCatalog();catalog.fields[1].choices=[{value:'1',label:'Surcharge'}];
  await assertLoadRefusal({catalog,document:historicalDocument(),en},'regular fields must refuse extraneous choice metadata');
  cases.push('regular field extraneous choices refusal');
 }
 {
  const document={schema_version:48,revision:'1',history:[],values:{task_phase_gates:['plan'],task_max_attempts:'3',cockpit_default_autonomy:'3'}};
  await assertLoadRefusal({catalog:baseCatalog(),document,en},'non-member saved value must refuse the document');
  cases.push('non-member current value refusal');
 }
 // Advice proposals need exact string membership; numbers and null never coerce.
 await mountFixture({catalog:baseCatalog(),document:historicalDocument(),en});
 await page.click('[data-action="edit"]');
 await page.click('[data-settings-group="planning"] > summary');
 assert.equal(await page.$eval(select,e=>e.value),'2');
 await page.click('[data-settings-advice-field="cockpit_default_autonomy"]');
 await page.type('[data-advice-objective]','Choose the initial cockpit autonomy; the saved mission mode wins.');
 await page.$$eval('dialog.engine-settings-help button',buttons=>buttons.find(b=>b.textContent.includes('Préparer le contexte IA')).click());
 const prompt=await page.$eval('[data-advice-prompt]',e=>e.value);
 const context=JSON.parse(prompt.slice(prompt.indexOf('\n')+1));
 assert.equal(context.expected_revision,'1');
 const adviceField=context.target_fields.find(f=>f.name==='cockpit_default_autonomy');
 assert.equal(adviceField.unit,'choice');
 assert.deepEqual(adviceField.choices,choiceChoices);
 assert.deepEqual(adviceField.saved,{unknown:true});
 assert.equal(adviceField.draft,'2');
 assert.equal(adviceField.initial_default,'2');
 const adviceReply=async value=>{
  await page.$eval('[data-advice-reply]',(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},JSON.stringify(value));
  await page.$$eval('dialog.engine-settings-help button',buttons=>buttons.find(b=>b.textContent.includes('Examiner la proposition')).click());
 };
 const proposal={expected_revision:'1',explanation:'Exact choice required; nothing numeric is guessed.'};
 for(const bad of [{...proposal,changes:{cockpit_default_autonomy:'9'}},{...proposal,changes:{cockpit_default_autonomy:2}},{...proposal,changes:{cockpit_default_autonomy:null}}]){
  await adviceReply(bad);
  assert.equal(await page.$eval('dialog.engine-settings-help [role="alert"]',e=>e.hidden),false,'non-member, numeric or null proposal must refuse');
  assert.equal(await page.$eval(select,e=>e.value),'2');
 }
 await adviceReply({...proposal,changes:{cockpit_default_autonomy:'1'}});
 assert.equal(await page.$eval('dialog.engine-settings-help [role="alert"]',e=>e.hidden),true);
 await page.$$eval('dialog.engine-settings-help button',buttons=>buttons.find(b=>b.textContent.includes('Utiliser la sélection dans le brouillon')).click());
 await page.waitForFunction(()=>!document.querySelector('dialog.engine-settings-help'));
 assert.equal(await page.$eval(select,e=>e.value),'1');
 const advicePreview=await page.evaluate(()=>window.__calls.filter(c=>c.op==='preview').at(-1).request);
 assert.equal(advicePreview.values.cockpit_default_autonomy,'1');
 await page.keyboard.press('Escape');
 assert.equal(await page.evaluate(()=>window.__calls.filter(c=>c.op==='apply').length),0,'advice must never save by itself');
 assert.equal(await page.evaluate(()=>window.__state.document.revision),'1');
 cases.push('advice exact string membership/no numeric or null fallback/draft only');
 // A newer independent choice does not adopt every older optional policy group.
 // Rendering preserves actual absence; only an explicitly opened draft proposes
 // defaults and closing it must not publish them.
 {
  const catalog=baseCatalog();catalog.supported_document_versions=[1,47,48];
  catalog.fields.push({name:'preparation_fixture',label:'Politique indépendante',unit:'count',introduced_schema_version:47,help:'Fixture de groupe absent.'});
  catalog.defaults.preparation_fixture='17';
  const document={schema_version:48,revision:'1',history:[],values:{task_phase_gates:['plan'],task_max_attempts:'3',cockpit_default_autonomy:'2'}};
  await mountFixture({catalog,document,en});
  assert.equal(await page.$eval('[data-action="edit"]',e=>e.disabled),false);
  assert((await page.$eval('[data-settings-summary-group="preparations"]',e=>e.textContent)).includes('Not configured in this version'));
  await page.click('[data-action="edit"]');
  assert.equal(await page.$eval('[data-setting="preparation_fixture"]',e=>e.value),'17');
  assert.equal(await page.evaluate(()=>Object.hasOwn(window.__state.document.values,'preparation_fixture')),false);
  await page.keyboard.press('Escape');
  assert.equal(await page.evaluate(()=>window.__calls.filter(c=>c.op==='apply').length),0);
  cases.push('independent group absence stays unconfigured despite newer envelope');
 }
 {
  const document=historicalDocument();delete document.values.task_max_attempts;
  await assertLoadRefusal({catalog:baseCatalog(),document,en},'missing v1 core field must still refuse');
  cases.push('missing core field refusal');
 }
 {
  const document=historicalDocument();document.values.cockpit_default_autonomy='2';
  await assertLoadRefusal({catalog:baseCatalog(),document,en},'future-present choice must still refuse');
  cases.push('future field presence refusal');
 }
 assert.deepEqual(pageErrors,[]);
 console.log('PASS '+cases.length+' scalar choice Admin UI cases: '+cases.join(' | '));
}finally{if(browser)await browser.close();if(server)await new Promise(resolve=>server.close(resolve))}})().catch(e=>{console.error(e);process.exitCode=1});
