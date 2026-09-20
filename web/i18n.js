'use strict';
// UI strings only. Never translate model output, user documents or JSON payloads.
(function(root){
 const supported=['fr','en'],storageKey='swarm.language';
 let saved='fr';try{saved=localStorage.getItem(storageKey)||'fr'}catch{}
 const requested=typeof location==='object'?new URLSearchParams(location.search).get('lang'):null;
 const language=supported.includes(requested)?requested:supported.includes(saved)?saved:'fr';
 if(supported.includes(requested))try{localStorage.setItem(storageKey,requested)}catch{}
 const catalog=root.SwarmEnglish||{};
 function t(source,values){let value=language==='en'&&Object.hasOwn(catalog,source)?catalog[source]:source;if(values)for(const [key,text]of Object.entries(values))value=value.replaceAll('{'+key+'}',()=>String(text));return value}
 const patterns=Object.entries(catalog).filter(([s,t])=>s!==t&&/%(?:\.\d+)?[dsfqwv]/.test(s)).sort((a,b)=>b[0].length-a[0].length).map(([source,target])=>{const parts=source.split(/(%(?:\.\d+)?[dsfqwv])/);const expression=parts.map(p=>/^%/.test(p)?'(.*?)':p.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')).join('');return [new RegExp('^'+expression+'$'),target]});
 function engine(source){if(typeof source!=='string'||language!=='en')return source;const direct=t(source);if(direct!==source)return direct;if(source.length>10000)return source;const deliveryStart='Livraison incomplète : ',deliveryEnd='. Aucun appel de revue lancé ; le responsable doit examiner les éléments manquants avant une reprise autorisée.';if(source.startsWith(deliveryStart)&&source.endsWith(deliveryEnd))return t(deliveryStart)+engine(source.slice(deliveryStart.length,-deliveryEnd.length))+t(deliveryEnd);for(const prefix of ['Organisation invalide : ','Politique de validation non autorisée : ','Responsable parent absent : ','Tâche sans responsable : ','Validation incomplète : '])if(source.startsWith(prefix))return t(prefix)+t(source.slice(prefix.length));for(const [pattern,target]of patterns){const match=source.match(pattern);if(match){let i=0;return target.replace(/%(?:\.\d+)?[dsfqwv]/g,()=>match[++i])}}return source}
 const api={language,engine,locale:language==='en'?'en-US':'fr-FR',t};root.SwarmI18n=api;
 if(typeof module==='object')module.exports=api;
 if(typeof document!=='object')return;
 document.documentElement.lang=language;
 // This script runs before application scripts. Only the original static DOM is
 // visited; dynamically inserted mission titles and reports are never scanned.
 const walker=document.createTreeWalker(document.documentElement,NodeFilter.SHOW_TEXT);
 const nodes=[];while(walker.nextNode())nodes.push(walker.currentNode);
 for(const n of nodes){if(n.parentElement?.closest('script,style,textarea,pre,code'))continue;const raw=n.nodeValue;const text=raw.trim();if(text&&Object.hasOwn(catalog,text))n.nodeValue=raw.replace(text,()=>t(text))}
 for(const e of document.querySelectorAll('[title],[placeholder],[aria-label],[alt]'))for(const key of ['title','placeholder','aria-label','alt'])if(e.hasAttribute(key))e.setAttribute(key,t(e.getAttribute(key)));
 const host=document.querySelector('.rail-bottom,.prep-header nav,.terminal-toolbar')||document.querySelector('header');
 if(host){const label=document.createElement('label');label.className='language-picker';label.htmlFor='swarm-language';label.textContent=t('Langue');const select=document.createElement('select');select.id='swarm-language';select.name='ui-language';for(const [value,text]of [['fr','Français'],['en','English']]){const option=document.createElement('option');option.value=value;option.textContent=text;select.append(option)}select.value=language;label.append(select);host.append(label);
 select.addEventListener('change',()=>{const next=select.value;if(!supported.includes(next))return;const event=new CustomEvent('swarm:language-change',{cancelable:true,detail:{language:next}});if(!dispatchEvent(event)){select.value=language;return}try{localStorage.setItem(storageKey,next)}catch{}const url=new URL(location.href);url.searchParams.set('lang',next);location.assign(url.href)});
 }
})(globalThis);
