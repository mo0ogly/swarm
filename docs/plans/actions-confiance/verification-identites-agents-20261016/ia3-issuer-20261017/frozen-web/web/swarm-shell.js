'use strict';
// Presentation only. No engine commands, permission changes or mission mutations.
(function(root){
 const tr=source=>root.SwarmI18n?.t(source)??source;
 const get=key=>{try{return localStorage.getItem(key)}catch{return null}};
 const put=(key,value)=>{try{localStorage.setItem(key,value)}catch{}};
 const nav=document.getElementById('tabs');
 const toggle=document.getElementById('nav-toggle');
 const collapse=document.getElementById('nav-collapse');
 function setCollapsed(collapsed){
  document.body.dataset.navCollapsed=String(collapsed);
  collapse?.setAttribute('aria-expanded',String(!collapsed));
  if(collapse){
   collapse.querySelector('[data-nav-label]').textContent=tr(collapsed?'Ouvrir le menu':'Replier le menu');
   collapse.querySelector('[aria-hidden]').textContent=collapsed?'→':'←';
  }
 }
 if(collapse){
  setCollapsed(get('swarm-nav:collapsed')==='true');
  collapse.addEventListener('click',()=>{
   const collapsed=document.body.dataset.navCollapsed!=='true';
   setCollapsed(collapsed);put('swarm-nav:collapsed',String(collapsed));
  });
 }
 function closeMobile(){document.body.dataset.navOpen='false';toggle?.setAttribute('aria-expanded','false')}
 function navigate(name,close=true){
  const active=nav?.querySelector('[data-view="'+name+'"]');
  if(active){active.closest('details').open=true;}
  if(close)closeMobile();
 }
 let decisionAction=null;
 function updateDecisions(count,action,label){
  decisionAction=action;
  const button=document.getElementById('nav-interventions');
  if(!button)return;
  button.hidden=!(count>0);button.querySelector('span').textContent=label||tr('Décisions à traiter');document.getElementById('nav-decision-count').textContent=String(count);
 }
 root.SwarmShell={navigate,updateDecisions};
 for(const detail of document.querySelectorAll('[data-nav-group]')){
  const key='swarm-nav:'+detail.dataset.navGroup,saved=get(key);
  if(saved!==null)detail.open=saved==='open';
  detail.addEventListener('toggle',()=>put(key,detail.open?'open':'closed'));
 }
 nav?.querySelector('[aria-current=page]')?.closest('details')?.setAttribute('open','');
 toggle?.addEventListener('click',()=>{
  const open=document.body.dataset.navOpen!=='true';document.body.dataset.navOpen=String(open);toggle.setAttribute('aria-expanded',String(open));
  if(open)document.querySelector('.rail #work')?.focus();
 });
 document.addEventListener('keydown',event=>{
  if(event.key==='Escape'&&document.body.dataset.navOpen==='true'){closeMobile();toggle?.focus();}
 });
 nav?.addEventListener('click',event=>{
  if(!event.target.closest('[data-view]'))return;
  document.getElementById('main-content')?.focus({preventScroll:true});
 });
 document.getElementById('nav-interventions')?.addEventListener('click',()=>{
  decisionAction?.();
 });
 // i18n inserts this picker before the shell is initialized.
 const picker=document.querySelector('.rail-bottom>.language-picker');
 if(picker)document.querySelector('.rail-settings-body')?.append(picker);
 const initialized=new WeakSet();
 function restoreFolds(container=document){
  const folds=[...container.querySelectorAll('details[data-fold-key]')];
  if(container.matches?.('details[data-fold-key]'))folds.unshift(container);
  for(const detail of folds){
   if(initialized.has(detail))continue;initialized.add(detail);
   const key='swarm-reading:'+detail.dataset.foldKey,saved=get(key);
   if(saved!==null)detail.open=saved==='open';
   detail.addEventListener('toggle',()=>{if(detail.isConnected)put(key,detail.open?'open':'closed')});
  }
 }
 restoreFolds();
 // Feature renderers mount graph controls after the first snapshot.
 new MutationObserver(records=>{
  for(const record of records)for(const added of record.addedNodes){
   // SVG changes and terminal output cannot add reading accordions.
   if(added.nodeType===1&&added.namespaceURI==='http://www.w3.org/1999/xhtml')restoreFolds(added);
  }
 }).observe(document.body,{childList:true,subtree:true});
})(globalThis);
