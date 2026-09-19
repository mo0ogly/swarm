'use strict';
const assert=require('node:assert/strict');
const G=require('../web/pilot-graph.js');
const tasks=[{id:'A',depends:[]},{id:'B',depends:['A']},{id:'C',depends:['A']},{id:'D',depends:['B','C']}];
assert.deepEqual([...G.visible(tasks,['B'])].sort(),['A','B','C','D']);
assert.deepEqual([...G.visible(tasks,['B','C'])].sort(),['A','B','C']);
assert.deepEqual(G.hiddenBelow(tasks,'B',['B','C']),['D']);
assert.deepEqual(G.reveal(tasks,'D',['A','B','C']),['C']);
assert.equal(G.descendants(tasks,'A').length,3);
assert.deepEqual([...G.visible([],[])],[]);
const chain=[{id:'A',depends:[]},{id:'B',depends:['A']},{id:'C',depends:['B']},{id:'D',depends:['C']}];
assert.deepEqual([...G.visible(chain,['C'])],['A','B','C']);
assert.deepEqual(G.reveal(chain,'D',['A','C']),[]);
const bad=G.preferences({orientation:'oops',zoom:Infinity,collapsed:'bad',view:'evil'});
assert.equal(bad.orientation,'LR');assert.equal(bad.zoom,1);assert.deepEqual(bad.collapsed,[]);
console.log('PASS: shared branches, nested folds, reveal, identity-safe preferences');

const roots=[{id:'A',depends:[]},{id:'X',depends:[]},{id:'D',depends:['A','X']}];
assert.deepEqual([...G.visible(roots,['A'])].sort(),['A','D','X']);
assert.deepEqual(G.hiddenBelow(roots,'A',['A']),[]);
assert.deepEqual([...G.visible([...chain,{id:'E',depends:['D']}],['A'])],['A']);
assert.deepEqual(G.hiddenBelow([...chain,{id:'E',depends:['D']}],'A',['A']),['B','C','D','E']);
assert.deepEqual([...G.visible([{id:'A',depends:[]},{id:'C',depends:['A']}],['B'])],['A','C']);
console.log('PASS: multiple roots, topology growth and removed node identity');

assert.deepEqual(G.preferences({groups:{'En activité':false,'Historique':'evil'}}).groups,{'En activité':false});

assert.equal(G.preferences().view,'dependencies');

// Revealing a shared node is independent of input order and preserves unrelated folds.
for(const graph of [tasks,[...tasks].reverse()]) assert.deepEqual(G.reveal(graph,'D',['A','B','C']),['C']);
const unequal=[{id:'A',depends:[]},{id:'B',depends:['A']},{id:'X',depends:[]},{id:'D',depends:['B','X']}];
assert.deepEqual(G.reveal(unequal,'D',['A','B','X']),['A','B']);
assert.deepEqual(G.reveal(unequal,'removed',['A','B','X']),['A','B','X']);
console.log('PASS: deterministic reveal, shorter path and removed target');
