'use strict';
const assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const m=vm.runInNewContext(fs.readFileSync(__dirname+'/../web/mission.js','utf8')+';Mission');
const base={enabled:true,paused:false,validated:0,running:0,total:1,tasks:[]};
const task=(state,id='t1',impact=0)=>({state,id,title:id,reason:'Motif du moteur',impact});
for(const [d,kind,label]of [
 [{total:0},'prepare','Préparer les tâches'],
 [{paused:true,tasks:[task('intervention')]},'decision','Résoudre le blocage'],
 [{running:1,tasks:[task('running'),task('review')]},'decision','Examiner le résultat'],
 [{running:1,tasks:[task('running')]},'follow','Suivre cette tâche'],
 [{paused:true,tasks:[task('waiting')]},'resume','Reprendre la mission'],
 [{enabled:false,tasks:[task('manual')]},'start','Lancer la mission'],
 [{validated:1,tasks:[task('validated')]},'results','Voir les résultats'],
 [{tasks:[task('waived')]},'results','Voir les résultats'],
 [{tasks:[task('waiting')]},'results','Voir ce qui attend'],
]){const o=m.overview({...base,...d});assert.equal(o.kind,kind);assert.equal(o.label,label)}
assert.equal(m.overview({...base,tasks:[task('review','a',1),task('intervention','b',7)]}).task.id,'b');
assert.equal(m.overview({...base,tasks:[task('waived')]}).tone,'attention');
console.log('PASS: 11 décisions de pilotage');
