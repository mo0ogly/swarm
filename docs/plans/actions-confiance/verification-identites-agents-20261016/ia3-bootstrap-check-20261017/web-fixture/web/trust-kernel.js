'use strict';
function trustKernelPolicyRequest(form){
 const seconds=form.kernel_check_timeout_seconds.value.trim(),bytes=form.kernel_check_ring_bytes.value.trim(),timeout=Number(seconds),ring=Number(bytes),bounds=trustView.policy_bounds.kernel_check_ring_bytes;
 if(!seconds||!bytes||!Number.isSafeInteger(timeout)||timeout<=0||timeout>trustView.policy_bounds.kernel_check_timeout_seconds.max||!Number.isSafeInteger(ring)||ring<bounds.min||ring>bounds.max||!Number.isInteger(Math.log2(ring)))throw Error(trustTr('Le diagnostic noyau exige une durée positive et un ring puissance de deux dans les limites techniques.'));
 return{enabled:form.kernel_check_enabled.checked,timeout_seconds:timeout,ring_bytes:ring};
}
function openTrustKernelPolicy(form){const p=trustView.kernel_check;form.kernel_check_enabled.checked=!!p.enabled;form.kernel_check_timeout_seconds.value=p.timeout_seconds;form.kernel_check_ring_bytes.value=p.ring_bytes;trust('trust-kernel-bounds').textContent=trustTr('Capacité technique du ring (octets) :')+' '+trustView.policy_bounds.kernel_check_ring_bytes.min+'–'+trustView.policy_bounds.kernel_check_ring_bytes.max}
function renderTrustKernel(){const enabled=trustView.kernel_check.enabled;trust('trust-kernel-check').disabled=!enabled;trust('trust-kernel-status').textContent=trustTr(enabled?'Diagnostic disponible sur la fixture possédée uniquement.':'Activez le diagnostic noyau dans Admin.');trust('trust-kernel-status').className='notice info';trust('trust-kernel-result').hidden=true}
trust('trust-kernel-check').addEventListener('click',async()=>{
 const button=trust('trust-kernel-check'),status=trust('trust-kernel-status'),output=trust('trust-kernel-result'),revision=trustView.revision;
 button.disabled=true;output.hidden=true;status.className='notice info';status.textContent=trustTr('Diagnostic noyau en cours…');
 try{const result=await trustAPI('/api/v1/trust-actions?action=kernel-check',{},trust('trust-kernel-token').value.trim());if(trustView.revision!==revision||result.revision!==revision)throw Error(trustTr('Configuration modifiée : recommencez la vérification.'));output.textContent=JSON.stringify(result,null,2);output.hidden=false;
  status.textContent=trustTr(!result.available?'Collecte eBPF indisponible. Aucun résultat noyau qualifié.':result.fixture_verified?'Fixture observée. Les agents et la qualification globale restent à vérifier.':'Diagnostic incomplet. Aucun résultat noyau qualifié.');status.className='notice '+(result.available&&result.fixture_verified?'info':'alert');
 }catch(error){status.textContent=error.message;status.className='notice alert'}finally{button.disabled=!trustView.kernel_check.enabled}
});
if(trustView)renderTrustKernel();
