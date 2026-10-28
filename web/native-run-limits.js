'use strict';
/** SQL override Admin: backend owns scope, policy, history and actual publication.
 * UI numbers are exact strings, zero inherits, no local execution-policy ceiling. */
(()=>{
 const names=['silence_seconds','tool_seconds','max_tool_calls','max_repeated_calls','max_consecutive_errors'];
 /** Known signed64 wire scalars, never a business ceiling. Metadata revisions
  * retain signed representation; override negativity is rejected by Root. */
 function integer(v){if(typeof v!=='string'||! /^-?(0|[1-9][0-9]*)$/.test(v)||BigInt(v)<-9223372036854775808n||BigInt(v)>9223372036854775807n)throw Error('run-limits-integer-invalid');return BigInt(v).toString()}
 /** Encode only protocol fields; token stays transport-only. No generic raw JSON. */
 function encode(r){
  const fields=['"scope":'+JSON.stringify(r.scope),'"mission_id":'+JSON.stringify(r.mission_id),'"scope_key":'+JSON.stringify(r.scope_key),'"event_id":'+JSON.stringify(r.event_id),'"reason":'+JSON.stringify(r.reason),'"expected_revision":'+integer(r.expected_revision),'"preview_token":'+JSON.stringify(r.preview_token??'')];
  if(Object.hasOwn(r,'to_revision'))fields.push('"to_revision":'+integer(r.to_revision));
  else fields.push('"schema_version":1','"values":{'+['observation_mode',...names].map(k=>JSON.stringify(k)+':'+integer(r.values[k])).join(',')+'}','"rollback_of":0');
  return '{'+fields.join(',')+'}';
 }
 /** Mount a read-only summary plus an explicit scope modal. Source reload,
  * preview and apply are separate actions; no automatic mutation or retry. */
 function mount(host,transport,{translate:tr=x=>x}={}){
  const el=(tag,text,cls,literal=false)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=literal?text:tr(text);if(cls)e.className=cls;return e};
  const card=el('article',undefined,'engine-settings-admin'),status=el('p','Chargement…','notice info'),open=el('button','Régler les limites d’exécution'),refresh=el('button','Recharger'),toolbar=el('div',undefined,'toolbar');open.type=refresh.type='button';open.disabled=true;open.dataset.limitsOpen='';status.setAttribute('role','status');toolbar.append(open,refresh);card.append(el('h3','Limites des prochains départs'),el('p','Projet, mission, rôle et tâche. Les tentatives déjà lancées gardent leurs limites gelées.'),status,toolbar);host.append(card);let dialog,last,busy=false;
  const labels={silence_seconds:'Silence (secondes)',tool_seconds:'Outil (secondes)',max_tool_calls:'Appels d’outils',max_repeated_calls:'Appels identiques',max_consecutive_errors:'Erreurs consécutives'};
  /** Require typed exact content; wrong keys/missing data never become zero. */
  function valid(view,q){return ['absent','stored_sql'].includes(view?.source)&&view?.scope===q.scope&&view.mission_id===q.mission_id&&view.scope_key===q.scope_key&&typeof view.exact?.entry?.revision==='string'&&['observation_mode',...names].every(k=>typeof view.exact.entry.values?.[k]==='string')&&Array.isArray(view.exact.history??[])&&Array.isArray(view.issues)}
  async function loadCard(){if(busy)return;busy=true;open.disabled=refresh.disabled=true;try{const q={scope:'project',mission_id:'',scope_key:''},view=await transport.show(q);if(!valid(view,q))throw Error('run-limits-view-invalid');last=view;status.textContent=tr('Portée projet lue. Aucun départ modifié.');status.className='notice info';open.disabled=false}catch(e){status.className='notice alert';status.textContent=tr('Lecture impossible. Rechargez avant de reprendre.')+' '+e.message}finally{busy=false;refresh.disabled=false}}
  /** Native dialog keeps actions visible and original recorded text literal.
   * Context changes invalidate loaded source; explicit reload discards the draft.
   * A known positive receipt survives failed readback. Escape restores focus
   * after summary reload, so a temporarily disabled opener cannot lose it. */
  function show(){
   if(dialog||!last)return;const origin=document.activeElement;dialog=el('dialog',undefined,'engine-settings-dialog');dialog.dataset.runLimits='';dialog.setAttribute('aria-labelledby','run-limits-title');const form=el('form'),header=el('header',undefined,'modal-header'),title=el('h2','Limites des prochains départs'),body=el('div',undefined,'modal-body'),footer=el('div',undefined,'toolbar modal-footer'),state=el('p','Chargez la portée à modifier.','notice info');title.id='run-limits-title';header.append(title);state.setAttribute('role','status');state.tabIndex=-1;state.dataset.limitsStatus='';
   const context=el('fieldset'),legend=el('legend','Portée'),scope=el('select'),mission=el('input'),key=el('input'),read=el('button','Charger la portée'),fields={};scope.dataset.limitsScope='';mission.dataset.limitsMission='';key.dataset.limitsKey='';scope.name='scope';mission.name='mission_id';key.name='scope_key';read.type='button';read.dataset.limitsRead='';
   for(const [value,label]of [['project','Projet'],['mission','Mission'],['role','Rôle'],['task','Tâche']]){const option=el('option',label);option.value=value;scope.append(option)}
   for(const [node,label]of [[scope,'Portée'],[mission,'Identifiant de mission'],[key,'Clé de rôle ou de tâche']]){const wrap=el('label',label);wrap.append(node);context.append(wrap)}context.prepend(legend);context.append(read);
   const grid=el('div',undefined,'engine-settings-fields');
   for(const name of names){const input=el('input');input.type='text';input.inputMode='numeric';input.dataset.limitsField=name;const wrap=el('label',labels[name]);wrap.append(input);grid.append(wrap);fields[name]=input}
   const observation=el('input');observation.type='checkbox';observation.dataset.limitsObservation='';const observeLabel=el('label','Mode observation (mission avec motif explicite)');observeLabel.prepend(observation);grid.append(observeLabel);
   const reason=el('textarea');reason.dataset.limitsReason='';const reasonLabel=el('label','Motif du changement');reasonLabel.append(reason);
   const source=el('p'),policy=el('details'),policyBody=el('div'),history=el('details'),historyBody=el('div'),diff=el('pre');policy.append(el('summary','Plafonds et défauts enregistrés'),policyBody);history.append(el('summary','Historique'),historyBody);diff.hidden=true;diff.dataset.limitsDiff='';
   const preview=el('button','Prévisualiser'),save=el('button','Enregistrer','primary'),reload=el('button','Recharger'),close=el('button','Fermer');preview.type=save.type=reload.type=close.type='button';preview.dataset.limitsPreview='';save.dataset.limitsApply='';reload.dataset.limitsReload='';save.disabled=preview.disabled=true;footer.append(preview,save,reload,close);
   body.append(el('p','Zéro hérite du niveau englobant. Cet aperçu administratif ne remplace pas l’admission du fournisseur, du contrat de mission et des tentatives précédentes.'),context,source,grid,reasonLabel,policy,history,diff,state);footer.prepend(state);state.classList.add('run-limits-status');form.append(header,body,footer);dialog.append(form);document.body.append(dialog);
   let current,pending,inflight=false,recorded=false;
   const q=()=>({scope:scope.value,mission_id:scope.value==='project'?'':mission.value,scope_key:['role','task'].includes(scope.value)?key.value:''});
   function reveal(){state.focus({preventScroll:true});state.scrollIntoView({block:'nearest'})}
   function controls(value){inflight=value;for(const node of [scope,mission,key,read,preview,save,reload,close,reason,observation,...Object.values(fields),...historyBody.querySelectorAll('button')])node.disabled=value;if(!value){mission.disabled=scope.value==='project';key.disabled=!['role','task'].includes(scope.value);preview.disabled=!current||current.bounds===null;save.disabled=!pending||!current;for(const node of [reason,observation,...Object.values(fields)])node.disabled=!current||current.bounds===null;preview.className=pending?'':'primary';save.className=pending?'primary':'';for(const node of historyBody.querySelectorAll('button'))node.disabled=!current||current.bounds===null;}}
   function invalidate(){pending=null;save.disabled=true;diff.hidden=true;}
   function error(e,uncertain=true){invalidate();if(uncertain)current=null;preview.disabled=!current||current.bounds===null;state.className='notice alert';state.textContent=tr(recorded?'Enregistré ; relecture indisponible. Rechargez pour vérifier.':uncertain?'Résultat non confirmé. Relisez la portée avant de reprendre.':'Aperçu refusé. Corrigez le brouillon ou rechargez la portée.')+' '+e.message;reveal()}
   function contextChanged(){if(inflight)return;invalidate();current=null;source.textContent='';historyBody.replaceChildren();state.className='notice info';state.textContent=tr('Chargez la portée à modifier.');controls(false)}
   scope.addEventListener('change',()=>{mission.value=key.value='';contextChanged()});mission.addEventListener('input',contextChanged);key.addEventListener('input',contextChanged);
   for(const node of [reason,observation,...Object.values(fields)])node.addEventListener('input',()=>{invalidate();reason.required=observation.checked});
   /** Actual current values/history/bounds, not reconstructed effective limits. */
   function render(view){
    current=view;for(const name of names)fields[name].value=view.exact.entry.values[name];observation.checked=view.exact.entry.values.observation_mode==='1';reason.value='';reason.required=observation.checked;
    source.textContent=tr('Révision')+' '+view.exact.entry.revision+' · '+tr(view.source==='absent'?'Aucune valeur enregistrée : héritage.':'Configuration enregistrée');
    policyBody.replaceChildren();for(const bound of view.exact.bounds??[]){const block=el('p');block.textContent=tr(labels[bound.name]??bound.name)+' · '+tr('Plafond de définition')+' : '+(bound.max===null?tr('Sans plafond'):bound.max)+' · '+tr('Défaut initial')+' : '+view.exact.defaults[bound.name];policyBody.append(block)}
    historyBody.replaceChildren();for(const row of view.exact.history??[]){const section=el('section'),restore=el('button','Prévisualiser le retour');restore.type='button';restore.dataset.limitsRestore=row.revision;restore.addEventListener('click',()=>makePreview(row.revision));section.append(el('h4',tr('Révision')+' '+row.revision),el('p',row.reason,undefined,true),el('p',row.actor,undefined,true),el('p',row.at,undefined,true),restore);historyBody.append(section)}
    if(!(view.exact.history??[]).length)historyBody.append(el('p','Aucune modification enregistrée.'));
   }
   async function load(initial=false){
    if(inflight)return;invalidate();controls(true);const wanted=q();try{const view=await transport.show(wanted);if(!valid(view,wanted))throw Error('run-limits-view-invalid');render(view);recorded=false;state.className=view.issues.length?'notice alert':'notice info';state.textContent=view.issues.length?tr('Inspectez les réglages et les valeurs enregistrées avant application.')+' '+view.issues.join(', '):tr('Configuration relue.')}catch(e){error(e)}finally{controls(false);if(initial){body.scrollTop=0;scope.focus()}else if(current){fields[names[0]].focus()}else reveal()}
   }
   /** Preview explicit proposed values or source-bound historical rollback. */
   async function makePreview(target){
    if(inflight||!current)return;invalidate();controls(true);try{
     const base={...q(),event_id:crypto.randomUUID(),expected_revision:current.exact.entry.revision,reason:reason.value};
     const request=target===undefined?{...base,schema_version:1,values:Object.fromEntries(['observation_mode',...names].map(name=>[name,name==='observation_mode'?(observation.checked?'1':'0'):fields[name].value]))}:{...base,to_revision:target};
     encode(request);const result=await (target===undefined?transport.preview(request):transport.rollbackPreview(request));
     if(result.preview!==true||result.applied!==false||typeof result.preview_token!=='string'||!result.exact?.current||!result.exact?.proposed)throw Error('run-limits-preview-invalid');
     diff.textContent=JSON.stringify({[tr('Avant')]:result.exact.current.values,[tr('Après')]:result.exact.proposed},null,2);diff.hidden=false;
     if(!result.revision_ok&&!result.replayed){state.className='notice alert';state.textContent=tr('La portée a changé. Rechargez avant de confirmer.');}
     else{pending={request:{...request,preview_token:result.preview_token},rollback:target!==undefined,expectation:result};state.className='notice info';state.textContent=tr('Aperçu sans écriture. Confirmez pour enregistrer.');}
    }catch(e){error(e,false)}finally{controls(false);reveal()}
   }
   preview.addEventListener('click',()=>makePreview());read.addEventListener('click',()=>load());reload.addEventListener('click',()=>load());
   save.addEventListener('click',async()=>{if(inflight||!pending)return;const action=pending;controls(true);try{const result=await (action.rollback?transport.rollback(action.request):transport.apply(action.request));const proof=SwarmPolicyReceipt.verifyScopeMutation(result,action.request,action.expectation);recorded=true;invalidate();controls(false);await load();if(!current){recorded=true;throw Error('run-limits-readback-failed')}let readback;try{readback=SwarmPolicyReceipt.verifyScopeReadback(current,proof)}catch(error){recorded=false;throw error}recorded=false;state.className='notice success';state.textContent=tr(readback.replayed?'Rejeu reconnu ; état courant relu.':readback.newer?'Changement enregistré ; état plus récent relu.':'Enregistré et relu.');}catch(e){error(e)}finally{controls(false);reveal()}});
   close.addEventListener('click',()=>dialog.close());dialog.addEventListener('cancel',e=>{if(inflight)e.preventDefault()});dialog.addEventListener('close',async()=>{dialog.remove();dialog=null;await loadCard();if(origin?.isConnected&&!origin.disabled)origin.focus();else refresh.focus()},{once:true});dialog.showModal();scope.focus();controls(false);load(true);
  }
  open.addEventListener('click',show);refresh.addEventListener('click',loadCard);loadCard();return {reload:loadCard,open:show};
 }
 globalThis.SwarmRunLimitsAdmin=Object.freeze({mount,encode});
})();
