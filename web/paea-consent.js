'use strict';
/** Real approver consent for the engine-held prepared PAEA action. Reads the
 * public guarded observation first (GET /api/v1/paea/git-call/approval for the
 * minimal startup bound boolean — the capability stays Null, nothing derived
 * from the frozen command passes under the cookie-only session, BLP-RET-05),
 * then the AUTHORIZED projection behind GET
 * /api/v1/paea/git-call/approval/presentation (exact request digest and
 * operation) and GET /api/v1/paea/git-call/preview (exact prepared action).
 * Both guarded reads carry the READER credential (Authorization: Bearer) read
 * from the ephemeral input at request time: the session cookie alone is never
 * authorization for a classified projection, the credential is erased on EVERY
 * exit of the load — success, refusal, HTTP or network error — never stored,
 * logged or placed in a URL, no frontend length cap is applied, and reader
 * refusals (paea-reader-*) keep their exact code visible with the re-read
 * control available. The consent form then submits ONE explicit operator
 * action: POST /api/v1/paea/git-call/approval with the operator's enrolled
 * password (Authorization: Enrollment …) and an EMPTY body — the frozen
 * command is held server-side and no request byte ever reaches the engine
 * chain; enrollment authority is never reinterpreted as classification
 * clearance, so the form stays reachable while the reader reads are refused.
 * The human compares the displayed request_digest out of band; the page never
 * re-encodes or substitutes the held command. Refusals (role/separation,
 * stale revision, credentials, any non-empty body) and every explicit state
 * are shown verbatim; no success is ever fabricated and no automatic retry
 * exists. The success wording is fail-closed: only an exact
 * `state:"approved"` decision renders success — the ordinary full approved
 * object and the redacted `trust-status-ack-v1` acknowledgement both carry
 * that state — while `admitted`, `unknown` and any missing, null or empty
 * state never do. No Git call is triggered from here; this page only routes
 * the real Trust approve on the project root.
 */
