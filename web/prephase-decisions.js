const tr_web_prephase_decisions_js = source => globalThis.SwarmI18n?.t(source) ?? source;
// Human answers bind the exact reviewed plan/revision. No local answer cap:
// the authoritative saved plan/document policies validate the whole request.
/** Edit ordered answers bound to one plan hash/revision. Unknown publication
 * keeps an immutable event for replay. A recorded historical receipt disables
 * further submission until explicit reload; errors never discard the draft. */
export class PreparationDecisions {
 constructor(api,current,committed){
  this.api=api;this.current=current;this.committed=committed;this.draft=null;this.pending=null;this.busy=false;this.recorded=false;
  this.dialog=document.getElementById('decisions-dialog');this.fields=document.getElementById('decisions-fields');this.error=document.getElementById('decisions-error');this.save=document.getElementById('decisions-save');
  document.getElementById('decisions-open').addEventListener('click',e=>{if(e.isTrusted)this.open().catch(e=>this.fail(e));});
  document.getElementById('decisions-form').addEventListener('submit',e=>{e.preventDefault();if(e.isTrusted)this.submit();});
  document.getElementById('decisions-reset').addEventListener('click',async e=>{if(!e.isTrusted||this.busy||this.pending)return;this.busy=true;this.lock();try{const p=await this.current();if(!p)return;const fresh=await this.api('preparations/show?id='+encodeURIComponent(p.id));this.draft=null;this.build(fresh);}catch(e){this.fail(e)}finally{this.busy=false;this.lock()}});
 }
 /** Unsubmitted, uncertain or recorded-but-not-reloaded state blocks navigation. */
 hasDraft(){return !!this.pending||this.recorded||!!this.draft&&this.draft.decisions.some((q,i)=>q.answer!==this.draft.original[i]);}
 fail(e){this.error.textContent=e.message;}
 paint(p){
  let plan;try{plan=JSON.parse(p.documents?.plan?.text||'')}catch{}
  const qs=Array.isArray(plan?.questions)?plan.questions:[];
  const valid=qs.every(q=>q&&typeof q.question==='string'&&typeof q.answer==='string');
  const count=valid?qs.filter(q=>!q.answer.trim()).length:0;
  document.getElementById('decisions-open').hidden=!valid||!qs.length;
  document.getElementById('decisions-open').textContent=count?tr_web_prephase_decisions_js('Répondre aux ')+count+tr_web_prephase_decisions_js(' décisions en attente'):tr_web_prephase_decisions_js('Relire les ')+qs.length+tr_web_prephase_decisions_js(' décisions');
 }
 /** Replace the draft only after explicit open/reload of an observed snapshot. */
 build(p){
  const plan=JSON.parse(p.documents.plan.text);
  if(!Array.isArray(plan.questions)||!plan.questions.length)throw new Error(tr_web_prephase_decisions_js('Aucune question dans ce plan.'));
  this.recorded=false;this.draft={id:p.id,revision:p.revision,hash:p.documents.plan.sha256,decisions:structuredClone(plan.questions),original:plan.questions.map(q=>q.answer)};
  this.render();
 }
 render(){
  this.fields.replaceChildren();this.error.textContent='';
  this.draft.decisions.forEach((q,i)=>{const box=document.createElement('div'),label=document.createElement('label'),input=document.createElement('textarea');box.className='decision-row';input.id='decision-'+i;input.rows=3;input.value=q.answer;input.placeholder=tr_web_prephase_decisions_js('Votre décision…');label.htmlFor=input.id;label.textContent=(i+1)+'. '+q.question;input.addEventListener('input',e=>{if(e.isTrusted)this.draft.decisions[i].answer=input.value});box.append(label,input);this.fields.append(box)});
  this.lock();
 }
 /** Freeze inputs while IO is pending or historical recording awaits reload. */
 lock(){this.save.disabled=this.busy||this.recorded;this.save.textContent=this.pending?tr_web_prephase_decisions_js('Vérifier l’enregistrement'):tr_web_prephase_decisions_js('Enregistrer mes décisions');document.getElementById('decisions-reset').disabled=this.busy||!!this.pending;for(const input of this.fields.querySelectorAll('textarea'))input.readOnly=this.busy||!!this.pending||this.recorded;}
 async open(){
  const p=await this.current();if(!p)return;
  if(!this.draft||this.draft.id!==p.id||!this.hasDraft())this.build(p);else this.render();
  this.dialog.showModal();this.fields.querySelector('textarea')?.focus();
 }
 /** Send/replay the same event; validate identity/revision before clearing it.
  * Static HTTP refusals preserve the editable draft; uncertain errors retain it. */
 async submit(){
  if(this.busy||this.recorded||!this.draft)return;
  const d=this.draft;
  this.pending ||= {version:1,action:'answer-questions',preparation_id:d.id,event_id:crypto.randomUUID(),expected_revision:d.revision,sha256:d.hash,decisions:structuredClone(d.decisions)};
  this.busy=true;this.lock();
  try{
   const p=await this.api('preparations/command',this.pending);
   if(!p||p.id!==d.id||!Number.isSafeInteger(p.revision)||p.revision!==this.pending.expected_revision+1)throw new Error(tr_web_prephase_decisions_js('Réponse de préparation incohérente. Vérifiez l’enregistrement avant de modifier le texte.'));
   this.pending=null;
   if(p.receipt_historical){this.recorded=true;throw new Error(tr_web_prephase_decisions_js('Réponses enregistrées, puis préparation modifiée ailleurs. Rechargez le plan pour examiner sa version courante.'));}
   this.committed(p);this.draft=null;this.dialog.close();document.getElementById('decisions-open').focus();
  }catch(e){if(e.status&&e.status<500)this.pending=null;this.fail(e)}finally{this.busy=false;this.lock()}
 }
}
