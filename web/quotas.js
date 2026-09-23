'use strict';
const trQuotas=s=>globalThis.SwarmI18n?.t(s)??s;
const Quotas={
 open(){
  const p=snapshot?.work.planning;
  if(!p){notice(trQuotas('Cette mission ne dispose pas de planification hiérarchique.'),true);return}
  openModal(trQuotas('Plafonds de planification et de vérification'),trQuotas('Les consommations sont conservées. Cette modification ne lance aucun agent et ne valide aucun résultat. Une mission active pourra utiliser le nouveau plafond au prochain passage du conducteur.'),{action:'quotas',hasReviewer:!!p.reviewer});
  const box=node('section',undefined,'notice info field-wide');box.append(node('p',trQuotas('Consommés ou engagés : ')+p.activations+' / '+p.max_activations+' · '+trQuotas('Décisions')+' '+p.decisions+' / '+p.max_decisions+' · '+trQuotas('Vérifications')+' '+(p.reviewer?p.reviewer.calls+' / '+p.reviewer.max_calls:trQuotas('Non configuré'))));
  if(p.quota_authorization){const a=p.quota_authorization;box.append(node('p',a.actor+' · '+a.at+' · '+a.reason))}
  box.append(node('p',trQuotas('Les plafonds propres aux sous-planificateurs restent inchangés. Les tentatives de production se réautorisent depuis leur tâche, avec une consigne corrective.')));
  for(const s of p.scopes||[])if(s.max_activations)box.append(node('p',s.id+' : '+s.activations+' / '+s.max_activations));$('modal-fields').append(box);
  for(const [key,label,value,min,max]of [['planning_activations','Activations de planification',p.max_activations,Math.max(1,p.activations),200],['planning_decisions','Décisions de planification',p.max_decisions,Math.max(1,p.decisions),100],...(p.reviewer?[['review_calls','Appels du vérificateur',p.reviewer.max_calls,Math.max(1,p.reviewer.calls),100]]:[])]){const el=field(key,trQuotas(label),value);el.type='number';el.min=String(min);el.max=String(max);el.step='1';el.required=true}
  const reason=field('quota_reason',trQuotas('Motif de la modification'),'',null,true);reason.required=true;reason.minLength=8;reason.maxLength=2000;
  $('confirm').textContent=trQuotas('Prévisualiser les plafonds');$('modal-fields').oninput=()=>{if(modalContext?.action==='quotas'){modalContext.quotaPreview=null;$('preview').hidden=true;$('confirm').textContent=trQuotas('Prévisualiser les plafonds')}};
 },
 async submit(c,f){
  const r={limits:{planning_activations:Number(f.planning_activations),planning_decisions:Number(f.planning_decisions),review_calls:c.hasReviewer?Number(f.review_calls):null},reason:f.quota_reason};const signature=JSON.stringify(r);
  if(c.quotaPreview!==signature){const p=await act('quotas-preview',{quotas:r});if(modalContext!==c)return;const lines=[];for(const [key,label]of [['planning_activations','Activations de planification'],['planning_decisions','Décisions de planification'],['review_calls','Appels du vérificateur']]){if(p.current.limits[key]!==null)lines.push(trQuotas(label)+' : '+p.current.limits[key]+' → '+p.proposed.limits[key]+' · '+trQuotas('Restant : ')+p.proposed.remaining[key])}preview(lines.join('\n'));c.quotaPreview=signature;$('confirm').textContent=trQuotas('Autoriser ces plafonds');return}
  await act('quotas',{quotas:r});if(modalContext!==c)return;closeModal();await refresh();notice(trQuotas('Plafonds enregistrés ; consommations et historique conservés.'));
 }
};
$('quotas-open').onclick=()=>Quotas.open();
