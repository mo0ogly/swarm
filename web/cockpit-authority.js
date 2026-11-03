'use strict';
/** Main cockpit view "Utilisateurs et accès" (Wattson-style administration of
 * accounts, permissions and access logs). Mounts the shared module
 * `SwarmAuthorityAdmin` (talks only to the authority daemon API); the menu
 * entry and view stay hidden when the cockpit is not behind the daemon
 * (owner-key mode). No data is read or written by this file itself.
 *
 * Menu filtering (ADM-04): once `GET …/me` answers, menu entries whose
 * `view.<id>` right is not held are hidden, empty navigation groups close,
 * and `globalThis.SwarmAuthorityMenu` publishes the allowed set plus a
 * `visibleView()` check other scripts call before opening a view. A direct
 * access without the right shows a clear message instead of the section;
 * the API answers 403 on its own — the UI never grants anything. */
document.addEventListener('DOMContentLoaded',async()=>{
 const host=document.getElementById('native-authority-admin'),entry=document.querySelector('#tabs [data-view="authority"]');
 if(!host||!globalThis.SwarmAuthorityAdmin)return;
 const tr=s=>globalThis.SwarmI18n?.t(s)??s;
 // The cockpit marks every view with a generic "not yet connected" note;
 // this view is fully served by the daemon API, so that note is removed.
 for(const note of document.querySelectorAll('#authority > p.notice'))note.remove();
 let result={mode:'none'};
 try{result=await SwarmAuthorityAdmin.mount(host,document.querySelector('.page-header'),{translate:tr})}catch{}
 if(result.mode==='none'&&entry)entry.hidden=true;
 // Menu filtering (ADM-04). Runs only in authority mode with a views list;
 // owner-key mode keeps the full menu (no rights are known there).
 const meViews=result&&result.mode==='authority'&&Array.isArray(result.views)?result.views:null;
 if(meViews){
  // `/me` returns the permission KEYS (`view.<id>`); buttons carry the bare
  // view ids, so the allowed set is stored as ids.
  const allowed=new Set(meViews.map(k=>k.startsWith('view.')?k.slice(5):k));
  for(const b of document.querySelectorAll('#tabs [data-view]'))if(!allowed.has(b.dataset.view))b.hidden=true;
  for(const g of document.querySelectorAll('[data-nav-group]')){
   if(!g.querySelector('.nav-items [data-view]:not([hidden])')){
    if(g.querySelector('.nav-items a'))continue; // links (prepare, trust, PAEA) keep the group open
    g.hidden=true;
   }
  }
  globalThis.SwarmAuthorityMenu=Object.freeze({
   visibleView:name=>allowed.has(name),
   deniedMessage:()=>tr('Accès refusé : votre compte n’a pas le droit de voir cette page. Demandez-le à un administrateur (Utilisateurs et accès, onglet Habilitations).'),
  });
 }
});
