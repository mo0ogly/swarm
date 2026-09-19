'use strict';
const LifecycleManager={
 items:[],selected:'',loading:false,
 labels:{archive:'Archiver',restore:'Restaurer',purge:'Purger l’historique',delete:'Supprimer'},
 completionLabels:{archive:'Archivage terminé',restore:'Restauration terminée',purge:'Purge de l’historique terminée',delete:'Mise en corbeille terminée'},
 impactLabels:{works:'Mission',events:'Historique des modifications',agent_logs:'Journaux des agents',terminal_events:'Sorties du terminal',agent_dialogue_turns:'Dialogues des agents',cockpit_events:'Événements du pilotage',cockpit_controls:'Réglages du pilotage',cockpit_tasks:'Détails des tâches',decisions:'Décisions',session_visits:'Visites de session',budgets:'Budgets',reservations:'Réservations des agents',assist_turns:'Échanges avec l’assistant',assist_previews:'Aperçus de l’assistant',assist_reservations:'Réservations de l’assistant',mission_supervision:'Suivi du conducteur',preparations:'Préparations',preparation_documents:'Documents de préparation',preparation_commands:'Commandes de préparation',preparation_turns:'Échanges de préparation',preparation_reservations:'Réservations de préparation',preparation_launch_locks:'Verrous de préparation',mission_lifecycle:'État d’archivage',mission:'Mission',historique:'Historique des modifications',agents:'Tentatives des agents',journaux:'Journaux des agents',journaux_agents:'Journaux des agents',sorties_terminal:'Sorties du terminal',pilotage_et_recus:'Pilotage et reçus',assistant:'Échanges avec l’assistant',echanges_assistant:'Échanges avec l’assistant',preparations_liees:'Préparations liées'},
 async load(keep=''){
  if(this.loading)return;this.loading=true;
  try{this.items=await api('/api/v1/lifecycle/missions');const wanted=keep||this.selected;this.selected=this.items.some(x=>x.work_id===wanted)?wanted:'';this.render()}
  catch(e){$('manage-status').className='notice alert';$('manage-status').textContent='Liste indisponible : '+e.message}
  finally{this.loading=false}
 },
 filtered(){const q=$('manage-search').value.trim().toLocaleLowerCase('fr'),state=$('manage-state').value;return this.items.filter(x=>(state==='all'||x.state===state)&&(!q||(x.title+' '+x.work_id).toLocaleLowerCase('fr').includes(q)))},
 render(){
  const rows=this.filtered(),host=$('manage-list');host.replaceChildren();
  for(const item of rows){const b=node('button',undefined,'mission-manager-item');b.type='button';b.setAttribute('role','option');b.setAttribute('aria-selected',String(item.work_id===this.selected));b.dataset.mission=item.work_id;b.append(node('strong',item.title),node('span',item.work_id),node('span',item.state_label,'badge '+(item.state==='active'?'info':item.state==='archived'?'attention':'neutral')));b.addEventListener('click',()=>{this.selected=item.work_id;this.render();[...host.children].find(x=>x.dataset.mission===item.work_id)?.focus()});host.append(b)}
  const selectedVisible=rows.some(x=>x.work_id===this.selected);
  $('manage-status').className='notice '+(this.selected&&!selectedVisible?'attention':'info');$('manage-status').textContent=rows.length+' mission(s) affichée(s) sur '+this.items.length+'.'+(this.selected&&!selectedVisible?' La mission sélectionnée est masquée par les filtres.':'');
  const item=this.items.find(x=>x.work_id===this.selected),selection=$('manage-selection');selection.replaceChildren();
  if(!item){selection.append(node('h3','Sélectionnez une mission'),node('p','Les effets, volumes, exclusions et refus seront affichés dans un aperçu avant confirmation.'));return}
  selection.append(node('h3',item.title),node('p',item.state_label+' · '+item.work_id+' · révision '+item.revision));const actions=node('div',undefined,'mission-manager-actions');
  if(!selectedVisible){const hidden=node('p','Cette sélection est conservée mais masquée par la recherche ou le filtre d’état.','notice attention');const reveal=button('Afficher la mission sélectionnée',()=>this.revealSelected());reveal.dataset.manageReveal='true';selection.append(hidden,reveal)}
  const available=item.state==='trash'?['restore']:item.state==='archived'?['restore','purge','delete']:['archive','purge','delete'];
  for(const action of available){const b=button(this.labels[action],()=>this.open(item,action));b.dataset.lifecycleAction=action;if(action==='delete')b.className='danger-secondary';actions.append(b)}selection.append(actions);
  if(item.state==='trash')selection.append(node('p','Cette mission est récupérable : Restaurer remet toutes ses données internes en service.','notice info'));
 },
 revealSelected(){$('manage-search').value='';$('manage-state').value='all';this.render();[...$('manage-list').children].find(x=>x.dataset.mission===this.selected)?.focus()},
 open(item,action){
  const descriptions={archive:'Crée une archive ZIP exportable et range la mission sans supprimer ses données.',restore:'Réactive une mission archivée ou restaure toutes ses données depuis la corbeille.',purge:'Retire seulement les anciens journaux et sorties selon une durée choisie.',delete:'Place les données internes dans la corbeille récupérable. Aucun fichier du projet ne sera effacé.'};
  openModal(this.labels[action]+' — '+item.title,descriptions[action],{action:'lifecycle',lifecycleAction:action,target:item,workID:item.work_id,revision:item.revision});$('modal-context').textContent='Gestion des missions';
  if(action==='purge'){const retention=field('retention_days','Conserver les journaux des derniers jours','30');retention.type='number';retention.min='1';retention.max='36500';retention.required=true}
  if(action==='delete'){const name=field('mission_name','Saisissez le nom exact de la mission','',null,false,'Confirmation concrète : '+item.title);name.required=true;name.autocomplete='off'}
  for(const input of $('modal-fields').querySelectorAll('input,select,textarea'))input.addEventListener('input',()=>this.invalidate());
  $('confirm').textContent='Examiner l’aperçu';
 },
 invalidate(){if(!modalContext||modalContext.action!=='lifecycle'||!modalContext.lifecyclePreview)return;modalContext.lifecyclePreview=null;modalContext.lifecycleSignature='';$('preview').hidden=true;$('confirm').textContent='Examiner l’aperçu';$('modal-error').hidden=true},
 fields(c){const retention=Number($('field-retention_days')?.value||0),name=$('field-mission_name')?.value||'';return {retention_days:retention,mission_name:name,action:c.lifecycleAction}},
 previewText(result){const lines=['Mission : '+result.title+' ('+result.work_id+')','Action : '+this.labels[result.action],'Révision : '+result.revision,'','Données concernées :'];if(result.purge_before)lines.push('Date limite : '+new Date(result.purge_before).toLocaleString('fr-FR')+' (éléments strictement antérieurs uniquement)');for(const item of result.data)lines.push('- '+(this.impactLabels[item.kind]||'Données internes')+' : '+item.count+' élément(s), '+item.bytes+' octet(s)');lines.push('','Restent exclues :',...result.excluded.map(x=>'- '+x));if(result.blocked?.length)lines.push('','Action refusée actuellement :',...result.blocked.map(x=>'- '+x));lines.push('','Effet : '+result.confirmation);return lines.join('\n')},
 async submit(c){
  const fields=this.fields(c),item=c.target;
  if(c.lifecycleAction==='purge'&&(!Number.isInteger(fields.retention_days)||fields.retention_days<1||fields.retention_days>36500))throw Error('Choisissez une rétention comprise entre 1 et 36 500 jours.');
  if(c.lifecycleAction==='delete'&&fields.mission_name!==item.title)throw Error('Le nom ne correspond pas. Saisissez exactement « '+item.title+' ».');
  const signature=JSON.stringify(fields);
  if(!c.lifecyclePreview){const result=await api('/api/v1/action',{kind:'lifecycle-preview',work:item.work_id,expected_revision:item.revision,lifecycle_action:c.lifecycleAction,retention_days:fields.retention_days});c.lifecyclePreview=result;c.lifecycleSignature=signature;c.lifecycleEvent=crypto.randomUUID();preview(this.previewText(result));if(result.blocked?.length){$('modal-error').textContent='Action refusée : '+result.blocked.join(' ; ');$('modal-error').hidden=false;$('confirm').disabled=true;return}$('confirm').textContent=c.lifecycleAction==='delete'?'Confirmer la mise en corbeille':'Confirmer : '+this.labels[c.lifecycleAction];return}
  if(signature!==c.lifecycleSignature){this.invalidate();throw Error('Les champs ont changé. Examinez le nouvel aperçu avant de confirmer.')}
  const result=await api('/api/v1/action',{kind:'lifecycle-apply',work:item.work_id,expected_revision:item.revision,event_id:c.lifecycleEvent,lifecycle_action:c.lifecycleAction,retention_days:fields.retention_days,preview_token:c.lifecyclePreview.token});
  closeModal();await this.load(item.work_id);await this.syncWorks(item.work_id,c.lifecycleAction);notice(this.completionLabels[c.lifecycleAction]+' pour « '+result.title+' ». '+(result.restorable?'Une récupération reste disponible.':'Le reçu est conservé.'));[...$('manage-list').children].find(x=>x.dataset.mission===item.work_id)?.focus();
 },
 async syncWorks(target,action){
  const before=[...$('work').options].map(x=>x.value),old=work,works=await api('/api/v1/works');selectOptions($('work'),works.map(w=>[w.id,w.title]),old);
  if(action==='delete'&&old===target){const oldIndex=Math.max(0,before.indexOf(target)),next=works[Math.min(oldIndex,Math.max(0,works.length-1))];work=next?.id||'';$('work').value=work;if(work)await chooseWork();else{snapshot=null;stream?.close();stream=null;$('title').textContent='Aucune mission active';$('objective').textContent='Restaurez une mission depuis la corbeille ou préparez un nouveau projet.'}}
  else if(action==='restore'&&!old&&works.some(x=>x.id===target)){work=target;$('work').value=target;await chooseWork()}
 },
 help(){openModal('Aide — gérer et nettoyer les missions','Choisissez l’action selon l’effet recherché. Aucune action n’est exécutée depuis cette aide.',{action:'lifecycle-help'});$('confirm').hidden=true;$('cancel').textContent='Fermer l’aide';$('modal-help-toggle').hidden=true;$('modal-rich').hidden=false;const list=node('dl',undefined,'mission-manager-help');for(const [term,text]of [['Archiver','Crée un ZIP exportable et range la mission ; toutes ses données restent présentes.'],['Restaurer','Réactive une archive ou récupère une mission placée dans la corbeille.'],['Purger l’historique','Supprime uniquement les vieux journaux et sorties visés par la rétention ; preuves, reçus et fichiers restent exclus.'],['Supprimer','Place les données internes en corbeille récupérable. Le nom exact et un aperçu sont requis.']]){list.append(node('dt',term),node('dd',text))}$('modal-rich').append(list,node('p','Un agent, une intention, un conducteur, un dialogue ou un contrôle actif entraîne un refus expliqué. Fermez la modale, résolvez ce point, puis demandez un nouvel aperçu.','notice attention'))}
};
