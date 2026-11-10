 'use strict';
/** Regression for valid task IDs colliding with dagre/object prototypes. Uses
 * actual local vendor and canonical helpers, not a mock graph. No provider/IO
 * mutation, watchdog or engine policy. Throws on lost nodes/edges/grouping. */
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict'),dagre=require('../web/lib/dagre.min.js');
const context=vm.createContext({snapshot:{agents:[]},active:()=>true});vm.runInContext(fs.readFileSync('web/graph.js','utf8'),context);
const key=id=>vm.runInContext('graphNodeKey('+JSON.stringify(id)+')',context);
for(const id of ['ordinary','__proto__','constructor','toString','hasOwnProperty','swarm-node:ordinary']){
 const graph=new dagre.graphlib.Graph();graph.setGraph({});graph.setDefaultEdgeLabel(()=>({}));graph.setNode(key(id),{width:260,height:72});graph.setNode(key('dependent'),{width:260,height:72});graph.setEdge(key(id),key('dependent'));dagre.layout(graph);
 assert.equal(graph.nodes().length,2);assert(Number.isFinite(graph.node(key(id)).x));assert(graph.edge(key(id),key('dependent')).points.length>0);console.log('PASS placement-preserves-'+id);
 context.snapshot.agents=[{agent:{task_id:id}}];assert.equal(vm.runInContext('graphAgentsParTache()[snapshot.agents[0].agent.task_id].length',context),1);console.log('PASS agent-index-preserves-'+id);
}
assert.notEqual(key('ordinary'),key('swarm-node:ordinary'));console.log('PASS injective-source-identifier-boundary');
