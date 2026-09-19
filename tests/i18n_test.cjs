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
assert.equal(api.engine('Fournisseur inconnu : my-provider'),'Unknown provider: my-provider');
assert.equal(api.engine('Le serveur a répondu HTTP 401. Vérifiez la clé, le modèle et l’adresse.'),'The server returned HTTP 401. Check the key, model and URL.');
assert.equal(api.engine('Envoi non confirmé : /renvoyer. error: $& /my/path'),'Send not confirmed: /renvoyer. error: $& /my/path');
assert.equal(api.engine('Enregistrez d’abord un plan JSON valide : détail conservé'),'Save a valid JSON plan first: détail conservé');
for(const [source,target]of Object.entries(catalog)){
 assert.equal(typeof target,'string',source);
 const verbs=s=>(s.match(/%(?:\[\d+\])?[+\-#0]*(?:\d+)?(?:\.\d+)?[sdqvftxT]/g)||[]);
 assert.deepEqual(verbs(target),verbs(source),'format placeholders: '+source);
}
const generated={};vm.runInNewContext(fs.readFileSync('web/i18n-en.js','utf8'),generated);
assert.equal(JSON.stringify(generated.SwarmEnglish),JSON.stringify(catalog),'rebuild the web catalogue');
console.log('PASS i18n locale, fallback, interpolation, engine formats and catalogue parity');
