'use strict';
function validateTrustObserveResponse(r,p,agent){
 const bad=()=>{throw Error(trustTr('Réponse d’observation incomplète ou incohérente.'))};
 const fields=['schema_version','available','source','scope','attribution','effective_policy','revision','events','attempts','emitted','kernel_lost','omitted','cleanup_verified','complete','global_qualification','stop_reason'];
 if(!r||fields.some(k=>r[k]===null||r[k]===undefined)||r.schema_version!==1||typeof r.available!=='boolean'||r.source!=='ebpf'||r.scope!=='registered_main_thread'||r.cleanup_verified!==true||r.complete!==false||r.global_qualification!==false||!Number.isSafeInteger(r.revision)||!Array.isArray(r.events)||r.events.length>p.max_events)bad();
 if(Object.keys(p).some(k=>r.effective_policy[k]!==p[k]))bad();
 const a=r.attribution;if(a.agent_id!==agent||!a.mission_id||!a.task_id||!a.attempt_id||!Number.isSafeInteger(a.pid)||a.pid<=0||!/^\d+$/.test(a.pid_start_time)||!/^[a-f0-9]{64}$/.test(a.resource_digest))bad();
 for(const k of ['attempts','emitted','kernel_lost','omitted'])if(typeof r[k]!=='string'||!/^(0|[1-9][0-9]*)$/.test(r[k])||BigInt(r[k])>18446744073709551615n)bad();
 const attempts=BigInt(r.attempts),emitted=BigInt(r.emitted),lost=BigInt(r.kernel_lost),omitted=BigInt(r.omitted);
 if(attempts<emitted||lost!==attempts-emitted||emitted<BigInt(r.events.length)||omitted!==emitted-BigInt(r.events.length))bad();
 if(!r.available){if(r.stop_reason!=='unavailable'||r.events.length||attempts||emitted||lost||omitted)bad()}
 else if(!['window','event_limit'].includes(r.stop_reason)||(r.stop_reason==='event_limit'&&r.events.length!==p.max_events))bad();
}

let trustObserveController=null;
function trustObservePolicyRequest(form){
 const p={enabled:form.kernel_observation_enabled.checked};
 for(const key of ['window_seconds','timeout_seconds','ring_bytes','max_events']){
  const text=form['kernel_observation_'+key].value.trim(),value=Number(text),bounds=trustView.policy_bounds['kernel_observation_'+key];
  if(!text||!Number.isSafeInteger(value)||value<bounds.min||value>bounds.max)throw Error(trustTr('Observation noyau : renseignez des entiers positifs dans les limites techniques.'));p[key]=value;
 }
 if(p.window_seconds>=p.timeout_seconds||!Number.isInteger(Math.log2(p.ring_bytes)))throw Error(trustTr('La fenêtre doit être inférieure au délai total et le ring une puissance de deux.'));
 return p;
}
function openTrustObservePolicy(form){const p=trustView.kernel_observation;form.kernel_observation_enabled.checked=!!p.enabled;for(const key of ['window_seconds','timeout_seconds','ring_bytes','max_events'])form['kernel_observation_'+key].value=p[key];trust('trust-observe-bounds').textContent=trustTr('Limites techniques de représentation :')+' '+JSON.stringify({ring_bytes:trustView.policy_bounds.kernel_observation_ring_bytes,max_events:trustView.policy_bounds.kernel_observation_max_events})}
function renderTrustObserve(){trust('trust-observe-start').disabled=!trustView.kernel_observation.enabled||!!trustObserveController;trust('trust-observe-status').textContent=trustTr(trustView.kernel_observation.enabled?'Observation explicite disponible pour une session agent active.':'Activez l’observation des agents dans Admin.');trust('trust-observe-result').hidden=true}
trust('trust-observe-form').addEventListener('submit',async event=>{
 event.preventDefault();if(trustObserveController)return;
 const button=trust('trust-observe-start'),cancel=trust('trust-observe-cancel'),status=trust('trust-observe-status'),output=trust('trust-observe-result'),revision=trustView.revision;
 trustObserveController=new AbortController();button.disabled=true;cancel.hidden=false;output.hidden=true;status.className='notice info';status.textContent=trustTr('Observation en cours… Vous pouvez l’annuler.');
 try{
  const result=await trustAPI('/api/v1/trust-actions?action=kernel-observe',{agent_id:trust('trust-observe-agent').value.trim()},trust('trust-observe-token').value.trim(),JSON.stringify,trustObserveController.signal);
  validateTrustObserveResponse(result,trustView.kernel_observation,trust('trust-observe-agent').value.trim());
  if(result.revision!==revision||trustView.revision!==revision)throw Error(trustTr('Configuration modifiée : recommencez la vérification.'));
  output.textContent=JSON.stringify(result,null,2);output.hidden=false;
  status.textContent=trustTr(result.available?'Observations reçues. La couverture complète et la qualification globale restent non établies.':'Collecte eBPF indisponible. Aucun résultat noyau qualifié.');status.className='notice '+(result.available?'info':'alert');
 }catch(error){status.className='notice alert';status.textContent=error.name==='AbortError'?trustTr('Observation annulée. Aucun résultat validé.'):error.message}
 finally{const restore=document.activeElement===cancel;trustObserveController=null;cancel.hidden=true;button.disabled=!trustView.kernel_observation.enabled;if(restore)button.focus()}
});
trust('trust-observe-cancel').addEventListener('click',()=>trustObserveController?.abort());
if(trustView)renderTrustObserve();
