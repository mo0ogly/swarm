# Dossier de revue B03 — édition du graphe avec aperçu

## Identité, portée et limite critique

- Mission / tâche / tentative / producteur de reprise : `w-768ed45d1845fbec90bea04d` / `B03` / `a-1925d779502529f20d3d9024` / `auto-a3f26dc72d4f0e385c45`.
- Candidat : HEAD `cc3069dc7bb61b90168d21f945cb2eb5e27578ed`, checkout sale partagé, révision de travail 61 au départ ; correction du focus présente, aucun commit ni acceptation.
- Portée : édition web reliée aux routes B01, renderer produit existant, recette B03, i18n et aides FR/EN.
- Correction du reçu bloqué : `selectNode` utilise désormais `$eval(selector, element => element.focus())`; l’empreinte de la recette passe de `5d1ff3…` (reçu) à `b9df25…` (candidat courant).
- Limite fraîche : le contrôle exact sort 1 avant Chromium car ce fournisseur ajoute `EPERM` à `spawnSync` malgré `status:0` et un stdout JSON valide; un essai Chrome distinct est refusé sur `setsockopt`. Les résultats/captures hôte conservés sur les empreintes ci-dessous restent historiques, pas un PASS frais.
- Revue indépendante requise après rejeu conducteur. Ce dossier complète docs/B03.md ; aucune acceptation n’est annoncée.

## Contrat à examiner

Le mode édition se superpose au graphe Dagre/SVG existant. Clic, Entrée ou Espace sur deux nœuds produit `add_dependency` ou `remove_dependency`. Chaque proposition non vide undo/redo est persistée par `POST /api/v1/graph-drafts`. Le retour à vide reste local, désactive apply et ne supprime aucune proposition enregistrée; preview et apply restent séparés. Un conflit conserve l'historique local et impose la recréation sur la révision courante. Apply utilise un nouvel `event_id`, puis relit le snapshot. B01 reste seul responsable des cycles, doublons, tâches inconnues, droits, portées actives et conflits.

La mini-carte dérive du SVG produit, sans dépendance ajoutée. Le statut annonce tâches/liens cachés. Styles : jetons Wattson existants. Traductions : `locales/en.json`, puis génération de `web/i18n-en.js`.

## Empreintes courantes
```text
641351c57a57609120aea0e446a9861dbe068409b2d9f5494aff26a019a34c5f  web/graph-draft.js
05a40f9cfd160969432750a079a4b0bf2e49834ea6c3eca74c7c5a3f300f2ff8  web/graph.js
27cb5ab12a0e0cc1e67a9a7c08b8eba0421bbd4bb3fba050f49c2145d7f0b6f5  web/pilotage.js
c3a323463a290dbcad5be6dd91b4ff8554be0577379d7ecb3ff87a03158eae93  web/cockpit.js
71dedb20ce6517e612e73d326b5843888ab9e4ef3fe341e2272aba743882a569  web/pilotage.css
6fd7640121fe1cf9b14a5fc00eef4ac08dc124aba500e8e7a13e38fe217f4074  web/graph.css
56bfc03d1fa6a612fc6f94b5b5ff0b885ceb760a9e801ebeea6a42c3acbb9fd9  web/index.html
cf7b353d24eea04b7e4f36b2261493b70e691a7a02f085326c035c6707cf45b9  locales/en.json
350a2e0153db9f6479d74af30dd2073fff4839457055528720e34d09dd51365e  web/i18n-en.js
b9df258912218d833fa2c1549d5d949d539dc1ff24c8223976092a676ccfaa61  tests/graph_draft_ui.cjs
98af439d69ed371e87caeade5ccf3f6ec2ddaae2c4b3969205b7099f9a40a68d  evidence_projection.go
832b5b70bebff95b56c90505f5b95b43fee8792c4a1adbe08bbd4cee8c5504fe  evidence_contract_test.go
0bd4e7ceea70605f065875c74bd226ba54a452a16c020df0c5125773af69ebd0  mission_status.go
9dbb8ca65083e5d86f8f70a6481615ebe39b7bdc0a4cfb33cabb715eaab519c1  runtime_health_test.go
d006617570a7fdd47a333b507b03fce91156955eb27cd073e8a5c7c0ef031db5  GUIDE-UTILISATEUR.md
96accb3a6f26478c3b31e4e6fa162b4ef4551c1e2b1e25b14c00ddb03ce23488  docs/en/USER-GUIDE.md
69ad09758b3cf55e406cc30cba1a9f7e723cd7c9a060cb21d743da945353c7b3  docs/plans/graphe-automatisation-20261005/execution/b/verify.py
```

