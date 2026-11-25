'use strict';
/** Read adapter for the existing product cockpit. Reuses its canonical DOM and
 * shared presentation shell; does not emulate a Go snapshot. Budget edits use their separate explicit dialog.
 * Reads are explicit, with no polling or invented browser wait policy. Actual
 * server admission/transport policies remain authoritative. Unsupported legacy
 * commands and automatic visit writes stay unavailable, visibly, in this profile. */
(()=>{
 const $=id=>document.getElementById(id),tr=s=>globalThis.SwarmI18n?.t(s)??s;
 let works=[],selected='',generation=0,current=null,budgetEditor=null,resumePanel=null,logsPanel=null,instructions=null;const views=new Set([...document.querySelectorAll('#tabs [data-view]')].map(b=>b.dataset.view));
 /** Only safe error families reach the UI; original causes remain on rejection.
  * A transport/JSON/HTTP failure never becomes an empty successful document. */
 function failure(code,cause){const error=new Error(code,{cause});error.code=code;return error}
 /** One same-origin, cookie-protected GET. No CSRF mutation, auto retry, provider,
  * local cap or deadline is added. Detailed readers keep the dedicated worker
  * or authority refusal code after an exact text match. Other bodies stay generic. */
 async function read(path,detailed=false){let response;try{response=await fetch(path,{credentials:'same-origin',headers:{Accept:'application/json'}})}catch(cause){throw failure('web-native-response-unavailable',cause)}
  if(detailed&&response.status===503&&/^text\/plain(?:;|$)/i.test(response.headers.get('content-type')||'')){
   const reason=await response.text().catch(()=>null);
   throw failure(reason==='web-workers-saturated'?reason:'web-native-service-unavailable');
  }
  if(detailed&&response.status===502&&/^text\/plain(?:;|$)/i.test(response.headers.get('content-type')||'')){
   const reason=await response.text().catch(()=>null);
   throw failure(['authority-engine-unavailable','authority-engine-handshake'].includes(reason)
    ?reason:'web-native-service-unavailable');
  }
  let value;try{value=await response.json()}catch(cause){throw failure(response.status===403?'web-access-refused':response.status===503?'web-native-service-unavailable':'web-native-response-invalid',cause)}
  if(!response.ok)throw responseFailure(value,'web-native-request-refused',detailed);
  return value;
 }
 /** Render only a static engine reason or error family from a failed response. */
 function responseFailure(value,fallback,detailed=true){
  const code=[detailed?value?.failure?.reason:null,value?.failure?.code,value?.error]
   .find(code=>typeof code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(code));
  return failure(code||fallback);
 }
 /** Observe one protected, revision-matched acceptance precheck. A refused
  * condition surfaces as an engine code. The observation grants no later write. */
 async function previewTaskReview(work,task,revision){
  const value=await read('/api/v1/action?'+new URLSearchParams({work,task,revision}),true);
  if(value?.schema_version!==1||value.work!==work||value.task!==task||value.revision!==revision||value.eligible!==true)
   throw failure('web-native-task-review-preview-invalid');
  return value;
 }
 /** Submit the exact observed task/revision through the existing mutation
  * route, then independently read Work. The UI context is checked after CSRF
  * and before POST. A lost response or readback never triggers another write. */
 async function acceptTaskReview(id,task,revision,isCurrent){
  const ticket=generation;
  const currentIntent=()=>ticket===generation&&selected===id&&works.find(work=>work.id===id)?.revision===revision&&isCurrent();
  if(!currentIntent())throw failure('task-review-revision-changed');
  const session=await read('/api/v1/native-session');
  if(typeof session?.csrf_token!=='string'||!/^[0-9a-f]{64}$/.test(session.csrf_token))throw failure('web-native-session-invalid');
  if(!currentIntent())throw failure('task-review-revision-changed');
  let response,value;
  try{
   response=await fetch('/api/v1/action',{method:'POST',credentials:'same-origin',
    headers:{Accept:'application/json','Content-Type':'application/json','X-Swarm-CSRF':session.csrf_token},
    body:JSON.stringify({kind:'task',work:id,task,event_id:'task-review-'+crypto.randomUUID(),
     expected_revision:revision,request:{status:'accepted'}})});
   value=await response.json();
  }catch(cause){throw failure('web-native-task-review-outcome-unknown',cause)}
  if(!response.ok){
   if(value?.recorded_work||response.status>=500||['command-commit-uncertain','command-transaction-lost','command-rollback']
    .includes(value?.failure?.code))throw failure('web-native-task-review-outcome-unknown');
   throw responseFailure(value,'web-native-task-review-refused');
  }
  const accepted=work=>work?.id===id&&Array.isArray(work.tasks)&&work.tasks.some(item=>item.id===task&&item.status==='accepted');
  if(!accepted(value)||value.revision!==revision+1)throw failure('web-native-task-review-outcome-unknown');
  const observed=await read('/api/v1/planning?'+new URLSearchParams({work:id}))
   .catch(cause=>{throw failure('web-native-task-review-readback-unconfirmed',cause)});
  if(!accepted(observed)||!Number.isSafeInteger(observed.revision)||observed.revision<value.revision)
   throw failure('web-native-task-review-readback-unconfirmed');
  works=works.map(work=>work.id===id&&work.revision<=observed.revision?observed:work);
  if(ticket===generation&&selected===id){
   await select(id,true);
   document.querySelector('.workbench-task-dialog[open] h2')?.focus();
  }
  return observed;
 }
 /** Read one revision-bound original or the task's protected originals list,
  * optionally including the initial report or its explicit per-file error.
  * Server checks ownership and hashes. Explicit fileTiming=1 requests the
  * server's elapsed phases on named files only. */
 async function readTaskOriginal(work,task,revision,file,includeReport=false){
  const query=new URLSearchParams({work,task,revision});
  if(file!==undefined)query.set('file',file);
  else if(includeReport)query.set('include_report','1');
  const profiling=file!==undefined&&new URLSearchParams(location.search).get('fileTiming')==='1';
  if(profiling)query.set('timing','1');
  const value=await read('/api/v1/agent-logs/task-review-files?'+query,true);
  if(value?.schema_version!==1||value.work!==work||value.task!==task||value.revision!==revision
   ||!value.data||(file===undefined?!Array.isArray(value.data.files):value.data.id!==file))
   throw failure('task-review-files-response-invalid');
  if(includeReport){
   const report=value.data.report;
   const hasText=typeof report?.text==='string';
   const hasError=typeof report?.error==='string'&&/^[a-z][a-z0-9_.-]*$/.test(report.error);
   if(report?.id!=='report'||hasText===hasError)throw failure('task-review-files-response-invalid');
  }
  if(profiling){
   const timing=value.timing_ms;
   const phases=['engine','checks_and_preparation','work_snapshot','receipt','submission','git'];
   if(!timing||!phases.every(key=>Number.isFinite(timing[key])&&timing[key]>=0))
    throw failure('task-review-files-response-invalid');
   const receiptPhases=timing.receipt_phases;
   const receiptKeys=['receipt_decode','receipt_originals','receipt_publication','receipt_recheck'];
   if(receiptPhases&&!receiptKeys.every(key=>Number.isFinite(receiptPhases[key])&&receiptPhases[key]>=0))
    throw failure('task-review-files-response-invalid');
   const verified=Object.fromEntries(phases.map(key=>[key,timing[key]]));
   if(receiptPhases)verified.receipt_phases=Object.fromEntries(receiptKeys.map(key=>[key,receiptPhases[key]]));
   return {...value.data,timing:verified};
  }
  return value.data;
 }
 const taskReview=Object.freeze({preview:previewTaskReview,accept:acceptTaskReview,original:readTaskOriginal,read});
 /** Read safe engine codes for this Work and the observed control revision.
  * Reject missing, stale or malformed data and propagate safe transport codes. */
 async function readLaunchDiagnostics(id, control){
  const value=await read('/api/v1/mission?'+new URLSearchParams({work:id,action:'launch-diagnostics'}));
  if(value?.work!==id||!Number.isSafeInteger(value.revision)||!Array.isArray(value.refusals)||
    !value.refusals.every(r=>r&&(r.task===null||typeof r.task==='string')&&
      Array.isArray(r.codes)&&r.codes.length>0&&r.codes.every(code=>typeof code==='string'&&/^[a-zA-Z0-9:_-]+$/.test(code))))
    throw failure('web-native-launch-diagnostics-invalid');
  if(value.revision!==control?.revision)throw failure('revision_conflict');
  return value;
 }
 /** Preserve language and theme on actual supported destinations. Only trusted
  * relative paths are used, including their local fragment; Work identifiers
  * always become query data. */
 function url(path){const u=new URL(path,location.href);u.searchParams.set('lang',globalThis.SwarmI18n?.language==='en'?'en':'fr');u.searchParams.set('theme',document.documentElement.dataset.theme);return u.pathname+u.search+u.hash}
 /** Reuse original sections/navigation. Unconnected sections remain reachable,
  * with visible limitations and disabled commands, never silently empty success. */
 function showView(name,focus=false){if(!views.has(name))name='conduite';
  // ADM-04: a direct view access without the menu right stays refused with a
  // clear message; the API answers 403 on its own — the UI grants nothing.
  if(globalThis.SwarmAuthorityMenu&&!globalThis.SwarmAuthorityMenu.visibleView(name)){
   $('message').hidden=false;$('message').className='notice alert';$('message').textContent=globalThis.SwarmAuthorityMenu.deniedMessage();
   return;
  }
  for(const section of document.querySelectorAll('.view'))section.hidden=section.id!==name;for(const button of document.querySelectorAll('#tabs [data-view]')){if(button.dataset.view===name)button.setAttribute('aria-current','page');else button.removeAttribute('aria-current')}
  $('assistant').hidden=true;globalThis.SwarmShell?.navigate(name);const u=new URL(location.href);u.searchParams.set('view',name);history.replaceState(null,'',u);if(focus)$('main-content').focus();if(name==='manage')renderManager();if(name==='resume')resumePanel?.refresh();if(name==='logs')logsPanel?.refresh();if(name!=='logs'&&current?.readMode!=='history'&&current?.statusObserved===false)select(selected);
 }
 /** Filter the original mission manager with actual Work rows. No lifecycle
  * state is inferred from this list, so only the all-missions filter is enabled. */
 function renderManager(){const query=$('manage-search').value.trim().toLocaleLowerCase(),list=$('manage-list');list.replaceChildren();const shown=works.filter(w=>[w.id,w.title,w.objective].some(s=>s.toLocaleLowerCase().includes(query)));
  for(const work of shown){const button=document.createElement('button');button.type='button';button.textContent=work.title+' · '+work.id;button.addEventListener('click',()=>{$('work').value=work.id;select(work.id);showView('conduite',true)});list.append(button)}
  $('manage-status').textContent=tr(shown.length?'Choisissez une mission pour consulter ses tâches.':'Aucune mission ne correspond à votre recherche.');
 }
 /** Read mission/control/panel observations for an explicit Work, and notify the
  * separately observed lazy resume/log panels. Logs do not request calculated
  * mission status; opening another view observes it then, with its refusal intact. A generation
  * prevents delayed responses replacing newer selection. Errors are visible and
  * data is read-only; controls do not establish conductor/process liveness.
  * An empty/missing selection clears the owned graph modal and view memory;
  * another Work cannot inherit that modal or its pending presentation. A confirmed
  * acceptance can keep the same Work's inspector open while reloading panels. */
 async function select(id,preserveTaskDetail=false){const ticket=++generation,changedContext=selected!==id;selected=id;resumePanel?.setWork(id);logsPanel?.setWork(id);const work=works.find(w=>w.id===id);budgetEditor?.setWork(work?id:'');current=null;if(!preserveTaskDetail||changedContext)globalThis.SwarmWorkbench?.clear(tr(id?'Lecture de la mission…':'Aucune mission sélectionnée.'));if(instructions){if(changedContext)SwarmNativeInstructions.clear(instructions);else instructions.replaceChildren()}SwarmRustCockpitPanels.clear(tr(id?'Lecture de la mission…':'Aucune mission sélectionnée.'));const u=new URL(location.href);if(id)u.searchParams.set('work',id);else u.searchParams.delete('work');history.replaceState(null,'',u);
  if(!work){$('title').textContent=tr('Choisissez une mission');$('objective').textContent='';for(const id of ['task-count','accepted-count','active-count','decision-count'])$(id).textContent='—';for(const id of ['tasks-body','conduite-plan'])$(id).replaceChildren();SwarmRustDependencyGraph.clear(tr('Aucune mission sélectionnée.'));$('main-content').removeAttribute('aria-busy');$('connection').textContent=tr('Aucune mission sélectionnée.');return}
  $('title').textContent=work.title;$('objective').textContent=work.objective;$('connection').textContent=tr('Lecture de la mission…');$('main-content').setAttribute('aria-busy','true');
  // Clear the previous mission before starting a potentially failing read.
  if(!preserveTaskDetail||changedContext)SwarmRustCockpitRead.render({work,mission:null,missionError:'web-native-read-pending',control:null,taskReview});
  let control=null,mission=null,error=null,panelsObserved=false,readMode='current';let statusObserved=false,launchDiagnostics=null,launchDiagnosticError=null;
  try{control=await read('/api/v1/mission?'+new URLSearchParams({work:id}));if(!control||!Number.isSafeInteger(control.revision)||typeof control.archived!=='boolean')throw failure('web-native-control-invalid')}catch(cause){error=cause.code||'web-native-control-invalid'}
  if(ticket!==generation)return;
  // A saved history read consumes public records, not execution qualification.
  // Paused missions default to that view; an explicit URL/button chooses either.
  readMode=new URL(location.href).searchParams.get('read')||(control?.paused===true?'history':'current');
  if(!['history','current'].includes(readMode))readMode='current';
  const switcher=$('mission-read-mode');switcher.textContent=tr(readMode==='history'?'Vérifier le statut actuel':'Lire l’historique enregistré');
  statusObserved=readMode==='current'&&$('logs').hidden;
  if(statusObserved){
  try{const value=await read('/api/v1/mission?'+new URLSearchParams({work:id,action:'status'}));if(!value||!Object.hasOwn(value,'tasks')||typeof value.summary!=='string'||typeof value.next!=='string'||!(value.tasks===null||Array.isArray(value.tasks)&&value.tasks.every(t=>t&&typeof t.id==='string')))throw failure('web-native-status-invalid');mission=value}catch(cause){error=cause.code||'web-native-status-invalid'}
  }else if(readMode!=='history')error='web-native-status-not-observed';
  if(ticket!==generation)return;
  if(statusObserved){
   try{launchDiagnostics=await readLaunchDiagnostics(id,control)}
   catch(cause){launchDiagnosticError=cause.code||'web-native-launch-diagnostics-invalid'}
  }
  if(ticket!==generation)return;
  try{const observation=await read('/api/v1/cockpit-read?'+new URLSearchParams({work:id}));if(ticket!==generation)return;if(observation?.work_id!==id)throw failure('web-native-observation-invalid');SwarmRustCockpitPanels.render(observation);panelsObserved=true}catch(cause){if(ticket!==generation)return;error=cause.code||'web-native-observation-invalid';SwarmRustCockpitPanels.clear(tr('Lecture impossible. Actualisez pour reprendre.')+' ('+error+')')}
  current={work,mission,missionError:error,control,panelsObserved,statusObserved,readMode,launchDiagnostics,launchDiagnosticError,taskReview};SwarmNativeInstructions.render(instructions,work,rejectInstruction,true);SwarmRustCockpitRead.render(current);$('main-content').removeAttribute('aria-busy');$('connection').textContent=tr(error||launchDiagnosticError?'Lecture partielle — actualisez pour reprendre.':readMode==='history'?'Historique reçu — statut opérationnel non évalué.':'Lecture reçue — actualisation manuelle.');
 }
 /** Reject only the explicitly selected, observed instruction revision. CSRF
  * is freshly read; the server owns authorization/CAS/active-task checks. Lost
  * responses are uncertain and never retried. Success requires a protected Work
  * readback showing the rejection before the cockpit is refreshed. A concurrent
  * refresh of the same Work cannot replace this readback with an older cache;
  * another Work or a newer revision is preserved. */
 async function rejectInstruction(id,revision,task,addition,reason){
  const ticket=generation;if(selected!==id||current?.work.revision!==revision)throw failure('web-native-instruction-context-changed');
  const session=await read('/api/v1/native-session');if(typeof session?.csrf_token!=='string'||!/^[0-9a-f]{64}$/.test(session.csrf_token))throw failure('web-native-session-invalid');
  if(ticket!==generation||selected!==id)throw failure('web-native-instruction-context-changed');
  let response,value;try{response=await fetch('/api/v1/planning?'+new URLSearchParams({work:id,action:'reject-instructions'}),{method:'POST',credentials:'same-origin',headers:{Accept:'application/json','Content-Type':'application/json','X-Swarm-CSRF':session.csrf_token},body:JSON.stringify({schema_version:1,event_id:'instruction-reject-'+crypto.randomUUID(),expected_revision:revision,task_id:task,instruction_revision_id:addition,reason})});value=await response.json()}catch(cause){throw failure('web-native-instruction-response-unknown',cause)}
  if(!response.ok){const raw=value?.failure?.code||value?.error;const cause=failure(typeof raw==='string'&&/^[a-z][a-z0-9_.-]*$/.test(raw)?raw:'web-native-instruction-refused');cause.outcome=!value?.recorded_work&&[400,403,409].includes(response.status)?'refused':'unknown';throw cause}
  if(value?.id!==id||value.revision!==revision+1)throw failure('web-native-instruction-response-invalid');
  const observed=await read('/api/v1/planning?'+new URLSearchParams({work:id}));const changed=observed?.tasks?.find(t=>t.id===task)?.instruction_revisions?.find(r=>r.id===addition);
  if(observed?.id!==id||!Number.isSafeInteger(observed.revision)||observed.revision<value.revision||changed?.state!=='rejected')throw failure('web-native-instruction-readback-unconfirmed');
  // A same-Work refresh can advance generation while this POST is in flight.
  // The acknowledged readback supersedes older cached data; a newer observation
  // is retained. Selecting again also invalidates an older unfinished load.
  works=works.map(w=>w.id===id&&(!Number.isSafeInteger(w.revision)||w.revision<=observed.revision)?observed:w);
  if(selected===id&&(!current||current.work.id!==id||current.work.revision<=observed.revision)){
   await select(id);
   if(selected===id&&(current?.work.id!==id||!Number.isSafeInteger(current.work.revision)||current.work.revision<observed.revision))throw failure('web-native-instruction-cockpit-refresh-failed');
  }
  return selected===id&&current?.work.id===id&&current.work.revision>=observed.revision?current.work:observed;
 }
 /** Validate the Work shape consumed by the selector and task renderer. */
 const validWork=work=>work&&typeof work.id==='string'&&typeof work.title==='string'&&typeof work.objective==='string'
  &&(work.tasks==null||Array.isArray(work.tasks)&&work.tasks.every(task=>task&&typeof task.id==='string'));
 /** Build options for the observed catalog without changing the selected Work model. */
 function renderWorkSelector(id){
  const selector=$('work');selector.replaceChildren(new Option(tr('Choisissez une mission'),''));
  for(const work of works)selector.append(new Option(work.title,work.id));
  selector.value=id;
 }
 /** Start a linked Work from its protected record before requesting the full
  * catalog. The catalog enriches navigation only, and cannot overwrite the
  * mission or DOM after another selection or refresh starts. */
 async function openLinkedWork(id){
  let ticket=++generation,opened=false;
  $('work').disabled=true;$('refresh').disabled=true;$('connection').textContent=tr('Lecture de la mission…');
  try{
   const session=await read('/api/v1/native-session');
   if(ticket!==generation)return;
   if(session?.schema_version!==1)throw failure('web-native-session-invalid');
   const work=await read('/api/v1/planning?'+new URLSearchParams({work:id}));
   if(ticket!==generation)return;
   if(!validWork(work)||work.id!==id||!Number.isSafeInteger(work.revision)||work.revision<1)
    throw failure('web-native-work-invalid');
   works=[{...work,tasks:work.tasks??[]}];renderWorkSelector(id);renderManager();
   const selection=select(id);ticket=generation;opened=true;
   $('work').disabled=false;$('refresh').disabled=false;
   await selection;
   if(ticket!==generation)return;
   const list=await read('/api/v1/works');
   if(ticket!==generation)return;
   if(!Array.isArray(list)||!list.every(validWork))throw failure('web-native-work-list-invalid');
   works=[works.find(row=>row.id===id),...list.filter(row=>row.id!==id).map(row=>({...row,tasks:row.tasks??[]}))];
   renderWorkSelector(id);renderManager();
  }catch(cause){
   if(ticket!==generation)return;
   if(!opened&&cause.code==='work-missing')return load();
   $('message').hidden=false;$('message').className='notice alert';
   $('message').textContent=tr(opened?'Liste des missions indisponible. La mission affichée reste consultable.'
    :'Lecture impossible. Actualisez pour reprendre.')+' ('+(cause.code||'web-native-response-invalid')+')';
  }finally{
   if(ticket===generation){$('refresh').disabled=false;$('work').disabled=works.length===0;}
  }
 }
 /** Bootstrap reads only. Empty/malformed lists and missing selected IDs are
  * distinguished; a missing explicit Work never falls back to another mission.
  * Returns true only when the selected panels were actually reread; swallowed
  * startup/read failure must not become a write consumer success. Selection
  * preferences contain identifiers only, never reports or credentials. */
 async function load(){const ticket=++generation;$('work').disabled=true;$('refresh').disabled=true;$('connection').textContent=tr('Chargement des missions.');
  try{const session=await read('/api/v1/native-session');if(session?.schema_version!==1)throw failure('web-native-session-invalid');const data=await read('/api/v1/works');if(ticket!==generation)return;
   if(!Array.isArray(data)||!data.every(validWork))throw failure('web-native-work-list-invalid');works=data.map(w=>({...w,tasks:w.tasks??[]}));
   const requested=new URL(location.href).searchParams.get('work')||selected;const found=works.some(w=>w.id===requested);renderWorkSelector(found?requested:'');renderManager();await select(found?requested:'');
   if(requested&&!found){$('message').hidden=false;$('message').className='notice alert';$('message').textContent=tr('La mission demandée ne figure pas dans la liste reçue.')}else if(!works.length){$('message').hidden=false;$('message').className='notice info';$('message').textContent=tr('Aucune mission enregistrée.')}else $('message').hidden=true;
   return !!current&&current.work.id===selected&&current.panelsObserved;
  }catch(cause){$('message').hidden=false;$('message').className='notice alert';$('message').textContent=tr('Lecture impossible. Actualisez pour reprendre.')+' ('+(cause.code||'web-native-response-invalid')+')';return false}finally{$('main-content').removeAttribute('aria-busy');$('refresh').disabled=false;$('work').disabled=works.length===0}
 }
 /** Enable the cockpit guide on the existing rail control. Read-only guidance
  * works without a selected mission. Shortcuts use current view permissions
  * and normal navigation; opening and closing the guide only change the DOM. */
 function setupCockpitHelp(){
  const opener=$('help'),dialog=document.createElement('dialog');
  const title='Aide du cockpit IA',header=document.createElement('header'),heading=document.createElement('h2');
  const close=document.createElement('button'),body=document.createElement('section');
  const railSettings=opener.closest('.rail-settings');
  railSettings?.before(opener);
  opener.disabled=false;opener.textContent=tr(title);opener.setAttribute('aria-haspopup','dialog');
  dialog.id='native-cockpit-help';dialog.setAttribute('aria-labelledby','native-cockpit-help-title');
  heading.id='native-cockpit-help-title';heading.textContent=tr(title);heading.tabIndex=-1;
  close.type='button';close.textContent=tr('Fermer');close.addEventListener('click',()=>dialog.close());
  header.append(heading,close);dialog.append(header,body);

  const shortcuts=[];
  for(const [label,description,links] of [
   ['Lire un livrable','Dans Conduite, choisissez Voir sous À vérifier. Le rapport remis s’ouvre dès qu’il est disponible. Utilisez Rechercher pour trouver un passage.',[['conduite','Ouvrir Conduite']]],
   ['Comprendre la validation','Après lecture, utilisez Valider si le bouton est actif. En cas de blocage, le détail indique le motif et, pour une revue manquante, les appels utilisés si la mission les enregistre. Ouvrez Diagnostic pour voir le code.',[]],
   ['Consulter les journaux et la reprise','Journaux affiche les événements des agents. Reprise présente les éléments enregistrés pour reprendre le travail.',[['logs','Ouvrir les journaux'],['resume','Ouvrir Reprise']]],
  ]){
   const section=document.createElement('section'),subtitle=document.createElement('h3');
   const text=document.createElement('p'),actions=document.createElement('div');
   subtitle.textContent=tr(label);text.textContent=tr(description);
   for(const [view,label] of links){
    const button=document.createElement('button');
    button.type='button';button.textContent=tr(label);
    button.addEventListener('click',()=>dialog.close(view));
    shortcuts.push([view,button]);actions.append(button);
   }
   section.append(subtitle,text);
   if(links.length)section.append(actions);
   body.append(section);
  }
  opener.addEventListener('click',()=>{
   for(const [view,button] of shortcuts)button.hidden=globalThis.SwarmAuthorityMenu?.visibleView(view)===false;
   dialog.returnValue='';
   dialog.showModal();
   if(globalThis.SwarmShell)SwarmShell.focusWindowStart(dialog);
   else heading.focus();
  });
  dialog.addEventListener('close',()=>{
   if(views.has(dialog.returnValue)&&globalThis.SwarmAuthorityMenu?.visibleView(dialog.returnValue)!==false)
    showView(dialog.returnValue,true);
   else opener.focus({preventScroll:true});
  });
  document.body.append(dialog);
 }
 /** Limitations sit in their original sections, with working navigation and an
  * actual links to supported Rust Admin/settings and its scoped limits modal.
  * Historical Go-only forms stay hidden in this native profile. No unsupported action sends
  * an HTTP request, records a visit, mutates a mission or starts an agent.
  * Explicit graph editors register a separate awaited readback consumer; their
  * protected writes are not inferred from the disabled historical controls. */
 function setup(){for(const section of document.querySelectorAll('.view')){
   for(const control of section.querySelectorAll('button,input,select,textarea'))control.disabled=true;
   const note=document.createElement('p');note.className='notice info';note.textContent=section.id==='conduite'?tr('Le graphe permet de créer des tâches et de modifier leurs liens. Les consignes attribuées sont consultables et rejetables avant transmission.'):section.id==='budget'?tr('Budget, plafonds et tarifs sont réglables ici. Les autres commandes restent à raccorder.'):tr(['tasks','conduite','manage','budget','agents','resume','logs'].includes(section.id)?'Lecture Rust des données enregistrées. Les commandes restent à raccorder.':'Ce parcours de l’interface existante n’est pas encore raccordé au moteur Rust.');section.prepend(note);
  }
  $('manage-search').disabled=false;$('manage-search').addEventListener('input',renderManager);$('manage-state').value='all';
  $('assistant').hidden=true;$('mode').disabled=true;$('version-open').disabled=false;
  setupCockpitHelp();
  const link=document.createElement('a');link.href=url('/native-runtime.html?view=admin');link.textContent=tr('Ouvrir les réglages Rust');link.className='workbench-admin-primary';$('admin-title').after(link);const limits=document.createElement('a');limits.href=url('/native-runtime.html?view=admin#native-run-limits');limits.textContent=tr('Régler les limites d’exécution');limits.className='workbench-admin-primary';link.after(limits);for(const node of $('admin').children)if(![link,limits,$('admin-title')].includes(node)&&!node.querySelector('#admin-pricing-open'))node.hidden=true;
  for(const a of document.querySelectorAll('a[href^="/"]')){if(a===link||a===limits)continue;const target=new URL(a.href);if(['/prepare.html','/trust-actions.html','/security-proofs.html','/paea.html'].includes(target.pathname)){const destination=target.pathname+target.search+target.hash;const sync=()=>{a.href=url(destination)};sync();a.addEventListener('click',sync);continue;}if(!['/','/trust-actions.html','/security-proofs.html','/paea.html'].includes(target.pathname)){a.addEventListener('click',event=>{event.preventDefault();$('message').hidden=false;$('message').className='notice info';$('message').textContent=tr('Cette destination reste à raccorder au moteur Rust.');$('message').scrollIntoView({block:'nearest'})})}}
  // Saving remains pending through the actual observation reload. The editor
  // owns error display and duplicate-submit protection; preserve changed views.
  SwarmRustDependencyGraph.setRefreshHandler(async(id,revision)=>{if(selected!==id)return;const observed=await load();if(selected===id&&(!observed||!current||!Number.isSafeInteger(current.work.revision)||current.work.revision<revision))throw failure('web-native-graph-refresh-unconfirmed')});
  $('tabs').addEventListener('click',event=>{const button=event.target.closest('[data-view]');if(button)showView(button.dataset.view,true)});$('work').addEventListener('change',()=>select($('work').value));$('refresh').addEventListener('click',load);
  /** Change only the known presentation enum and its authoritative URL value.
   * Language navigation and reload consume that URL; unrelated query data and
   * history position remain intact. No local storage, mission or policy write;
   * URL/history refusal keeps the old DOM/URL, shows a safe diagnostic and
   * propagates a typed error retaining its original cause. */
  function toggleTheme(){const theme=document.documentElement.dataset.theme==='etat'?'sombre':'etat';const currentUrl=new URL(location.href);currentUrl.searchParams.set('theme',theme);try{history.replaceState(null,'',currentUrl)}catch(cause){$('message').hidden=false;$('message').className='notice alert';$('message').textContent=tr('Préférence de thème non enregistrée. Le thème précédent est conservé.');throw failure('web-native-theme-update-refused',cause)}document.documentElement.dataset.theme=theme;$('theme').textContent=tr(theme==='etat'?'Passer au thème sombre':'Passer au thème clair');link.href=url('/native-runtime.html?view=admin');limits.href=url('/native-runtime.html?view=admin#native-run-limits')}
  $('theme').addEventListener('click',toggleTheme);
  const readModeButton=document.createElement('button');readModeButton.type='button';readModeButton.id='mission-read-mode';readModeButton.textContent=tr('Lire l’historique enregistré');$('refresh').after(readModeButton);
  readModeButton.addEventListener('click',()=>{const u=new URL(location.href);u.searchParams.set('read',current?.readMode==='history'?'current':'history');history.replaceState(null,'',u);select(selected)});
  globalThis.SwarmWorkbench?.mount();
  instructions=document.createElement('div');instructions.id='native-instruction-history';$('conduite-plan').after(instructions);
  resumePanel=SwarmNativeResume.mount();
  logsPanel=SwarmNativeLogs.mount();
  SwarmNativePricing.enable();
  SwarmCockpitQuotas.mount({opener:$('quotas-open'),selectedWork:()=>selected});
  budgetEditor=SwarmNativeWorkBudget.mount({reload:async()=>{if(!await load())throw Error('web-native-budget-cockpit-refresh-failed')}});
  $('theme').textContent=tr(document.documentElement.dataset.theme==='etat'?'Passer au thème sombre':'Passer au thème clair');showView(new URL(location.href).searchParams.get('view')||'conduite');
  const linkedWork=new URL(location.href).searchParams.get('work');
  if(linkedWork)openLinkedWork(linkedWork);else load();
 }
 setup();
})();
