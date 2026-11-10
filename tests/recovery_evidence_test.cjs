'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
class Element{constructor(tag,text,kind){this.tag=tag;this.textContent=text||'';this.className=kind||'';this.children=[];this.dataset={}}append(...children){this.children.push(...children)}}
const en=JSON.parse(fs.readFileSync('locales/en.json','utf8'));
for(const lang of ['fr','en']){
 const tr=s=>lang==='en'?(en[s]??s):s;
 const c=vm.createContext({Mission:{},PilotActions:{},SwarmI18n:{t:tr},node:(...a)=>new Element(...a),missionText:tr});vm.runInContext(fs.readFileSync('web/mission-insights.js','utf8'),c);
 const p={task:'t1',kept:[],redone:[],criteria:['criterion 1','criterion 2'],remaining_criteria:['criterion 2'],correction:'Fix one file',since_refusal:{items:[{revision:42,at:'now',label:'Consigne de reprise modifiée'}],more:true},evidence:[{report:'same.md',state:'unchanged',reason:'Contenu inchangé ; réutilisable comme entrée seulement, pas comme validation.'},{report:'changed.md',state:'changed',reason:'Contenu modifié depuis l’avis : ancienne preuve périmée, vérification requise.'},{report:'missing.md',state:'unknown',reason:'Empreinte ou contenu indisponible : réutilisation non démontrée.'}]};
 const box=c.Mission.recoveryPreviewView(p);const flatten=e=>[e.textContent,...e.children.flatMap(flatten)];const text=flatten(box).join('\n');
 assert.match(text,lang==='fr'?/Depuis le refus/:/Since the refusal/);assert.match(text,/r42/);assert.match(text,/same.md/);assert.match(text,/changed.md/);assert.match(text,/missing.md/);assert.match(text,lang==='fr'?/pas comme validation/:/never as acceptance/);assert.match(text,lang==='fr'?/périmée/:/stale/);assert.match(text,lang==='fr'?/non démontrée/:/not demonstrated/);assert.match(text,lang==='fr'?/Historique partiel/:/Partial history/);
 const index=box.children.findIndex(e=>e.tag==='h4'&&e.textContent===tr('Critères restant à vérifier'));assert.equal(box.children[index+1].children.length,1);assert.equal(box.children[index+1].children[0].textContent,'criterion 2');
 assert.equal(box.children.filter(e=>e.tag==='button').length,0,'preview must never execute recovery');
 p.dialogue={recommended_path:'reconcile_effects',recommendation:'Vérifier les effets incertains ; ne pas rejouer automatiquement les outils.',resume_supported:false,decision_recorded:false,checks:[{code:'effects_observed',status:'fail',message:'Un résultat d’outil manque ou sa visibilité est perdue ; qualifier les effets avant toute reprise.'},{code:'launch_preflight',status:'unknown',message:'Session fournisseur, accès actuels et réservations non vérifiés par cet aperçu.'},{code:'untrusted',status:'unknown',message:'<img src=x onerror=alert(1)>'}]};
 const qualified=c.Mission.recoveryPreviewView(p),content=flatten(qualified).join('\n'),section=qualified.children.find(e=>e.dataset.dialogueRecovery);
 assert(section);assert.equal(section.dataset.dialogueRecovery,'reconcile_effects');
 assert.match(content,lang==='fr'?/Reprise de conversation/:/Conversation recovery/);assert.match(content,lang==='fr'?/Bloquant/:/Blocking/);assert.match(content,lang==='fr'?/Non vérifié/:/Not verified/);assert.match(content,lang==='fr'?/ne lance aucun agent/:/starts no agent/);assert(content.includes('<img src=x onerror=alert(1)>'));
 const all=e=>[e,...e.children.flatMap(all)];assert.equal(all(qualified).filter(e=>['img','script','button'].includes(e.tag)).length,0,'diagnosis renders text and has no launch controls');
}
console.log('PASS FR/EN recovery delta, changed/unknown inputs and remaining criteria without approval');
