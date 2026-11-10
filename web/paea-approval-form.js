'use strict';
/** Explicit approval form for the single frozen PAEA approval command held by
 * the native service. Consumes only the three guarded approval endpoints:
 * GET /api/v1/paea/git-call/approval (minimal startup observation: the `bound`
 * boolean only — the capability stays Null under the cookie-only session,
 * BLP-RET-05), GET /api/v1/paea/git-call/approval/presentation (the authorized
 * projection: operation, request digest, canonical identity, engine-derived
 * agreement terms and current witness state) and POST
 * /api/v1/paea/git-call/approval (single-intent consumption with an EMPTY
 * body). The browser never invents JSON, never selects a root, action or term
 * and never transports command bytes: the command stays server-side (bound
 * approval) and the confirmation is connected to the digest presented above.
 * The guarded presentation read also carries the READER credential
 * (Authorization: Bearer) taken from the ephemeral input at request time: the
 * session cookie alone is never authorization for a classified projection,
 * the credential is erased on EVERY exit of the load — success, HTTP or
 * network error, unbound observation, refused presentation, malformed reply —
 * never stored, logged or transported anywhere else, and reader refusals
 * (paea-reader-*) keep their exact code visible with the re-read control
 * available. The reader
 * input carries NO maximum-length cutoff: the producer-side credential check
 * (`credential_valid`, trust_types.rs) enforces a 16-byte minimum, forbidden
 * whitespace and reserved prefixes with no maximum, so any frontend cap would
 * reject valid tokens the engine accepts. The too-short/whitespace shapes are
 * refused server-side with the exact `paea-reader-credential`/typed verdict,
 * never pre-cut here. Consumption sends only the explicit checkbox
 * confirmation and the approver's enrollment password (sent once in the
 * Authorization header, erased immediately after use, never stored; distinct
 * from the reader credential — enrollment authority is never reinterpreted
 * as classification clearance). The confirmation checkbox and the send
 * button stay usable only while the last rendered presentation is fully
 * admissible (current Trust revision AND bound physical roots): loading,
 * unavailable, stale, wrong-roots and read-error states clear that common
 * admissibility state and DISABLE the checkbox, so no real keyboard
 * gesture, change event or asynchronous completion (the guarded exit) can
 * solicit a confirmation on them; the existing re-read control restores
 * usability after a genuinely admissible read. A business refusal ack
 * (`state:"denied"`, HTTP 200 included) is never shown as a success; any
 * refusal keeps its exact code visible. The consumption success is
 * fail-closed: only an exact `state:"approved"` decision renders success —
 * the ordinary full approved object and the redacted `trust-status-ack-v1`
 * acknowledgement both carry that state — while `admitted`, `unknown` and any
 * missing, null or empty state never do. No Git effect, no second approval
 * system, no consent captured by display alone.
 */
