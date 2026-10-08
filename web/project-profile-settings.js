'use strict';
const ProjectProfiles={
 catalog:null,
 tr(s){return globalThis.SwarmI18n?.t(s)??s},
 describe(workflow){
  const p=workflow?.project;
  if(!p)return {text:this.tr('Profil non enregistré pour cette tentative'),icon:'▤',family:'unknown',paths:[]};
  const paths=p.sources.map(s=>s.path),family=p.source?.startsWith('claude')||paths.some(p=>/(^|\/)CLAUDE\.md$/.test(p))?'claude':p.source?.startsWith('gemini')||paths.some(p=>/(^|\/)GEMINI\.md$/.test(p))?'gemini':'shared';
  return {text:this.tr('Profil transmis :')+' '+p.name+(p.source?' · '+({'claude-project':'.claude','claude-root':'CLAUDE.md','agents-root':'AGENTS.md','agents-directory':'.agents','gemini-root':'GEMINI.md','gemini-directory':'.gemini','custom':this.tr('Personnalisé')}[p.source]||p.source):''),icon:family==='claude'?'✳':family==='gemini'?'✦':'▤',family,paths,hash:p.sha256};
 },
 summary(workflow){const d=this.describe(workflow),p=workflow?.project;if(!p)return workflow?.skills?.length?'🧩 '+workflow.skills.map(s=>s.name).join(', '):d.icon+' '+d.text;const origin={'claude-project':'.claude','claude-root':'CLAUDE.md','agents-root':'AGENTS.md','agents-directory':'.agents','gemini-root':'GEMINI.md','gemini-directory':'.gemini','custom':this.tr('Personnalisé')}[p.source]||this.tr('Personnalisé');return d.icon+' '+origin+' · '+p.name+(workflow?.skills?.length?' · 🧩 '+workflow.skills.map(s=>s.name).join(', '):'')},
 badge(workflow){const d=this.describe(workflow),tag=document.createElement('span'),icon=document.createElement('span'),text=document.createElement('span');tag.className='profile-marker';tag.dataset.profileFamily=d.family;tag.dataset.profileHash=d.hash||'';icon.textContent=d.icon;icon.setAttribute('aria-hidden','true');text.textContent=d.text;tag.append(icon,text);tag.title=d.paths.join('\n')+(d.hash?'\nSHA-256 '+d.hash:'');if(workflow?.skills?.length){text.textContent+=' · 🧩 '+workflow.skills.map(s=>s.name).join(', ');tag.title+='\n'+workflow.skills.map(s=>s.path+' · SHA-256 '+s.sha256).join('\n')}return tag},
 async load(){
  const host=document.getElementById('project-profile-config');if(!host)return;
  host.replaceChildren();const title=node('h3',this.tr('Profil de consignes du projet'));host.append(title);
  try{
   const c=await api('/api/v1/project-profile/catalog');this.catalog=c;
   host.append(node('p',c.active_name?this.tr('Profil actif :')+' '+c.active_name+(c.active_source?' · '+(c.choices.find(v=>v.id===c.active_source)?.title||c.active_source):''):this.tr('Aucun profil de projet configuré.'),'notice info'));
   host.append(node('p',this.tr('Choisissez les consignes du dépôt. Les permissions, clés et réglages natifs ne sont pas importés.')));
   const label=node('label',this.tr('Choisir le profil du projet'));label.htmlFor='project-profile-choice';const select=node('select');select.id='project-profile-choice';select.append(new Option(this.tr('Choisir…'),''));
   for(const choice of c.choices){const option=new Option(this.tr(choice.title)+(choice.available?'':this.tr(' — indisponible')),choice.id);option.disabled=!choice.available;select.append(option)}
   const details=node('p',undefined,'hint');details.id='project-profile-choice-details';
   const choose=button(this.tr('Examiner et sélectionner ce profil'),()=>this.open(select.value));choose.id='project-profile-select';choose.disabled=true;
   select.addEventListener('change',()=>{const choice=c.choices.find(v=>v.id===select.value);choose.disabled=!choice?.available;details.textContent=choice?.available?choice.profile.instructions.map(f=>f.path).join(' · '):this.tr(choice?.reason||'Choisir un profil disponible.')});
   host.append(label,select,details,choose);
   const missing=c.choices.filter(v=>!v.available);if(missing.length){const d=node('details');d.append(node('summary',this.tr('Profils absents ou indisponibles')));for(const choice of missing)d.append(node('p',this.tr(choice.title)+' : '+(globalThis.SwarmI18n?.engine(choice.reason)||choice.reason)));host.append(d)}
  }catch(e){host.append(node('p',e.message,'notice alert'))}
 },
 open(id){const c=this.catalog,choice=c?.choices.find(v=>v.id===id);if(!choice?.available)return;openModal(this.tr('Sélectionner le profil du projet'),this.tr('Cette sélection prépare les prochains appels. Chaque tentative conserve le profil qui lui a été transmis.'),{action:'project-profile-select',profileID:id,profileDigest:c.active_sha256});preview(choice.profile.instructions.map(s=>s.path+' → '+s.roles.map(r=>this.tr({preparation:'Préparation',planner:'Planificateur',subplanner:'Sous-planificateur',worker:'Exécutant',reviewer:'Vérificateur'}[r]||r)).join(', ')).join('\n'));$('confirm').textContent=this.tr('Utiliser ce profil');},
 async submit(c){await api('/api/v1/project-profile/select',{id:c.profileID,expected_sha256:c.profileDigest});if(modalContext!==c)return;closeModal();await this.load();$('project-profile-choice')?.focus();notice(this.tr('Profil du projet enregistré. Aucun agent lancé.'))}
};
globalThis.ProjectProfiles=ProjectProfiles;
