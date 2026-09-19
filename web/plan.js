'use strict';
let plansRenderKey='';
function prepareActionPlan(){
 if(!snapshot.work.planning_brief){notice('Adoptez d’abord un brief.',true);return}
 openModal('Préparer le plan d’action avec l’IA','L’IA doit respecter le cadre structuré. Vous relirez les missions avant leur ajout au plan. Aucun worker ne sera lancé.',{action:'brainstorm',launchEvent:crypto.randomUUID(),planBriefHash:snapshot.work.planning_brief.sha256});
 field('provider','IA qui prépare le plan','',Object.keys(snapshot.providers?.providers||{}).map(k=>[k,k]));field('workspace','Espace à examiner',snapshot.root);field('instruction','Précisions pour le plan','Transforme le brief adopté en plan d’action conforme au cadre JSON obligatoire fourni par le moteur. Décompose les missions, leurs dépendances et leurs preuves. Laisse les décisions non résolues à l’opérateur. Ne réalise aucune mission.',null,true);field('capture','Capture détaillée','false',[['false','Désactivée'],['true','Activée']]);addModelFields('brainstorm');$('confirm').textContent='Examiner le contexte';
}
function renderPlans(){
 const key=JSON.stringify([work,snapshot.work.plans,snapshot.work.tasks.map(t=>[t.id,t.title,t.status,snapshot.validation?.tasks[t.id]?.state])]);if(key===plansRenderKey)return;plansRenderKey=key;const target=$('action-plans');target.replaceChildren();
 for(const plan of snapshot.work.plans||[]){const box=node('section',undefined,'card');box.append(node('h3',plan.spec.objective),node('p','Plan enregistré le '+plan.at+' · '+plan.task_ids.length+' missions. Les dépendances doivent être acceptées avant lancement.'));for(const id of plan.task_ids){const t=snapshot.work.tasks.find(x=>x.id===id);if(!t)continue;const row=node('div',undefined,'plan-task-line');row.append(node('span',t.title),badge(snapshot.validation?.tasks[t.id]?.state||t.status),button('Piloter cette mission',()=>taskDialog(id)));box.append(row)}target.append(box)}
}
async function reviewActionPlan(task){
 openModal('Examiner le plan d’action','Chargement du plan structuré…',{action:'plan-review',task:task.id});$('confirm').hidden=true;
 try{const review=await act('plan-read',{task:task.id});if(!modalContext||modalContext.task!==task.id)return;modalContext.plan=review;$('modal-description').textContent='Relisez les périmètres, gates et plafonds. Répondez aux décisions ouvertes. L’enregistrement crée les tâches et leurs dépendances, sans les lancer.';drawPlanEditor(review.spec);$('confirm').hidden=false;$('confirm').textContent='Enregistrer les missions dans le plan';}
 catch(e){if(modalContext?.task!==task.id)return;$('modal-error').textContent=e.message+' Aucune mission créée. Fermez cette fenêtre puis utilisez Préparer le plan d’action pour demander une réponse conforme.';$('modal-error').hidden=false;}
}
function drawPlanEditor(spec){
 $('modal-fields').replaceChildren();const intro=node('section',undefined,'plan-editor');intro.append(node('h3','Objectif et décisions'));
 function input(parent,name,label,value,options=null,multi=false){const e=field(name,label,value,options,multi);parent.append(e.parentElement);return e}
 const objective=input(intro,'plan-objective','Objectif du plan',spec.objective,null,true),assumptions=input(intro,'plan-assumptions','Hypothèses explicites — une par ligne',spec.assumptions.join('\n'),null,true);
 const questions=spec.questions.map((q,i)=>({question:q.question,input:input(intro,'plan-question-'+i,'Décision à résoudre : '+q.question,q.answer,null,true)}));$('modal-fields').append(intro);
 const groups=node('div',undefined,'plan-missions');$('modal-fields').append(groups);const missions=[];
 function addMission(t){const box=node('details',undefined,'plan-mission');box.open=true;const summary=node('summary',t.id+' · '+t.title),body=node('div',undefined,'plan-editor');box.append(summary,body);const fields={};
 for(const [key,label]of [['id','Identifiant local'],['title','Titre de la mission'],['scope','Périmètre et exclusions'],['deliverable','Livrable attendu'],['criteria','Critères observables — un par ligne'],['depends','Dépendances — identifiants séparés par des virgules'],['proof','Preuves attendues'],['entry','Gate entry — prérequis'],['validation','Gate validation — vérifications'],['delivery','Gate delivery — conditions de livraison'],['stop','Conditions d’arrêt et OODA']]){let v=t[key];if(Array.isArray(v))v=v.join(key==='depends'?', ':'\n');fields[key]=input(body,'mission-'+missions.length+'-'+key,label,v,null,['scope','criteria','proof','entry','validation','delivery','stop'].includes(key))}
 fields.role=input(body,'mission-'+missions.length+'-role','Rôle',t.role,[['worker','Worker — réalise'],['planner','Planner — planifie'],['subplanner','Subplanner — planifie un sous-périmètre']]);
 for(const [key,label,max]of [['max_attempts','Tentatives maximum',3],['max_tool_calls','Appels d’outils maximum par tentative',100]]){fields[key]=input(body,'mission-'+missions.length+'-'+key,label,String(t[key]));fields[key].type='number';fields[key].min='1';fields[key].max=String(max)}
 const record={box,fields};missions.push(record);body.append(button('Retirer cette mission',()=>{box.remove();record.removed=true}));groups.append(box);
 }
 spec.tasks.forEach(addMission);
 $('modal-fields').append(button('Ajouter une mission',()=>{if(missions.filter(m=>!m.removed).length>=8){$('modal-error').textContent='Huit missions maximum par plan.';$('modal-error').hidden=false;return}addMission({id:'T'+(missions.length+1),title:'Nouvelle mission',role:'worker',scope:'',deliverable:'',criteria:[],depends:[],proof:'',entry:'',validation:'',delivery:'',stop:'Deux corrections maximum puis OODA et arrêt sur blocage.',max_attempts:2,max_tool_calls:30})}));
 modalContext.collectPlan=()=>({version:1,objective:objective.value,assumptions:assumptions.value.split('\n').map(v=>v.trim()).filter(Boolean),questions:questions.map(q=>({question:q.question,answer:q.input.value})),tasks:missions.filter(m=>!m.removed).map(({fields})=>{const t={};for(const [k,e]of Object.entries(fields))t[k]=e.value;t.depends=t.depends.split(',').map(v=>v.trim()).filter(Boolean);t.criteria=t.criteria.split('\n').map(v=>v.trim()).filter(Boolean);t.max_attempts=Number(t.max_attempts);t.max_tool_calls=Number(t.max_tool_calls);return t})});
}
async function submitActionPlan(c){const spec=c.collectPlan();await act('plan-commit',{plan:{...c.plan,spec}});closeModal();await refresh();notice('Plan enregistré : '+spec.tasks.length+' missions ajoutées avec leurs dépendances. Sélectionnez une mission pour la lancer.');showView('tasks');$('action-plans').scrollIntoView({block:'start'});}
