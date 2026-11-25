/* Recorded Work and protected task review in the Conduite workbench. */
(()=>{
 'use strict';
 const t=s=>globalThis.SwarmI18n?.t(s)??s,engine=s=>globalThis.SwarmI18n?.engine(s)??s;
 let mounted=false,view='graph',workId='',model=null,selected='',query='',role='',host,lanes,detail,feedback,search,roleSelect,buttons,summary,filters,filter='',observer,missionCard,taskDialog,taskDialogHeader,taskDialogFiles,taskDialogSelection,disposeTaskFiles,returnToTaskButton,reviewInFlight=false;
 const preference='swarm-conduite:view:v1';
 /** Create a detached text-only element. User records are never parsed as HTML. */
 function el(tag,text,cls){const node=document.createElement(tag);if(text!==undefined)node.textContent=text;if(cls)node.className=cls;return node}
 /** Render the recorded text and allow only exact local Codex chat URIs.
  * Other schemes and malformed IDs remain literal. The desktop handler
  * receives a URI only on user activation, with no task or authority changes. */
 function deliverableText(text){
  const value=String(text),result=el('p');
  const links=/(^|[\s(])(codex:\/\/threads\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})(?=$|[\s)])/gu;
  let position=0;
  for(const match of value.matchAll(links)){
   const start=match.index+match[1].length,url=match[2],link=el('a',url);
   link.href=url;
   result.append(value.slice(position,start),link);
   position=start+url.length;
  }
  result.append(value.slice(position));
  return result;
 }
 /** Choose a visual signal for an exact recorded state, without deriving rights
  * or inferring execution from a title/reason. Unknown states remain neutral. */
 function signal(state){if(['validated','accepted','done','completed'].includes(state))return 'success';if(['blocked','failed','error','rejected'].includes(state))return 'alert';if(['review','in_review','awaiting_review','submitted','stale','waived'].includes(state))return 'warn';if(['running','active','in_progress'].includes(state))return 'info';return 'neutral'}
 /** Translate exact recorded state codes when a detailed status label is absent.
  * Input is an engine string, output a local UI label; unknown values keep the
  * existing engine translator. No state, validation or permission is inferred. */
 function stateLabel(value){const known={accepted:'Acceptée',blocked:'Bloquée',todo:'À faire',running:'En cours',submitted:'À vérifier',stale:'À vérifier',waived:'Dérogation',abandoned:'Abandonnée',failed:'Échec'};return Object.hasOwn(known,value)?t(known[value]):engine(value)}
 /** Build a detached text-only status badge for an exact recorded state and
  * optional display label. Tone depends only on state; unknown states stay
  * neutral and missing labels explicit. No DOM insertion, writes or authority;
  * DOM/translation errors propagate to the rendering caller. */
 function statusBadge(state,label=state){const badge=el('span',label?stateLabel(label):t('État non disponible.'),'workbench-badge '+signal(state));badge.dataset.taskState=state??'';return badge}
 /** Build a detached badge from an exact recorded agent state. Unlike task
  * acceptance, completed means only that the attempt ended. Unconfirmed and
  * interrupted observations warn; unknown codes remain neutral literal text.
  * No liveness probe, task validation, network or write; DOM errors propagate. */
 function agentStatusBadge(state){
  const names={queued:'En attente',starting:'Démarrage',running:'En cours',stopping:'Arrêt demandé',completed:'Terminé',failed:'Échec',interrupted:'Interrompu','queued/unconfirmed':'En attente — processus non confirmé','starting/unconfirmed':'Démarrage — processus non confirmé','running/unconfirmed':'En cours — processus non confirmé','unknown/other-host':'Inconnu — autre poste','unknown/supervisor-lost':'Inconnu — superviseur perdu'};
  const tone=state==='completed'?'success':state==='failed'?'alert':['queued','starting','running'].includes(state)?'info':['stopping','interrupted','queued/unconfirmed','starting/unconfirmed','running/unconfirmed','unknown/other-host','unknown/supervisor-lost'].includes(state)?'warn':'neutral';
  const badge=el('span',Object.hasOwn(names,state)?t(names[state]):state?engine(state):t('État non disponible.'),'workbench-badge '+tone);badge.dataset.agentState=state??'';return badge;
 }
 /** Select an in-memory presentation. Storage contains only the enum, never Work
  * data; refused storage reports loss of persistence and does not block reading. */
 function choose(next,persist=true){view=next;const graph=document.getElementById('graph');if(graph)graph.hidden=view!=='graph';lanes.hidden=view!=='lanes';for(const [key,button]of Object.entries(buttons))button.setAttribute('aria-pressed',String(key===view));if(persist)try{localStorage.setItem(preference,view)}catch{feedback.textContent=t('Préférence d’affichage non mémorisée sur cet appareil.')}if(view==='graph')graph?.graphRestoreView?.()}
 /** Explain the next approval condition in user-facing text. The caller
  * displays the safe engine code separately for diagnosis. */
 function reviewReason(code){
  const labels={
   'native-scoped-submitted-review-required':t('Un avis indépendant d’origine est requis pour cette tâche planifiée.'),
   'gate-required':t('Une preuve de livraison validée est requise.'),
   'gate-known-stale':t('La preuve de livraison doit être requalifiée.'),
   'settings-quality-policy-not-configured':t('La politique qualité de livraison est indisponible.'),
   'task-review-revision-changed':t('La révision a changé. Actualisez la mission.'),
   revision_conflict:t('La révision a changé. Actualisez la mission.'),
   'task-validation-controller-active':t('Un contrôle de validation est actif ou reste à réconcilier.'),
   'task-active-agents':t('Un agent est actif sur cette tâche.'),
   'planning-closure-review-active':t('Le résultat de la revue indépendante est requis avant validation.'),
   'web-native-task-review-outcome-unknown':t('Enregistrement incertain. Actualisez la mission avant de réessayer.'),
   'web-native-task-review-readback-unconfirmed':t('La relecture de la validation a échoué. Actualisez la mission.'),
  };
  return Object.hasOwn(labels,code)?labels[code]:t('Actualisez la mission et consultez le diagnostic si le blocage persiste.');
 }
 /** Present human validation only in the open task dialog. Preview and commit
  * use the captured Work revision. Closing or repainting invalidates the
  * panel before a write, and the pending lock excludes concurrent sends.
  * Only a protected readback can change the accepted status in the workbench. */
 function appendTaskReview(task){
  const {work,taskReview}=model,dialog=taskDialog;
  if(!dialog?.open||!taskReview||task.status!=='submitted'||task.validation_policy?.mode!=='human')return;
  const panel=el('section',undefined,'workbench-review'),button=el('button',t('Valider'),'task-detail-approve');
  const status=el('p',t('Vérification des conditions…'),'hint');
  status.setAttribute('role','status');status.tabIndex=-1;
  button.type='button';button.disabled=true;
  const diagnostic=el('details',undefined,'task-detail-diagnostic');
  const codeLabel=el('code');
  diagnostic.append(el('summary',t('Diagnostic')),codeLabel);diagnostic.hidden=true;
  panel.append(button,status,diagnostic);taskDialogHeader.append(panel);
  const live=()=>taskDialog===dialog&&dialog.open&&panel.isConnected&&model?.work.id===work.id
   &&model.work.revision===work.revision&&selected===task.id;
  const showError=error=>{
   if(!live())return;
   status.className='notice warn';
   const code=error.code||'web-native-task-review-refused';
   status.textContent=reviewReason(code);
   codeLabel.textContent=code;diagnostic.hidden=false;
   if(document.activeElement===button)status.focus();
  };
  if(!Number.isSafeInteger(work.revision)||work.revision<1){
   showError({code:'task-review-revision-changed'});
   return;
  }
  taskReview.preview(work.id,task.id,work.revision).then(()=>{
   if(!live())return;
   status.textContent=t('Cette tâche attend votre validation.');
   diagnostic.hidden=true;
   button.disabled=false;
  }).catch(showError);
  button.addEventListener('click',async()=>{
   if(button.disabled||!live()||reviewInFlight)return;
   reviewInFlight=true;
   status.textContent=t('Enregistrement et relecture de la validation…');status.focus();
   button.disabled=true;
   try{await taskReview.accept(work.id,task.id,work.revision,live)}
   catch(error){showError(error)}
   finally{reviewInFlight=false}
  });
 }
 /** Own one wide task dialog and restore focus when it closes. The native
  * Work inspector stays in the cockpit. Dialog readers are disposed on rerender
  * and close so delayed responses cannot display another task's files. */
 function openSummaryTask(task,button){
  selected=task.id;
  paint();
  const dialog=el('dialog',undefined,'workbench-task-dialog');
  const title=el('h2',t('Détail de la tâche'));
  title.id='workbench-task-dialog-title';title.tabIndex=-1;
  dialog.setAttribute('aria-labelledby',title.id);
  const top=el('div',undefined,'task-detail-titlebar');
  const close=el('button',t('Fermer'));close.type='button';close.addEventListener('click',()=>dialog.close());
  top.append(title,close);
  taskDialogHeader=el('header',undefined,'task-detail-header');
  taskDialogFiles=el('div',undefined,'task-detail-files');
  dialog.append(top,taskDialogHeader,taskDialogFiles);document.body.append(dialog);
  taskDialog=dialog;returnToTaskButton=true;
  dialog.addEventListener('close',()=>{
   disposeTaskFiles?.();disposeTaskFiles=null;
   dialog.remove();
   if(taskDialog!==dialog)return;
   taskDialog=null;taskDialogSelection=null;
   if(returnToTaskButton){
    const target=button.isConnected?button:summary.querySelector('[data-summary="'+signal(stateOf(task))+'"] button')
     ||buttons[view];
    target?.focus();
   }
  },{once:true});
  dialog.showModal();paintTaskDialog(task);close.focus();
 }

 /** Supply local metadata plus independently reloadable originals and Logs
  * for the current Work revision. Keys resolve only through these readers.
  * Discovery errors surface in their own group without replacing a file. */
 function taskDocuments({work,taskReview},task){
  const files=[], readers=new Map(), query=new URLSearchParams({work:work.id,task:task.id});
  const add=(id,path,kind,read,readable=true)=>{
   const entry={id,path,kind,readable},index=files.findIndex(file=>file.id===id);
   if(index<0)files.push(entry);
   else files[index]=entry;
   if(readable)readers.set(id,read);
   else readers.delete(id);
  };
  const json=value=>({text:JSON.stringify(value,null,2),language:'json'});
  const technical=t('Détails techniques'),deliverables=t('Livrables'),logs=t('Journaux');
  const summary={id:task.id,revision:work.revision,title:task.title,status:task.status,
   deliverable:task.deliverable,criteria:task.criteria,owner:task.owner,depends:task.depends,next:task.next};
  add('summary',technical+'/'+t('Résumé')+'.json','json',()=>json(summary));
  if(task.automatic_validation||task.independent_review){
   add('review',technical+'/'+t('Validation')+'.json','json',()=>json({
    validation_policy:task.validation_policy?.mode,
    controls:task.automatic_validation?.controls,
    independent_review:task.independent_review,
   }));
  }
  if(taskReview){
   add('actions',technical+'/'+t('Actions')+'.json','json',async()=>json(
    await taskReview.read('/api/v1/agent-logs/task-actions?'+query,true)));
  }
  const addLog=agent=>{
   const id='log:'+agent;
   add(id,logs+'/'+agent+'.log','log',async()=>{
    const page=await taskReview.read('/api/v1/agent-logs?'+new URLSearchParams({work:work.id,agent,after:'0'}),true);
    if(page.work_id!==work.id||page.agent_id!==agent||!Array.isArray(page.rows))throw Error('task-detail-log-invalid');
    return {text:page.rows.map(row=>[row.at,row.kind,row.message].filter(part=>typeof part==='string').join(' · ')).join('\n'),
     truncated:page.rows.length===200};
   });
  };
  const producer=task.automatic_validation?.producer_agent_id;
  if(taskReview&&typeof producer==='string'&&producer)addLog(producer);
  const hasOriginals=Boolean(taskReview&&task.status==='submitted'&&task.automatic_validation?.receipt);
  const sources=[];
  if(hasOriginals)sources.push({label:deliverables,primary:true,load:async()=>{
   const data=await taskReview.original(work.id,task.id,work.revision,undefined,true);
   // The initial report belongs to this revision-scoped explorer only.
   // Opening it again, including retries, uses a new checked request.
   let initialReport=data.report;
   for(const file of data.files){
    add('original:'+file.id,deliverables+'/'+(file.id==='report'?t('Rapport remis')+'.md':file.path),file.kind,
     ()=>{
      if(file.id==='report'&&initialReport){
       const report=initialReport;
       initialReport=null;
       if(report.error)throw Object.assign(new Error(report.error),{code:report.error});
       return report;
      }
      return taskReview.original(work.id,task.id,work.revision,file.id);
     },file.readable);
   }
   return files.slice();
  }});
  if(taskReview)sources.push({label:logs,load:async()=>{
   const data=await taskReview.read('/api/v1/cockpit-read?'+new URLSearchParams({work:work.id}),true);
   if(data.work_id!==work.id||!Array.isArray(data.agents))throw Error('task-detail-agent-list-invalid');
   for(const agent of data.agents.filter(agent=>agent.work_id===work.id&&agent.task_id===task.id))addLog(agent.id);
   return files.slice();
  }});
  return {
   initialFiles:files.slice(),sources,groupOrder:[deliverables,logs,technical],technical,
   preferredId:'original:report',
   async read(id){
    const reader=readers.get(id);
    if(!reader)throw Error('task-review-file-unavailable');
    return reader();
   }
  };
 }
 /** Reveal the selected task in the existing graph. A dialog close defers
  * focus restoration to the graph target rather than the summary opener. */
 function taskGraphButton(task){
  const button=el('button',t('Voir dans le graphe'));
  button.type='button';
  button.addEventListener('click',()=>{
   if(taskDialog?.open){returnToTaskButton=false;taskDialog.close()}
   choose('graph');
   const graph=document.getElementById('graph');
   if(graph?.graphRevealTask){graph.graphRevealTask(task.id);return}
   const target=[...document.querySelectorAll('#graph [data-task]')].find(node=>node.dataset.task===task.id);
   target?.scrollIntoView({block:'nearest',inline:'center'});
   target?.querySelector('.graph-task-open')?.focus();
  });
  return button;
 }
 /** Paint the modal header and mount a revision-scoped, read-only file reader.
  * Recorded metadata is available immediately. Optional originals and Logs
  * loads use their own protected requests, so they cannot hide approval status. */
 function paintTaskDialog(task){
  if(!taskDialog?.open)return;
  const next=[model.work.id,model.work.revision,task.id];
  if(taskDialogSelection?.every((item,index)=>item===next[index]))return;
  taskDialogSelection=next;
  disposeTaskFiles?.();disposeTaskFiles=null;
  taskDialogHeader.replaceChildren();taskDialogFiles.replaceChildren();
  const top=el('div',undefined,'task-detail-task');
  top.append(el('h3',task.title||task.id),statusBadge(task.status));
  const metadata=el('details',undefined,'task-detail-metadata');
  metadata.append(el('summary',t('Informations sur la tâche')),el('p',task.id,'hint'));
  if(task.deliverable)metadata.append(deliverableText(task.deliverable));
  metadata.append(taskGraphButton(task));top.append(metadata);
  taskDialogHeader.append(top);
  appendTaskReview(task);
  if(!taskDialogHeader.querySelector('.workbench-review')){
   const message=task.status==='accepted'?'Validation enregistrée.':task.status!=='submitted'
    ?'La tâche doit être soumise avant validation.':task.validation_policy?.mode==='human'
     ?'La validation est indisponible dans cette vue.':'La tâche ne relève pas de la validation humaine.';
   const panel=el('section',undefined,'workbench-review');
   panel.append(el('p',t(message),'hint'));
   if(task.status!=='accepted'){
    const button=el('button',t('Valider'),'task-detail-approve');
    button.type='button';button.disabled=true;panel.append(button);
   }
   taskDialogHeader.append(panel);
  }
  if(!globalThis.SwarmTaskDetailFiles){
   taskDialogFiles.append(el('p',t('Le lecteur de pièces est indisponible.'),'notice warn'));
   return;
  }
  try{disposeTaskFiles=globalThis.SwarmTaskDetailFiles.mount(taskDialogFiles,taskDocuments(model,task))}
  catch{taskDialogFiles.replaceChildren(el('p',t('Le lecteur de pièces est indisponible.'),'notice warn'))}
 }
 /** Render the selected recorded task; dependencies come from Work, observations
  * from its matching projection. For a submitted human-mode task the open
  * dialog attaches the protected review action. Rendering only observes it. */
 function inspect(){detail.replaceChildren();const task=model?.work.tasks.find(task=>task.id===selected);if(!task){detail.append(el('h3',t('Choisissez une tâche')),el('p',t('Sélectionnez une fiche pour consulter son état et ses dépendances.')));return}const projection=model.projections.get(task.id),heading=el('h3',task.title||task.id);heading.tabIndex=-1;detail.append(el('p',task.id,'eyebrow'),heading);const state=projection?.label||projection?.state||task.status;detail.append(statusBadge(projection?.state||task.status,state));
  const facts=el('dl',undefined,'native-facts');for(const [label,value]of [[t('Rôle'),task.plan_role||task.owner||t('Non attribué')],[t('Dépend de'),task.depends?.join(', ')||t('Aucune dépendance enregistrée')],[t('Débloque'),model.work.tasks.filter(item=>item.depends?.includes(task.id)).map(item=>item.id).join(', ')||'—']])facts.append(el('dt',label),el('dd',value));detail.append(facts);if(projection?.reason)detail.append(el('p',engine(projection.reason),'notice warn'));if(task.deliverable)detail.append(deliverableText(task.deliverable));detail.append(taskGraphButton(task));if(taskDialog?.open)paintTaskDialog(task);
 }
 /** Rebuild role × recorded-state lanes from the current model. Search, role
  * and state-chip filters are local; empty matches and unavailable observations stay explicit. */
 function paint(){if(!model)return;const grid=lanes.querySelector('.workbench-matrix');grid.replaceChildren();const all=model.work.tasks,shown=all.filter(task=>inFilter(task)&&(!role||(task.plan_role||task.owner||'')===role)&&[task.id,task.title,task.plan_role,task.owner].filter(Boolean).join(' ').toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  if(!shown.length){grid.append(el('p',t(all.length?'Aucune tâche ne correspond à votre recherche.':'Aucune tâche enregistrée.')));inspect();return}
  const stateOf=task=>{const p=model.projections.get(task.id);return p?.state||task.status||''};const states=[...new Set(shown.map(stateOf))],roles=[...new Set(shown.map(task=>task.plan_role||task.owner||''))];
  const table=el('table',undefined,'workbench-table'),caption=el('caption',t('Rôles et états enregistrés'));table.append(caption);const head=el('thead'),row=el('tr');row.append(el('th',t('Rôle')));for(const state of states){const sample=shown.find(task=>stateOf(task)===state),label=model.projections.get(sample?.id)?.label||state;const th=el('th',label?stateLabel(label):t('État non disponible.'));th.scope='col';row.append(th)}head.append(row);table.append(head);const body=el('tbody');
  for(const lane of roles){const tr=el('tr'),label=el('th',lane||t('Non attribué'));label.scope='row';tr.append(label);for(const state of states){const cell=el('td');for(const task of shown.filter(task=>(task.plan_role||task.owner||'')===lane&&stateOf(task)===state)){const p=model.projections.get(task.id),button=el('button',undefined,'workbench-task');button.type='button';button.dataset.laneTask=task.id;button.setAttribute('aria-pressed',String(task.id===selected));button.append(el('span',task.id,'eyebrow'),el('strong',task.title||task.id),statusBadge(state,p?.label||state));button.addEventListener('click',()=>{selected=task.id;for(const peer of grid.querySelectorAll('[data-lane-task]'))peer.setAttribute('aria-pressed',String(peer.dataset.laneTask===selected));inspect()});cell.append(button)}tr.append(cell)}body.append(tr)}table.append(body);grid.append(table);inspect();
 }

 /** Recorded state of one task: matching projection state first, else the Work
  * status. Pure read of the current model; infers no activity or validation. */
 function stateOf(task){const p=model?.projections.get(task.id);return p?.state||task.status||''}
 /** Paint the recorded review, blocked and accepted counts. The summary
  * action opens the corresponding task inspector in a dialog and leaves the
  * current view selected. No engine operation or mutation is performed. */
 function summarize(){if(!summary)return;summary.replaceChildren();const tasks=model?.work.tasks||[],known=!!model;
  const pick=tone=>tasks.filter(task=>signal(stateOf(task))===tone);
  for(const [tone,title]of [['warn','À vérifier'],['alert','Bloquées']]){const matches=pick(tone),card=el('article',undefined,'workbench-summary-card '+tone);card.dataset.summary=tone;const count=el('strong',known?String(matches.length):'—','workbench-count '+tone),text=el('div');text.append(el('h3',t(title)),el('p',known?(matches.length?(matches[0].title||matches[0].id):t('Aucune tâche dans cet état.')):t('Aucune mission sélectionnée.')));card.append(count,text);
   if(known&&matches.length){const see=el('button',t('Voir'));see.type='button';see.addEventListener('click',()=>openSummaryTask(matches[0],see));card.append(see)}summary.append(card)}
  const done=pick('success').length,card=el('article',undefined,'workbench-summary-card progress');card.dataset.summary='progress';const text=el('div');text.append(el('h3',t('Avancement')));const ratio=el('p',undefined,'workbench-ratio');ratio.append(el('strong',known?`${done} / ${tasks.length}`:'—'),el('span',' '+t('tâches acceptées')));text.append(ratio);const bar=el('progress');bar.max=Math.max(tasks.length,1);bar.value=known?done:0;bar.setAttribute('aria-label',t('Avancement'));text.append(bar);card.append(text);summary.append(card)}

 /** State filter groups shared by lanes and graph. Keys are local UI enums;
  * membership derives only from the recorded-state signal, never from titles. */
 const filterGroups=[['','Toutes',[]],['act','À traiter',['warn','alert']],['info','En cours',['info']],['success','Acceptées',['success']]];
 /** True when a task belongs to the active filter (always true for "Toutes"). */
 function inFilter(task){const group=filterGroups.find(([key])=>key===filter);return!filter||!!group&&group[2].includes(signal(stateOf(task)))}
 /** Dim graph cards outside the active filter. Visual only: cards stay in the
  * layout, focusable and openable; no Work, layout or preference is written. */
 function dimGraph(){const tasks=new Map((model?.work.tasks||[]).map(task=>[task.id,task]));for(const node of document.querySelectorAll('#graph .graph-noeud[data-task]')){const task=tasks.get(node.dataset.task);node.classList.toggle('graph-state-dim',!!filter&&!!task&&!inFilter(task))}}
 /** Repaint chips with per-group counts and pressed state; "—" without a model.
  * Selecting a chip filters lanes and dims graph cards; it starts nothing. */
 function paintFilters(){if(!filters)return;filters.replaceChildren();const tasks=model?.work.tasks||[];for(const [key,label,tones]of filterGroups){const chip=el('button',undefined,'workbench-chip');chip.type='button';chip.dataset.filter=key;chip.setAttribute('aria-pressed',String(key===filter));const n=model?(key?tasks.filter(task=>tones.includes(signal(stateOf(task)))).length:tasks.length):'—';chip.append(el('span',t(label)),el('span',String(n),'workbench-chip-count'));chip.addEventListener('click',()=>{filter=key;paintFilters();paint();dimGraph()});filters.append(chip)}}

 /** Show a graph card in the shared inspector when it receives focus (keyboard
  * or pointer). Ignores ids absent from the validated model and unchanged
  * selections; the card's own open/edit behaviour is untouched. Local only. */
 function follow(id){if(taskDialog?.open||!id||id===selected||!model?.work.tasks.some(task=>task.id===id))return;selected=id;for(const peer of lanes.querySelectorAll('[data-lane-task]'))peer.setAttribute('aria-pressed',String(peer.dataset.laneTask===selected));inspect()}

 /** Group the secondary Conduite accordions (assistant, work state, inbox,
  * activity feed) under one closed "Détails de la mission" disclosure, as on
  * the board. Elements are moved, not copied: their ids, listeners and
  * producers are unchanged. Missing elements are skipped. */
 function groupDetails(){const section=document.getElementById('conduite');if(!section||section.querySelector('#conduite-more'))return;const parts=[document.getElementById('assistant'),document.getElementById('conduite-overview'),document.getElementById('conduite-inbox-section'),section.querySelector('.conduite-fil')].filter(Boolean);if(!parts.length)return;const more=el('details',undefined,'workbench-more');more.id='conduite-more';more.append(el('summary',t('Détails de la mission')));parts[0].before(more);more.append(...parts)}
 /** Paint the rail "selected mission" card from the validated Work (title and
  * id as text only); hidden without a model. The select stays the control. */
 function paintMissionCard(){if(!missionCard)return;missionCard.replaceChildren();if(!model){missionCard.hidden=true;return}missionCard.hidden=false;missionCard.append(el('span',t('Mission sélectionnée'),'rail-mission-label'),el('strong',model.work.title||model.work.id),el('code',model.work.id))}
 /** Mount once into the canonical cockpit. A missing host is a harmless non-
  * cockpit page. The component owns only its controls, lanes and inspector;
  * it moves #graph into a stage whose side inspector serves graph and lanes. */
 function mount(){if(mounted)return;host=document.querySelector('#conduite .conduite-principal');if(!host)return;mounted=true;document.body.classList.add('workbench-cockpit');groupDetails();const workSelect=document.getElementById('work');if(workSelect){missionCard=el('div',undefined,'rail-mission-card');missionCard.hidden=true;missionCard.setAttribute('aria-live','polite');workSelect.after(missionCard)}const top=el('div',undefined,'workbench-view-switch');top.setAttribute('role','group');top.setAttribute('aria-label',t('Vue de la conduite'));buttons={};for(const [key,label]of [['graph','Graphe'],['lanes','Couloirs']]){const button=el('button',t(label));button.type='button';button.dataset.workbenchView=key;button.addEventListener('click',()=>choose(key));buttons[key]=button;top.append(button)}feedback=el('p',undefined,'hint');feedback.setAttribute('role','status');host.querySelector('.graph-heading')?.append(top);summary=el('section',undefined,'workbench-summary');summary.setAttribute('aria-label',t('Synthèse des états enregistrés'));host.querySelector('.graph-heading')?.before(summary);host.querySelector('.graph-heading')?.after(feedback);filters=el('div',undefined,'workbench-filters');filters.setAttribute('role','group');filters.setAttribute('aria-label',t('Filtrer par état enregistré'));feedback.after(filters);const graphHost=document.getElementById('graph');if(graphHost&&typeof MutationObserver==='function'){observer=new MutationObserver(()=>{if(filter)dimGraph()});observer.observe(graphHost,{childList:true,subtree:true})}
  lanes=el('section',undefined,'workbench-lanes');lanes.id='workbench-lanes';lanes.setAttribute('aria-label',t('Couloirs de la mission'));const toolbar=el('div',undefined,'toolbar'),searchLabel=el('label',t('Rechercher une tâche'));search=el('input');search.type='search';search.id='workbench-search';search.addEventListener('input',()=>{query=search.value;paint()});searchLabel.append(search);const roleLabel=el('label',t('Rôle'));roleSelect=el('select');roleSelect.id='workbench-role';roleSelect.addEventListener('change',()=>{role=roleSelect.value;paint()});roleLabel.append(roleSelect);toolbar.append(searchLabel,roleLabel);const content=el('div',undefined,'workbench-lanes-content'),matrix=el('div',undefined,'workbench-matrix');matrix.tabIndex=0;matrix.setAttribute('aria-label',t('Rôles et états enregistrés'));detail=el('aside',undefined,'workbench-inspector');detail.setAttribute('aria-label',t('Détail de la tâche'));content.append(matrix);lanes.append(toolbar,content);const graphEl=document.getElementById('graph'),stage=el('div',undefined,'workbench-stage'),column=el('div',undefined,'workbench-stage-main');graphEl.before(stage);column.append(graphEl,lanes);stage.append(column,detail);graphEl.addEventListener('focusin',event=>follow(event.target?.closest?.('[data-task]')?.dataset.task));try{const saved=localStorage.getItem(preference);if(['graph','lanes'].includes(saved))view=saved}catch{feedback.textContent=t('Préférence d’affichage non mémorisée sur cet appareil.')}choose(view,false);clear(t('Aucune mission sélectionnée.'));
 }
 /** Validate a new read model, rejecting cross-Work or duplicate records before
  * display. Changed Work clears selection/filter. Does not turn read errors into
  * success: a missing/failed projection keeps the visible recorded-only notice.
  * Repaints the recorded-state summary cards from the same validated model. */
 function render(data){mount();if(!mounted)return;try{const work=data?.work;if(!work||typeof work.id!=='string'||!Array.isArray(work.tasks)||!work.tasks.every(task=>task&&typeof task.id==='string'&&typeof task.title==='string'&&(task.depends===undefined||task.depends===null||Array.isArray(task.depends)&&task.depends.every(id=>typeof id==='string')))||new Set(work.tasks.map(task=>task.id)).size!==work.tasks.length)throw Error('workbench-work-invalid');if(work.id!==workId){selected='';query='';role='';filter='';search.value='';workId=work.id}const projections=new Map();if(!data.missionError&&Array.isArray(data.mission?.tasks)){for(const p of data.mission.tasks){if(!p||typeof p.id!=='string'||!work.tasks.some(task=>task.id===p.id)||projections.has(p.id))throw Error('workbench-projection-invalid');projections.set(p.id,p)}}model={work,projections,taskReview:data.taskReview};if(!work.tasks.some(task=>task.id===selected))selected=work.tasks[0]?.id||'';roleSelect.replaceChildren(new Option(t('Tous les rôles'),''));for(const name of new Set(work.tasks.map(task=>task.plan_role||task.owner||'').filter(Boolean)))roleSelect.append(new Option(name,name));roleSelect.value=role;paint();summarize();paintFilters();dimGraph();paintMissionCard();lanes.dataset.work=work.id;lanes.dataset.observed=String(!data.missionError&&!!data.mission);if(data.missionError)feedback.textContent=t('Suivi détaillé indisponible. Les fiches affichent les données enregistrées.');else feedback.textContent=t('Les couloirs présentent les rôles et états reçus, sans autoriser une action.');}catch(error){clear(t('Lecture des couloirs impossible. Actualisez pour reprendre.')+' ('+error.message+')');return false}return true}
 /** Clear the old mission immediately, including inspector and filters. Called
  * before new reads; never exposes previous-Work data while loading fails.
  * Summary cards return to "—" with the lanes. */
 function clear(message){if(!mounted)return;if(taskDialog?.open){returnToTaskButton=false;taskDialog.close()}model=null;selected='';workId='';query='';role='';search.value='';roleSelect.replaceChildren(new Option(t('Tous les rôles'),''));lanes.querySelector('.workbench-matrix').replaceChildren(el('p',message));detail.replaceChildren();delete lanes.dataset.work;filter='';summarize();paintFilters();dimGraph();paintMissionCard()}
 globalThis.SwarmWorkbench=Object.freeze({mount,render,clear,statusBadge,agentStatusBadge});
})();
