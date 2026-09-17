// Human answers are bound to the exact plan and revision reviewed in this dialog.
export class PreparationDecisions {
 constructor(api,current,committed){
  this.api=api;this.current=current;this.committed=committed;this.draft=null;this.pending=null;this.busy=false;
  this.dialog=document.getElementById('decisions-dialog');this.fields=document.getElementById('decisions-fields');this.error=document.getElementById('decisions-error');this.save=document.getElementById('decisions-save');
  document.getElementById('decisions-open').addEventListener('click',e=>{if(e.isTrusted)this.open().catch(e=>this.fail(e));});
  document.getElementById('decisions-form').addEventListener('submit',e=>{e.preventDefault();if(e.isTrusted)this.submit();});
  document.getElementById('decisions-reset').addEventListener('click',async e=>{if(!e.isTrusted||this.busy||this.pending)return;try{const p=await this.current();if(!p)return;const fresh=await this.api('preparations/show?id='+encodeURIComponent(p.id));this.draft=null;this.build(fresh);}catch(e){this.fail(e)}});
 }
 hasDraft(){return !!this.pending||!!this.draft&&this.draft.decisions.some((q,i)=>q.answer!==this.draft.original[i]);}
 fail(e){this.error.textContent=e.message;}
 paint(p){
  let plan;try{plan=JSON.parse(p.documents.plan?.text||'')}catch{}
  const qs=Array.isArray(plan?.questions)?plan.questions:[];
  const valid=qs.every(q=>q&&typeof q.question==='string'&&typeof q.answer==='string');
  const count=valid?qs.filter(q=>!q.answer.trim()).length:0;
  document.getElementById('decisions-open').hidden=!valid||!qs.length;
  document.getElementById('decisions-open').textContent=count?'Répondre aux '+count+' décisions en attente':'Relire les '+qs.length+' décisions';
 }
 build(p){
  const plan=JSON.parse(p.documents.plan.text);
  if(!Array.isArray(plan.questions)||!plan.questions.length)throw new Error('Aucune question dans ce plan.');
  this.draft={id:p.id,revision:p.revision,hash:p.documents.plan.sha256,decisions:structuredClone(plan.questions),original:plan.questions.map(q=>q.answer)};
  this.render();
 }
 render(){
  this.fields.replaceChildren();this.error.textContent='';
  this.draft.decisions.forEach((q,i)=>{const box=document.createElement('div'),label=document.createElement('label'),input=document.createElement('textarea');box.className='decision-row';input.id='decision-'+i;input.rows=3;input.maxLength=4000;input.value=q.answer;input.placeholder='Votre décision…';label.htmlFor=input.id;label.textContent=(i+1)+'. '+q.question;input.addEventListener('input',e=>{if(e.isTrusted)this.draft.decisions[i].answer=input.value});box.append(label,input);this.fields.append(box)});
  this.lock();
 }
 lock(){this.save.disabled=this.busy;this.save.textContent=this.pending?'Vérifier l’enregistrement':'Enregistrer mes décisions';document.getElementById('decisions-reset').disabled=this.busy||!!this.pending;for(const input of this.fields.querySelectorAll('textarea'))input.readOnly=this.busy||!!this.pending;}
 async open(){
  const p=await this.current();if(!p)return;
  if(!this.draft||this.draft.id!==p.id||!this.hasDraft())this.build(p);else this.render();
  this.dialog.showModal();this.fields.querySelector('textarea')?.focus();
 }
 async submit(){
  if(this.busy||!this.draft)return;
  const d=this.draft;
  this.pending ||= {version:1,action:'answer-questions',preparation_id:d.id,event_id:crypto.randomUUID(),expected_revision:d.revision,sha256:d.hash,decisions:structuredClone(d.decisions)};
  this.busy=true;this.lock();
  try{
   const p=await this.api('preparations/command',this.pending);this.pending=null;
   if(p.receipt_historical)throw new Error('Réponses enregistrées, puis préparation modifiée ailleurs. Rechargez le plan pour examiner sa version courante.');
   this.committed(p);this.draft=null;this.dialog.close();document.getElementById('decisions-open').focus();
  }catch(e){if(e.status&&e.status<500)this.pending=null;this.fail(e)}finally{this.busy=false;this.lock()}
 }
}
