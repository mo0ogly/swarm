export class PreparationConversion {
 constructor(api,current,committed){
  this.api=api;this.current=current;this.committed=committed;this.pending=null;this.busy=false;
  this.dialog=document.getElementById('conversion-dialog');this.confirm=document.getElementById('conversion-confirm');this.error=document.getElementById('conversion-error');
  document.getElementById('conversion-open').addEventListener('click',e=>{if(e.isTrusted)this.open()});
  this.confirm.addEventListener('click',e=>{if(e.isTrusted)this.submit()});
 }
 hasPending(){return !!this.pending}
 paint(p){
  const box=document.getElementById('conversion-state'),button=document.getElementById('conversion-open'),link=document.getElementById('conversion-pilotage'),status=document.getElementById('conversion-status');
  box.hidden=!p.plan_ready&&!p.conversion;
  if(p.conversion){
   const c=p.conversion;link.hidden=false;link.href='/?work='+encodeURIComponent(c.work_id);
   status.textContent=c.task_ids.length+' missions créées. '+(c.released_at?'Démarrage autorisé ; le moteur contrôle encore les profils, dépendances, budget, pause et autonomie.':'Démarrage verrouillé : autorisez-le quand vous êtes prêt.');
   button.hidden=!!c.released_at;button.textContent='Relire et autoriser le démarrage';
  }else{link.hidden=true;button.hidden=false;button.textContent='Créer les missions';status.textContent='Plan vérifié. Relisez les missions avant de les créer dans le pilotage.'}
 }
 async open(){
  try{
   const p=await this.current();if(!p)return;
   if(this.pending){this.dialog.showModal();return}
   const review=await this.api('preparations/conversion?id='+encodeURIComponent(p.id));
   if(review.preparation.revision!==p.revision)throw new Error('Préparation modifiée : actualisez les versions avant de relire les missions.');
   this.review=review;this.error.textContent='';const c=p.conversion;
   document.getElementById('conversion-title').textContent=c?'Autoriser les missions créées':'Créer les missions du plan';
   document.getElementById('conversion-target').textContent=(p.work_id?'Travail existant : ':'Nouveau travail : ')+review.work_title;
   document.getElementById('conversion-explanation').textContent=c?'Vous autorisez le plan créé, présenté ci-dessous, même si le brouillon a évolué depuis. En mode automatique, les missions éligibles pourront démarrer. Profils, dépendances, budget et pause restent applicables.':'Ces missions et leurs dépendances seront créées ensemble. Leur démarrage restera verrouillé jusqu’à votre autorisation séparée. Aucun résultat ne sera considéré comme validé.';
   const list=document.getElementById('conversion-missions');list.replaceChildren();
   for(const m of review.spec.tasks){const item=document.createElement('li'),title=document.createElement('strong'),text=document.createElement('p');title.textContent=m.id+' — '+m.title;text.textContent='Rôle : '+m.role+' · Livrable : '+m.deliverable+' · Dépendances : '+(m.depends.join(', ')||'aucune')+' · Critères : '+m.criteria.join(' ; ');item.append(title,text);list.append(item)}
   this.confirm.textContent=c?'Autoriser le démarrage des missions':'Créer les '+review.spec.tasks.length+' missions';this.confirm.disabled=false;this.dialog.showModal();
  }catch(e){document.getElementById('error').textContent=e.message;document.getElementById('error').hidden=false}
 }
 async submit(){
  if(this.busy||!this.review)return;
  const p=this.review.preparation,c=p.conversion;
  this.pending ||= {version:1,action:c?'release-plan':'create-missions',preparation_id:p.id,event_id:crypto.randomUUID(),expected_revision:p.revision,expected_work_revision:this.review.work_revision,sha256:c?c.plan_hash:p.documents.plan.sha256};
  this.busy=true;this.confirm.disabled=true;
  try{
   let result=await this.api('preparations/command',this.pending);this.pending=null;
   if(result.receipt_historical)result=await this.api('preparations/show?id='+encodeURIComponent(p.id));
   this.committed(result);this.dialog.close();document.getElementById('conversion-pilotage').focus();
  }catch(e){if(e.status&&e.status<500)this.pending=null;this.error.textContent=e.message;this.confirm.textContent=this.pending?'Vérifier l’enregistrement':'Réessayer après relecture';if(!this.pending){this.confirm.disabled=true;this.review=null}}
  finally{this.busy=false;if(this.review)this.confirm.disabled=false}
 }
}
