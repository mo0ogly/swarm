'use strict';
// Business codes are deliberately language independent. This module is loaded
// before cockpit.js and planning.js so both views use the same decisions.
const SwarmStatusContract={
 validationCodes:new Set(['validated','waived','blocked','abandoned','partial','open']),
 validation(value){
  const state=value?.state;
  if(!this.validationCodes.has(state))throw new Error('unknown validation state: '+String(state));
  return {state,tone:state==='validated'?'success':'attention',complete:state==='validated'};
 },
 renderValidation(element,value,total,running,translate=s=>s){
  const verdict=this.validation(value);
  element.className='notice '+verdict.tone;element.dataset.validationState=verdict.state;
  element.textContent=(verdict.complete?translate('Tous les résultats sont validés.'):translate('La validation finale reste à obtenir.'))+' '+value.validated+'/'+total+translate(' tâches validées.')+(value.stale?' '+value.stale+translate(' résultat(s) doivent être vérifiés à nouveau.'):'')+(running?translate(' Des agents travaillent actuellement.'):'');
  return verdict;
 },
 planning(p){
  if(p?.failure)return 'error';
  if(p?.paused)return 'paused';
  if(Array.isArray(p?.scopes)&&p.scopes.length>0&&p.scopes.every(scope=>scope.state==='closed'))return 'completed';
  return 'active';
 },
 review(p,tasks=[],paused=false){
  if(!p?.reviewer)return {availability:'absent',state:'absent',message:''};
  if(p.reviewer.failure)return {availability:'configured',state:'error',message:'Vérification interrompue. '};
  if(paused)return {availability:'configured',state:'paused',message:'Mission en pause. '};
  if(tasks.some(task=>task.independent_review?.state==='running'))return {availability:'configured',state:'running',message:'Examen en cours. '};
  if(!tasks.some(task=>task.independent_review))return {availability:'configured',state:'idle',message:'Aucun avis pour le moment. '};
  return {availability:'configured',state:'recorded',message:'Avis enregistré. '};
 }
};
