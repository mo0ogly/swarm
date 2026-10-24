'use strict';
/** Native mission quota modal. Only explicit user gestures read a Work or submit
 * a proposal. The adapter owns authentication and serialization; this component
 * never starts an agent, resets usage, invents a cap or retries a failed write. */
(()=>{
 const groups=[
  ['Planification',[['planning_activations','Activations'],['planning_decisions','Décisions'],['planning_tasks','Tâches'],['planning_scopes','Périmètres'],['delegation_depth','Profondeur']]],
  ['Revue indépendante',[['review_calls','Appels de revue'],['review_timeout_seconds','Durée totale de revue en secondes'],['review_context_tokens','Fenêtre client en tokens'],['review_context_files','Fichiers de contexte'],['review_source_bytes','Sources en octets'],['review_context_bytes','Contexte en octets'],['review_delivery_files','Fichiers livrés']]],
  ['Juge',[['judge_cycles','Cycles du juge']]]
 ];
 /** Resolve native duration metadata separately from inherited Go quota fields.
  * Missing data stays missing; neither read nor presentation adopts a default. */
 function valueOf(view,key){return key==='review_timeout_seconds'?view.review_timeout_seconds:view.limits[key]}
 /** Reject every unsafe JS number before presenting editable data. The JSON API
  * retains signed64-bit integers; the browser cannot represent them all exactly.
  * This is a technical UI limitation, never an operator policy or backend cap. */
 function exact(value){if(typeof value==='number'&&!Number.isSafeInteger(value))throw Error('quota-browser-integer-unrepresentable');if(value&&typeof value==='object')for(const v of Object.values(value))exact(v);return value}
 /** Mount once on a supplied host. Required hooks return actual protected API
  * replies or reject with safe causes; no failure becomes an empty successful list.
  * Optional opener reuses a canonical caller button; selectedWork is evaluated
  * only on explicit open and only an actually listed planning Work is selected.
  * Dialog close restores the opener, including Escape. Pending writes prevent
  * closing only until their actual response; no hidden time bound is introduced. */
 function mount(host,api,{translate=String,opener=null,selectedWork=()=>''}={}){
  const tr=translate,el=(tag,text,cls)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=tr(text);if(cls)n.className=cls;return n};
  const open=opener||el('button','Plafonds des missions');open.type='button';open.dataset.quotaOpen='';if(!opener)host.append(open);
  const dialog=el('dialog',undefined,'modal engine-settings-dialog native-quotas-dialog');dialog.dataset.nativeQuotas='';dialog.setAttribute('aria-labelledby','native-quotas-title');
  const header=el('header',undefined,'modal-header'),title=el('h2','Plafonds des missions');title.id='native-quotas-title';const close=el('button','Fermer');close.type='button';header.append(title,close);
  const body=el('div',undefined,'modal-body'),status=el('p',undefined,'notice info');status.role='status';status.dataset.quotaStatus='';
  const label=el('label','Mission'),select=el('select');select.dataset.quotaWork='';label.append(select);
  const help=el('button','Comprendre les plafonds');help.type='button';const fields=el('div'),reasonLabel=el('label','Motif de la modification'),reason=el('textarea');reason.dataset.quotaReason='';reasonLabel.append(reason);
  const comparison=el('div',undefined,'engine-settings-table');comparison.dataset.quotaComparison='';
  const footer=el('div',undefined,'modal-footer'),preview=el('button','Prévisualiser'),apply=el('button','Appliquer');preview.type=apply.type='button';preview.dataset.quotaPreview='';apply.dataset.quotaApply='';apply.className='primary';apply.disabled=true;footer.append(preview,apply);
  body.append(status,label,help,fields,reasonLabel,comparison);dialog.append(header,body,footer);host.append(dialog);
  const helpDialog=el('dialog',undefined,'modal engine-settings-dialog engine-settings-help');helpDialog.setAttribute('aria-labelledby','native-quotas-help-title');const helpTitle=el('h2','Comprendre les plafonds');helpTitle.id='native-quotas-help-title';const helpBody=el('div',undefined,'modal-body');helpBody.append(helpTitle,
   el('p','Les consommations et l’historique sont conservés. Une hausse ne lance aucun agent. Un planificateur ou une revue en cours empêche la modification.'),
   el('p','Activations, décisions, tâches et appels de revue : plafonds positifs, au moins égaux à la consommation. Périmètres, profondeur, contexte et cycles du juge : zéro retire seulement ce plafond. Une valeur de contexte absente reste non configurée.'),
   el('p','Allocation locale : zéro rétablit l’héritage ; les plafonds des ancêtres restent applicables. Pour la fenêtre client en tokens, zéro rétablit le comportement du client, sans garantir un contexte illimité. Une valeur positive différente et le manifeste de revue natif ne sont pas encore pris en charge.'),
   el('p','Durée de revue : zéro retire le délai total ; une valeur positive fixe un délai en secondes. Une valeur absente conserve le réglage existant. Les limites des flux et des événements restent indépendantes. Les durées non représentables par le moteur sont refusées.'),
   el('p','Les tailles sont en octets UTF8. Le minimum du motif porte sur le texte rogné, le maximum sur le texte original ; ces deux valeurs sont configurables dans les réglages Admin. Le navigateur exige des entiers exactement représentables ; utiliser la CLI pour les valeurs plus grandes.'));
  const helpClose=el('button','Fermer');helpClose.type='button';helpBody.append(helpClose);helpDialog.append(helpBody);host.append(helpDialog);
  let busy=false,current=null,proposal=null,inputs=new Map(),scopeInputs=new Map();
  /** Invalidate confirmation whenever any editable value changes. No apply can
   * reuse a preview from an older Work selection or changed reason/input. */
  function invalidate(){proposal=null;apply.disabled=true;comparison.replaceChildren()}
  /** Render a fixed safe error family and retain known-write uncertainty. Causes
   * remain on rejected adapter errors, while private bodies are never rendered. */
  function failed(error){const code=typeof error.code==='string'?error.code:/^quota-[a-z0-9-]+$/.test(error.message)?error.message:'quota-operation-failed';status.className='notice alert';status.textContent=tr(error.response?.recorded_work?'La modification peut déjà être enregistrée. Relisez la mission avant de reprendre.':'Réponse native non confirmée. Vérifiez l’état avant de reprendre.')+' ('+code+')';invalidate()}
  /** Lock controls through actual completion; errors remain visible. A failed
   * write is not retried. Selection changes and close cannot race a publication. */
  async function run(action){if(busy)return;busy=true;close.disabled=select.disabled=preview.disabled=apply.disabled=true;fields.inert=true;reason.disabled=true;try{await action()}catch(error){failed(error)}finally{busy=false;close.disabled=select.disabled=false;preview.disabled=!current;fields.inert=false;reason.disabled=false;apply.disabled=!proposal}}
  /** Present server values without changing absent/default/unlimited semantics.
   * Fixed labels use textContent; arbitrary Work names never become markup. */
  function render(view){exact(view);current=view;inputs=new Map();scopeInputs=new Map();fields.replaceChildren();invalidate();reason.value='';
   for(const [name,entries] of groups){const details=el('details',undefined,'engine-settings-group'),summary=el('summary',name),grid=el('div',undefined,'engine-settings-fields');details.append(summary,grid);details.open=name==='Planification';
    for(const [key,text] of entries){const value=valueOf(view,key);if((value===undefined||value===null)&&!(key.startsWith('review_context_')||key==='review_source_bytes'||key==='review_delivery_files')||((key.startsWith('review_'))&&view.limits.review_calls==null))continue;const field=el('label',text,'engine-settings-field'),input=el('input');input.type='text';input.inputMode='numeric';input.value=value==null?'':String(value);input.placeholder=tr('Non configuré');input.dataset.quotaField=key;const used=view.consumed[key];const usage=el('small');usage.textContent=key==='review_timeout_seconds'?tr('Zéro : sans délai total.'):tr('Consommé')+' : '+(used??'—');field.append(input,usage);grid.append(field);inputs.set(key,input)}
    if(grid.childElementCount)fields.append(details);
   }
   const local=el('details',undefined,'engine-settings-group');local.append(el('summary','Allocations des périmètres'));const grid=el('div',undefined,'engine-settings-fields');local.append(grid);
   for(const scope of view.scopes){const field=el('label',undefined,'engine-settings-field');field.append(document.createTextNode(scope.id));const input=el('input');input.type='text';input.inputMode='numeric';input.value=String(scope.max_activations??0);input.dataset.quotaScope=scope.id;const used=el('small');used.textContent=tr('Consommé')+' : '+String(scope.activations??0);field.append(input,used);grid.append(field);scopeInputs.set(scope.id,input)}fields.append(local);
   status.className='notice info';status.textContent=tr('Consommations conservées. Prévisualisez avant d’appliquer.');
  }
  /** Parse browser-representable nonnegative integer data, not a product ceiling.
   * Server validation remains authoritative for consumed/hierarchy/role limits. */
  function number(input){if(!/^(0|[1-9][0-9]*)$/.test(input.value))throw Error('quota-browser-integer-invalid');const n=Number(input.value);if(!Number.isSafeInteger(n))throw Error('quota-browser-integer-unrepresentable');return n}
  /** Build one fresh event for a displayed revision; only changed scope overrides
   * are sent, preserving unlisted allocations. Native reviewer duration is sent
   * separately from inherited quota DTO fields; explicit zero is retained.
   * Missing duration stays omitted, never a default. Actor/time are not input. */
  function prepare(){if(!current||!select.value)throw Error('quota-work-required');const limits={...current.limits},scopes={};let timeout;for(const [key,input] of inputs){if(key==='review_timeout_seconds'){timeout=number(input);continue}if(input.value===''&&current.limits[key]==null)delete limits[key];else limits[key]=number(input)}for(const [id,input] of scopeInputs){const value=number(input),original=current.scopes.find(s=>s.id===id).max_activations??0;if(value!==original)scopes[id]=value}return {schema_version:1,event_id:'quota-'+crypto.randomUUID(),expected_revision:current.revision,limits,reason:reason.value,...(timeout!==undefined?{review_timeout_seconds:timeout}:{}),...(Object.keys(scopes).length?{scope_activations:scopes}:{})}}
  /** Display actual/proposed quotas and native duration with their labeled units.
   * The preview is evidence of validation at that instant, not a reservation. */
  function showPreview(value){exact(value);comparison.replaceChildren();const table=el('table'),head=el('tr');for(const text of ['Paramètre','Avant','Après'])head.append(el('th',text));table.append(head);for(const [,entries] of groups)for(const [key,text] of entries){if(valueOf(value.before,key)===undefined&&valueOf(value.after,key)===undefined)continue;const row=el('tr');row.append(el('th',text),el('td',String(valueOf(value.before,key)??'—')),el('td',String(valueOf(value.after,key)??'—')));table.append(row)}for(const [id] of scopeInputs){const old=value.before.scopes.find(s=>s.id===id),next=value.after.scopes.find(s=>s.id===id);if(old.max_activations===next.max_activations)continue;const row=el('tr');row.append(el('th',id),el('td',String(old.max_activations??0)),el('td',String(next.max_activations??0)));table.append(row)}comparison.append(table)}
  /** Reuse the real list and optional explicit selected Work on user open.
   * All options are built before one read; no default Work/planning is adopted.
   * Failed list/getter/show leaves current null and confirmation unavailable. */
  async function loadWorks(){const works=await api.list();if(!Array.isArray(works))throw Error('quota-work-list-invalid');const initial=el('option','Choisir une mission');initial.value='';select.append(initial);const eligible=works.filter(w=>w.planning);for(const work of eligible){const option=el('option');option.value=work.id;option.textContent=work.title+' — '+work.id;select.append(option)}const requested=selectedWork();if(requested&&eligible.some(w=>w.id===requested)){select.value=requested;render(await api.show(requested))}else{status.textContent=tr(eligible.length?'Choisir une mission':'Aucune mission avec planification');status.className='notice info'}}
  open.addEventListener('click',()=>{dialog.showModal();select.replaceChildren();current=null;fields.replaceChildren();reason.value='';invalidate();run(loadWorks)});
  select.addEventListener('change',()=>{current=null;preview.disabled=true;fields.replaceChildren();invalidate();if(select.value)run(async()=>render(await api.show(select.value)))});
  fields.addEventListener('input',invalidate);reason.addEventListener('input',invalidate);
  preview.addEventListener('click',()=>run(async()=>{const value=prepare(),reply=await api.preview(select.value,value);showPreview(reply);proposal=value;status.className='notice info';status.textContent=tr('Prévisualisation validée. La sauvegarde revérifiera la révision et les plafonds.')}));
  apply.addEventListener('click',()=>{if(!proposal)return;const value=proposal;invalidate();run(async()=>{const reply=await api.apply(select.value,value);try{if(reply.id!==select.value||!Number.isSafeInteger(reply.revision)||reply.revision!==value.expected_revision+1)throw Error('quota-apply-response-invalid');render(await api.show(select.value))}catch(cause){const error=new Error('quota-post-apply-read-failed',{cause});error.code='quota-post-apply-read-failed';error.response={recorded_work:{id:select.value,revision:reply?.revision}};throw error}status.textContent=tr('Plafonds enregistrés et relus. Consommations conservées.')})});
  close.addEventListener('click',()=>{if(!busy)dialog.close()});dialog.addEventListener('cancel',event=>{if(busy)event.preventDefault()});dialog.addEventListener('close',()=>open.focus());
  help.addEventListener('click',()=>helpDialog.showModal());helpClose.addEventListener('click',()=>helpDialog.close());helpDialog.addEventListener('close',()=>help.focus());
  return {isOpen:()=>dialog.open};
 }
 globalThis.SwarmNativeQuotas=Object.freeze({mount});
})();
