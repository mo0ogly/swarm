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
  if(typeof ProductViews!=='undefined'){Pilot.state.productLevel='all';Pilot.state.productKey='';Pilot.state.view='dependencies';Pilot.changed()}
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
