'use strict';
const assert=require('node:assert/strict');
const PilotGraph=require('../web/pilot-graph.js');

const agents=[
 {agent:{id:'latest-active',task_id:'Q1',status:'running'}},
 {agent:{id:'older-finished',task_id:'Q1',status:'completed'}},
 {agent:{id:'other-task',task_id:'Q2',status:'failed'}}
];

assert.deepEqual(PilotGraph.taskSession(agents,'Q1'),{
 agent:agents[0].agent,
 label:'Voir l’agent travailler'
});
agents[0].agent.status='completed';
assert.equal(PilotGraph.taskSession(agents,'Q1').label,'Voir la session');
assert.equal(PilotGraph.taskSession(agents,'absente'),null);
console.log('PASS q1_session_contract');
