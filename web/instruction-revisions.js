'use strict';
const InstructionRevisions={
 feedback:new Map(),
 key:(workID,id)=>workID+"|"+id,
 tr:s=>globalThis.SwarmI18n?.t(s)??s,
 render(task){
  if(!task.instruction_revisions?.length)return null;
  const box=node('section',undefined,'instruction-revisions');box.setAttribute('aria-label',this.tr('Consignes améliorées'));box.append(node('h4',this.tr('Consignes améliorées')),node('p',this.tr('Ces ajouts préparent les prochaines tâches à partir des retours de l’équipe.')));
  for(const r of task.instruction_revisions){
   const card=node('article',undefined,'instruction-revision');card.dataset.instructionRevision=r.id;card.dataset.state=r.state;
   card.append(node('strong',this.tr(r.state==='rejected'?'Rejetée':r.used_by_attempt?'Transmise à un agent':'Validée par défaut')),node('p',r.addition,'instruction-addition'));
   const details=node('details');details.append(node('summary',this.tr('Pourquoi cet ajout ?')),node('p',r.reason),node('p',this.tr('Responsable : ')+r.scope),node('p',this.tr('Date : ')+new Date(r.at).toLocaleString()));
   if(r.rejected_reason)details.append(node('p',r.rejected_reason));card.append(details);
   if(r.state==='applied'){
    const canReject=task.status==='todo'&&!r.used_by_attempt&&(task.attempts||[]).length===r.attempts_at_creation;
    const b=Pilot.command(this.tr('Rejeter'),()=>this.reject(task.id,r.id,card));b.dataset.inspectorAction='reject-instruction-'+r.id;b.disabled=!canReject||this.feedback.get(this.key(snapshot.work.id,r.id))?.pending;card.append(b);
    if(!canReject)card.append(node('p',this.tr('La consigne déjà transmise reste conservée dans cette tentative.')));
   }
   const feedback=this.feedback.get(this.key(snapshot.work.id,r.id));if(feedback)card.append(this.feedbackNode(feedback));
   box.append(card);
  }
  return box;
 },
 feedbackNode(feedback){const p=node('p',feedback.text,'instruction-feedback');p.setAttribute('role',feedback.error?'alert':'status');if(feedback.error)p.classList.add('notice','alert');return p;},
 async reject(task,id,card){
  const workID=snapshot.work.id,revision=snapshot.work.revision,button=card.querySelector('button');button.disabled=true;
  const key=this.key(workID,id);this.feedback.set(key,{text:this.tr('Rejet en cours…'),pending:true});card.querySelector('.instruction-feedback')?.remove();card.append(this.feedbackNode(this.feedback.get(key)));
  try{await api('/api/v1/planning?work='+encodeURIComponent(workID)+'&action=reject-instructions',{schema_version:1,event_id:crypto.randomUUID(),expected_revision:revision,task_id:task,instruction_revision_id:id,reason:this.tr('Ajout rejeté par l’utilisateur depuis le pilotage.')});this.feedback.delete(key);await refresh(true);if(snapshot.work.id===workID){Pilot.inspectorKey='';PilotInspector.render();const updated=document.querySelector(`[data-instruction-revision="${CSS.escape(id)}"]`);if(updated){updated.tabIndex=-1;updated.focus({preventScroll:true});}}}
  catch(e){
   const feedback={text:this.tr('Rejet non appliqué : ')+(e.message||String(e)),error:true,pending:false};this.feedback.set(key,feedback);
   // The refusal is already known. Show it before a possibly slow readback;
   // refresh and inspector caching must not leave a stale "pending" message.
   if(snapshot.work.id===workID){
    const currentCard=document.querySelector(`[data-instruction-revision="${CSS.escape(id)}"]`);
    if(currentCard){currentCard.querySelector('.instruction-feedback')?.remove();currentCard.append(this.feedbackNode(feedback));}
    Pilot.inspectorKey='';PilotInspector.render();
    try{await refresh(true)}catch{}
    Pilot.inspectorKey='';PilotInspector.render();
   }
  }
 }
};