(() => {
  const $=id=>document.getElementById(id),t=(s,v)=>globalThis.SwarmI18n.t(s,v);
  /** Render a translated visible state, never an inferred success. */
  function status(id,kind,message){$(id).dataset.state=kind;$(id).textContent=t(message)}
  /** Purge every exact-value projection of a previous observation. Each load
   * renders zero or more of these containers; any exit that skips a render
   * (throwing guarded read, refused prerequisite, HTTP/network/JSON error)
   * would otherwise leave the previous success's exact digests displayed
   * under a refusal or error wording (counterexample of 2026-11-01,
   * glm-consent-refusal-cache). Covers the guarded-read projections
   * (#approval-capability, #approval-preview, #preview-status) AND the
   * approval POST result containers (#result-detail, #result-raw,
   * #result-raw-box): a stale submission record under a fresh refusal
   * wording would claim an action state the current observation never
   * produced. Purging at the START of every load makes the no-data wording
   * of any refusal literally true; the per-render replaceChildren in
   * bound/presentation/preview stays as defense in depth for every path
   * that renders a container. Errors: none; side effects: DOM only,
   * containers re-created on the next successful render. */
  function purgeProjections(){
    $('approval-capability').replaceChildren();
    $('approval-preview').replaceChildren();
    $('result-detail').replaceChildren();
    $('result-raw').textContent='';
    $('result-raw-box').hidden=true;
    $('preview-status').dataset.state='empty';
    $('preview-status').textContent=t('Aperçu non chargé.');
  }
  /** Read one guarded GET endpoint; refusals stay typed, never swallowed. The
   * optional bearer credential authenticates the READER frontier (the session
   * cookie alone is never authorization for a classified projection); it
   * travels only inside this request's headers, is read from the ephemeral
   * input at call time and is never persisted anywhere. */
  async function read(path,readerToken){const headers={};if(readerToken)headers.Authorization='Bearer '+readerToken;const r=await fetch(path,{method:'GET',cache:'no-store',credentials:'same-origin',headers});const raw=await r.text();let body;try{body=JSON.parse(raw)}catch{throw Error(t('Réponse illisible ; vérifier l’état avant reprise'))}if(!r.ok)throw Error(body?.failure?.code||body?.error||String(r.status));return body}
  /** Verbatim dt/dd pair; values are shown as held, never reinterpreted. */
  function pair(dl,key,value){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=t(key);dd.textContent=String(value);dl.append(dt,dd)}
  /** Startup observation of the approval route: the minimal `bound` boolean
   * only (`capability` stays Null — no request-derived operation or digest,
   * BLP-RET-05). The exact frozen-command metadata is presented solely
   * through the guarded presentation read below. */
  function bound(view){
    // Purge first: a previous success's exact digests must never survive a
    // new observation's verdict, even when this render itself stays minimal
    // (counterexample of 2026-11-01).
    $('approval-capability').replaceChildren();
    if(!view.bound){status('approval-status','empty','Aucune commande d’approbation n’est liée au démarrage du service. La préparation deux phases (octets figés + permission provisionnée) reste la voie réelle ; cette page ne prépare rien.');return false}
    if(view.capability!=null){status('approval-status','error','Rapport hors contrat : la route de statut a servi une opération dérivée de la commande figée, ce que la frontière lecteur interdit. Aucun envoi possible.');return false}
    $('approval-detail').hidden=false;
    return true;
  }
  /** Guarded presentation of the exact frozen-command metadata: the reader
   * credential is consumed at request time, the digest and operation are
   * rendered verbatim, and a paea-reader-* refusal keeps its exact code
   * visible — the consent form stays reachable (enrollment authority is
   * never reinterpreted as classification clearance), the operator decides
   * out of band whether to proceed without the displayed digest. Returns
   * true only when the exact metadata is displayed. */
  function presentation(view){
    const dl=$('approval-capability');dl.replaceChildren();
    pair(dl,'Commande d’approbation liée','oui');
    if(!view.available){
      const reason=String(view.reason??'');
      if(reason.startsWith('paea-reader-'))status('approval-status','empty',t('Lecture refusée : {code}. L’action préparée est liée, mais son contenu classifié n’est présenté qu’à un lecteur autorisé par une attribution courante de confidentialité ; aucune donnée de l’action n’est affichée.',{code:reason}));
      else status('approval-status','empty',t('Refus : {code}',{code:reason}));
      pair(dl,'Métadonnées exactes de la commande','non exposées sous cette session ; lecture gardée de la présentation requise');
      return false;
    }
    pair(dl,'Empreinte de la commande figée',view.request_digest??'');
    pair(dl,'Opération unique liée',view.operation??'');
    return true;
  }
  /** Present the prepared action exactly as the preview route holds it;
   * returns the observed prerequisite fields, or null when unavailable. */
  function preview(view){
    const dl=$('approval-preview');dl.replaceChildren();
    if(!view.available){pair(dl,'Aperçu indisponible',view.reason??'');return null}
    const canon=view.canonical??{},current=view.current??{};
    pair(dl,'Empreinte d’action',canon.action_digest??'');
    pair(dl,'Intention métier',canon.intent_id??'');
    pair(dl,'Correspondance de l’empreinte d’action',canon.digest_matches?t('oui'):t('non'));
    pair(dl,'Racines physiquement liées',current.roots_bound?t('oui'):t('non'));
    pair(dl,'Révision de confiance actuelle',current.trust_revision??'');
    pair(dl,'Révision de confiance courante',current.trust_revision_current?t('oui'):t('non'));
    const action=view.proposal?.action??{};
    const labels={payment:{invoice_id:'Facture',amount_minor:'Montant en unités mineures',currency:'Devise',beneficiary_account_id:'Compte bénéficiaire'},git:{repository_id:'Identifiant du service attendu',target_ref:'Référence Git cible',expected_old_oid:'OID précédent attendu',candidate_oid:'OID candidat'}};
    for(const [key,value] of Object.entries(action)){
      if(key==='kind')continue;
      // Exact held value rendered verbatim as the visible term; the translated
      // field label only names the term. Never substitute or normalize value.
      pair(dl,labels[action.kind]?.[key]??key,String(value));
    }
    const terms=view.agreement_terms;
    if(terms&&typeof terms==='object'){
      pair(dl,'Accord attendu — ressource',terms.resource??'');
      pair(dl,'Accord attendu — montant',terms.amount_minor??'');
      pair(dl,'Accord attendu — devise',terms.currency??'');
      pair(dl,'Accord attendu — bénéficiaire',terms.beneficiary??'');
      pair(dl,'Accord attendu — révision',terms.revision??'');
    }
    pair(dl,'Consentement capturé',view.limits?.consent??'');
    return {canon,current};
  }
  /** Load all three guarded reads; prerequisites gate the form, never
   * silently. Erasure of the reader credential covers EVERY exit of this
   * load, success or failure — any earlier return would leave the typed
   * credential in the DOM. The token is read from the ephemeral input at
   * request time only: no storage, URL or log, no closure past this load.
   * Contract: every load starts by purging all prior projections (see
   * purgeProjections) and leaves a TERMINAL state on #approval-status
   * ('success'/'empty'/'error', never 'loading') whatever the exit — the
   * form stays reachable except when a preview prerequisite explicitly
   * hides it. #approval-detail stays visible once the source is bound
   * (the reader input + re-read control live there). */
  async function load(){
    status('approval-status','loading','Chargement…');
    // Purge EVERY previous observation's exact projections before this load
    // can fail anywhere: after this point no throw, refusal or error can
    // leave a prior success's digests displayed under a refusal/error wording
    // (counterexample of 2026-11-01: an authorized read followed by an HTTP
    // 403 kept both old digests in the DOM). The purge covers the approval
    // POST result containers too (#result-detail/#result-raw): a stale
    // submission record shown under a fresh refusal wording would claim an
    // action state the current observation never produced. #approval-detail
    // is NOT re-hidden: it carries the reader input and the re-read control,
    // and an operator must always be able to retype a credential and
    // re-observe after a refused status read — hiding it on a pre-bound exit
    // used to leave the refusal terminal with no owned way out except a full
    // page reload.
    purgeProjections();
    $('consent-form').hidden=true;
    const readerField=$('approval-reader-credential');
    let readerToken='';
    const clearReader=()=>{readerToken='';readerField.value=''};
    try{
      let boundOk=false;
      try{
        boundOk=bound(await read('/api/v1/paea/git-call/approval'));
        if(boundOk)$('approval-detail').hidden=false;
      }catch(error){status('approval-status','error',t('Refus : {code}',{code:error.message}))}
      if(!boundOk)return;
      // Success is asserted ONLY on displayed exact metadata (counterexample
      // of 2026-11-01, run4 red: a preview success raced ahead of the
      // presentation render). The presentation pair carries the bound digest;
      // presenting it is a precondition of any success wording below.
      // The reader credential is consumed at request time only; it authorizes
      // the classified presentation and the exact prepared-action preview —
      // the session cookie alone never does (BLP-RET-05). `presented` holds
      // the exact displayed metadata; it gates the success wording below
      // (counterexample of 2026-11-01, run4 red: success raced ahead of the
      // presentation render).
      readerToken=readerField.value;
      let presented=false;
      try{presented=presentation(await read('/api/v1/paea/git-call/approval/presentation',readerToken))}
      catch(error){
        // A presentation refusal keeps its typed code; the consent form stays
        // reachable — enrollment authority is never read clearance. A thrown
        // refusal (HTTP 403, network, malformed JSON) is the same reader
        // frontier as an `available:false` verdict: render the same held-
        // command pair and the exact code, terminal 'empty' — never a bare
        // 'error' whose wording could claim a success projection above it.
        const dl=$('approval-capability');
        if(error.message.startsWith('paea-reader-')){
          pair(dl,'Métadonnées exactes de la commande','non exposées sous cette session ; lecture gardée de la présentation requise');
          status('approval-status','empty',t('Lecture refusée : {code}. L’action préparée est liée, mais son contenu classifié n’est présenté qu’à un lecteur autorisé par une attribution courante de confidentialité ; aucune donnée de l’action n’est affichée.',{code:error.message}));
        }else{
          pair(dl,'Métadonnées exactes de la commande','non exposées sous cette session ; lecture gardée de la présentation requise');
          status('approval-status','error',t('Refus : {code}',{code:error.message}));
        }
      }
      // The form exists from the minimal bound observation; the prepared-action
      // prerequisites decide whether it stays reachable, each with its own
      // explicit refusal — never a silent gate. EVERY preview exit renders a
      // terminal typed state on approval-status (counterexample of
      // 2026-11-01, run4 red: an `available:false` preview refusal returned
      // null and left the status at 'loading' — a refusal must never read as
      // an in-flight load, on this page any more than on the approval form).
      $('consent-form').hidden=false;
      try{
        const prepared=preview(await read('/api/v1/paea/git-call/preview',readerToken));
        if(!prepared){
          // The preview refusal mirrors the presentation verdict (same reader
          // frontier): the held-command notice stays the visible contract and
          // no prerequisite is invented. The typed refusal is already on
          // approval-preview; approval-status gets the explicit non-success
          // terminal state ('empty' — nothing further is asserted here).
          // Catalog-only strings (locales/en.json is a declared input of the
          // i18n recipes; no new keys are introduced).
          status('approval-status','empty',t('Aperçu indisponible : ')+String($('approval-preview').textContent));
        }else if(!prepared.current.trust_revision_current){
          $('consent-form').hidden=true;
          status('approval-status','error','Préparation périmée : la révision de confiance actuelle diffère de la révision déclarée. Aucun consentement n’est sollicité sur un état périmé ; reprendre la préparation avant toute suite.');
        }else if(!prepared.current.roots_bound){
          $('consent-form').hidden=true;
          status('approval-status','error','Prérequis non réuni : les témoins physiques des deux racines ne correspondent plus à la préparation. Aucun consentement n’est sollicité sur cet état ; reprendre la préparation avant toute suite.');
        }else if(!prepared.canon.digest_matches){
          $('consent-form').hidden=true;
          status('approval-status','error','Empreinte d’action recalculée différente de l’empreinte liée : état signalé tel quel, aucune normalisation ni consentement sollicité.');
        }else if(!presented){
          // The presentation verdict already rendered its typed refusal pair
          // and terminal state on approval-capability/approval-status; the
          // preview succeeded but success is gated on the exact metadata
          // being displayed (counterexample of 2026-11-01, run4 red). Keep
          // the refusal wording — never overwrite it with a different claim.
        }else{
          status('approval-status','success','Commande figée liée et action préparée présentées. Le formulaire envoie uniquement votre mot de passe enrôlé ; la commande reste détenue par le service, sans octet de requête.');
        }
      }catch(error){
        // The preview refusal is shown without inventing a prerequisite: the
        // guarded preview sits behind the reader frontier (BLP-RET-05). The
        // typed code lands on approval-preview AND the terminal state moves
        // to approval-status — never an in-flight 'loading' after a refusal.
        // Catalog-only strings.
        status('preview-status','error',t('Refus : {code}',{code:error.message}));
        status('approval-status','empty',t('Aperçu indisponible : ')+error.message);
      }
    }finally{
      clearReader();
    }
  }
  /** Submit the one explicit operator action: empty body, password only.
   * The held command stays server-side; nothing request-supplied reaches it. */
  async function send(password){
    // Enrollment credential, exactly the trust-native pattern: UTF-8 password
    // bytes, base64url without padding. No password copy is kept after send.
    let binary='';for(const byte of new TextEncoder().encode(password))binary+=String.fromCharCode(byte);
    const r=await fetch('/api/v1/paea/git-call/approval',{
      method:'POST',cache:'no-store',credentials:'same-origin',
      headers:{'X-Swarm-CSRF':globalThis.SwarmConsent.csrf,'Authorization':'Enrollment '+btoa(binary).replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'')},
      body:new Uint8Array(0),
    });
    const text=await r.text();let value;try{value=JSON.parse(text)}catch{throw Error(t('Réponse illisible ; vérifier l’état avant reprise')+' · '+r.status)}
    if(!r.ok&&value.trust_state_may_be_recorded===true)throw Error(t('Effet possiblement enregistré — relire l’état avant toute reprise')+' · '+String(value.trust_publication||''));
    if(!r.ok)throw Error(value.failure?.code||value.error||String(r.status));
    return value;
  }
  /** Show the mediated reply verbatim; fail-closed success: only an exact
   * `state:"approved"` renders the success wording — the ordinary full
   * approved object and the redacted `trust-status-ack-v1` acknowledgement
   * (`content_redacted:true`) both carry that state and both are accepted,
   * so no redacted-only shape is imposed. A denied state stays a visible
   * refusal with its reason; `admitted`, `pending_approval`, `committed`,
   * `unknown`, `reconciled` and any missing, null or empty state render the
   * "Réponse inattendue" refusal with the state verbatim — NEVER a success. */
  function result(value){
    const dl=$('result-detail');dl.replaceChildren();
    for(const [key,label] of [['kind','Type d’accusé'],['object_id','Identifiant de l’objet'],['state','État rapporté'],['reason','Raison'],['next_action','Suite indiquée'],['business_mutated','Effet métier réalisé']])
      if(value[key]!==undefined)pair(dl,label,value[key]);
    $('result-raw-box').hidden=false;$('result-raw').textContent=JSON.stringify(value,null,2);
    const state=$('result-status');
    if(value.state==='denied'){
      state.dataset.state='error';
      state.textContent=t('Refus enregistré par le moteur : la commande n’a pas produit d’accord. La raison et la suite indiquée s’affichent telles quelles ; relire l’état avant toute reprise.');
    }else if(value.state==='approved'){
      state.dataset.state='success';
      state.textContent=t('Accusé du canal d’approbation : état rapporté ci-dessous, attesté par le moteur sur le Root projet. Le contenu de l’action reste rédigé (content_redacted) ; l’accord exact est ensuite observable côté moteur.');
    }else{
      state.dataset.state='error';
      state.textContent=t('Réponse inattendue : {state} — vérifier l’état avant reprise',{state:String(value.state??'')});
    }
  }
  function navigation(){for(const link of document.querySelectorAll('[data-paea-link]')){const url=new URL(link.href);url.searchParams.set('lang',globalThis.SwarmI18n.language);url.searchParams.set('theme',document.documentElement.dataset.theme);link.href=url.pathname+url.search}}
  const theme=new URL(location.href).searchParams.get('theme');if(['etat','sombre'].includes(theme))document.documentElement.dataset.theme=theme;
  $('paea-theme').addEventListener('click',()=>{document.documentElement.dataset.theme=document.documentElement.dataset.theme==='etat'?'sombre':'etat';navigation()});
  $('consent-form').addEventListener('submit',async event=>{
    event.preventDefault();
    const password=$('consent-password').value;
    if(!globalThis.SwarmConsent){const state=$('result-status');state.dataset.state='error';state.textContent=t('Refus : {code}',{code:'csrf-unavailable'});return}
    const button=$('consent-send');button.disabled=true;
    const state=$('result-status');state.dataset.state='loading';state.textContent=t('Exécution de la commande figée liée sous votre autorité enrôlée…');
    try{result(await send(password))}
    catch(error){state.dataset.state='error';state.textContent=t('Refus : {code}',{code:error.message})}
    finally{button.disabled=false;$('consent-password').value='';navigation()}
  });
  /** CSRF token: dedicated session token from the ordinary native-session read. */
  (async()=>{try{const s=await read('/api/v1/native-session');if(typeof s?.csrf_token!=='string')throw Error('web-native-session-invalid');globalThis.SwarmConsent={csrf:s.csrf_token};load()}catch(error){status('approval-status','error',t('Refus : {code}',{code:error.message}))}})();
  /** Explicit reader re-read: a refused paea-reader-* state stays visible and
   * the reader may correct the ephemeral credential and re-observe through
   * the same guarded endpoints — no page reload, no stored state. */
  $('approval-reload').addEventListener('click',()=>{load()});
  navigation();
})();
