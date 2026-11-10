'use strict';
/** Native mission readers with a separate explicit consultation bookmark. Uses only authenticated Work/control/status
 * readers supplied by the host. Selection/search never authorize a mission,
 * launch a provider or mark a visit. No polling, hidden retry or policy is added. */
(()=>{
 /** Mount a mission browser once. API hooks must reject failed reads, not return
  * empty success. A generation binds asynchronous results to the selected Work;
  * older reads cannot overwrite a newer selection or refresh. User content is
  * inserted as text only. No private API payload is stored in browser storage. */
 function mount(host,api,{translate=String}={}){
  const tr=translate,engine=s=>globalThis.SwarmI18n?.engine(s)??s;
  const el=(tag,text,cls)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=tr(text);if(cls)n.className=cls;return n};
  const shell=el('div',undefined,'native-mission-layout'),browser=el('section',undefined,'native-mission-browser');browser.setAttribute('aria-label',tr('Missions disponibles'));
  const label=el('label','Rechercher une mission'),search=el('input');search.type='search';search.placeholder=tr('Nom, objectif ou identifiant');label.append(search);
  const refresh=el('button','Actualiser les missions');refresh.type='button';refresh.dataset.cockpitRefresh='';
  const feedback=el('p',undefined,'notice info');feedback.setAttribute('role','status');
  const list=el('ul',undefined,'native-mission-list'),detail=el('section',undefined,'native-mission-detail');detail.setAttribute('aria-label',tr('Mission sélectionnée'));detail.dataset.cockpitDetail='';
  browser.append(label,refresh,feedback,list);shell.append(browser,detail);host.append(shell);
  let works=[],selected='',generation=0,loading=false,listReady=false;const opened=new Set();
  /** Safe fixed error family only. Response bodies, credentials and stack traces
   * remain on the original rejection and are never interpolated into markup. */
  function failure(error){return typeof error?.code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(error.code)?error.code:'web-native-response-invalid'}
  /** Display a named fact. Unknown values stay unknown; false/zero are retained.
   * Callers provide plain strings or safe integer representations, never HTML. */
  function fact(dl,label,value){dl.append(el('dt',label));const dd=el('dd');dd.textContent=value===undefined||value===null||value===''?tr('Non disponible'):String(value);dl.append(dd)}
  /** Choose a mission explicitly through a button or an existing URL. A missing
   * Work remains visibly unavailable; it never silently falls back to another. */
  function renderList(){list.replaceChildren();const needle=search.value.trim().toLocaleLowerCase();const filtered=works.filter(w=>[w.id,w.title,w.objective].some(v=>v.toLocaleLowerCase().includes(needle)));
   for(const work of filtered){const li=el('li'),button=el('button');button.type='button';button.dataset.work=work.id;const name=el('strong'),id=el('small');name.textContent=work.title||work.id;id.textContent=work.id;button.append(name,id);if(selected===work.id)button.setAttribute('aria-current','true');button.addEventListener('click',()=>select(work.id,true));li.append(button);list.append(li)}
   if(listReady){feedback.className='notice info';feedback.textContent=tr(works.length===0?'Aucune mission enregistrée.':'Aucune mission ne correspond à cette recherche.');feedback.hidden=filtered.length!==0;}
  }
  /** Render a readable task with server explanations and agent/attempt/model
   * facts when present. Stored Task fallback carries no completion or validation
   * claim. Expansion state is retained only in memory for this mission/task. */
  function taskCard(task,observed){
   const fold=el('details',undefined,'native-task'),summary=el('summary'),name=el('strong'),state=el('span',undefined,'native-state');name.textContent=task.title||task.id;state.textContent=engine(task.label||task.state||tr('Non disponible'));summary.append(name,state);fold.append(summary);const body=el('div');
   for(const value of [task.reason,task.understanding?.what,task.understanding?.next_step])if(typeof value==='string'&&value){const p=el('p');p.textContent=engine(value);body.append(p)}
   const dl=el('dl',undefined,'native-facts');fact(dl,'Identifiant',task.id);
   if(observed&&task.attempt){fact(dl,'Agent',task.attempt.agent_id);fact(dl,'Tentative',task.attempt.attempt_id);fact(dl,'Modèle demandé',task.attempt.requested_model);fact(dl,'Modèle observé',task.attempt.observed_model);fact(dl,'Validation',engine(task.attempt.validation_state||''));}
   body.append(dl);fold.append(body);const key=selected+':'+task.id;fold.open=opened.has(key);fold.addEventListener('toggle',()=>{if(fold.open)opened.add(key);else opened.delete(key)});return fold;
  }
  /** Render independent control/status observations. A refused rich status keeps
   * its visible error alongside explicitly recorded Work data; absence is never
   * a zero, success, completed mission or authorization. Each read is timestamped
   * locally and the UI does not claim an atomic snapshot across API requests. */
  /** Read exchanges only on explicit action for this selected Work/generation.
   * Complete typed response or visible failure; a stale/detached selection never
   * receives late data. Recorded content is text, not authenticated delivery,
   * permission, liveness or acceptance. No polling, provider or visit mutation. */
  /** Explicit paginated activity tied to Work/selection generation. A failed
   * page preserves already read rows and cursor for deliberate retry. Policy
   * changes reset by explicit first-page read; no inferred identity/acceptance.
   * All recorded content is literal text; no polling or page limit fallback. */
  function renderActivity(work){
   const ticket=generation,section=el('details',undefined,'native-task'),rows=el('div'),state=el('p','Lecture à la demande.','notice info'),read=el('button','Lire l’activité'),more=el('button','Lire la suite'),filter=el('input'),label=el('label','Décisions humaines uniquement');filter.type='checkbox';label.prepend(filter);read.type=more.type='button';more.hidden=true;read.dataset.activityRead='';more.dataset.activityMore='';filter.dataset.activityDecisions='';state.dataset.activityStatus='';state.setAttribute('role','status');section.dataset.activity='';section.append(el('summary','Fil d’activité'),el('p','L’origine est déduite des données enregistrées ; elle ne prouve pas l’identité de leur auteur.'),label,read,more,state,rows);detail.append(section);let busy=false,cursor='';
   async function load(append){if(busy)return;busy=true;read.disabled=more.disabled=filter.disabled=true;state.className='notice info';state.textContent=tr('Chargement…');try{
    if(typeof api.activity!=='function')throw {code:'activity-reader-unavailable'};
    const page=await api.activity({work,before:append?cursor:'',decisions:filter.checked?'1':'0'});if(ticket!==generation||selected!==work||!section.isConnected)return;
    if(!page||!Array.isArray(page.entries)||typeof page.more!=='boolean'||typeof page.next_cursor!=='string'||typeof page.history_truncated!=='boolean'||!page.entries.every(e=>e&&['at','rank','origin','kind','label','message'].every(k=>typeof e[k]==='string')&&(!Object.hasOwn(e,'task_id')||typeof e.task_id==='string'))||page.more&&(page.entries.length===0||page.next_cursor!==page.entries.at(-1).at+'|'+page.entries.at(-1).rank)||!page.more&&page.next_cursor!=='')throw {code:'activity-view-invalid'};
    const fragment=document.createDocumentFragment();for(const entry of page.entries){const card=el('article',undefined,'native-task'),heading=el('h4'),meta=el('p'),message=el('p');heading.textContent=engine(entry.label);meta.textContent=entry.at+' · '+engine(entry.origin)+(entry.task_id?' · '+entry.task_id:'');message.textContent=engine(entry.message);card.append(heading,meta,message);fragment.append(card)}
    if(!append)rows.replaceChildren();rows.append(fragment);if(!append&&!page.entries.length)rows.append(el('p','Aucune entrée dans cette lecture.'));cursor=page.next_cursor;more.hidden=!page.more;state.textContent=tr(page.history_truncated?'Historique tronqué. Pour lire les entrées plus anciennes, augmentez le maximum par source dans Administration → Pagination du fil d’activité, puis relisez.':'Activité relue.');
   }catch(error){if(ticket!==generation||selected!==work||!section.isConnected)return;state.className='notice alert';state.textContent=tr('L’activité n’a pas pu être lue. Relancez la lecture pour reprendre.')+' ('+failure(error)+')'}finally{busy=false;read.disabled=more.disabled=filter.disabled=false}}
   read.addEventListener('click',()=>load(false));more.addEventListener('click',()=>load(true));filter.addEventListener('change',()=>{cursor='';more.hidden=true;rows.replaceChildren();state.textContent=tr('Lecture à la demande.')});
  }
  function renderExchanges(work){
   const ticket=generation,section=el('details',undefined,'native-task'),title=el('summary','Échanges entre agents'),body=el('div'),read=el('button','Lire les échanges'),state=el('p','Lecture à la demande.','notice info'),rows=el('div');read.type='button';read.dataset.exchangesRead='';state.dataset.exchangesStatus='';state.setAttribute('role','status');section.dataset.exchanges='';body.append(el('p','Les déclarations ne prouvent ni transmission, ni réception, ni droit d’exécution.'),read,state,rows);section.append(title,body);detail.append(section);let busy=false;
   read.addEventListener('click',async()=>{if(busy)return;busy=true;read.disabled=true;state.className='notice info';state.textContent=tr('Chargement…');rows.replaceChildren();try{
    if(typeof api.exchanges!=='function')throw {code:'exchange-reader-unavailable'};
    const entries=await api.exchanges(work);if(ticket!==generation||selected!==work||!section.isConnected)return;
    const required=['id','work_id','kind','source_agent_id','source_task_id','source_attempt_id','recipient_task_id','recipient_role','state','created_at'];
    if(!Array.isArray(entries)||!entries.every(e=>e&&required.every(k=>typeof e[k]==='string')&&e.work_id===work&&['recipient_attempt_id','reply_to','result_state','need','deadline_at','acknowledged_at','acknowledged_by','acknowledgement'].every(k=>!Object.hasOwn(e,k)||typeof e[k]==='string')&&(e.artifacts===null||Array.isArray(e.artifacts)&&e.artifacts.every(a=>a&&typeof a.path==='string'&&typeof a.sha256==='string'))))throw {code:'exchange-view-invalid'};
    if(!entries.length)rows.append(el('p','Aucun échange enregistré.'));
    for(const e of entries){const card=el('article',undefined,'native-task'),heading=el('h4');heading.textContent=e.id;const facts=el('dl',undefined,'native-facts');for(const[label,value]of [['Type',e.kind],['État enregistré',e.state],['Source',e.source_task_id+' · '+e.source_agent_id],['Destinataire',e.recipient_task_id+' · '+e.recipient_role],['Date enregistrée',e.created_at]])fact(facts,label,value);card.append(heading,facts);for(const value of [e.need,e.result_state,e.acknowledgement])if(value){const p=el('p');p.textContent=value;card.append(p)}if(e.artifacts?.length){card.append(el('h5','Pièces déclarées'));for(const a of e.artifacts){const p=el('p');p.textContent=a.path+' · '+a.sha256;card.append(p)}}rows.append(card)}
    state.textContent=tr('Échanges relus.');
   }catch(error){if(ticket!==generation||selected!==work||!section.isConnected)return;state.className='notice alert';state.textContent=tr('Les échanges n’ont pas pu être lus. Relancez la lecture pour reprendre.')+' ('+failure(error)+')'}finally{busy=false;read.disabled=false}});
  }
  /** Explicit historical read, never operation execution/current authority.
   * Preserve prior rows on failure. Ignore detached/old selection results without
   * claiming backend cancellation. Exact integer metadata prevents rounding. */
  function renderPlanningHistory(work){
   const ticket=generation,section=el('details',undefined,'native-task'),read=el('button','Lire les décisions'),state=el('p','Lecture à la demande.','notice info'),rows=el('div');let busy=false;
   section.dataset.planningHistory='';read.dataset.planningHistoryRead='';state.dataset.planningHistoryStatus='';read.type='button';state.setAttribute('role','status');
   section.append(el('summary','Historique des décisions de planification'),el('p','Données enregistrées : aucune opération n’est exécutée et aucun droit actuel n’est déduit de cette lecture.'),read,state,rows);detail.append(section);
   const labels={kind:'Type',id:'Identifiant',title:'Titre',requirements:'Exigences',deliverable:'Livrable',criteria:'Critères',depends:'Dépendances',next:'Suite enregistrée',judge_verdict:'Verdict déclaré',judge_cycle:'Cycle déclaré',expected_candidate:'Candidat déclaré',max_tasks:'Plafond de tâches déclaré',max_activations:'Plafond d’activations déclaré'};
   const strings=['judge_verdict','expected_candidate','deliverable','next'],numbers=['judge_cycle','max_tasks','max_activations'],arrays=['requirements','criteria','depends'];
   const array=value=>value===null||Array.isArray(value)&&value.every(item=>typeof item==='string');
   function valid(entries){return Array.isArray(entries)&&entries.every(entry=>entry&&['id','at','scope','reason'].every(key=>typeof entry[key]==='string')&&(entry.operations===null||Array.isArray(entry.operations)&&entry.operations.every(op=>op&&typeof op==='object'&&!Array.isArray(op)&&['kind','id','title'].every(key=>typeof op[key]==='string')&&array(op.requirements)&&Object.entries(op).every(([key,value])=>['kind','id','title'].includes(key)||strings.includes(key)&&typeof value==='string'||numbers.includes(key)&&typeof value==='number'&&Number.isInteger(value)||arrays.includes(key)&&array(value)))))}
   read.addEventListener('click',async()=>{if(busy)return;busy=true;read.disabled=true;state.className='notice info';state.textContent=tr('Chargement…');try{
    if(typeof api.planningHistory!=='function')throw {code:'planning-history-reader-unavailable'};
    const entries=await api.planningHistory(work);if(ticket!==generation||selected!==work||!section.isConnected)return;if(!valid(entries))throw {code:'planning-history-view-invalid'};
    const fragment=document.createDocumentFragment();
    for(const entry of entries){const card=el('article',undefined,'native-task'),heading=el('h4'),meta=el('p'),reason=el('p');heading.textContent=entry.id;meta.textContent=entry.at+' · '+entry.scope;reason.textContent=entry.reason;card.append(heading,meta,reason);
     if(entry.operations===null)card.append(el('p','Opérations non renseignées.'));
     else if(entry.operations.length===0)card.append(el('p','Aucune opération enregistrée dans cette décision.'));
     else for(const op of entry.operations){const fields=el('dl',undefined,'native-facts');for(const [key,value]of Object.entries(op)){const dt=el('dt');dt.textContent=tr(labels[key]);const dd=el('dd');dd.textContent=typeof value==='number'?SwarmRecordedCursor.field(op,key,true):typeof value==='string'?value:JSON.stringify(value);fields.append(dt,dd)}card.append(fields)}
     fragment.append(card);
    }
    if(entries.length===0)fragment.append(el('p','Aucune décision de planification enregistrée.'));rows.replaceChildren(fragment);state.textContent=tr('Historique des décisions relu.');
   }catch(error){if(ticket!==generation||selected!==work||!section.isConnected)return;state.className='notice alert';state.textContent=tr('Historique indisponible. Les décisions précédentes sont conservées ; relancez la lecture.')+' ('+failure(error)+')'}finally{busy=false;read.disabled=false}});
  }
  function renderDetail(work,controls,status){
   detail.replaceChildren();const top=el('div',undefined,'section-heading'),heading=el('h3');heading.textContent=work.title||work.id;heading.tabIndex=-1;top.append(heading);const reread=el('button','Actualiser cette mission');reread.type='button';reread.addEventListener('click',()=>select(work.id,false));top.append(reread);detail.append(top);
   if(work.objective){const objective=el('p',undefined,'native-objective');objective.textContent=work.objective;detail.append(objective)}
   const meta=el('dl',undefined,'native-facts');fact(meta,'Mission',work.id);fact(meta,'Révision enregistrée',Number.isSafeInteger(work.revision)?String(work.revision):undefined);
   if(controls.status==='fulfilled'){
    const c=controls.value;if(c&&typeof c==='object'&&typeof c.archived==='boolean'){
     fact(meta,'Nouveaux départs',c.archived?tr('Mission archivée'):c.paused===true?tr('Suspendus'):c.paused===false?tr('Non suspendus — autorisations requises'):undefined);
     fact(meta,'Consentement local',c.consent_enabled===true?tr('Accord enregistré'):c.consent_enabled===false?tr('Accord non enregistré'):undefined);
    }else{controls={status:'rejected',reason:{code:'web-native-control-invalid'}}}
   }
   detail.append(meta);
   if(controls.status==='rejected'){const error=el('p',undefined,'notice alert');error.setAttribute('role','alert');error.textContent=tr('Les contrôles de la mission n’ont pas pu être lus.')+' ('+failure(controls.reason)+')';detail.append(error)}
   let observed=status.status==='fulfilled'&&status.value&&Object.hasOwn(status.value,'tasks')&&typeof status.value.summary==='string'&&typeof status.value.next==='string'&&(status.value.tasks==null||Array.isArray(status.value.tasks)&&status.value.tasks.every(t=>t&&typeof t.id==='string'&&typeof t.title==='string'));
   if(status.status==='fulfilled'&&!observed)status={status:'rejected',reason:{code:'web-native-status-invalid'}};
   if(!observed){const error=el('p',undefined,'notice alert');error.setAttribute('role','alert');error.textContent=tr('Le suivi détaillé est indisponible. Les tâches ci-dessous proviennent des données enregistrées, sans confirmation de leur exécution.')+' ('+failure(status.reason)+')';detail.append(error)}
   if(observed){const current=status.value;
    for(const value of [current.summary,current.next,current.supervision?.last_error])if(typeof value==='string'&&value){const p=el('p',undefined,value===current.supervision?.last_error?'notice alert':'notice info');p.textContent=engine(value);detail.append(p)}
    const counts=el('dl',undefined,'native-facts');for(const [key,label] of [['validated','Validées'],['running','En cours'],['review','En revue'],['active_agents','Agents actifs']])fact(counts,label,Number.isSafeInteger(current[key])&&current[key]>=0?String(current[key]):undefined);detail.append(counts);
   }
   const tasks=observed?status.value.tasks:work.tasks;
   detail.append(el('h4',observed?'Tâches et prochaines étapes':'Tâches enregistrées'));
   if(!tasks?.length)detail.append(el('p',observed?'Aucune tâche dans ce suivi.':'Aucune tâche enregistrée.'));
   else for(const task of tasks)detail.append(taskCard(task,observed));
   if(api.visit){const visit=el('section');detail.append(visit);SwarmSessionVisit.mount(visit,work,api.visit,{translate:tr});}
   if(api.planningHistory)renderPlanningHistory(work.id);
   renderActivity(work.id);
   renderExchanges(work.id);
   const time=el('p',undefined,'native-reading-time');time.textContent=tr('Lectures reçues à')+' '+new Date().toLocaleTimeString(globalThis.SwarmI18n?.locale)+' · '+tr('Actualisation manuelle. Les lectures ne constituent pas un instantané atomique.');detail.append(time);
  }
  /** Load the selected Work's two read surfaces. Generation suppression ignores
   * stale results without cancelling a shared authenticated request or claiming
   * backend cancellation. Errors stay visible with an explicit refresh action. */
  async function select(id,focus){
   selected=id;const ticket=++generation;renderList();const url=new URL(location.href);if(id)url.searchParams.set('work',id);else url.searchParams.delete('work');history.replaceState(null,'',url);
   const work=works.find(w=>w.id===id);detail.replaceChildren();
   if(!id){detail.append(el('h3','Choisissez une mission'),el('p','Sélectionnez une mission dans la liste pour consulter son objectif et son suivi.'));return}
   if(!work){detail.append(el('p','Cette mission ne figure pas dans la liste reçue. Actualisez les missions pour reprendre.','notice alert'));return}
   const title=el('h3');title.textContent=work.title||work.id;title.tabIndex=-1;detail.append(title,el('p','Lecture de la mission…','notice info'));detail.setAttribute('aria-busy','true');if(focus)title.focus();
   const [controls,status]=await Promise.allSettled([api.controls(id),api.status(id)]);
   if(ticket!==generation)return;detail.removeAttribute('aria-busy');const keepFocus=detail.contains(document.activeElement);renderDetail(work,controls,status);if(keepFocus)detail.querySelector('h3').focus();
  }
  /** Refresh the list once per explicit gesture; failure discards stale detail
   * and stays visible. No elapsed-time threshold, retry loop or auto selection.
   * A malformed response is rejected as a whole before rendering any Work. */
  async function load(){if(loading)return;loading=true;refresh.disabled=true;listReady=false;generation++;detail.removeAttribute('aria-busy');detail.replaceChildren();list.replaceChildren();feedback.hidden=false;feedback.className='notice info';feedback.textContent=tr('Chargement des missions.');
   try{const value=await api.list();if(!Array.isArray(value)||!value.every(w=>w&&typeof w.id==='string'&&typeof w.title==='string'&&typeof w.objective==='string'&&(w.tasks==null||Array.isArray(w.tasks)&&w.tasks.every(t=>t&&typeof t.id==='string'&&typeof t.title==='string')))||new Set(value.map(w=>w.id)).size!==value.length)throw {code:'web-native-work-list-invalid'};
    works=value;listReady=true;renderList();await select(new URL(location.href).searchParams.get('work')||'',false);
   }catch(error){works=[];listReady=false;list.replaceChildren();detail.replaceChildren();feedback.hidden=false;feedback.className='notice alert';feedback.textContent=tr('Les missions n’ont pas pu être chargées. Utilisez Actualiser pour reprendre.')+' ('+failure(error)+')'}finally{loading=false;refresh.disabled=false}}
  search.addEventListener('input',()=>{if(listReady)renderList()});refresh.addEventListener('click',load);load();
  return Object.freeze({refresh:load});
 }
 globalThis.SwarmNativeCockpit=Object.freeze({mount});
})();
