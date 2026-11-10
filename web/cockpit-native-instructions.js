'use strict';
/** Native instruction-history widget. Derived from the GLM proposal, corrected
 * for omitted Go empty attempts and contextual async readback. Text is rendered
 * safely; the caller owns authenticated mutation, freshness and readback. */
(()=>{
 const tr=s=>globalThis.SwarmI18n?.t(s)??s;
 const feedback=new Map();let selected='',epoch=0,renderedRevision=null;
 /** Build a semantic node without interpreting operator/model content as HTML. */
 function node(tag,text,cls){const el=document.createElement(tag);if(text!==undefined)el.textContent=text;if(cls)el.className=cls;return el}
 /** Empty/omitted attempts are Go's zero attempts; malformed values deny writes.
  * This precheck grants no authority: the writer rechecks SQL ownership and CAS. */
 function allowed(work,task,r){return Number.isSafeInteger(work.revision)&&work.revision>0&&task.status==='todo'&&r.state==='applied'&&!r.used_by_attempt&&(task.attempts==null||Array.isArray(task.attempts))&&Number.isSafeInteger(r.attempts_at_creation)&&r.attempts_at_creation===(task.attempts?.length??0)}
 /** Clear the previous context, invalidating in-flight focus/feedback. No IO. */
 function clear(container){epoch++;selected='';feedback.clear();container.replaceChildren()}
 /** Render typed Work history. No network, autosave or retry is performed.
  * onReject returns only after protected write and authoritative Work readback.
  * Malformed data hides mutation; errors remain local to the observed context.
  * observed=true is reserved for host readback and clears settled old feedback;
  * internal busy/error redraws must keep it false. */
 function render(container,work,onReject,observed=false){
  const id=typeof work?.id==='string'?work.id:'';if(selected!==id){epoch++;feedback.clear();selected=id}renderedRevision=work?.revision;if(observed)for(const [key,value] of feedback){if(!value.pending)feedback.delete(key)}container.replaceChildren();
  const rows=(Array.isArray(work?.tasks)?work.tasks:[]).flatMap(task=>(Array.isArray(task.instruction_revisions)?task.instruction_revisions:[]).filter(r=>r&&typeof r.id==='string'&&typeof r.addition==='string').map(r=>({task,r})));
  if(!id||!rows.length)return;
  const section=node('section',undefined,'instruction-revisions');section.setAttribute('aria-label',tr('Consignes améliorées'));section.append(node('h3',tr('Consignes améliorées')),node('p',tr('Ces ajouts préparent les prochaines tâches à partir des retours de l’équipe.')));
  for(const {task,r} of rows){
   const card=node('article',undefined,'instruction-revision');card.dataset.instructionRevision=r.id;card.dataset.state=r.state??'';card.tabIndex=-1;
   card.append(node('strong',task.title||task.id),node('p',tr(r.state==='rejected'?'Rejetée':r.state==='applied'?r.used_by_attempt?'Transmise à un agent':'Validée par défaut':'État inconnu')));
   const detail=node('details');detail.append(node('summary',tr('Détails de la consigne')),node('p',r.addition,'instruction-addition'));
   if(r.reason)detail.append(node('p',tr('Pourquoi cet ajout ? ')+r.reason));if(r.holder)detail.append(node('p',tr('Responsable : ')+r.holder));if(r.rejected_reason)card.append(node('p',tr('Motif du rejet : ')+r.rejected_reason));card.append(detail);
   const key=JSON.stringify([id,r.id]),entry=feedback.get(key);let button;
   if(r.state==='applied'){
    button=node('button',tr('Rejeter'));button.type='button';button.dataset.inspectorAction='reject-instruction-'+r.id;button.disabled=!allowed(work,task,r)||Boolean(entry?.pending)||Boolean(entry?.uncertain&&entry.revision===work.revision);
    if(!allowed(work,task,r))card.append(node('p',tr('La consigne déjà transmise reste conservée dans cette tentative.')));
    button.addEventListener('click',()=>reject(container,work,task,r,onReject,key,epoch));card.append(button);
   }
   if(entry){const p=node('p',entry.text,'notice '+(entry.error?'alert':'info'));p.setAttribute('role',entry.error?'alert':'status');card.append(p)}
   section.append(card);
  }container.append(section);
 }
 /** One explicit gesture, never automatically retried. During a request the
  * same revision cannot be resubmitted. Unknown outcomes require fresh readback;
  * another selected Work never receives the old result or focus. */
 async function reject(container,work,task,r,onReject,key,ticket){
  if(ticket!==epoch||selected!==work.id||feedback.get(key)?.pending||!allowed(work,task,r))return;
  feedback.set(key,{pending:true,text:tr('Rejet en cours…')});render(container,work,onReject);
  try{
   const observed=await onReject(work.id,work.revision,task.id,r.id,tr('Ajout rejeté par l’utilisateur depuis le pilotage.'));
   if(ticket!==epoch||selected!==work.id)return;feedback.delete(key);render(container,observed,onReject);
   const card=[...container.querySelectorAll('[data-instruction-revision]')].find(n=>n.dataset.instructionRevision===r.id);card?.focus({preventScroll:true});
  }catch(cause){
   if(ticket!==epoch||selected!==work.id)return;
   if(renderedRevision!==work.revision){feedback.delete(key);return}
   const code=/^[a-z][a-z0-9_.-]*$/.test(cause.code||cause.message)?cause.code||cause.message:'web-native-instruction-failed';
   feedback.set(key,{error:true,uncertain:true,revision:work.revision,text:tr(cause.outcome==='refused'?'Rejet refusé. Actualisez pour relire la révision avant de réessayer.':'La modification peut être enregistrée. Actualisez pour relire son état avant de réessayer.')+' ('+code+')'});render(container,work,onReject);
  }
 }
 globalThis.SwarmNativeInstructions=Object.freeze({render,clear});
})();
