'use strict';
// Pure graph contracts shared by the renderer and its independent Node recipes.
const PilotGraph = (() => {
 function children(tasks) {
  const out=new Map(tasks.map(t=>[t.id,[]]));
  for(const t of tasks) for(const d of t.depends||[]) out.get(d)?.push(t.id);
  for(const list of out.values())list.sort();
  return out;
 }
 function visible(tasks,collapsed) {
  const edges=children(tasks),closed=new Set(collapsed),seen=new Set();
  const known=new Set(tasks.map(t=>t.id));
  const todo=tasks.filter(t=>!(t.depends||[]).some(d=>known.has(d))).map(t=>t.id).sort().reverse();
  while(todo.length){const id=todo.pop();if(seen.has(id))continue;seen.add(id);if(!closed.has(id))todo.push(...(edges.get(id)||[]).slice().reverse())}
  return seen;
 }
 function descendants(tasks,id) {
  const edges=children(tasks),seen=new Set([id]),todo=[...(edges.get(id)||[])],out=[];
  while(todo.length){const next=todo.pop();if(seen.has(next))continue;seen.add(next);out.push(next);todo.push(...(edges.get(next)||[]))}
  return out.sort();
 }
 function hiddenBelow(tasks,id,collapsed) {const shown=visible(tasks,collapsed);return descendants(tasks,id).filter(x=>!shown.has(x))}
 function reveal(tasks,target,collapsed) {
  const closed=new Set(collapsed),edges=children(tasks),known=new Set(tasks.map(t=>t.id));
  const queue=tasks.filter(t=>!(t.depends||[]).some(d=>known.has(d))).map(t=>({id:t.id,cost:0,path:[t.id]})),best=new Set();
  const compare=(a,b)=>a.cost-b.cost||a.path.length-b.path.length||a.path.join('/').localeCompare(b.path.join('/'));
  while(queue.length){
   queue.sort(compare);const current=queue.shift();if(best.has(current.id))continue;best.add(current.id);
   if(current.id===target)return collapsed.filter(id=>!current.path.slice(0,-1).includes(id));
   for(const id of edges.get(current.id)||[]) if(!best.has(id))queue.push({id,cost:current.cost+(closed.has(current.id)?1:0),path:[...current.path,id]});
  }
  return collapsed;
 }
 function preferences(v={}) {
  return {view:['agents','dependencies'].includes(v.view)?v.view:'dependencies',
   orientation:v.orientation==='TB'?'TB':'LR',detail:v.detail==='detailed'?'detailed':'simple',
   groups:Object.fromEntries(['En activité','À examiner','Historique','À préparer'].filter(k=>typeof v.groups?.[k]==='boolean').map(k=>[k,v.groups[k]])),
   grouped:v.grouped===true,filter:['all','active','unknown','review','waiting','finished'].includes(v.filter)?v.filter:'all',
   search:typeof v.search==='string'?v.search.slice(0,200):'',collapsed:Array.isArray(v.collapsed)?v.collapsed.filter(x=>typeof x==='string'):[],
   zoom:Number.isFinite(v.zoom)?Math.max(.15,Math.min(2,v.zoom)):1,
   x:Number.isFinite(v.x)?Math.max(0,v.x):0,y:Number.isFinite(v.y)?Math.max(0,v.y):0,
   selection:v.selection&&['task','agent','decision'].includes(v.selection.kind)&&typeof v.selection.id==='string'?v.selection:null};
 }
 return {children,visible,descendants,hiddenBelow,reveal,preferences};
})();
if(typeof module!=='undefined')module.exports=PilotGraph;
