const tr = source => globalThis.SwarmI18n?.t(source) ?? source;
const ns='http://www.w3.org/2000/svg';
const svg=(tag,attributes={})=>{const e=document.createElementNS(ns,tag);for(const [k,v] of Object.entries(attributes))e.setAttribute(k,String(v));return e};

export class PreparationWorkGraph {
 constructor($){this.$=$;this.$('template-work-task').addEventListener('change',()=>this.inspect(this.$('template-work-task').value));this.resize=new ResizeObserver(()=>this.center());this.resize.observe(this.$('template-work-graph'))}
 paint(plan,decisions){
  this.plan=plan;this.$('template-work-task').replaceChildren(...plan.tasks.map(t=>new Option(t.id+' · '+t.title,t.id)));
  this.$('template-work-decisions').replaceChildren(...decisions.map(text=>{const li=document.createElement('li');li.textContent=text;return li}));
  const host=this.$('template-work-graph');host.replaceChildren();
  if(globalThis.dagre){
   const g=new dagre.graphlib.Graph();g.setGraph({rankdir:'TB',nodesep:28,ranksep:48,marginx:24,marginy:24});g.setDefaultEdgeLabel(()=>({}));
   for(const t of plan.tasks)g.setNode(t.id,{width:220,height:94});
   for(const t of plan.tasks)for(const dep of t.depends)g.setEdge(dep,t.id);
   dagre.layout(g);const root=svg('svg',{viewBox:`0 0 ${g.graph().width} ${g.graph().height}`,width:g.graph().width,height:g.graph().height,role:'group','aria-label':tr('Graphe du modèle')});
   const defs=svg('defs'),marker=svg('marker',{id:'template-arrow',viewBox:'0 0 10 10',refX:9,refY:5,markerWidth:7,markerHeight:7,orient:'auto-start-reverse'});marker.append(svg('path',{d:'M 0 0 L 10 5 L 0 10 z',class:'template-edge-head'}));defs.append(marker);root.append(defs);
   for(const edge of g.edges()){const points=g.edge(edge).points;root.append(svg('path',{d:points.map((p,i)=>(i?'L':'M')+p.x+' '+p.y).join(' '),class:'template-edge','marker-end':'url(#template-arrow)'}))}
   for(const t of plan.tasks){const n=g.node(t.id),group=svg('g',{transform:`translate(${n.x-110},${n.y-47})`,class:'template-work-node',role:'button',tabindex:0,'aria-label':t.id+' · '+t.title,'data-task-id':t.id});
    group.append(svg('rect',{width:220,height:94,rx:6}));const lines=[t.id,...this.wrap(t.title,26)];lines.slice(0,4).forEach((line,i)=>{const text=svg('text',{x:14,y:20+i*21});text.textContent=line;group.append(text)});
    group.addEventListener('click',()=>this.select(t.id));group.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();this.select(t.id)}});root.append(group)
   }
   host.append(root);
  }
  this.inspect(plan.tasks[0].id);
 }
 wrap(text,width){const lines=[];let line='';for(const word of text.split(' ')){if(line.length+word.length+1>width){lines.push(line);line=''}line+=(line?' ':'')+word}if(line)lines.push(line);return lines}
 select(id){this.$('template-work-task').value=id;this.inspect(id)}
 center(){const host=this.$('template-work-graph'),node=host.querySelector('[aria-pressed=true]');if(!node||!host.clientWidth)return;const b=node.getBoundingClientRect(),h=host.getBoundingClientRect();host.scrollLeft+=b.left+b.width/2-h.left-host.clientWidth/2;host.scrollTop+=b.top+b.height/2-h.top-host.clientHeight/2}
 inspect(id){
  const t=this.plan.tasks.find(t=>t.id===id);if(!t)return;
  const box=this.$('template-work-inspector');box.replaceChildren();const title=document.createElement('h4');title.textContent=t.id+' · '+t.title;box.append(title);
  const list=document.createElement('dl');for(const [label,value] of [[tr('Périmètre'),t.scope],[tr('Prérequis'),t.depends.join(', ')||tr('Aucun prérequis dans cet incrément')],[tr('Livrable'),t.deliverable],[tr('Critères et contrôles'),t.criteria.join('\n')],[tr('Preuves'),t.proof],[tr('Validation'),t.delivery],[tr('Arrêt et reprise'),t.stop],[tr('Limites proposées'),t.max_attempts+' / '+t.max_tool_calls+' · '+tr('tentatives / appels outils, à adopter')]]){const term=document.createElement('dt'),detail=document.createElement('dd');term.textContent=label;detail.textContent=value;list.append(term,detail)}box.append(list);
  for(const node of this.$('template-work-graph').querySelectorAll('[data-task-id]'))node.setAttribute('aria-pressed',String(node.dataset.taskId===id));
  this.center();
 }
}
