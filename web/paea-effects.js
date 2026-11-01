'use strict';
/** Read-only observation of real PAEA effect surfaces. Consumes only the three
 * public guarded reads: GET /api/v1/paea/git-call/preview (protected preview of
 * the exact held prepared action, addressed to a future approver, no consent),
 * GET /api/v1/paea/git-call (minimal startup observation: `bound` only, no
 * consumption) and
 * GET /api/v1/paea/reconcile-effects (recomputed journal reconciliation). No
 * POST, no one-shot call consumption, no authority, no repair: a preview is not
 * an agreement and `verified=true` stays recomputed correspondence, never
 * authenticity or acceptance. Any failure keeps the exact refusal code visible.
 */
(() => {
  const $=id=>document.getElementById(id),t=(s,v)=>globalThis.SwarmI18n.t(s,v);
  /** Render a translated visible state, never an inferred success. */
  function status(id,kind,message){$(id).dataset.state=kind;$(id).textContent=t(message)}
  /** Read one guarded GET endpoint; refuse to hide a refusal code. */
  async function read(path){const r=await fetch(path,{method:'GET',cache:'no-store',credentials:'same-origin'});const raw=await r.text();let body;try{body=JSON.parse(raw)}catch{throw Error(t('Réponse illisible ; vérifier l’état avant reprise'))}if(!r.ok)throw Error(body?.failure?.code||body?.error||String(r.status));return body}
  /** Startup observation shown as-is; nothing is added or reinterpreted.
   * The status route serves the minimal projection only (the `bound` boolean;
   * `capability` stays Null — no request-derived call/service/operation or
   * digest, BLP-RET-05): the exact capability metadata of the held action is
   * presented solely through the guarded preview read. */
  function capability(view){
    if(!view.bound){status('gitcall-status','empty','Aucun appel Git préparé n’est lié au démarrage du service. La préparation deux phases (CLI ou enrôlement) reste la voie réelle.');return}
    if(view.capability!=null){status('gitcall-status','error','Rapport hors contrat : la route de statut a servi une capacité dérivée de la requête préparée, ce que la frontière lecteur interdit. Aucun appel effectué.');return}
    $('gitcall-detail').hidden=false;
    status('gitcall-status','success','Un appel Git préparé est lié au démarrage du service : aucune métadonnée dérivée n’est exposée sous cette session (frontière lecteur). Le détail exact reste lisible via la lecture gardée, adressée à un futur approbateur.');
  }
  /** Explicit journal state per domain; findings list exact refusal codes. */
  function journal(domain,heading){
    const stateBox=$(domain+'-state'),list=$(domain+'-findings');list.replaceChildren();
    if(heading.state==='uninitialized'){stateBox.textContent=t('Journal non initialisé : aucun effet rapproché (état explicite, pas un succès vide).');return}
    if(heading.state==='corrupt'){
      stateBox.textContent=t('Schéma altéré ou incomplet : aucune ligne rapprochée, aucune réparation tentée.');
      const ul=list;for(const code of heading.codes??[]){const li=document.createElement('li');li.textContent=code;ul.append(li)}
      return;
    }
    if(heading.state==='consistent'){
      const n=heading.effectCount??heading.effect_count??0;
      const findings=heading.findings??[];
      const ok=findings.filter(f=>f.verified).length,ko=findings.length-ok;
      stateBox.textContent=t('{n} effet(s) durable(s) rapproché(s) : {ok} correspondance(s) recalculée(s), {ko} écart(s) signalé(s).',{n,ok,ko});
      if(!findings.length){const li=document.createElement('li');li.textContent=t('Aucun effet durable enregistré dans ce journal.');list.append(li);return}
      for(const f of findings){
        const li=document.createElement('li');
        li.textContent=t('intent {intent} · effet {digest} · {verdict} {codes}',{intent:f.intent_id,digest:f.action_digest,verdict:f.verified?t('correspondance recalculée'):t('écart'),codes:(f.refusals??[]).join(' ')});
        list.append(li);
      }
      return;
    }
    stateBox.textContent=t('État de journal inconnu : rapport hors contrat, aucun verdict affiché.');
  }
  function report(view){
    if(view.schema_version!==1){status('reconcile-status','error','Rapport hors contrat (version de schéma inattendue) ; original conservé dans le rapport technique.');$('reconcile-report').hidden=false;$('reconcile-report-raw').textContent=JSON.stringify(view,null,2);return}
    $('reconcile-report').hidden=false;$('reconcile-report-raw').textContent=JSON.stringify(view,null,2);
    journal('settlement',view.settlement??{});
    journal('git',view.git??{});
    status('reconcile-status','success','Rapprochement recalculé affiché pour les deux domaines réels. Correspondance recalculée n’établit pas l’authenticité ; séparation UID, ancrages externes et stockage immuable restent des prérequis ouverts.');
  }
  /** Add one visible dt/dd pair; values are shown verbatim, never reinterpreted. */
  function pair(dl,key,value){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=t(key);dd.textContent=String(value);dl.append(dt,dd)}
  /** Explicit preview of the exact held action, addressed to a future approver.
   * Refusals, unavailability and stale prerequisites stay visible failures;
   * no consent button exists and no field is presented as an agreement. */
  function preview(view){
    if(view.schema_version!==1){status('preview-status','error',t('Rapport hors contrat (version de schéma inattendue) ; original conservé dans le rapport technique.'));$('preview-detail').hidden=false;$('preview-raw-body').textContent=JSON.stringify(view,null,2);return}
    if(!view.available){
      // Reader-frontier refusals (paea-reader-*) mean the prepared call IS
      // bound but this session/credential holds no current classification
      // authorization: say exactly that, never the "nothing bound" text —
      // and show no held field (the reply carries none either).
      const readerRefused=String(view.reason??'').startsWith('paea-reader-');
      status('preview-status','empty',
        readerRefused
          ? t('Lecture refusée : {code}. L’action préparée est liée, mais son contenu classifié n’est présenté qu’à un lecteur autorisé par une attribution courante de confidentialité ; aucune donnée de l’action n’est affichée.',{code:view.reason??''})
          : t('Aperçu explicitement indisponible : {code}. Aucun appel Git préparé n’est lié au démarrage du service ; la préparation deux phases reste la voie réelle.',{code:view.reason??''}));
      for(const id of ['preview-identity','preview-witness','preview-declared','preview-limits-list'])$(id).replaceChildren();
      $('preview-detail').hidden=true;
      return;
    }
    $('preview-detail').hidden=false;$('preview-raw-body').textContent=JSON.stringify(view,null,2);
    const canon=view.canonical??{},current=view.current??{},cap=view.capability??{},limits=view.limits??{};
    const identity=$('preview-identity');identity.replaceChildren();
    pair(identity,'Empreinte d’action',canon.action_digest??'');
    pair(identity,'Intention métier',canon.intent_id??'');
    pair(identity,'Empreinte de politique',canon.policy_digest??'');
    pair(identity,'Correspondance de l’empreinte d’action',canon.digest_matches?t('oui'):t('non'));
    const witness=$('preview-witness');witness.replaceChildren();
    pair(witness,'Racines physiquement liées',current.roots_bound?t('oui'):t('non'));
    pair(witness,'Révision de confiance déclarée',current.declared_trust_revision??'');
    if(current.trust_revision===null||current.trust_revision===undefined){pair(witness,'Journal de confiance absent',t('non'))}
    else{pair(witness,'Révision de confiance actuelle',current.trust_revision)}
    pair(witness,'Révision de confiance courante',current.trust_revision_current?t('oui'):t('non'));
    if(!current.trust_revision_current){status('preview-status','error',t('Préparation périmée : la révision de confiance observée diffère de la révision déclarée. Aucun accord n’est sollicité sur un état périmé ; reprendre la préparation avant toute suite.'));return}
    if(!current.roots_bound){status('preview-status','error',t('Prérequis non réuni : les témoins physiques des deux racines ne correspondent plus à la préparation. Aucun accord n’est sollicité sur cet état ; reprendre la préparation avant toute suite.'));return}
    if(!canon.digest_matches){status('preview-status','error',t('Empreinte d’action recalculée différente de l’empreinte liée : état signalé tel quel, aucune normalisation ni accord sollicité.'));return}
    const declared=$('preview-declared');declared.replaceChildren();
    for(const [key,value] of Object.entries(cap))pair(declared,key,String(value));
    const proposal=view.proposal??{},action=proposal.action??{};
    const labels={payment:{invoice_id:'Facture',invoice_version:'Version de facture',amount_minor:'Montant en unités mineures',currency:'Devise',beneficiary_account_id:'Compte bénéficiaire'},git:{repository_id:'Identifiant du service attendu',target_ref:'Référence Git cible',expected_old_oid:'OID précédent attendu',candidate_oid:'OID candidat'}};
    for(const [key,value] of Object.entries(action)){
      if(key==='kind')continue;
      // Exact held value rendered verbatim as the visible term; the translated
      // field label only names the term. Never substitute or normalize value.
      pair(declared,labels[action.kind]?.[key]??key,String(value));
    }
    pair(declared,'Projet',view.context?.project_id??'');
    pair(declared,'Mandat',view.context?.mandate_id??'');
    pair(declared,'Révision de politique',view.context?.policy_revision??'');
    const limitList=$('preview-limits-list');limitList.replaceChildren();
    for(const [key,value] of Object.entries(limits))pair(limitList,key,String(value));
    status('preview-status','success',t('Aperçu exact de l’action préparée détenue par le moteur, présenté avant tout accord. Aucun consentement n’est capturé ici : ce contenu n’est pas un accord ; poursuivre ensuite le consentement réel sur cette même action.'));
  }
  function navigation(){for(const link of document.querySelectorAll('[data-paea-link]')){const url=new URL(link.href);url.searchParams.set('lang',globalThis.SwarmI18n.language);url.searchParams.set('theme',document.documentElement.dataset.theme);link.href=url.pathname+url.search}}
  async function load(){
    status('preview-status','loading','Chargement…');status('gitcall-status','loading','Chargement…');status('reconcile-status','loading','Chargement…');
    $('preview-detail').hidden=true;$('gitcall-detail').hidden=true;$('reconcile-report').hidden=true;
    try{preview(await read('/api/v1/paea/git-call/preview'))}catch(error){status('preview-status','error',t('Refus : {code}',{code:error.message}))}
    try{capability(await read('/api/v1/paea/git-call'))}catch(error){status('gitcall-status','error',t('Refus : {code}',{code:error.message}))}
    try{report(await read('/api/v1/paea/reconcile-effects'))}catch(error){status('reconcile-status','error',t('Refus : {code}',{code:error.message}))}
  }
  const theme=new URL(location.href).searchParams.get('theme');if(['etat','sombre'].includes(theme))document.documentElement.dataset.theme=theme;
  $('paea-theme').addEventListener('click',()=>{document.documentElement.dataset.theme=document.documentElement.dataset.theme==='etat'?'sombre':'etat';navigation()});
  navigation();load();
})();
