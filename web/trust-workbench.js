/* Reorganize existing Trust controls; keep their IDs, listeners and API gates. */
(()=>{
 'use strict';
 const tr=s=>globalThis.SwarmI18n?.t(s)??s;
 /** Create a text-only local node; no response/user HTML or credential copied. */
 function el(tag,text,cls){const node=document.createElement(tag);if(text!==undefined)node.textContent=tr(text);if(cls)node.className=cls;return node}
 /** Build same-origin navigation, retaining only public language/theme choices.
  * Values are reread on click, so theme changes are carried to the next page.
  * Never copies tokens or other query data into a link. */
 function link(text,view,path='/trust-actions.html'){const a=el('a',text),url=new URL(path,location.href);if(view)url.searchParams.set('view',view);const sync=()=>{url.searchParams.set('lang',document.documentElement.lang==='en'?'en':'fr');url.searchParams.set('theme',document.documentElement.dataset.theme||'sombre');a.href=url.pathname+url.search};sync();a.addEventListener('click',sync);return a}
 /** Reparent actual form groups into accessible tabs. Form IDs/listeners remain
  * authoritative. Keyboard selection changes visibility only, not policy. */
 function tabs(){const form=document.getElementById('configuration');if(!form)return;const names=[['biba','Intégrité — Biba'],['blp','Confidentialité — Bell-LaPadula'],['roles','Conflits de rôles'],['identities','Identités'],['keys','Clés de preuve']],bar=el('div',undefined,'workbench-security-tabs');bar.setAttribute('role','tablist');bar.setAttribute('aria-label',tr('Domaines de sécurité'));const panels=new Map();
  for(const [key,label]of names){const button=el('button',label);button.type='button';button.id='security-tab-'+key;button.setAttribute('role','tab');button.setAttribute('aria-controls','security-panel-'+key);const panel=el('section',undefined,'workbench-security-panel');panel.id='security-panel-'+key;panel.setAttribute('role','tabpanel');panel.setAttribute('aria-labelledby',button.id);panel.tabIndex=0;panels.set(key,{button,panel});bar.append(button);button.addEventListener('click',()=>select(key));button.addEventListener('keydown',event=>{const index=names.findIndex(([name])=>name===key);let next;if(event.key==='ArrowRight')next=(index+1)%names.length;else if(event.key==='ArrowLeft')next=(index+names.length-1)%names.length;else if(event.key==='Home')next=0;else if(event.key==='End')next=names.length-1;else return;event.preventDefault();select(names[next][0]);panels.get(names[next][0]).button.focus()})}
  /** Show exactly one known group with roving keyboard focus; no remote action. */
  function select(key){for(const [name,{button,panel}]of panels){const on=key===name;button.setAttribute('aria-selected',String(on));button.tabIndex=on?0:-1;panel.hidden=!on}}
  const policy=document.getElementById('policy'),technical=policy?.nextElementSibling,separation=document.getElementById('separation')?.closest('label'),ids=document.getElementById('identities')?.closest('label');
  const groups={biba:[policy,technical,separation],blp:[document.getElementById('blp-settings')],roles:[document.getElementById('trust-role-settings')],identities:[ids,ids?.nextElementSibling],keys:[document.getElementById('trust-signing-enabled')?.closest('fieldset')]};
  const anchor=document.getElementById('admin-message');form.insertBefore(bar,anchor);for(const [name,nodes]of Object.entries(groups)){const panel=panels.get(name).panel;for(const node of nodes)if(node)panel.append(node);form.insertBefore(panel,anchor)}select('biba');
 }
 /** Recorded transaction state → badge tone. Unknown codes stay neutral. */
 const auditStateTone={reconciled:'ok',committed:'ok',approved:'run',pending_approval:'wait',unknown:'run',denied:'bad'};
 /** Recorded evidence result → tone and short label (translated at display). */
 const auditResult={admitted:['ok','admis'],approved:['ok','approuvé'],committed:['ok','engagé'],reconciled:['ok','réconcilié'],denied:['bad','refusé'],unknown:['run','inconnu'],revoked:['bad','révoqué'],pending_approval:['wait','Approbation requise']};
 /** Evidence kind → step label (translated at display); other kinds stay raw. */
 const auditKind={'mandate-create':'Mandat admis','mandate-revoke':'Mandat révoqué',request:'Demande déposée',approve:'Approbation',commit:'Effet engagé',reconcile:'Réconciliation'};
 /** Text-only pill with a tone attribute; `label` is already translated. */
 function badge(tone,label){const b=el('span',undefined,'audit-badge');b.dataset.tone=tone;b.textContent=label;return b}
 /** Lay out the audit view like the "Audit et preuves" board: heading with the
  * real export/anchor buttons, a token strip, counters computed from the
  * recorded states carried by #trust-audit-action options, then three columns
  * (transactions, proof chain drawn from the last audit report, public proof
  * check). Existing controls are moved, never copied, so trust-native.js keeps
  * its listeners, tokens and API gates. Selecting a transaction only changes
  * the local select; reading its chain stays an explicit click because the
  * server records each audit access. No count, step or verdict is inferred:
  * missing data shows "—" or an empty state. Missing source nodes are skipped.
  * @param {HTMLElement} main page main element
  * @param {HTMLElement} header page header (already in main)
  * @param {HTMLElement|null} proofs original "Preuves" section
  * @returns {void} DOM only; no request, storage or token copy */
 function auditLayout(main,header,proofs){
  const $=id=>document.getElementById(id),select=$('trust-audit-action'),report=$('trust-audit-report');
  document.body.classList.add('audit-workbench');
  const heading=el('div',undefined,'audit-heading'),h1=header.querySelector('h1'),lead=main.querySelector('.workbench-lead');heading.append(el('p','04 · Suivi','audit-eyebrow'));if(h1)heading.append(h1);if(lead){lead.textContent=tr('Retracer une action de confiance, vérifier ses preuves, exporter pour un tiers. Lecture seule.');heading.append(lead)}header.prepend(heading);
  const tools=el('div',undefined,'audit-actions');for(const id of ['export','anchor'])if($(id))tools.append($(id));heading.after(tools);
  const strip=el('section',undefined,'audit-credentials');strip.append(el('h2','Identité de lecture'));for(const id of ['token','trust-admin-token']){const label=$(id)?.closest('label');if(label)strip.append(label)}if($('refresh'))strip.append($('refresh'));strip.append(el('p','Le jeton administrateur I3 est requis pour consulter une chaîne de preuve.','audit-hint'));
  const cards=el('section',undefined,'audit-cards');cards.setAttribute('aria-label',tr('Synthèse des transactions'));
  const grid=el('div',undefined,'audit-grid');
  const txPanel=el('section',undefined,'audit-panel audit-transactions');txPanel.append(el('h2','Transactions'));const filterLabel=el('label',undefined,'audit-filter'),filter=el('input');filter.type='search';filter.placeholder=tr('Identifiant, état, ressource…');filterLabel.append(el('span','Filtrer'),filter);const list=el('div',undefined,'audit-list');txPanel.append(filterLabel,list);
  const chain=el('section',undefined,'audit-panel audit-chain'),chainId=el('p',undefined,'audit-mono'),chainTitle=el('h2'),steps=el('ol',undefined,'audit-steps'),missing=el('p',undefined,'notice alert audit-missing');chainTitle.id='audit-chain-title';chain.setAttribute('aria-labelledby',chainTitle.id);missing.hidden=true;chain.append(chainId,chainTitle);
  const read=$('trust-audit-read'),summary=$('trust-audit-summary'),raw=report?.closest('details');const readRow=el('div',undefined,'audit-read');if(read)readRow.append(read);readRow.append(el('p','Chaque consultation est enregistrée dans le journal d’audit.','audit-hint'));chain.append(readRow);if(summary)chain.append(summary);chain.append(missing,steps);if(raw)chain.append(raw);
  const aside=el('aside',undefined,'audit-panel audit-verify'),verify=$('trust-public-verify')?.closest('details');if(verify){verify.open=true;aside.append(verify)}const technical=proofs?.querySelector('details');if(technical)aside.append(technical);
  grid.append(txPanel,chain,aside);
  const anchorNode=proofs||main.lastElementChild;(anchorNode||header).after(strip);strip.after(cards);cards.after(grid);
  for(const legacy of [select?.closest('label'),$('trust-audit-title')?.closest('section'),proofs])if(legacy)legacy.hidden=true;
  let chosen=select?.value||'';
  /** Option for an id, or null. */
  const optionOf=id=>select?[...select.options].find(o=>o.value===id)||null:null;
  /** Translated recorded-state label, taken from the option text after " · ". */
  const stateLabel=o=>{const text=o.textContent||'',at=text.indexOf(' · ');return at<0?(o.dataset.state||''):text.slice(at+3)};
  /** Human title: resource, then amount and currency when recorded. */
  const titleOf=o=>[o.dataset.resource,o.dataset.amount&&o.dataset.amount+' '+(o.dataset.currency||'')].filter(Boolean).join(' · ')||o.value;
  /** Rebuild the four counters from the option data; "—" before any load. */
  function paintCards(){const options=select?[...select.options]:[],loaded=select?.dataset.loaded==='true',count=state=>options.filter(o=>o.dataset.state===state).length;cards.replaceChildren();
   for(const [label,value,tone,hint] of [['Transactions auditables',options.length,'run',loaded?tr('Résultat incertain')+' : '+count('unknown'):tr('Non chargé')],['Approbations attendues',count('pending_approval'),'wait',tr('Approbation requise')],['Refusées',count('denied'),'bad',tr('refusé')]]){const card=el('div',undefined,'audit-card');card.append(el('span',label,'audit-card-label'),badge(loaded?tone:'none',loaded?String(value):'—'),el('span',undefined,'audit-card-hint'));card.lastChild.textContent=loaded?hint:tr('Non chargé');cards.append(card)}
   const paea=link('Vérifications PAEA',null,'/paea.html');paea.className='audit-card audit-card-link';paea.replaceChildren(el('span','Vérifications PAEA','audit-card-label'),el('span','Ouvrir les vérifications','audit-card-hint'));cards.append(paea)}
  /** Rebuild the filtered transaction buttons; the pressed one mirrors select. */
  function paintList(){const options=select?[...select.options]:[],needle=filter.value.trim().toLowerCase();list.replaceChildren();if(!options.length){list.append(el('p',select?.dataset.loaded==='true'?'Aucune transaction enregistrée.':'Transactions non lues : Actualiser.','audit-empty'));return}
   const shown=options.filter(o=>!needle||[o.value,stateLabel(o),o.dataset.resource||''].some(v=>v.toLowerCase().includes(needle)));if(!shown.length){list.append(el('p','Aucune transaction ne correspond au filtre.','audit-empty'));return}
   for(const o of shown){const b=el('button',undefined,'audit-tx');b.type='button';b.dataset.tx=o.value;b.setAttribute('aria-pressed',String(o.value===chosen));const top=el('span',undefined,'audit-tx-top'),id=el('span',undefined,'audit-mono');id.textContent=o.value;top.append(id,badge(auditStateTone[o.dataset.state]||'none',stateLabel(o)));const title=el('span',undefined,'audit-tx-title');title.textContent=titleOf(o);b.append(top,title);b.addEventListener('click',()=>{chosen=o.value;select.value=o.value;select.dispatchEvent(new Event('change',{bubbles:true}));paintList();paintChain()});list.append(b)}}
  /** Draw the chain for the chosen transaction from the last audit report,
   * only when that report belongs to it; otherwise show the empty state. */
  function paintChain(){const o=optionOf(chosen);chainId.textContent=o?o.value:'';chainTitle.textContent=o?tr('Chaîne de preuve')+' — '+titleOf(o):tr('Choisir une transaction');steps.replaceChildren();missing.hidden=true;
   let v=null;try{v=report?.textContent?JSON.parse(report.textContent):null}catch{v=null}
   if(!o||!v||v.action_id!==o.value||!Array.isArray(v.events)){steps.append(el('li','Chaîne non consultée pour cette transaction.','audit-empty'));return}
   for(const e of v.events){const [tone,resultLabel]=auditResult[e.result]||['none',String(e.result??'')];const li=el('li',undefined,'audit-step'),dot=el('span',undefined,'audit-dot'),body=el('div',undefined,'audit-step-body'),line=el('div',undefined,'audit-step-line'),label=el('strong'),op=el('span',undefined,'audit-mono');dot.dataset.tone=tone;label.textContent=auditKind[e.kind]?tr(auditKind[e.kind]):String(e.kind??'');op.textContent=String(e.kind??'');line.append(label,op,badge(tone,tr(resultLabel)));body.append(line);
    const who=el('div',undefined,'audit-step-detail');who.textContent=[e.actor,e.integrity,e.at].filter(v=>v!=null&&v!=='').map(String).join(' · ');body.append(who);if(e.reason){const why=el('div',undefined,'audit-step-detail');why.textContent=String(e.reason);body.append(why)}
    const hash=el('div',undefined,'audit-mono');hash.textContent='#'+String(e.sequence??'')+(typeof e.hash==='string'?' · '+e.hash.slice(0,16)+'…':'');body.append(hash);li.append(dot,body);steps.append(li)}
   if(Array.isArray(v.missing_evidence)&&v.missing_evidence.length){missing.hidden=false;missing.textContent=tr('Preuves manquantes')+' : '+v.missing_evidence.map(String).join(', ')}}
  filter.addEventListener('input',paintList);
  if(select)new MutationObserver(()=>{if(chosen&&optionOf(chosen))select.value=chosen;else chosen=select.value||'';paintCards();paintList();paintChain()}).observe(select,{childList:true,attributes:true,attributeFilter:['data-loaded']});
  if(report)new MutationObserver(paintChain).observe(report,{childList:true,characterData:true,subtree:true});
  const more=el('nav',undefined,'toolbar');more.append(link('Sécurité et preuves',null,'/security-proofs.html'),link('Vérifications PAEA',null,'/paea.html'));main.append(more);
  paintCards();paintList();paintChain();
 }
 /** Apply the adopted security/audit composition to the existing authenticated
  * page. The view and incoming theme accept only their known UI enum values; all writes still use trust-native.js and its real
  * preview/token/revision controls. Missing source DOM leaves the old page usable. */
 function mount(){const main=document.querySelector('body>main'),header=document.querySelector('body>header');if(!main||!header||!document.getElementById('command'))return;const theme=new URL(location.href).searchParams.get('theme');if(['etat','sombre'].includes(theme))document.documentElement.dataset.theme=theme;const requested=new URL(location.href).searchParams.get('view'),mode=['security','audit'].includes(requested)?requested:'actions';document.body.classList.add('trust-workbench');const rail=el('aside',undefined,'workbench-rail');rail.setAttribute('aria-label',tr('Navigation principale'));const brand=link('Swarm',null,'/'),logo=el('img');logo.src='/swarm-logo.png';logo.alt='';logo.width=44;logo.height=44;brand.prepend(logo);brand.className='workbench-brand';rail.append(brand);const nav=el('nav');for(const [label,view,path]of [['03 · Conduite',null,'/'],['Actions de confiance','actions','/trust-actions.html'],['04 · Audit et preuves','audit','/trust-actions.html'],['05 · Administration · Sécurité','security','/trust-actions.html']]){const a=link(label,view,path);if(view===mode)a.setAttribute('aria-current','page');nav.append(a)}rail.append(nav);document.body.prepend(rail);header.classList.add('workbench-page-header');main.prepend(header);themeSwitch(header);const h1=header.querySelector('h1');if(mode!=='actions'&&h1)h1.textContent=tr(mode==='security'?'Sécurité':'Audit et preuves');
  const acting=document.getElementById('command').closest('section'),actions=document.getElementById('actions').closest('section'),proofs=document.getElementById('export').closest('section');
  if(mode!=='actions'){acting.hidden=true;actions.hidden=true;const intro=el('p',mode==='security'?'Politiques d’intégrité et de confidentialité, rôles, identités et clés de preuve.':'Retracer une action de confiance et vérifier ses preuves.','workbench-lead');header.after(intro)}
  if(mode==='security'){proofs.hidden=true;const summary=el('section',undefined,'workbench-security-overview');summary.append(el('h2','Administration · Sécurité'));for(const label of ['Intégrité — Biba','Confidentialité — Bell-LaPadula','Conflits de rôles','Identités','Clés de preuve'])summary.append(el('p',label));const admin=document.getElementById('admin');admin.textContent=tr('Régler la sécurité');summary.append(admin);main.append(summary);main.append(link('Ouvrir l’audit et les preuves','audit'))}
  if(mode==='audit')auditLayout(main,header,proofs);tabs();
 }
 /** Replace the single "change theme" button by a visible Dark/Light switch at
  * the right end of the page header. The original #theme button stays in the
  * DOM, hidden, because trust-native.js binds it. A choice sets data-theme and
  * the public `theme` URL parameter (replaceState, so links and reloads keep
  * it); no storage, request or other query data is written. A refused URL
  * update leaves the theme applied for this page only.
  * @param {HTMLElement} header page header receiving the switch
  * @returns {void} */
 function themeSwitch(header){const old=document.getElementById('theme');if(old)old.hidden=true;const group=el('div',undefined,'workbench-theme-switch');group.setAttribute('role','group');group.setAttribute('aria-label',tr('Thème'));const buttons=[['sombre','Sombre'],['etat','Clair']].map(([value,label])=>{const b=el('button',label);b.type='button';b.dataset.themeChoice=value;b.addEventListener('click',()=>{document.documentElement.dataset.theme=value;paint();try{const url=new URL(location.href);url.searchParams.set('theme',value);history.replaceState(history.state,'',url)}catch{}});return b});
  /** Mark the button matching the current data-theme as pressed. */
  function paint(){const current=document.documentElement.dataset.theme==='etat'?'etat':'sombre';for(const b of buttons)b.setAttribute('aria-pressed',String(b.dataset.themeChoice===current))}
  group.append(...buttons);header.append(group);paint();new MutationObserver(paint).observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']})}
 document.addEventListener('DOMContentLoaded',mount);
})();
