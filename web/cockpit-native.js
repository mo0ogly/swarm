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
  * local cap or deadline is added. The caller owns visible failure/recovery. */
 async function read(path){let response;try{response=await fetch(path,{credentials:'same-origin',headers:{Accept:'application/json'}})}catch(cause){throw failure('web-native-response-unavailable',cause)}
  let value;try{value=await response.json()}catch(cause){throw failure('web-native-response-invalid',cause)}
  if(!response.ok){const code=value?.failure?.code||value?.error;throw failure(typeof code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(code)?code:'web-native-request-refused')}
  return value;
 }
 /** Preserve language and theme on actual supported destinations. Only trusted
  * relative paths are used; Work identifiers always become query data. */
 function url(path){const u=new URL(path,location.href);u.searchParams.set('lang',globalThis.SwarmI18n?.language==='en'?'en':'fr');u.searchParams.set('theme',document.documentElement.dataset.theme);return u.pathname+u.search}
 /** Reuse original sections/navigation. Unconnected sections remain reachable,
  * with visible limitations and disabled commands, never silently empty success. */
 function showView(name,focus=false){if(!views.has(name))name='conduite';for(const section of document.querySelectorAll('.view'))section.hidden=section.id!==name;for(const button of document.querySelectorAll('#tabs [data-view]')){if(button.dataset.view===name)button.setAttribute('aria-current','page');else button.removeAttribute('aria-current')}
  $('assistant').hidden=true;globalThis.SwarmShell?.navigate(name);const u=new URL(location.href);u.searchParams.set('view',name);history.replaceState(null,'',u);if(focus)$('main-content').focus();if(name==='manage')renderManager();if(name==='resume')resumePanel?.refresh();if(name==='logs')logsPanel?.refresh();if(name!=='logs'&&current?.statusObserved===false)select(selected);
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
  * data is read-only; controls do not establish conductor/process liveness. */
 async function select(id){const ticket=++generation,changedContext=selected!==id;selected=id;resumePanel?.setWork(id);logsPanel?.setWork(id);const work=works.find(w=>w.id===id);budgetEditor?.setWork(work?id:'');current=null;if(instructions){if(changedContext)SwarmNativeInstructions.clear(instructions);else instructions.replaceChildren()}SwarmRustCockpitPanels.clear(tr(id?'Lecture de la mission…':'Aucune mission sélectionnée.'));const u=new URL(location.href);if(id)u.searchParams.set('work',id);else u.searchParams.delete('work');history.replaceState(null,'',u);
  if(!work){$('title').textContent=tr('Choisissez une mission');$('objective').textContent='';for(const id of ['task-count','accepted-count','active-count','decision-count'])$(id).textContent='—';for(const id of ['tasks-body','conduite-plan','graph'])$(id).replaceChildren();$('main-content').removeAttribute('aria-busy');$('connection').textContent=tr('Aucune mission sélectionnée.');return}
  $('title').textContent=work.title;$('objective').textContent=work.objective;$('connection').textContent=tr('Lecture de la mission…');$('main-content').setAttribute('aria-busy','true');
  // Clear the previous mission before starting a potentially failing read.
  SwarmRustCockpitRead.render({work,mission:null,missionError:'web-native-read-pending',control:null});
  let control=null,mission=null,error=null,panelsObserved=false;const statusObserved=$('logs').hidden;
  try{control=await read('/api/v1/mission?'+new URLSearchParams({work:id}));if(!control||!Number.isSafeInteger(control.revision)||typeof control.archived!=='boolean')throw failure('web-native-control-invalid')}catch(cause){error=cause.code||'web-native-control-invalid'}
  if(ticket!==generation)return;
  if(statusObserved){
  try{const value=await read('/api/v1/mission?'+new URLSearchParams({work:id,action:'status'}));if(!value||!Object.hasOwn(value,'tasks')||typeof value.summary!=='string'||typeof value.next!=='string'||!(value.tasks===null||Array.isArray(value.tasks)&&value.tasks.every(t=>t&&typeof t.id==='string')))throw failure('web-native-status-invalid');mission=value}catch(cause){error=cause.code||'web-native-status-invalid'}
  }else error='web-native-status-not-observed';
  if(ticket!==generation)return;
  try{const observation=await read('/api/v1/cockpit-read?'+new URLSearchParams({work:id}));if(ticket!==generation)return;if(observation?.work_id!==id)throw failure('web-native-observation-invalid');SwarmRustCockpitPanels.render(observation);panelsObserved=true}catch(cause){if(ticket!==generation)return;error=cause.code||'web-native-observation-invalid';SwarmRustCockpitPanels.clear(tr('Lecture impossible. Actualisez pour reprendre.')+' ('+error+')')}
  current={work,mission,missionError:error,control,panelsObserved,statusObserved};SwarmNativeInstructions.render(instructions,work,rejectInstruction,true);SwarmRustCockpitRead.render(current);$('main-content').removeAttribute('aria-busy');$('connection').textContent=tr(error?'Lecture partielle — actualisez pour reprendre.':'Lecture reçue — actualisation manuelle.');
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
 /** Bootstrap reads only. Empty/malformed lists and missing selected IDs are
  * distinguished; a missing explicit Work never falls back to another mission.
  * Returns true only when the selected panels were actually reread; swallowed
  * startup/read failure must not become a write consumer success. Selection
  * preferences contain identifiers only, never reports or credentials. */
 async function load(){const ticket=++generation;$('work').disabled=true;$('refresh').disabled=true;$('connection').textContent=tr('Chargement des missions.');
  try{const session=await read('/api/v1/native-session');if(session?.schema_version!==1)throw failure('web-native-session-invalid');const data=await read('/api/v1/works');if(ticket!==generation)return;
   if(!Array.isArray(data)||!data.every(w=>w&&typeof w.id==='string'&&typeof w.title==='string'&&typeof w.objective==='string'&&(w.tasks==null||Array.isArray(w.tasks)&&w.tasks.every(t=>t&&typeof t.id==='string'))))throw failure('web-native-work-list-invalid');works=data.map(w=>({...w,tasks:w.tasks??[]}));$('work').replaceChildren();
   const initial=document.createElement('option');initial.value='';initial.textContent=tr('Choisissez une mission');$('work').append(initial);for(const work of works){const option=document.createElement('option');option.value=work.id;option.textContent=work.title;$('work').append(option)}
   const requested=new URL(location.href).searchParams.get('work')||selected;const found=works.some(w=>w.id===requested);$('work').value=found?requested:'';renderManager();await select(found?requested:'');
   if(requested&&!found){$('message').hidden=false;$('message').className='notice alert';$('message').textContent=tr('La mission demandée ne figure pas dans la liste reçue.')}else if(!works.length){$('message').hidden=false;$('message').className='notice info';$('message').textContent=tr('Aucune mission enregistrée.')}else $('message').hidden=true;
   return !!current&&current.work.id===selected&&current.panelsObserved;
  }catch(cause){$('message').hidden=false;$('message').className='notice alert';$('message').textContent=tr('Lecture impossible. Actualisez pour reprendre.')+' ('+(cause.code||'web-native-response-invalid')+')';return false}finally{$('main-content').removeAttribute('aria-busy');$('refresh').disabled=false;$('work').disabled=works.length===0}
 }
 /** Limitations sit in their original sections, with working navigation and an
  * actual link to the already supported Rust Admin. No unsupported action sends
  * an HTTP request, records a visit, mutates a mission or starts an agent. */
 function setup(){for(const section of document.querySelectorAll('.view')){
   for(const control of section.querySelectorAll('button,input,select,textarea'))control.disabled=true;
   const note=document.createElement('p');note.className='notice info';note.textContent=section.id==='conduite'?tr('Les consignes attribuées sont consultables et rejetables avant transmission. Les autres commandes restent à raccorder.'):section.id==='budget'?tr('Budget, plafonds et tarifs sont réglables ici. Les autres commandes restent à raccorder.'):tr(['tasks','conduite','manage','budget','agents','resume','logs'].includes(section.id)?'Lecture Rust des données enregistrées. Les commandes restent à raccorder.':'Ce parcours de l’interface existante n’est pas encore raccordé au moteur Rust.');section.prepend(note);
  }
  $('manage-search').disabled=false;$('manage-search').addEventListener('input',renderManager);$('manage-state').value='all';
  $('assistant').hidden=true;$('mode').disabled=true;$('help').disabled=true;$('version-open').disabled=false;
  const link=document.createElement('a');link.href=url('/native-runtime.html?view=admin');link.textContent=tr('Ouvrir les réglages Rust');$('admin-title').after(link);
  for(const a of document.querySelectorAll('a[href^="/"]')){if(a===link)continue;const target=new URL(a.href);if(target.pathname!=='/'){a.addEventListener('click',event=>{event.preventDefault();$('message').hidden=false;$('message').className='notice info';$('message').textContent=tr('Cette destination reste à raccorder au moteur Rust.');$('message').scrollIntoView({block:'nearest'})})}}
  $('tabs').addEventListener('click',event=>{const button=event.target.closest('[data-view]');if(button)showView(button.dataset.view,true)});$('work').addEventListener('change',()=>select($('work').value));$('refresh').addEventListener('click',load);
  $('theme').addEventListener('click',()=>{const theme=document.documentElement.dataset.theme==='etat'?'sombre':'etat';document.documentElement.dataset.theme=theme;$('theme').textContent=tr(theme==='etat'?'Passer au thème sombre':'Passer au thème clair');link.href=url('/native-runtime.html?view=admin')});
  instructions=document.createElement('div');instructions.id='native-instruction-history';$('conduite-plan').after(instructions);
  resumePanel=SwarmNativeResume.mount();
  logsPanel=SwarmNativeLogs.mount();
  SwarmNativePricing.enable();
  SwarmCockpitQuotas.mount({opener:$('quotas-open'),selectedWork:()=>selected});
  budgetEditor=SwarmNativeWorkBudget.mount({reload:async()=>{if(!await load())throw Error('web-native-budget-cockpit-refresh-failed')}});
  $('theme').textContent=tr(document.documentElement.dataset.theme==='etat'?'Passer au thème sombre':'Passer au thème clair');showView(new URL(location.href).searchParams.get('view')||'conduite');load();
 }
 setup();
})();
