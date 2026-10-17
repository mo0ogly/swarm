'use strict';
const tr_web_product_views_js=source=>globalThis.SwarmI18n?.t(source)??source;

// One map at a time. Navigation never mutates a mission or starts an agent.
const ProductViews={
 signature:'',page:1,query:'',catalogue:null,
 mount(host){
  const panel=node('section',undefined,'product-views');panel.id='product-views';panel.setAttribute('aria-label',tr_web_product_views_js('Parcours et stories'));
  host.insertBefore(panel,$('pilot-canvas'));
 },
 navigate(level,key=''){
  if(GraphDraft.editing)GraphDraft.close();
  Pilot.state.productParent=level==='story'?(Pilot.state.productLevel==='journey'?Pilot.state.productKey:Pilot.state.productParent):level==='journey'?key:'';
  Pilot.state.productLevel=level;Pilot.state.productKey=key;Pilot.state.search='';Pilot.state.filter='all';Pilot.state.collapsed=[];
  Pilot.fitPending=true;this.page=1;this.signature='';Pilot.changed();
  $('product-heading')?.focus();
 },
 taskIDs(){return this.catalogue?new Set(ProductGraph.selection(this.catalogue,Pilot.state)):null},
 contains(id){return !this.catalogue||this.taskIDs().has(id)},
 organization(tasks){
  const p=snapshot.work.planning;if(!p||!this.catalogue?.journeys.length||Pilot.state.productLevel==='all')return PilotGraph.organization(snapshot.work,tasks,snapshot.paused);
  const ids=new Set(['root',...tasks.map(t=>t.scope_id).filter(Boolean)]),byID=new Map(p.scopes.map(s=>[s.id,s]));
  for(const id of ids){let s=byID.get(id);while(s?.parent&&!ids.has(s.parent)){ids.add(s.parent);s=byID.get(s.parent)}}
  return PilotGraph.organization({...snapshot.work,planning:{...p,scopes:p.scopes.filter(s=>ids.has(s.id))}},tasks,snapshot.paused);
 },
 summary(ids){
  const s=ProductGraph.stats(snapshot,ids);
  let text=s.accepted+'/'+s.total+' '+tr_web_product_views_js('tâches avec preuves acceptées');
  if(s.blocked)text+=' · '+s.blocked+' '+tr_web_product_views_js('bloquées');
  if(s.stale)text+=' · '+s.stale+' '+tr_web_product_views_js('preuves périmées');
  if(s.review)text+=' · '+s.review+' '+tr_web_product_views_js('à examiner');
  return text;
 },
 render(){
  this.catalogue=ProductGraph.catalogue(snapshot.work);const c=this.catalogue,state=Pilot.state;
  const normalized=ProductGraph.normalize(c,state);if(normalized.productLevel!==state.productLevel){Pilot.state=normalized;Pilot.save();this.signature=''}
  const panel=$('product-views');panel.hidden=!c.journeys.length;
  if(!c.journeys.length)return false;
  const current=Pilot.state,graphMode=['overview','journey'].includes(current.productLevel);
  const signature=JSON.stringify([work,snapshot.work.revision,snapshot.validation,snapshot.decisions,current.productLevel,current.productKey,current.productParent,current.search,current.orientation,this.page]);
  if(signature===this.signature)return graphMode;
  this.signature=signature;const focused=document.activeElement?.dataset.productFocus;panel.replaceChildren();
  const nav=node('nav',undefined,'product-breadcrumb');nav.setAttribute('aria-label',tr_web_product_views_js('Navigation du produit'));
  const add=(text,level,key='')=>{const b=Pilot.command(text,()=>this.navigate(level,key));b.dataset.productFocus=level+':'+key;nav.append(b);return b};
  add(tr_web_product_views_js('Application'),'overview');
  const journey=ProductGraph.parent(c,current);
  if(journey)add(journey.title,'journey',journey.key);
  const story=current.productLevel==='story'?c.byStory.get(current.productKey):null;
  if(story){const label=node('span',story.title);label.setAttribute('aria-current','page');nav.append(label)}
  add(tr_web_product_views_js('Tâches communes et non classées'),'common');add(tr_web_product_views_js('Toutes les tâches'),'all');panel.append(nav);
  const title=node('h3',story?story.title:current.productLevel==='journey'?journey.title:current.productLevel==='common'?tr_web_product_views_js('Tâches communes et non classées'):current.productLevel==='all'?tr_web_product_views_js('Toutes les tâches'):tr_web_product_views_js('Parcours utilisateur'));
  title.id='product-heading';title.tabIndex=-1;panel.append(title);
  if(graphMode){
   panel.append(node('p',tr_web_product_views_js('Ouvrez un parcours, puis une story pour afficher son graphe de tâches. La navigation ne modifie pas le travail.')));
   const items=current.productLevel==='journey'?journey.storyKeys.map(k=>c.byStory.get(k)):c.journeys;
   const query=current.search.trim().toLocaleLowerCase();if(query!==this.query){this.query=query;this.page=1}
   const filtered=items.filter(item=>!query||[item.id,item.title,item.goal,item.user,item.value,...item.taskIDs.map(id=>c.tasks.get(id)?.title)].filter(Boolean).join(' ').toLocaleLowerCase().includes(query));
   this.page=Math.max(1,Math.min(this.page,Math.ceil(filtered.length/20)||1));
   const start=(this.page-1)*20,shown=filtered.slice(start,start+20);
   panel.append(node('p',(shown.length?start+1:0)+'–'+(start+shown.length)+'/'+filtered.length+' '+tr_web_product_views_js('éléments affichés'),'product-count'));
   this.map(panel,shown,current.productLevel==='journey'?'story':'journey');
   const list=node('div',undefined,'product-cards');
   for(const item of shown){
    const card=node('article',undefined,'product-card'),level=current.productLevel==='journey'?'story':'journey';
    const open=Pilot.command(item.id+' · '+item.title,()=>this.navigate(level,item.key),'product-open');open.dataset.productFocus=item.key;
    card.append(open,node('p',item.goal||item.value),node('p',item.taskIDs.length?this.summary(item.taskIDs):tr_web_product_views_js('Non planifiée : aucune tâche liée.')));
    if(level==='story'){
     card.append(node('p',tr_web_product_views_js('Utilisateur : ')+item.user));
     const shared=item.taskIDs.filter(id=>(c.membership.get(id)||[]).length>1).length;
     if(shared)card.append(node('p',shared+' '+tr_web_product_views_js('tâches partagées')));
     const external=ProductGraph.boundary(snapshot.work,item.taskIDs);if(external.incoming.length)card.append(node('p',external.incoming.length+' '+tr_web_product_views_js('prérequis extérieurs')));
    }else{
     card.append(node('p',item.storyKeys.length+' '+tr_web_product_views_js('stories')));
     const external=ProductGraph.boundary(snapshot.work,item.taskIDs);if(external.incoming.length)card.append(node('p',external.incoming.length+' '+tr_web_product_views_js('prérequis extérieurs')));
    }
    list.append(card);
   }
   panel.append(list);
   if(this.page>1){const previous=Pilot.command(tr_web_product_views_js('20 éléments précédents'),()=>{this.page--;this.signature='';Pilot.render();$('product-heading').focus()});panel.append(previous)}
   if(start+shown.length<filtered.length){const more=Pilot.command(tr_web_product_views_js('20 éléments suivants'),()=>{this.page++;this.signature='';Pilot.render();$('product-heading').focus()});panel.append(more)}
   if(!filtered.length)panel.append(node('p',tr_web_product_views_js('Aucun parcours ou story pour cette recherche.')));
  }else{
   const ids=ProductGraph.selection(c,current);
   if(story){
    if(ids.length)panel.append(node('p',this.summary(ids)));
    const context=node('details');context.append(node('summary',tr_web_product_views_js('Objectif et critères de la story')),node('p',tr_web_product_views_js('Utilisateur : ')+story.user),node('p',story.value));
    const criteria=node('ul');for(const criterion of story.criteria||[])criteria.append(node('li',criterion));context.append(criteria);
    context.append(node('p',tr_web_product_views_js('Ces compteurs concernent les tâches liées ; ils ne certifient pas la réalisation complète de la story.')));panel.append(context);
    if(!ids.length)panel.append(node('p',tr_web_product_views_js('Non planifiée : aucune tâche liée.'),'notice info'));
    const shared=ids.filter(id=>(c.membership.get(id)||[]).length>1);if(shared.length)panel.append(node('p',shared.length+' '+tr_web_product_views_js('tâches partagées : état unique dans toutes les vues.')));
   }
   this.boundaries(panel,ids);
  }
  const relevant=new Set(ProductGraph.selection(c,current));
  const decisions=(snapshot.decisions||[]).filter(d=>!d.resolved_at&&(!d.task_id||graphMode||relevant.has(d.task_id)));
  if(decisions.length){const details=node('details'),summary=node('summary',decisions.length+' '+tr_web_product_views_js('décisions humaines en attente'));details.append(summary);for(const d of decisions)details.append(Pilot.command(d.title||d.message||d.id,()=>Pilot.inspect('decision',d.id)));panel.append(details)}
  if(focused)[...panel.querySelectorAll('[data-product-focus]')].find(e=>e.dataset.productFocus===focused)?.focus();
  return graphMode;
 },
 boundaries(panel,ids){
  const b=ProductGraph.boundary(snapshot.work,ids),c=this.catalogue;
  for(const [kind,items]of [['incoming',b.incoming],['outgoing',b.outgoing]]){
   if(!items.length)continue;
   const details=node('details',undefined,'product-boundaries');details.open=kind==='incoming';
   details.append(node('summary',items.length+' '+tr_web_product_views_js(kind==='incoming'?'prérequis extérieurs':'tâches dépendantes dans les autres vues')));
   for(const item of items){
    const task=c.tasks.get(item.id),row=node('p');row.append(Pilot.command(task?.title||item.id,()=>Pilot.inspect('task',item.id)));
    const v=snapshot.validation?.tasks?.[item.id];row.append(node('span',' · '+(labels[v?.state||task?.status]||tr_web_product_views_js('Preuves non disponibles'))));
    const start=snapshot.task_actions?.[item.id]?.find(a=>a.kind==='start');if(start&&!start.disponible)row.append(node('span',' · '+(globalThis.SwarmI18n?.engine(start.raison)||start.raison||'')));
    details.append(row);
   }
   panel.append(details);
  }
 },
 map(panel,items,kind){
  if(!items.length)return;
  const ids=new Set(items.map(i=>i.key));
  const links=kind==='story'?items.flatMap(s=>s.dependencyKeys.filter(id=>ids.has(id)).map(id=>({from:id,to:s.key}))):ProductGraph.links(snapshot.work,items);
  const g=new dagre.graphlib.Graph();g.setGraph({rankdir:Pilot.state.orientation,nodesep:22,ranksep:45,marginx:12,marginy:12});g.setDefaultEdgeLabel(()=>({}));
  for(const item of items)g.setNode(item.key,{width:240,height:82});for(const e of links)g.setEdge(e.from,e.to);dagre.layout(g);
  const svg=svgNode('svg',{viewBox:'0 0 '+g.graph().width+' '+g.graph().height,width:g.graph().width,height:g.graph().height,class:'product-map',role:'group','aria-label':tr_web_product_views_js(kind==='story'?'Dépendances déclarées entre stories':'Relations entre parcours issues des dépendances de tâches')});
  for(const e of links){const points=g.edge(e.from,e.to).points;svg.append(svgNode('polyline',{points:points.map(p=>p.x+','+p.y).join(' '),class:'graph-arete'}));const end=points.at(-1),prev=points.at(-2),angle=prev?Math.atan2(end.y-prev.y,end.x-prev.x)*180/Math.PI:0;svg.append(svgNode('path',{d:`M ${end.x-7} ${end.y-5} L ${end.x} ${end.y} L ${end.x-7} ${end.y+5}`,transform:`rotate(${angle} ${end.x} ${end.y})`,class:'graph-arete'}))}
  for(const item of items){
   const p=g.node(item.key),x=p.x-120,y=p.y-41;
   const n=svgNode('g',{role:'button',tabindex:0,'aria-label':item.title+' · '+(item.taskIDs.length?this.summary(item.taskIDs):tr_web_product_views_js('Non planifiée')),class:'product-map-node'});n.dataset.productFocus='map:'+item.key;
   const count=ProductGraph.stats(snapshot,item.taskIDs);
   n.dataset.state=count.blocked?'blocked':count.stale?'stale':'neutral';
   n.append(svgNode('rect',{x,y,width:240,height:82,rx:10}),svgNode('text',{x:x+12,y:y+27},item.title.length>29?item.title.slice(0,28)+'…':item.title),svgNode('text',{x:x+12,y:y+53},item.taskIDs.length?count.accepted+'/'+count.total+' '+tr_web_product_views_js('tâches acceptées'):tr_web_product_views_js('Non planifiée')));
   const open=()=>this.navigate(kind,item.key);n.addEventListener('click',e=>{if(e.isTrusted)open()});n.addEventListener('keydown',e=>{if(e.isTrusted&&['Enter',' '].includes(e.key)){e.preventDefault();open()}});svg.append(n);
  }
  const container=node('div',undefined,'product-map-container');container.append(svg);panel.append(container);
  panel.append(node('p',tr_web_product_views_js(kind==='story'?'Liens : dépendances déclarées des stories. Le moteur utilise les dépendances des tâches pour autoriser leur départ.':'Liens : relations entre groupes de tâches ; une vue agrégée peut avoir des liens dans les deux sens.')));
 }
};