## Module complet
```javascript
'use strict';
const tr_web_graph_draft_js = source => globalThis.SwarmI18n?.t(source) ?? source;

// The browser owns interaction history. Nonempty proposals are saved through
// the engine; returning to an empty history does not apply or delete a proposal.
const GraphDraft = {
 editing:false,source:'',target:'',kind:'add_dependency',draft:null,preview:null,
 history:[[]],cursor:0,pending:false,conflict:false,workID:'',returnFocus:null,
 clone(value){return value.map(item=>({...item}))},
 current(){return this.clone(this.history[this.cursor]||[])},
 mount(){
  if($('graph-draft-panel'))return;
  const panel=node('section',undefined,'graph-draft-panel');panel.id='graph-draft-panel';panel.hidden=true;
  panel.setAttribute('aria-labelledby','graph-draft-title');
  const heading=node('div',undefined,'graph-draft-heading');
  heading.append(node('h4',tr_web_graph_draft_js('Éditer les dépendances')));
  heading.firstChild.id='graph-draft-title';
  const close=this.button('graph-draft-close',tr_web_graph_draft_js('Quitter l’édition'),()=>this.close());heading.append(close);
  const help=node('p',tr_web_graph_draft_js('Choisissez une opération, puis le prérequis et la tâche dépendante dans le graphe. Entrée et Espace offrent le même parcours que la souris.'),'pilot-muted');
  const fields=node('div',undefined,'graph-draft-fields');
  fields.append(this.select('graph-draft-kind',tr_web_graph_draft_js('Opération'),[['add_dependency',tr_web_graph_draft_js('Ajouter une dépendance')],['remove_dependency',tr_web_graph_draft_js('Retirer une dépendance')]],value=>{this.kind=value;this.preview=null;this.render()}),
   this.select('graph-draft-source',tr_web_graph_draft_js('Prérequis'),[],value=>{this.source=value;this.preview=null;this.render()}),
   this.select('graph-draft-target',tr_web_graph_draft_js('Tâche dépendante'),[],value=>{this.target=value;this.preview=null;this.render()}));
  const controls=node('div',undefined,'graph-draft-controls');
  controls.append(this.button('graph-draft-stage',tr_web_graph_draft_js('Ajouter au brouillon'),()=>this.stage()),this.button('graph-draft-undo',tr_web_graph_draft_js('Annuler'),()=>this.move(-1)),this.button('graph-draft-redo',tr_web_graph_draft_js('Rétablir'),()=>this.move(1)),this.button('graph-draft-preview',tr_web_graph_draft_js('Prévisualiser'),()=>this.doPreview(),'primary'),this.button('graph-draft-apply',tr_web_graph_draft_js('Appliquer explicitement'),()=>this.apply(),'primary'));
  const status=node('p','', 'graph-draft-status');status.id='graph-draft-status';status.setAttribute('role','status');status.setAttribute('aria-live','polite');
  const error=node('div','', 'notice alert graph-draft-error');error.id='graph-draft-error';error.hidden=true;error.setAttribute('role','alert');error.tabIndex=-1;
  const recover=this.button('graph-draft-recover',tr_web_graph_draft_js('Recharger et recréer le brouillon'),()=>this.recover());error.append(node('span'),recover);
  const list=node('ol');list.id='graph-draft-operations';list.setAttribute('aria-label',tr_web_graph_draft_js('Modifications proposées'));
  panel.append(heading,help,fields,controls,status,error,list);
  $('pilot-canvas').before(panel);
  const start=this.button('pilot-edit-graph',tr_web_graph_draft_js('Éditer les dépendances'),()=>this.open());$('pilot-actions').append(start);
 },
 button(id,text,fn,cls=''){const value=node('button',text,cls);value.type='button';value.id=id;value.addEventListener('click',e=>{if(e.isTrusted)Promise.resolve(fn()).catch(()=>{})});return value},
 select(id,title,options,change){const label=node('label',title),select=node('select');label.htmlFor=id;select.id=id;selectOptions(select,options,'');select.addEventListener('change',e=>{if(e.isTrusted)change(e.target.value)});label.append(select);return label},
 open(){
  this.returnFocus=document.activeElement;this.editing=true;this.source='';this.target='';this.preview=null;this.conflict=false;
  if(this.workID!==work){this.workID=work;this.draft=null;this.history=[[]];this.cursor=0}
  $('graph-draft-panel').hidden=false;this.render();$('graph-draft-kind').focus();
 },
 close(){this.editing=false;$('graph-draft-panel').hidden=true;this.render();this.returnFocus?.focus()},
 choose(id){
  if(!this.source||this.source===id&&this.target){this.source=id;this.target=''}else if(id!==this.source)this.target=id;
  this.preview=null;this.render();
  if(this.source&&this.target)$('graph-draft-stage').focus();
 },
 async stage(){
  if(!this.source||!this.target||this.source===this.target)return this.fail(tr_web_graph_draft_js('Choisissez deux tâches différentes.'));
  const operation={kind:this.kind,prerequisite:this.source,dependent:this.target},operations=this.current();
  if(operations.some(item=>item.kind===operation.kind&&item.prerequisite===operation.prerequisite&&item.dependent===operation.dependent))return this.fail(tr_web_graph_draft_js('Cette modification est déjà dans le brouillon.'));
  await this.commit([...operations,operation]);this.source='';this.target='';this.render();
 },
 async move(delta){
  const next=this.cursor+delta;if(next<0||next>=this.history.length)return;
  const operations=this.clone(this.history[next]);this.preview=null;
  if(operations.length)await this.persist(operations);
  else{this.draft=null;this.conflict=false;this.clearError()}
  this.cursor=next;this.render();
 },
 async commit(operations){this.history=this.history.slice(0,this.cursor+1);this.history.push(this.clone(operations));this.cursor++;await this.persist(operations)},
 async persist(operations){
  this.busy(true);try{this.draft=await api('/api/v1/graph-drafts',{schema_version:1,work_id:work,draft_id:this.draft?.draft_id||undefined,expected_revision:snapshot.work.revision,expected_draft_revision:this.draft?.revision||undefined,operations});this.preview=null;this.conflict=false;this.clearError()}
  catch(error){this.handle(error);throw error}finally{this.busy(false);this.render()}
 },
 async doPreview(){
  if(!this.draft||!this.current().length)return this.fail(tr_web_graph_draft_js('Ajoutez au moins une modification au brouillon.'));
  this.busy(true);try{this.preview=await api('/api/v1/graph-drafts/preview',{schema_version:1,work_id:work,draft_id:this.draft.draft_id,expected_revision:snapshot.work.revision});this.conflict=false;this.clearError();this.render();$('graph-draft-apply').focus()}
  catch(error){this.handle(error)}finally{this.busy(false);this.render()}
 },
 async apply(){
  if(!this.preview)return this.fail(tr_web_graph_draft_js('Prévisualisez le brouillon avant de l’appliquer.'));
  this.busy(true);try{await api('/api/v1/graph-drafts/apply',{schema_version:1,work_id:work,draft_id:this.preview.draft_id,event_id:crypto.randomUUID(),expected_revision:snapshot.work.revision,preview_token:this.preview.preview_token,content_digest:this.preview.content_digest});this.draft=null;this.preview=null;this.history=[[]];this.cursor=0;this.source='';this.target='';this.clearError();await refresh(true);this.render();$('graph-draft-status').textContent=tr_web_graph_draft_js('Révision appliquée. Aucun agent n’a été lancé.')}
  catch(error){this.handle(error)}finally{this.busy(false);this.render()}
 },
 async recover(){
  const operations=this.current();this.busy(true);try{await refresh(true);this.draft=null;this.preview=null;this.conflict=false;await this.persist(operations);this.clearError();$('graph-draft-status').textContent=tr_web_graph_draft_js('Brouillon recréé sur la révision courante. Prévisualisez-le à nouveau.')}
  catch(error){this.handle(error)}finally{this.busy(false);this.render()}
 },
 handle(error){this.conflict=['revision_conflict','draft_conflict','preview_stale','active_scope_conflict'].includes(error.code);this.fail((error.message||tr_web_graph_draft_js('Modification refusée.'))+(this.conflict?tr_web_graph_draft_js(' Vos modifications sont conservées ; rechargez avant de réessayer.') : ''))},
 fail(message){const box=$('graph-draft-error');box.hidden=false;box.firstChild.textContent=message;$('graph-draft-recover').hidden=!this.conflict;box.focus()},
 clearError(){const box=$('graph-draft-error');box.hidden=true;box.firstChild.textContent=''},
 busy(value){this.pending=value;for(const element of $('graph-draft-panel')?.querySelectorAll('button,select')||[])element.disabled=value},
 render(){
  if(!$('graph-draft-panel')||!snapshot)return;
  if(this.workID&&this.workID!==work){this.editing=false;$('graph-draft-panel').hidden=true;this.workID='';this.draft=null;this.history=[[]];this.cursor=0}
  const options=[['',tr_web_graph_draft_js('Choisir une tâche')],...snapshot.work.tasks.map(task=>[task.id,task.id+' — '+task.title])];
  const optionsKey=JSON.stringify(options);
  for(const [id,value]of [['graph-draft-source',this.source],['graph-draft-target',this.target]]){const select=$(id);if(select.dataset.optionsKey!==optionsKey){selectOptions(select,options,value);select.dataset.optionsKey=optionsKey}else if(select.value!==value)select.value=value}
  if($('graph-draft-kind').value!==this.kind)$('graph-draft-kind').value=this.kind;
  const operations=this.current(),list=$('graph-draft-operations'),operationsKey=JSON.stringify([operations,globalThis.SwarmI18n?.locale]);
  if(this.operationsKey!==operationsKey){this.operationsKey=operationsKey;list.replaceChildren(...operations.map(item=>node('li',(item.kind==='add_dependency'?tr_web_graph_draft_js('Ajouter'):tr_web_graph_draft_js('Retirer'))+' '+item.prerequisite+' → '+item.dependent)));
  if(!operations.length)list.append(node('li',tr_web_graph_draft_js('Aucune modification préparée.')));}
  $('graph-draft-undo').disabled=this.pending||this.cursor===0;$('graph-draft-redo').disabled=this.pending||this.cursor===this.history.length-1;
  $('graph-draft-stage').disabled=this.pending||!this.source||!this.target;$('graph-draft-preview').disabled=this.pending||!this.draft||!operations.length;$('graph-draft-apply').disabled=this.pending||!this.preview;
  let status;
  if(this.pending)status=tr_web_graph_draft_js('Enregistrement en cours…');
  else if(this.preview)status=tr_web_graph_draft_js('Aperçu moteur : ') + this.preview.affected_tasks.length+tr_web_graph_draft_js(' tâche(s) affectée(s). Aucun départ implicite. Confirmez pour appliquer.');
  else status=operations.length+tr_web_graph_draft_js(' modification(s) dans le brouillon. La mission reste inchangée.');
  if($('graph-draft-status').textContent!==status)$('graph-draft-status').textContent=status;
  for(const group of document.querySelectorAll('.graph-noeud'))this.decorate(group,group.dataset.task);
 },
 decorate(group,id){for(const [key,value]of [['draftEditing',this.editing],['draftSource',this.editing&&this.source===id],['draftTarget',this.editing&&this.target===id]]){const text=String(value);if(group.dataset[key]!==text)group.dataset[key]=text}},
 minimap(svg){
  const old=$('graph-minimap');if(svg&&svg===this.minimapSource&&old?.isConnected)return;old?.remove();if(!svg||!$('pilot-canvas'))return;this.minimapSource=svg;
  const map=svgNode('svg',{id:'graph-minimap',class:'graph-minimap',viewBox:svg.getAttribute('viewBox'),role:'img','aria-label':tr_web_graph_draft_js('Mini-carte du graphe')});
  for(const edge of svg.querySelectorAll('.graph-arete,.graph-organisation-link'))map.append(svgNode('polyline',{points:edge.getAttribute('points'),class:'graph-minimap-edge'}));
  for(const item of svg.querySelectorAll('.graph-noeud .graph-cadre,.graph-responsibility rect'))map.append(svgNode('rect',{x:item.getAttribute('x'),y:item.getAttribute('y'),width:item.getAttribute('width'),height:item.getAttribute('height'),rx:8,class:'graph-minimap-node'}));
  $('pilot-canvas').append(map);
 }
};
globalThis.GraphDraft=GraphDraft;

```

