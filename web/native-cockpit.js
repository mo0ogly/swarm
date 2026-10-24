'use strict';
/** Native read-only mission cockpit. Uses only authenticated Work/control/status
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
