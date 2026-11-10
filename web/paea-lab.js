'use strict';
/** Read an explicitly supplied laboratory report locally. No upload, campaign
 * launch, authenticated approval, integrity verification or live effect occurs.
 * The producer verifies retained recipe evidence; this reader checks shape only.
 */
(() => {
  const $=id=>document.getElementById(id),t=s=>globalThis.SwarmI18n.t(s);
  const labels={
    test_t04_substitution_then_equivalent_json_recovery:'T04 — substitution et reprise',
    test_t06_t21_crash_new_attempt_exact_marker_and_reconciliation:'T06/T21 — réponse perdue et rapprochement',
    test_t05_t17_ordered_revocation_first_use_and_opposite_order:'T05/T17 — révocation ordonnée avant première utilisation',
    test_unavailable_oracle_stays_unknown_then_recovers_without_creation:'Oracle indisponible : UNKNOWN puis reprise',
    test_git_stale_and_symbolic_target_refuse_then_recover:'Git : référence périmée ou symbolique',
    test_output_failure_after_effect_is_unknown_and_reconciles:'Échec de sortie après effet : UNKNOWN',
    test_invoice_alias_is_a_counterexample_without_authoritative_registry:'Aliases bruts : unicité réfutée',
    test_resolved_invoice_aliases_converge_on_one_effect:'Aliases résolus : un effet dans la fixture'
  };
  let raw=null,busy=false;
  /** Remove all prior content/export immediately; invalid input cannot retain success. */
  function clear(){raw=null;$('lab-download').disabled=true;$('lab-results').hidden=true;$('lab-tests').replaceChildren();$('lab-identities').replaceChildren()}
  /** Render a translated visible state, never a protected conformance verdict. */
  function status(kind,message){$('lab-status').dataset.state=kind;$('lab-status').textContent=t(message)}
  /** Closed objects and exact fixture markers reject empty, unknown or partial reports.
   * Eight cases are this recipe's versioned contract, not a product execution limit.
   * Local hashes are provenance claims; no signature/evidence verification occurs.
   */
  function validate(value){
    const keys=['schema_version','scope','recipe_id','binary_sha256','recipe_result_sha256','source_inventory_sha256','calls_sha256','tests','refuted_claim','authority','isolation','independent_evaluation','protected_invariants'];
    const exact=(o,k)=>o&&typeof o==='object'&&!Array.isArray(o)&&Object.keys(o).length===k.length&&k.every(n=>Object.hasOwn(o,n));
    if(!exact(value,keys)||value.schema_version!==1||value.scope!=='isolated-laboratory'||value.recipe_id!=='paea-pilot-lab'||
      !['binary_sha256','recipe_result_sha256','source_inventory_sha256','calls_sha256'].every(n=>typeof value[n]==='string'&&/^[a-f0-9]{64}$/.test(value[n]))||
      value.refuted_claim!=='raw-invoice-identifiers-guarantee-global-debt-uniqueness'||value.authority!=='simulated'||value.isolation!=='same-uid'||
      value.independent_evaluation!=='not-established'||value.protected_invariants!=='not-established'||!Array.isArray(value.tests)||value.tests.length!==Object.keys(labels).length||
      value.tests.some(v=>!exact(v,['id','status'])||!Object.hasOwn(labels,v.id)||v.status!=='PASS')||new Set(value.tests.map(v=>v.id)).size!==Object.keys(labels).length)
      throw Error(t('Rapport de laboratoire invalide ou incomplet.'));
  }
  /** Read one exact UTF-8 file, validate and show assertions with their limitations.
   * Disable the field during async reading; any failure clears all stale state.
   * No network or persistence; browser decoding/reading failures remain visible.
   */
  async function read(event){
    event.preventDefault();if(busy)return;clear();busy=true;$('lab-inputs').disabled=true;$('lab-read').disabled=true;status('loading','Lecture du rapport de laboratoire…');
    try{
      const file=$('lab-file').files[0];if(!file)throw Error(t('Sélectionner un rapport de laboratoire.'));
      const bytes=await file.arrayBuffer();let text;
      try{text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes)}catch{throw Error(t('Fichier UTF-8 invalide ; choisir un fichier valide.'))}
      let value;try{value=JSON.parse(text)}catch{throw Error(t('Rapport de laboratoire invalide ou incomplet.'))}validate(value);
      for(const item of value.tests){const li=document.createElement('li');li.textContent=t(labels[item.id])+' — '+t('Test local réussi');$('lab-tests').append(li)}
      for(const [label,key] of [['Binaire exercé','binary_sha256'],['Résultat de recette','recipe_result_sha256'],['Inventaire des sources','source_inventory_sha256'],['Index des appels et oracles','calls_sha256']]){
        const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=t(label);dd.textContent=value[key];$('lab-identities').append(dt,dd)
      }
      raw=text;$('lab-download').disabled=false;$('lab-results').hidden=false;status('read','Huit tests déclarés réussis. Protection et authenticité non établies.');
    }catch(error){clear();status('error',error.message)}finally{busy=false;$('lab-inputs').disabled=false;$('lab-read').disabled=false}
  }
  /** Export only the exact accepted input; no author, signature or result is added. */
  function download(){if(raw===null||busy)return;const url=URL.createObjectURL(new Blob([raw],{type:'application/json'})),a=document.createElement('a');a.href=url;a.download='paea-lab-report.json';a.click();URL.revokeObjectURL(url)}
  /** Preserve language/theme between the laboratory and existing PAEA views. */
  function navigation(){for(const link of document.querySelectorAll('[data-paea-link]')){const url=new URL(link.href);url.searchParams.set('lang',globalThis.SwarmI18n.language);url.searchParams.set('theme',document.documentElement.dataset.theme);link.href=url.pathname+url.search}}
  const theme=new URL(location.href).searchParams.get('theme');if(['etat','sombre'].includes(theme))document.documentElement.dataset.theme=theme;
  $('paea-theme').addEventListener('click',()=>{document.documentElement.dataset.theme=document.documentElement.dataset.theme==='etat'?'sombre':'etat';navigation()});
  $('lab-form').addEventListener('submit',read);$('lab-file').addEventListener('change',()=>{if(!busy){clear();status('empty','Sélectionner un rapport de laboratoire.')}});$('lab-download').addEventListener('click',download);
  navigation();status('empty','Sélectionner un rapport de laboratoire.');$('lab-read').disabled=false;
})();
