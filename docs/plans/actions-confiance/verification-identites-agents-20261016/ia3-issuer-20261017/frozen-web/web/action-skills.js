'use strict';
const ProjectSkills={
 tr(text){return globalThis.SwarmI18n?.t(text)??text},
 selected(fields){
  if(modalContext?.skillLoading)throw new Error(this.tr('Attendez le chargement des skills du projet.'));
  if(modalContext?.skillError)throw new Error(this.tr('La liste des skills est indisponible. Rouvrez cette action pour réessayer.'));
  if(fields.role && fields.role!=='worker')return [];
  return JSON.parse(fields.skills||'[]');
 },
 async mount(context){
  const host=node('fieldset',undefined,'action-skills field-wide');host.id='action-skills';
  host.append(node('legend',this.tr('Skills pour cette tâche')));
  host.append(node('p',this.tr('Choisissez les méthodes utiles à cette action. Aucun script ni permission supplémentaire n’est activé.')));
  const state=node('p',this.tr('Chargement des skills…'),'notice info');state.setAttribute('role','status');host.append(state);
  const input=node('input');input.type='hidden';input.name='skills';input.value='[]';host.append(input);
  $('modal-fields').append(host);context.skillLoading=true;context.skillError=false;
  try{
   const data=await api('/api/v1/skills');if(modalContext!==context||!host.isConnected)return;
   const prior=context.action==='retry'?snapshot.agents.find(x=>x.agent.id===$('field-agent')?.value)?.agent?.workflow?.skills:context.data?.task?.launch_profile?.skills;
   const selected=new Set((prior||[]).map(s=>s.path));
   const items=data.skills||[];const boxes=[];
   for(const item of items){
    const label=node('label',undefined,'action-skill-option'),box=node('input');box.type='checkbox';box.dataset.skillPath=item.path;box.disabled=!item.available;box.checked=item.available&&selected.has(item.path);
    const text=node('span');text.append(node('strong',item.name),node('small',item.description||this.tr('Description non fournie.')),node('small',item.path));
    if(!item.available)text.append(node('small',globalThis.SwarmI18n?.engine(item.reason)||item.reason,'notice attention'));
    label.append(box,text);host.append(label);boxes.push({box,item});
   }
   const update=()=>{
    const chosen=boxes.filter(v=>v.box.checked).map(v=>({path:v.item.path,sha256:v.item.sha256}));
    input.value=JSON.stringify(chosen);state.textContent=items.length?this.tr('Skills sélectionnés : ')+chosen.length+' / 8':this.tr('Aucun skill trouvé dans .claude/skills ou .agents/skills.');
    for(const {box,item} of boxes)box.disabled=!item.available||(!box.checked&&chosen.length>=8);
    input.dispatchEvent(new Event('input',{bubbles:true}));
   };
   for(const {box} of boxes)box.addEventListener('change',update);
   $('field-agent')?.addEventListener('change',()=>{const prior=snapshot.agents.find(x=>x.agent.id===$('field-agent').value)?.agent?.workflow?.skills||[];const paths=new Set(prior.map(s=>s.path));for(const {box,item} of boxes)box.checked=item.available&&paths.has(item.path);update()});
   context.skillLoading=false;update();
   const role=$('field-role');const syncRole=()=>{host.hidden=!!role&&role.value!=='worker'};role?.addEventListener('change',syncRole);syncRole();
  }catch(error){if(modalContext!==context||!host.isConnected)return;context.skillLoading=false;context.skillError=true;state.className='notice alert';state.textContent=error.message;}
 }
};
globalThis.ProjectSkills=ProjectSkills;
