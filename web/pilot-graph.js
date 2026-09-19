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
 // pilotAgents is newest-first. Keep that server contract here so every task
 // entry opens the same, exact attempt instead of whichever card was rendered.
 function taskSession(agents,taskID) {
  const item=agents.find(x=>x.agent?.task_id===taskID);
  if(!item)return null;
  const running=['queued','starting','running','stopping'].includes(item.agent.status);
  return {agent:item.agent,label:running?'Voir l’agent travailler':'Voir la session'};
 }
 function role(task,agent) {
  const key=agent?.role||task.launch_profile?.role||task.plan_role||'';
  const roles={planner:['◇','Planificateur','info'],subplanner:['⑂','Responsable de branche','attention'],worker:['⚙','Exécutant','info'],reviewer:['✓','Vérificateur','succes']};
  const [icon,label,tone]=roles[key]||['?','Rôle à préciser','attention'];
  return {icon,label,tone};
 }
 function guidance(task,go,validation) {
  if(validation?.state==='accepted')return '✓ Résultat validé';
  if(task.status==='abandoned')return 'Tâche abandonnée';
  if(task.status==='submitted')return 'À examiner : le résultat reçu';
  if(task.status==='blocked')return 'À résoudre : '+(task.blocker||go?.reason||'examiner la tentative');
  if(task.status==='running')return 'Suivi : ouvrir le journal de l’agent';
  if(validation?.state==='stale')return 'À vérifier : preuves périmées';
  const missing=[];
  if(!task.deliverable?.trim())missing.push('livrable');
  if(!task.criteria?.length)missing.push('critère de réussite');
  if(!task.next?.trim()&&!task.launch_profile?.instruction?.trim())missing.push('consigne');
  if(missing.length)return 'À compléter : '+missing.join(', ');
  if(go?.visible&&!go.ready)return 'En attente : '+go.reason;
  return go?.ready?'Prête à démarrer':'Voir les conditions de la tâche';
 }
 function organization(work,tasks,paused=false){
  const p=work.planning;if(!p)return {nodes:[],edges:[]};
  const nodes=(p.scopes||[]).map(s=>({id:'@scope/'+s.id,kind:'planner',scope:s.id,tone:s.parent?'attention':'info',title:s.parent?'⑂ Sous-responsable · '+s.id:'◇ Orchestrateur · '+p.provider,description:paused||p.paused?'En pause':s.holder?'Décision en cours':s.state==='closed'?'Périmètre terminé':'Attend les retours',detail:(s.requirements||[]).length+' exigences · décide sans coder'}));
  nodes.push({id:'@reviewer',kind:'reviewer',tone:p.reviewer?'succes':'attention',title:p.reviewer?'✓ Vérificateur IA · '+p.reviewer.provider:p.repository?'✓ Contrôleur du moteur':'! Vérificateur IA absent',description:p.reviewer?(paused?'En pause':p.reviewer.failure?'Vérification interrompue':tasks.some(t=>t.independent_review?.state==='running')?'Examen en cours':'Attend les résultats'):p.repository?'Tests et intégration Git':'À configurer',detail:p.reviewer?p.reviewer.calls+'/'+p.reviewer.max_calls+' appels · session indépendante':p.repository?'Aucune revue IA indépendante':'Aucun avis IA ne sera inventé'});
  const ids=new Set(nodes.map(n=>n.id)),edges=[];
  for(const s of p.scopes||[])if(s.parent&&ids.has('@scope/'+s.parent))edges.push({from:'@scope/'+s.parent,to:'@scope/'+s.id});
  for(const t of tasks){if(ids.has('@scope/'+t.scope_id))edges.push({from:'@scope/'+t.scope_id,to:t.id});edges.push({from:t.id,to:'@reviewer'})}
  if(!tasks.length&&ids.has('@scope/root'))edges.push({from:'@scope/root',to:'@reviewer'});
  return {nodes,edges};
 }
 return {organization,role,guidance,children,visible,descendants,hiddenBelow,reveal,preferences,taskSession};
})();
if(typeof module!=='undefined')module.exports=PilotGraph;
