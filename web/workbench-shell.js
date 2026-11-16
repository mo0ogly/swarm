/* Shared workbench shell for standalone pages (PAEA, security proofs). */
(()=>{
 'use strict';
 const tr=s=>globalThis.SwarmI18n?.t(s)??s;
 /** Create a text-only node; text is translated, never parsed as HTML. */
 function el(tag,text,cls){const node=document.createElement(tag);if(text!==undefined)node.textContent=tr(text);if(cls)node.className=cls;return node}
 /** Same-origin link keeping only the public language/theme choices, reread on
  * click so a theme change reaches the next page. Never copies tokens or other
  * query data. */
 function link(text,path,view){const a=el('a',text),url=new URL(path,location.href);if(view)url.searchParams.set('view',view);const sync=()=>{url.searchParams.set('lang',document.documentElement.lang==='en'?'en':'fr');url.searchParams.set('theme',document.documentElement.dataset.theme||'etat');a.href=url.pathname+url.search};sync();a.addEventListener('click',sync);return a}
 /** Rail entries shared with web/trust-workbench.js: label, path, view, key. */
 const entries=[['03 · Conduite','/',null,'conduite'],['Actions de confiance','/trust-actions.html','actions','actions'],['04 · Audit et preuves','/trust-actions.html','audit','audit'],['Vérifications PAEA','/paea.html',null,'paea'],['05 · Administration · Sécurité','/trust-actions.html','security','security'],['Sécurité et preuves','/security-proofs.html',null,'security-proofs']];
 /** Visible Dark/Light switch at the end of the page header. The page's own
  * toggle button (id from data-workbench-theme-button) stays in the DOM,
  * hidden, because the page script binds it. A choice sets data-theme and the
  * public `theme` URL parameter (replaceState); no storage or request.
  * @param {HTMLElement} header page header receiving the switch
  * @param {string|undefined} oldId id of the page's original toggle */
 function themeSwitch(header,oldId){const old=oldId&&document.getElementById(oldId);if(old)old.hidden=true;const group=el('div',undefined,'workbench-theme-switch');group.setAttribute('role','group');group.setAttribute('aria-label',tr('Thème'));const buttons=[['sombre','Sombre'],['etat','Clair']].map(([value,label])=>{const b=el('button',label);b.type='button';b.dataset.themeChoice=value;b.addEventListener('click',()=>{document.documentElement.dataset.theme=value;try{const url=new URL(location.href);url.searchParams.set('theme',value);history.replaceState(history.state,'',url)}catch{}});return b});
  /** Mark the button matching the current data-theme as pressed. */
  function paint(){const current=document.documentElement.dataset.theme==='sombre'?'sombre':'etat';for(const b of buttons)b.setAttribute('aria-pressed',String(b.dataset.themeChoice===current))}
  group.append(...buttons);header.append(group);paint();new MutationObserver(paint).observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']})}
 /** Mount the shell when <body data-workbench-shell> is present: rail with the
  * current entry (data-workbench-current) marked, header moved into main with
  * an eyebrow (data-workbench-eyebrow) and the first paragraph of main as its
  * lead, Dark/Light switch, and links marked data-workbench-links gathered in a
  * card grid. Nodes are moved, never copied: IDs, listeners, hrefs and API
  * calls stay with the page script. Missing header/main leaves the page as is.
  * @returns {void} DOM only; no request, storage or token copy */
 function mount(){const body=document.body;if(!body?.hasAttribute('data-workbench-shell'))return;const header=document.querySelector('body>header'),main=document.querySelector('body>main');if(!header||!main)return;
  const theme=new URL(location.href).searchParams.get('theme');if(['etat','sombre'].includes(theme))document.documentElement.dataset.theme=theme;
  body.classList.add('trust-workbench','workbench-shell-page');
  const rail=el('aside',undefined,'workbench-rail');rail.setAttribute('aria-label',tr('Navigation principale'));const brand=link('Swarm','/'),logo=el('img');logo.src='/swarm-logo.png';logo.alt='';logo.width=44;logo.height=44;brand.prepend(logo);brand.className='workbench-brand';rail.append(brand);const nav=el('nav');for(const [label,path,view,key]of entries){const a=link(label,path,view);if(key===body.dataset.workbenchCurrent)a.setAttribute('aria-current','page');nav.append(a)}rail.append(nav);
  const skip=body.querySelector(':scope>a[href^="#"]');if(skip)skip.after(rail);else body.prepend(rail);
  header.classList.add('workbench-page-header');main.prepend(header);
  const heading=el('div',undefined,'audit-heading'),h1=header.querySelector('h1');if(body.dataset.workbenchEyebrow)heading.append(el('p',body.dataset.workbenchEyebrow,'audit-eyebrow'));if(h1)heading.append(h1);const lead=header.nextElementSibling;if(lead?.tagName==='P'&&!lead.id){lead.classList.add('workbench-lead');heading.append(lead)}header.prepend(heading);
  themeSwitch(header,body.dataset.workbenchThemeButton);
  const links=[...main.querySelectorAll('[data-workbench-links] a, p>a[data-paea-link]')];if(links.length){const grid=el('nav',undefined,'shell-links');grid.setAttribute('aria-label',tr(body.dataset.workbenchLinksLabel||'Vues'));const holders=new Set();for(const a of links){holders.add(a.parentElement);a.classList.add('shell-link');grid.append(a)}const first=[...holders][0];first.before(grid);for(const p of holders)if(p!==grid&&!p.children.length&&!p.textContent.trim())p.remove()}
 }
 document.addEventListener('DOMContentLoaded',mount);
})();
