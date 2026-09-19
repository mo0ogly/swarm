export class PreparationConversion {
 constructor(api,current,committed){
  this.api=api;this.current=current;this.committed=committed;this.pending=null;this.busy=false;this.routeGeneration=0;
  this.dialog=document.getElementById('conversion-dialog');this.confirm=document.getElementById('conversion-confirm');this.error=document.getElementById('conversion-error');
  document.getElementById('conversion-open').addEventListener('click',e=>{if(e.isTrusted)this.open()});
  this.confirm.addEventListener('click',e=>{if(e.isTrusted)this.submit()});
 for(const id of ['organization-provider','organization-level'])document.getElementById(id)?.addEventListener('change',()=>this.resolve());
 document.getElementById('organization-validation')?.addEventListener('change',()=>{document.getElementById('organization-checks').hidden=document.getElementById('organization-validation').value!=='automatic';});
 }
 hasPending(){return !!this.pending}
 paint(p){
  const box=document.getElementById('conversion-state'),button=document.getElementById('conversion-open'),link=document.getElementById('conversion-pilotage'),status=document.getElementById('conversion-status');
  box.hidden=!p.plan_ready&&!p.conversion;
  if(p.conversion){
   const c=p.conversion;link.hidden=false;link.href='/?work='+encodeURIComponent(c.work_id);
   status.textContent=c.task_ids.length+' missions créées. '+(c.released_at?'Démarrage autorisé ; le moteur contrôle encore les profils, dépendances, budget, pause et autonomie.':'Démarrage verrouillé : autorisez-le quand vous êtes prêt.');
   const revised=p.plan_ready&&p.documents.plan.sha256!==c.plan_hash;button.hidden=!!c.released_at&&!revised;button.textContent=revised?'Comparer et appliquer la révision':'Relire et autoriser le démarrage';
  }else{link.hidden=true;button.hidden=false;button.textContent='Créer les missions';status.textContent='Plan vérifié. Relisez les missions avant de les créer dans le pilotage.'}
 }
 async open(){
  try{
   const p=await this.current();if(!p)return;
   if(this.pending){this.dialog.showModal();return}
   const review=await this.api('preparations/conversion?id='+encodeURIComponent(p.id));
   if(review.preparation.revision!==p.revision)throw new Error('Préparation modifiée : actualisez les versions avant de relire les missions.');
   this.review=review;this.error.textContent='';const c=p.conversion;
   const revising=review.action==='revise-missions';document.getElementById('conversion-organization').hidden=!!c;document.getElementById('conversion-changes').hidden=!revising;const changes=document.getElementById('conversion-change-list');changes.replaceChildren();for(const change of review.changes||[]){const li=document.createElement('li');li.textContent=change.title+' : '+change.kind;changes.append(li)}document.getElementById('conversion-warning').textContent=review.warning||'';
   document.getElementById('conversion-title').textContent=revising?'Réviser ce Swarm':c?'Autoriser les missions créées':'Créer les missions du plan';
   document.getElementById('conversion-target').textContent=(p.work_id?'Travail existant : ':'Nouveau travail : ')+review.work_title;
   document.getElementById('conversion-explanation').textContent=revising?'La version relue remplacera les définitions des missions. Les résultats concernés seront à revérifier. Le démarrage sera verrouillé et la mission mise en pause.':c?'Vous autorisez le plan créé, présenté ci-dessous, même si le brouillon a évolué depuis. En mode automatique, les missions éligibles pourront démarrer. Profils, dépendances, budget et pause restent applicables.':'Ces missions et leurs dépendances seront créées ensemble. Leur démarrage restera verrouillé jusqu’à votre autorisation séparée. Aucun résultat ne sera considéré comme validé.';
   const list=document.getElementById('conversion-missions');list.replaceChildren();
   for(const m of review.spec.tasks){const item=document.createElement('li'),title=document.createElement('strong'),text=document.createElement('p');title.textContent=m.id+' — '+m.title;text.textContent='Rôle : '+({worker:'Exécutant',planner:'Planificateur',subplanner:'Planificateur de branche'}[m.role]||m.role)+' · Livrable : '+m.deliverable+' · Dépendances : '+(m.depends.join(', ')||'aucune')+' · Critères : '+m.criteria.join(' ; ');item.append(title,text);list.append(item)}
   this.confirm.textContent=revising?'Appliquer la révision et mettre en pause':c?'Autoriser le démarrage des missions':'Créer l’équipe et ses missions';this.confirm.disabled=false;this.dialog.showModal();if(!c)await this.setupOrganization();
  }catch(e){document.getElementById('error').textContent=e.message;document.getElementById('error').hidden=false}
 }
 async submit(){
  if(this.busy||!this.review)return;
 if(!this.review.preparation.conversion&&!this.pending){try{this.organization=this.readOrganization()}catch(e){this.error.textContent=e.message;return}}
  const p=this.review.preparation,c=p.conversion;
  this.pending ||= {version:1,action:this.review.action||(c?'release-plan':'create-missions'),preparation_id:p.id,event_id:crypto.randomUUID(),expected_revision:p.revision,expected_work_revision:this.review.work_revision,sha256:this.review.action==='revise-missions'?p.documents.plan.sha256:c?c.plan_hash:p.documents.plan.sha256,...(!c?{organization:this.organization}:{})};
  this.busy=true;this.confirm.disabled=true;
  try{
   let result=await this.api('preparations/command',this.pending);this.pending=null;
   if(result.receipt_historical)result=await this.api('preparations/show?id='+encodeURIComponent(p.id));
   this.committed(result);this.dialog.close();document.getElementById('conversion-pilotage').focus();
  }catch(e){if(e.status&&e.status<500)this.pending=null;this.error.textContent=e.message;this.confirm.textContent=this.pending?'Vérifier l’enregistrement':'Réessayer après relecture';if(!this.pending){this.confirm.disabled=true;this.review=null}}
  finally{this.busy=false;if(this.review)this.confirm.disabled=false}
 }
 async setupOrganization(){
  const data=await this.api('preparations/providers'),select=document.getElementById('organization-provider');select.replaceChildren(new Option('Choisir une IA…',''));for(const p of data.providers){const o=new Option(p.provider,p.provider);o.disabled=!p.available;select.add(o)}const workers=document.getElementById('organization-worker');workers.replaceChildren(new Option('Même fournisseur si ses outils le permettent',''));for(const p of data.providers.filter(p=>p.available&&!p.text_only))workers.add(new Option(p.provider,p.provider));const chosen=document.getElementById('chat-provider')?.value;if(chosen)select.value=chosen;
  document.getElementById('organization-workspace').value='.';
  const host=document.getElementById('organization-checks');host.replaceChildren();const hint=document.createElement('p');hint.className='notice info';hint.textContent='Chaque contrôle est exécuté avec les arguments indiqués, au plus 30 secondes. Maximum huit contrôles par mission : trois étapes et jusqu’à cinq critères. Les commandes doivent réellement prouver le résultat.';host.append(hint);
  for(const task of this.review.spec.tasks){const box=document.createElement('section'),title=document.createElement('h4');title.textContent=task.title;box.append(title);
   const checks=[['plan-entry','Conditions de départ : '+task.entry,1],['plan-validation','Vérification : '+task.validation,1],['plan-delivery','Livraison : '+task.delivery,1],...task.criteria.map((text,i)=>['plan-criterion-'+(i+1),'Critère : '+text,i+1])];
   for(const [id,text,criterion] of checks){const group=document.createElement('fieldset'),legend=document.createElement('legend'),program=document.createElement('select'),args=document.createElement('textarea'),label=document.createElement('label'),programLabel=document.createElement('label');legend.textContent=text;program.id='organization-program-'+task.id+'-'+id;programLabel.htmlFor=program.id;programLabel.textContent='Programme du contrôle';for(const name of ['git','go','node','npm','python3','pytest'])program.add(new Option(name,name));args.id='organization-control-'+task.id+'-'+id;args.dataset.task=task.id;args.dataset.control=id;args.dataset.criterion=String(criterion);args.dataset.program=program.id;args.dataset.justification=text;args.rows=2;args.placeholder='Un argument par ligne, par exemple :\n-m\npytest\ntests/test_exemple.py';label.htmlFor=args.id;label.textContent='Arguments — un par ligne';group.append(legend,programLabel,program,label,args);box.append(group)}host.append(box)}

  await this.resolve();
 }
 async resolve(){const gen=++this.routeGeneration;this.route=null;this.confirm.disabled=true;const info=document.getElementById('organization-model');if(!document.getElementById('organization-provider').value){info.textContent='Choisissez une IA pour voir le modèle utilisé.';return}info.textContent='Résolution du modèle…';try{const data=await this.api('providers/resolve?'+new URLSearchParams({provider:document.getElementById('organization-provider').value,level:document.getElementById('organization-level').value,purpose:'planning'}));if(gen!==this.routeGeneration)return;this.route=data.route;info.textContent=data.route?data.route.model+(data.route.effort?' · effort '+data.route.effort:'')+' · '+data.route.level:'Modèle géré par l’exécutable';this.confirm.disabled=false;}catch(e){if(gen===this.routeGeneration)info.textContent=e.message}}
 readOrganization(){
  const value=id=>document.getElementById(id).value;const validation=value('organization-validation');if(!validation)throw Error('Choisissez qui accepte les résultats.');if(!this.route)throw Error('Choisissez une IA disponible.');const controls={};
  if(validation==='automatic')for(const input of document.querySelectorAll('#organization-checks textarea')){if(!input.value.trim())throw Error('Précisez les arguments de chaque contrôle.');const command=[document.getElementById(input.dataset.program).value,...input.value.split('\n').filter(v=>v.trim())];(controls[input.dataset.task]||=[]).push({id:input.dataset.control,command,criteria:[Number(input.dataset.criterion)],justification:input.dataset.justification.slice(0,500),dir:'.',timeout_seconds:30})}

  if(value('organization-provider').startsWith('api-')&&!value('organization-worker'))throw Error('Choisissez un agent avec outils pour les exécutants.');
  return {worker_provider:value('organization-worker'),provider:value('organization-provider'),level:value('organization-level'),model_policy_hash:this.route.policy_hash,workspace:value('organization-workspace'),validation,controls,max_tasks:Number(value('organization-tasks')),max_calls:Number(value('organization-calls'))};
 }

}
