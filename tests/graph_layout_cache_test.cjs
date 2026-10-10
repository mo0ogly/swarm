'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const dagre=require('../web/lib/dagre.min.js');
const source=fs.readFileSync(require('node:path').join(__dirname,'../web/graph.js'),'utf8');
const implementation=source.slice(source.indexOf('function graphLayout('),source.indexOf('\nfunction renderGraph('));
const saved=new Map();let layouts=0;
const context=vm.createContext({dagre:{...dagre,layout:g=>{layouts++;dagre.layout(g)}},sessionStorage:{getItem:key=>saved.get(key),setItem:(key,value)=>saved.set(key,value)}});
vm.runInContext(implementation+';this.run=graphLayout',context);
function graph(dir='LR',width=310){const g=new dagre.graphlib.Graph();g.setGraph({rankdir:dir,ranker:'tight-tree',nodesep:28,ranksep:70,marginx:20,marginy:20});g.setDefaultEdgeLabel(()=>({}));for(const id of ['root','left','right','shared'])g.setNode(id,{width,height:210});for(const [v,w]of [['root','left'],['root','right'],['left','shared'],['right','shared']])g.setEdge(v,w);return g}
const geometry=g=>JSON.parse(JSON.stringify({graph:g.graph(),nodes:g.nodes().map(id=>[id,g.node(id)]),edges:g.edges().map(e=>[e,g.edge(e)])}));
const first=graph();context.run(first,'same topology');assert.equal(layouts,1);
const expected=geometry(first),warm=graph();context.run(warm,'same topology');assert.equal(layouts,1,'same geometry must avoid the expensive layout');assert.deepEqual(geometry(warm),expected,'cached node positions and every routed edge must match dagre');
const vertical=graph('TB');context.run(vertical,'same topology');assert.equal(layouts,2,'orientation invalidates cache');assert.notDeepEqual(geometry(vertical),expected);
const changed=graph('TB');changed.setEdge('root','shared');context.run(changed,'changed topology');assert.equal(layouts,3,'new dependency invalidates cache');assert.equal(changed.edge('root','shared').points.length>=2,true);
context.run(graph('TB',400),'changed topology');assert.equal(layouts,4,'node dimensions invalidate cache');
for(const corrupt of [v=>{v.graph.width=null},v=>{v.nodes[0].x=-1},v=>{v.edges[0].points=[]},v=>{v.edges[0].v='wrong-task'}]){
 const g=graph();context.run(g,'same topology');const [key,value]=[...saved][0];const payload=JSON.parse(value);corrupt(payload);saved.set(key,JSON.stringify(payload));const count=layouts,next=graph();context.run(next,'same topology');assert.equal(layouts,count+1,'corrupt geometry must recompute');assert.deepEqual(geometry(next),expected);
}
context.sessionStorage={getItem(){throw Error('storage unavailable')},setItem(){throw Error('quota exceeded')}};
const count=layouts,fallback=graph();context.run(fallback,'same topology');assert.equal(layouts,count+1);assert.deepEqual(geometry(fallback),expected);
console.log('PASS graph geometry cache: exact dagre nodes/routes; topology/orientation/dimension invalidation; corrupt/unavailable storage fallback');
