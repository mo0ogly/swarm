'use strict';
const tr_web_graph_studio_js=source=>globalThis.SwarmI18n?.t(source)??source;
// Presentation coordinates never enter the engine's task graph. Dependency edits
// are batches in GraphDraft and still require engine preview + explicit apply.
const GraphStudio={
 key:'',positions:Object.create(null),selected:null,connection:null,gesture:null,
 mount(){
  if($('graph-studio'))return;
  const studio=node('section',undefined,'graph-studio');studio.id='graph-studio';
  $('pilot-toolbar').before(studio);
  for(const id of ['pilot-toolbar','pilot-actions','pilot-status','graph-draft-panel','pilot-canvas','pilot-list'])studio.append($(id));
  const tools=document.querySelector('.graph-tools');studio.insertBefore(tools,$('pilot-status'));
  const full=GraphDraft.button('pilot-fullscreen',tr_web_graph_studio_js('Plein écran'),()=>this.fullscreen());full.setAttribute('aria-expanded','false');$('pilot-actions').append(full);
  const reset=GraphDraft.button('pilot-layout-reset',tr_web_graph_studio_js('Ranger les cartes'),()=>{this.positions=Object.create(null);this.persist();this.update()});$('pilot-actions').append(reset);
  const selection=node('div',undefined,'graph-link-toolbar');selection.id='graph-link-toolbar';selection.hidden=true;
  const label=node('span');label.id='graph-link-label';selection.append(label,GraphDraft.button('graph-link-delete',tr_web_graph_studio_js('Retirer ce lien'),()=>this.remove()),GraphDraft.button('graph-link-source',tr_web_graph_studio_js('Changer le prérequis'),()=>this.connect(this.selected.to,'in',this.selected)),GraphDraft.button('graph-link-target',tr_web_graph_studio_js('Changer la destination'),()=>this.connect(this.selected.from,'out',this.selected)));
  studio.insertBefore(selection,$('pilot-canvas'));
  const hint=node('p',tr_web_graph_studio_js('Déplacez les cartes par leur poignée. Reliez une sortie à une entrée. Sélectionnez un lien pour le modifier. Échap annule le geste ou quitte le plein écran.'),'graph-studio-hint');hint.id='graph-studio-hint';hint.setAttribute('role','status');studio.insertBefore(hint,$('pilot-canvas'));
  studio.addEventListener('keydown',e=>{if(e.isTrusted&&e.key==='Tab'&&studio.classList.contains('is-fullscreen')){const controls=[...studio.querySelectorAll('button,input,select,textarea,[tabindex="0"]')].filter(n=>!n.disabled&&n.getClientRects().length&&!n.closest('[hidden],details:not([open])'));const first=controls[0],last=controls.at(-1);if(e.shiftKey&&document.activeElement===first){e.preventDefault();last?.focus()}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus()}}if(!e.isTrusted||e.key!=='Escape')return;e.preventDefault();if(this.gesture)this.cancel();else if(this.connection){this.connection=null;this.update()}else if(studio.classList.contains('is-fullscreen'))this.fullscreen(false)});
 },
 fullscreen(value=!$('graph-studio').classList.contains('is-fullscreen')){
  const studio=$('graph-studio');studio.classList.toggle('is-fullscreen',value);document.body.classList.toggle('graph-studio-open',value);$('pilot-fullscreen').textContent=tr_web_graph_studio_js(value?'Quitter le plein écran':'Plein écran');$('pilot-fullscreen').setAttribute('aria-expanded',String(value));$('pilot-fullscreen').focus();
 },
 restore(){const key=Pilot.key+':positions:'+Pilot.state.orientation;if(key===this.key)return;this.cancel();this.key=key;this.selected=null;this.connection=null;this.positions=Object.create(null);
  try{const data=JSON.parse(localStorage.getItem(key)||'{}');for(const [id,p]of Object.entries(data))if(p&&Number.isFinite(p.x)&&Number.isFinite(p.y))this.positions[id]={x:p.x,y:p.y}}catch{this.message(tr_web_graph_studio_js('Positions indisponibles ; placement automatique conservé.'))}
 },
 persist(){try{localStorage.setItem(this.key,JSON.stringify(this.positions))}catch{this.message(tr_web_graph_studio_js('Positions non mémorisées sur cet appareil.'))}},
 message(text){$('graph-studio-hint').textContent=text},
 help(){this.message(tr_web_graph_studio_js('Déplacez les cartes par leur poignée. Reliez une sortie à une entrée. Sélectionnez un lien pour le modifier. Échap annule le geste ou quitte le plein écran.'))},
 render(svg){if(!svg)return;this.restore();this.svg=svg;
  if(!svg.dataset.studio){svg.dataset.studio='true';this.units=new Map();const controls=new Map();for(const child of svg.querySelectorAll('.graph-go,.graph-fold')){const list=controls.get(child.dataset.task)||[];list.push(child);controls.set(child.dataset.task,list)}
   for(const main of [...svg.querySelectorAll('.graph-noeud,.graph-responsibility')]){
    const id=main.dataset.task||main.dataset.responsibility,rect=main.querySelector('rect'),unit=svgNode('g',{class:'graph-unit'});unit.dataset.node=id;if(main.dataset.task)unit.dataset.task=id;
    unit.dataset.x=rect.getAttribute('x');unit.dataset.y=rect.getAttribute('y');unit.dataset.width=rect.getAttribute('width');unit.dataset.height=rect.getAttribute('height');
    main.before(unit);unit.append(main);this.units.set(id,unit);for(const child of controls.get(id)||[])unit.append(child);
    const x=Number(unit.dataset.x),y=Number(unit.dataset.y),w=Number(unit.dataset.width),h=Number(unit.dataset.height);
    const handle=svgNode('g',{class:'graph-drag',role:'button',tabindex:0,'aria-label':tr_web_graph_studio_js('Déplacer la carte')+' '+id});handle.dataset.task=id;handle.append(svgNode('rect',{x:x+w-38,y:y+6,width:30,height:30,rx:6}),svgNode('text',{x:x+w-32,y:y+27},'⠿'));unit.append(handle);
    handle.addEventListener('keydown',e=>{if(!e.isTrusted||!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key))return;e.preventDefault();e.stopPropagation();const p=this.positions[id]||{x:0,y:0};this.positions[id]={x:Math.max(-x,p.x+(e.key==='ArrowLeft'?-16:e.key==='ArrowRight'?16:0)),y:Math.max(-y,p.y+(e.key==='ArrowUp'?-16:e.key==='ArrowDown'?16:0))};this.update();this.persist()});
    handle.addEventListener('pointerdown',e=>this.drag(e,id,handle));
    if(main.dataset.task)for(const side of ['in','out']){const vertical=Pilot.state.orientation==='TB',port=svgNode('circle',{class:'graph-port',role:'button',tabindex:0,r:9,cx:vertical?x+w/2:side==='in'?x:x+w,cy:vertical?(side==='in'?y:y+h):y+h/2,'aria-label':tr_web_graph_studio_js(side==='out'?'Relier cette sortie':'Relier cette entrée')+' '+id});port.dataset.task=id;port.dataset.side=side;
     port.addEventListener('keydown',e=>{if(e.isTrusted&&['Enter',' '].includes(e.key)){e.preventDefault();e.stopPropagation();this.port(id,side)}});port.addEventListener('click',e=>{if(e.isTrusted){e.stopPropagation();this.port(id,side)}});port.addEventListener('pointerdown',e=>this.linkDrag(e,id,side,port));unit.append(port)}
   }
   for(const edge of svg.querySelectorAll('.graph-arete')){edge.setAttribute('tabindex','0');edge.setAttribute('role','button');edge.setAttribute('aria-label',edge.dataset.from+' → '+edge.dataset.to);edge.addEventListener('click',e=>{if(e.isTrusted){e.stopPropagation();this.select(edge)}});edge.addEventListener('keydown',e=>{if(!e.isTrusted)return;if(['Enter',' '].includes(e.key)){e.preventDefault();this.select(edge)}else if(['Delete','Backspace'].includes(e.key)){e.preventDefault();this.select(edge);this.remove().catch(()=>{})}})}
  }this.update();
 },
 bounds(id){const unit=this.units?.get(id);if(!unit)return null;const p=this.positions[id]||{x:0,y:0};return {x:Number(unit.dataset.x)+p.x,y:Number(unit.dataset.y)+p.y,w:Number(unit.dataset.width),h:Number(unit.dataset.height)}},
 endpoint(id,side){const b=this.bounds(id);if(!b)return null;return Pilot.state.orientation==='TB'?{x:b.x+b.w/2,y:side==='in'?b.y-12:b.y+b.h+12}:{x:side==='in'?b.x-12:b.x+b.w+12,y:b.y+b.h/2}},
 curve(a,b){const vertical=Pilot.state.orientation==='TB',distance=Math.max(60,Math.abs(vertical?b.y-a.y:b.x-a.x)/2);return vertical?`M${a.x},${a.y} C${a.x},${a.y+distance} ${b.x},${b.y-distance} ${b.x},${b.y}`:`M${a.x},${a.y} C${a.x+distance},${a.y} ${b.x-distance},${b.y} ${b.x},${b.y}`},
 update(){if(!this.svg?.isConnected)return;
  let width=Number(this.svg.dataset.width),height=Number(this.svg.dataset.height);
  for(const unit of this.svg.querySelectorAll('.graph-unit')){const p=this.positions[unit.dataset.node]||{x:0,y:0};unit.setAttribute('transform',`translate(${p.x} ${p.y})`);const b=this.bounds(unit.dataset.node);width=Math.max(width,b.x+b.w+30);height=Math.max(height,b.y+b.h+30)}
  this.svg.setAttribute('viewBox',`0 0 ${width} ${height}`);this.svg.setAttribute('width',width*Pilot.state.zoom);this.svg.setAttribute('height',height*Pilot.state.zoom);
  for(const edge of this.svg.querySelectorAll('.graph-arete,.graph-organisation-link')){const a=this.endpoint(edge.dataset.from,'out'),b=this.endpoint(edge.dataset.to,'in');if(a&&b)edge.setAttribute('d',this.curve(a,b));edge.classList.toggle('is-selected',!!this.selected&&edge.dataset.from===this.selected.from&&edge.dataset.to===this.selected.to)}
  const selected=this.selected&&[...this.svg.querySelectorAll('.graph-arete')].some(e=>e.dataset.from===this.selected.from&&e.dataset.to===this.selected.to);$('graph-link-toolbar').hidden=!selected;if(selected)$('graph-link-label').textContent=this.selected.from+' → '+this.selected.to;
  this.svg.querySelector('.graph-link-ghost')?.remove();const mapKey=JSON.stringify([this.key,width,height,this.positions]);if(this.mapSVG!==this.svg||this.mapKey!==mapKey){this.mapSVG=this.svg;this.mapKey=mapKey;GraphDraft.minimapSource=null}GraphDraft.minimap(this.svg);
 },
 point(e){const p=this.svg.createSVGPoint();p.x=e.clientX;p.y=e.clientY;return p.matrixTransform(this.svg.getScreenCTM().inverse())},
 drag(e,id,handle){if(!e.isTrusted||e.button!==0)return;e.preventDefault();e.stopPropagation();handle.focus();const origin={...(this.positions[id]||{x:0,y:0})},start=this.point(e),b=this.bounds(id);this.gesture={id,origin,handle,pointer:e.pointerId};handle.setPointerCapture(e.pointerId);
  handle.onpointermove=event=>{if(!this.gesture)return;const p=this.point(event);this.positions[id]={x:Math.max(origin.x-b.x,origin.x+p.x-start.x),y:Math.max(origin.y-b.y,origin.y+p.y-start.y)};this.update()};handle.onpointerup=()=>{this.finish();this.persist()};handle.onpointercancel=()=>this.cancel();
 },
 finish(){const g=this.gesture;if(!g)return;g.handle.onpointermove=null;g.handle.onpointerup=null;g.handle.onpointercancel=null;if(g.handle.hasPointerCapture(g.pointer))g.handle.releasePointerCapture(g.pointer);this.gesture=null},
 cancel(){const g=this.gesture;if(g?.id)this.positions[g.id]=g.origin;this.finish();this.connection=null;this.update()},
 linkDrag(e,id,side,port){if(!e.isTrusted||e.button!==0)return;e.stopPropagation();e.preventDefault();port.focus();const start=this.point(e);this.gesture={handle:port,pointer:e.pointerId};port.setPointerCapture(e.pointerId);let moved=false;
  port.onpointermove=event=>{const p=this.point(event);moved=Math.hypot(p.x-start.x,p.y-start.y)>4;this.svg.querySelector('.graph-link-ghost')?.remove();if(moved){const a=this.endpoint(id,side),ghost=svgNode('path',{class:'graph-link-ghost',d:side==='out'?this.curve(a,p):this.curve(p,a)});this.svg.append(ghost)}};
  port.onpointerup=event=>{const target=document.elementFromPoint(event.clientX,event.clientY)?.closest('.graph-port,.graph-unit'),other=target?.dataset.task;this.finish();this.update();if(moved){this.suppressClick=true;setTimeout(()=>{this.suppressClick=false},0);if(other&&other!==id)this.propose(side==='out'?id:other,side==='out'?other:id).catch(()=>{})}};port.onpointercancel=()=>this.cancel();
 },
 connect(id,side,replacement=null){this.connection={id,side,replacement};this.message(tr_web_graph_studio_js('Choisissez l’autre carte pour préparer le lien. Aucune modification avant confirmation.'))},
 port(id,side){if(this.suppressClick)return;const current=this.connection;if(current&&current.id!==id&&current.side!==side){this.connection=null;this.propose(current.side==='out'?current.id:id,current.side==='out'?id:current.id,current.replacement).catch(()=>{})}else this.connect(id,side)},
 select(edge){this.selected={from:edge.dataset.from,to:edge.dataset.to};this.connection=null;this.update()},
 async propose(from,to,replacement=null){if(GraphDraft.pending)return;if(!GraphDraft.editing)GraphDraft.open();const operations=GraphDraft.current();if(replacement)operations.push({kind:'remove_dependency',prerequisite:replacement.from,dependent:replacement.to});operations.push({kind:'add_dependency',prerequisite:from,dependent:to});await GraphDraft.commit(operations);this.message(tr_web_graph_studio_js('Lien préparé. Prévisualisez puis confirmez dans le brouillon.'))},
 async remove(){if(!this.selected||GraphDraft.pending)return;if(!GraphDraft.editing)GraphDraft.open();await GraphDraft.commit([...GraphDraft.current(),{kind:'remove_dependency',prerequisite:this.selected.from,dependent:this.selected.to}])}
};
globalThis.GraphStudio=GraphStudio;
