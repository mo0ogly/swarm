'use strict';
(function () {
  const t = (source) => globalThis.SwarmI18n?.t ? globalThis.SwarmI18n.t(source) : source;
  const SVG_NS = 'http://www.w3.org/2000/svg';
  let refreshWork = null;

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

  /** Open a task editor on one observed Work revision. Inputs remain visible on
   * refusal. Explicit submit uses scoped CSRF and the engine's task contracts;
   * no provider or automatic retry. A lost/malformed response is uncertain and
   * disables resubmission until the operator closes and refreshes. Success needs
   * a protected Work readback; the cockpit then reloads its actual observations.
   * Visible/live saving feedback and the button busy state last through that
   * entire sequence, then clear on refusal or uncertainty without enabling retry.
   * @param {object} work recorded Work, never a fabricated snapshot
   * @param {object|null} task null creates, otherwise edits the recorded task
   * @param {string[]|undefined} proposedDependencies optional unsaved link gesture
   */
  function editTask(work, task=null, proposedDependencies) {
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
    field('title','Titre',task?.title||'');field('deliverable','Livrable',task?.deliverable||'');
    field('criteria','Critères d’acceptation (un par ligne)',(task?.criteria||[]).join('\n'),true);
    const label=document.createElement('label');label.textContent=t('Prérequis');label.htmlFor='native-graph-depends';
    const dependencies=document.createElement('select');dependencies.id='native-graph-depends';dependencies.multiple=true;
    for(const other of work.tasks||[]){if(other.id===task?.id)continue;const option=document.createElement('option');option.value=other.id;option.textContent=other.id+' — '+other.title;option.selected=!!(proposedDependencies??task?.depends)?.includes(other.id);dependencies.append(option)}label.append(dependencies);body.append(label);
    const notice=document.createElement('p');notice.className='notice info';notice.textContent=t('Choisissez les prérequis. Enregistrer modifie le plan sans lancer d’agent.');body.append(notice);
    const progress=document.createElement('p');progress.id='native-graph-progress';progress.setAttribute('role','status');progress.setAttribute('aria-live','polite');body.append(progress);
    const error=document.createElement('p');error.className='notice alert';error.setAttribute('role','alert');error.hidden=true;body.append(error);
    const footer=document.createElement('footer'),save=document.createElement('button');save.type='submit';save.className='primary';save.id='native-graph-save';save.textContent=t('Enregistrer');footer.append(save);form.append(header,body,footer);dialog.append(form);document.body.append(dialog);
    let pending=false;
    dialog.addEventListener('cancel',e=>{if(pending)e.preventDefault()});
    dialog.addEventListener('close',()=>{dialog.remove();if(completionFocus?.isConnected)completionFocus.focus();else if(returnFocus?.isConnected)returnFocus.focus()});
    form.addEventListener('submit',async e=>{
      e.preventDefault();if(pending)return;pending=true;save.disabled=true;save.setAttribute('aria-busy','true');save.textContent=t('Enregistrement en cours…');progress.textContent=t('Enregistrement en cours…');close.disabled=true;error.hidden=true;let submitted=false;
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
        completionFocus=[...container().querySelectorAll('.graph-task-open')].find(n=>n.dataset.task===taskID);
        dialog.close();
      }catch(cause){error.hidden=false;error.textContent=t('Modification refusée ou résultat non confirmé. Fermez et actualisez avant de reprendre.')+' ('+cause.message+')';save.disabled=submitted&&!cause.refused}
      finally{pending=false;save.setAttribute('aria-busy','false');save.textContent=t('Enregistrer');progress.textContent='';close.disabled=false;for(const input of form.querySelectorAll('input,textarea,select'))input.disabled=false;if(!submitted)save.disabled=false}
    });
    dialog.showModal();fields[task?'title':'id'].focus();
  }

  /** Mount local presentation controls around a validated recorded graph.
   * Coordinates/zoom are device preferences, scoped to origin, Work and direction;
   * they grant no mutation or execution rights. Storage failure remains visible.
   * Native modal semantics own focus/inertness/Escape in fullscreen.
   * @param {Element} host existing graph container, replaced after mounting
   * @param {HTMLElement} canvas scroll viewport containing the recorded SVG
   * @param {SVGElement} svg validated Dagre layout, dimensions in SVG units
   * @param {object} work recorded Work with validated tasks/dependencies
   * @param {'LR'|'TB'} orientation displayed layout direction
   * @returns {void} installs listeners and cleanup; storage refusals stay visible
   */
  function navigation(host, canvas, svg, work, orientation) {
    const studio=document.createElement('section');studio.id='graph-studio';studio.className='graph-studio native-graph-studio';
    const toolbar=document.createElement('div');toolbar.className='pilot-actions';
    const status=document.createElement('p');status.className='graph-studio-hint';status.setAttribute('role','status');
    const key='swarm-native-graph:v1:'+work.id+':'+orientation;
    let positions=Object.create(null),previousPositions=null,zoom=1,gesture=null,connection=null,disposed=false,copying=false;
    try{const saved=JSON.parse(localStorage.getItem(key)||'null');if(saved){if(Number.isFinite(saved.zoom))zoom=Math.max(.15,Math.min(2,saved.zoom));for(const [id,p]of Object.entries(saved.positions||{}))if(p&&Number.isFinite(p.x)&&Number.isFinite(p.y))positions[id]={x:p.x,y:p.y}}}catch{status.textContent=t('Positions indisponibles ; placement automatique conservé.')}
    const base={width:Number(svg.getAttribute('width')),height:Number(svg.getAttribute('height'))},units=new Map();
    const extent={width:base.width,height:base.height};
    const map=svgNode('svg',{class:'graph-minimap',role:'group',tabindex:0,'aria-label':t('Naviguer avec la mini-carte')});canvas.append(map);
    const viewport=svgNode('rect',{class:'graph-minimap-viewport','aria-hidden':'true'});
    canvas.id='pilot-canvas';canvas.classList.add('native-graph-canvas');
    const modal=document.createElement('dialog');modal.className='native-graph-dialog';modal.setAttribute('aria-label',t('Graphe du travail'));
    /** Persist presentation only; report refusal without claiming persistence. */
    function save(){try{localStorage.setItem(key,JSON.stringify({zoom,positions}))}catch{status.textContent=t('Positions non mémorisées sur cet appareil.')}}
    /** Copy only owned coordinate entries, including prototype-shaped IDs.
     * @param {object} source local positions, never Work data
     * @returns {object} independent null-prototype coordinate snapshot
     */
    function copyPositions(source=positions){const result=Object.create(null);for(const [id,p]of Object.entries(source))result[id]={x:p.x,y:p.y};return result}
    /** Retain one previous placement only after a real completed change.
     * Cancellation/no-op preserve the previous undo; absent and zero offsets
     * describe the same placement. Memory resets on rerender;
     * neither history nor a task operation is persisted by this helper.
     */
    function rememberPlacement(before){const ids=new Set([...Object.keys(before),...Object.keys(positions)]);if([...ids].some(id=>(before[id]?.x??0)!==(positions[id]?.x??0)||(before[id]?.y??0)!==(positions[id]?.y??0)))previousPositions=before}
    /** Project native scroll bounds into unscaled SVG coordinates, clamped to
     * the graph extent. Only the minimap viewport marker changes; no IO/layout.
     */
    function updateViewport(){const x=Math.min(extent.width,canvas.scrollLeft/zoom),y=Math.min(extent.height,canvas.scrollTop/zoom);for(const [key,value]of Object.entries({x,y,width:Math.max(0,Math.min(canvas.clientWidth/zoom,extent.width-x)),height:Math.max(0,Math.min(canvas.clientHeight/zoom,extent.height-y))}))viewport.setAttribute(key,value)}
    /** Recompute curves from moved endpoints and expand the scrollable extent. */
    function update(){
      let width=base.width,height=base.height;
      for(const [id,u]of units){const p=positions[id]||{x:0,y:0};u.group.setAttribute('transform',`translate(${p.x} ${p.y})`);width=Math.max(width,u.x+p.x+u.w+30);height=Math.max(height,u.y+p.y+u.h+30)}
      for(const edge of svg.querySelectorAll('.graph-arete')){const a=units.get(edge.dataset.from),b=units.get(edge.dataset.to);if(!a||!b)continue;const ap=positions[edge.dataset.from]||{x:0,y:0},bp=positions[edge.dataset.to]||{x:0,y:0},vertical=orientation==='TB';
        const x1=a.x+ap.x+(vertical?a.w/2:a.w),y1=a.y+ap.y+(vertical?a.h:a.h/2),x2=b.x+bp.x+(vertical?b.w/2:0),y2=b.y+bp.y+(vertical?0:b.h/2),d=Math.max(60,Math.abs(vertical?y2-y1:x2-x1)/2);
        edge.setAttribute('d',vertical?`M${x1},${y1} C${x1},${y1+d} ${x2},${y2-d} ${x2},${y2}`:`M${x1},${y1} C${x1+d},${y1} ${x2-d},${y2} ${x2},${y2}`);
      }
      extent.width=width;extent.height=height;svg.setAttribute('viewBox',`0 0 ${width} ${height}`);svg.setAttribute('width',width*zoom);svg.setAttribute('height',height*zoom);
      map.setAttribute('viewBox',`0 0 ${width} ${height}`);map.replaceChildren();
      for(const edge of svg.querySelectorAll('.graph-arete'))map.append(svgNode('path',{d:edge.getAttribute('d'),class:'graph-minimap-edge'}));
      for(const [id,u]of units){const p=positions[id]||{x:0,y:0};map.append(svgNode('rect',{x:u.x+p.x,y:u.y+p.y,width:u.w,height:u.h,rx:8,class:'graph-minimap-node'}))}
      map.append(viewport);updateViewport();undo.disabled=previousPositions===null;
      zoomText.textContent=Math.round(zoom*100)+' %';
    }
    /** Create a native action with textContent and scoped listener. */
    function button(id,label,action){const b=document.createElement('button');b.type='button';b.id=id;b.textContent=t(label);b.addEventListener('click',action);toolbar.append(b);return b}
    const zoomText=document.createElement('output');zoomText.setAttribute('aria-label',t('Zoom'));
    button('pilot-zoom-out','Réduire le zoom',()=>{zoom=Math.max(.15,zoom-.15);update();save()});
    toolbar.append(zoomText);
    button('pilot-zoom-in','Agrandir le zoom',()=>{zoom=Math.min(2,zoom+.15);update();save()});
    button('pilot-fit','Vue d’ensemble',()=>{update();const required=Math.min(1,(canvas.clientWidth-24)/extent.width,(canvas.clientHeight-24)/extent.height);zoom=Math.max(.15,required);update();canvas.scrollTo(0,0);if(required<.15)status.textContent=t('Zoom minimum atteint. Rapprochez les cartes pour tout voir.');save()});
    const full=button('pilot-fullscreen','Plein écran',()=>{if(modal.open){modal.close();return}modal.showModal();modal.append(studio);studio.classList.add('is-fullscreen');document.body.classList.add('graph-studio-open');full.textContent=t('Quitter le plein écran');full.setAttribute('aria-expanded','true');full.focus()});full.setAttribute('aria-expanded','false');
    button('pilot-layout-reset','Ranger les cartes',()=>{const before=copyPositions();positions=Object.create(null);rememberPlacement(before);update();save()});
    const undo=button('pilot-layout-undo','Annuler le placement',()=>{if(!previousPositions)return;finish(true);positions=previousPositions;previousPositions=null;update();save()});undo.disabled=true;
    button('pilot-add-task','Ajouter une tâche',()=>editTask(work));
    const direction=document.createElement('select');direction.id='pilot-orientation';direction.setAttribute('aria-label',t('Orientation'));
    for(const [value,label]of [['LR','Gauche → droite'],['TB','Haut → bas']]){const option=document.createElement('option');option.value=value;option.textContent=t(label);direction.append(option)}direction.value=orientation;
    direction.addEventListener('change',()=>{host.dataset.orientation=direction.value;render(work)});toolbar.append(direction);
    const search=document.createElement('div');search.className='native-graph-search';search.setAttribute('role','search');search.setAttribute('aria-label',t('Rechercher une tâche'));
    const searchLabel=document.createElement('label');searchLabel.htmlFor='pilot-search';searchLabel.textContent=t('Rechercher une tâche');
    const searchInput=document.createElement('input');searchInput.id='pilot-search';searchInput.type='search';searchInput.autocomplete='off';searchInput.setAttribute('aria-controls','pilot-canvas');
    const searchStatus=document.createElement('output');searchStatus.id='pilot-search-status';searchStatus.setAttribute('role','status');searchStatus.setAttribute('aria-live','polite');
    let matches=[],matchIndex=-1;
    /** Normalize literal search text for case/accent-insensitive substring
     * matching. No regular expression is built from user input; no IO/storage.
     * @param {string} value validated task ID/title or the local input value
     * @returns {string} Unicode-decomposed, mark-free lowercase search key
     */
    function searchKey(value){return value.normalize('NFD').replace(/\p{M}/gu,'').toLowerCase()}
    /** Refresh local matches without hiding nodes/edges or changing layout.
     * Empty queries clear highlights; no match disables navigation and reports
     * it visibly. Query/matches are not persisted and reset with this renderer.
     * @returns {void} updates classes, output and navigation control only
     */
    function findMatches(){
      const query=searchKey(searchInput.value.trim());matches=query?work.tasks.filter(task=>searchKey(task.id).includes(query)||searchKey(task.title).includes(query)):[];matchIndex=-1;
      const ids=new Set(matches.map(task=>task.id));for(const [id,u]of units)u.group.classList.toggle('graph-search-match',ids.has(id));
      nextMatch.disabled=matches.length===0;clearSearch.disabled=!searchInput.value;
      searchStatus.textContent=!query?'':matches.length===0?t('Aucune tâche correspondante.'):matches.length===1?t('Une tâche trouvée.'):matches.length+' '+t('tâches trouvées.');
    }
    /** Center the next literal match in the available scroll extent and focus
     * its existing opener without scrolling the page. Clamping at graph edges
     * is native browser behavior. No task, coordinates, zoom or policy is saved.
     * @returns {void} no-op for empty matches or a detached renderer
     */
    function centerMatch(){
      if(!matches.length||disposed)return;matchIndex=(matchIndex+1)%matches.length;const task=matches[matchIndex],target=units.get(task.id)?.group.querySelector('.graph-task-open');if(!target)return;
      const rect=target.getBoundingClientRect(),view=canvas.getBoundingClientRect();canvas.scrollLeft+=rect.left+rect.width/2-view.left-view.width/2;canvas.scrollTop+=rect.top+rect.height/2-view.top-view.height/2;
      target.focus({preventScroll:true});searchStatus.textContent=(matchIndex+1)+' / '+matches.length+' · '+task.id+' — '+task.title;
    }
    const nextMatch=button('pilot-search-next','Suivant',centerMatch);
    const clearSearch=button('pilot-search-clear','Effacer la recherche',()=>{searchInput.value='';findMatches();searchInput.focus()});
    searchInput.addEventListener('input',findMatches);
    searchInput.addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();e.stopPropagation();centerMatch()}else if(e.key==='Escape'&&searchInput.value){e.preventDefault();e.stopPropagation();searchInput.value='';findMatches()}});
    search.append(searchLabel,searchInput,nextMatch,clearSearch,searchStatus);toolbar.append(search);findMatches();
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
    const help=document.createElement('details');help.id='pilot-graph-help';help.className='native-graph-help';
    const helpToggle=document.createElement('summary');helpToggle.id='pilot-graph-help-toggle';helpToggle.textContent=t('Aide des gestes');help.append(helpToggle);
    const helpList=document.createElement('ul');for(const text of [
      'Zoom : boutons ou Ctrl + molette. Vue d’ensemble inclut les cartes déplacées.',
      'Déplacez le fond pour naviguer et les poignées pour placer les cartes. Annuler le placement restaure le dernier déplacement ou rangement.',
      'Mini-carte : cliquez pour centrer ; utilisez les flèches, Début et Fin au clavier.',
      'Recherche : Entrée ou Suivant centre une correspondance ; Échap efface la saisie.',
      'Liens : reliez une sortie à une entrée, puis confirmez dans le formulaire. Aucun geste ne sauvegarde un lien.',
      'ID copie l’identifiant de la carte. Une copie non confirmée laisse le texte sélectionnable.',
      'Échap annule un geste en cours ou ferme le plein écran. Entrée ouvre la carte ciblée.'
    ]){const item=document.createElement('li');item.textContent=t(text);helpList.append(item)}help.append(helpList);
    const helpClose=document.createElement('button');helpClose.type='button';helpClose.id='pilot-graph-help-close';helpClose.textContent=t('Fermer l’aide');
    /** Close only this disclosure and restore its summary focus. */
    function closeHelp(){help.open=false;helpToggle.focus()}
    helpClose.addEventListener('click',closeHelp);help.append(helpClose);help.addEventListener('keydown',e=>{if(e.key==='Escape'&&help.open){e.preventDefault();e.stopPropagation();closeHelp()}});
    /** Convert pointer coordinates to unscaled SVG units; no engine state. */
    function point(e){const p=svg.createSVGPoint();p.x=e.clientX;p.y=e.clientY;return p.matrixTransform(svg.getScreenCTM().inverse())}
    /** End or roll back a captured gesture; cancellation never persists changes. */
    function finish(cancel=false){if(!gesture)return;const g=gesture;gesture=null;if(cancel&&g.id){if(g.hadPosition)positions[g.id]=g.origin;else delete positions[g.id]}if(cancel&&!g.id&&!g.link)canvas.scrollTo(g.left,g.top);if(g.target.hasPointerCapture(g.pointer))g.target.releasePointerCapture(g.pointer);svg.querySelector('.graph-link-ghost')?.remove();if(!cancel&&g.id)rememberPlacement(g.snapshot);update();if(!cancel&&g.id)save()}
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
      open.addEventListener('click',()=>editTask(work,work.tasks.find(t=>t.id===id)));
      open.addEventListener('keydown',e=>{if(!['Enter',' '].includes(e.key))return;e.preventDefault();editTask(work,work.tasks.find(t=>t.id===id))});
      const handle=svgNode('g',{class:'graph-drag',role:'button',tabindex:0,'aria-label':t('Déplacer la carte')+' '+id});handle.dataset.task=id;handle.append(svgNode('rect',{x:x+w-34,y:y+4,width:30,height:30,rx:6}),svgNode('text',{x:x+w-30,y:y+25},'⠿'));group.append(handle);
      handle.addEventListener('keydown',e=>{if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key))return;e.preventDefault();e.stopPropagation();const before=copyPositions(),p=positions[id]||{x:0,y:0};positions[id]={x:Math.max(-x,p.x+(e.key==='ArrowLeft'?-16:e.key==='ArrowRight'?16:0)),y:Math.max(-y,p.y+(e.key==='ArrowUp'?-16:e.key==='ArrowDown'?16:0))};rememberPlacement(before);update();save()});
      handle.addEventListener('pointerdown',e=>{if(e.button!==0)return;e.preventDefault();e.stopPropagation();handle.focus();gesture={id,target:handle,pointer:e.pointerId,snapshot:copyPositions(),hadPosition:Object.hasOwn(positions,id),origin:{...(positions[id]||{x:0,y:0})},start:point(e)};handle.setPointerCapture(e.pointerId)});
      handle.addEventListener('pointermove',e=>{if(gesture?.id!==id)return;const p=point(e);positions[id]={x:Math.max(-x,gesture.origin.x+p.x-gesture.start.x),y:Math.max(-y,gesture.origin.y+p.y-gesture.start.y)};update()});
      handle.addEventListener('pointerup',()=>finish());handle.addEventListener('pointercancel',()=>finish(true));
      const copy=svgNode('g',{class:'graph-copy',role:'button',tabindex:0,'aria-label':t('Copier l’identifiant')+' '+id});copy.dataset.task=id;copy.append(svgNode('title',{},t('Copier l’identifiant')+' '+id),svgNode('rect',{x:x+w-82,y:y+2,width:44,height:44,rx:6}),svgNode('text',{x:x+w-68,y:y+28},'ID'));group.append(copy);copy.addEventListener('pointerdown',e=>e.stopPropagation());copy.addEventListener('click',e=>{e.stopPropagation();copyId(id,copy)});copy.addEventListener('keydown',e=>{if(['Enter',' '].includes(e.key)){e.preventDefault();e.stopPropagation();copyId(id,copy)}});
      for(const side of ['in','out']){
        const vertical=orientation==='TB',port=svgNode('circle',{class:'graph-port',role:'button',tabindex:0,r:9,cx:vertical?x+w/2:side==='in'?x:x+w,cy:vertical?(side==='in'?y:y+h):y+h/2,'aria-label':t(side==='out'?'Relier cette sortie':'Relier cette entrée')+' '+id});port.dataset.task=id;port.dataset.side=side;let suppressClick=false;group.append(port);
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
    map.addEventListener('keydown',e=>{if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End'].includes(e.key))return;e.preventDefault();e.stopPropagation();if(e.key==='Home')canvas.scrollTo(0,0);else if(e.key==='End')canvas.scrollTo(canvas.scrollWidth,canvas.scrollHeight);else{canvas.scrollLeft+=(e.key==='ArrowLeft'?-1:e.key==='ArrowRight'?1:0)*canvas.clientWidth/2;canvas.scrollTop+=(e.key==='ArrowUp'?-1:e.key==='ArrowDown'?1:0)*canvas.clientHeight/2}updateViewport()});canvas.addEventListener('scroll',updateViewport);
    canvas.addEventListener('pointerdown',e=>{if(e.button!==0||e.target.closest('.graph-noeud'))return;e.preventDefault();canvas.focus();gesture={target:canvas,pointer:e.pointerId,start:{x:e.clientX,y:e.clientY},left:canvas.scrollLeft,top:canvas.scrollTop};canvas.setPointerCapture(e.pointerId)});
    canvas.addEventListener('pointermove',e=>{if(!gesture||gesture.id||gesture.link)return;canvas.scrollLeft=gesture.left+gesture.start.x-e.clientX;canvas.scrollTop=gesture.top+gesture.start.y-e.clientY});canvas.addEventListener('pointerup',()=>finish());canvas.addEventListener('pointercancel',()=>finish(true));
    canvas.addEventListener('wheel',e=>{if(!e.ctrlKey)return;e.preventDefault();zoom=Math.max(.15,Math.min(2,zoom+(e.deltaY<0?.15:-.15)));update();save()},{passive:false});
    for(const edge of svg.querySelectorAll('.graph-arete')){edge.setAttribute('role','button');edge.setAttribute('tabindex','0');edge.setAttribute('aria-label',t('Retirer ce lien')+' '+edge.dataset.from+' → '+edge.dataset.to);const remove=()=>{const target=work.tasks.find(t=>t.id===edge.dataset.to);editTask(work,target,(target.depends||[]).filter(id=>id!==edge.dataset.from))};edge.addEventListener('click',remove);edge.addEventListener('keydown',e=>{if(['Enter',' ','Delete','Backspace'].includes(e.key)){e.preventDefault();remove()}})}
    studio.addEventListener('keydown',e=>{if(e.key!=='Escape')return;if(gesture||connection){e.preventDefault();e.stopPropagation();finish(true);connection=null;status.textContent=''}else if(modal.open){e.preventDefault();modal.close()}});
    modal.addEventListener('close',()=>{if(disposed)return;finish(true);host.insertBefore(studio,modal);studio.classList.remove('is-fullscreen');document.body.classList.remove('graph-studio-open');full.textContent=t('Plein écran');full.setAttribute('aria-expanded','false');full.focus()});
    const viewportObserver=new ResizeObserver(updateViewport);viewportObserver.observe(canvas);
    studio.append(toolbar,help,status,copyStatus,copyFallback,canvas);host.replaceChildren(studio,modal);host.graphCleanup=()=>{disposed=true;viewportObserver.disconnect();finish(true);if(modal.open)modal.close();document.body.classList.remove('graph-studio-open')};update();
  }

  /**
   * Render the dependency graph of a Work into the original #graph container.
   * Recorded projection with local navigation and explicit task-editor handlers.
   * Rendering performs no writes; saving an editor uses protected engine operations.
   * Side effect: replaces the container's children (scrollable canvas + svg).
   * @param {object} work Work {id,title,tasks:[{id,title,status,depends?:string[]}]}
   * @throws {Error} on invalid Work, duplicate ids, unknown-shaped dependencies or cycles
   */
  function render(work) {
    const host = container();
    host.graphCleanup?.();host.graphCleanup=null;
    const orientation=host.dataset.orientation==='TB'?'TB':'LR';
    const { tasks, depPairs, unknown } = validateWork(work);
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
    navigation(host,canvas,svg,work,orientation);
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
   * Side effect: replaces the container's children.
   * @param {string} message literal message to display
   * @throws {Error} when the container is absent
   */
  function clear(message) {
    const host = container();
    host.graphCleanup?.();host.graphCleanup=null;
    const p = document.createElement('p');
    p.textContent = typeof message === 'string' ? message : '';
    host.replaceChildren(p);
  }

  globalThis.SwarmRustDependencyGraph = { render, clear, setRefreshHandler };
})();