## Recette complète
```javascript
'use strict';
// B03 product journey: real candidate binary, isolated roots, HTTP service and
// Chromium. It never opens the store directly and never starts a provider.
const assert=require('node:assert/strict');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto');
const {spawn,execFileSync}=require('node:child_process');
const puppeteer=require(process.env.PUPPETEER_MODULE||'puppeteer');
const binary=path.resolve(process.argv[2]),outDir=path.resolve(process.argv[3]);
if(!process.argv[2]||!process.argv[3])throw Error('usage: node tests/graph_draft_ui.cjs BINAIRE DOSSIER_SORTIE');
fs.mkdirSync(outDir,{recursive:true});
const temporary=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-b03-')),uuid=()=>crypto.randomUUID().replaceAll('-','');
const cli=(root,args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',maxBuffer:16*1024*1024,input:input?JSON.stringify(input):undefined}));
const mutate=(root,args,revision,fields)=>cli(root,args,{schema_version:1,event_id:uuid(),expected_revision:revision,...fields}).work;
function fixture(root,count=6){
 fs.mkdirSync(root,{recursive:true});cli(root,['init']);let work=mutate(root,['work','create'],0,{title:'Graphe B03',objective:'Éditer les dépendances dans le produit',scope:'recette isolée',criteria:['édition vérifiée']});
 for(let index=0;index<count;index++){const id='t'+String(index).padStart(3,'0'),depends=index?[index<6?'t'+String(index-1).padStart(3,'0'):'t000']:[];work=mutate(root,['task','add',work.id],work.revision,{id,title:'Tâche '+index,deliverable:'docs/'+id+'.md',criteria:['effet observable'],owner:index%3===0?'planner':'worker',next:'préparer',depends})}
 return work;
}
function start(root){
 const child=spawn(binary,['--root',root,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
 return new Promise((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(Error('serveur non démarré: '+output)),15000);const read=data=>{output+=data;const match=output.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(match){clearTimeout(timer);resolve({child,url:match[0]})}};child.stdout.on('data',read);child.stderr.on('data',data=>output+=data);child.on('exit',code=>reject(Error('serveur arrêté '+code+': '+output)))})
}
const waitForRevision=(page,revision)=>page.waitForFunction(value=>snapshot?.work?.revision>value,{},revision);
const p95=values=>values.slice().sort((a,b)=>a-b)[Math.ceil(values.length*.95)-1];
async function choose(page,selector,value){const index=await page.$eval(selector,(element,wanted)=>[...element.options].findIndex(option=>option.value===wanted),value);assert.ok(index>=0,`option ${value} absente`);await page.focus(selector);await page.keyboard.press('Home');for(let step=0;step<index;step++)await page.keyboard.press('ArrowDown');await page.keyboard.press('Enter')}
async function openProduct(browser,root,work,lang,theme,diagnostics){
 const server=await start(root),page=await browser.newPage();page.setDefaultTimeout(25000);
 await page.setViewport({width:1440,height:1000});
 page.on('pageerror',error=>diagnostics.console_errors.push(String(error)));
 page.on('console',message=>{
  if(message.type()!=='error')return;
  const text=message.text(),status=text.match(/^Failed to load resource: the server responded with a status of (\d+) /),url=message.location().url;
  if(status&&url&&diagnostics.expected_http.has(status[1]+':'+new URL(url).pathname)){(diagnostics.expected_console_errors??=[]).push(text);return}
  diagnostics.console_errors.push(text);
 });
 diagnostics.inflightRequests=new Set();diagnostics.reloadAbortRequests=new Set();diagnostics.expected_network_aborts=[];
 page.on('request',request=>diagnostics.inflightRequests.add(request));
 page.on('requestfinished',request=>diagnostics.inflightRequests.delete(request));
 page.on('requestfailed',request=>{
  diagnostics.inflightRequests.delete(request);
  if(request.failure()?.errorText==='net::ERR_ABORTED'&&diagnostics.reloadAbortRequests.has(request)){diagnostics.expected_network_aborts.push(new URL(request.url()).pathname);return}
  diagnostics.network_errors.push('failed '+new URL(request.url()).pathname+' '+request.failure()?.errorText);
 });
 page.on('response',response=>{if(response.status()>=400&&!diagnostics.expected_http.has(response.status()+':'+new URL(response.url()).pathname))diagnostics.network_errors.push(response.status()+' '+response.url())});
 const target=new URL(server.url);target.searchParams.set('work',work.id);target.searchParams.set('lang',lang);await page.goto(target.href,{waitUntil:'domcontentloaded'});await page.waitForFunction(id=>snapshot?.work?.id===id&&document.querySelectorAll('.graph-noeud').length>0,{},work.id);
 if(await page.$eval('html',element=>element.dataset.theme)!==theme)await page.click('#theme');assert.equal(await page.$eval('html',element=>element.dataset.theme),theme);
 return {server,page};
}
async function selectNode(page,id){
 const selector=`.graph-noeud[data-task="${id}"]`;
 await page.$eval(selector,element=>element.focus());
 assert.equal(await page.$eval(selector,element=>document.activeElement===element),true,'SVG node did not receive keyboard focus');
 await page.keyboard.press('Enter');
}
async function variant(browser,base,work,name,lang,theme){
 const root=path.join(temporary,'variant-'+name);fs.cpSync(base,root,{recursive:true});const diagnostics={console_errors:[],network_errors:[],expected_http:new Set()};let server,page;
 try{
  ({server,page}=await openProduct(browser,root,work,lang,theme,diagnostics));
  const before=cli(root,['work','show',work.id]).work,agentsBefore=cli(root,['agent','list',work.id]).agents.length;
  assert.ok(await page.$$eval('.graph-arete',nodes=>nodes.length)>=5,'flèches initiales absentes');
  await choose(page,'#pilot-orientation','TB');await page.waitForFunction(()=>document.querySelector('.graph-svg')?.dataset.height);assert.ok(await page.$$eval('.graph-arete',nodes=>nodes.length)>=5,'flèches perdues après orientation');
  await page.click('#pilot-collapse');await page.waitForFunction(()=>document.querySelector('#pilot-status').textContent.includes('masqué')||document.querySelector('#pilot-status').textContent.includes('hidden'));assert.match(await page.$eval('#pilot-status',node=>node.textContent),/masqué|hidden/);await page.click('#pilot-expand');
  assert.ok(await page.$('#graph-minimap'),'mini-carte absente');
  await page.click('#pilot-zoom-in');await page.click('#pilot-zoom-in');const canvasBefore=await page.$eval('#pilot-canvas',node=>{node.scrollLeft=35;node.scrollTop=45;node.dispatchEvent(new Event('scroll'));return {x:node.scrollLeft,y:node.scrollTop}});
  await page.focus('#pilot-edit-graph');await page.keyboard.press('Enter');await page.waitForSelector('#graph-draft-panel:not([hidden])');
  await selectNode(page,'t001');await selectNode(page,'t003');assert.equal(await page.$eval('#graph-draft-stage',node=>node.disabled),false);await page.keyboard.press('Enter');await page.waitForFunction(()=>document.querySelectorAll('#graph-draft-operations li').length===1&&!document.querySelector('#graph-draft-preview').disabled);
  await page.click('#graph-draft-undo');await page.waitForFunction(()=>document.querySelector('#graph-draft-status').textContent.startsWith('0'));assert.equal(await page.$eval('#graph-draft-error',node=>node.hidden),true,'undo empty proposal failed');assert.equal(await page.$eval('#graph-draft-apply',node=>node.disabled),true,'undo retained an applicable preview');assert.equal(cli(root,['work','show',work.id]).work.revision,before.revision,'undo changed the applied plan');await page.click('#graph-draft-redo');await page.waitForFunction(()=>!document.querySelector('#graph-draft-preview').disabled);
  await page.click('#graph-draft-preview');await page.waitForFunction(()=>!document.querySelector('#graph-draft-apply').disabled);assert.match(await page.$eval('#graph-draft-status',node=>node.textContent),/Aucun départ implicite|No implicit start/);
  const firstViewport=await page.$eval('#pilot-canvas',node=>({x:node.scrollLeft,y:node.scrollTop}));
  const revision=before.revision;await page.click('#graph-draft-apply');await waitForRevision(page,revision);await page.waitForFunction(()=>[...document.querySelectorAll('.graph-arete')].some(edge=>edge.dataset.from==='t001'&&edge.dataset.to==='t003'));
  const firstAfter=await page.$eval('#pilot-canvas',node=>({x:node.scrollLeft,y:node.scrollTop}));assert.ok(Math.abs(firstAfter.x-firstViewport.x)<=2&&Math.abs(firstAfter.y-firstViewport.y)<=2,'apply moved the graph viewport');
  const applied=cli(root,['work','show',work.id]).work;assert.ok(applied.tasks.find(task=>task.id==='t003').depends.includes('t001'));assert.equal(cli(root,['agent','list',work.id]).agents.length,agentsBefore,'départ implicite');
  // Create a stale preview with a public concurrent CLI mutation, then recover
  // without discarding the browser operations.
  await page.click('.graph-noeud[data-task="t000"] .graph-titre');await page.click('.graph-noeud[data-task="t004"] .graph-titre');assert.equal(await page.$eval('#graph-draft-stage',node=>node.disabled),false,'mouse selection did not choose two tasks');await page.click('#graph-draft-stage');await page.waitForFunction(()=>!document.querySelector('#graph-draft-preview').disabled);
  const current=cli(root,['work','show',work.id]).work;mutate(root,['task','add',work.id],current.revision,{id:'concurrent',title:'Ajout concurrent',deliverable:'docs/concurrent.md',criteria:['conservé'],owner:'worker',next:'attendre'});
  diagnostics.expected_http.add('409:/api/v1/graph-drafts/preview');await page.click('#graph-draft-preview');await page.waitForSelector('#graph-draft-error:not([hidden])');assert.match(await page.$eval('#graph-draft-error',node=>node.textContent),/conserv|preserv/);assert.equal(await page.$$eval('#graph-draft-operations li',nodes=>nodes.length),1);
  await page.click('#graph-draft-recover');await page.waitForFunction(()=>!document.querySelector('#graph-draft-preview').disabled&&document.querySelector('#graph-draft-error').hidden);await page.click('#graph-draft-preview');await page.waitForFunction(()=>!document.querySelector('#graph-draft-apply').disabled);
  const recoveredViewport=await page.$eval('#pilot-canvas',node=>({x:node.scrollLeft,y:node.scrollTop}));
  const recoveredRevision=cli(root,['work','show',work.id]).work.revision;await page.click('#graph-draft-apply');await waitForRevision(page,recoveredRevision);
  const after=cli(root,['work','show',work.id]).work;assert.ok(after.tasks.find(task=>task.id==='t004').depends.includes('t000'));assert.equal(cli(root,['agent','list',work.id]).agents.length,agentsBefore);
  const canvasAfter=await page.$eval('#pilot-canvas',node=>({x:node.scrollLeft,y:node.scrollTop}));assert.ok(Math.abs(canvasAfter.x-recoveredViewport.x)<=2&&Math.abs(canvasAfter.y-recoveredViewport.y)<=2,`viewport déplacé ${JSON.stringify({recoveredViewport,canvasAfter})}`);
  const screenshot=name+'.png';await page.screenshot({path:path.join(outDir,screenshot),fullPage:true});
  await page.click('#graph-draft-close');assert.equal(await page.evaluate(()=>document.activeElement.id),'pilot-edit-graph','focus non rendu');
  await page.screenshot({path:path.join(outDir,name+'-closed.png'),fullPage:true});
  await page.click('#mode');await page.click('#tabs [data-view=tasks]');
  await page.waitForFunction(count=>document.querySelectorAll('#tasks-body tr').length===count,{},after.tasks.length);
  for(const task of after.tasks)assert.equal(await page.$eval('#tasks-body tr[data-task="'+task.id+'"] td:nth-child(4)',element=>element.textContent),(task.depends||[]).join(', ')||'Aucune','task table did not use current snapshot');
  await page.click('#tabs [data-view=conduite]');assert.deepEqual(diagnostics.console_errors,[]);assert.deepEqual(diagnostics.network_errors,[]);
  return {variant:name,assertions:{arrows:true,keyboard_connect:true,mouse_connect:true,preview_apply:true,no_implicit_launch:true,conflict:true,undo_redo:true,fold_counts:true,viewport:true,focus_restore:true,task_table_fresh:true},console_errors:diagnostics.console_errors,network_errors:diagnostics.network_errors,expected_console_errors:diagnostics.expected_console_errors||[],screenshot,viewport_observations:[{before:firstViewport,after:firstAfter},{before:recoveredViewport,after:canvasAfter}],revision_before:before.revision,revision_after:after.revision};
 }finally{await page?.close();server?.child.kill('SIGTERM')}
}
async function loadMeasurement(browser,cards){
 const root=path.join(temporary,'load-'+cards),work=fixture(root,cards),diagnostics={console_errors:[],network_errors:[],expected_http:new Set()},initial=[],keyboard=[];let server,page;
 try{
  ({server,page}=await openProduct(browser,root,work,'fr','etat',diagnostics));
  const sampleCount=5;
  for(let sample=0;sample<sampleCount;sample++){
   diagnostics.reloadAbortRequests=new Set(diagnostics.inflightRequests);const startAt=Date.now();await page.reload({waitUntil:'domcontentloaded'});await page.waitForFunction(count=>document.querySelectorAll('.graph-noeud').length===count,{},cards);initial.push(Date.now()-startAt);
   await page.focus('#pilot-edit-graph');await page.keyboard.press('Enter');await page.waitForSelector('#graph-draft-panel:not([hidden])');
   await page.$eval('.graph-noeud[data-task="t000"]',async element=>{
    element.focus();
    await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
    window.b03KeyboardMeasurement=new Promise(resolve=>element.addEventListener('keydown',event=>{
     if(event.key!=='Enter'||!event.isTrusted)return;
     const start=event.timeStamp;
     requestAnimationFrame(()=>requestAnimationFrame(()=>resolve({milliseconds:performance.now()-start,selected:element.dataset.draftSource==='true',focused:document.activeElement===element})));
    },{capture:true,once:true}));
   });
   await page.keyboard.press('Enter');
   const observation=await page.evaluate(()=>window.b03KeyboardMeasurement);
   assert.equal(observation.selected,true,'keyboard input did not select the graph node');assert.equal(observation.focused,true,'keyboard focus lost');keyboard.push(observation.milliseconds);await page.click('#graph-draft-close');assert.equal(await page.evaluate(()=>document.activeElement.id),'pilot-edit-graph','load journey lost focus');
  }
  assert.deepEqual(diagnostics.console_errors,[]);assert.deepEqual(diagnostics.network_errors,[]);const initialP95=p95(initial),keyboardP95=p95(keyboard);fs.writeFileSync(path.join(outDir,'load-'+cards+'-measurements.json'),JSON.stringify({cards,initial,keyboard,keyboard_method:'trusted keydown event timestamp through two animation frames, browser clock; focus setup and automation round trips excluded',initialP95,keyboardP95},null,2));assert.ok(initialP95<=(cards===500?2000:1000),`rendu p95 ${cards}: ${initialP95} ms`);assert.ok(keyboardP95<=100,`clavier p95 ${cards}: ${keyboardP95} ms`);
  return {cards,samples:sampleCount,initial_p95_ms:initialP95,keyboard_p95_ms:keyboardP95,initial_samples_ms:initial,keyboard_samples_ms:keyboard,keyboard_method:'trusted keydown timestamp to second animation frame',expected_reload_aborts:diagnostics.expected_network_aborts,arrows:await page.$$eval('.graph-arete',nodes=>nodes.length>0),focus_recovered:true};
 }finally{await page?.close();server?.child.kill('SIGTERM')}
}
(async()=>{
 const base=path.join(temporary,'base'),work=fixture(base),browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',userDataDir:path.join(temporary,'chrome'),args:['--no-sandbox','--disable-dev-shm-usage','--disable-crash-reporter','--disable-breakpad']});
 try{
  if(process.env.SWARM_B03_LOAD_ONLY){const cards=Number(process.env.SWARM_B03_LOAD_ONLY);assert.ok([50,200,500].includes(cards));const row=await loadMeasurement(browser,cards);fs.writeFileSync(path.join(outDir,'diagnostic-load-'+cards+'.json'),JSON.stringify(row,null,2));console.log(JSON.stringify({diagnostic_only:true,row}));return}
  const variants=[];for(const [name,lang,theme]of[['fr-sombre','fr','sombre'],['fr-etat','fr','etat'],['en-sombre','en','sombre'],['en-etat','en','etat']])variants.push(await variant(browser,base,work,name,lang,theme));
  const load_measurements=[];for(const cards of [50,200,500])load_measurements.push(await loadMeasurement(browser,cards));
  const results={surface:'swarm-product',environment:{host:`${os.platform()} ${os.release()} ${os.arch()} ${os.cpus()[0]?.model||'unknown'}`,browser:await browser.version(),viewport:'1440x1000',binary},variants,load_measurements};fs.writeFileSync(path.join(outDir,'results.json'),JSON.stringify(results,null,2));console.log(JSON.stringify(results,null,2));
 }finally{await browser.close()}
})().catch(error=>{fs.writeFileSync(path.join(outDir,'failure.json'),JSON.stringify({error:error.stack,temporary_root:temporary},null,2));console.error(error);process.exitCode=1});

```

