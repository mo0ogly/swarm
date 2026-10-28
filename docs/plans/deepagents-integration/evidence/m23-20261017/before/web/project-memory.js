'use strict';
(() => {
 const $=id=>document.getElementById(id),tr=s=>globalThis.SwarmI18n?.t(s)??s;
 let csrf='',works=[],current=null,entries=[],mode='admit',editing=null,pending=null,chosen=[],previewEpoch=0;
 const text=(tag,value,cls)=>{const n=document.createElement(tag);n.textContent=value;if(cls)n.className=cls;return n};
 const message=(value,error=false)=>{const n=$('memory-message');n.textContent=value;n.className='notice '+(error?'alert':'info')};
 async function api(path,body){
  const options=body?{method:'POST',headers:{'Content-Type':'application/json','X-Swarm-CSRF':csrf},body:JSON.stringify(body)}:{};
  if(!globalThis.SwarmWebRequests)throw Error(tr('Service de requêtes indisponible.'));
  const {response,value}=await globalThis.SwarmWebRequests.json(path,options);
  if(!response.ok)throw Error(globalThis.SwarmI18n?.engine(value.error)||value.error||response.statusText);
  return value;
 }
 function resetPreview(){previewEpoch++;pending=null;$('memory-preview').hidden=true;$('memory-examine').classList.add('primary')}
 function newLesson(){$('memory-examine').disabled=!current||!$('memory-source').value;mode='admit';editing=null;resetPreview();$('memory-editor-title').textContent=tr('Admettre une leçon');$('memory-examine').textContent=tr('Examiner l’admission');$('memory-id').readOnly=false;$('memory-id').value='lesson-'+crypto.randomUUID();$('memory-content-fields').hidden=false;for(const id of ['memory-source','memory-body','memory-conditions'])$(id).required=true;$('memory-reason').value='';$('memory-conditions').value='';$('memory-source').dispatchEvent(new Event('change'))}
 function selectedTask(){return current?.tasks.find(t=>t.id===$('memory-task').value)}
 function restoreSelection(){
  const refs=chosen;let missing=0;
  for(const input of document.querySelectorAll('[data-memory-choice]'))input.checked=refs.some(ref=>JSON.stringify(ref)===input.dataset.ref);
  for(const ref of refs)if(!entries.some(e=>e.status==='active'&&JSON.stringify(e.ref)===JSON.stringify(ref)))missing++;
  $('memory-selection-state').textContent=!selectedTask()?tr('Aucune tâche exécutante future dans cette mission.'):missing?tr('La sélection comprend des versions non affichées ou indisponibles : vérifiez l’aperçu.'):tr('Seules les versions actives cochées seront transmises.');
 }
 function renderEntries(){
  const list=$('memory-list');list.replaceChildren();
  if(!entries.length){list.append(text('p',tr('Aucune leçon trouvée. Admettez une leçon depuis un RETEX vérifié.')));return}
  for(const e of entries){
   const v=e.version,card=document.createElement('article');card.className='card';card.dataset.memory=e.ref.id;card.dataset.state=e.status;
   const choice=document.createElement('label');choice.className='memory-choice';const check=document.createElement('input');check.type='checkbox';check.dataset.memoryChoice='true';check.dataset.ref=JSON.stringify(e.ref);check.disabled=e.status!=='active';check.setAttribute('aria-label',tr('Sélectionner la leçon')+' '+e.ref.id+' v'+e.ref.version);check.addEventListener('change',()=>{chosen=chosen.filter(ref=>!(ref.work===e.ref.work&&ref.id===e.ref.id));if(check.checked)chosen.push(e.ref);restoreSelection();resetPreview()});choice.append(check,text('span',e.ref.id+' · v'+e.ref.version+' · '+(e.status==='active'?tr('Active'):e.status==='historical'?tr('Historique'):tr('Indisponible'))));
   card.append(choice,text('p',e.source_title),text('pre',v.body),text('p',v.conditions),text('p',globalThis.SwarmI18n?.engine(e.reason)||e.reason),text('p',v.kind==='procedure'?tr('Procédure'):tr('Expérience')));
   if(e.used_by?.length){card.append(text('p',tr('Tentatives concernées :')+' '+e.used_by.length,'notice attention'));for(const use of e.used_by){const link=text('a',use.work+' · '+use.task+' · '+use.agent);link.href='/?view=agents&work='+encodeURIComponent(use.work);card.append(link)}}
   const details=document.createElement('details');details.append(text('summary',tr('Sources et historique de la décision')),text('p',e.ref.work+' · RETEX '+v.source_retex+' · '+v.actor+' · '+v.at,'memory-receipt'),text('p',v.reason),text('p','SHA-256 '+e.ref.sha256,'memory-receipt'));card.append(details);
   if(e.status!=='historical'){
    const actions=document.createElement('div');actions.className='memory-actions';
    for(const [action,label]of [['replace','Remplacer cette version'],['withdraw','Retirer cette leçon']]){const button=text('button',tr(label));button.type='button';button.disabled=action==='withdraw'&&v.action==='withdraw';button.addEventListener('click',()=>edit(e,action).catch(error=>message(error.message,true)));actions.append(button)}card.append(actions);
   }
   list.append(card);
  }
  restoreSelection();
 }
 async function chooseWork(id){
  const oldTask=$('memory-task').value;current=works.find(w=>w.id===id)||null;resetPreview();
  $('memory-back').href='/?view=retex'+(current?'&work='+encodeURIComponent(current.id):'');
  $('memory-source').replaceChildren();$('memory-task').replaceChildren();
  for(const r of current?.retex||[])if(r.status==='verifie')$('memory-source').append(new Option(r.problem,r.id));
  $('memory-source-state').textContent=$('memory-source').options.length?'':tr('Aucun RETEX vérifié dans cette mission. Choisissez une autre mission ou qualifiez ses preuves.');
  for(const t of current?.tasks||[])if(t.status==='todo'&&t.plan_role!=='subplanner'&&!t.brainstorm)$('memory-task').append(new Option(t.title,t.id));
  if(!$('memory-task').options.length)$('memory-task').append(new Option(tr('Aucune tâche exécutante future'),''));
  $('memory-examine').disabled=!current;$('memory-selection').querySelector('button').disabled=!$('memory-task').value;$('memory-clear').disabled=!$('memory-task').value;
  if([...$('memory-task').options].some(o=>o.value===oldTask))$('memory-task').value=oldTask;chosen=[...(selectedTask()?.memory_selections||[])];newLesson();restoreSelection();
 }
 async function refresh(){
  const old=$('memory-work').value||new URLSearchParams(location.search).get('work');
  const values=await Promise.all([api('/api/v1/works'),api('/api/v1/project-memory?q='+encodeURIComponent($('memory-query').value))]);
  works=values[0];entries=values[1].entries;$('memory-work').replaceChildren(...works.map(w=>new Option(w.title,w.id)));$('memory-work').value=works.some(w=>w.id===old)?old:works[0]?.id||'';
  await chooseWork($('memory-work').value);renderEntries();message(tr('Mémoire du projet actualisée.'));
 }
 async function edit(e,action){
  $('memory-work').value=e.ref.work;await chooseWork(e.ref.work);editing=e;mode=action;
  $('memory-editor-title').textContent=tr(action==='replace'?'Remplacer cette version':'Retirer cette leçon');$('memory-examine').textContent=tr('Examiner la décision');$('memory-id').value=e.ref.id;$('memory-id').readOnly=true;$('memory-source').value=e.version.source_retex;$('memory-kind').value=e.version.kind;$('memory-body').value=e.version.body;$('memory-conditions').value=e.version.conditions;
  $('memory-examine').disabled=action!=='withdraw'&&!$('memory-source').value;$('memory-content-fields').hidden=action==='withdraw';for(const id of ['memory-source','memory-body','memory-conditions'])$(id).required=action!=='withdraw';$('memory-reason').focus();
 }
 async function examine(action,q){
  const epoch=previewEpoch;const result=await api('/api/v1/project-memory?action='+action+'-preview',q);if(epoch!==previewEpoch){message(tr('Formulaire modifié : refaire l’aperçu.'));return}pending={action,q:{...q,preview_token:result.preview_token}};
  $('memory-preview-content').textContent=action==='withdraw'?tr('Cette leçon ne sera plus sélectionnable. Les versions et contextes historiques sont conservés.'):result.context||(result.preview.body+'\n\n'+result.preview.conditions);
  if(action==='replace'&&result.before)$('memory-preview-content').textContent=tr('Avant :')+'\n'+result.before.body+'\n'+result.before.conditions+'\n\n'+tr('Après :')+'\n'+$('memory-preview-content').textContent;
  if(result.affected_attempts?.length)$('memory-preview-content').textContent+='\n\n'+tr('Tentatives concernées :')+' '+result.affected_attempts.length+'\n'+result.affected_attempts.map(u=>u.work+' · '+u.task+' · '+u.agent).join('\n');
  $('memory-preview-receipt').textContent=JSON.stringify(result,null,2);$('memory-preview').hidden=false;$('memory-examine').classList.remove('primary');$('memory-apply').focus();message(tr('Aperçu prêt. Vérifiez le contenu avant application.'));
 }
 $('memory-editor').addEventListener('submit',event=>{event.preventDefault();if(!current)return;const q={schema_version:1,event_id:crypto.randomUUID(),expected_revision:current.revision,work:current.id,id:$('memory-id').value,reason:$('memory-reason').value};if(mode!=='admit')q.expected_sha256=editing.ref.sha256;if(mode!=='withdraw')Object.assign(q,{source_retex:$('memory-source').value,kind:$('memory-kind').value,body:$('memory-body').value,conditions:$('memory-conditions').value});examine(mode,q).catch(error=>message(error.message,true))});
 $('memory-selection').addEventListener('submit',event=>{event.preventDefault();if(!current)return;const selections=[...chosen];examine('select',{schema_version:1,event_id:crypto.randomUUID(),expected_revision:current.revision,work:current.id,task:$('memory-task').value,selections,reason:$('memory-selection-reason').value}).catch(error=>message(error.message,true))});
 $('memory-apply').addEventListener('click',async()=>{if(!pending)return;const operation=pending;$('memory-apply').disabled=true;try{await api('/api/v1/project-memory?action='+operation.action,operation.q);await refresh();$('memory-refresh').focus();message(tr('Décision enregistrée. Les anciens contextes sont conservés.'))}catch(error){message(error.message,true)}finally{$('memory-apply').disabled=false}});
 $('memory-cancel').addEventListener('click',()=>{resetPreview();$('memory-examine').focus()});$('memory-new').addEventListener('click',newLesson);
 $('memory-editor').addEventListener('input',resetPreview);$('memory-selection').addEventListener('input',resetPreview);$('memory-task').addEventListener('change',()=>{chosen=[...(selectedTask()?.memory_selections||[])];resetPreview();restoreSelection()});
 $('memory-clear').addEventListener('click',()=>{chosen=[];for(const n of document.querySelectorAll('[data-memory-choice]'))n.checked=false;resetPreview();message(tr('Sélection vidée dans le formulaire. Examinez puis appliquez pour enregistrer.'))});
 $('memory-source').addEventListener('change',()=>{$('memory-body').value=current?.retex?.find(r=>r.id===$('memory-source').value)?.proposal||'';resetPreview()});
 $('memory-work').addEventListener('change',()=>chooseWork($('memory-work').value).catch(error=>message(error.message,true)));
 for(const id of ['memory-search','memory-refresh'])$(id).addEventListener(id==='memory-search'?'submit':'click',event=>{event.preventDefault();refresh().catch(error=>message(error.message,true))});
 const setTheme=value=>{document.documentElement.dataset.theme=value;try{localStorage.setItem('swarm-theme',value)}catch{}};
 try{setTheme(localStorage.getItem('swarm-theme')||'etat')}catch{}$('memory-theme').addEventListener('click',()=>setTheme(document.documentElement.dataset.theme==='etat'?'sombre':'etat'));
 document.addEventListener('keydown',event=>{if(event.key==='Escape'&&pending){resetPreview();$('memory-examine').focus()}});
 api('/api/v1/session').then(session=>{csrf=session.csrf;$('memory-work').disabled=false;$('memory-refresh').disabled=false;return refresh()}).catch(error=>message(error.message,true));
})();
