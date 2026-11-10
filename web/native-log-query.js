 'use strict';
/** Explicit historical log search. No polling, process control or retention.
 * The host supplies one Work/Agent scope; rows remain unauthenticated records. */
(()=>{
 /** Mount literal filters and an explicit next-page action. Only a validated
  * scope/shape/exact-cursor page is adopted. Source failures stay visible and
  * earlier results stay labelled; no empty success, retry or row fallback. */
 function mount(host,transport,{translate:tr=x=>x,agent}={}){
  const el=(tag,text,cls)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=tr(text);if(cls)e.className=cls;return e};
  const fold=el('details'),title=el('summary','Rechercher dans l’historique'),controls=el('div',undefined,'live-logtools'),query=el('input'),kind=el('input'),qLabel=el('label','Texte littéral'),kLabel=el('label','Type exact, vide pour tous'),read=el('button','Rechercher'),more=el('button','Lire la suite'),status=el('p','Lecture à la demande.','live-status'),anchor=el('p'),rows=el('ol',undefined,'live-log');query.type='search';kind.type='text';query.dataset.logQueryText='';kind.dataset.logQueryKind='';read.type=more.type='button';read.dataset.logQueryRead='';more.dataset.logQueryMore='';more.hidden=true;rows.dataset.logQueryRows='';status.dataset.logQueryStatus='';status.setAttribute('role','status');status.tabIndex=-1;qLabel.append(query);kLabel.append(kind);controls.append(qLabel,kLabel,read,more);fold.dataset.logQuery='';fold.append(title,el('p','Les filtres portent sur les lignes conservées de cet agent. Une entrée n’est pas une preuve d’exécution ou d’identité.'),controls,status,anchor,rows);host.append(fold);let busy=false,after='0',generation=0;
  /** Discard unconfirmed local page state after a filter edit, without IO. */
  function invalidate(){generation++;after='0';more.hidden=true;rows.replaceChildren();anchor.textContent='';status.className='live-status';status.textContent=tr('Recherche modifiée. Lancez une nouvelle lecture.')}
  /** One awaited read; cursor adoption uses decimal i64 strings throughout. A
   * stale/detached result is ignored, not published under another filter/scope. */
  async function load(append){if(busy)return;busy=true;const ticket=++generation;for(const e of [query,kind,read,more])e.disabled=true;status.className='live-status';status.textContent=tr('Chargement…');const before=append?after:'0';try{
   const page=await transport.read({after:before,q:query.value,kind:kind.value});if(ticket!==generation||!fold.isConnected)return;
   if(!page||!Array.isArray(page.entries)||typeof page.more!=='boolean'||typeof page.retention_gap!=='boolean'||!page.exact||!Array.isArray(page.exact.sequences)||page.exact.sequences.length!==page.entries.length)throw {code:'log-query-view-invalid'};
   const C=globalThis.SwarmRecordedCursor,next=C.integer(page.next_cursor,page.exact.next_cursor,true),first=C.integer(page.retained_from,page.exact.retained_from,true),seq=page.entries.map((e,i)=>{if(!e||e.agent_id!==agent||!['at','kind','message'].every(k=>typeof e[k]==='string'))throw {code:'log-query-scope-invalid'};return C.integer(e.seq,page.exact.sequences[i],true)});
   if(seq.some((v,i)=>BigInt(v)<=BigInt(i?seq[i-1]:before))||next!==(seq.at(-1)??before)||page.more&&!seq.length)throw {code:'log-query-cursor-invalid'};
   const fragment=document.createDocumentFragment();for(const [i,e]of page.entries.entries()){const li=el('li',undefined,['error','output-limit'].includes(e.kind)?'kind-error':'kind-message'),time=el('time'),type=el('span',undefined,'live-log-kind'),message=el('span',undefined,'live-log-msg');time.textContent=e.at;type.textContent=seq[i]+' · '+e.kind;message.textContent=e.message;li.append(time,type,message);fragment.append(li)}
   if(!append)rows.replaceChildren();rows.append(fragment);if(!append&&!seq.length)rows.append(el('li','Aucune ligne ne correspond à cette recherche.'));after=next;more.hidden=!page.more;anchor.textContent=tr('Première séquence conservée')+' : '+first;status.textContent=tr(page.retention_gap?'Des lignes antérieures au curseur ne sont plus conservées. Relancez la recherche pour repartir des données disponibles.':'Journaux relus.');
  }catch(error){if(ticket!==generation||!fold.isConnected)return;status.className='live-status is-error';const code=typeof error?.code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(error.code)?error.code:'log-query-read-failed';status.textContent=tr('La recherche a échoué. Les résultats déjà lus sont conservés ; relancez pour reprendre.')+' ('+code+')'}finally{busy=false;for(const e of [query,kind,read,more])e.disabled=false}}
  query.addEventListener('input',invalidate);kind.addEventListener('input',invalidate);read.addEventListener('click',()=>load(false));more.addEventListener('click',()=>load(true));return {read:()=>load(false)};
 }
 globalThis.SwarmNativeLogQuery=Object.freeze({mount});
})();