(() => {
  const $=id=>document.getElementById(id),t=(s,v)=>globalThis.SwarmI18n.t(s,v);
  // `confirmable` is the common UI admissibility state: true only while the
  // last rendered presentation is fully admissible (current Trust revision
  // AND bound roots). Every state transition writes it (load start, success
  // presentation, unavailable, stale, wrong-roots, read error); sync(), the
  // checkbox's disabled state and the guarded exits only read it. It carries
  // no policy and no server field: it prevents the UI from soliciting a
  // confirmation the presentation does not admit.
  let csrf=null,busy=false,confirmable=false;
  /** Render a translated visible state, never an inferred success. */
  function status(id,kind,message){$(id).dataset.state=kind;$(id).textContent=t(message)}
  /** Read one guarded GET endpoint; refuse to hide a refusal code. The
   * optional bearer credential authenticates the READER frontier (the
   * session cookie alone is never authorization for a classified
   * projection); it travels only inside this request's headers, is read
   * from the ephemeral input at call time and is never persisted anywhere. */
  async function read(path,readerToken){const headers={};if(readerToken)headers.Authorization='Bearer '+readerToken;const r=await fetch(path,{method:'GET',cache:'no-store',credentials:'same-origin',headers});const raw=await r.text();let body;try{body=JSON.parse(raw)}catch{throw Error(t('Réponse illisible ; vérifier l’état avant reprise'))}if(!r.ok)throw Error(body?.failure?.code||body?.error||String(r.status));return body}
  /** Add one visible dt/dd pair; values are shown verbatim, never reinterpreted. */
  function pair(dl,key,value){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=t(key);dd.textContent=String(value);dl.append(dt,dd)}
  /** Render the authorized projection verbatim: the public operation beside
   * the engine-derived agreement terms (binding, resource, amount,
   * beneficiary, revision). Term rows render only present, non-empty values:
   * the recipe terms carry deliberately empty action_id/content_digest (the
   * command identity stays the request digest above), and a labeled empty
   * row would present a private-field shape the projection does not serve.
   * The private command fields (action_id, mandate_id, content_digest) are
   * never labeled here; the request digest stands as the command identity.
   * Returns true only for a fully admissible presentation (current revision
   * AND bound roots): the stale and wrong-roots branches clear the common
   * admissibility state (`confirmable`) and disable the confirmation
   * checkbox before returning false, so no later change event or key
   * gesture can solicit a confirmation on the refusal they rendered. */
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
    if(!current.trust_revision_current){status('approval-status','error',t('Approbation périmée : la révision de confiance observée diffère de la révision déclarée. Aucune confirmation n’est sollicitée sur un état périmé ; reprendre la préparation avant toute suite.'));$('approval-send').disabled=true;$('approval-confirm').checked=false;$('approval-confirm').disabled=true;confirmable=false;return false}
    if(!current.roots_bound){status('approval-status','error',t('Prérequis non réuni : les témoins physiques des racines ne correspondent plus à la préparation. Aucune confirmation n’est sollicitée sur cet état ; reprendre la préparation avant toute suite.'));$('approval-send').disabled=true;$('approval-confirm').checked=false;$('approval-confirm').disabled=true;confirmable=false;return false}
    status('approval-status','success',t('Approbation présentée : opération unique, termes dérivés par le moteur et identité canonique de l’action Git préparée. La commande reste détenue côté service ; confirmer explicitement puis s’authentifier pour la consommer.'));
    // Fully admissible presentation: the confirmation becomes usable again
    // (the recovery of a previously non-confirmable state runs through the
    // existing re-read control reaching this branch).
    confirmable=true;$('approval-confirm').disabled=false;
    return true;
  }
  /** Explicit absence is a visible state with its reason, never an empty
   * success. Reader-frontier refusals (paea-reader-*) mean the approval IS
   * bound but this session/credential holds no current classification
   * authorization: say exactly that (the ephemeral reader input stays
   * available for a corrected credential), never the "nothing bound" text.
   * The stale-policy refusal keeps the exact stale wording so a stale state
   * is never presented as confirmable — no confirmation is solicited.
   * Unavailability clears the common admissibility state and disables the
   * confirmation checkbox: nothing rendered here is confirmable, including
   * by keyboard. */
  function unavailable(view){
    $('approval-detail').hidden=true;$('approval-send').disabled=true;$('approval-confirm').checked=false;$('approval-confirm').disabled=true;confirmable=false;
    const reason=String(view.reason??''),readerRefused=reason.startsWith('paea-reader-');
    status('approval-status','empty',
      readerRefused
        ? (reason==='paea-reader-policy-stale'
          ? t('Approbation périmée : la révision de confiance observée diffère de la révision déclarée. Aucune confirmation n’est sollicitée sur un état périmé ; reprendre la préparation avant toute suite.')
          : t('Lecture refusée : {code}. L’action préparée est liée, mais son contenu classifié n’est présenté qu’à un lecteur autorisé par une attribution courante de confidentialité ; aucune donnée de l’action n’est affichée.',{code:reason}))
        : t('Approbation explicitement indisponible : {code}. Aucune commande d’approbation n’est liée au démarrage du service ; la préparation deux phases (CLI ou enrôlement) reste la voie réelle.',{code:view.reason??''}));
  }
  /** One explicit consumption: EMPTY body (the command stays server-side),
   * enrollment password in the Authorization header once, erased in `finally`,
   * no retry, no bearer fallback. Fail-closed success: this returns ONLY on an
   * exact `state:"approved"` decision — the ordinary full approved object and
   * the redacted `trust-status-ack-v1` acknowledgement (`content_redacted:true`)
   * both carry that state and both are accepted, so no redacted-only shape is
   * imposed. A denied ack (HTTP 200 possible) stays a typed refusal with its
   * reason; `admitted`, `pending_approval`, `committed`, `unknown`,
   * `reconciled` and any missing, null or empty state refuse as
   * "Réponse inattendue" and NEVER return as a success. */
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
    if(result.state!=='approved')throw Error(t('Réponse inattendue : {state} — vérifier l’état avant reprise',{state:String(result.state??'')}));
    return result;
  }
  /** Guarded gesture: disable every control during one request, surface the
   * exact outcome, erase the password whatever happened. `confirmable` is
   * written only by the state transitions (load start, unavailable, stale
   * and wrong-roots branches, admissible success), never by this exit: the
   * finally restores the checkbox per that state, so an asynchronous
   * completion can never make a non-confirmable presentation confirmable
   * (a reload resolved on stale/wrong-roots/unavailable keeps the checkbox
   * disabled) while a consumption refusal on an admissible presentation
   * keeps it usable for the existing recovery (corrected password). The
   * password field is always restored: erasing its value and freeing it can
   * never solicit a confirmation. */
  async function guarded(action){if(busy)return;busy=true;$('approval-send').disabled=true;$('approval-confirm').disabled=true;$('approval-password').disabled=true;try{await action()}catch(error){status('approval-status','error',t('Refus : {code}',{code:error.message}))}finally{$('approval-password').value='';busy=false;$('approval-confirm').disabled=!confirmable;$('approval-password').disabled=false;sync()}}
  /** The send button enables only on an admissible presentation AND the
   * explicit checkbox confirmation: the common admissibility state
   * (`confirmable`) gates it exactly like busy and hidden do, so a change
   * event or key gesture can never enable sending on a loading, unavailable,
   * stale, wrong-roots or errored presentation. */
  function sync(){$('approval-send').disabled=busy||!confirmable||!$('approval-confirm').checked||$('approval-detail').hidden}
  $('approval-confirm').addEventListener('change',sync);
  $('approval-send').addEventListener('click',()=>guarded(async()=>{
    const view=presented;if(!view)throw Error(t('Présentation indisponible ; recharger la page avant reprise'));
    const result=await send();
    // The ack reached send()'s exact `state:"approved"` gate: the full
    // approved object or the redacted trust-status-ack-v1 both qualify by
    // that state; shown once, then the explicit gesture resets so any replay
    // stays a fresh decision.
    $('approval-confirm').checked=false;
    status('approval-status','success',t('Approbation enregistrée par le moteur : décision reçue ({state}). Aucun effet Git n’a été exécuté par ce formulaire ; relire les états d’effets pour la suite.',{state:String(result.state)}));
  }));
  /** Explicit reader re-read: a refused paea-reader-* state stays visible and
   * the reader may correct the ephemeral credential and re-observe through
   * the same guarded endpoint — no page reload, no stored state, the same
   * cleared-then-refetched presentation path as `load()`. */
  $('approval-reload').addEventListener('click',()=>guarded(async()=>{presented=null;await load()}));
  let presented=null;
  /** Escape clears the password field and returns focus to the confirmation
   * checkbox without submitting anything. */
  document.addEventListener('keydown',event=>{if(event.key==='Escape'&&!$('approval-detail').hidden){$('approval-password').value='';$('approval-confirm').focus()}});
  function navigation(){for(const link of document.querySelectorAll('[data-paea-link]')){const url=new URL(link.href);url.searchParams.set('lang',globalThis.SwarmI18n.language);url.searchParams.set('theme',document.documentElement.dataset.theme);link.href=url.pathname+url.search}}
  async function load(){
    // Load start clears admissibility: nothing is confirmable until a fully
    // admissible presentation is rendered (success branch below).
    status('approval-status','loading','Chargement…');$('approval-detail').hidden=true;$('approval-send').disabled=true;$('approval-confirm').checked=false;$('approval-confirm').disabled=true;confirmable=false;
    // Erasure covers EVERY exit of this load, success or failure — any earlier
    // return leaves the typed credential in the DOM (counterexample of
    // 2026-11-01: session error, status error and unbound each kept it). The
    // clear runs in a finally-equivalent position before the outcome is
    // rendered; the token itself is never stored, logged or placed in a URL.
    const readerField=$('approval-reader-credential');
    let readerToken='';
    const clearReader=()=>{readerToken='';readerField.value=''};
    try{
      try{
        const session=await read('/api/v1/native-session');
        if(session.schema_version!==1||!/^[a-f0-9]{64}$/.test(session.csrf_token??''))throw Error('web-native-session-invalid');
        csrf=session.csrf_token;
      }catch(error){status('approval-status','error',t('Refus : {code}',{code:error.message}));return}
      try{
        const statusView=await read('/api/v1/paea/git-call/approval');
        if(statusView.bound!==true){unavailable({reason:'paea-git-call-approval-unavailable'});return}
        // Minimal status contract (BLP-RET-05): a bound approval reports a
        // NULL capability — the served capability stays behind the guarded
        // presentation read below. A non-null capability is an out-of-contract
        // projection and must stop the load visibly, never be rendered.
        if(statusView.capability!=null)throw Error('paea-status-out-of-contract');
        // The reader credential is read from the ephemeral input at request
        // time only: no storage, URL or log, no closure past this load.
        readerToken=readerField.value;
        const view=await read('/api/v1/paea/git-call/approval/presentation',readerToken);
        if(view.available!==true){unavailable(view);return}
        presented=view;
        presentation(view);
        sync();
      }catch(error){status('approval-status','error',t('Refus : {code}',{code:error.message}))}
    }finally{
      clearReader();
    }
  }
  const theme=new URL(location.href).searchParams.get('theme');if(['etat','sombre'].includes(theme))document.documentElement.dataset.theme=theme;
  // No #paea-theme listener here: the host page (paea-effects.html) also loads
  // paea-effects.js, whose own listener owns the toggle. Two listeners would
  // cancel each other on every click; this script only carries the language
  // and theme params of the approval form's navigation links.
  navigation();load();
})();
