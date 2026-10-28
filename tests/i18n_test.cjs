'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const catalog=JSON.parse(fs.readFileSync('locales/en.json','utf8'));
const code=fs.readFileSync('web/i18n.js','utf8');
function runtime(query='',saved='fr'){const storage=new Map([['swarm.language',saved]]);const ctx={SwarmEnglish:catalog,URLSearchParams,location:{search:query},localStorage:{getItem:k=>storage.get(k),setItem:(k,v)=>storage.set(k,v)}};vm.runInNewContext(code,ctx);return {api:ctx.SwarmI18n,storage}}
assert.equal(runtime().api.t('Langue'),'Langue');
const {api,storage}=runtime('?lang=en');assert.equal(api.t('Langue'),'Language');assert.equal(storage.get('swarm.language'),'en');
assert.equal(runtime('',storage.get('swarm.language')).api.language,'en');
assert.equal(runtime('?lang=invalid','en').api.language,'en');
assert.equal(api.t('My own task title'),'My own task title');
assert.equal(api.t('{name}',{name:'$& $` $\''}),'$& $` $\'');
assert.equal(api.engine('2 tâche(s) ne peuvent pas continuer sans intervention.'),'2 task(s) cannot continue without intervention.');
assert.equal(api.engine('Fournisseur inconnu : my-provider'),'Unknown provider: my-provider');
assert.equal(api.engine('Le serveur a répondu HTTP 401. Vérifiez la clé, le modèle et l’adresse.'),'The server returned HTTP 401. Check the key, model and URL.');
assert.equal(api.engine('Envoi non confirmé : /renvoyer. error: $& /my/path'),'Send not confirmed: /renvoyer. error: $& /my/path');
assert.equal(api.engine('Enregistrez d’abord un plan JSON valide : détail conservé'),'Save a valid JSON plan first: détail conservé');
assert.equal(api.engine('Essai correctif autorisé. Mission en pause : reprendre la mission pour autoriser les départs'),'Corrective attempt authorized. Mission paused: resume it to authorize starts');
for(const [source,target]of Object.entries(catalog)){
 assert.equal(typeof target,'string',source);
 const verbs=s=>(s.match(/%(?:\[\d+\])?[+\-#0]*(?:\d+)?(?:\.\d+)?[sdqvftxT]/g)||[]);
 assert.deepEqual(verbs(target),verbs(source),'format placeholders: '+source);
}
const generated={};vm.runInNewContext(fs.readFileSync('web/i18n-en.js','utf8'),generated);
assert.equal(JSON.stringify(generated.SwarmEnglish),JSON.stringify(catalog),'rebuild the web catalogue');
console.log('PASS i18n locale, fallback, interpolation, engine formats and catalogue parity');
// Missing task fields must be localized without rewriting user text.
const graphContext={SwarmI18n:api,module:{exports:{}}};
vm.runInNewContext(fs.readFileSync('web/pilot-graph.js','utf8'),graphContext);
assert.equal(graphContext.module.exports.guidance({}), 'Complete: deliverable, success criterion, instructions');
{
 const cp=require('node:child_process'),os=require('node:os'),path=require('node:path');
 const fixture=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-i18n-build-'));
 try{
  fs.mkdirSync(path.join(fixture,'locales'));fs.mkdirSync(path.join(fixture,'web'));
  fs.writeFileSync(path.join(fixture,'locales/en.json'),JSON.stringify({Langue:'Language'}));
  fs.writeFileSync(path.join(fixture,'locales/trust-actions-en.json'),JSON.stringify({Pilote:'Pilot'}));
  const script=path.resolve('scripts/i18n/build.cjs');
  cp.execFileSync(process.execPath,[script],{cwd:fixture,stdio:'pipe'});
  cp.execFileSync(process.execPath,[script,'--check'],{cwd:fixture,stdio:'pipe'});
  const target=path.join(fixture,'web/trust-actions-en.js'),other=path.join(fixture,'web/i18n-en.js');
  const retained=fs.readFileSync(other);fs.writeFileSync(target,'stale original output\n');
  const check=cp.spawnSync(process.execPath,[script,'--check'],{cwd:fixture,encoding:'utf8'});
  assert.notEqual(check.status,0,'check must reject a stale generated artifact');
  assert.equal(fs.readFileSync(target,'utf8'),'stale original output\n','check must not repair tested sources');
  assert.deepEqual(fs.readFileSync(other),retained,'check changed another generated artifact');
  console.log('PASS i18n build check accepts current output and refuses stale output without mutation');
 }finally{fs.rmSync(fixture,{recursive:true,force:true})}
}
