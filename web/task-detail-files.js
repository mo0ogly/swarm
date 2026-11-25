'use strict';
(()=>{
const KINDS=new Set(['json','log','markdown','text','document']);
const KIND_LABELS={json:'JSON',log:'Journal',markdown:'Markdown',text:'Texte',document:'Document'};
const KIND_LANG={json:'json',markdown:'markdown',log:'plaintext',text:'plaintext'};
const LOG_PATTERNS=[
 ['error',/\b(error|erreur|fatal|critique|panic|failed|failure)\b/i],
 ['warn',/\b(warn(?:ing)?|alerte|avertissement)\b/i],
 ['info',/\b(debug|débogage|trace)\b/i]
];

const tr=(key)=>globalThis.SwarmI18n?.t?.(key)??key;
function diagnostic(error){
 const code=error?.code||error?.message;
 return typeof code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(code)?' ('+code+')':'';
}
/** Translate the known transport refusal without displaying its response body. */
function readFailure(error,fallback){
 return tr(error?.code==='web-workers-saturated'?'Le serveur traite déjà d’autres lectures.':fallback);
}
const el=(tag,text,cls)=>{
 const node=document.createElement(tag);
 if(text!=null)node.textContent=text;
 if(cls)node.className=cls;
 return node;
};
const setStatus=(node,message,error=false)=>{
 node.textContent=message;
 node.hidden=!message;
 node.className='task-detail-file-status '+(error?'notice alert':'notice info');
};

let stylesheet=null;
function monacoBundle(){
 stylesheet??=new Promise((resolve,reject)=>{
  const link=el('link');
  link.rel='stylesheet';
  link.href='/lib/monaco/editor.css';
  link.addEventListener('load',resolve,{once:true});
  link.addEventListener('error',()=>{link.remove();stylesheet=null;reject(Error('task-files-monaco-css'));},{once:true});
  document.head.append(link);
 });
 globalThis.MonacoEnvironment??={
  getWorker:(_module,label)=>new Worker(label==='json'?'/lib/monaco/json-worker.js':'/lib/monaco/worker.js',{type:'module'})
 };
 // The bundled JSON mode registers editor services. Load it before the
 // first editor freezes the service registry, including on later remounts.
 return Promise.all([import('/lib/monaco/editor.js'),import('/lib/monaco/jsonMode-FCANVPFQ.js'),stylesheet])
  .then(([lib])=>lib);
}

let uid=0;

/** Mount a read-only explorer in a connected dialog. Each source loads its
 * group independently and returns the current descriptor list. The caller owns
 * revision and read authority. Prefer the submitted report when available,
 * preserve explicit selection across retries and ignore results after disposal. */
function mount(host,{read,initialFiles=[],sources=[],groupOrder=[],technical,preferredId}={}){
 if(!(host instanceof Element))throw Error('task-files-host-required');
 if(typeof read!=='function')throw Error('task-files-callbacks-required');

 let alive=true,ticket=0,treeHasFiles=false;
 let monaco=null,editor=null,model=null,decorations=null,themeObserver=null;
 let currentId='';
 const groups=new Map(sources.map(source=>[source.label,{source,state:'loading'}]));
 const primary=[...groups.values()].find(group=>group.source.primary);
 const openPaths=new Map();
 let files=[];

 // All validation controls belong to the surrounding dialog.
 const root=el('section','','swarm-files');

 const titleId='swarm-task-files-title-'+(++uid);
 const title=el('h2',tr('Fichiers de la tâche'),'swarm-files-title');
 title.id=titleId;
 root.setAttribute('aria-labelledby',titleId);

 const search=el('input','','swarm-files-search');
 search.type='search';
 search.autocomplete='off';
 search.spellcheck=false;
 search.placeholder=tr('Filtrer les fichiers…');
 search.setAttribute('aria-label',tr('Filtrer les fichiers'));
 search.addEventListener('input',()=>applyFilter(search.value));

 const treeStatus=el('p',undefined,'task-detail-file-status');treeStatus.setAttribute('role','status');
 const treeList=el('ul','','task-detail-file-list');
 const tree=el('nav','','task-detail-file-tree');
 tree.id=titleId+'-tree';
 tree.setAttribute('aria-label',tr('Arborescence des fichiers de la tâche'));
 tree.append(title,search,treeStatus,treeList);
 const compact=matchMedia('(max-width:760px)');
 const filesToggle=el('button',tr('Fichiers'),'task-detail-mobile-files');
 filesToggle.type='button';
 filesToggle.setAttribute('aria-controls',tree.id);
 filesToggle.setAttribute('aria-expanded','false');
 filesToggle.addEventListener('click',()=>{
  const opening=filesToggle.getAttribute('aria-expanded')==='false';
  filesToggle.setAttribute('aria-expanded',String(opening));
  if(opening)search.focus();
 });
 const fileNav=el('div','','task-detail-file-nav');
 fileNav.append(filesToggle,tree);
 // Roving focus for the disclosure tree (Enter/Space/Tab stay native; Escape untouched).
 treeList.addEventListener('keydown',(event)=>{
  if(event.key!=='ArrowDown'&&event.key!=='ArrowUp')return;
  const buttons=[...treeList.querySelectorAll('button')].filter((b)=>b.offsetParent!==null);
  const index=buttons.indexOf(event.target);
  if(index<0)return;
  event.preventDefault();
  const step=event.key==='ArrowDown'?1:-1;
  buttons[(index+step+buttons.length)%buttons.length].focus();
 });

 const pathLabel=el('code','','swarm-files-path');
 const kindBadge=el('span','','task-detail-file-type');
 const truncated=el('p',tr('Aperçu tronqué : le contenu affiché est incomplet.'),'notice info swarm-files-truncated');
 truncated.hidden=true;
 const find=el('button',tr('Rechercher…'),'swarm-files-find');
 find.type='button';
 find.disabled=true;
 find.title=tr('Rechercher dans le fichier');
 find.addEventListener('click',()=>{if(editor)editor.trigger('swarm-task-files','actions.find');});
 find.dataset.taskFileFind='';
 const download=el('button',tr('Télécharger l’original'),'task-detail-file-download');
 download.type='button';download.hidden=true;
 const bar=el('div','','task-detail-file-toolbar');
 bar.append(pathLabel,kindBadge,find,download);
 bar.hidden=true;
 const viewerStatus=el('p',undefined,'task-detail-file-status');viewerStatus.setAttribute('role','status');
 const retryViewer=el('button',tr('Réessayer'),'task-detail-file-retry');
 retryViewer.type='button';retryViewer.hidden=true;
 const pre=el('pre','','task-detail-file-plain');
 pre.hidden=true;
 pre.tabIndex=0;
 pre.setAttribute('aria-label',tr('Aperçu texte du fichier'));
 const editorHost=el('div','','task-detail-file-editor');
 editorHost.hidden=true;
 const viewer=el('section','','task-detail-file-viewer');
 viewer.tabIndex=-1;
 viewer.setAttribute('aria-label',tr('Aperçu du fichier, lecture seule'));
 viewer.append(bar,truncated,viewerStatus,retryViewer,pre,editorHost);
 const body=el('div','','task-detail-file-layout');
 body.append(fileNav,viewer);
 root.append(body);
 host.append(root);

 // — theme: wattson variables + sombre variant (same approach as the Debug journal) —
 const syncTheme=()=>{
  const sombre=document.documentElement.dataset.theme==='sombre';
  if(!monaco)return;
  const computed=getComputedStyle(document.documentElement),value=(key)=>computed.getPropertyValue('--wattson-'+key).trim();
  monaco.editor.defineTheme('swarm-task-files',{base:sombre?'vs-dark':'vs',inherit:true,rules:[],
   colors:{'editor.background':value('champ'),'editor.foreground':value('texte'),'editorLineNumber.foreground':value('texte-discret')}});
  monaco.editor.setTheme('swarm-task-files');
 };
 syncTheme();
 themeObserver=new MutationObserver(syncTheme);
 themeObserver.observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});

 const byName=(a,b)=>a.localeCompare(b,undefined,{numeric:true,sensitivity:'base'});

 /** Refresh directory descriptors and open the submitted report exactly once
  * when resolved. Discovery never replaces a user's explicit selection. */
 function adopt(items){
  if(!alive)return;
  if(!Array.isArray(items)||items.some(item=>!item||typeof item.id!=='string'
     ||typeof item.path!=='string'||!item.path||!KINDS.has(item.kind))
     ||new Set(items.map(item=>item.id)).size!==items.length)throw Error('task-files-list-invalid');
  files=items;
  treeHasFiles=items.length>0;
  renderTree(files);
  applyFilter(search.value);
  if(!currentId){
   const report=items.find(item=>item.id===preferredId&&item.readable!==false);
   if(report){select(report);return;}
   retryViewer.hidden=primary?.state!=='error';
   retryViewer.onclick=retryViewer.hidden?null:()=>loadGroup(primary);
   const message=primary?.state==='loading'?tr('Chargement du rapport remis…')
    :primary?.state==='error'?readFailure(primary.error,'Impossible de charger les livrables.')
    :tr('Aucun rapport à ouvrir. Choisissez un fichier dans l’arborescence.');
   setStatus(viewerStatus,message,primary?.state==='error');
  }
 }
 /** Retry only one read-only discovery request; leave the selected file and
  * unrelated sources in place. Keep failed discovery visible until retried. */
 async function loadGroup(group){
  if(group.active||!alive)return;
  group.active=true;group.state='loading';
  adopt(files);
  try{
   const items=await group.source.load();
   if(!alive)return;
   group.state='ready';
   adopt(items);
  }catch(error){
   group.state='error';group.error=error;
   if(alive)adopt(files);
  }finally{group.active=false;}
 }

 /** Rebuild the current tree while preserving disclosed paths and keyboard
  * focus on a surviving file or folder during an asynchronous refresh. */
 function renderTree(files){
  const rootNode={dirs:new Map(),files:[]};
  const groupNode=()=>({dirs:new Map(),files:[]});
  for(const label of groupOrder){
   const group=groups.get(label);
   if(group&&group.state!=='ready')rootNode.dirs.set(label,groupNode());
  }
  for(const file of files){
   const segments=file.path.split('/').filter(Boolean);
   const name=segments[segments.length-1]||file.path;
   let node=rootNode;
   for(const segment of segments.slice(0,-1)){
    let child=node.dirs.get(segment);
    if(!child)node.dirs.set(segment,child=groupNode());
    node=child;
   }
   node.files.push({file,name});
  }
  const focused=treeList.contains(document.activeElement)?document.activeElement.dataset:null;
  treeList.replaceChildren();
  fillGroup(rootNode,treeList);
  if(focused){
   const target=[...treeList.querySelectorAll('button')].find(button=>
    (focused.fileId&&button.dataset.fileId===focused.fileId)||(focused.directory&&button.dataset.directory===focused.directory));
   (target||search).focus({preventScroll:true});
  }
 }

 /** Render typed rows and group-local loading or recovery controls. */
 function fillGroup(node,group,parent=[]){
  const priority=name=>groupOrder.includes(name)?groupOrder.indexOf(name):groupOrder.length;
  const dirs=[...node.dirs].sort((a,b)=>(parent.length?0:priority(a[0])-priority(b[0]))||byName(a[0],b[0]));
  for(const [name,child] of dirs){
   const segments=[...parent,name],key=segments.join('/');
   const open=openPaths.get(key)??(parent.length>0||name!==technical);
   const toggle=el('button','','swarm-files-dir-toggle');
   toggle.type='button';
   toggle.dataset.directory=key;
   toggle.setAttribute('aria-expanded',String(open));
   const caret=el('span','▸','swarm-files-caret');
   caret.setAttribute('aria-hidden','true');
   toggle.append(caret,el('span',name,'swarm-files-dir-name'));
   const children=el('ul','','task-detail-file-list');
   children.hidden=!open;
   fillGroup(child,children,segments);
   const state=parent.length===0?groups.get(name):null;
   if(state&&state.state!=='ready'){
    const row=el('li',undefined,'task-detail-group-state');
    const status=el('p');status.setAttribute('role','status');
    const failed=state.state==='error';
    setStatus(status,failed?readFailure(state.error,'Lecture indisponible'):tr('Chargement…'),failed);
    row.append(status);
    if(failed){
     status.title=diagnostic(state.error).trim();
     const retry=el('button',tr('Réessayer'));
     retry.type='button';retry.addEventListener('click',()=>{toggle.focus();loadGroup(state)});
     row.append(retry);
    }
    children.prepend(row);
   }
   toggle.addEventListener('click',()=>{
    const open=toggle.getAttribute('aria-expanded')==='true';
    openPaths.set(key,!open);
    toggle.setAttribute('aria-expanded',String(!open));
    children.hidden=open;
   });
   const li=el('li','','swarm-files-dir');
   li.append(toggle,children);
   group.append(li);
  }
  for(const entry of [...node.files].sort((a,b)=>byName(a.name,b.name))){
   const file=entry.file;
   const button=el('button','','task-detail-file-row');
   button.type='button';
   button.dataset.path=file.path;button.dataset.fileId=file.id;
   if(file.id===currentId)button.setAttribute('aria-current','true');
   button.setAttribute('aria-label',file.path+' — '+tr(KIND_LABELS[file.kind]));
   const tag=el('span',tr(KIND_LABELS[file.kind]),'task-detail-file-type');
   tag.dataset.kind=file.kind;
   button.append(tag,el('span',entry.name,'task-detail-file-row-name'));
   button.addEventListener('click',()=>{
    select(file);
    if(compact.matches){
     filesToggle.setAttribute('aria-expanded','false');
     viewer.focus({preventScroll:true});
    }
   });
   const li=el('li','','task-detail-file-item');
   li.append(button);
   if(file.id===preferredId)group.prepend(li);
   else group.append(li);
  }
 }

 function applyFilter(raw){
  if(!treeHasFiles)return;
  const query=raw.trim().toLowerCase();
  let visible=0;
  for(const li of treeList.querySelectorAll('.task-detail-file-item')){
   const match=!query||li.firstElementChild.dataset.path.toLowerCase().includes(query);
   li.hidden=!match;
   if(match)visible++;
  }
  // Deepest first, so a directory keeps any visible descendant.
  for(const dir of [...treeList.querySelectorAll('.swarm-files-dir')].reverse()){
   const has=dir.querySelector('.task-detail-file-item:not([hidden]),.swarm-files-dir:not([hidden])');
   dir.hidden=Boolean(query)&&!has;
   const toggle=dir.querySelector('.swarm-files-dir-toggle');
   const open=query?Boolean(has):openPaths.get(toggle.dataset.directory)??toggle.dataset.directory!==technical;
   toggle.setAttribute('aria-expanded',String(open));
   toggle.nextElementSibling.hidden=!open;
  }
  setStatus(treeStatus,query&&!visible?tr('Aucun fichier ne correspond au filtre.'):'');
 }

 // — selection (generation-tracked; stale responses are dropped) —
 /** Report a failed selected-file request and offer one explicit retry. The
  * caller has already checked that the selection and viewer are still current. */
 function retrySelected(error,retry){
  setStatus(viewerStatus,readFailure(error,'Impossible de lire ce fichier.')+diagnostic(error),true);
  retryViewer.hidden=false;retryViewer.onclick=()=>{viewer.focus();retry()};
 }
 /** Read one declared file. Retry stays attached to that selection and late
  * responses cannot overwrite a newer file or a closed dialog. */
 async function select(file){
  const started=performance.now();
  const ticketValue=++ticket;
  delete viewer.dataset.reviewTiming;
  currentId=String(file.id);
  for(const selected of treeList.querySelectorAll('[aria-current=true]'))selected.removeAttribute('aria-current');
  [...treeList.querySelectorAll('[data-file-id]')].find(row=>row.dataset.fileId===file.id)?.setAttribute('aria-current','true');

  bar.hidden=false;
  pathLabel.textContent=file.path;
  kindBadge.textContent=tr(KIND_LABELS[file.kind]);kindBadge.dataset.kind=file.kind;
  download.hidden=true;download.onclick=null;
  truncated.hidden=true;
  pre.hidden=true;
  pre.textContent='';
  editorHost.hidden=true;
  find.disabled=true;
  retryViewer.hidden=true;retryViewer.onclick=null;

  if(file.readable===false){setStatus(viewerStatus,tr('Ce fichier ne peut pas être affiché.'));return;}
  if(file.kind==='document'){
   setStatus(viewerStatus,tr('Téléchargez le document original pour l’ouvrir.'));
   download.hidden=false;download.disabled=false;
   // One explicit request for the current document, followed by a local save
   // only if the selection survives. Failure leaves the same file retryable.
   async function downloadOriginal(){
    download.disabled=true;
    retryViewer.hidden=true;retryViewer.onclick=null;
    setStatus(viewerStatus,tr('Chargement de « {path} »…').replace('{path}',file.path));
    try{
     const result=await read(file.id);
     if(!alive||ticketValue!==ticket)return;
     if(typeof result?.download!=='string')throw Error('task-files-content-invalid');
     const bytes=Uint8Array.from(atob(result.download),character=>character.charCodeAt(0));
     const url=URL.createObjectURL(new Blob([bytes],{type:'application/octet-stream'}));
     const link=el('a');link.href=url;link.download=file.path.split('/').at(-1);
     try{viewer.append(link);link.click()}finally{link.remove();requestAnimationFrame(()=>URL.revokeObjectURL(url))}
     setStatus(viewerStatus,tr('Téléchargez le document original pour l’ouvrir.'));
    }catch(error){
     if(alive&&ticketValue===ticket)retrySelected(error,downloadOriginal);
    }finally{if(alive&&ticketValue===ticket)download.disabled=false}
   }
   download.onclick=downloadOriginal;
   return;
  }
  setStatus(viewerStatus,tr('Chargement de « {path} »…').replace('{path}',file.path));

  let result;
  try{result=await read(file.id);}
  catch(error){
   if(!alive||ticketValue!==ticket)return;
   retrySelected(error,()=>select(file));
   return;
  }
  if(!alive||ticketValue!==ticket)return;
  if(!result
    ||typeof result.text!=='string'
    ||(result.language!==undefined&&typeof result.language!=='string')
    ||(result.truncated!==undefined&&typeof result.truncated!=='boolean')){
   setStatus(viewerStatus,tr('Contenu de fichier invalide.')+' (task-files-content-invalid)',true);
   return;
  }
  truncated.hidden=!result.truncated;
  if(result.text===''){setStatus(viewerStatus,tr('Le fichier est vide.'));return;}
  const renderStarted=performance.now();
  await show(result,file.kind,ticketValue);
  // The opt-in sample is retained on the current viewer only. Two frames give
  // Monaco a paint opportunity without blocking normal selection or retries.
  if(result.timing)requestAnimationFrame(()=>requestAnimationFrame(()=>{
   if(!alive||ticketValue!==ticket)return;
   const rendered=performance.now();
   viewer.dataset.reviewTiming=JSON.stringify({...result.timing,
    request:renderStarted-started,render:rendered-renderStarted,total:rendered-started,
    mode:editorHost.hidden?'text':'monaco'});
  }));
 }

 async function show(result,kind,ticketValue){
  const language=result.language||KIND_LANG[kind]||'plaintext';
  try{
   monaco=await monacoBundle();
   if(!alive||ticketValue!==ticket)return;
   syncTheme();
   if(!editor){
    editorHost.hidden=false;
    editor=monaco.editor.create(editorHost,{model:null,theme:'swarm-task-files',
     readOnly:true,domReadOnly:true,automaticLayout:true,wordWrap:'on',minimap:{enabled:false},
     scrollBeyondLastLine:false,renderLineHighlight:'none',links:false,editContext:false,
     ariaLabel:tr('Aperçu du fichier, lecture seule')});
    decorations=editor.createDecorationsCollection();
   }
   const replacement=monaco.editor.createModel(result.text,language);
   try{editor.setModel(replacement)}catch(error){replacement.dispose();throw error}
   model?.dispose();
   model=replacement;
   decorateLog(kind);
   pre.hidden=true;
   editorHost.hidden=false;
   setStatus(viewerStatus,'');
   find.disabled=false;
   find.title=tr('Rechercher dans le fichier');
  }catch{
   // Readable plain-text fallback; editor/model survive for a later retry.
   if(!alive||ticketValue!==ticket)return;
   editorHost.hidden=true;
   pre.textContent=result.text;
   pre.hidden=false;
   find.disabled=true;
   find.title=tr('Recherche indisponible');
   setStatus(viewerStatus,tr('Monaco est indisponible. Le fichier reste lisible en texte.'),true);
  }
 }

 function decorateLog(kind){
  if(!decorations)return;
  decorations.set([]);
  if(kind!=='log'||!model)return;
  const ranges=[];
  const total=model.getLineCount();
  for(let number=1;number<=total;number++){
   const content=model.getLineContent(number);
   for(const [level,pattern] of LOG_PATTERNS){
    if(pattern.test(content)){
     ranges.push({range:new monaco.Range(number,1,number,1),options:{isWholeLine:true,className:'task-detail-log-'+level}});
     break;
    }
   }
  }
  decorations.set(ranges);
 }

 adopt(initialFiles);
 for(const group of groups.values())loadGroup(group);
 return function dispose(){
  if(!alive)return;
  alive=false;
  ticket++;
  themeObserver?.disconnect();
  themeObserver=null;
  try{editor?.dispose()}finally{model?.dispose();root.remove()}
  editor=null;model=null;monaco=null;decorations=null;
 };
}

globalThis.SwarmTaskDetailFiles=Object.freeze({mount});
})();
