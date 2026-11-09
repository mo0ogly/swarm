/* Validated workbench design: presentation of recorded Work/status only. */
(()=>{
 'use strict';
 const t=s=>globalThis.SwarmI18n?.t(s)??s,engine=s=>globalThis.SwarmI18n?.engine(s)??s;
 let mounted=false,view='graph',workId='',model=null,selected='',query='',role='',host,lanes,detail,feedback,search,roleSelect,buttons,summary,filters,filter='',observer;
 const preference='swarm-conduite:view:v1';
 /** Create a detached text-only element. User records are never parsed as HTML. */
 function el(tag,text,cls){const node=document.createElement(tag);if(text!==undefined)node.textContent=text;if(cls)node.className=cls;return node}
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
 /** Render the selected recorded task; dependencies come from Work, observations
  * from its matching projection. No task mutation, acceptance or agent launch. */
 function inspect(){detail.replaceChildren();const task=model?.work.tasks.find(task=>task.id===selected);if(!task){detail.append(el('h3',t('Choisissez une tâche')),el('p',t('Sélectionnez une fiche pour consulter son état et ses dépendances.')));return}const projection=model.projections.get(task.id),heading=el('h3',task.title||task.id);heading.tabIndex=-1;detail.append(el('p',task.id,'eyebrow'),heading);const state=projection?.label||projection?.state||task.status;detail.append(statusBadge(projection?.state||task.status,state));
  const facts=el('dl',undefined,'native-facts');for(const [label,value]of [[t('Rôle'),task.plan_role||task.owner||t('Non attribué')],[t('Dépend de'),task.depends?.join(', ')||t('Aucune dépendance enregistrée')],[t('Débloque'),model.work.tasks.filter(item=>item.depends?.includes(task.id)).map(item=>item.id).join(', ')||'—']])facts.append(el('dt',label),el('dd',value));detail.append(facts);if(projection?.reason)detail.append(el('p',engine(projection.reason),'notice warn'));if(task.deliverable)detail.append(el('p',task.deliverable));const graph=el('button',t('Voir dans le graphe'));graph.type='button';graph.addEventListener('click',()=>{choose('graph');const target=[...document.querySelectorAll('#graph [data-task]')].find(node=>node.dataset.task===task.id);target?.scrollIntoView({block:'nearest',inline:'center'});target?.querySelector('.graph-task-open')?.focus()});detail.append(graph);
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
 /** Paint the three summary cards from recorded states only: review-like states
  * (to check), blocked/failed states, and accepted over total. "See" switches to
  * lanes and selects the first matching task locally; it starts no agent, writes
  * nothing and grants no decision. Without a model every count shows "—". */
 function summarize(){if(!summary)return;summary.replaceChildren();const tasks=model?.work.tasks||[],known=!!model;
  const pick=tone=>tasks.filter(task=>signal(stateOf(task))===tone);
  for(const [tone,title]of [['warn','À vérifier'],['alert','Bloquées']]){const matches=pick(tone),card=el('article',undefined,'workbench-summary-card '+tone);card.dataset.summary=tone;const count=el('strong',known?String(matches.length):'—','workbench-count '+tone),text=el('div');text.append(el('h3',t(title)),el('p',known?(matches.length?(matches[0].title||matches[0].id):t('Aucune tâche dans cet état.')):t('Aucune mission sélectionnée.')));card.append(count,text);
   if(known&&matches.length){const see=el('button',t('Voir'));see.type='button';see.addEventListener('click',()=>{selected=matches[0].id;choose('lanes');paint();detail.querySelector('h3')?.focus()});card.append(see)}summary.append(card)}
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
 function follow(id){if(!id||id===selected||!model?.work.tasks.some(task=>task.id===id))return;selected=id;for(const peer of lanes.querySelectorAll('[data-lane-task]'))peer.setAttribute('aria-pressed',String(peer.dataset.laneTask===selected));inspect()}
 /** Mount once into the canonical cockpit. A missing host is a harmless non-
  * cockpit page. The component owns only its controls, lanes and inspector;
  * it moves #graph into a stage whose side inspector serves graph and lanes. */
 function mount(){if(mounted)return;host=document.querySelector('#conduite .conduite-principal');if(!host)return;mounted=true;document.body.classList.add('workbench-cockpit');const top=el('div',undefined,'workbench-view-switch');top.setAttribute('role','group');top.setAttribute('aria-label',t('Vue de la conduite'));buttons={};for(const [key,label]of [['graph','Graphe'],['lanes','Couloirs']]){const button=el('button',t(label));button.type='button';button.dataset.workbenchView=key;button.addEventListener('click',()=>choose(key));buttons[key]=button;top.append(button)}feedback=el('p',undefined,'hint');feedback.setAttribute('role','status');host.querySelector('.graph-heading')?.append(top);summary=el('section',undefined,'workbench-summary');summary.setAttribute('aria-label',t('Synthèse des états enregistrés'));host.querySelector('.graph-heading')?.before(summary);host.querySelector('.graph-heading')?.after(feedback);filters=el('div',undefined,'workbench-filters');filters.setAttribute('role','group');filters.setAttribute('aria-label',t('Filtrer par état enregistré'));feedback.after(filters);const graphHost=document.getElementById('graph');if(graphHost&&typeof MutationObserver==='function'){observer=new MutationObserver(()=>{if(filter)dimGraph()});observer.observe(graphHost,{childList:true,subtree:true})}
  lanes=el('section',undefined,'workbench-lanes');lanes.id='workbench-lanes';lanes.setAttribute('aria-label',t('Couloirs de la mission'));const toolbar=el('div',undefined,'toolbar'),searchLabel=el('label',t('Rechercher une tâche'));search=el('input');search.type='search';search.id='workbench-search';search.addEventListener('input',()=>{query=search.value;paint()});searchLabel.append(search);const roleLabel=el('label',t('Rôle'));roleSelect=el('select');roleSelect.id='workbench-role';roleSelect.addEventListener('change',()=>{role=roleSelect.value;paint()});roleLabel.append(roleSelect);toolbar.append(searchLabel,roleLabel);const content=el('div',undefined,'workbench-lanes-content'),matrix=el('div',undefined,'workbench-matrix');matrix.tabIndex=0;matrix.setAttribute('aria-label',t('Rôles et états enregistrés'));detail=el('aside',undefined,'workbench-inspector');detail.setAttribute('aria-label',t('Détail de la tâche'));content.append(matrix);lanes.append(toolbar,content);const graphEl=document.getElementById('graph'),stage=el('div',undefined,'workbench-stage'),column=el('div',undefined,'workbench-stage-main');graphEl.before(stage);column.append(graphEl,lanes);stage.append(column,detail);graphEl.addEventListener('focusin',event=>follow(event.target?.closest?.('[data-task]')?.dataset.task));try{const saved=localStorage.getItem(preference);if(['graph','lanes'].includes(saved))view=saved}catch{feedback.textContent=t('Préférence d’affichage non mémorisée sur cet appareil.')}choose(view,false);clear(t('Aucune mission sélectionnée.'));
 }
 /** Validate a new read model, rejecting cross-Work or duplicate records before
  * display. Changed Work clears selection/filter. Does not turn read errors into
  * success: a missing/failed projection keeps the visible recorded-only notice.
  * Repaints the recorded-state summary cards from the same validated model. */
 function render(data){mount();if(!mounted)return;try{const work=data?.work;if(!work||typeof work.id!=='string'||!Array.isArray(work.tasks)||!work.tasks.every(task=>task&&typeof task.id==='string'&&typeof task.title==='string'&&(task.depends===undefined||task.depends===null||Array.isArray(task.depends)&&task.depends.every(id=>typeof id==='string')))||new Set(work.tasks.map(task=>task.id)).size!==work.tasks.length)throw Error('workbench-work-invalid');if(work.id!==workId){selected='';query='';role='';filter='';search.value='';workId=work.id}const projections=new Map();if(!data.missionError&&Array.isArray(data.mission?.tasks)){for(const p of data.mission.tasks){if(!p||typeof p.id!=='string'||!work.tasks.some(task=>task.id===p.id)||projections.has(p.id))throw Error('workbench-projection-invalid');projections.set(p.id,p)}}model={work,projections};if(!work.tasks.some(task=>task.id===selected))selected=work.tasks[0]?.id||'';roleSelect.replaceChildren(new Option(t('Tous les rôles'),''));for(const name of new Set(work.tasks.map(task=>task.plan_role||task.owner||'').filter(Boolean)))roleSelect.append(new Option(name,name));roleSelect.value=role;paint();summarize();paintFilters();dimGraph();lanes.dataset.work=work.id;lanes.dataset.observed=String(!data.missionError&&!!data.mission);if(data.missionError)feedback.textContent=t('Suivi détaillé indisponible. Les fiches affichent les données enregistrées.');else feedback.textContent=t('Les couloirs présentent les rôles et états reçus, sans autoriser une action.');}catch(error){clear(t('Lecture des couloirs impossible. Actualisez pour reprendre.')+' ('+error.message+')');return false}return true}
 /** Clear the old mission immediately, including inspector and filters. Called
  * before new reads; never exposes previous-Work data while loading fails.
  * Summary cards return to "—" with the lanes. */
 function clear(message){if(!mounted)return;model=null;selected='';workId='';query='';role='';search.value='';roleSelect.replaceChildren(new Option(t('Tous les rôles'),''));lanes.querySelector('.workbench-matrix').replaceChildren(el('p',message));detail.replaceChildren();delete lanes.dataset.work;filter='';summarize();paintFilters();dimGraph()}
 globalThis.SwarmWorkbench=Object.freeze({mount,render,clear,statusBadge,agentStatusBadge});
})();
