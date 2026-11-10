'use strict';
const assert=require('node:assert/strict'),P=require('../web/product-graph.js'),G=require('../web/pilot-graph.js');
const tasks=[{id:'common',title:'Foundation',depends:[],status:'accepted'},{id:'a',title:'Order',depends:['common'],status:'accepted'},{id:'b',title:'History',depends:['a'],status:'blocked'},{id:'outside',title:'Delivery',depends:['b'],status:'todo'}];
const product={mode:'existing',journeys:[{id:'order',title:'Order',story_ids:['s01-order']},{id:'history',title:'History',story_ids:['s02-history','s03-later']}],stories:[
 {id:'s01-order',title:'Order',task_ids:['A'],depends:[]},
 {id:'s02-history',title:'History',task_ids:['A','B'],depends:['s01-order']},
 {id:'s03-later',title:'Not planned',task_ids:[],depends:[]}
]};
const plan={source:'p',task_map:{A:'a',B:'b'},spec:{product,tasks:[{id:'A',phase:'implement'},{id:'B',phase:'review'}]}};
const work={tasks,plans:[plan]},c=P.catalogue(work);
assert.equal(c.stories.length,3);assert.deepEqual(c.byStory.get('p/s02-history').taskIDs,['a','b']);
assert.deepEqual(c.membership.get('a'),['p/s01-order','p/s02-history']);assert.deepEqual(c.common,['common','outside']);
assert.equal(c.phases.get('a'),'implement');
assert.deepEqual(P.selection(c,{productLevel:'story',productKey:'p/s03-later'}),[]);
assert.deepEqual(P.boundary(work,['a','b']),{incoming:[{id:'common',affects:['a']}],outgoing:[{id:'outside',depends:['b']}]});
const snap={work,validation:{tasks:{a:{state:'stale'},b:{state:'blocked'}}}};
assert.deepEqual(P.stats(snap,['a','b']),{total:2,accepted:0,blocked:1,review:0,stale:1,running:0});
assert.equal(P.stats({work},['a']).accepted,0,'stored accepted without fresh evidence is not certified');
assert.deepEqual(P.links(work,c.journeys).map(e=>[e.from,e.to]),[['p/order','p/history']]);
const next=JSON.parse(JSON.stringify(plan));next.spec.product.stories=next.spec.product.stories.slice(0,1);next.spec.product.journeys=next.spec.product.journeys.slice(0,1);
assert.equal(P.catalogue({...work,plans:[plan,next]}).stories.length,1,'only latest plan revision');
assert.equal(P.normalize(P.catalogue({...work,plans:[next]}),{productLevel:'story',productKey:'p/s02-history'}).productLevel,'overview');
assert.equal(G.preferences({productLevel:'bad',productKey:27}).productLevel,'overview');
assert.equal(G.preferences({productLevel:'story',productKey:'p/s01-order'}).productKey,'p/s01-order');
const shared=JSON.parse(JSON.stringify(plan));shared.spec.product.journeys[1].story_ids.push('s01-order');
const sharedCatalogue=P.catalogue({...work,plans:[shared]});
assert.equal(P.parent(sharedCatalogue,{productLevel:'story',productKey:'p/s01-order',productParent:'p/history'}).key,'p/history','keep the journey actually opened');
assert.equal(P.parent(sharedCatalogue,{productLevel:'story',productKey:'p/s01-order',productParent:'removed'}).key,'p/order','removed parent falls back to an existing journey');
assert.equal(G.preferences({productParent:'p/history'}).productParent,'p/history');
const other={...plan,source:'q'};assert.equal(P.catalogue({...work,plans:[plan,other]}).stories.length,6,'sources do not collide');
assert.equal(P.catalogue({tasks}).journeys.length,0,'legacy tasks do not fabricate a product');
const large=JSON.parse(JSON.stringify(plan));large.spec.product.stories=Array.from({length:300},(_,i)=>({id:'s'+i+'-story',task_ids:['A'],depends:[]}));large.spec.product.journeys=[{id:'all',story_ids:large.spec.product.stories.map(s=>s.id)}];
const l=P.catalogue({...work,plans:[large]});assert.equal(l.stories.length,300);assert.equal(l.journeys[0].taskIDs.length,1,'shared tasks counted once');
console.log('PASS product hierarchy: canonical mappings, shared tasks, external links, evidence freshness, plan revisions, legacy and 300 stories');