## Raccords complets depuis la base Git
```diff
diff --git a/web/graph.css b/web/graph.css
index d6b45f5..9be3175 100644
--- a/web/graph.css
+++ b/web/graph.css
@@ -67 +67,15 @@
 .graph-noeud[role=button]:focus-visible .graph-cadre{stroke:var(--wattson-lien);stroke-width:4;stroke-dasharray:4 2}
+.graph-noeud[data-draft-editing=true]{cursor:crosshair}
+.graph-noeud[data-draft-source=true] .graph-cadre{stroke:var(--wattson-info-encre);stroke-width:5;stroke-dasharray:8 3}
+.graph-noeud[data-draft-target=true] .graph-cadre{stroke:var(--wattson-accent);stroke-width:5}
+.graph-minimap{position:sticky;left:12px;bottom:12px;width:180px;height:110px;padding:4px;background:var(--wattson-carte);border:1px solid var(--wattson-ligne);border-radius:8px;box-shadow:0 2px 8px color-mix(in srgb,var(--wattson-titre) 16%,transparent)}
+.graph-minimap-edge{fill:none;stroke:var(--wattson-libelle);stroke-width:8;vector-effect:non-scaling-stroke}
+.graph-minimap-node{fill:var(--wattson-info-fond);stroke:var(--wattson-info-encre);stroke-width:5;vector-effect:non-scaling-stroke}
+.graph-draft-panel{padding:16px;border:1px solid var(--wattson-info-encre);border-top:0;background:var(--wattson-info-fond);color:var(--wattson-info-encre)}
+.graph-draft-panel[hidden]{display:none}
+.graph-draft-heading,.graph-draft-controls,.graph-draft-fields{display:flex;flex-wrap:wrap;gap:10px;align-items:end}
+.graph-draft-heading{justify-content:space-between;align-items:center}.graph-draft-heading h4{margin:0;color:inherit}
+.graph-draft-fields label{display:flex;flex:1 1 220px;flex-direction:column;gap:5px;font-size:12px;font-weight:600}
+.graph-draft-fields select{min-height:40px;background:var(--wattson-champ);color:var(--wattson-texte);border:1px solid var(--wattson-ligne)}
+.graph-draft-controls{margin-top:12px}.graph-draft-status{font-weight:600}.graph-draft-error{display:flex;gap:12px;align-items:center}.graph-draft-error[hidden]{display:none}
+.graph-draft-panel ol{margin-bottom:0}.graph-draft-panel button:focus-visible,.graph-draft-panel select:focus-visible{outline:2px solid var(--wattson-lien);outline-offset:2px}
diff --git a/web/graph.js b/web/graph.js
index 78b155c..f003bce 100644
--- a/web/graph.js
+++ b/web/graph.js
@@ -79,2 +79,3 @@ function drawPilotGraph(){
  const canvas=$('pilot-canvas'),state=Pilot.state,tasks=snapshot.work.tasks;
+ const tasksByID=new Map(tasks.map(task=>[task.id,task])),selectedTask=Pilot.selectedTask();
  const shown=PilotGraph.visible(tasks,state.collapsed);
@@ -86,2 +87,3 @@ function drawPilotGraph(){
  const height=state.detail==='detailed'?324:210,width=310;
+ let restoredViewport=null,restoredFocus=null;
  if(shape!==Pilot.graphKey){
@@ -90,3 +92,3 @@ function drawPilotGraph(){
   const x=canvas.scrollLeft,y=canvas.scrollTop;
-  const g=new dagre.graphlib.Graph();g.setGraph({rankdir:state.orientation,nodesep:28,ranksep:70,marginx:20,marginy:20});g.setDefaultEdgeLabel(()=>({}));
+  const g=new dagre.graphlib.Graph();g.setGraph({rankdir:state.orientation,ranker:'tight-tree',nodesep:28,ranksep:70,marginx:20,marginy:20});g.setDefaultEdgeLabel(()=>({}));
   for(const t of kept)g.setNode(t.id,{width,height});
@@ -129,3 +131,3 @@ function drawPilotGraph(){
    group.append(svgNode('rect',{x:left+10,y:top+height-88,width:width-20,height:20,rx:4,class:'graph-profile-surface'}),svgNode('text',{x:left+14,y:top+height-73,class:'graph-profile-text','data-profile-line':'true'}));
-   const open=()=>Pilot.inspect('task',t.id);
+   const open=()=>GraphDraft.editing?GraphDraft.choose(t.id):Pilot.inspect('task',t.id);
    group.addEventListener('click',e=>{if(e.isTrusted)open()});
@@ -133,2 +135,3 @@ function drawPilotGraph(){
    svg.append(group);
+   GraphDraft.decorate(group,t.id);
    const go=svgNode('g',{class:'graph-go',tabindex:0,role:'button'});go.dataset.task=t.id;
@@ -150,7 +153,8 @@ function drawPilotGraph(){
   if(!kept.length)canvas.append(node('p',tasks.length?tr_web_graph_js('Aucun nœud pour ces filtres. Affichez tout le travail.'):tr_web_graph_js('Aucune tâche : le graphe apparaîtra dès qu’un plan existe.')));
-  canvas.scrollLeft=x||state.x;canvas.scrollTop=y||state.y;
-  if(focus)[...canvas.querySelectorAll(goFocus?'.graph-go':foldFocus?'.graph-fold':'.graph-noeud')].find(n=>n.dataset.task===focus)?.focus();
+  restoredViewport={x:x||state.x,y:y||state.y};
+  if(focus)restoredFocus=[...canvas.querySelectorAll(goFocus?'.graph-go':foldFocus?'.graph-fold':'.graph-noeud')].find(n=>n.dataset.task===focus);
  }
  const svg=canvas.querySelector('svg');if(!svg)return;
- scalePilotGraph(svg,state.zoom);
+  scalePilotGraph(svg,state.zoom);
+ GraphDraft.minimap(svg);
  for(const n of organization.nodes){const group=[...svg.querySelectorAll('.graph-responsibility')].find(e=>e.dataset.responsibility===n.id);if(!group)continue;const lines=n.role==='subplanner'?[n.title,n.scopeLabel,n.description,n.detail]:[n.title,n.description,n.detail,tr_web_graph_js('Ouvrir les décisions et avis')];lines.push(globalThis.ProjectProfiles?ProjectProfiles.summary(n.workflow):'');group.setAttribute('aria-label',lines.join('. '));for(const text of group.querySelectorAll('[data-role-line]')){const value=lines[Number(text.dataset.roleLine)];text.textContent=value.length>40?value.slice(0,39)+'…':value}}
@@ -158,3 +162,3 @@ function drawPilotGraph(){
  for(const group of canvas.querySelectorAll('.graph-noeud')){
-  const t=tasks.find(t=>t.id===group.dataset.task),v=snapshot.validation?.tasks[t.id],agent=byTask[t.id]?.[0]?.agent;
+  const t=tasksByID.get(group.dataset.task),v=snapshot.validation?.tasks[t.id],agent=byTask[t.id]?.[0]?.agent;
   const profileText=group.querySelector('[data-profile-line]');if(profileText&&globalThis.ProjectProfiles){const profileAgent=agent||snapshot.agents.filter(x=>x.agent.task_id===t.id).sort((a,b)=>(b.agent.started||'').localeCompare(a.agent.started||''))[0]?.agent;const label=profileAgent?ProjectProfiles.summary(profileAgent.workflow):'▤ '+tr_web_graph_js('Profil non encore transmis');const profile=ProjectProfiles.describe(profileAgent?.workflow);profileText.dataset.profileFamily=profile.family;group.querySelector('.graph-profile-surface').dataset.profileFamily=profile.family;profileText.textContent=label.length>41?label.slice(0,40)+'…':label;profileText.setAttribute('aria-label',label);profileText.replaceChildren(document.createTextNode(profileText.textContent),svgNode('title',{},label));}
@@ -162,3 +166,4 @@ function drawPilotGraph(){
   group.dataset.etat=uncertain?'attention':graphTonalites[v?.state||t.status]||'neutre';
-  group.dataset.selected=String(Pilot.selectedTask()===t.id);
+  group.dataset.selected=String(selectedTask===t.id);
+  GraphDraft.decorate(group,t.id);
   const lines=[t.title,uncertain||labels[v?.state||t.status]||t.status,agent?agent.provider+' · '+(uncertain?(snapshot.pilotage?.health[agent.id]?.stop_requested?tr_web_graph_js('Arrêt demandé'):tr_web_graph_js('Activité non confirmée')):agent.progress?.detail||agent.progress?.action||tr_web_graph_js('Activité non reçue')):t.id];
@@ -179,3 +184,3 @@ function drawPilotGraph(){
  for(const button of canvas.querySelectorAll('.graph-go')){
-  const t=tasks.find(t=>t.id===button.dataset.task),go=Pilot.goState(t),session=PilotGraph.taskSession(snapshot.agents,t.id);
+  const t=tasksByID.get(button.dataset.task),go=Pilot.goState(t),session=PilotGraph.taskSession(snapshot.agents,t.id);
   if(session){button.style.display='';button.dataset.ready='true';button.dataset.taskSession=t.id;button.dataset.sessionLocation='graph';button.dataset.agentSession=session.agent.id;button.setAttribute('aria-label',session.label+' : '+t.title);button.querySelector('text').textContent=session.label;button.querySelector('title').textContent=tr_web_graph_js('Ouvrir la dernière tentative de cette tâche');continue}
@@ -192,4 +197,5 @@ function drawPilotGraph(){
  }
+ const linksByPair=new Map(links.map(e=>[JSON.stringify([e.from_task_id,e.to_task_id]),e]));
  for(const edge of canvas.querySelectorAll('.graph-arete')){
-  const e=links.find(e=>e.from_task_id===edge.dataset.from&&e.to_task_id===edge.dataset.to);
+  const e=linksByPair.get(JSON.stringify([edge.dataset.from,edge.dataset.to]));
   edge.dataset.lien=e?.satisfied_now?'satisfait':'attente';
@@ -197,2 +203,5 @@ function drawPilotGraph(){
  }
+ // Restore position after the SVG writes; fitting owns the initial viewport.
+ if(restoredViewport&&!Pilot.fitPending)canvas.scrollTo(restoredViewport.x,restoredViewport.y);
+ restoredFocus?.focus();
 }
diff --git a/web/pilotage.css b/web/pilotage.css
index dfb38b2..c91f475 100644
--- a/web/pilotage.css
+++ b/web/pilotage.css
@@ -10,3 +10,3 @@
 .pilot-status,.pilot-muted{font-size:13px;color:var(--wattson-texte-discret)}
-.pilot-canvas{overflow:auto;max-width:100%;max-height:65vh;min-height:220px;border:1px solid var(--wattson-ligne);border-radius:12px;background:var(--wattson-carte-appuyee);overscroll-behavior:contain}
+.pilot-canvas{overflow:auto;max-width:100%;max-height:65vh;min-height:220px;border:1px solid var(--wattson-ligne);border-radius:12px;background:var(--wattson-carte-appuyee);overscroll-behavior:contain;contain:layout paint}
 .pilot-canvas:focus-visible{outline:2px solid var(--wattson-lien);outline-offset:3px}
diff --git a/web/pilotage.js b/web/pilotage.js
index 11627f3..280a6a3 100644
--- a/web/pilotage.js
+++ b/web/pilotage.js
@@ -65,2 +65,3 @@ const Pilot = {
   $('graph').replaceChildren(mission,toolbar,actions,status,canvas,list);
+  GraphDraft.mount();
  },
@@ -123,6 +124,8 @@ const Pilot = {
   $('pilot-reveal').disabled=!this.selectedTask();
+  const fitWidth=graph&&this.fitPending?$('pilot-canvas').clientWidth:0;
   if(graph){drawPilotGraph();if(this.fitPending){
    const svg=$('pilot-canvas').querySelector('svg');
-   if(svg){const port=$('pilot-canvas');this.state.zoom=Math.max(.15,Math.min(1,(port.clientWidth-20)/Number(svg.dataset.width),(innerHeight*.65-50)/Number(svg.dataset.height)));scalePilotGraph(svg,this.state.zoom);this.state.x=0;this.state.y=0;port.scrollTo(0,0);this.fitPending=false;this.save()}
+   if(svg){const port=$('pilot-canvas');this.state.zoom=Math.max(.15,Math.min(1,(fitWidth-20)/Number(svg.dataset.width),(innerHeight*.65-50)/Number(svg.dataset.height)));scalePilotGraph(svg,this.state.zoom);this.state.x=0;this.state.y=0;port.scrollTo(0,0);this.fitPending=false;this.save()}
   }if(this.restorePosition){$('pilot-canvas').scrollTo(this.state.x,this.state.y);this.restorePosition=false}}else this.cards();
+  GraphDraft.render();
   const edges=snapshot.pilotage?.edges||[];
@@ -130,3 +133,3 @@ const Pilot = {
   $('pilot-status').classList.toggle('notice',graph&&!edges.length&&snapshot.work.tasks.length>0);$('pilot-status').classList.toggle('info',graph&&!edges.length&&snapshot.work.tasks.length>0);
-  if(graph&&edges.length){const visible=$('pilot-canvas').querySelectorAll('.graph-arete').length;$('pilot-status').append(' '+visible+tr_web_pilotage_js(' dépendances affichées sur ')+edges.length+'.');if(visible<edges.length)$('pilot-status').append(tr_web_pilotage_js(' Des branches sont repliées ou filtrées : utilisez « Toutes les dépendances ».'))}
+  if(graph&&edges.length){const visible=$('pilot-canvas').querySelectorAll('.graph-arete').length,visibleTasks=$('pilot-canvas').querySelectorAll('.graph-noeud').length,hiddenTasks=Math.max(0,snapshot.work.tasks.length-visibleTasks),hiddenLinks=Math.max(0,edges.length-visible);$('pilot-status').append(' '+visible+tr_web_pilotage_js(' dépendances affichées sur ')+edges.length+'.');if(hiddenTasks||hiddenLinks)$('pilot-status').append(' '+hiddenTasks+tr_web_pilotage_js(' tâche(s) et ')+hiddenLinks+tr_web_pilotage_js(' lien(s) masqué(s).'));if(visible<edges.length)$('pilot-status').append(tr_web_pilotage_js(' Des branches sont repliées ou filtrées : utilisez « Toutes les dépendances ».'))}
   if(graph&&snapshot.work.planning)$('pilot-status').append(tr_web_pilotage_js(' Pointillés : responsabilités et remise au vérificateur. Traits pleins : dépendances entre tâches.'));

```

