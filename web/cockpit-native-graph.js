'use strict';
(function () {
  const t = (source) => globalThis.SwarmI18n?.t ? globalThis.SwarmI18n.t(source) : source;
  const SVG_NS = 'http://www.w3.org/2000/svg';
  const VIEW_PREFIX = 'swarm-native-graph-view:v1:';
  let refreshWork = null;

  /** Copy only the editable definition of a recorded/staged task. Never copy
   * status, review, model, authority, instructions or consumption into a clone.
   */
  function taskDefinition(task){return{id:task.id,title:task.title,deliverable:task.deliverable||'',criteria:[...(task.criteria||[])],depends:[...(task.depends||[])]}}

  /** Validate presentation data from authenticated responses or decrypted local
   * storage without granting engine authority. Native preview validates meaning.
   * Unknown fields, malformed lists and ambiguous identities refuse as a whole.
   */
  function graphChangesValid(changes,removals=false){const ids=new Set(),core=value=>value&&typeof value==='object'&&!Array.isArray(value)&&Object.keys(value).length===5&&['id','title','deliverable'].every(k=>typeof value[k]==='string')&&value.id.length>0&&['criteria','depends'].every(k=>Array.isArray(value[k])&&value[k].every(v=>typeof v==='string'));return Array.isArray(changes)&&changes.every(change=>{if(!change||Object.keys(change).length!==2||!(change.before===null||core(change.before))||!(change.after===null&&removals||core(change.after))||!change.before&&!change.after||change.before&&change.after&&change.before.id!==change.after.id)return false;const id=(change.after||change.before).id;if(ids.has(id))return false;ids.add(id);return true})}

  /** Validate the complete decrypted record before restoring any proposal.
   * Corruption stays visible; no missing/false/null record is accepted as an
   * empty success. Accepted edits require successful local persistence.
   */
  function businessRecordValid(stored,work){if(!stored||typeof stored!=='object'||Array.isArray(stored))return false;const last=stored.last_batch,pending=stored.pending;return Object.keys(stored).length===7&&stored.schema_version===1&&stored.work===work&&Number.isSafeInteger(stored.base_revision)&&stored.base_revision>0&&graphChangesValid(stored.changes)&&typeof stored.event_id==='string'&&stored.event_id.length>0&&(last===null||last&&Object.keys(last).length===2&&typeof last.event_id==='string'&&Number.isSafeInteger(last.revision)&&last.revision>0)&&(pending===null||pending&&Object.keys(pending).length===6&&pending.schema_version===1&&typeof pending.event_id==='string'&&Number.isSafeInteger(pending.expected_revision)&&pending.expected_revision>0&&graphChangesValid(pending.changes)&&typeof pending.undo_of==='string'&&/^[a-f0-9]{64}$/.test(pending.preview_digest)&&!!pending.undo_of===!pending.changes.length)}

  /** Open the manual graph batch workbench. Drafts are isolated per Work and
   * browser session: local storage contains AES-GCM ciphertext only, using a
   * session-derived key which is never stored. Rotation can make old drafts
   * unreadable; storage/crypto refusals remain visible and preserve the original.
   * Preview is read-only; application/recovery reuses one exact event and the
   * common engine transaction. Undo comes only from the last recorded batch.
   * No provider, automatic retry, server draft or hierarchical edit is implied.
   * @param {object} work protected recorded Work; never a fabricated base
   */
  async function openBusinessDraft(work){
    const returnFocus=document.activeElement,dialog=document.createElement('dialog');dialog.className='native-graph-editor native-graph-batch';dialog.setAttribute('aria-labelledby','native-graph-batch-title');
    const header=document.createElement('header'),heading=document.createElement('h2');heading.id='native-graph-batch-title';heading.textContent=t('Brouillon multi-tâches');
    const close=document.createElement('button');close.type='button';close.textContent=t('Fermer');header.append(heading,close);
    const body=document.createElement('div');body.className='modal-body';
    const intro=document.createElement('p');intro.textContent=t('Préparez les tâches et leurs liens, examinez l’aperçu, puis appliquez le lot en une seule opération. Aucun agent n’est lancé.');
    const status=document.createElement('p');status.id='native-graph-batch-status';status.setAttribute('role','status');
    const error=document.createElement('p');error.id='native-graph-batch-error';error.className='notice alert';error.setAttribute('role','alert');error.hidden=true;
    const storageStatus=document.createElement('p');storageStatus.id='native-graph-batch-storage';storageStatus.setAttribute('role','status');
    const controls=document.createElement('div');controls.className='pilot-actions';
    const label=document.createElement('label');label.htmlFor='native-graph-batch-task';label.textContent=t('Tâche à modifier ou dupliquer');const taskChoice=document.createElement('select');taskChoice.id='native-graph-batch-task';label.append(taskChoice);
    const list=document.createElement('ol');list.id='native-graph-batch-changes';
    const previewBox=document.createElement('section');previewBox.id='native-graph-batch-preview';previewBox.hidden=true;
    let base=work,record={schema_version:1,work:work.id,base_revision:work.revision,changes:[],event_id:'',last_batch:null,pending:null},preview=null,busy=true,session=null,key=null,storageKey=null,blockedStorage=false,ownsDraft=false,discardAllowed=false,observedCipher=null,releaseDraft=()=>{};
    const actionButtons=[];
    /** Add a named action; actual listeners own errors and busy state. */
    function action(id,text,handler){const button=document.createElement('button');button.type='button';button.id=id;button.textContent=t(text);button.addEventListener('click',()=>run(handler));actionButtons.push(button);controls.append(button);return button}
    /** Render known safe diagnostic codes/families, never server/storage bodies
     * or secrets. User identities are appended separately as literal text.
     */
    function fail(message,code=''){const reasons={'task-dependency-cycle':'Cycle de dépendances : retirez un lien du brouillon.','task-dependency-missing':'Un prérequis n’existe pas dans le plan proposé.','graph-batch-active-work':'Une activité est en cours. Attendez sa fin avant de modifier le plan.','graph-authorization-required':'Vous ne disposez pas du droit requis pour cette opération.','graph-authorization-scope':'Le brouillon dépasse votre périmètre autorisé.','graph-undo-not-last-batch':'Le plan a changé depuis ce lot ; son annulation est refusée.','graph-undo-task-used':'Ces tâches ne sont plus reconnues comme vierges avec la configuration courante. Leur retrait est refusé.','graph-local-draft-in-use':'Ce brouillon est ouvert dans un autre onglet. Fermez son atelier avant de le modifier ici.','graph-local-draft-stale':'La copie locale a changé. Fermez puis rouvrez l’atelier avant de reprendre.','graph-local-crypto-unavailable':'Ce navigateur ne permet pas le brouillon sécurisé. Utilisez un navigateur récent sur l’adresse locale.','graph-local-locks-unavailable':'Ce navigateur ne permet pas de protéger le brouillon entre onglets. Utilisez un navigateur récent.'};error.hidden=false;error.textContent=t(reasons[code]||message)+(code?' ('+code+')':'')}
    /** Serialize workbench transitions. Disabling controls prevents overlapping
     * staging, encryption, preview or writes; no operation is retried here.
     */
    async function run(handler){if(busy)return;busy=true;render();try{await handler()}catch(cause){fail('Opération non confirmée. Le brouillon est conservé.',/^[a-z][a-z0-9_.-]*$/.test(cause?.message)?cause.message:storageErrorFamily(cause))}finally{busy=false;render()}}
    /** Return a display projection without replacing the confirmed graph. Only
     * editable fields are staged; engine preview remains authoritative.
     */
    function projection(){const tasks=base.tasks.map(task=>({...task}));for(const change of record.changes){const i=tasks.findIndex(task=>task.id===change.after.id);if(i<0)tasks.push({...change.after,status:'todo'});else tasks[i]={...tasks[i],...change.after}}return{...base,tasks}}
    /** Hold an origin/Work/session Web Lock until the dialog closes. This is
     * record mutual exclusion, not a worker concurrency policy. Contention
     * refuses immediately instead of silently overwriting another editor.
     * Browser context destruction also releases the lease; no timer/retry.
     */
    function claimDraft(){return new Promise((resolve,reject)=>{const lifetime=new Promise(done=>{releaseDraft=done});let delivered=false;navigator.locks.request(storageKey,{ifAvailable:true},async lock=>{if(!lock){delivered=true;resolve(false);return}ownsDraft=true;delivered=true;resolve(true);await lifetime;ownsDraft=false}).catch(cause=>{ownsDraft=false;if(!delivered)reject(cause);else if(dialog.isConnected){session=null;blockedStorage=true;discardAllowed=false;fail('Protection locale perdue. Aucun nouveau lot ne sera envoyé.',storageErrorFamily(cause));render()}})})}
    /** Encrypt and retain this exact record through acknowledged localStorage
     * writes before a POST. Browser storage is not a server backup/fsync proof. A
     * refusal never overwrites an unreadable record or claims successful saving.
     * Pending application is retained for explicit idempotent recovery.
     */
    async function persist(){
      if(blockedStorage||!key||!storageKey||!ownsDraft){storageStatus.textContent=t('Brouillon non mémorisé. Aucun nouveau lot ne sera envoyé.');return false}
      try{const iv=crypto.getRandomValues(new Uint8Array(12)),bytes=new TextEncoder().encode(JSON.stringify(record)),cipher=await crypto.subtle.encrypt({name:'AES-GCM',iv},key,bytes),encoded=JSON.stringify({schema_version:1,iv:Array.from(iv),cipher:Array.from(new Uint8Array(cipher))});if(localStorage.getItem(storageKey)!==observedCipher){blockedStorage=true;discardAllowed=false;fail('La copie locale a changé. Fermez puis rouvrez l’atelier avant de reprendre.','graph-local-draft-stale');return false}localStorage.setItem(storageKey,encoded);observedCipher=encoded;storageStatus.textContent=t('Brouillon chiffré conservé sur cet appareil pour cette session.');return true}catch(cause){storageStatus.textContent=t('Brouillon non mémorisé. Aucun nouveau lot ne sera envoyé.')+' ('+storageErrorFamily(cause)+')';return false}
    }
    /** Accept a draft edit only after local persistence. Failure keeps the old
     * accepted record and the task form input; no dirty memory cache can later
     * replace a newer disk record. Caller invalidates its own preview/status.
     */
    async function reviseLocal(update){const before=record,next=structuredClone(record);update(next);record=next;if(!await persist()){record=before;throw Error('graph-local-save-unconfirmed')}}
    /** Stage one complete definition against the original recorded base. Editing
     * an added card replaces its proposal; returning to the original removes it.
     * Changes invalidate preview, never trigger a server write.
     */
    async function stage(id,value,{creating=false}={}){const proposed=record.changes.find(change=>change.after.id===id),previous=base.tasks.find(task=>task.id===id);if(creating&&projection().tasks.some(task=>task.id===id))throw Error('graph-draft-task-id-duplicate');if(!creating&&!proposed&&!previous)throw Error('graph-draft-task-missing');const before=creating?null:proposed?structuredClone(proposed.before):taskDefinition(previous),after={id,...value};await reviseLocal(next=>{next.changes=next.changes.filter(c=>c.after.id!==id);if(JSON.stringify(before)!==JSON.stringify(after))next.changes.push({before,after})});preview=null;render()}
    const add=action('native-graph-batch-add','Préparer une tâche',()=>editTask(projection(),null,undefined,{stage}));
    const edit=action('native-graph-batch-edit','Modifier dans le brouillon',()=>{const task=projection().tasks.find(task=>task.id===taskChoice.value);if(task)editTask(projection(),task,undefined,{stage})});
    const duplicate=action('native-graph-batch-duplicate','Dupliquer dans le brouillon',()=>{const task=projection().tasks.find(task=>task.id===taskChoice.value);if(task)editTask(projection(),null,task.depends,{stage,seed:taskDefinition(task)})});
    const clear=action('native-graph-batch-clear','Vider le brouillon',async()=>{await reviseLocal(next=>{next.changes=[];next.event_id='graph-'+crypto.randomUUID()});preview=null;error.hidden=true;status.textContent=t('Brouillon vidé. Aucune modification appliquée.');});
    const rebase=action('native-graph-batch-rebase','Actualiser la base du brouillon',async()=>{
      preview=null;
      const response=await fetch('/api/v1/planning?'+new URLSearchParams({work:work.id}),{credentials:'same-origin'}),observed=await response.json();if(!response.ok||observed.id!==work.id||!Number.isSafeInteger(observed.revision))throw Error('graph-base-read-unconfirmed');validateWork(observed);
      const conflicts=record.changes.filter(c=>c.before?!observed.tasks.some(t=>t.id===c.before.id):observed.tasks.some(t=>t.id===c.after.id));if(conflicts.length){fail('Reprise impossible : retirez ou renommez les propositions dont la tâche a été créée ou supprimée.');error.append(document.createTextNode(' ('+conflicts.map(c=>c.after.id).join(', ')+')'));return}
      await reviseLocal(next=>{next.changes=next.changes.map(c=>({...c,before:c.before?taskDefinition(observed.tasks.find(t=>t.id===c.before.id)):null}));next.base_revision=observed.revision;next.event_id='graph-'+crypto.randomUUID()});base=observed;error.hidden=true;status.textContent=t('Base actualisée. Examinez le nouvel aperçu avant toute application.');
    });
    /** Send a strict protected read/write request. Failure codes are rendered
     * safely; only explicitly non-recorded client refusals permit editing again.
     */
    async function send(path,request){const response=await fetch(path,{method:'POST',credentials:'same-origin',headers:{'Content-Type':'application/json','X-Swarm-CSRF':session.csrf_token},body:JSON.stringify({work:work.id,request})});const result=await response.json();if(!response.ok){const code=result.failure?.reason||result.failure?.code||result.error,cause=Error(typeof code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(code)?code:'graph-response-invalid');cause.refused=!result.recorded_work&&[400,403,409,422,501].includes(response.status);throw cause}return result}
    /** Read-only engine preview. Any failure invalidates the previous preview,
     * including a refused undo; no stale proposal remains applicable.
     */
    async function inspect(undo=false){preview=null;error.hidden=true;const request={schema_version:1,event_id:record.event_id,expected_revision:record.base_revision,changes:undo?[]:record.changes,undo_of:undo?record.last_batch.event_id:'',preview_digest:''};const result=await send('/api/v1/graph-batch/preview',request);if(result.work!==work.id||result.revision!==record.base_revision||!graphChangesValid(result.changes,true)||!result.changes.length||typeof result.preview_digest!=='string'||!/^[a-f0-9]{64}$/.test(result.preview_digest)||result.no_implicit_launch!==true)throw Error('graph-preview-unconfirmed');preview={request:{...request,preview_digest:result.preview_digest},changes:result.changes};status.textContent=t('Aperçu validé. Aucune modification appliquée.');}
    const inspectButton=action('native-graph-batch-inspect','Prévisualiser le lot',()=>inspect());
    const undo=action('native-graph-batch-undo','Prévisualiser l’annulation du dernier lot',()=>inspect(true));
    /** Apply/replay only the exact explicitly previewed request. Persist before
     * POST; unknown outcomes freeze editing. Explicit recovery reuses the same
     * event and bytes so Root replay cannot duplicate an applied batch.
     */
    async function apply(recovery=false){
      const request=recovery?record.pending:preview?.request;if(!request)return;
      if(!recovery)record.pending=structuredClone(request);
      if(!await persist()){if(!recovery)record.pending=null;fail('Le brouillon n’a pas pu être conservé. Aucun lot n’a été envoyé.');return}
      error.hidden=true;status.textContent=t('Application du lot en cours…');
      try{
        const result=await send('/api/v1/graph-batch/apply',request);
        if(result.event_id!==request.event_id||result.applied_revision!==request.expected_revision+1||result.work?.id!==work.id||!Number.isSafeInteger(result.work.revision)||result.work.revision<result.applied_revision||typeof result.replayed!=='boolean')throw Error('graph-apply-unconfirmed');
        const response=await fetch('/api/v1/planning?'+new URLSearchParams({work:work.id}),{credentials:'same-origin'}),observed=await response.json();if(!response.ok||observed.id!==work.id||observed.revision<result.applied_revision)throw Error('graph-readback-unconfirmed');validateWork(observed);
        if(!refreshWork)throw Error('web-native-graph-refresh-unavailable');await refreshWork(work.id,observed.revision);
        base=observed;record.base_revision=observed.revision;record.last_batch=!request.undo_of&&observed.revision===result.applied_revision?{event_id:request.event_id,revision:result.applied_revision}:null;record.changes=[];record.pending=null;record.event_id='graph-'+crypto.randomUUID();preview=null;await persist();status.textContent=t(request.undo_of?'Annulation enregistrée. L’historique est conservé.':'Lot enregistré et plan relu.');
      }catch(cause){if(cause.refused){record.pending=null;preview=null;await persist();fail('Le moteur a refusé le lot. Actualisez sa base et examinez un nouvel aperçu.',cause.message)}else{fail('Résultat incertain. Reprenez la même demande sans doublon avant de modifier le brouillon.',/^[a-z][a-z0-9_.-]*$/.test(cause?.message)?cause.message:'graph-outcome-unknown')}}
    }
    const applyButton=action('native-graph-batch-apply','Appliquer explicitement',()=>apply());applyButton.className='primary';
    const recover=action('native-graph-batch-recover','Reprendre la même demande sans doublon',()=>apply(true));
    const discard=action('native-graph-batch-discard','Écarter le brouillon local illisible',async()=>{if(!ownsDraft||!discardAllowed)return;try{localStorage.removeItem(storageKey);observedCipher=null;blockedStorage=false;discardAllowed=false;error.hidden=true;await persist()}catch(cause){storageStatus.textContent=t('Brouillon local non effacé.')+' ('+storageErrorFamily(cause)+')'}});
    /** Present user-facing definition fields, including complete prerequisites
     * and criteria, as literal text rather than implementation JSON keys.
     */
    function definitionPanel(title,value){const panel=document.createElement('section'),heading=document.createElement('h4');heading.textContent=t(title);panel.append(heading);if(!value){const empty=document.createElement('p');empty.textContent=t('Absent');panel.append(empty);return panel}const fields=document.createElement('dl');for(const [label,text]of [['Identifiant',value.id],['Titre',value.title],['Livrable',value.deliverable],['Critères d’acceptation (un par ligne)',value.criteria.join('\n')],['Prérequis',value.depends.join(', ')||t('Aucun')]]){const name=document.createElement('dt'),content=document.createElement('dd');name.textContent=t(label);content.textContent=text;fields.append(name,content)}panel.append(fields);return panel}
    /** Render literal definitions/differences, never HTML or a replacement of
     * the confirmed graph. Selected controls and errors remain in the main flow.
     */
    function render(){
      const previous=taskChoice.value;taskChoice.replaceChildren();for(const task of projection().tasks){const option=document.createElement('option');option.value=task.id;option.textContent=task.id+' — '+task.title;taskChoice.append(option)}if([...taskChoice.options].some(o=>o.value===previous))taskChoice.value=previous;
      list.replaceChildren();for(const change of record.changes){const item=document.createElement('li'),text=document.createElement('span'),remove=document.createElement('button');text.textContent=(change.before?t('Modifier : '):t('Ajouter : '))+change.after.id+' — '+change.after.title;remove.type='button';remove.textContent=t('Retirer du brouillon');remove.disabled=busy||!!record.pending||blockedStorage||!ownsDraft;remove.addEventListener('click',()=>run(async()=>{await reviseLocal(next=>{next.changes=next.changes.filter(c=>c.after.id!==change.after.id)});preview=null}));item.append(text,remove);list.append(item)}
      previewBox.replaceChildren();previewBox.hidden=!preview;if(preview){const title=document.createElement('h3');title.textContent=t(preview.request.undo_of?'Aperçu de l’annulation':'Aperçu du lot');previewBox.append(title);for(const change of preview.changes){const section=document.createElement('details'),summary=document.createElement('summary'),comparison=document.createElement('div');summary.textContent=(change.after?change.before?t('Modifier : '):t('Ajouter : '):t('Retirer : '))+(change.after||change.before).id+' — '+(change.after||change.before).title;comparison.className='native-graph-batch-comparison';comparison.append(definitionPanel('Avant',change.before),definitionPanel('Après',change.after));section.append(summary,comparison);previewBox.append(section)}}
      for(const button of actionButtons)button.disabled=busy||!!record.pending||!!base.planning||!session||blockedStorage||!ownsDraft;
      edit.disabled=edit.disabled||!taskChoice.value;duplicate.disabled=duplicate.disabled||!taskChoice.value;clear.disabled=clear.disabled||record.changes.length===0;inspectButton.disabled=inspectButton.disabled||record.changes.length===0;applyButton.disabled=applyButton.disabled||!preview;undo.disabled=undo.disabled||record.changes.length!==0||!record.last_batch||record.last_batch.revision!==record.base_revision||record.base_revision!==base.revision;recover.hidden=!record.pending;recover.disabled=busy||!record.pending||!session||blockedStorage||!ownsDraft;discard.hidden=!blockedStorage||!discardAllowed||!ownsDraft;discard.disabled=busy||!storageKey||!ownsDraft;close.disabled=busy;taskChoice.disabled=busy||!!record.pending||blockedStorage||!ownsDraft;dialog.setAttribute('aria-busy',String(busy));
    }
    const taskActions=document.createElement('div');taskActions.className='pilot-actions';taskActions.append(add,edit,duplicate);
    const management=document.createElement('details'),manageTitle=document.createElement('summary');management.id='native-graph-batch-options';manageTitle.textContent=t('Options du brouillon');const manageBody=document.createElement('div');manageBody.className='pilot-actions';manageBody.append(clear,rebase,discard);const recoveryHelp=document.createElement('p');recoveryHelp.textContent=t('Actualiser la base conserve vos propositions mais relit les définitions de départ. L’annulation concerne uniquement le dernier lot sans modification intermédiaire. Une rotation de session peut rendre un brouillon local illisible.');management.append(manageTitle,manageBody,recoveryHelp);
    body.append(intro,status,error,storageStatus,label,taskActions,list,management,previewBox,controls);dialog.append(header,body);document.body.append(dialog);
    close.addEventListener('click',()=>dialog.close());dialog.addEventListener('cancel',e=>{if(busy)e.preventDefault()});dialog.addEventListener('close',()=>{releaseDraft();dialog.remove();container().graphRestoreView?.();if(returnFocus?.isConnected)returnFocus.focus({preventScroll:true})});dialog.showModal();render();
    try{
      const response=await fetch('/api/v1/native-session',{credentials:'same-origin'});session=await response.json();if(!response.ok||session.csrf_format!=='hmac-sha256-v1'||typeof session.csrf_token!=='string'||!/^[a-f0-9]{64}$/.test(session.csrf_token))throw Error('web-native-session-invalid');
      if(typeof globalThis.crypto?.randomUUID!=='function'||!globalThis.crypto?.subtle)throw Error('graph-local-crypto-unavailable');if(typeof globalThis.navigator?.locks?.request!=='function')throw Error('graph-local-locks-unavailable');record.event_id='graph-'+crypto.randomUUID();
      const material=await crypto.subtle.digest('SHA-256',new TextEncoder().encode('swarm-graph-draft-key:v1:'+session.csrf_token));key=await crypto.subtle.importKey('raw',material,'AES-GCM',false,['encrypt','decrypt']);const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(work.id+':'+session.csrf_token));storageKey='swarm-native-graph-draft:v1:'+Array.from(new Uint8Array(digest),b=>b.toString(16).padStart(2,'0')).join('');
      if(!await claimDraft())throw Error('graph-local-draft-in-use');
      discardAllowed=true;let stored;const saved=localStorage.getItem(storageKey);observedCipher=saved;
      if(saved!==null){const envelope=JSON.parse(saved),byte=v=>Number.isInteger(v)&&v>=0&&v<=255;if(!envelope||typeof envelope!=='object'||Object.keys(envelope).length!==3||envelope.schema_version!==1||!Array.isArray(envelope.iv)||envelope.iv.length!==12||!envelope.iv.every(byte)||!Array.isArray(envelope.cipher)||envelope.cipher.length<16||!envelope.cipher.every(byte))throw Error('graph-local-draft-invalid');const decrypted=await crypto.subtle.decrypt({name:'AES-GCM',iv:new Uint8Array(envelope.iv)},key,new Uint8Array(envelope.cipher)),diskRecord=JSON.parse(new TextDecoder().decode(decrypted));if(!businessRecordValid(diskRecord,work.id))throw Error('graph-local-draft-invalid');if(stored===undefined)stored=diskRecord}
      if(stored!==undefined){if(!businessRecordValid(stored,work.id))throw Error('graph-local-draft-invalid');record=structuredClone(stored);status.textContent=record.pending?t('Résultat incertain. Reprenez la même demande sans doublon avant de modifier le brouillon.'):record.base_revision!==work.revision?t('Le plan a changé. Actualisez la base du brouillon puis examinez un nouvel aperçu.'):t('Brouillon local restauré.')}
      if(work.planning)fail('Les lots manuels ne modifient pas une mission hiérarchique. Utilisez son parcours de révision de contrat.');
    }catch(cause){if(typeof session?.csrf_token!=='string'||!ownsDraft)session=null;blockedStorage=true;fail('Brouillon local indisponible. Vérifiez le plan avant de l’écarter.',/^[a-z][a-z0-9_.-]*$/.test(cause?.message)?cause.message:storageErrorFamily(cause))}
    finally{busy=false;render();heading.tabIndex=-1;heading.focus({preventScroll:true})}
  }

  /** Register the cockpit's asynchronous readback consumer. The callback takes
   * Work ID and acknowledged revision, resolves after observations are reread,
   * or rejects on refresh failure. It performs no mutation itself. A changed
   * selection must be preserved. Missing handlers refuse editor completion.
   * @param {(id:string, revision:number)=>Promise<void>} handler native consumer
   * @throws {TypeError} for a non-function; no existing handler is replaced then
   */
  function setRefreshHandler(handler) {
    if(typeof handler!=='function')throw new TypeError('graph-refresh-handler-required');
    refreshWork=handler;
  }

  /**
   * Text node helper using textContent (no innerHTML).
   * @param {string} text literal user-facing text
   * @returns {Text}
   */
  function textNode(text) { return document.createTextNode(text); }

  /**
   * Locate the original #graph container.
   * @returns {Element} container
   * @throws {Error} when the container is absent
   */
  function container() {
    const el = document.getElementById('graph');
    if (!el) throw new Error(t('Conteneur #graph introuvable.'));
    return el;
  }

  /**
   * Validate a Work object and its tasks/dependencies.
   * Pure check; throws on any structural defect.
   * @param {*} work candidate Work
   * @returns {{tasks: Array, depPairs: Array<{from: string, to: string}>, unknown: Array<{taskId: string, dep: string}>}}
   * @throws {Error} on invalid Work, task ids, statuses, duplicate ids, bad dependency types
   */
  function validateWork(work) {
    if (!work || typeof work !== 'object' || Array.isArray(work)) {
      throw new Error(t('Graphe : objet Work attendu.'));
    }
    if (typeof work.id !== 'string' || !work.id) throw new Error(t('Graphe : identifiant de Work manquant.'));
    if (typeof work.title !== 'string') throw new Error(t('Graphe : titre de Work manquant.'));
    if (!Array.isArray(work.tasks)) throw new Error(t('Graphe : liste de tâches attendue.'));
    const seen = new Set();
    const depPairs = [];
    const unknown = [];
    for (const task of work.tasks) {
      if (!task || typeof task !== 'object' || Array.isArray(task)) {
        throw new Error(t('Graphe : tâche invalide.'));
      }
      if (typeof task.id !== 'string' || !task.id) throw new Error(t('Graphe : identifiant de tâche manquant.'));
      if (typeof task.title !== 'string' || !task.title) throw new Error(t('Graphe : titre de tâche manquant.'));
      if (typeof task.status !== 'string' || !task.status) throw new Error(t('Graphe : état de tâche manquant.'));
      if (seen.has(task.id)) throw new Error(t('Graphe : identifiant de tâche en double : ') + task.id);
      seen.add(task.id);
    }
    const ids = seen;
    for (const task of work.tasks) {
      if (task.depends === undefined || task.depends === null) continue;
      if (!Array.isArray(task.depends)) throw new Error(t('Graphe : dépendances de tâche invalides : ') + task.id);
      for (const dep of task.depends) {
        if (typeof dep !== 'string' || !dep) throw new Error(t('Graphe : dépendance invalide pour la tâche : ') + task.id);
        if (ids.has(dep)) depPairs.push({ from: dep, to: task.id });
        else unknown.push({ taskId: task.id, dep });
      }
    }
    if (depPairs.length) detectCycles(work.tasks, depPairs);
    return { tasks: work.tasks, depPairs, unknown };
  }

  /**
   * Reject dependency cycles before layout (no arbitrary cap: full traversal).
   * @param {Array} tasks validated tasks
   * @param {Array<{from:string,to:string}>} depPairs prereq->task pairs
   * @throws {Error} when a cycle exists; remaining IDs may include downstream tasks
   */
  function detectCycles(tasks, depPairs) {
    const dependents = new Map();
    for (const e of depPairs) {
      if (!dependents.has(e.from)) dependents.set(e.from, []);
      dependents.get(e.from).push(e.to);
    }
    const indegree = new Map(tasks.map(task => [task.id, 0]));
    for (const e of depPairs) indegree.set(e.to, (indegree.get(e.to) || 0) + 1);
    const queue = tasks.filter(task => (indegree.get(task.id) || 0) === 0).map(task => task.id);
    let visited = 0;
    while (queue.length) {
      const id = queue.pop();
      visited++;
      for (const next of dependents.get(id) || []) {
        indegree.set(next, indegree.get(next) - 1);
        if (indegree.get(next) === 0) queue.push(next);
      }
    }
    if (visited !== tasks.length) {
      const remaining = tasks.map(task => task.id).filter(id => (indegree.get(id) || 0) > 0);
      throw new Error(t('Graphe : cycle de dépendances détecté entre : ') + remaining.join(', '));
    }
  }

  /**
   * Show an explicit empty-graph message in the original container.
   * @param {Element} host container
   * @param {string} message literal message
   */
  function showEmpty(host, message) {
    const canvas = document.createElement('div');
    canvas.className = 'pilot-canvas';
    const p = document.createElement('p');
    p.textContent = message;
    canvas.append(p);
    host.replaceChildren(canvas);
  }

  /** Normalize literal search text for case/accent-insensitive substring
   * matching. No regular expression is built from user input; no IO/storage.
   * @param {string} value validated task ID/title or the local input value
   * @returns {string} Unicode-decomposed, mark-free lowercase search key
   */
  function searchKey(value){return value.normalize('NFD').replace(/\p{M}/gu,'').toLowerCase()}

  /** Keep only known browser storage/JSON error families; never expose a raw
   * message, stored value or arbitrary error name in the preference diagnostic.
   */
  function storageErrorFamily(cause){return ['SecurityError','QuotaExceededError','SyntaxError','InvalidStateError','TypeError'].includes(cause?.name)?cause.name:'StorageError'}
  /** Read only this Work's versioned presentation preference. Coordinates must
   * be finite/nonnegative; unknown fields, malformed JSON and invalid direction
   * are reported, never treated as a valid empty preference. No writes occur.
   * @param {string} workId validated recorded Work ID
   * @returns {{view:object|null,error:string,errorCode?:string}} safe localized diagnostic on refusal
   * Full screen is deliberately absent from storage and always false on reload.
   */
  function readStoredView(workId){
    let raw;try{raw=localStorage.getItem(VIEW_PREFIX+workId)}catch(cause){const code=storageErrorFamily(cause);return{view:null,error:t('Vue mémorisée indisponible sur cet appareil.')+' ('+code+')',errorCode:code}}
    if(raw===null)return{view:null,error:''};
    try{const value=JSON.parse(raw),coords=['left','top','studioLeft','studioTop'],keys=new Set(['orientation','toolsOpen',...coords]);if(!value||typeof value!=='object'||Array.isArray(value)||!['LR','TB'].includes(value.orientation)||typeof value.toolsOpen!=='boolean'||!coords.every(key=>Number.isFinite(value[key])&&value[key]>=0)||!Object.keys(value).every(key=>keys.has(key)))throw Error('invalid-local-view');return{view:{work:workId,orientation:value.orientation,left:value.left,top:value.top,studioLeft:value.studioLeft,studioTop:value.studioTop,toolsOpen:value.toolsOpen,fullscreen:false},error:''}}
    catch(cause){const code=cause?.name==='SyntaxError'?'SyntaxError':'InvalidViewError';return{view:null,error:t('Vue mémorisée invalide ; navigation par défaut.')+' ('+code+')',errorCode:code}}
  }

  /** Open a task editor on one observed Work revision. Inputs remain visible on
   * refusal. Prerequisite search keeps selected options visible. Explicit submit uses scoped CSRF and the engine's task contracts;
   * no provider or automatic retry. A lost/malformed response is uncertain and
   * disables resubmission until the operator closes and refreshes. Success needs
   * a protected Work readback; the cockpit then reloads its actual observations.
   * Visible/live saving feedback and the button busy state last through that
   * entire sequence, then clear on refusal or uncertainty without enabling retry.
   * Duplication hands off after close, never mounting duplicate field identities.
   * @param {object} work recorded Work, never a fabricated snapshot
   * @param {object|null} task null creates, otherwise edits the recorded task
   * @param {string[]|undefined} proposedDependencies optional unsaved link gesture
   * @param {object} options optional definition-only clone seed or asynchronous
   * stage callback (id, definition, {creating}); staging changes only the local
   * draft, never the engine, and preserves creation intent on duplicate IDs
   */
  function editTask(work, task=null, proposedDependencies, options={}) {
    const returnFocus=document.activeElement,dialog=document.createElement('dialog');let completionFocus=null;
    dialog.className='native-graph-editor';dialog.setAttribute('aria-labelledby','native-graph-editor-title');
    const form=document.createElement('form'),header=document.createElement('header'),heading=document.createElement('h2');
    heading.id='native-graph-editor-title';heading.textContent=t(task?'Modifier la tâche':'Ajouter une tâche');
    const close=document.createElement('button');close.type='button';close.textContent=t('Fermer');close.addEventListener('click',()=>dialog.close());header.append(heading,close);
    const body=document.createElement('div');body.className='modal-body';
    const fields={};
    /** Add a named, labeled input; user values are plain text only. */
    function field(name,label,value,multiline=false){const wrap=document.createElement('label');wrap.textContent=t(label);const input=document.createElement(multiline?'textarea':'input');input.name=name;input.id='native-graph-'+name;input.value=value;input.required=true;wrap.htmlFor=input.id;wrap.append(input);body.append(wrap);fields[name]=input;return input}
    field('id','Identifiant',task?.id||'').readOnly=!!task;
    field('title','Titre',task?.title||options.seed?.title||'');field('deliverable','Livrable',task?.deliverable||options.seed?.deliverable||'');
    field('criteria','Critères d’acceptation (un par ligne)',(task?.criteria||options.seed?.criteria||[]).join('\n'),true);
    const label=document.createElement('label');label.textContent=t('Prérequis');label.htmlFor='native-graph-depends';
    const dependencies=document.createElement('select');dependencies.id='native-graph-depends';dependencies.multiple=true;
    for(const other of work.tasks||[]){if(other.id===task?.id)continue;const option=document.createElement('option');option.value=other.id;option.textContent=other.id+' — '+other.title;option.selected=!!(proposedDependencies??task?.depends)?.includes(other.id);dependencies.append(option)}label.append(dependencies);
    const dependencySearch=document.createElement('label');dependencySearch.htmlFor='native-graph-dependency-search';dependencySearch.textContent=t('Rechercher un prérequis');
    const dependencyQuery=document.createElement('input');dependencyQuery.id='native-graph-dependency-search';dependencyQuery.type='search';dependencyQuery.setAttribute('aria-controls',dependencies.id);dependencySearch.append(dependencyQuery);
    const selectedDependencies=document.createElement('output');selectedDependencies.id='native-graph-dependency-selection';selectedDependencies.setAttribute('aria-live','polite');
    /** Filter literal IDs/titles without discarding selected options. Selected
     * prerequisites remain visible outside the query; output lists exact IDs.
     * No Work mutation or selection change occurs until explicit user choice.
     * Enter in the query never submits the task form; Escape clears the query.
     */
    function filterDependencies(){const query=searchKey(dependencyQuery.value.trim());for(const option of dependencies.options)option.hidden=!option.selected&&!searchKey(option.textContent).includes(query);const ids=[...dependencies.selectedOptions].map(o=>o.value);selectedDependencies.textContent=t('Prérequis sélectionnés : ')+(ids.join(', ')||t('Aucun'))+(query&&![...dependencies.options].some(o=>searchKey(o.textContent).includes(query))?' · '+t('Aucun prérequis correspondant.'):'');}
    dependencyQuery.addEventListener('input',filterDependencies);dependencyQuery.addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();e.stopPropagation();filterDependencies()}else if(e.key==='Escape'&&dependencyQuery.value){e.preventDefault();e.stopPropagation();dependencyQuery.value='';filterDependencies()}});dependencies.addEventListener('change',filterDependencies);filterDependencies();body.append(dependencySearch,label,selectedDependencies);
    const notice=document.createElement('p');notice.className='notice info';notice.textContent=t('Choisissez les prérequis. Enregistrer modifie le plan sans lancer d’agent.');body.append(notice);
    const progress=document.createElement('p');progress.id='native-graph-progress';progress.setAttribute('role','status');progress.setAttribute('aria-live','polite');body.append(progress);
    const error=document.createElement('p');error.className='notice alert';error.setAttribute('role','alert');error.hidden=true;body.append(error);
    const footer=document.createElement('footer'),save=document.createElement('button');save.type='submit';save.className='primary';save.id='native-graph-save';save.textContent=t('Enregistrer');footer.append(save);form.append(header,body,footer);dialog.append(form);document.body.append(dialog);
    let pending=false,duplicateButton=null;
    if(options.stage){save.textContent=t('Ajouter au brouillon');notice.textContent=t('Cette étape prépare le brouillon. Seule l’application explicite du lot modifiera le plan.');}
    if(task&&!options.stage){const duplicate=document.createElement('button');duplicateButton=duplicate;duplicate.type='button';duplicate.id='native-graph-duplicate';duplicate.textContent=t('Dupliquer la tâche');duplicate.addEventListener('click',()=>{dialog.addEventListener('close',()=>editTask(work,null,task.depends,{seed:taskDefinition(task)}),{once:true});dialog.close()});footer.prepend(duplicate)}
    dialog.addEventListener('cancel',e=>{if(pending)e.preventDefault()});
    dialog.addEventListener('close',()=>{dialog.remove();container().graphRestoreView?.();if(completionFocus?.isConnected)completionFocus.focus({preventScroll:true});else if(returnFocus?.isConnected)returnFocus.focus({preventScroll:true})});
    form.addEventListener('submit',async e=>{
      if(options.stage){e.preventDefault();if(pending)return;pending=true;save.disabled=true;close.disabled=true;save.setAttribute('aria-busy','true');save.textContent=t('Préparation du brouillon…');progress.textContent=t('Préparation du brouillon…');error.hidden=true;for(const input of form.querySelectorAll('input,textarea,select'))input.disabled=true;try{await options.stage(fields.id.value,{title:fields.title.value,deliverable:fields.deliverable.value,criteria:fields.criteria.value.split('\n').map(s=>s.trim()).filter(Boolean),depends:[...dependencies.selectedOptions].map(o=>o.value)},{creating:task===null});dialog.close()}catch(cause){error.hidden=false;error.textContent=t(cause?.message==='graph-draft-task-id-duplicate'?'Cet identifiant est déjà utilisé. Choisissez un nouvel identifiant.':'Ajout au brouillon non confirmé. La saisie est conservée.')}finally{pending=false;save.disabled=false;close.disabled=false;save.setAttribute('aria-busy','false');save.textContent=t('Ajouter au brouillon');progress.textContent='';for(const input of form.querySelectorAll('input,textarea,select'))input.disabled=false;if(!error.hidden)fields.id.focus()}return}
      e.preventDefault();if(pending)return;pending=true;save.disabled=true;save.setAttribute('aria-busy','true');save.textContent=t('Enregistrement en cours…');progress.textContent=t('Enregistrement en cours…');close.disabled=true;error.hidden=true;let submitted=false;
      if(duplicateButton)duplicateButton.disabled=true;
      const request={title:fields.title.value,deliverable:fields.deliverable.value,criteria:fields.criteria.value.split('\n').map(s=>s.trim()).filter(Boolean),depends:[...dependencies.selectedOptions].map(o=>o.value)};
      const taskID=fields.id.value;for(const input of form.querySelectorAll('input,textarea,select'))input.disabled=true;
      try{
        const sessionResponse=await fetch('/api/v1/native-session',{credentials:'same-origin'}),session=await sessionResponse.json();
        if(!sessionResponse.ok||typeof session.csrf_token!=='string')throw Error('web-native-session-invalid');
        submitted=true;
        const response=await fetch('/api/v1/action',{method:'POST',credentials:'same-origin',headers:{'Content-Type':'application/json','X-Swarm-CSRF':session.csrf_token},body:JSON.stringify({kind:task?'task':'task-add',work:work.id,task:taskID,event_id:'graph-'+crypto.randomUUID(),expected_revision:work.revision,request})});
        const result=await response.json();
        if(!response.ok){const code=result.failure?.code||result.error;const refusal=Error(typeof code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(code)?code:'web-native-response-invalid');refusal.refused=!result.recorded_work&&[400,403,409,422,501].includes(response.status);throw refusal}
        if(result.id!==work.id||result.revision!==work.revision+1)throw Error('web-native-response-invalid');
        const read=await fetch('/api/v1/planning?'+new URLSearchParams({work:work.id}),{credentials:'same-origin'}),observed=await read.json(),actual=observed.tasks?.find(t=>t.id===taskID);
        if(!read.ok||observed.id!==work.id||!Number.isSafeInteger(observed.revision)||observed.revision<result.revision||!actual||actual.title!==request.title||actual.deliverable!==request.deliverable||JSON.stringify(actual.criteria)!==JSON.stringify(request.criteria)||JSON.stringify(actual.depends||[])!==JSON.stringify(request.depends))throw Error('web-native-readback-unconfirmed');
        if(!refreshWork)throw Error('web-native-graph-refresh-unavailable');
        await refreshWork(work.id,observed.revision);
        completionFocus=container().graphView?.().work===work.id?[...container().querySelectorAll('.graph-task-open')].find(n=>n.dataset.task===taskID):null;
        dialog.close();
      }catch(cause){error.hidden=false;error.textContent=t('Modification refusée ou résultat non confirmé. Fermez et actualisez avant de reprendre.')+' ('+cause.message+')';save.disabled=submitted&&!cause.refused}
      finally{pending=false;save.setAttribute('aria-busy','false');save.textContent=t('Enregistrer');progress.textContent='';close.disabled=false;if(duplicateButton)duplicateButton.disabled=false;for(const input of form.querySelectorAll('input,textarea,select'))input.disabled=false;if(!submitted)save.disabled=false}
    });
    dialog.showModal();fields[task?'title':'id'].focus();
  }

  /** Inspect a recorded dependency and prepare its replacement/removal in the
   * existing task editor. No POST occurs here; Save retains native revision,
   * rights and cycle checks. Closing restores the invoking edge's focus.
   * @param {object} work recorded Work, never a local draft projection
   * @param {string} from prerequisite ID
   * @param {string} to dependent task ID
   */
  function inspectLink(work,from,to){
    const task=work.tasks.find(task=>task.id===to);if(!task)return;
    const focus=document.activeElement,dialog=document.createElement('dialog');dialog.className='native-graph-editor native-graph-link-editor';dialog.setAttribute('aria-labelledby','native-graph-link-title');
    const header=document.createElement('header'),title=document.createElement('h2'),close=document.createElement('button');title.id='native-graph-link-title';title.textContent=t('Modifier un lien');close.type='button';close.textContent=t('Fermer');header.append(title,close);
    const body=document.createElement('div');body.className='modal-body';const relation=document.createElement('p');relation.textContent=from+' → '+to;const note=document.createElement('p');note.textContent=t('Ce lien signifie que la tâche de droite attend le prérequis de gauche. Le formulaire suivant demande une confirmation. Retirer un lien ne supprime aucune tâche.');
    const label=document.createElement('label'),choice=document.createElement('select');choice.id='native-graph-link-source';label.htmlFor=choice.id;label.textContent=t('Nouveau prérequis');for(const candidate of work.tasks){if(candidate.id===to)continue;const option=document.createElement('option');option.value=candidate.id;option.textContent=candidate.id+' — '+candidate.title;choice.append(option)}choice.value=from;label.append(choice);
    const footer=document.createElement('footer'),replace=document.createElement('button'),remove=document.createElement('button');replace.type=remove.type='button';replace.id='native-graph-link-replace';remove.id='native-graph-link-remove';replace.className='primary';replace.textContent=t('Préparer le changement');remove.textContent=t('Retirer ce lien');
    /** Hand off a proposal after closing this modal, without overlapping task
     * forms or mutating the recorded Work. The task editor owns confirmation.
     */
    function propose(depends){dialog.addEventListener('close',()=>editTask(work,task,depends),{once:true});dialog.close()}
    replace.addEventListener('click',()=>propose([...new Set([...(task.depends||[]).filter(id=>id!==from),choice.value])]));remove.addEventListener('click',()=>propose((task.depends||[]).filter(id=>id!==from)));close.addEventListener('click',()=>dialog.close());dialog.addEventListener('close',()=>{dialog.remove();container().graphRestoreView?.();if(focus?.isConnected)focus.focus({preventScroll:true})});footer.append(replace,remove);body.append(relation,note,label);dialog.append(header,body,footer);document.body.append(dialog);dialog.showModal();choice.focus();
  }

  /** Mount local presentation controls around a validated recorded graph.
   * Coordinates/zoom are device preferences, scoped to origin, Work and direction;
   * they grant no mutation or execution rights. Storage failure remains visible.
   * Native modal semantics own focus/inertness/Escape in fullscreen. Full-screen
   * state and undo/redo stay in memory; validated navigation fields may persist.
   * Explicit SVG export initiates a local download
   * and reports generation failures without a server write or saved-file claim.
   * @param {Element} host existing graph container, replaced after mounting
   * @param {HTMLElement} canvas scroll viewport containing the recorded SVG
   * @param {SVGElement} svg validated Dagre layout, dimensions in SVG units
   * @param {object} work recorded Work with validated tasks/dependencies
   * @param {'LR'|'TB'} orientation displayed layout direction
   * @param {object|null} initial same-Work view from memory or validated local storage
   * @param {string} viewError safe preference diagnostic, never raw stored content
   * @param {string} viewErrorCode safe browser/validation family for inspection
   * @returns {void} installs listeners, view capture and cleanup; storage refusals stay visible
   */
  function navigation(host, canvas, svg, work, orientation, initial=null, viewError='', viewErrorCode='') {
    const studio=document.createElement('section');studio.id='graph-studio';studio.className='graph-studio native-graph-studio';
    const toolbar=document.createElement('div');toolbar.className='pilot-actions';
    const status=document.createElement('p');status.className='graph-studio-hint';status.setAttribute('role','status');
    const key='swarm-native-graph:v1:'+work.id+':'+orientation,viewKey=VIEW_PREFIX+work.id;
    const preferenceStatus=document.createElement('p');preferenceStatus.id='pilot-view-status';preferenceStatus.className='native-graph-preference-status';preferenceStatus.setAttribute('role','status');preferenceStatus.textContent=viewError||initial?.preferenceError||'';if(viewErrorCode||initial?.preferenceErrorCode)preferenceStatus.dataset.viewError=viewErrorCode||initial.preferenceErrorCode;
    const media=matchMedia('(min-width:761px)'),tools=document.createElement('details');tools.id='pilot-tools';tools.className='native-graph-tools';tools.open=initial?.toolsOpen??media.matches;
    const toolsToggle=document.createElement('summary');toolsToggle.id='pilot-tools-toggle';toolsToggle.textContent=t('Outils du graphe');const toolsBody=document.createElement('div');toolsBody.className='pilot-actions';tools.append(toolsToggle,toolsBody);
    let lastViewAttempt=null,resetting=false;
    let visibleIds=null,selectionMode=false;
    const selectedIds=new Set();
    const successors=new Map(work.tasks.map(task=>[task.id,[]])),predecessors=new Map(work.tasks.map(task=>[task.id,[]]));
    for(const task of work.tasks)for(const dep of task.depends||[])if(successors.has(dep)){successors.get(dep).push(task.id);predecessors.get(task.id).push(dep)}
    let positions=Object.create(null),previousPositions=null,redoPositions=null,zoom=1,gesture=null,connection=null,disposed=false,copying=false,exportUrl=null,resumeFullscreen=!!initial?.fullscreen;
    let pendingScroll=initial?.orientation===orientation?{left:initial.left,top:initial.top}:null;
    let pendingStudio=initial?{left:initial.studioLeft??0,top:initial.studioTop??0}:null;
    try{const saved=JSON.parse(localStorage.getItem(key)||'null');if(saved){if(Number.isFinite(saved.zoom))zoom=Math.max(.15,Math.min(2,saved.zoom));for(const [id,p]of Object.entries(saved.positions||{}))if(p&&Number.isFinite(p.x)&&Number.isFinite(p.y))positions[id]={x:p.x,y:p.y}}}catch{status.textContent=t('Positions indisponibles ; placement automatique conservé.')}
    if(initial?.orientation===orientation&&Number.isFinite(initial.zoom)&&initial.positions){zoom=initial.zoom;positions=copyPositions(initial.positions)}
    const base={width:Number(svg.getAttribute('width')),height:Number(svg.getAttribute('height'))},units=new Map();
    const extent={width:base.width,height:base.height};
    const map=svgNode('svg',{class:'graph-minimap',role:'group',tabindex:0,'aria-label':t('Naviguer avec la mini-carte')});canvas.append(map);
    const viewport=svgNode('rect',{class:'graph-minimap-viewport','aria-hidden':'true'});
    canvas.id='pilot-canvas';canvas.classList.add('native-graph-canvas');
    const modal=document.createElement('dialog');modal.className='native-graph-dialog';modal.setAttribute('aria-label',t('Graphe du travail'));
    /** Persist local zoom, completed placement and navigation as separate keys;
     * captured drags stay provisional. A navigation-key refusal does not undo a
     * successful coordinate write. Each refusal is visible, never a success. */
    function save(){try{localStorage.setItem(key,JSON.stringify({zoom,positions:committedPositions()}));persistView()}catch{status.textContent=t('Positions non mémorisées sur cet appareil.')}}
    /** Encode only validated presentation fields, never full screen or task data.
     * Pending restore/gesture values stay committed. This constant-size record
     * does not clone card positions on each scroll event.
     */
    function viewRecord(){if(host.graphView!==captureView)return null;return{orientation,left:pendingScroll?.left??(gesture&&!gesture.id&&!gesture.link?gesture.left:canvas.scrollLeft),top:pendingScroll?.top??(gesture&&!gesture.id&&!gesture.link?gesture.top:canvas.scrollTop),studioLeft:pendingStudio?.left??studio.scrollLeft,studioTop:pendingStudio?.top??studio.scrollTop,toolsOpen:tools.open}}
    /** Persist a changed local view, with no retry of an identical attempt.
     * Force is reserved for an explicit orientation change. Refusal is visible;
     * success clears only the preference diagnostic, not other graph messages.
     */
    function persistView(force=false){if(disposed||resetting)return;const value=viewRecord();if(!value)return;const encoded=JSON.stringify(value);if(!force&&encoded===lastViewAttempt)return;lastViewAttempt=encoded;try{localStorage.setItem(viewKey,encoded);preferenceStatus.textContent='';delete preferenceStatus.dataset.viewError}catch(cause){const code=storageErrorFamily(cause);preferenceStatus.dataset.viewError=code;preferenceStatus.textContent=t('Navigation non mémorisée sur cet appareil.')+' ('+code+')'}}
    /** Copy only owned coordinate entries, including prototype-shaped IDs.
     * @param {object} source local positions, never Work data
     * @returns {object} independent null-prototype coordinate snapshot
     */
    function copyPositions(source=positions){const result=Object.create(null);for(const [id,p]of Object.entries(source))result[id]={x:p.x,y:p.y};return result}
    /** Snapshot completed coordinates for persistence and rerender consumers.
     * A captured drag supplies its original full placement; callers receive an
     * independent copy. No storage, engine write or gesture completion occurs.
     */
    function committedPositions(){return copyPositions(gesture?.id?gesture.snapshot:positions)}
    /** Retain one previous placement only after a real completed change.
     * Cancellation/no-op preserve the previous undo; absent and zero offsets
     * describe the same placement. Undo/redo memory resets on rerender;
     * neither history nor a task operation is persisted by this helper.
     */
    function rememberPlacement(before){const ids=new Set([...Object.keys(before),...Object.keys(positions)]);if([...ids].some(id=>(before[id]?.x??0)!==(positions[id]?.x??0)||(before[id]?.y??0)!==(positions[id]?.y??0))){previousPositions=before;redoPositions=null}}
    /** Traverse recorded adjacency once, including the origin; no recursion,
     * path enumeration, IO or mutation. Unknown IDs return an empty set.
     * @param {string} origin validated select value
     * @param {Map<string,string[]>} adjacency forward or reverse recorded links
     * @returns {Set<string>} transitive reachable IDs in O(V+E)
     */
    function reachable(origin,adjacency){const seen=new Set(),queue=adjacency.has(origin)?[origin]:[];while(queue.length){const id=queue.pop();if(seen.has(id))continue;seen.add(id);for(const next of adjacency.get(id))if(!seen.has(next))queue.push(next)}return seen}
    /** Test only the ephemeral analyst projection; no Work or geometry changes. */
    function isVisible(id){return visibleIds===null||visibleIds.has(id)}
    /** Apply one common translation to a gesture's original group. Clamp the
     * group at SVG origin without changing relative distances. Intermediate
     * positions remain provisional until finish; caller owns update/save.
     * @param {object} snapshot independent before-placement map
     * @param {string[]} ids recorded visible movement group
     * @param {number} dx requested horizontal displacement in SVG units
     * @param {number} dy requested vertical displacement in SVG units
     */
    function translateGroup(snapshot,ids,dx,dy){for(const id of ids){dx=Math.max(dx,-units.get(id).x-(snapshot[id]?.x??0));dy=Math.max(dy,-units.get(id).y-(snapshot[id]?.y??0))}for(const id of ids)positions[id]={x:(snapshot[id]?.x??0)+dx,y:(snapshot[id]?.y??0)+dy}}
    /** Selected cards move together; an unselected handle moves only itself. */
    function movementIds(id){return selectedIds.has(id)?[...selectedIds].filter(isVisible):[id]}
    /** Project native scroll bounds into unscaled SVG coordinates, clamped to
     * the graph extent. Only the minimap viewport marker changes; no IO/layout.
     */
    function updateViewport(){const x=Math.min(extent.width,canvas.scrollLeft/zoom),y=Math.min(extent.height,canvas.scrollTop/zoom);for(const [key,value]of Object.entries({x,y,width:Math.max(0,Math.min(canvas.clientWidth/zoom,extent.width-x)),height:Math.max(0,Math.min(canvas.clientHeight/zoom,extent.height-y))}))viewport.setAttribute(key,value)}
    /** Recompute moved curves/extent and mirror zoom/undo/redo availability.
     * Presentation only; this helper performs no storage or engine operation. */
    function update(){
      let width=base.width,height=base.height;
      for(const [id,u]of units){const p=positions[id]||{x:0,y:0};u.group.setAttribute('transform',`translate(${p.x} ${p.y})`);width=Math.max(width,u.x+p.x+u.w+30);height=Math.max(height,u.y+p.y+u.h+30)}
      for(const edge of svg.querySelectorAll('.graph-arete')){const a=units.get(edge.dataset.from),b=units.get(edge.dataset.to);if(!a||!b)continue;const ap=positions[edge.dataset.from]||{x:0,y:0},bp=positions[edge.dataset.to]||{x:0,y:0},vertical=orientation==='TB';
        const x1=a.x+ap.x+(vertical?a.w/2:a.w),y1=a.y+ap.y+(vertical?a.h:a.h/2),x2=b.x+bp.x+(vertical?b.w/2:0),y2=b.y+bp.y+(vertical?0:b.h/2),d=Math.max(60,Math.abs(vertical?y2-y1:x2-x1)/2);
        edge.setAttribute('d',vertical?`M${x1},${y1} C${x1},${y1+d} ${x2},${y2-d} ${x2},${y2}`:`M${x1},${y1} C${x1+d},${y1} ${x2-d},${y2} ${x2},${y2}`);
      }
      extent.width=width;extent.height=height;svg.setAttribute('viewBox',`0 0 ${width} ${height}`);svg.setAttribute('width',width*zoom);svg.setAttribute('height',height*zoom);
      map.setAttribute('viewBox',`0 0 ${width} ${height}`);map.replaceChildren();
      for(const edge of svg.querySelectorAll('.graph-arete'))if(isVisible(edge.dataset.from)&&isVisible(edge.dataset.to))map.append(svgNode('path',{d:edge.getAttribute('d'),class:'graph-minimap-edge'}));
      for(const [id,u]of units){if(!isVisible(id))continue;const p=positions[id]||{x:0,y:0};map.append(svgNode('rect',{x:u.x+p.x,y:u.y+p.y,width:u.w,height:u.h,rx:8,class:'graph-minimap-node','data-task':id}))}
      map.append(viewport);updateViewport();undo.disabled=previousPositions===null;redo.disabled=redoPositions===null;zoomOut.disabled=zoom<=.15;zoomIn.disabled=zoom>=2;
      zoomText.textContent=Math.round(zoom*100)+' %';
    }
    /** Add a local decorative vector without changing a control's text/name.
     * Paths are static icon geometry, never external resources or user content.
     */
    function decorate(control,kind){const paths={previous:'M15 4l-8 8 8 8',next:'M9 4l8 8-8 8',clear:'M5 5l14 14M19 5L5 19',zoomPlus:'M3 10a7 7 0 1 0 14 0a7 7 0 1 0-14 0M15 15l6 6M6 10h8M10 6v8',zoom:'M3 10a7 7 0 1 0 14 0a7 7 0 1 0-14 0M15 15l6 6M6 10h8',plus:'M12 4v16M4 12h16',fit:'M8 3H3v5M16 3h5v5M3 16v5h5M21 16v5h-5M8 12h8M12 8v8',tools:'M4 4h6v6H4zM14 14h6v6h-6zM14 4h6v6h-6zM7 10v7h7',undo:'M8 4L3 9l5 5M3 9h10a7 7 0 0 1 0 14',help:'M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M9 9a3 3 0 1 1 4 3l-1 1v1M12 17v1',draft:'M5 3h14v18H5zM8 7h8M8 11h8M8 15h5'};const icon=svgNode('svg',{class:'native-graph-tool-icon',viewBox:'0 0 24 24','aria-hidden':'true',focusable:'false'});icon.append(svgNode('path',{d:paths[kind]||paths.tools,fill:'none',stroke:'currentColor','stroke-width':2,'stroke-linecap':'round','stroke-linejoin':'round'}));control.prepend(icon)}
    /** Create a labelled native action with a decorative icon and listener. */
    function button(id,label,action){const b=document.createElement('button');b.type='button';b.id=id;actionLabel(b,label);b.addEventListener('click',action);toolbar.append(b);return b}
    /** Keep a visible desktop label and an accessible compact label on mobile.
     * Titles describe icon-only controls; no action or authority changes here.
     */
    function actionLabel(control,label){const id=control.id,kind=id==='pilot-search-previous'?'previous':id==='pilot-search-next'?'next':id==='pilot-search-clear'?'clear':id==='pilot-zoom-in'?'zoomPlus':id==='pilot-zoom-100'?'fit':id.includes('zoom')?'zoom':id==='pilot-add-task'?'plus':id.includes('fit')||id.includes('fullscreen')?'fit':id.includes('undo')||id.includes('redo')?'undo':id.includes('help')?'help':id.includes('draft')?'draft':'tools';control.replaceChildren();const text=document.createElement('span');text.className='native-graph-action-label';text.textContent=t(label);control.append(text);control.title=t(label);decorate(control,kind)}
    const zoomText=document.createElement('output');zoomText.setAttribute('aria-label',t('Zoom'));
    const zoomOut=button('pilot-zoom-out','Réduire le zoom',()=>{zoom=Math.max(.15,zoom-.15);update();save()});
    toolbar.append(zoomText);
    const zoomIn=button('pilot-zoom-in','Agrandir le zoom',()=>{zoom=Math.min(2,zoom+.15);update();save()});
    /** Reset to the existing 100% scale around the viewport center, under the
     * native scroll clamp. Placement and Work are untouched; preferences save.
     */
    function zoom100(){const x=(canvas.scrollLeft+canvas.clientWidth/2)/zoom,y=(canvas.scrollTop+canvas.clientHeight/2)/zoom;zoom=1;update();canvas.scrollLeft=x-canvas.clientWidth/2;canvas.scrollTop=y-canvas.clientHeight/2;updateViewport();save()}
    button('pilot-zoom-100','100 %',zoom100);
    /** Frame the current projection, preserving all coordinates. Filtered
     * bounds include a 12 SVG-unit visual gutter, matching the existing 24px
     * viewport gutter. Existing zoom clamps remain unchanged. No path count,
     * task or operator limit is introduced; only local zoom/scroll are saved.
     */
    function fitProjection(){update();let left=0,top=0,right=extent.width,bottom=extent.height;if(visibleIds?.size){left=Infinity;top=Infinity;right=0;bottom=0;for(const [id,u]of units){if(!isVisible(id))continue;const x=u.x+(positions[id]?.x??0),y=u.y+(positions[id]?.y??0);left=Math.min(left,x);top=Math.min(top,y);right=Math.max(right,x+u.w);bottom=Math.max(bottom,y+u.h)}left=Math.max(0,left-12);top=Math.max(0,top-12);right+=12;bottom+=12}const required=Math.min(1,(canvas.clientWidth-24)/(right-left),(canvas.clientHeight-24)/(bottom-top));zoom=Math.max(.15,required);update();canvas.scrollTo(left*zoom,top*zoom);if(required<.15)status.textContent=t('Zoom minimum atteint. Rapprochez les cartes pour tout voir.');save()}
    button('pilot-fit','Vue d’ensemble',fitProjection);
    const full=button('pilot-fullscreen','Plein écran',()=>{resumeFullscreen=false;if(modal.open){modal.close();return}modal.showModal();modal.append(studio);studio.classList.add('is-fullscreen');document.body.classList.add('graph-studio-open');actionLabel(full,'Quitter le plein écran');full.setAttribute('aria-expanded','true');full.focus()});full.setAttribute('aria-expanded','false');
    button('pilot-layout-reset','Ranger les cartes',()=>{const before=copyPositions();positions=Object.create(null);rememberPlacement(before);update();save()});
    const undo=button('pilot-layout-undo','Annuler le placement',()=>{if(!previousPositions)return;finish(true);redoPositions=copyPositions();positions=previousPositions;previousPositions=null;update();save()});undo.disabled=true;
    const redo=button('pilot-layout-redo','Rétablir le placement',()=>{if(redoPositions===null)return;finish(true);previousPositions=copyPositions();positions=redoPositions;redoPositions=null;update();save()});redo.disabled=true;
    /** Remove only the navigation preference. Existing per-direction zoom and
     * placements retain their dedicated controls. Failure changes no visible
     * view; successful reset preserves an explicitly open full screen only in
     * memory. Cleanup must not rewrite the deleted preference.
     */
    function resetNavigation(){finish(true);try{localStorage.removeItem(viewKey)}catch(cause){const code=storageErrorFamily(cause);preferenceStatus.dataset.viewError=code;preferenceStatus.textContent=t('Réinitialisation de navigation non confirmée.')+' ('+code+')';return}resetting=true;host.dataset.orientation='LR';host.graphView=()=>({work:work.id,orientation:'LR',left:0,top:0,studioLeft:0,studioTop:0,fullscreen:modal.open,toolsOpen:media.matches});render(work)}
    button('pilot-view-reset','Réinitialiser la navigation',resetNavigation);
    button('pilot-add-task','Ajouter une tâche',()=>editTask(work)).className='primary';
    button('pilot-business-draft','Brouillon multi-tâches',()=>openBusinessDraft(work));
    const neighbors=document.createElement('label');neighbors.className='native-graph-neighbors';neighbors.textContent=t('Repérer les dépendances');neighbors.htmlFor='pilot-neighbors';
    const neighborSelect=document.createElement('select');neighborSelect.id='pilot-neighbors';const noNeighbor=document.createElement('option');noNeighbor.value='';noNeighbor.textContent=t('Choisir une carte');neighborSelect.append(noNeighbor);
    for(const task of work.tasks){const option=document.createElement('option');option.value=task.id;option.textContent=task.id+' — '+task.title;neighborSelect.append(option)}neighbors.append(neighborSelect);toolbar.append(neighbors);
    const neighborStatus=document.createElement('output');neighborStatus.id='pilot-neighbor-status';neighborStatus.setAttribute('role','status');
    /** Highlight only direct recorded prerequisites/dependents. No node or link
     * is hidden, no geometry or Work is changed. Empty selection clears markers.
     */
    function showNeighbors(){const id=neighborSelect.value,task=work.tasks.find(task=>task.id===id),incoming=new Set(task?.depends||[]),outgoing=new Set(work.tasks.filter(task=>(task.depends||[]).includes(id)).map(task=>task.id));for(const [node,u]of units){u.group.classList.toggle('graph-neighbor-focus',node===id);u.group.classList.toggle('graph-neighbor-in',incoming.has(node));u.group.classList.toggle('graph-neighbor-out',outgoing.has(node))}for(const edge of svg.querySelectorAll('.graph-arete'))edge.classList.toggle('graph-neighbor-link',!!id&&(edge.dataset.to===id||edge.dataset.from===id));neighborStatus.textContent=task?id+' · '+t('Prérequis : ')+([...incoming].join(', ')||t('Aucun'))+' · '+t('Dépendants : ')+([...outgoing].join(', ')||t('Aucun')):'';clearNeighbors.disabled=!task;}
    neighborSelect.addEventListener('change',showNeighbors);const clearNeighbors=button('pilot-neighbors-clear','Effacer le repérage',()=>{neighborSelect.value='';showNeighbors();neighborSelect.focus()});clearNeighbors.disabled=true;

    const analysis=document.createElement('details');analysis.id='pilot-analysis';analysis.className='native-graph-analysis';
    const analysisToggle=document.createElement('summary');analysisToggle.id='pilot-analysis-toggle';analysisToggle.textContent=t('Analyser et sélectionner');analysis.append(analysisToggle);
    const analysisBody=document.createElement('div');analysisBody.className='pilot-actions';analysis.append(analysisBody);
    const analysisStatus=document.createElement('p');analysisStatus.id='pilot-analysis-status';analysisStatus.setAttribute('role','status');
    const selectionStatus=document.createElement('p');selectionStatus.id='pilot-selection-status';selectionStatus.setAttribute('role','status');
    /** Build a labeled native selector from recorded IDs/titles only. Values are
     * literal and never interpolated into selectors or markup. No IO.
     */
    function analystSelect(id,label){const wrap=document.createElement('label');wrap.className='native-graph-neighbors';wrap.htmlFor=id;wrap.textContent=t(label);const select=document.createElement('select');select.id=id;const empty=document.createElement('option');empty.value='';empty.textContent=t('Choisir une carte');select.append(empty);for(const task of work.tasks){const option=document.createElement('option');option.value=task.id;option.textContent=task.id+' — '+task.title;select.append(option)}wrap.append(select);analysisBody.append(wrap);return select}
    const branch=analystSelect('pilot-branch','Branche à isoler');
    const isolate=button('pilot-branch-apply','Isoler la branche',()=>{if(!successors.has(branch.value))return;const before=reachable(branch.value,predecessors),after=reachable(branch.value,successors);setProjection(new Set([...before,...after]),t('Branche isolée : ')+branch.value)});analysisBody.append(isolate);isolate.disabled=true;branch.addEventListener('change',()=>{isolate.disabled=!successors.has(branch.value)});
    const pathFrom=analystSelect('pilot-path-from','Départ du chemin'),pathTo=analystSelect('pilot-path-to','Arrivée du chemin');
    const pathButton=button('pilot-path-apply','Montrer les chemins',()=>{if(!successors.has(pathFrom.value)||!successors.has(pathTo.value))return;const forward=reachable(pathFrom.value,successors),backward=reachable(pathTo.value,predecessors);if(!forward.has(pathTo.value)){analysisStatus.textContent=t('Aucun chemin orienté entre ces tâches. La vue est conservée.');return}setProjection(new Set([...forward].filter(id=>backward.has(id))),t('Chemins orientés : ')+pathFrom.value+' → '+pathTo.value)});analysisBody.append(pathButton);pathButton.disabled=true;
    /** Enable path inspection only for two actual recorded IDs. */
    function pathAvailability(){pathButton.disabled=!successors.has(pathFrom.value)||!successors.has(pathTo.value)}
    pathFrom.addEventListener('change',pathAvailability);pathTo.addEventListener('change',pathAvailability);
    const showAll=button('pilot-analysis-clear','Voir tout le graphe',()=>{setProjection(null,'');analysisToggle.focus()});analysisBody.append(showAll);showAll.disabled=true;
    const selectMode=button('pilot-selection-mode','Sélectionner plusieurs cartes',()=>{finish(true);selectionMode=!selectionMode;updateSelection();if(selectionMode){analysis.open=false;analysisToggle.focus()}});analysisBody.append(selectMode);
    const clearSelection=button('pilot-selection-clear','Effacer la sélection',()=>{finish(true);selectedIds.clear();updateSelection()});analysisBody.append(clearSelection);
    const selectionHelp=document.createElement('p');selectionHelp.textContent=t('Activez la sélection, puis cliquez sur les cartes ou utilisez Entrée. Les poignées déplacent le groupe ; Échap annule le geste.');analysisBody.append(selectionHelp);
    const closeAnalysis=button('pilot-analysis-close','Fermer l’analyse',()=>{analysis.open=false;analysisToggle.focus()});analysisBody.append(closeAnalysis);
    /** Keep only the chosen analyst workflow expanded, using native exclusive
     * disclosures. Inputs remain available when collapsed; no state is saved.
     * @param {string} label localized workflow heading
     * @param {Element[]} controls owned controls/labels moved from the body
     */
    function analystSection(label,controls){const section=document.createElement('details');section.className='native-graph-analysis-section';section.name='native-graph-analyst-workflow';const summary=document.createElement('summary');summary.textContent=t(label);const body=document.createElement('div');body.className='pilot-actions';body.append(...controls);section.append(summary,body);analysisBody.insertBefore(section,showAll)}
    analystSection('Lire une branche',[branch.parentElement,isolate]);
    analystSection('Comparer deux tâches',[pathFrom.parentElement,pathTo.parentElement,pathButton]);
    analystSection('Déplacer un groupe',[selectMode,clearSelection,selectionHelp]);
    analysis.addEventListener('keydown',e=>{if(e.key==='Escape'&&!gesture){e.preventDefault();e.stopPropagation();analysis.open=false;analysisToggle.focus()}});
    /** Mirror local selection with text and pressed states, independently of
     * task status colors. Does not change Work, placement or persistence.
     */
    function updateSelection(){selectMode.setAttribute('aria-pressed',String(selectionMode));selectMode.textContent=t(selectionMode?'Terminer la sélection':'Sélectionner plusieurs cartes');clearSelection.disabled=selectedIds.size===0;selectionStatus.textContent=selectedIds.size?t('Cartes sélectionnées : ')+[...selectedIds].join(', '):selectionMode?t('Sélection active : choisissez les cartes à déplacer ensemble.'):'';for(const [id,u]of units){u.group.classList.toggle('graph-multi-selected',selectedIds.has(id));const open=u.group.querySelector('.graph-task-open');if(selectionMode)open.setAttribute('aria-pressed',String(selectedIds.has(id)));else open.removeAttribute('aria-pressed')}}
    /** Apply a reversible local filter without relayout. Hidden cards/edges
     * leave keyboard navigation, search and minimap. Cancel any provisional
     * gesture and clear selection so hidden cards cannot move unexpectedly.
     * No-path requests never call this helper and retain the current view.
     */
    function setProjection(ids,label){finish(true);connection=null;visibleIds=ids;selectedIds.clear();for(const [id,u]of units){const hidden=!isVisible(id);u.group.classList.toggle('graph-analysis-hidden',hidden);u.group.setAttribute('aria-hidden',String(hidden));for(const control of u.group.querySelectorAll('[tabindex]'))control.setAttribute('tabindex',hidden?'-1':'0')}for(const edge of svg.querySelectorAll('.graph-arete')){const hidden=!isVisible(edge.dataset.from)||!isVisible(edge.dataset.to);edge.classList.toggle('graph-analysis-hidden',hidden);edge.setAttribute('aria-hidden',String(hidden));edge.setAttribute('tabindex',hidden?'-1':'0')}analysis.open=false;analysisToggle.focus({preventScroll:true});showAll.disabled=ids===null;analysisStatus.textContent=ids?label+' · '+ids.size+' / '+work.tasks.length+' · '+t('Vue filtrée ; les autres tâches sont masquées.'):'';updateSelection();findMatches();update()}

    /** Download a standalone SVG of the current recorded layout. Resolve the
     * local palette into presentation attributes, strip controls/active content
     * and external references. No credentials or HTTP request are serialized.
     * Browser download initiation is not an acknowledgement of a saved file.
     * XML 1.0 character ranges reject unrepresentable text before download.
     * The trusted SVG tree supplies element names; XMLSerializer escapes values.
     * Generation errors produce a safe visible diagnostic and preserve Work.
     * One blob URL is retained until next export or renderer cleanup.
     */
    function exportGraph(){
      try{
        const copy=svg.cloneNode(true),originals=[svg,...svg.querySelectorAll('*')],copies=[copy,...copy.querySelectorAll('*')];
        const properties=['fill','stroke','stroke-width','stroke-dasharray','opacity','font-family','font-size','font-weight','text-anchor','dominant-baseline','paint-order','vector-effect'];
        for(let i=0;i<copies.length;i++){const node=copies[i],style=getComputedStyle(originals[i]);for(const attribute of [...node.attributes])if(attribute.name==='style'||attribute.name==='tabindex'||attribute.name==='role'||attribute.name.startsWith('aria-')||attribute.name.startsWith('on')||attribute.localName==='href')node.removeAttributeNode(attribute);for(const property of properties){const value=style.getPropertyValue(property);if(value&&!/url\s*\(/i.test(value))node.setAttribute(property,value)}}
        for(const node of copy.querySelectorAll('.graph-analysis-hidden,.graph-drag,.graph-copy,.graph-port,.graph-link-ghost,script,foreignObject,image,use,a,style,animate,animateTransform,set'))node.remove();
        for(const node of copy.querySelectorAll('*'))for(const attribute of [...node.attributes])if(/url\s*\(/i.test(attribute.value)&&!/^url\(#[A-Za-z0-9_-]+\)$/.test(attribute.value))node.removeAttributeNode(attribute);
        copy.setAttribute('xmlns',SVG_NS);copy.setAttribute('width',extent.width);copy.setAttribute('height',extent.height);copy.setAttribute('role','img');copy.setAttribute('aria-label',work.title);
        const background=svgNode('rect',{x:0,y:0,width:extent.width,height:extent.height,fill:getComputedStyle(canvas).backgroundColor});copy.prepend(background);
        const content=new XMLSerializer().serializeToString(copy);for(const character of content){const cp=character.codePointAt(0);if(!(cp===9||cp===10||cp===13||cp>=0x20&&cp<=0xD7FF||cp>=0xE000&&cp<=0xFFFD||cp>=0x10000&&cp<=0x10FFFF))throw new DOMException('Invalid XML character','InvalidStateError')}if(exportUrl)URL.revokeObjectURL(exportUrl);exportUrl=URL.createObjectURL(new Blob([content],{type:'image/svg+xml;charset=utf-8'}));const anchor=document.createElement('a');anchor.href=exportUrl;anchor.download='swarm-graph.svg';document.body.append(anchor);try{anchor.click()}finally{anchor.remove()}status.textContent=t('Export SVG demandé.');
      }catch(cause){const name=/^[A-Za-z]+Error$/.test(cause?.name)?cause.name:'ExportError';status.textContent=t('Export SVG impossible.')+' ('+name+')'}
    }
    button('pilot-export','Exporter le SVG',exportGraph);
    const direction=document.createElement('select');direction.id='pilot-orientation';direction.setAttribute('aria-label',t('Orientation'));
    for(const [value,label]of [['LR','Gauche → droite'],['TB','Haut → bas']]){const option=document.createElement('option');option.value=value;option.textContent=t(label);direction.append(option)}direction.value=orientation;
    direction.addEventListener('change',()=>{host.dataset.orientation=direction.value;render(work)});toolbar.append(direction);
    const search=document.createElement('div');search.className='native-graph-search';search.setAttribute('role','search');search.setAttribute('aria-label',t('Rechercher une tâche'));
    const searchLabel=document.createElement('label');searchLabel.htmlFor='pilot-search';searchLabel.textContent=t('Rechercher une tâche');
    const searchInput=document.createElement('input');searchInput.id='pilot-search';searchInput.type='search';searchInput.autocomplete='off';searchInput.setAttribute('aria-controls','pilot-canvas');
    const searchStatus=document.createElement('output');searchStatus.id='pilot-search-status';searchStatus.setAttribute('role','status');searchStatus.setAttribute('aria-live','polite');
    let matches=[],matchIndex=-1;
    /** Refresh local matches without hiding nodes/edges or changing layout.
     * Empty queries clear highlights; no match disables navigation and reports
     * it visibly. Query/matches are not persisted and reset with this renderer.
     * @returns {void} updates classes, output and navigation control only
     */
    function findMatches(){
      const query=searchKey(searchInput.value.trim());matches=query?work.tasks.filter(task=>isVisible(task.id)&&(searchKey(task.id).includes(query)||searchKey(task.title).includes(query))):[];matchIndex=-1;
      const ids=new Set(matches.map(task=>task.id));for(const [id,u]of units)u.group.classList.toggle('graph-search-match',ids.has(id));
      nextMatch.disabled=matches.length===0;previousMatch.disabled=matches.length===0;clearSearch.disabled=!searchInput.value;
      searchStatus.textContent=!query?'':matches.length===0?t('Aucune tâche correspondante.'):matches.length===1?t('Une tâche trouvée.'):matches.length+' '+t('tâches trouvées.');
    }
    /** Center a literal match in either direction in the available scroll extent and focus
     * its existing opener without scrolling the page. Clamping at graph edges
     * is native browser behavior. No task, coordinates, zoom or policy is saved.
     * @param {number} direction internal +1 or -1, never an event object
     * @returns {void} no-op for empty matches or a detached renderer
     */
    function centerMatch(direction=1){
      if(!matches.length||disposed)return;matchIndex=matchIndex<0?(direction<0?matches.length-1:0):(matchIndex+direction+matches.length)%matches.length;const task=matches[matchIndex],target=units.get(task.id)?.group.querySelector('.graph-task-open');if(!target)return;
      const rect=target.getBoundingClientRect(),view=canvas.getBoundingClientRect();canvas.scrollLeft+=rect.left+rect.width/2-view.left-view.width/2;canvas.scrollTop+=rect.top+rect.height/2-view.top-view.height/2;
      target.focus({preventScroll:true});searchStatus.textContent=(matchIndex+1)+' / '+matches.length+' · '+task.id+' — '+task.title;
    }
    const previousMatch=button('pilot-search-previous','Précédent',()=>centerMatch(-1));
    const nextMatch=button('pilot-search-next','Suivant',()=>centerMatch(1));
    const clearSearch=button('pilot-search-clear','Effacer la recherche',()=>{searchInput.value='';findMatches();searchInput.focus()});
    searchInput.addEventListener('input',findMatches);
    searchInput.addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();e.stopPropagation();centerMatch(e.shiftKey?-1:1)}else if(e.key==='Escape'&&searchInput.value){e.preventDefault();e.stopPropagation();searchInput.value='';findMatches()}});
    search.append(searchLabel,searchInput,previousMatch,nextMatch,clearSearch,searchStatus);toolbar.append(search);findMatches();
    const copyStatus=document.createElement('p');copyStatus.id='pilot-copy-status';copyStatus.setAttribute('role','status');copyStatus.setAttribute('aria-live','polite');
    const copyFallback=document.createElement('label');copyFallback.className='native-graph-copy-fallback';copyFallback.hidden=true;copyFallback.htmlFor='pilot-copy-value';copyFallback.textContent=t('Identifiant à copier');
    const copyValue=document.createElement('input');copyValue.id='pilot-copy-value';copyValue.readOnly=true;copyFallback.append(copyValue);
    /** Copy the exact literal task ID only on explicit activation. Success is
     * announced after clipboard acknowledgement. Rejection/unavailability is
     * unconfirmed, exposes only a safe error family and selects a readonly ID.
     * @param {string} id validated task ID
     * @param {SVGElement} control activated copy button; busy prevents duplicates
     * @returns {Promise<void>} handles clipboard errors locally, never writes Work
     */
    async function copyId(id,control){
      if(copying||disposed)return;copying=true;control.setAttribute('aria-busy','true');copyStatus.dataset.copyState='pending';copyStatus.textContent=t('Copie en cours…');
      try{if(typeof navigator.clipboard?.writeText!=='function'){const error=new Error();error.name='ClipboardUnavailableError';throw error}await navigator.clipboard.writeText(id);if(!disposed){copyFallback.hidden=true;copyStatus.dataset.copyState='copied';delete copyStatus.dataset.copyError;copyStatus.textContent=t('Identifiant copié : ')+id}}
      catch(error){if(!disposed){copyStatus.dataset.copyState='unconfirmed';copyStatus.dataset.copyError=/^[A-Za-z]+Error$/.test(error?.name)?error.name:'ClipboardError';copyStatus.textContent=t('Copie non confirmée. Sélectionnez l’identifiant.');copyFallback.hidden=false;copyValue.value=id;copyValue.focus();copyValue.select()}}
      finally{copying=false;control.setAttribute('aria-busy','false')}
    }
    const help=document.createElement('dialog');help.id='pilot-graph-help';help.className='native-graph-help';help.setAttribute('aria-labelledby','pilot-graph-help-title');
    const helpToggle=button('pilot-graph-help-toggle','Aide des gestes',()=>{help.showModal();helpTitle.focus()});helpToggle.setAttribute('aria-haspopup','dialog');const helpHeader=document.createElement('header'),helpTitle=document.createElement('h2');helpTitle.id='pilot-graph-help-title';helpTitle.textContent=t('Aide des gestes');helpTitle.tabIndex=-1;helpHeader.append(helpTitle);const helpBody=document.createElement('div');helpBody.className='modal-body';help.append(helpHeader,helpBody);
    const helpList=document.createElement('ul');for(const text of [
      'Zoom : boutons ou Ctrl + molette. Vue d’ensemble inclut les cartes déplacées.',
      'Outils du graphe regroupe placement, orientation, repérage et export. Échap referme ce panneau.',
      '100 % rétablit l’échelle autour du centre. Précédent ou Maj + Entrée remonte les résultats.',
      'Rétablir le placement récupère une annulation. Un nouveau déplacement terminé remplace ce témoin.',
      'La navigation locale retrouve orientation et zone après rechargement, sans ouvrir le plein écran.',
      'Ctrl + molette garde le point sous le curseur. La vue est conservée après l’enregistrement sur le même travail.',
      'Repérer les dépendances distingue prérequis et dépendants sans masquer de carte.',
      'Analyser et sélectionner isole une branche ou tous les chemins orientés entre deux tâches. Voir tout le graphe restaure la vue.',
      'En mode sélection, cliquez ou appuyez sur Entrée sur chaque carte, puis déplacez une poignée pour déplacer le groupe. Annuler le placement restaure tout le groupe.',
      'L’export SVG télécharge le placement affiché ; les boutons ne sont pas exportés.',
      'Déplacez le fond pour naviguer. Glissez le cadre lui-même ou sa poignée pour le placer ; un clic sans déplacement ouvre la tâche. Annuler le placement restaure le dernier déplacement ou rangement.',
      'Ajouter une tâche ouvre le formulaire : choisissez un identifiant, un titre, un livrable et des critères, puis enregistrez. Le brouillon multi-tâches permet de préparer plusieurs changements avant application.',
      'Cliquez sur une flèche pour changer son prérequis ou préparer son retrait. Le formulaire confirme le changement. Suppr sur un lien prépare directement son retrait.',
      'Mini-carte : cliquez pour centrer ; utilisez les flèches, Début et Fin au clavier.',
      'Recherche : Entrée ou Suivant centre une correspondance ; Échap efface la saisie.',
      'Liens : reliez une sortie à une entrée, puis confirmez dans le formulaire. Aucun geste ne sauvegarde un lien.',
      'ID copie l’identifiant de la carte. Une copie non confirmée laisse le texte sélectionnable.',
      'Échap annule un geste en cours ou ferme le plein écran. Entrée ouvre la carte ciblée.'
    ]){const item=document.createElement('li');item.textContent=t(text);helpList.append(item)}helpBody.append(helpList);
    const helpClose=document.createElement('button');helpClose.type='button';helpClose.id='pilot-graph-help-close';helpClose.textContent=t('Fermer l’aide');
    /** Close only this disclosure and restore its summary focus. */
    function closeHelp(){help.close()}
    const helpFooter=document.createElement('footer');helpClose.addEventListener('click',closeHelp);helpFooter.append(helpClose);help.append(helpFooter);help.addEventListener('close',()=>{if(helpToggle.isConnected)helpToggle.focus({preventScroll:true})});
    /** Convert pointer coordinates to unscaled SVG units; no engine state. */
    function point(e){const p=svg.createSVGPoint();p.x=e.clientX;p.y=e.clientY;return p.matrixTransform(svg.getScreenCTM().inverse())}
    /** End or roll back a captured gesture; cancellation never persists changes. */
    function finish(cancel=false){if(!gesture)return;const g=gesture;gesture=null;if(g.card&&g.moved)g.suppressClick();if(cancel&&g.id)positions=copyPositions(g.snapshot);if(cancel&&!g.id&&!g.link)canvas.scrollTo(g.left,g.top);if(g.target.hasPointerCapture(g.pointer))g.target.releasePointerCapture(g.pointer);svg.querySelector('.graph-link-ghost')?.remove();if(!cancel&&g.id&&(!g.card||g.moved))rememberPlacement(g.snapshot);update();if(!cancel&&g.id&&(!g.card||g.moved))save();else if(!cancel)persistView()}
    /** Stage one link in the destination task form; never save from a gesture.
     * The engine checks cycles/activity/revision on explicit confirmation.
     */
    function link(from,to){connection=null;if(from===to){status.textContent=t('Choisissez deux tâches différentes.');return}const target=work.tasks.find(t=>t.id===to);if(!target)return;editTask(work,target,[...new Set([...(target.depends||[]),from])])}
    /** Keyboard/click ports provide the same proposal as a pointer connection. */
    function activatePort(id,side){if(!connection){connection={id,side};status.textContent=t('Choisissez l’autre extrémité du lien. Échap annule.');return}if(connection.side===side){connection={id,side};return}const first=connection;link(side==='in'?first.id:id,side==='in'?id:first.id);status.textContent=''}
    for(const group of svg.querySelectorAll('.graph-noeud')){
      const rect=group.querySelector('rect'),id=group.dataset.task,x=Number(rect.getAttribute('x')),y=Number(rect.getAttribute('y')),w=Number(rect.getAttribute('width')),h=Number(rect.getAttribute('height'));
      group.classList.add('graph-unit');group.dataset.node=id;units.set(id,{group,x,y,w,h});
      const open=svgNode('g',{class:'graph-task-open',role:'button',tabindex:0,'aria-label':group.getAttribute('aria-label')});open.dataset.task=id;
      while(group.firstChild)open.append(group.firstChild);group.append(open);group.setAttribute('role','group');
      /** Selection mode replaces opening only for this renderer; ordinary mode
       * retains the task editor. A toggle performs no storage or engine write. */
      let suppressCardClick=false;
      function activateCard(event){if(event?.detail&&suppressCardClick)return;if(selectionMode){finish(true);if(selectedIds.has(id))selectedIds.delete(id);else selectedIds.add(id);updateSelection()}else editTask(work,work.tasks.find(t=>t.id===id))}
      open.addEventListener('click',activateCard);
      // A four-screen-pixel threshold distinguishes a click from a drag,
      // matching the existing port gesture tolerance; never an engine policy.
      open.addEventListener('pointerdown',e=>{if(e.button!==0)return;finish(true);suppressCardClick=false;open.focus({preventScroll:true});gesture={id,ids:movementIds(id),card:true,moved:false,suppressClick:()=>{suppressCardClick=true},target:open,pointer:e.pointerId,snapshot:copyPositions(),start:point(e),screen:{x:e.clientX,y:e.clientY}};open.setPointerCapture(e.pointerId)});
      open.addEventListener('pointermove',e=>{if(gesture?.target!==open)return;const g=gesture;if(!g.moved&&Math.hypot(e.clientX-g.screen.x,e.clientY-g.screen.y)<=4)return;g.moved=true;e.preventDefault();const p=point(e);translateGroup(g.snapshot,g.ids,p.x-g.start.x,p.y-g.start.y);update()});
      open.addEventListener('pointerup',()=>{if(gesture?.target===open)finish()});open.addEventListener('pointercancel',()=>{if(gesture?.target===open)finish(true)});
      open.addEventListener('keydown',e=>{if(!['Enter',' '].includes(e.key))return;e.preventDefault();activateCard()});
      const handle=svgNode('g',{class:'graph-drag',role:'button',tabindex:0,'aria-label':t('Déplacer la carte')+' '+id});handle.dataset.task=id;handle.append(svgNode('rect',{x:x+w-34,y:y+4,width:30,height:30,rx:6}),svgNode('text',{x:x+w-30,y:y+25},'⠿'));group.append(handle);
      handle.addEventListener('keydown',e=>{if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key))return;e.preventDefault();e.stopPropagation();finish(true);const before=copyPositions();translateGroup(before,movementIds(id),e.key==='ArrowLeft'?-16:e.key==='ArrowRight'?16:0,e.key==='ArrowUp'?-16:e.key==='ArrowDown'?16:0);rememberPlacement(before);update();save()});
      handle.addEventListener('pointerdown',e=>{if(e.button!==0)return;e.preventDefault();e.stopPropagation();finish(true);handle.focus();gesture={id,ids:movementIds(id),target:handle,pointer:e.pointerId,snapshot:copyPositions(),start:point(e)};handle.setPointerCapture(e.pointerId)});
      handle.addEventListener('pointermove',e=>{if(gesture?.id!==id)return;const p=point(e);translateGroup(gesture.snapshot,gesture.ids,p.x-gesture.start.x,p.y-gesture.start.y);update()});
      handle.addEventListener('pointerup',()=>finish());handle.addEventListener('pointercancel',()=>finish(true));
      const copy=svgNode('g',{class:'graph-copy',role:'button',tabindex:0,'aria-label':t('Copier l’identifiant')+' '+id});copy.dataset.task=id;copy.append(svgNode('title',{},t('Copier l’identifiant')+' '+id),svgNode('rect',{x:x+w-82,y:y+2,width:44,height:44,rx:6}),svgNode('text',{x:x+w-68,y:y+28},'ID'));group.append(copy);copy.addEventListener('pointerdown',e=>e.stopPropagation());copy.addEventListener('click',e=>{e.stopPropagation();copyId(id,copy)});copy.addEventListener('keydown',e=>{if(['Enter',' '].includes(e.key)){e.preventDefault();e.stopPropagation();copyId(id,copy)}});
      for(const side of ['in','out']){
        const vertical=orientation==='TB',port=svgNode('circle',{class:'graph-port',role:'button',tabindex:0,r:9,cx:vertical?x+w/2:side==='in'?x:x+w,cy:vertical?(side==='in'?y:y+h):y+h/2,'aria-label':t(side==='out'?'Relier cette sortie':'Relier cette entrée')+' '+id});port.dataset.task=id;port.dataset.side=side;port.append(svgNode('title',{},t(side==='in'?'Entrée : prérequis':'Sortie : dépendants')+' — '+id));let suppressClick=false;group.append(port);
        port.addEventListener('keydown',e=>{if(!['Enter',' '].includes(e.key))return;e.preventDefault();e.stopPropagation();activatePort(id,side)});
        port.addEventListener('click',e=>{e.stopPropagation();if(suppressClick){suppressClick=false;return}activatePort(id,side)});
        port.addEventListener('pointerdown',e=>{if(e.button!==0)return;e.preventDefault();e.stopPropagation();port.focus();gesture={link:true,target:port,pointer:e.pointerId,start:point(e),moved:false};port.setPointerCapture(e.pointerId)});
        port.addEventListener('pointermove',e=>{if(!gesture?.link||gesture.target!==port)return;const p=point(e);gesture.moved=Math.hypot(p.x-gesture.start.x,p.y-gesture.start.y)>4;svg.querySelector('.graph-link-ghost')?.remove();if(gesture.moved)svg.append(svgNode('path',{class:'graph-link-ghost',d:`M${gesture.start.x},${gesture.start.y} L${p.x},${p.y}`}))});
        port.addEventListener('pointerup',e=>{e.stopPropagation();if(!gesture?.link)return;const moved=gesture.moved,other=document.elementFromPoint(e.clientX,e.clientY)?.closest('.graph-port,.graph-noeud');finish();if(moved){suppressClick=true;const target=other?.dataset.task;if(target)link(side==='out'?id:target,side==='out'?target:id)}});port.addEventListener('pointercancel',()=>{suppressClick=true;finish(true)});
      }
    }
    /** Center a graph-space point selected through the minimap. Native scroll
     * clamping applies; only viewport/focus changes, never geometry or Work.
     */
    function centerMap(x,y){canvas.scrollLeft=x*zoom-canvas.clientWidth/2;canvas.scrollTop=y*zoom-canvas.clientHeight/2;updateViewport()}
    map.addEventListener('pointerdown',e=>e.stopPropagation());map.addEventListener('click',e=>{e.preventDefault();e.stopPropagation();const p=map.createSVGPoint();p.x=e.clientX;p.y=e.clientY;const at=p.matrixTransform(map.getScreenCTM().inverse());centerMap(at.x,at.y);map.focus({preventScroll:true})});
    map.addEventListener('keydown',e=>{if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End'].includes(e.key))return;e.preventDefault();e.stopPropagation();if(e.key==='Home')canvas.scrollTo(0,0);else if(e.key==='End')canvas.scrollTo(canvas.scrollWidth,canvas.scrollHeight);else{canvas.scrollLeft+=(e.key==='ArrowLeft'?-1:e.key==='ArrowRight'?1:0)*canvas.clientWidth/2;canvas.scrollTop+=(e.key==='ArrowUp'?-1:e.key==='ArrowDown'?1:0)*canvas.clientHeight/2}updateViewport()});canvas.addEventListener('scroll',()=>{updateViewport();persistView()});studio.addEventListener('scroll',()=>persistView());
    canvas.addEventListener('pointerdown',e=>{if(e.button!==0||e.target.closest('.graph-noeud,.graph-arete'))return;e.preventDefault();canvas.focus();gesture={target:canvas,pointer:e.pointerId,start:{x:e.clientX,y:e.clientY},left:canvas.scrollLeft,top:canvas.scrollTop};canvas.setPointerCapture(e.pointerId)});
    canvas.addEventListener('pointermove',e=>{if(!gesture||gesture.id||gesture.link)return;canvas.scrollLeft=gesture.left+gesture.start.x-e.clientX;canvas.scrollTop=gesture.top+gesture.start.y-e.clientY});canvas.addEventListener('pointerup',()=>finish());canvas.addEventListener('pointercancel',()=>finish(true));
    /** Keep the graph point under the cursor stable on Ctrl+wheel. The native
     * scroll clamp applies at edges and the existing zoom limits are unchanged.
     * Ordinary wheel events retain browser scrolling; no engine operation.
     */
    function zoomAtCursor(e){if(!e.ctrlKey)return;e.preventDefault();const at=point(e),view=canvas.getBoundingClientRect(),x=e.clientX-view.left-canvas.clientLeft,y=e.clientY-view.top-canvas.clientTop;zoom=Math.max(.15,Math.min(2,zoom+(e.deltaY<0?.15:-.15)));update();canvas.scrollLeft=at.x*zoom-x;canvas.scrollTop=at.y*zoom-y;updateViewport();save()}
    canvas.addEventListener('wheel',zoomAtCursor,{passive:false});
    for(const edge of svg.querySelectorAll('.graph-arete')){edge.setAttribute('role','button');edge.setAttribute('tabindex','0');edge.setAttribute('aria-label',t('Modifier un lien')+' '+edge.dataset.from+' → '+edge.dataset.to);const remove=()=>{const target=work.tasks.find(t=>t.id===edge.dataset.to);editTask(work,target,(target.depends||[]).filter(id=>id!==edge.dataset.from))};edge.addEventListener('click',()=>inspectLink(work,edge.dataset.from,edge.dataset.to));edge.addEventListener('keydown',e=>{if(['Enter',' ','Delete','Backspace'].includes(e.key)){e.preventDefault();if(['Delete','Backspace'].includes(e.key))remove();else inspectLink(work,edge.dataset.from,edge.dataset.to)}})}
    studio.addEventListener('keydown',e=>{if(e.key!=='Escape')return;if(gesture||connection){e.preventDefault();e.stopPropagation();finish(true);connection=null;status.textContent=''}else if(selectionMode){e.preventDefault();e.stopPropagation();selectionMode=false;selectedIds.clear();updateSelection()}else if(modal.open){e.preventDefault();modal.close()}});
    modal.addEventListener('close',()=>{if(disposed)return;finish(true);host.insertBefore(studio,modal);studio.classList.remove('is-fullscreen');document.body.classList.remove('graph-studio-open');actionLabel(full,'Plein écran');full.setAttribute('aria-expanded','false');full.focus()});
    const viewportObserver=new ResizeObserver(()=>{if(disposed)return;host.graphRestoreView?.();updateViewport();persistView()});viewportObserver.observe(canvas);
    for(const id of ['pilot-layout-reset','pilot-layout-undo','pilot-layout-redo','pilot-view-reset','pilot-neighbors','pilot-neighbors-clear','pilot-export','pilot-orientation']){const control=toolbar.querySelector('#'+id);if(control)toolsBody.append(control===neighborSelect?neighbors:control)}
    const closeTools=document.createElement('button');closeTools.type='button';closeTools.id='pilot-tools-close';closeTools.textContent=t('Fermer les outils');toolsBody.append(closeTools);
    /** Collapse secondary actions with summary focus, without closing the graph. */
    function collapseTools(){tools.open=false;toolsToggle.focus()}
    closeTools.addEventListener('click',collapseTools);tools.addEventListener('keydown',e=>{if(e.key==='Escape'&&tools.open){e.preventDefault();e.stopPropagation();collapseTools()}});tools.addEventListener('toggle',()=>persistView());
    /** Reapply the responsive default only when crossing the existing layout
     * breakpoint; retain focus if the focused secondary control becomes hidden.
     */
    function resizeTools(e){if(!e.matches&&tools.contains(document.activeElement))toolsToggle.focus();tools.open=e.matches;persistView()}
    media.addEventListener('change',resizeTools);
    decorate(toolsToggle,'tools');decorate(analysisToggle,'tools');
    const legend=document.createElement('div');legend.id='pilot-graph-colors';legend.className='native-graph-color-key';legend.setAttribute('aria-label',t('Couleurs des états'));for(const [tone,label]of [['attente','à faire'],['active','en cours'],['attention','à vérifier'],['alerte','bloquée'],['validee','acceptée']]){const item=document.createElement('span');item.className='graph-teinte';item.dataset.etat=tone;item.textContent=t(label);legend.append(item)}
    const gestureHint=document.createElement('p');gestureHint.className='native-graph-gesture-hint';gestureHint.textContent=t('Glissez un cadre pour le déplacer. Cliquez pour ouvrir la tâche ; cliquez sur une flèche pour modifier le lien.');
    studio.append(toolbar,analysis,tools,status,analysisStatus,selectionStatus,preferenceStatus,neighborStatus,copyStatus,copyFallback,gestureHint,legend,canvas);host.replaceChildren(studio,modal,help);
    /** Capture completed same-Work presentation, including deferred modal/scroll.
     * In-progress card movement and canvas panning are excluded, as on cancel.
     * Memory only; callers must compare Work IDs before restoring this object.
     */
    function captureView(){return{work:work.id,zoom,positions:committedPositions(),...viewRecord(),fullscreen:modal.open||resumeFullscreen,preferenceError:preferenceStatus.textContent,preferenceErrorCode:preferenceStatus.dataset.viewError||''}}
    host.graphView=captureView;
    /** Restore after rerender, or after an editor closes. Never open a graph
     * modal above a task editor; scroll is retained until its geometry is ready.
     */
    host.graphRestoreView=()=>{if(disposed||resumeFullscreen&&document.querySelector('.native-graph-editor[open]'))return;if(!resumeFullscreen&&(!canvas.clientWidth||!canvas.clientHeight))return;if(resumeFullscreen){resumeFullscreen=false;if(!modal.open)full.click()}if(pendingScroll){canvas.scrollTo(pendingScroll.left,pendingScroll.top);pendingScroll=null}if(pendingStudio){studio.scrollTo(pendingStudio.left,pendingStudio.top);pendingStudio=null}updateViewport()};
    host.graphCleanup=()=>{if(help.open)help.close();finish(true);persistView();disposed=true;viewportObserver.disconnect();media.removeEventListener('change',resizeTools);if(exportUrl)URL.revokeObjectURL(exportUrl);if(modal.open)modal.close();document.body.classList.remove('graph-studio-open')};updateSelection();update();host.graphRestoreView();lastViewAttempt=JSON.stringify(viewRecord());if(initial&&initial.orientation!==orientation)persistView(true);
  }

  /**
   * Render the dependency graph of a Work into the original #graph container.
   * Recorded projection with local navigation and explicit task-editor handlers.
   * Rendering performs no engine writes; same-Work presentation survives rerender.
   * Saving an editor uses protected engine operations; another Work restores only
   * its own validated preference and never inherits full screen or undo history.
   * Side effect: replaces the container's children (scrollable canvas + svg).
   * @param {object} work Work {id,title,tasks:[{id,title,status,depends?:string[]}]}
   * @throws {Error} on invalid Work, duplicate ids, unknown-shaped dependencies or cycles
   */
  function render(work) {
    const host = container();
    const { tasks, depPairs, unknown } = validateWork(work);
    const previous=host.graphView?.(),initial=previous?.work===work.id?previous:null;
    const recovered=initial?{view:null,error:''}:readStoredView(work.id);
    host.graphCleanup?.();host.graphCleanup=null;host.graphView=null;host.graphRestoreView=null;
    const orientation=initial?(host.dataset.orientation==='TB'?'TB':'LR'):(recovered.view?.orientation||'LR');host.dataset.orientation=orientation;
    if (typeof graphNodeKey !== 'function' || typeof svgNode !== 'function' || typeof graphEdgePath !== 'function' || typeof graphTonalites !== 'object' || !globalThis.dagre || !globalThis.dagre.graphlib || !globalThis.dagre.layout) {
      throw new Error(t('Graphe : moteur dagre indisponible.'));
    }
    const canvas = document.createElement('div');
    canvas.className = 'pilot-canvas';
    canvas.tabIndex=0;canvas.setAttribute('aria-label',t('Dépendances : du prérequis vers la tâche'));

    const labels = {
      todo: t('à faire'), running: t('en cours'), blocked: t('bloquée'),
      submitted: t('soumise'), accepted: t('acceptée'), waived: t('levée'),
      abandoned: t('abandonnée'), stale: t('à vérifier')
    };
    const tonalites = graphTonalites;

    const nodeWidth = 260, nodeHeight = 72;
    const g = new globalThis.dagre.graphlib.Graph();
    g.setGraph({ rankdir: orientation, ranker: 'tight-tree', nodesep: 28, ranksep: 70, marginx: 20, marginy: 20 });
    g.setDefaultEdgeLabel(() => ({}));
    for (const task of tasks) g.setNode(graphNodeKey(task.id), { width: nodeWidth, height: nodeHeight });
    for (const e of depPairs) g.setEdge(graphNodeKey(e.from), graphNodeKey(e.to));
    if (!depPairs.length) {
      tasks.forEach((task, i) => g.setNode(graphNodeKey(task.id), {
        width: nodeWidth, height: nodeHeight,
        x: 20 + nodeWidth / 2 + (orientation==='LR'?i*(nodeWidth+28):0),
        y: 20 + nodeHeight / 2 + (orientation==='TB'?i*(nodeHeight+28):0)
      }));
    } else {
      globalThis.dagre.layout(g);
    }
    const totalWidth = depPairs.length ? g.graph().width : orientation==='LR'?Math.max(nodeWidth+40,tasks.length*(nodeWidth+28)+12):nodeWidth+40;
    const totalHeight = depPairs.length ? g.graph().height : orientation==='TB'?Math.max(nodeHeight+40,tasks.length*(nodeHeight+28)+12):nodeHeight+40;

    const svg = svgNode('svg', {});
    svg.setAttribute('width',String(totalWidth));svg.setAttribute('height',String(totalHeight));
    const defs=svgNode('defs',{}),marker=svgNode('marker',{id:'recorded-dependency-arrow',viewBox:'0 0 8 8',refX:7,refY:4,markerWidth:7,markerHeight:7,orient:'auto'});
    marker.append(svgNode('path',{d:'M0,0 L8,4 L0,8 z',class:'graph-fleche'}));defs.append(marker);svg.append(defs);
    svg.setAttribute('viewBox', '0 0 ' + Math.max(1, totalWidth) + ' ' + Math.max(1, totalHeight));
    svg.setAttribute('class', 'graph-svg');
    svg.setAttribute('role', 'group');
    svg.setAttribute('aria-label', t('Dépendances : du prérequis vers la tâche'));

    for (const e of depPairs) {
      const data = g.edge(graphNodeKey(e.from), graphNodeKey(e.to));
      const d = graphEdgePath(data.points);
      const edge = document.createElementNS(SVG_NS, 'path');
      edge.setAttribute('d', d);
      edge.setAttribute('class', 'graph-arete');
      edge.setAttribute('marker-end','url(#recorded-dependency-arrow)');
      edge.dataset.from = e.from;
      edge.dataset.to = e.to;
      edge.append(svgNodeTitle(t('Dépendance : ') + e.from + t(' vers ') + e.to));
      svg.append(edge);
    }

    for (const task of tasks) {
      const pos = g.node(graphNodeKey(task.id));
      const left = pos.x - nodeWidth / 2, top = pos.y - nodeHeight / 2;
      const stateLabel = labels[task.status] || task.status;
      const group = document.createElementNS(SVG_NS, 'g');
      group.setAttribute('class', 'graph-noeud');
      group.setAttribute('role', 'img');
      group.setAttribute('aria-label', t('Tâche : ') + task.title + t(' — identifiant : ') + task.id + t(' — état : ') + stateLabel);
      group.dataset.task = task.id;
      group.dataset.etat = tonalites[task.status] || 'neutre';
      group.append(document.createElementNS(SVG_NS, 'title'));
      group.lastChild.textContent = task.title + ' (' + task.id + ') — ' + stateLabel;
      const rect = document.createElementNS(SVG_NS, 'rect');
      rect.setAttribute('x', String(left));
      rect.setAttribute('y', String(top));
      rect.setAttribute('width', String(nodeWidth));
      rect.setAttribute('height', String(nodeHeight));
      rect.setAttribute('rx', '12');
      rect.setAttribute('class', 'graph-cadre');
      group.append(rect);
      const title = document.createElementNS(SVG_NS, 'text');
      title.setAttribute('x', String(left + 14));
      title.setAttribute('y', String(top + 28));
      title.setAttribute('class', 'graph-titre');
      appendTruncated(title, task.title);
      group.append(title);
      const sub = document.createElementNS(SVG_NS, 'text');
      sub.setAttribute('x', String(left + 14));
      sub.setAttribute('y', String(top + 52));
      sub.setAttribute('class', 'graph-sous-titre');
      appendTruncated(sub, task.id + ' · ' + stateLabel);
      group.append(sub);
      svg.append(group);
    }

    canvas.append(svg);
    if(!tasks.length){const empty=document.createElement('p');empty.textContent=t('Aucune tâche : le graphe apparaîtra dès qu’un plan existe.');canvas.append(empty)}
    if (unknown.length) {
      const warning = document.createElement('p');
      warning.setAttribute('role', 'alert');
      warning.textContent = t('Dépendances inconnues (non tracées) : ') +
        unknown.map(u => u.dep + ' → ' + u.taskId).join(', ');
      canvas.append(warning);
    }
    navigation(host,canvas,svg,work,orientation,initial||recovered.view,recovered.error,recovered.errorCode);
  }

  /** Display at most 24 UTF-16 code units including the ellipsis to leave room
   * for copy/movement controls; retain full literal text in a title when clipped.
   * @param {SVGElement} text target text element, replaced without HTML parsing
   * @param {string} value validated title or task/status label
   * @returns {void} mutates only SVG text and optional tooltip
   */
  function appendTruncated(text, value) {
    const shown = value.length > 24 ? value.slice(0, 23) + '…' : value;
    text.textContent = shown;
    if (shown !== value) {
      const full = document.createElementNS(SVG_NS, 'title');
      full.textContent = value;
      text.append(full);
    }
  }

  /** @returns {SVGElement} a <title> element with textContent */
  function svgNodeTitle(value) {
    const el = document.createElementNS(SVG_NS, 'title');
    el.textContent = value;
    return el;
  }

  /**
   * Clear the original #graph container and replace its content with a message.
   * Close its owned modal, disconnect observers and discard in-memory view state.
   * Cleanup failures propagate rather than claiming an empty successful graph.
   * Side effect: replaces the container's children.
   * @param {string} message literal message to display
   * @throws {Error} when the container is absent
   */
  function clear(message) {
    const host = container();
    host.graphCleanup?.();host.graphCleanup=null;host.graphView=null;host.graphRestoreView=null;
    const p = document.createElement('p');
    p.textContent = typeof message === 'string' ? message : '';
    host.replaceChildren(p);
  }

  globalThis.SwarmRustDependencyGraph = { render, clear, setRefreshHandler };
})();
