'use strict';
const assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
class Element{constructor(tag,text,kind){this.tag=tag;this.textContent=text||'';this.className=kind||'';this.children=[];this.dataset={};this.attributes={}}append(...children){this.children.push(...children)}replaceChildren(...children){this.children=children}setAttribute(name,value){this.attributes[name]=value}}
const m=vm.runInNewContext(fs.readFileSync(__dirname+'/../web/mission.js','utf8')+';Mission',{node:(tag,text,kind)=>new Element(tag,text,kind),document:{activeElement:null}});
const base={authorized:true,enabled:true,paused:false,validated:0,running:0,total:1,active_agents:0,uncertain_agents:0,supervision:{state:'active',source:'serveur web'},tasks:[]};
const task=(state,id='t1',impact=0)=>({state,id,title:id,reason:'Motif du moteur',impact});
for(const [d,kind,label]of [
 [{total:0},'prepare','Préparer les tâches'],
 [{paused:true,tasks:[task('configure')]},'configure','Préparer le lancement'],
 [{paused:true,tasks:[task('intervention')]},'decision','Résoudre le blocage'],
 [{running:1,tasks:[task('running'),task('review')]},'decision','Examiner le résultat'],
 [{running:1,tasks:[task('running')]},'follow','Suivre cette tâche'],
 [{paused:true,tasks:[task('waiting')]},'resume','Reprendre la mission'],
 [{authorized:false,enabled:false,tasks:[task('manual')]},'start','Lancer la mission'],

 [{authorized:true,enabled:false,supervision:{state:'absent'},tasks:[task('waiting')]},'supervision','Comprendre la reprise'],
 [{authorized:true,enabled:false,supervision:{state:'error'},tasks:[task('waiting')]},'supervision','Comprendre la reprise'],
 [{validated:1,tasks:[task('validated')]},'results','Voir les résultats'],
 [{tasks:[task('waived')]},'results','Voir les résultats'],
 [{tasks:[task('waiting')]},'results','Voir ce qui attend'],
]){const o=m.overview({...base,...d});assert.equal(o.kind,kind);assert.equal(o.label,label)}
assert.equal(m.overview({...base,tasks:[task('review','a',1),task('intervention','b',7)]}).task.id,'b');
assert.equal(m.overview({...base,tasks:[task('waived')]}).tone,'attention');
const grouped=m.overview({...base,total:3,tasks:[task('configure','a'),task('configure','b'),task('waiting','c')]});
assert.equal(grouped.kind,'configure');assert.match(grouped.next,/2 tâches/);
const facts=m.understandingView({what:'Une attente normale.',next_step:'Le superviseur réessaiera.',actor:'Le superviseur',actor_kind:'supervisor',situation:'attente_normale'});
assert.equal(facts.tag,'dl');assert.equal(facts.dataset.situation,'attente_normale');assert.equal(facts.dataset.actor,'supervisor');
assert.deepEqual(Array.from(facts.children,row=>row.children[0].textContent),['Ce qui se passe','Prochaine étape','Qui agit']);
assert.deepEqual(Array.from(facts.children,row=>row.children[1].textContent),['Une attente normale.','Le superviseur réessaiera.','Le superviseur']);
const contract=m.launchContractView({scope:'2 tâches dans la recette',budget:'5 USD disponibles',recovery:'2 reprises maximum',validation:'1 contrôle préautorisé'});assert.equal(contract.tag,'dl');assert.deepEqual(Array.from(contract.children,row=>row.children[0].textContent),['Portée','Budget','Reprises','Validations']);assert.deepEqual(Array.from(contract.children,row=>row.children[1].textContent),['2 tâches dans la recette','5 USD disponibles','2 reprises maximum','1 contrôle préautorisé']);
const diagnostic=m.diagnosticView({observed_errors:3,consecutive_error_limit:3,limit_reached:true,summary:'3 erreurs d’outil observées. Le plafond est de 3 erreurs consécutives.',items:[{category:'environment',label:'Environnement sandbox ou montage',cause:'Montage refusé.',consequence:'Résultat non démontré.',action:'Vérifier le montage avant reprise.',action_kind:'verify_environment',traces:['permission denied']} ]});
assert.equal(diagnostic.dataset.limitReached,'true');
assert.match(diagnostic.children[1].textContent,/3 erreurs d’outil observées/);
const issue=diagnostic.children[2];assert.equal(issue.dataset.category,'environment');
assert.deepEqual(Array.from(issue.children.slice(1,4),e=>e.textContent),['Cause : Montage refusé.','Conséquence : Résultat non démontré.','Action disponible : Vérifier le montage avant reprise.']);
assert.equal(issue.children[4].tag,'details');assert.match(issue.children[4].children[0].textContent,/Traces techniques/);
const supervised={...base,total:0,coordination:[{kind:'work',label:'Travail',summary:'Un agent travaille.',actor:'L’agent',next_step:'Attendre.'},{kind:'wait',label:'Attente',summary:'Assembler attend Produire.',actor:'Le superviseur',next_step:'Reprendre après la remise.'},{kind:'exchange',label:'Échanges',summary:'Produire remet un résultat à Assembler.',actor:'Assembler',next_step:'Consommer la remise.',at:'2026-09-18T09:59:00Z',relative:'il y a environ 1 min'},{kind:'validation',label:'Vérification',summary:'Un contrôle est en cours.',actor:'Le superviseur',next_step:'Publier la décision.'}],supervision:{state:'active',source:'mission watch',late:true,last_check_at:'2026-09-18T10:00:00Z',last_check_relative:'à l’instant',next_check_at:'2026-09-18T10:00:02Z',next_check_relative:'en retard d’environ 1 s',last_action:{at:'2026-09-18T09:58:00Z',relative:'il y a environ 2 min',actor:'Conducteur Swarm',kind:'dispatch',summary:'t1 : départ automatique'}}};
const host=new Element('section');m.status(host,supervised,false);
const text=e=>e.textContent+' '+e.children.map(text).join(' '),supervision=host.children.find(e=>e.className==='mission-current');
assert.ok(supervision);assert.match(text(supervision),/vérification en retard/);assert.match(text(supervision),/prochaine vérification : en retard d’environ 1 s/);assert.match(text(supervision),/Dernière action — Conducteur Swarm/);assert.match(text(supervision),/moteur local/);assert.match(text(supervision),/supervision externe Codex/);
const coordination=host.children.find(e=>e.className==='mission-coordination');assert.ok(coordination);assert.equal(coordination.attributes['aria-label'],'Coordination de la mission');assert.deepEqual(Array.from(coordination.children.slice(1),e=>e.children[0].textContent),['Travail','Attente','Échanges','Vérification']);assert.match(text(coordination),/Assembler attend Produire/);assert.match(text(coordination),/Acteur : Le superviseur/);assert.match(text(coordination),/18 sept\. 2026/);
const noAction=new Element('section');m.status(noAction,{...base,total:0,authorized:true,enabled:false,supervision:{state:'absent'}},false);assert.match(text(noAction),/aucune action enregistrée/);assert.doesNotMatch(text(noAction),/Conducteur Swarm observé/);
console.log('PASS: décisions de pilotage, coordination A8, triplet factuel, diagnostic Q3 et supervision Q5');