## Ordre complet des scripts
```html
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Swarm — Cockpit</title><script src="/i18n-en.js" defer></script><script src="/i18n.js" defer></script><script src="/status-contract.js" defer></script><link rel="icon" href="/favicon.svg" type="image/svg+xml"><link rel="stylesheet" href="/wattson_themes.css"><link rel="stylesheet" href="/cockpit.css"><link rel="stylesheet" href="/cockpit-ux.css"><link rel="stylesheet" href="/agent-terminal.css"><script src="/lib/preparation-security.js" defer></script><script src="/agent-terminal.js" defer></script><script src="/lifecycle-manager.js" defer></script><script src="/cockpit.js" defer></script><script src="/pricing.js" defer></script><script src="/quotas.js" defer></script><script src="/task-models.js" defer></script><script src="/role-models.js" defer></script><script src="/runtime-health.js" defer></script><script src="/lib/dagre.min.js" defer></script><script src="/pilot-graph.js" defer></script><script src="/pilot-actions.js" defer></script><link rel="stylesheet" href="/mission.css"><script src="/mission-help.js" defer></script><script src="/evidence-contract.js" defer></script><script src="/planning.js" defer></script><script src="/mission.js" defer></script><script src="/mission-insights.js" defer></script><script src="/pilot-batch.js" defer></script><script src="/pilotage.js" defer></script><script src="/pilot-inspector.js" defer></script><script src="/report-summary.js" defer></script><script src="/graph-draft.js" defer></script><script src="/graph.js" defer></script><link rel="stylesheet" href="/pilotage.css"><script src="/activity.js" defer></script><script src="/conduite.js" defer></script><link rel="stylesheet" href="/conduite.css"><link rel="stylesheet" href="/graph.css"><script src="/brainstorm.js" defer></script><script src="/retex.js" defer></script><script src="/cockpit-help.js" defer></script><script src="/plan.js" defer></script><link rel="stylesheet" href="/plan.css"><script src="/assistant.js" defer></script><link rel="stylesheet" href="/assistant.css"><script src="/ai-connections.js" defer></script><script src="/project-profile-settings.js" defer></script><script src="/action-skills.js" defer></script><link rel="stylesheet" href="/project-profile-settings.css"><script src="/providers.js" defer></script><link rel="stylesheet" href="/providers.css"><script src="/admin.js" defer></script><link rel="stylesheet" href="/context-help.css"><script src="/context-help.js" defer></script></head>
```

Les fonctions moteur/tableau et les contrôles de régression sont fournis intégralement dans docs/B03.md. Les quatre captures conservées ont été inspectées pendant cette reprise, sans anomalie visuelle évidente; cette auto-vérification n’est ni une revue indépendante ni un contrôle frais.
