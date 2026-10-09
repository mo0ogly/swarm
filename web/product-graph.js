'use strict';

// Read-only projections of adopted plans. No title parsing, inferred stories or
// independent copies of task state. The latest revision of each plan owns its map.
const ProductGraph=(()=>{
 function catalogue(work){
  const latest=new Map(),tasks=new Map((work.tasks||[]).map(t=>[t.id,t]));
  for(const plan of work.plans||[])latest.set(plan.source,plan);
  const journeys=[],stories=[],phases=new Map();
  for(const plan of latest.values()){
   const product=plan.spec?.product;if(!product)continue;
   const key=id=>plan.source+'/'+id,mapping=plan.task_map||{};
   const local=new Map((product.stories||[]).map(s=>[s.id,key(s.id)]));
   for(const t of plan.spec.tasks||[])if(mapping[t.id]&&tasks.has(mapping[t.id]))phases.set(mapping[t.id],t.phase||'');
   for(const s of product.stories||[])stories.push({...s,key:key(s.id),source:plan.source,taskIDs:[...new Set((s.task_ids||[]).map(id=>mapping[id]).filter(id=>tasks.has(id)))],dependencyKeys:(s.depends||[]).map(id=>local.get(id)).filter(Boolean)});
   for(const j of product.journeys||[])journeys.push({...j,key:key(j.id),source:plan.source,storyKeys:(j.story_ids||[]).map(id=>local.get(id)).filter(Boolean)});
  }
  const byStory=new Map(stories.map(s=>[s.key,s])),byJourney=new Map(journeys.map(j=>[j.key,j]));
  for(const j of journeys)j.taskIDs=[...new Set(j.storyKeys.flatMap(id=>byStory.get(id)?.taskIDs||[]))];
  const membership=new Map();for(const s of stories)for(const id of s.taskIDs){if(!membership.has(id))membership.set(id,[]);membership.get(id).push(s.key)}
  const common=[...tasks.keys()].filter(id=>!membership.has(id));
  return {journeys,stories,byStory,byJourney,tasks,phases,membership,common};
 }
 function selection(c,state){
  if(state.productLevel==='story')return c.byStory.get(state.productKey)?.taskIDs||[];
  if(state.productLevel==='common')return c.common;
  return [...c.tasks.keys()];
 }
 function stats(snapshot,ids){
  const tasks=new Map((snapshot.work.tasks||[]).map(t=>[t.id,t]));
  const out={total:ids.length,accepted:0,blocked:0,review:0,stale:0,running:0};
  for(const id of new Set(ids)){
   const t=tasks.get(id),v=snapshot.validation?.tasks?.[id];if(!t)continue;
   if(v?.state==='accepted')out.accepted++;
   if(v?.state==='stale')out.stale++;
   if(t.status==='blocked')out.blocked++;
   if(t.status==='submitted')out.review++;
   if(t.status==='running')out.running++;
  }
  return out;
 }
 function boundary(work,ids){
  const selected=new Set(ids),incoming=new Map(),outgoing=new Map();
  for(const task of work.tasks||[])for(const dep of task.depends||[]){
   if(selected.has(task.id)&&!selected.has(dep)){if(!incoming.has(dep))incoming.set(dep,[]);incoming.get(dep).push(task.id)}
   if(!selected.has(task.id)&&selected.has(dep)){if(!outgoing.has(task.id))outgoing.set(task.id,[]);outgoing.get(task.id).push(dep)}
  }
  return {incoming:[...incoming].map(([id,affects])=>({id,affects})),outgoing:[...outgoing].map(([id,depends])=>({id,depends}))};
 }
 function links(work,groups){
  const membership=new Map(),edges=new Map();
  for(const g of groups)for(const id of g.taskIDs){if(!membership.has(id))membership.set(id,[]);membership.get(id).push(g.key)}
  for(const task of work.tasks||[])for(const dep of task.depends||[])for(const from of membership.get(dep)||[])for(const to of membership.get(task.id)||[]){
   if(from===to)continue;const key=JSON.stringify([from,to]);if(!edges.has(key))edges.set(key,{from,to,taskLinks:[]});edges.get(key).taskLinks.push([dep,task.id]);
  }
  return [...edges.values()];
 }
 function normalize(c,state){
  if(state.productLevel==='journey'&&!c.byJourney.has(state.productKey)||state.productLevel==='story'&&!c.byStory.has(state.productKey))return {...state,productLevel:'overview',productKey:''};
  return state;
 }
 return {catalogue,selection,stats,boundary,links,normalize};
})();
if(typeof module!=='undefined')module.exports=ProductGraph;
