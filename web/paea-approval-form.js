'use strict';
/** Explicit approval form for the single frozen PAEA approval command held by
 * the native service. Consumes only the three guarded approval endpoints:
 * GET /api/v1/paea/git-call/approval (startup capability), GET
 * /api/v1/paea/git-call/approval/presentation (the authorized projection:
 * operation, request digest, canonical identity, engine-derived agreement
 * terms and current witness state) and POST /api/v1/paea/git-call/approval
 * (single-intent consumption with an EMPTY body). The browser never invents
 * JSON, never selects a root, action or term and never transports command
 * bytes: the command stays server-side (bound approval) and the confirmation
 * is connected to the digest presented above. Consumption sends only the
 * explicit checkbox confirmation and the approver's enrollment password (sent
 * once in the Authorization header, erased immediately after use, never
 * stored). A business refusal ack (`state:"denied"`, HTTP 200 included) is
 * never shown as a success; any refusal keeps its exact code visible. No Git
 * effect, no second approval system, no consent captured by display alone.
 */
(() => {
  const $=id=>document.getElementById(id),t=(s,v)=>globalThis.SwarmI18n.t(s,v);
  let csrf=null,busy=false;
  /** Render a translated visible state, never an inferred success. */
  function status(id,kind,message){$(id).dataset.state=kind;$(id).textContent=t(message)}
  /** Read one guarded GET endpoint; refuse to hide a refusal code. */
  async function read(path){const r=await fetch(path,{method:'GET',cache:'no-store',credentials:'same-origin'});const raw=await r.text();let body;try{body=JSON.parse(raw)}catch{throw Error(t('Réponse illisible ; vérifier l’état avant reprise'))}if(!r.ok)throw Error(body?.failure?.code||body?.error||String(r.status));return body}
  /** Add one visible dt/dd pair; values are shown verbatim, never reinterpreted. */
  function pair(dl,key,value){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=t(key);dd.textContent=String(value);dl.append(dt,dd)}
  /** Render the authorized projection verbatim: the public operation beside
   * the engine-derived agreement terms (binding, resource, amount,
   * beneficiary, revision). Term rows render only present, non-empty values:
   * the recipe terms carry deliberately empty action_id/content_digest (the
   * command identity stays the request digest above), and a labeled empty
   * row would present a private-field shape the projection does not serve.
   * The private command fields (action_id, mandate_id, content_digest) are
   * never labeled here; the request digest stands as the command identity. */
  function presentation(view){
    const declared=$('approval-declared');declared.replaceChildren();
    const terms=view.agreement_terms??{};
    const labels={binding:'Type de lien',resource:'Ressource',amount_minor:'Montant en unités mineures',currency:'Devise',beneficiary:'Bénéficiaire',revision:'Révision de confiance'};
    for(const [key,value] of Object.entries(terms))if(value!==null&&value!==undefined&&String(value).trim()!=='')pair(declared,labels[key]??key,String(value));
    const witness=$('approval-witness');witness.replaceChildren();
    const current=view.current??{},canon=view.canonical??{};
    pair(witness,'Empreinte de la commande figée',view.request_digest??'');
    pair(witness,'Opération unique liée',view.operation??'');
    pair(witness,'Racines physiquement liées',current.roots_bound?t('oui'):t('non'));
    if(current.trust_revision===null||current.trust_revision===undefined)pair(witness,'Journal de confiance absent',t('non'));
    else pair(witness,'Révision de confiance actuelle',current.trust_revision);
    pair(witness,'Révision de confiance courante',current.trust_revision_current?t('oui'):t('non'));
    pair(witness,'Empreinte d’action correspondante',canon.digest_matches?t('oui'):t('non'));
    $('approval-detail').hidden=false;
    if(!current.trust_revision_current){status('approval-status','error',t('Approbation périmée : la révision de confiance observée diffère de la révision déclarée. Aucune confirmation n’est sollicitée sur un état périmé ; reprendre la préparation avant toute suite.'));$('approval-send').disabled=true;$('approval-confirm').checked=false;return false}
    if(!current.roots_bound){status('approval-status','error',t('Prérequis non réuni : les témoins physiques des racines ne correspondent plus à la préparation. Aucune confirmation n’est sollicitée sur cet état ; reprendre la préparation avant toute suite.'));$('approval-send').disabled=true;$('approval-confirm').checked=false;return false}
    status('approval-status','success',t('Approbation présentée : opération unique, termes dérivés par le moteur et identité canonique de l’action Git préparée. La commande reste détenue côté service ; confirmer explicitement puis s’authentifier pour la consommer.'));
    return true;
  }
  /** Explicit absence is a visible state with its reason, never an empty
   * success: preparation stays the two-phase provisioning path. */
  function unavailable(view){
    $('approval-detail').hidden=true;$('approval-send').disabled=true;$('approval-confirm').checked=false;
    status('approval-status','empty',t('Approbation explicitement indisponible : {code}. Aucune commande d’approbation n’est liée au démarrage du service ; la préparation deux phases (CLI ou enrôlement) reste la voie réelle.',{code:view.reason??''}));
  }
  /** One explicit consumption: EMPTY body (the command stays server-side),
   * enrollment password in the Authorization header once, erased in `finally`,
   * no retry, no bearer fallback. A denied ack (HTTP 200 possible) is shown
   * as a refusal. */
  async function send(){
    const password=$('approval-password').value;
    if(!password)throw Error(t('Mot de passe d’enrôlement requis pour consommer la commande figée'));
    if(!$('approval-confirm').checked)throw Error(t('Confirmation explicite requise avant toute consommation'));
    let binary='';for(const byte of new TextEncoder().encode(password))binary+=String.fromCharCode(byte);
    const headers={'X-Swarm-CSRF':csrf,Authorization:'Enrollment '+btoa(binary).replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'')};
    const r=await fetch('/api/v1/paea/git-call/approval',{method:'POST',headers,cache:'no-store',credentials:'same-origin'});
    const raw=await r.text();let result;try{result=JSON.parse(raw)}catch{throw Error(t('Réponse illisible ; vérifier l’état avant reprise'))}
    if(!r.ok)throw Error(result?.failure?.code||String(r.status));
    if(result.state==='denied')throw Error(t('Intention refusée — décision journalisée')+' · '+(result.reason??''));
    if(result.state!=='approved'&&result.state!=='admitted')throw Error(t('Réponse inattendue : {state} — vérifier l’état avant reprise',{state:String(result.state??'')}));
    return result;
  }
  /** Guarded gesture: disable every control during one request, surface the
   * exact outcome, erase the password whatever happened. */
  async function guarded(action){if(busy)return;busy=true;$('approval-send').disabled=true;$('approval-confirm').disabled=true;$('approval-password').disabled=true;try{await action()}catch(error){status('approval-status','error',t('Refus : {code}',{code:error.message}))}finally{$('approval-password').value='';busy=false;$('approval-confirm').disabled=false;$('approval-password').disabled=false;sync()}}
  /** The send button enables only on explicit checkbox confirmation. */
  function sync(){$('approval-send').disabled=busy||!$('approval-confirm').checked||$('approval-detail').hidden}
  $('approval-confirm').addEventListener('change',sync);
  $('approval-send').addEventListener('click',()=>guarded(async()=>{
    const view=presented;if(!view)throw Error(t('Présentation indisponible ; recharger la page avant reprise'));
    const result=await send();
    // The ack is the engine's typed status (trust-status-ack-v1): shown once,
    // then the explicit gesture resets so any replay stays a fresh decision.
    $('approval-confirm').checked=false;
    status('approval-status','success',t('Approbation enregistrée par le moteur : décision reçue ({state}). Aucun effet Git n’a été exécuté par ce formulaire ; relire les états d’effets pour la suite.',{state:String(result.state)}));
  }));
  let presented=null;
  /** Escape clears the password field and returns focus to the confirmation
   * checkbox without submitting anything. */
  document.addEventListener('keydown',event=>{if(event.key==='Escape'&&!$('approval-detail').hidden){$('approval-password').value='';$('approval-confirm').focus()}});
  function navigation(){for(const link of document.querySelectorAll('[data-paea-link]')){const url=new URL(link.href);url.searchParams.set('lang',globalThis.SwarmI18n.language);url.searchParams.set('theme',document.documentElement.dataset.theme);link.href=url.pathname+url.search}}
  async function load(){
    status('approval-status','loading','Chargement…');$('approval-detail').hidden=true;$('approval-send').disabled=true;$('approval-confirm').checked=false;
    try{
      const session=await read('/api/v1/native-session');
      if(session.schema_version!==1||!/^[a-f0-9]{64}$/.test(session.csrf_token??''))throw Error('web-native-session-invalid');
      csrf=session.csrf_token;
    }catch(error){status('approval-status','error',t('Refus : {code}',{code:error.message}));return}
    try{
      const statusView=await read('/api/v1/paea/git-call/approval');
      if(statusView.bound!==true){unavailable({reason:'paea-git-call-approval-unavailable'});return}
      const view=await read('/api/v1/paea/git-call/approval/presentation');
      if(view.available!==true){unavailable(view);return}
      presented=view;
      presentation(view);
      sync();
    }catch(error){status('approval-status','error',t('Refus : {code}',{code:error.message}))}
  }
  const theme=new URL(location.href).searchParams.get('theme');if(['etat','sombre'].includes(theme))document.documentElement.dataset.theme=theme;
  // No #paea-theme listener here: the host page (paea-effects.html) also loads
  // paea-effects.js, whose own listener owns the toggle. Two listeners would
  // cancel each other on every click; this script only carries the language
  // and theme params of the approval form's navigation links.
  navigation();load();
})();
