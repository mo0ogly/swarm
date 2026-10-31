'use strict';
/** Real approver consent for the engine-held prepared PAEA action. Reads the
 * two public guarded observations (GET /api/v1/paea/git-call/approval for the
 * frozen bound command capability and GET /api/v1/paea/git-call/preview for
 * the exact prepared action presented verbatim), then submits ONE explicit
 * operator action: POST /api/v1/paea/git-call/approval with the operator's
 * enrolled password (Authorization: Enrollment …) and an EMPTY body — the
 * frozen command is held server-side and no request byte ever reaches the
 * engine chain. The human compares the displayed request_digest out of band;
 * the page never re-encodes or substitutes the held command. Refusals
 * (capability, role/separation, stale revision, credentials, any non-empty
 * body) and every explicit state are shown verbatim; no success is ever
 * fabricated and no automatic retry exists. No Git call is triggered from
 * here; this page only routes the real Trust approve on the project root.
 */
(() => {
  const $=id=>document.getElementById(id),t=(s,v)=>globalThis.SwarmI18n.t(s,v);
  /** Render a translated visible state, never an inferred success. */
  function status(id,kind,message){$(id).dataset.state=kind;$(id).textContent=t(message)}
  /** Read one guarded GET endpoint; refusals stay typed, never swallowed. */
  async function read(path){const r=await fetch(path,{method:'GET',cache:'no-store',credentials:'same-origin'});const raw=await r.text();let body;try{body=JSON.parse(raw)}catch{throw Error(t('Réponse illisible ; vérifier l’état avant reprise'))}if(!r.ok)throw Error(body?.failure?.code||body?.error||String(r.status));return body}
  /** Verbatim dt/dd pair; values are shown as held, never reinterpreted. */
  function pair(dl,key,value){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=t(key);dd.textContent=String(value);dl.append(dt,dd)}
  /** Labels of the bound-approval capability, translated for naming only. */
  const CAPABILITY_LABELS={scope:'Portée',operation:'Opération',request_digest:'Empreinte de la commande figée'};
  /** Present the bound approval capability; false when nothing is bound. */
  function bound(view){
    if(!view.bound){status('approval-status','empty','Aucune commande d’approbation n’est liée au démarrage du service. La préparation deux phases (octets figés + permission provisionnée) reste la voie réelle ; cette page ne prépare rien.');return false}
    const cap=view.capability;
    if(!cap||typeof cap!=='object'){status('approval-status','error','Capacité d’approbation absente d’une source pourtant liée : état incohérent signalé, aucun envoi possible.');return false}
    const dl=$('approval-capability');dl.replaceChildren();
    for(const [key,value] of Object.entries(cap)){if(!(key in CAPABILITY_LABELS))continue;pair(dl,CAPABILITY_LABELS[key],value)}
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
  /** Load both guarded reads; prerequisites gate the form, never silently. */
  async function load(){
    status('approval-status','loading','Chargement…');
    $('approval-detail').hidden=true;$('consent-form').hidden=true;$('result-raw-box').hidden=true;
    let boundOk=false;
    try{
      boundOk=bound(await read('/api/v1/paea/git-call/approval'));
      if(boundOk)$('approval-detail').hidden=false;
    }catch(error){status('approval-status','error',t('Refus : {code}',{code:error.message}))}
    if(!boundOk)return;
    // The form exists from the bound capability; the prepared-action
    // prerequisites decide whether it stays reachable, each with its own
    // explicit refusal — never a silent gate.
    $('consent-form').hidden=false;
    try{
      const prepared=preview(await read('/api/v1/paea/git-call/preview'));
      if(prepared&&!prepared.current.trust_revision_current){
        $('consent-form').hidden=true;
        status('approval-status','error','Préparation périmée : la révision de confiance actuelle diffère de la révision déclarée. Aucun consentement n’est sollicité sur un état périmé ; reprendre la préparation avant toute suite.');
      }else if(prepared&&!prepared.current.roots_bound){
        $('consent-form').hidden=true;
        status('approval-status','error','Prérequis non réuni : les témoins physiques des deux racines ne correspondent plus à la préparation. Aucun consentement n’est sollicité sur cet état ; reprendre la préparation avant toute suite.');
      }else if(prepared&&!prepared.canon.digest_matches){
        $('consent-form').hidden=true;
        status('approval-status','error','Empreinte d’action recalculée différente de l’empreinte liée : état signalé tel quel, aucune normalisation ni consentement sollicité.');
      }else if(prepared){
        status('approval-status','success','Commande figée liée et action préparée présentées. Le formulaire envoie uniquement votre mot de passe enrôlé ; la commande reste détenue par le service, sans octet de requête.');
      }
    }catch(error){
      // The capability alone still allows the route's own digest comparison;
      // the preview refusal is shown without inventing a prerequisite.
      status('preview-status','error',t('Refus : {code}',{code:error.message}));
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
  /** Show the mediated reply verbatim; a denied state stays a refusal view. */
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
      state.dataset.state='success';
      state.textContent=t('Réponse du canal d’approbation, présentée telle quelle.');
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
  navigation();
})();
