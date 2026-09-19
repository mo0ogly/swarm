'use strict';
// Read-only rendering of the server evidence projection. Business states stay
// invariant; only the surrounding labels are translated.
const SwarmEvidenceContract=(()=>{
 const known=(value,allowed,field)=>{if(!allowed.includes(value))throw new Error('unknown evidence '+field+': '+value);return value};
 const text=(e,tr=s=>s)=>{
  if(!e)throw new Error('missing evidence contract');
  known(e.freshness,['fresh','stale','unknown'],'freshness');
  known(e.report_review?.state,['not_configured','running','passed','changes_requested','error','stale'],'report review state');
  known(e.controls?.state,['unknown','passed','failed'],'controls state');
  known(e.acceptance?.state,['not_accepted','pending','accepted','stale','waived'],'acceptance state');
  const lines=[tr('Tentative')+' : '+(e.attempt_id||'unknown'),tr('Révision lue')+' : '+String(e.revision??'unknown'),tr('Fraîcheur')+' : '+e.freshness,
   tr('Revue du rapport')+' : '+e.report_review.state+' · '+tr('date')+' '+(e.report_review.at||'unknown'),
   tr('Acceptation')+' : '+e.acceptance.state+' · '+tr('révision')+' '+(e.acceptance.revision||'unknown')+' · '+tr('date')+' '+(e.acceptance.at||'unknown')];
  for(const c of e.controls.items||[]){const command=(c.command||[]).length?c.command.join(' '):'unknown';lines.push(tr('Contrôle')+' '+c.id+' : '+tr('tentative')+' '+(c.attempt_id||'unknown')+' · '+c.execution+' · '+tr('révision')+' '+(c.revision||'unknown')+' · '+tr('SHA candidat')+' '+(c.candidate_sha||'unknown')+' · '+tr('commande')+' '+command+' · '+tr('code de sortie')+' '+(c.exit_code??'unknown')+' · '+tr('début')+' '+(c.started_at||'unknown')+' · '+tr('fin')+' '+(c.finished_at||'unknown')+' · '+tr('fraîcheur')+' '+c.freshness)}
  for(const limit of [...(e.report_review.limits||[]),...(e.limits||[])])lines.push(tr('Limite')+' : '+limit);
  return lines.join('\n');
 };
 const render=(container,e,tr=s=>s)=>{container.dataset.evidenceFreshness=e.freshness;container.dataset.controlsState=e.controls.state;container.dataset.acceptanceState=e.acceptance.state;container.textContent=text(e,tr);return container};
 return Object.freeze({text,render});
})();
