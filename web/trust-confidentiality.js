'use strict';
/** BLP policy administration and explicit object attribution for the native
 * Trust dialog. Saving a policy or attributing one exact object does not
 * activate transversal enforcement; explicit preparation quarantine blocks
 * controlled sends without granting any classified-send authority. No
 * credentials enter storage or reports. Attribution of existing objects stays
 * a per-object operator decision; no enumeration or bulk scan exists here. */
globalThis.SwarmTrustConfidentiality={
 /** Install explicit read/preview/save/attribute gestures using the shared
  * protected API. Returns synchronization/cleanup hooks; no request is sent
  * on installation. */
 install({api,guarded,t}){
  const $=id=>document.getElementById(id);let view=null,proposal=null,attribution=null;
  /** Prevent double writes and stale previews even after another Admin gesture. */
  function sync(){$('blp-save').disabled=!proposal;$('blp-attribute-apply').disabled=!attribution;}
  /** Clear private form material and obsolete revisions when Admin closes. */
  function reset(){view=null;proposal=null;attribution=null;$('blp-policy').value='';$('blp-reason').value='';$('blp-status').textContent='';$('blp-object-digest').value='';$('blp-object-path').value='';$('blp-object-level').value='';$('blp-object-compartments').selectedIndex=-1;$('blp-object-reason').value='';$('blp-attribute-status').textContent='';sync();}
  /** Read exact persisted policy; an unconfigured installation stays unconfigured.
   * Never populate a permissive default or infer grants from displayed roles. */
  async function load(){
   const {result}=await api('blp-show','{}');
   if(result.capability?.runtime_enforced!==false||typeof result.capability.preparation_quarantined!=='boolean'||typeof result.trust_revision!=='string')throw Error(t('Réponse invalide'));
   view=result;proposal=null;attribution=null;
   const history=result.record?.history,current=history?.length?history[history.length-1]:null;
   $('blp-policy').value=current?JSON.stringify(current.policy,null,2):'';
   /** Level and compartment options come exactly from the re-read policy;
    * the page never invents or defaults a label. */
   const levels=current?.policy.levels||[],compartments=current?.policy.compartments||[];
   $('blp-object-level').replaceChildren(...levels.map(level=>{const o=document.createElement('option');o.value=level.id;o.textContent=level.id;return o;}));
   $('blp-object-compartments').replaceChildren(...compartments.map(name=>{const o=document.createElement('option');o.value=name;o.textContent=name;return o;}));
   $('blp-object-compartments').selectedIndex=-1;
   $('blp-status').textContent=t(result.capability.preparation_quarantined?'Envois de préparation en quarantaine ; protection transversale incomplète':result.capability.configured?'Politique enregistrée ; protection transversale non active':'Aucune politique de confidentialité enregistrée');sync();
  }
  /** Encode revision decimals without Number rounding or marker replacement. */
  function revision(value){if(typeof value!=='string'||!/^(0|[1-9][0-9]*)$/.test(value))throw Error(t('Réponse invalide'));return value;}
  /** Invalidate preview on every input, including credentials and other settings. */
  $('configuration').addEventListener('input',()=>{proposal=null;sync();});
  $('blp-load').addEventListener('click',()=>guarded(load));
  $('blp-example').addEventListener('click',()=>{
   proposal=null;
   $('blp-policy').value=JSON.stringify({schema_version:1,levels:[
    {id:'public',name_fr:'Public',name_en:'Public'},
    {id:'internal',name_fr:'Interne',name_en:'Internal'},
    {id:'confidential',name_fr:'Confidentiel',name_en:'Confidential'},
    {id:'restricted',name_fr:'Restreint',name_en:'Restricted'}
   ],compartments:[],clearances:{},preparation_quarantine:false},null,2);
   $('blp-status').textContent=t('Exemple sans habilitation ; aucune modification enregistrée');sync();
  });
  $('blp-preview').addEventListener('click',()=>guarded(async()=>{
   proposal=null;sync();if(!view)throw Error(t('Relire la politique avant prévisualisation'));
   const raw='{"schema_version":1,"event_id":'+JSON.stringify('blp-'+crypto.randomUUID())+
    ',"expected_revision":'+revision(view.capability.policy_revision||'0')+
    ',"expected_trust_revision":'+revision(view.trust_revision)+',"reason":'+JSON.stringify($('blp-reason').value)+
    ',"policy":'+$('blp-policy').value+'}';
   // Preserve exact user JSON so the engine can reject duplicate keys.
   const {result}=await api('blp-policy-preview',raw);
   if(result.applied!==false||result.runtime_enforced!==false)throw Error(t('Réponse invalide'));
   proposal=raw;$('blp-status').textContent=t('Prévisualisation validée ; aucun changement appliqué');sync();
  }));
  $('blp-save').addEventListener('click',()=>guarded(async()=>{
   if(!proposal)return;const raw=proposal;proposal=null;sync();
   const {result}=await api('blp-policy-apply',raw);
   if(result.applied!==true||result.runtime_enforced!==false)throw Error(t('Réponse invalide'));
   await load();$('blp-status').textContent=t(view.capability.preparation_quarantined?'Envois de préparation en quarantaine ; protection transversale incomplète':'Politique enregistrée et relue ; protection transversale non active');
  }));
  /** Exact object attribution: digest of the exact canonical action bytes, the
   * absolute project path, an explicit classification chosen from the re-read
   * policy and a non-empty reason. Preview then apply replays the SAME event
   * bytes; one gesture covers exactly one object. */
  $('blp-attribute-preview').addEventListener('click',()=>guarded(async()=>{
   attribution=null;sync();if(!view)throw Error(t('Relire la politique avant prévisualisation'));
   const digest=$('blp-object-digest').value.trim(),projectPath=$('blp-object-path').value.trim();
   if(!/^[0-9a-f]{64}$/.test(digest))throw Error(t('Empreinte canonique invalide ; 64 hexadécimales minuscules exigées'));
   if(!projectPath.startsWith('/')||projectPath.includes('\0')||!projectPath)throw Error(t('Chemin de projet absolu exigé'));
   const level=$('blp-object-level').value,reason=$('blp-object-reason').value;
   if(!level)throw Error(t('Relire la politique avant prévisualisation'));
   if(!reason.trim())throw Error(t('Motif exigé'));
   const selected=Array.from($('blp-object-compartments').selectedOptions).map(o=>o.value);
   const raw=JSON.stringify({schema_version:1,event_id:'blp-'+crypto.randomUUID(),
    expected_revision:Number(revision(view.capability.policy_revision||'0')),
    expected_trust_revision:Number(revision(view.trust_revision)),
    canonical_digest:digest,project_path:projectPath,
    classification:{level,compartments:selected},reason});
   const {result}=await api('blp-object-attribute',raw);
   if(result.attributed!==true||result.runtime_enforced!==false)throw Error(t('Réponse invalide'));
   attribution=raw;$('blp-attribute-status').textContent=t('Prévisualisation validée ; aucun changement appliqué');sync();
  }));
  $('blp-attribute-apply').addEventListener('click',()=>guarded(async()=>{
   if(!attribution)return;const raw=attribution;attribution=null;sync();
   const {result}=await api('blp-object-attribute',raw);
   if(result.attributed!==true||result.runtime_enforced!==false)throw Error(t('Réponse invalide'));
   await load();$('blp-attribute-status').textContent=t('Attribution enregistrée et relue ; l’objet est classé sous la politique courante');
  }));
  return {sync,reset};
 }
};
