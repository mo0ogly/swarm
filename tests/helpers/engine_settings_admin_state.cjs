/** Wait for actual Admin success OR visible refusal; throw outside polling.
 * Puppeteer predicates must return terminal state rather than throw inside the
 * poller, where an exception may leave a zero-watchdog wait pending. This helper
 * adds no deadline/retry, ignores no backend error and performs no UI mutation.
 * A missing DOM stays pending for external progress diagnosis. Caller owns the
 * page, diagnostics and phase; unknown phases refuse before browser interaction.
 */
'use strict';
async function waitForSettingsState(page,phase){
 if(!['loaded','preview','saved'].includes(phase))throw Error('Unknown Admin state phase');
 const handle=await page.waitForFunction(phase=>{
  const status=document.querySelector('.engine-settings-admin [role="status"]');
  if(status?.classList.contains('alert'))return {status:'failure',message:status.textContent};
  const modal=document.querySelector('dialog[data-engine-settings]'),error=modal?.querySelector('[role="alert"]');
  if(error&&!error.hidden)return {status:'failure',message:error.textContent};
  const edit=document.querySelector('[data-action="edit"]');
  if(phase==='loaded'&&edit&&!edit.disabled)return {status:'ready'};
  if(phase==='saved'&&!modal&&edit&&!edit.disabled)return {status:'ready'};
  const preview=modal?.querySelector('[data-setting-preview]');
  if(phase==='preview'&&preview&&!preview.hidden)return {status:'ready'};
  return false;
 },{},phase);
 let state,primary;
 try{state=await handle.jsonValue();}catch(e){primary=e;}
 try{await handle.dispose();}catch(e){if(primary)throw new AggregateError([primary,e],'Admin state read/cleanup failed');throw e;}
 if(primary)throw primary;
 if(state.status==='failure'){const e=Error(state.message||'Admin terminal failure');e.code='admin-state-failure';e.phase=phase;throw e;}
 return state;
}
module.exports={waitForSettingsState};
