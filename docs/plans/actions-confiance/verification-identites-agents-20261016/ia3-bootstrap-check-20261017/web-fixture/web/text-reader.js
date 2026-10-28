'use strict';
// Read-only structure. Source text is never HTML, translated, saved or executed.
(function(root){
 const tr=(text,values)=>root.SwarmI18n?.t(text,values)??text.replace(/\{(\w+)\}/g,(_,key)=>String(values?.[key]??'{'+key+'}'));
 let sequence=0;const viewStates=new Map();
 const item=line=>{const m=line.match(/^(\s*)([-+*]|\d+[.)])\s+(.+)$/);return m?{indent:m[1].replace(/\t/g,'    ').length,ordered:/\d/.test(m[2]),start:parseInt(m[2],10)||1,text:m[3]}:null};
 const cells=line=>line.trim().replace(/^\||\|$/g,'').split(/(?<!\\)\|/).map(s=>s.trim().replace(/\\\|/g,'|'));
 function parse(source){
  const lines=String(source).replace(/\r\n?/g,'\n').split('\n'),blocks=[];let i=0;
  const tableAt=n=>n+1<lines.length&&lines[n].includes('|')&&/^\s*\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)+\|?\s*$/.test(lines[n+1]);
  const heading=line=>line.match(/^(#{1,6})\s+(.+?)(?:\s+#+)?\s*$/);
  const section=n=>/^[^\s].*[:：]\s*$/.test(lines[n])&&!lines[n].includes('://')&&(n+1===lines.length||!lines[n+1].trim()||item(lines[n+1]));
  function list(){
   const first=item(lines[i]),block={type:'list',ordered:first.ordered,start:first.start,items:[]};
   while(i<lines.length){
    const current=item(lines[i]);if(!current||current.indent<first.indent)break;
    if(current.indent>first.indent){if(!block.items.length)break;block.items.at(-1).children.push(list());continue}
    if(current.ordered!==first.ordered)break;
    const task=current.text.match(/^\[([ xX])\]\s+(.*)$/);
    block.items.push({text:task?task[2]:current.text,checked:task?task[1]!==' ':null,children:[]});i++;
   }
   return block;
  }
  while(i<lines.length){
   const line=lines[i];if(!line.trim()){i++;continue}
   const fence=line.match(/^\s*(`{3,}|~{3,})\s*([^\s]*)\s*$/);
   if(fence){const body=[],end=new RegExp('^\\s*'+fence[1][0]+'{'+fence[1].length+',}\\s*$');i++;while(i<lines.length&&!end.test(lines[i]))body.push(lines[i++]);if(i<lines.length)i++;blocks.push({type:'code',language:fence[2],text:body.join('\n')});continue}
   const h=heading(line);if(h){blocks.push({type:'heading',level:h[1].length,text:h[2]});i++;continue}
   if(section(i)){blocks.push({type:'heading',level:3,text:line.trim()});i++;continue}
   if(tableAt(i)){const headers=cells(line);i+=2;const rows=[];while(i<lines.length&&lines[i].trim()&&lines[i].includes('|'))rows.push(cells(lines[i++]));blocks.push({type:'table',headers,rows});continue}
   if(item(line)){blocks.push(list());continue}
   if(/^\s*>/.test(line)){const body=[];while(i<lines.length&&/^\s*>/.test(lines[i]))body.push(lines[i++].replace(/^\s*>\s?/,''));const signal=body[0]?.match(/^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*$/);if(signal)body.shift();blocks.push({type:'quote',kind:signal?.[1]||'',text:body.join('\n')});continue}
   const paragraph=[line];i++;
   while(i<lines.length&&lines[i].trim()&&!heading(lines[i])&&!item(lines[i])&&!/^\s*(>|`{3,}|~{3,})/.test(lines[i])&&!tableAt(i)&&!section(i))paragraph.push(lines[i++]);
   blocks.push({type:'paragraph',text:paragraph.join('\n')});
  }
  return blocks;
 }
 function node(tag,text,cls){const el=document.createElement(tag);if(text!==undefined)el.textContent=text;if(cls)el.className=cls;return el}
 function inline(parent,text){
  const tokens=/(`+)([^`\n]+)\1|\*\*([^*\n]+)\*\*|__([^_\n]+)__|\*([^*\n]+)\*|\[([^\]\n]+)\]\(([^\s)]+)\)/g;let previous=0,match;
  while((match=tokens.exec(text))){
   parent.append(document.createTextNode(text.slice(previous,match.index)));
   if(match[1])parent.append(node('code',match[2]));
   else if(match[3]||match[4])parent.append(node('strong',match[3]||match[4]));
   else if(match[5])parent.append(node('em',match[5]));
   else if(/^https?:\/\//i.test(match[7])){try{const url=new URL(match[7]);if(!['http:','https:'].includes(url.protocol))throw Error();const link=node('a',match[6]);link.href=url.href;link.target='_blank';link.rel='noopener noreferrer';parent.append(link)}catch{parent.append(document.createTextNode(match[0]))}}
   else parent.append(document.createTextNode(match[0]));
   previous=tokens.lastIndex;
  }
  parent.append(document.createTextNode(text.slice(previous)));
 }
 function codeTokens(parent,text,language){
  if(!/^jsonc?$/i.test(language)){parent.textContent=text;return}
  const tokens=/("(?:[^"\\]|\\.)*")(\s*:)?|\b(?:true|false|null)\b|-?\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b/g;let previous=0,match;
  while((match=tokens.exec(text))){parent.append(document.createTextNode(text.slice(previous,match.index)));const value=match[0],cls=match[2]?'key':match[1]?'string':/^(true|false|null)$/.test(value)?'literal':'number';parent.append(node('span',value,'reader-token-'+cls));previous=tokens.lastIndex}
  parent.append(document.createTextNode(text.slice(previous)));
 }
 function content(blocks,host,headings,id){
  for(const block of blocks){
   if(block.type==='heading'){const heading=node('h'+Math.min(6,block.level+2),undefined,'text-reader-heading');heading.id=id+'-section-'+headings.length;heading.dataset.readerAction='section:'+headings.length;heading.tabIndex=-1;inline(heading,block.text);headings.push({element:heading,text:block.text});host.append(heading)}
   else if(block.type==='paragraph'){const p=node('p');inline(p,block.text);host.append(p)}
   else if(block.type==='list'){const list=node(block.ordered?'ol':'ul');if(block.ordered)list.start=block.start;for(const entry of block.items){const li=node('li');if(entry.checked!==null){const check=node('input');check.type='checkbox';check.checked=entry.checked;check.disabled=true;check.setAttribute('aria-label',entry.text);li.className='text-reader-check';li.append(check)}inline(li,entry.text);content(entry.children,li,headings,id);list.append(li)}host.append(list)}
   else if(block.type==='code'){const box=node('figure',undefined,'text-reader-code');box.append(node('figcaption',block.language||tr('Code')));const pre=node('pre'),code=node('code');codeTokens(code,block.text,block.language);pre.append(code);box.append(pre);host.append(box)}
   else if(block.type==='quote'){const tone={WARNING:'attention',CAUTION:'alert',NOTE:'info',TIP:'info',IMPORTANT:'info'}[block.kind];const quote=node('blockquote',undefined,tone?'text-reader-callout '+tone:'');if(block.kind)quote.append(node('strong',tr({WARNING:'Attention',CAUTION:'Prudence',NOTE:'Note',TIP:'Conseil',IMPORTANT:'Important'}[block.kind]),'text-reader-callout-title'));const p=node('p');inline(p,block.text);quote.append(p);host.append(quote)}
   else if(block.type==='table'){const scroll=node('div',undefined,'text-reader-table');scroll.tabIndex=0;scroll.setAttribute('role','region');scroll.setAttribute('aria-label',tr('Tableau du document'));const table=node('table'),head=node('thead'),row=node('tr');for(const value of block.headers){const th=node('th');th.scope='col';inline(th,value);row.append(th)}head.append(row);table.append(head);const body=node('tbody');for(const values of block.rows){const r=node('tr');for(const value of values){const cell=node('td');inline(cell,value);r.append(cell)}body.append(r)}table.append(body);scroll.append(table);host.append(scroll)}
  }
 }
 function render(source,options={}){
  const text=String(source??''),id='swarm-reading-'+(++sequence),box=node('div',undefined,'text-reader');box.dataset.reader=id;box.dataset.readerKey=options.stateKey||id;
  const state=options.stateKey?(viewStates.get(options.stateKey)||{source:false,outline:false}):{source:false,outline:false};if(options.stateKey)viewStates.set(options.stateKey,state);
  const tools=node('div',undefined,'text-reader-tools');tools.setAttribute('role','group');tools.setAttribute('aria-label',tr('Présentation du texte'));
  const reading=node('button',tr('Lecture structurée')),raw=node('button',tr('Texte source')),copy=node('button',tr('Copier le texte source'));
  for(const button of [reading,raw,copy])button.type='button';
  reading.dataset.readerAction='reading';raw.dataset.readerAction='source';copy.dataset.readerAction='copy';
  const prose=node('div',undefined,'text-reader-prose');prose.id=id+'-reading';const original=node('pre',text,'text-reader-source');original.id=id+'-source';original.tabIndex=0;original.hidden=true;
  reading.setAttribute('aria-controls',prose.id);raw.setAttribute('aria-controls',original.id);
  function mode(sourceMode){state.source=sourceMode;prose.hidden=sourceMode;original.hidden=!sourceMode;reading.setAttribute('aria-pressed',String(!sourceMode));raw.setAttribute('aria-pressed',String(sourceMode));if(outline)outline.hidden=sourceMode;box.dataset.mode=sourceMode?'source':'reading'}
  const headings=[];content(parse(text),prose,headings,id);
  if(!text.trim())prose.append(node('p',tr('Aucun texte à afficher.'),'text-reader-empty'));
  let outline=null;if(headings.length){outline=node('details',undefined,'text-reader-outline');outline.open=state.outline;outline.addEventListener('toggle',()=>{if(outline.isConnected)state.outline=outline.open});outline.append(node('summary',tr('Sommaire du texte')+' · '+headings.length));const nav=node('nav');nav.setAttribute('aria-label',tr('Sections du texte'));for(const heading of headings){const link=node('a',heading.text);link.href='#'+heading.element.id;link.addEventListener('click',event=>{event.preventDefault();heading.element.scrollIntoView({block:'start',behavior:'auto'});heading.element.focus({preventScroll:true})});nav.append(link)}outline.append(nav)}
  const status=node('span','', 'text-reader-copy-state');status.setAttribute('role','status');
  copy.disabled=!text;copy.addEventListener('click',async()=>{try{if(!navigator.clipboard?.writeText)throw Error();await navigator.clipboard.writeText(text);status.textContent=tr('Texte source copié.')}catch{mode(true);status.textContent=tr('Copie indisponible : sélectionnez le texte source.');original.focus()}});
  reading.addEventListener('click',()=>mode(false));raw.addEventListener('click',()=>mode(true));
  const meta=node('span',tr('{lines} lignes · {characters} caractères',{lines:text?text.split(/\r\n?|\n/).length:0,characters:[...text].length}),'text-reader-meta');
  tools.append(reading,raw,copy,meta,status);box.append(tools);if(outline)box.append(outline);box.append(prose,original);mode(state.source);
  if(options.className)box.classList.add(options.className);return box;
 }
 function attach(input){
  const preview=node('details',undefined,'text-preview field-wide');preview.append(node('summary',tr('Aperçu de lecture')));const host=node('div');preview.append(host);let previous=null;
  const update=()=>{if(preview.open&&previous!==input.value){previous=input.value;host.replaceChildren(render(input.value,{stateKey:'field:'+input.id}))}};
  preview.addEventListener('toggle',update);input.addEventListener('input',update);input.closest('label').after(preview);return preview;
 }
 const api={parse,render,attach};root.SwarmTextReader=api;if(typeof module==='object')module.exports=api;
})(globalThis);
