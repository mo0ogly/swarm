'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const en=JSON.parse(fs.readFileSync('locales/en.json','utf8'));
for(const lang of ['fr','en']){
 const context=vm.createContext({SwarmI18n:{t:s=>lang==='en'?(en[s]??s):s}});
 vm.runInContext(fs.readFileSync('web/task-models.js','utf8')+'\nglobalThis.models=TaskModels;',context);
 const requested={provider:'fixture',model_route:{model:'requested-model'}};
 const unknown=context.models.text({},requested);
 assert.match(unknown,/requested-model/);
 assert.match(unknown,lang==='fr'?/Modèle rapporté : Inconnu/:/Reported model: Unknown/);
 const legacy=context.models.text({},{provider:'fixture',reported_model:{model:'legacy-declaration'}});
 assert.match(legacy,/legacy-declaration/);
 assert.match(legacy,lang==='fr'?/Modèle demandé : fixture · Inconnu/:/Requested model: fixture · Unknown/);
 const divergent=context.models.text({},{...requested,reported_model:{model:'reported-model'}});
 assert.match(divergent,/requested-model/);
 assert.match(divergent,/reported-model/);
 assert.equal(requested.model_route.model,'requested-model');
 assert.doesNotMatch(divergent,lang==='fr'?/Utilisé :/:/Used: /);
}
console.log('PASS requested and reported models remain separate in FR/EN; missing observation remains unknown');
