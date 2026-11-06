'use strict';
/** Accounts and access (phase B, slice B8). Talks only to the authority
 * daemon's own API (`/api/v1/authority/*`, never relayed to the engine).
 * Before any daemon call, mount reads the same-origin native capability
 * document (`GET /api/v1/native-capabilities`): only `access.mode ===
 * 'authority'` proceeds to `GET …/me`; in owner-key mode the card stays
 * hidden with no daemon request at all (no doomed 404). A capability read
 * that is refused, unreachable or malformed stays an explicit visible
 * diagnostic (resolved mode 'error'), never a hidden success fallback; the
 * same holds for a failed first `GET …/me` in authority mode. Failures keep
 * their original cause/status on the thrown or resolved `error` for
 * diagnostics; only the safe translated message and stable code are shown.
 * Transport failures of daemon calls are wrapped under the stable code
 * `authority-me-unreachable` with the original error unmutated on `.cause`
 * (and its `reason` copied when a plain string); a 2xx body whose JSON decode
 * fails throws `authority-response-unreadable` (original decode cause on
 * `.cause`, never reused as a success). The /me catch keeps already-stable
 * errors unmutated (transport wrapper, daemon business codes, `http-<status>`,
 * the invalid-document code), flattens the unreadable-document carrier to
 * `authority-me-invalid` with the decode cause directly on `.cause`, and wraps
 * any other raw failure under the generic refusal code `authority-refused`
 * (original unmutated on `.cause`); the resolved diagnostic's `reason` is
 * that stable code, and only the safe translated message plus stable code
 * are rendered or logged. The shared refusal renderer (`explain`) applies on
 * every rendering path — mount diagnostics, section reload catches and
 * dialog submit states: a message that is not a known safe code (unknown or
 * hostile daemon code, prototype-inherited key) renders as the generic
 * refusal code, and the original error is read, never mutated. Every rule
 * (access policy, self-lockout, revisions, password checks) is
 * enforced by the daemon; this view never infers a right, never retries a
 * refusal and never keeps a password after its request. */
(()=>{
 const base='/api/v1/authority',roles=['viewer','operator','approver','admin'];
 /** Stable fallback code for any structured authority diagnostic that has no
  * more specific safe code; it carries no failure detail by itself. */
 const GENERIC_REFUSAL_CODE='authority-refused';
 /** English text of the daemon business-refusal codes this view can surface
  * verbatim (already published, user-intent codes — no failure detail). */
 const SAFE_DAEMON_MESSAGES={'group-exists':1,'group-missing':1,'account-exists':1,
  'account-missing':1,'account-authentication-refused':1,'authority-policy-denied':1,
  'authority-session-missing':1,'authority-attributes-line':1,'authority-password-mismatch':1,
  'authority-password-empty':1,'authority-policy-json':1,'authority-preview-required':1,
  'key-expiry-invalid':1,'key-name-invalid':1,'key-missing':1,'authority-purge-days-invalid':1,
  'authority-audit-chain-broken':1,'account-role-unknown':1,'authority-role-in-use':1,
  'authority-role-key':1,'authority-role-exists':1,'authority-role-label':1,
  'authority-condition-empty':1,'authority-policy-invalid':1,'web-route-unported':1};
 /** True for a message the view may surface as-is: a daemon-published business
  * code, one of the view's stable transport/invalid-document/unreadable-
  * response/invalid-configuration codes, or an `http-<status>` refusal marker. */
 const safeSurfaceMessage=m=>typeof m==='string'&&
  (SAFE_DAEMON_MESSAGES[m]===1||m==='authority-me-invalid'||m==='authority-me-unreachable'||
   m==='authority-response-unreadable'||m==='authority-config-invalid'||/^http-\d+$/.test(m));
 /** Message for a daemon business-refusal code. The daemon publishes these
  * codes deliberately for the UI; an unknown one keeps the generic refusal. */
 const daemonMessage=code=>SAFE_DAEMON_MESSAGES[code]===1?code:null;
 /** Stable wrapper for an unexpected transport failure: the original error
  * stays reachable, unmutated, on `.cause` for diagnostics, and its reason
  * (when a plain string) is copied onto the wrapper for direct reading. */
 const transportError=(code,cause)=>{const e=Error(code,{cause});
  if(typeof cause?.reason==='string')e.reason=cause.reason;return e};
 /** One JSON request. A fetch rejection is wrapped under the stable code
  * `authority-me-unreachable` (original error unmutated on `.cause`, its
  * plain-string `reason` copied); a 2xx body whose JSON decode fails throws
  * `authority-response-unreadable` with the decode cause on `.cause`; a
  * non-2xx response throws Error(code) with `status`/`reason`. */
 async function call(method,path,body){
  const headers={Accept:'application/json'};if(body!==undefined)headers['Content-Type']='application/json';
  let r;
  try{r=await fetch(base+path,{method,credentials:'same-origin',cache:'no-store',headers,body:body===undefined?undefined:JSON.stringify(body)})}
  catch(cause){throw transportError('authority-me-unreachable',cause)}
  let data,parseCause;
  try{data=await r.json()}catch(cause){parseCause=cause}
  if(!r.ok){const e=Error(daemonMessage(data?.error)||data?.error||'http-'+r.status);e.status=r.status;e.reason=data?.reason;throw e}
  if(parseCause!==undefined)throw Error('authority-response-unreadable',{cause:parseCause});
  return data;
 }
 /** Parse `k=v` lines; refuses a line without `=` (no silent drop). */
 function attributes(text){const out={};for(const line of text.split('\n').map(l=>l.trim()).filter(Boolean)){const i=line.indexOf('=');if(i<1)throw Error('authority-attributes-line');out[line.slice(0,i).trim()]=line.slice(i+1).trim()}return out}
 /** Stable diagnostic code for a structured consumer/mount error: the safe
  * surface message when present (stable code), else the generic refusal
  * code. Never derived from raw failure text. */
 const stableCodeOf=e=>safeSurfaceMessage(e?.message)?e.message:GENERIC_REFUSAL_CODE;
 /** Read the started access-gate mode from the same-origin native capability
  * document. Resolves 'authority' or 'owner-key'; throws Error with a stable
  * `authority-capability-*` code (and `.status` on refusal; original
  * network/parse failure retained on `cause`, never rendered) when the read
  * fails or the document is missing the mode — never a silent fallback, and
  * the returned mode grants no right (the daemon keeps authorizing). */
 async function accessMode(){
  let r;
  try{r=await fetch('/api/v1/native-capabilities',{method:'GET',credentials:'same-origin',cache:'no-store',headers:{Accept:'application/json'}})}
  catch(cause){throw Error('authority-capability-unreachable',{cause})}
  if(!r.ok){const e=Error('authority-capability-refused');e.status=r.status;throw e}
  let data=null;try{data=await r.json()}catch(cause){throw Error('authority-capability-invalid',{cause})}
  const mode=data?.access?.mode;
  if(mode!=='authority'&&mode!=='owner-key')throw Error('authority-capability-invalid');
  return mode;
 }
 const attributesText=a=>Object.entries(a||{}).map(([k,v])=>k+'='+v).join('\n');
 /** Mount into `host` (Admin card) and `nav` (header session controls).
  * Resolves `{mode:'authority'|'none'|'error', administer, reason?, error?}`
  * once the capability read settles (then, in authority mode, the first
  * daemon read). 'none' = owner-key capability: card hidden, no daemon
  * request. 'error' = capability read refused/unreachable/malformed, or a
  * failed/refused/malformed first `GET …/me` in authority mode: explicit
  * diagnostic rendered in `host` (made visible), never a hidden success
  * fallback. `error` keeps the structured cause (stable-code `message`,
  * optional `status`, original `cause`) for consumers; only the safe
  * translated message and stable code are rendered or logged. */
 async function mount(host,nav,{translate:tr=x=>x}={}){
  if(host.dataset.authorityMounted)return {mode:'mounted',administer:false};host.dataset.authorityMounted='1';
  const lang=document.documentElement.lang||'fr';
  const node=(tag,text,cls)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=tr(text);if(cls)e.className=cls;return e};
  const button=(text,cls)=>{const b=node('button',text,cls);b.type='button';return b};
  const when=s=>new Date(s*1000).toLocaleString(lang);
  const messages={
   'authority-self-lockout':'Refusé : cette modification vous retirerait l’administration.',
   'authority-revision-stale':'Refusé : la source a changé depuis votre lecture. Rechargez puis recommencez.',
   'group-exists':'Ce groupe existe déjà.',
   'group-missing':'Ce groupe n’existe plus. Rechargez la liste.',
   'authority-group-key':'Identifiant de groupe : lettres, chiffres, - ou _.',
   'authority-label-empty':'Donnez un libellé.',
   'account-exists':'Ce compte existe déjà.',
   'account-missing':'Ce compte n’existe plus. Rechargez la liste.',
   'account-authentication-refused':'Mot de passe actuel incorrect.',
   'authority-policy-denied':'Accès refusé par la politique d’accès.',
   'authority-session-missing':'Cette session est déjà terminée.',
   'authority-attributes-line':'Attributs : une ligne clé=valeur par attribut.',
   'authority-password-mismatch':'Les deux mots de passe diffèrent.',
   'authority-password-empty':'Le mot de passe est vide.',
   'authority-policy-json':'La politique n’est pas un JSON valide.',
   'authority-preview-required':'Prévisualisez ce texte exact avant d’enregistrer.',
   'key-expiry-invalid':'Durée de validité : un nombre entier de jours, au moins 1.',
   'key-name-invalid':'Donnez un nom à la clé.',
   'key-missing':'Cette clé n’existe plus.',
   'authority-purge-days-invalid':'Nombre de jours : un entier, au moins 1.',
   'authority-audit-chain-broken':'Purge refusée : la chaîne d’empreintes est rompue (elle masquerait la modification).',
   'account-role-unknown':'Rôle inconnu : enregistrez d’abord les habilitations.',
   'authority-role-in-use':'Ce rôle est encore attribué à un compte : retirez-le du compte avant de le supprimer.',
   'authority-role-key':'Identifiant de rôle : lettres, chiffres, - ou _.',
   'authority-role-exists':'Ce rôle existe déjà.',
   'authority-role-label':'Donnez un libellé au rôle.',
   'authority-condition-empty':'Choisissez au moins une habilitation et une condition.',
   'authority-policy-invalid':'Politique refusée : un rôle intégré manque, l’administrateur doit garder toutes les habilitations, ou une habilitation est inconnue.',
   'authority-capability-unreachable':'Capacité d’accès indisponible : le service natif n’a pas répondu.',
   'authority-capability-refused':'Lecture de la capacité d’accès refusée par le service natif.',
   'authority-capability-invalid':'Capacité d’accès illisible : document inattendu.',
  };
  /**
   * Shared refusal renderer, used by every rendering path of the view
   * (mount diagnostics, section reload catches, dialog submit states):
   * renders only a translated message plus a safe code. `e.message` must be
   * an error code (the view throws codes; `call()` builds its errors from
   * the daemon's published code or an `http-<status>` marker). The rendered
   * code is derived as: known local key → itself; daemon-safe surface code
   * (known to have no failure detail) → itself; anything else (unknown or
   * hostile daemon code, prototype-inherited key like 'constructor') → the
   * generic refusal code. Owner-only lookup (`Object.hasOwn`) so inherited
   * `Object.prototype` keys are never treated as known. The original error
   * is read, never mutated: its raw message/status/reason stay on the error
   * for console diagnostics and consumers. Errors: none (pure); a non-Error
   * input throws on property reads by design (all callers pass Errors).
   */
  const explain=e=>{
   const raw=e.message;
   const code=Object.hasOwn(messages,raw)?raw:
    safeSurfaceMessage(raw)?raw:GENERIC_REFUSAL_CODE;
   return (Object.hasOwn(messages,raw)?tr(messages[raw]):tr('Refus du démon d’autorité.'))+' ('+code+')';
  };
  // Capability gate before any daemon call: owner-key keeps the card absent
  // without a doomed request; a failed read stays an explicit diagnostic.
  const capability=await accessMode().catch(error=>error);
  if(capability instanceof Error){
   console.error('SwarmAuthorityAdmin:',capability.message,capability.status);
   host.hidden=false;
   host.replaceChildren(node('p',explain(capability),'notice alert'));
   return {mode:'error',administer:false,reason:capability.message,error:capability};
  }
  if(capability==='owner-key'){host.hidden=true;return {mode:'none',administer:false}}
  let me;
  try{me=await call('GET','/me');if(!me?.account?.subject)throw Error('authority-me-invalid')}
  catch(original){
   // Authority mode is established: a failed first read stays a visible
   // error, never a silent 'none'. Already-stable errors pass through
   // unmutated (transport wrapper, daemon business codes, `http-<status>`,
   // invalid-document code); the unreadable-document carrier is flattened
   // so the decode cause sits directly on `.cause`; any other raw failure
   // is wrapped under the generic refusal code — the original error is
   // NEVER mutated, only referenced on `.cause` of the stable wrapper.
   // DOM/console/reason carry only the stable code, never raw detail.
   const m=original.message;
   const error=m==='authority-me-unreachable'||m==='authority-response-unreadable'||safeSurfaceMessage(m)?
    (m==='authority-response-unreadable'?
      Error('authority-me-invalid',{cause:original.cause}):original):
    Error(GENERIC_REFUSAL_CODE,{cause:original});
   console.error('SwarmAuthorityAdmin:',stableCodeOf(error),original.status);
   host.hidden=false;
   host.replaceChildren(node('p',explain(error),'notice alert'));
   return {mode:'error',administer:false,reason:stableCodeOf(error),error};
  }
  host.hidden=false;
  // Header: who is signed in + own password change.
  const who=node('span',undefined,'native-session-user');who.textContent=me.account.display_name+' ('+me.account.subject+')';
  const own=button('Mon mot de passe');own.dataset.authorityOwnPassword='';
  const logout=nav?.querySelector('[data-action="logout"]');if(nav){nav.insertBefore(who,logout||null);nav.insertBefore(own,logout||null)}
  /** Generic modal: fields built by `build(body)`; `submit` returns a
   * message on success; errors stay visible with the input kept. */
  function dialog(title,build,submit,{submitLabel='Enregistrer',extra}={}){
   const origin=document.activeElement,d=node('dialog',undefined,'engine-settings-dialog'),form=node('form'),head=node('header',undefined,'modal-header'),body=node('div',undefined,'modal-body'),foot=node('footer',undefined,'toolbar modal-footer'),h=node('h2',title),state=node('p',undefined,'notice info'),ok=node('button',submitLabel,'primary'),close=button('Fermer');
   d.dataset.authorityDialog='';h.id='authority-dialog-'+Math.random().toString(36).slice(2);d.setAttribute('aria-labelledby',h.id);state.setAttribute('role','status');state.hidden=true;ok.type='submit';
   head.append(h);build(body);body.append(state);if(extra)foot.append(...extra(state,ok));foot.append(ok,close);form.append(head,body,foot);d.append(form);document.body.append(d);
   const done=()=>{d.close();d.remove();origin?.focus?.()};close.addEventListener('click',done);d.addEventListener('cancel',e=>{e.preventDefault();done()});
   form.addEventListener('submit',async e=>{e.preventDefault();ok.disabled=true;state.hidden=false;state.className='notice info';state.textContent=tr('Envoi…');try{const msg=await submit(state);done();if(msg)notice(msg)}catch(error){ok.disabled=false;if(error.quiet){state.hidden=true;return}state.className='notice alert';state.textContent=explain(error);state.focus?.()}});
   d.showModal();body.querySelector('input,textarea,select')?.focus();return {d,state,ok};
  }
  const field=(body,label,input)=>{const l=node('label',label,'engine-settings-field');l.append(input);body.append(l);return input};
  const input=(type,value='')=>{const i=document.createElement('input');i.type=type;i.value=value;if(type==='password')i.autocomplete='new-password';return i};
  const twice=(a,b)=>{if(!a.value)throw Error('authority-password-empty');if(a.value!==b.value)throw Error('authority-password-mismatch');return a.value};
  own.addEventListener('click',()=>{let cur,a,b;dialog('Changer mon mot de passe',body=>{cur=field(body,'Mot de passe actuel',input('password'));cur.autocomplete='current-password';a=field(body,'Nouveau mot de passe',input('password'));b=field(body,'Confirmer le nouveau mot de passe',input('password'))},async()=>{const next=twice(a,b);await call('POST','/me/password',{current:cur.value,new:next});cur.value=a.value=b.value='';return 'Mot de passe changé. Vos autres sessions sont terminées.'})});
  // Card.
  const card=node('article',undefined,'engine-settings-admin'),status=node('p',undefined,'notice info');status.setAttribute('role','status');status.tabIndex=-1;
  const summary=node('p');summary.textContent=tr('Connecté en tant que')+' '+me.account.display_name+' · '+tr('rôles')+' : '+me.account.roles.join(', ')+' · '+tr('intégrité')+' I'+me.account.integrity;
  card.append(node('h3','Comptes et accès'),node('p','Les comptes, les sessions et la politique d’accès sont tenus par le démon d’autorité, sous son propre compte système. Chaque requête du cockpit est vérifiée par lui avant d’atteindre le moteur.'),summary,status);host.replaceChildren(card);status.hidden=true;
  function notice(text,alert){status.hidden=false;status.className='notice '+(alert?'alert':'info');status.textContent=typeof text==='string'?tr(text):text;status.focus?.()}
  const group=(title,hint)=>{const g=node('details',undefined,'engine-settings-group'),s=node('summary',title),content=node('div');content.append(node('p','Chargement…','notice info'));g.append(s,node('p',hint),content);card.append(g);return content};
  const table=(heads)=>{const wrap=node('div',undefined,'engine-settings-table-wrap'),t=node('table',undefined,'engine-settings-table'),thead=node('thead'),tr0=node('tr'),tb=node('tbody');for(const h of heads){const th=node('th',h);th.scope='col';tr0.append(th)}thead.append(tr0);t.append(thead,tb);wrap.append(t);return {wrap,tb}};
  const cell=(row,text)=>{const td=node('td');td.textContent=text;row.append(td);return td};
  const toolbar=(...b)=>{const t=node('div',undefined,'toolbar');t.append(...b);return t};
  /** Section shortcuts at the top of the card (Wattson-style admin menu):
   * each opens its group, collapses the others and moves focus to it. */
  function jumpBar(){
   const groups=[...card.querySelectorAll(':scope > details.engine-settings-group')];if(groups.length<2)return;
   const bar=node('nav',undefined,'toolbar authority-tabs');bar.setAttribute('aria-label',tr('Sections de l’administration'));
   for(const g of groups){const b=button(g.querySelector('summary').textContent);b.dataset.authorityJump='';b.addEventListener('click',()=>{for(const o of groups)o.open=o===g;g.querySelector('summary').focus();g.scrollIntoView({block:'start'})});bar.append(b)}
   card.insertBefore(bar,groups[0]);
  }
  // Personal API keys (B9): every account manages its own; admins see all.
  const keysHost=group(me.administer?'Clés API':'Mes clés API','Une clé agit au nom de son compte pour des scripts, sans dépasser sa portée ni la politique d’accès. Elle ne permet jamais d’administrer comptes, sessions, clés ou politique. Le secret n’est affiché qu’une fois.');
  const scopes=['read','write','approve','administer'];
  async function loadKeys(){
   const add=button('Créer une clé','primary'),reload=button('Recharger');add.dataset.authorityKeyCreate='';reload.addEventListener('click',loadKeys);
   try{const {keys,now}=await call('GET',me.administer?'/keys':'/me/keys');const {wrap,tb}=table(['Compte','Nom','Portée','Expire le','Actions']);
    for(const k of keys){const row=node('tr');cell(row,k.subject);cell(row,k.name+' ('+k.id+')');cell(row,k.scope);cell(row,when(k.expires)+(k.expires<=now?' ('+tr('expirée')+')':''));const revoke=button('Révoquer');revoke.dataset.authorityKeyRevoke=k.id;cell(row,'').append(revoke);
     revoke.addEventListener('click',()=>{dialog(tr('Révoquer la clé')+' '+k.name,body=>{body.append(node('p','Les scripts qui l’utilisent seront refusés. La révocation est définitive.'))},async()=>{await call('POST',(k.subject===me.account.subject?'/me/keys/':'/keys/')+k.id+'/revoke',{});await loadKeys();return 'Clé révoquée.'},{submitLabel:'Révoquer'})});tb.append(row)}
    keysHost.replaceChildren(keys.length?wrap:node('p','Aucune clé.'),toolbar(add,reload));
   }catch(e){const p=node('p',undefined,'notice alert');p.textContent=explain(e);keysHost.replaceChildren(p,toolbar(reload))}
   add.addEventListener('click',()=>{let name,scope,days;dialog('Créer une clé API',body=>{name=field(body,'Nom (usage de la clé)',input('text'));scope=document.createElement('select');for(const x of scopes){const o=node('option');o.value=x;o.textContent=x;scope.append(o)}field(body,'Portée maximale',scope);days=field(body,'Durée de validité (jours)',input('text'));days.inputMode='numeric'},
    async state=>{if(!/^[1-9][0-9]*$/.test(days.value))throw Error('key-expiry-invalid');const r=await call('POST','/me/keys',{name:name.value,scope:scope.value,expires_in_seconds:Number(days.value)*86400});await loadKeys();
     setTimeout(()=>dialog('Clé créée',body=>{body.append(node('p','Copiez ce secret maintenant : il ne sera plus jamais affiché. Utilisez-le dans l’en-tête « Authorization: Bearer ».'));const out=input('text',r.secret);out.readOnly=true;field(body,'Secret',out);setTimeout(()=>out.select(),0)},async()=>'',{submitLabel:'J’ai copié le secret'}),0);return ''})});
  }
  if(!me.administer){card.append(node('p','Seuls les comptes autorisés à administrer gèrent les comptes, les sessions et la politique.'));if(me.permissions?.includes('audit.read'))buildAudit();jumpBar();await loadKeys();return {mode:'authority',administer:false,views:me.views||[]}}
  // Shared reads for the permissions matrix (role usage, ADM-03).
  let lastAccounts=[],lastGroups=[];
  // Account groups (ADM-01): rights = own roles ∪ roles of active groups.
  const groupsHost=group('Groupes','Rôles attribués à un groupe de comptes. Les droits effectifs d’un compte sont l’union de ses rôles propres et de ceux de ses groupes. Un membre retiré perd les droits du groupe à sa prochaine requête, sans fin de session.');
  async function loadGroups(){
   const add=button('Créer un groupe','primary'),reload=button('Recharger');add.dataset.authorityGroupCreate='';
   let accounts=[];
   try{accounts=(await call('GET','/accounts')).accounts}catch{} // member picker works even if the read fails
   try{const {groups}=await call('GET','/groups');lastGroups=groups;refreshMatrixIfIdle();const {wrap,tb}=table(['Groupe','Rôles','Membres','Actions']);
    for(const g of groups){const row=node('tr');cell(row,g.label+' ('+g.key+')');cell(row,g.roles.join(', ')||'—');cell(row,g.members.join(', ')||'—');
     const actions=cell(row,''),edit=button('Modifier'),dup=button('Dupliquer'),del=button('Supprimer');edit.dataset.authorityGroupEdit=dup.dataset.authorityGroupDuplicate=del.dataset.authorityGroupDelete=g.key;actions.append(edit,dup);
     edit.addEventListener('click',()=>openGroup(g,'edit'));
     dup.addEventListener('click',()=>openGroup(g,'duplicate'));
     del.addEventListener('click',()=>{dialog(tr('Supprimer le groupe')+' '+g.key,body=>{body.append(node('p','Les membres conservent leurs rôles propres et perdent les rôles du groupe à leur prochaine requête.'))},async()=>{await call('POST','/groups/'+encodeURIComponent(g.key)+'/delete',{});await loadGroups();return 'Groupe supprimé.'},{submitLabel:'Supprimer'})});
     tb.append(row)}
    groupsHost.replaceChildren(groups.length?wrap:node('p','Aucun groupe. Les comptes ne tiennent que leurs rôles propres.'),toolbar(add,reload));
   }catch(e){const p=node('p',undefined,'notice alert');p.textContent=explain(e);groupsHost.replaceChildren(p,toolbar(reload))}
   function groupFields(body,g,creating){
    const f={};if(creating){f.key=field(body,'Identifiant (lettres, chiffres, - ou _)',input('text',g?g.key+'-copie':''));f.key.autocomplete='off'}
    f.label=field(body,'Libellé',input('text',g?.label||''));
    const set=node('fieldset',undefined,'authority-roles');set.append(node('legend','Rôles du groupe'));f.roles={};for(const r of policyRoles){const l=node('label');const c=input('checkbox');c.checked=!!g?.roles?.includes(r.key);c.value=r.key;l.append(c,document.createTextNode(' '+r.label));set.append(l);f.roles[r.key]=c}body.append(set);
    const members=node('fieldset',undefined,'authority-roles');members.append(node('legend','Membres'));f.members={};for(const a of accounts){const l=node('label');const c=input('checkbox');c.checked=!!g?.members?.includes(a.subject);c.value=a.subject;l.append(c,document.createTextNode(' '+a.subject));members.append(l);f.members[a.subject]=c}body.append(members);
    return f;
   }
   const profileOf=f=>({label:f.label.value,roles:Object.keys(f.roles).filter(r=>f.roles[r].checked),members:Object.keys(f.members).filter(m=>f.members[m].checked)});
   function openGroup(g,mode){ // mode: 'create' | 'edit' | 'duplicate'
    let f;const creating=mode!=='edit';
    const target=g?'/groups/'+encodeURIComponent(g.key)+(mode==='duplicate'?'/duplicate':'/profile'):'/groups';
    dialog(mode==='create'?'Nouveau groupe':(mode==='duplicate'?tr('Dupliquer le groupe')+' '+g.key:tr('Modifier le groupe')+' '+g.key),body=>{f=groupFields(body,g,creating)},async()=>{
     const p=profileOf(f);
     if(mode==='create')await call('POST',target,{key:f.key.value,...p});
     else if(mode==='duplicate')await call('POST',target,{key:f.key.value,...p});
     else{ // edit: a changed identifier is a rename (label/roles/members still applied)
      if(f.key.value!==g.key)await call('POST','/groups/'+encodeURIComponent(g.key)+'/rename',{key:f.key.value});
      await call('POST','/groups/'+encodeURIComponent(f.key.value)+'/profile',p);
     }
     await loadGroups();return creating?'Groupe créé.':'Groupe modifié.'});
   }
   add.addEventListener('click',()=>openGroup(null,true));
  }
  // Accounts.
  const accountsHost=group('Comptes','Créer, modifier, réinitialiser ou désactiver un compte. Un compte désactivé ne peut plus se connecter et ses sessions prennent fin.');
  function profileFields(body,a,creating){
   const f={};if(creating){f.subject=field(body,'Identifiant (lettres, chiffres, - ou _)',input('text'));f.subject.autocomplete='off'}
   f.display=field(body,'Nom affiché',input('text',a?.display_name||''));
   const set=node('fieldset',undefined,'authority-roles');set.append(node('legend','Rôles'));f.roles={};for(const r of policyRoles){const l=node('label');const c=input('checkbox');c.checked=!!a?.roles?.includes(r.key);c.value=r.key;l.append(c,document.createTextNode(' '+r.label));set.append(l);f.roles[r.key]=c}body.append(set);
   f.integrity=document.createElement('select');for(const n of [1,2,3]){const o=node('option');o.value=String(n);o.textContent='I'+n;f.integrity.append(o)}f.integrity.value=String(a?.integrity||1);field(body,'Intégrité (I1 lecture, I2 écriture, I3 approbation et administration)',f.integrity);
   f.attributes=document.createElement('textarea');f.attributes.rows=3;f.attributes.value=attributesText(a?.attributes);field(body,'Attributs (une ligne clé=valeur)',f.attributes);
   if(creating){f.p1=field(body,'Mot de passe',input('password'));f.p2=field(body,'Confirmer le mot de passe',input('password'))}
   return f;
  }
  const profileOf=f=>({display_name:f.display.value,roles:Object.keys(f.roles).filter(r=>f.roles[r].checked),integrity:Number(f.integrity.value),attributes:attributes(f.attributes.value)});
  async function loadAccounts(){
   const add=button('Ajouter un compte','primary'),reload=button('Recharger');add.dataset.authorityAdd='';
   try{const {accounts,reset_requests:resets={}}=await call('GET','/accounts');lastAccounts=accounts;refreshMatrixIfIdle();const {wrap,tb}=table(['Identifiant','Nom','Rôles','Intégrité','Attributs','Actions']);
    for(const a of accounts){const row=node('tr');cell(row,a.subject+(a.subject===me.account.subject?' ('+tr('vous')+')':'')+(resets[a.subject]?' — '+tr('réinitialisation demandée le')+' '+when(resets[a.subject]):''));cell(row,a.display_name);cell(row,a.roles.join(', '));cell(row,'I'+a.integrity);cell(row,Object.entries(a.attributes).map(([k,v])=>k+'='+v).join(', ')||'—');
     const actions=cell(row,''),edit=button('Modifier'),reset=button('Mot de passe'),off=button('Désactiver');edit.dataset.authorityEdit=reset.dataset.authorityReset=off.dataset.authorityDisable=a.subject;actions.append(edit,reset);if(a.subject!==me.account.subject)actions.append(off);
     edit.addEventListener('click',()=>{let f;dialog(tr('Modifier le compte')+' '+a.subject,body=>{f=profileFields(body,a,false)},async()=>{await call('POST','/accounts/'+encodeURIComponent(a.subject)+'/profile',profileOf(f));await loadAccounts();return 'Compte modifié.'})});
     reset.addEventListener('click',()=>{let p1,p2;dialog(tr('Nouveau mot de passe pour')+' '+a.subject,body=>{body.append(node('p','Ses sessions ouvertes prennent fin.'));p1=field(body,'Nouveau mot de passe',input('password'));p2=field(body,'Confirmer le mot de passe',input('password'))},async()=>{await call('POST','/accounts/'+encodeURIComponent(a.subject)+'/password',{password:twice(p1,p2)});p1.value=p2.value='';await loadSessions();return 'Mot de passe remplacé.'})});
     off.addEventListener('click',()=>{dialog(tr('Désactiver le compte')+' '+a.subject,body=>{body.append(node('p','Le compte ne pourra plus se connecter et ses sessions prennent fin. Son historique reste dans le journal ; il faudra le recréer pour le réutiliser.'))},async()=>{await call('POST','/accounts/'+encodeURIComponent(a.subject)+'/disable',{});await loadAccounts();await loadSessions();return 'Compte désactivé.'},{submitLabel:'Désactiver'})});
     tb.append(row)}
    const waiting=Object.keys(resets).length,alertNode=waiting?node('p',undefined,'notice alert'):null;if(alertNode)alertNode.textContent=tr('Demandes de réinitialisation de mot de passe en attente')+' : '+Object.keys(resets).join(', ')+'. '+tr('Utilisez « Mot de passe » sur le compte concerné.');
    accountsHost.replaceChildren(...(alertNode?[alertNode]:[]),wrap,toolbar(add,reload));if(waiting)accountsHost.closest('details').open=true;
   }catch(e){accountsHost.replaceChildren(node('p',undefined,'notice alert'),toolbar(reload));accountsHost.firstChild.textContent=explain(e)}
   add.addEventListener('click',()=>{let f;dialog('Ajouter un compte',body=>{f=profileFields(body,null,true)},async()=>{const p=profileOf(f);await call('POST','/accounts',{subject:f.subject.value,...p,password:twice(f.p1,f.p2)});f.p1.value=f.p2.value='';await loadAccounts();return 'Compte créé.'})});
   reload.addEventListener('click',loadAccounts);
  }
  // Sessions.
  const sessionsHost=group('Sessions','Sessions ouvertes sur ce démon (tenues en mémoire : un redémarrage du service déconnecte tout le monde).');
  async function loadSessions(){
   const reload=button('Recharger');reload.addEventListener('click',loadSessions);
   try{const {sessions,current}=await call('GET','/sessions');const {wrap,tb}=table(['Compte','Ouverte','Dernière activité','Actions']);
    for(const s of sessions){const row=node('tr');cell(row,s.subject+(s.id===current?' ('+tr('cette session')+')':''));cell(row,when(s.created));cell(row,when(s.last_seen));const revoke=button('Révoquer');revoke.dataset.authorityRevoke=s.id;cell(row,'').append(revoke);
     revoke.addEventListener('click',async()=>{revoke.disabled=true;try{const r=await call('POST','/sessions/'+s.id+'/revoke',{});if(r.current){location.assign('/login');return}await loadSessions();notice('Session révoquée.')}catch(e){revoke.disabled=false;notice(explain(e),true)}});tb.append(row)}
    sessionsHost.replaceChildren(sessions.length?wrap:node('p','Aucune session ouverte.'),toolbar(reload));
   }catch(e){const p=node('p',undefined,'notice alert');p.textContent=explain(e);sessionsHost.replaceChildren(p,toolbar(reload))}
  }
  // Habilitations (policy schema 3: 14 familles + 16 vues): roles × permission matrix, edited as a
  // local draft, then validated by the daemon (preview) and saved with the
  // revision read. Built-in roles cannot be deleted; admin is locked.
  let policyRoles=roles.map(key=>({key,label:key}));
  /** Role usage in the matrix reads these lists; a late load refreshes the
   * matrix without re-reading the policy. */
  function refreshMatrixIfIdle(){if(matrixRefresh)matrixRefresh()}
  let matrixRefresh=null;
  const policyHost=group('Habilitations','Rôles et habilitations, comme une matrice : chaque case autorise une famille de routes. Les planchers d’intégrité (lecture I1, écriture I2, approbation et administration I3) s’appliquent toujours.');
  async function loadPolicy(){
   const reload=button('Recharger');reload.addEventListener('click',loadPolicy);
   let view;try{view=await call('GET','/policy')}catch(e){const p=node('p',undefined,'notice alert');p.textContent=explain(e);policyHost.replaceChildren(p,toolbar(reload));return}
   policyRoles=view.policy.roles.map(r=>({key:r.key,label:r.label}));
   const builtin=new Set(view.builtin_roles),draft=structuredClone(view.policy),state=node('p',undefined,'notice info');state.setAttribute('role','status');state.hidden=true;
   const unconditional=role=>{let g=draft.grants.find(x=>x.role===role&&!Object.keys(x.when||{}).length);if(!g){g={role,permissions:[]};draft.grants.push(g)}return g};
   const dirty=()=>{state.hidden=false;state.className='notice alert';state.textContent=tr('Modifications non enregistrées.');save.disabled=false};
   const meta=node('p');meta.textContent=tr('Source')+' : '+tr(view.source==='saved'?'enregistrée':'défaut d’installation')+' · '+tr('Révision')+' '+view.revision.slice(0,12);
   const matrixHost=node('div'),condHost=node('div');
   function renderMatrix(){
    // Section rows with per-section check-all/clear; view keys (menu rights)
    // are listed after the family keys of the same section. Orphan keys
    // (granted but unknown to the catalog) appear as a warning row.
    const catalogKeys=new Set(view.catalog.map(p=>p.key).concat(view.views.map(v=>v.key)));
    const orphans=[...new Set(draft.grants.filter(g=>!Object.keys(g.when||{}).length).flatMap(g=>g.permissions))].filter(k=>!catalogKeys.has(k));
    // Ordered union of sections: catalog sections first, then sections that
    // only hold view rows (Conduite, Suivi, Configuration, Pages…) — the two
    // catalogs share "Missions" but not their other section labels.
    const sections=[...new Set([...view.catalog.map(p=>p.section),...view.views.map(v=>v.section)])];
    const {wrap,tb}=table(['Habilitation',...draft.roles.map(r=>r.label)]);wrap.dataset.authorityMatrix='';
    const head=wrap.querySelector('thead tr');
    // Role actions under each role header.
    draft.roles.forEach((r,i)=>{const th=head.children[i+1];th.title=r.key+(r.description?' — '+r.description:'');if(r.key==='admin'){th.append(node('br'),node('small','verrouillé'));return}
     const box=node('div',undefined,'toolbar');const rn=button('Renommer'),dup=button('Dupliquer');rn.dataset.authorityRoleRename=dup.dataset.authorityRoleDuplicate=r.key;box.append(rn,dup);
     if(!builtin.has(r.key)){const del=button('Supprimer');del.dataset.authorityRoleDelete=r.key;box.append(del);del.addEventListener('click',()=>{draft.roles=draft.roles.filter(x=>x.key!==r.key);draft.grants=draft.grants.filter(g=>g.role!==r.key);renderMatrix();renderConditions();dirty()})}
     rn.addEventListener('click',()=>{let label,desc;dialog(tr('Renommer le rôle')+' '+r.key,body=>{label=field(body,'Libellé',input('text',r.label));desc=field(body,'Description',input('text',r.description||''))},async()=>{if(!label.value.trim())throw Error('authority-role-label');r.label=label.value.trim();r.description=desc.value.trim();renderMatrix();dirty();return ''},{submitLabel:'Appliquer au brouillon'})});
     dup.addEventListener('click',()=>roleDialog(r));th.append(box)});
    const held=role=>{const g=draft.grants.find(x=>x.role===role&&!Object.keys(x.when||{}).length);return g?new Set(g.permissions):new Set()};
    // Usage (ADM-03): accounts and groups holding each role, from the lists
    // already loaded; reloaded with them (the buttons call loadPolicy again).
    const usage=new Map(draft.roles.map(r=>[r.key,{accounts:0,groups:0}]));
    (lastAccounts||[]).forEach(a=>{for(const r of a.roles)if(usage.has(r))usage.get(r).accounts++});
    (lastGroups||[]).forEach(g=>{for(const r of g.roles)if(usage.has(r))usage.get(r).groups++});
    const sectionRow=section=>{
     const sr=node('tr'),sc=node('th',undefined);sc.colSpan=draft.roles.length+1;sc.scope='colgroup';
     const box=node('div',undefined,'toolbar authority-section');
     box.append(node('strong',section));
     for(const r of draft.roles){if(r.key==='admin')continue;const all=button('Tout cocher'),none=button('Tout décocher');all.dataset.authorityAll=none.dataset.authorityNone=r.key;all.addEventListener('click',()=>{const g=unconditional(r.key);g.permissions=[...new Set([...g.permissions,...items.map(p=>p.key)])];renderMatrix();dirty()});none.addEventListener('click',()=>{const g=unconditional(r.key);g.permissions=g.permissions.filter(k=>!items.some(p=>p.key===k));renderMatrix();dirty()});box.append(all,none)}
     sc.append(box);sr.append(sc);tb.append(sr);
    };
    for(const section of sections){
     const items=view.catalog.filter(p=>p.section===section);
     sectionRow(section);
     for(const p of items){
      const row=node('tr'),label=node('th');label.scope='row';label.textContent=tr(p.label)+' ';label.append(node('small','('+p.key+' · I'+view.floors[p.family]+')'));row.append(label);
      for(const r of draft.roles){const td=node('td'),c=input('checkbox');c.dataset.role=r.key;c.dataset.permission=p.key;c.setAttribute('aria-label',r.label+' — '+tr(p.label));
       c.checked=held(r.key).has(p.key);
       if(r.key==='admin'){c.checked=true;c.disabled=true}
       c.addEventListener('change',()=>{const g=unconditional(r.key);g.permissions=c.checked?[...new Set([...g.permissions,p.key])]:g.permissions.filter(x=>x!==p.key);dirty()});
       td.append(c);row.append(td)}
      tb.append(row)}
     // View keys of this section (menu entries guarded by these exclusive rights).
     for(const v of view.views.filter(v=>v.section===section)){
      const row=node('tr'),label=node('th');label.scope='row';label.textContent=tr(v.label)+' ';label.append(node('small','('+v.key+')'));row.append(label);
      for(const r of draft.roles){const td=node('td'),c=input('checkbox');c.dataset.role=r.key;c.dataset.permission=v.key;c.setAttribute('aria-label',r.label+' — '+tr(v.label));
       c.checked=held(r.key).has(v.key);
       if(r.key==='admin'){c.checked=true;c.disabled=true}
       c.addEventListener('change',()=>{const g=unconditional(r.key);g.permissions=c.checked?[...new Set([...g.permissions,v.key])]:g.permissions.filter(x=>x!==v.key);dirty()});
       td.append(c);row.append(td)}
      tb.append(row)}
    }
    if(orphans.length){const sr=node('tr'),sc=node('th',undefined);sc.colSpan=draft.roles.length+1;sc.scope='colgroup';sc.className='authority-orphans';sc.append(node('strong','Clés orphelines (attribuées mais inconnues du catalogue) : '+orphans.join(', ')));sr.append(sc);tb.append(sr)}
    // Counter: unconditional grants per role, shown under the matrix.
    const counter=node('p',undefined,'authority-counter');counter.textContent=tr('Droits par rôle')+' : '+draft.roles.map(r=>{const u=usage.get(r.key);return r.label+' '+held(r.key).size+(u&&(u.accounts||u.groups)?' ('+tr('usage')+' : '+[u.accounts?u.accounts+' '+tr('compte(s)'):'',u.groups?u.groups+' '+tr('groupe(s)'):''].filter(Boolean).join(', ')+')':'')}).join(' · ');
    matrixHost.replaceChildren(wrap,counter);
   }
   function roleDialog(from){let key,label,desc;dialog(from?tr('Dupliquer le rôle')+' '+from.key:'Nouveau rôle',body=>{key=field(body,'Identifiant du rôle (lettres, chiffres, - ou _)',input('text',from?from.key+'-copie':''));label=field(body,'Libellé',input('text',from?from.label+' '+tr('(copie)'):''));desc=field(body,'Description',input('text',from?.description||''))},
    async()=>{const k=key.value.trim();if(!/^[A-Za-z0-9_-]{1,64}$/.test(k))throw Error('authority-role-key');if(draft.roles.some(r=>r.key===k))throw Error('authority-role-exists');if(!label.value.trim())throw Error('authority-role-label');
     draft.roles.push({key:k,label:label.value.trim(),description:desc.value.trim()});if(from)for(const g of draft.grants.filter(g=>g.role===from.key))draft.grants.push({...structuredClone(g),role:k});renderMatrix();renderConditions();dirty();return ''},{submitLabel:'Appliquer au brouillon'})}
   function renderConditions(){
    const rows=draft.grants.filter(g=>Object.keys(g.when||{}).length);const {wrap,tb}=table(['Rôle','Habilitations','Conditions','Actions']);
    for(const g of rows){const row=node('tr');cell(row,g.role);cell(row,g.permissions.join(', '));cell(row,Object.entries(g.when).map(([k,v])=>k+'='+v).join(', '));const rm=button('Retirer');rm.addEventListener('click',()=>{draft.grants=draft.grants.filter(x=>x!==g);renderConditions();dirty()});cell(row,'').append(rm);tb.append(row)}
    const add=button('Ajouter une autorisation conditionnelle');add.dataset.authorityCondition='';
    add.addEventListener('click',()=>{let role,perms={},attrs;dialog('Autorisation conditionnelle',body=>{role=document.createElement('select');for(const r of draft.roles.filter(r=>r.key!=='admin')){const o=node('option');o.value=r.key;o.textContent=r.label;role.append(o)}field(body,'Rôle',role);
      const set=node('fieldset',undefined,'authority-roles');set.append(node('legend','Habilitations'));for(const p of view.catalog){const l=node('label'),c=input('checkbox');c.value=p.key;l.append(c,document.createTextNode(' '+tr(p.label)));set.append(l);perms[p.key]=c}body.append(set);
      attrs=document.createElement('textarea');attrs.rows=2;field(body,'Conditions : attributs du compte (une ligne clé=valeur)',attrs)},
     async()=>{const when=attributes(attrs.value),permissions=Object.keys(perms).filter(k=>perms[k].checked);if(!Object.keys(when).length||!permissions.length)throw Error('authority-condition-empty');draft.grants.push({role:role.value,permissions,when});renderConditions();dirty();return ''},{submitLabel:'Appliquer au brouillon'})});
    condHost.replaceChildren(node('h4','Autorisations conditionnelles (attributs du compte)'),rows.length?wrap:node('p','Aucune autorisation conditionnelle.'),toolbar(add));
   }
   const save=button('Enregistrer les habilitations','primary'),newRole=button('Nouveau rôle'),test=button('Tester une décision'),advanced=button('JSON avancé');save.dataset.authorityPolicySave='';newRole.dataset.authorityRoleNew='';save.disabled=true;
   newRole.addEventListener('click',()=>roleDialog(null));
   save.addEventListener('click',async()=>{save.disabled=true;state.hidden=false;state.className='notice info';state.textContent=tr('Prévisualisation…');
    try{const r=await call('POST','/policy/preview',{policy:draft,method:'GET',path:'/api/v1/authority/accounts',roles:me.account.roles,integrity:me.account.integrity,attributes:me.account.attributes});
     if(!r.keeps_your_administration)throw Error('authority-self-lockout');await call('POST','/policy',{policy:draft,revision:view.revision});await loadPolicy();notice('Habilitations enregistrées et appliquées.')}
    catch(e){state.className='notice alert';state.textContent=explain(e)+(e.message==='authority-role-in-use'?'':'');save.disabled=false;state.focus?.()}});
   test.addEventListener('click',()=>{let method,path,rs,integ,attrs,result;dialog('Tester une décision',body=>{body.append(node('p','Décrivez une requête et un sujet : la décision est calculée avec la politique enregistrée et avec le brouillon, sans rien modifier.'));method=field(body,'Méthode',input('text','POST'));path=field(body,'Chemin',input('text','/api/v1/mission'));rs=field(body,'Rôles (séparés par des virgules)',input('text',me.account.roles.join(',')));integ=field(body,'Intégrité',input('number',String(me.account.integrity)));attrs=document.createElement('textarea');attrs.rows=2;field(body,'Attributs (une ligne clé=valeur)',attrs);result=node('pre');body.append(result)},
    async()=>{const r=await call('POST','/policy/preview',{policy:draft,method:method.value,path:path.value,roles:rs.value.split(',').map(x=>x.trim()).filter(Boolean),integrity:Number(integ.value),attributes:attributes(attrs.value)});const d=x=>(x.allowed?tr('autorisé'):tr('refusé'))+' ('+x.permission+', '+x.reason+')';result.textContent=tr('Politique actuelle')+' : '+d(r.current)+'\n'+tr('Politique proposée')+' : '+d(r.proposed);throw Object.assign(Error('authority-test-done'),{quiet:true})},{submitLabel:'Calculer'})});
   advanced.addEventListener('click',()=>{let text;dialog('JSON avancé',body=>{body.append(node('p','Le brouillon complet (rôles et autorisations). Les modifications sont reprises dans la matrice ; rien n’est enregistré avant « Enregistrer les habilitations ».'));text=document.createElement('textarea');text.rows=16;text.spellcheck=false;text.value=JSON.stringify(draft,null,1);field(body,'Politique (JSON)',text)},
    async()=>{let next;try{next=JSON.parse(text.value)}catch{throw Error('authority-policy-json')}Object.assign(draft,next);renderMatrix();renderConditions();dirty();return ''},{submitLabel:'Appliquer au brouillon'})});
   renderMatrix();renderConditions();
   matrixRefresh=renderMatrix;
   policyHost.replaceChildren(meta,matrixHost,condHost,state,toolbar(save,newRole,test,advanced,reload));
  }
  // Daemon configuration.
  const configHost=group('Réglages du démon','Durées des sessions, coût du hachage des mots de passe (Argon2id, pour les prochains mots de passe) et taille maximale des requêtes relayées.');
  const configFields=[['session_absolute_seconds','Durée maximale d’une session (secondes)'],['session_idle_seconds','Inactivité avant fin de session (secondes)'],['argon2.memory_kib','Argon2id : mémoire (Kio)'],['argon2.passes','Argon2id : passes'],['argon2.parallelism','Argon2id : parallélisme'],['relay_body_max_bytes','Taille maximale d’une requête relayée (octets)']];
  const getPath=(o,p)=>p.split('.').reduce((v,k)=>v?.[k],o);
  async function loadConfig(){
   const edit=button('Modifier les réglages','primary'),reload=button('Recharger');edit.dataset.authorityConfig='';reload.addEventListener('click',loadConfig);
   try{const view=await call('GET','/config');const {wrap,tb}=table(['Réglage','Valeur']);for(const [k,label] of configFields){const row=node('tr');cell(row,tr(label));cell(row,String(getPath(view.config,k)));tb.append(row)}
    const meta=node('p');meta.textContent=tr('Source')+' : '+tr(view.source==='saved'?'enregistrée':'défaut d’installation')+' · '+tr('Révision')+' '+view.revision.slice(0,12);configHost.replaceChildren(meta,wrap,toolbar(edit,reload));
    edit.addEventListener('click',()=>{const inputs={};let previewed=null;const build=()=>{const c=structuredClone(view.config);for(const [k] of configFields){const t=inputs[k].value;if(!/^(0|[1-9][0-9]*)$/.test(t))throw Error('authority-config-invalid');const parts=k.split('.');let o=c;while(parts.length>1)o=o[parts.shift()];o[parts[0]]=Number(t)}return c};
     dialog('Modifier les réglages du démon',body=>{for(const [k,label] of configFields){inputs[k]=field(body,label,input('text',String(getPath(view.config,k))));inputs[k].inputMode='numeric';inputs[k].addEventListener('input',()=>{previewed=null})}},
      async()=>{const c=build();if(previewed!==JSON.stringify(c))throw Error('authority-preview-required');await call('POST','/config',{config:c,revision:view.revision});await loadConfig();return 'Réglages enregistrés et appliqués.'},
      {extra:(state,ok)=>{ok.disabled=true;const p=button('Prévisualiser');p.addEventListener('click',async()=>{state.hidden=false;try{const c=build();await call('POST','/config/preview',{config:c});previewed=JSON.stringify(c);ok.disabled=false;state.className='notice info';state.textContent=tr('Réglages valides. Vous pouvez enregistrer.')}catch(e){previewed=null;ok.disabled=true;state.className='notice alert';state.textContent=explain(e)}});return [p]}});
    });
   }catch(e){const p=node('p',undefined,'notice alert');p.textContent=explain(e);configHost.replaceChildren(p,toolbar(reload))}
  }
  // Access audit (Wattson-style, ADM-06/07/08/09/10): connections with
  // filters/sort/pagination/export, dashboard KPIs + 6 local SVG charts with
  // alternative tables + 20 recent lines, armed purge with retention,
  // daemon log, filterable administration actions tab. Shown with the
  // audit.read permission.
  function buildAudit(){
   const host=group('Journal d’accès','Connexions, consultations de pages, actions (intention puis résultat) et refus, avec le compte, la session, la durée et le moyen d’accès. La chaîne d’empreintes révèle une modification du fichier, sans l’empêcher.');
   const eventLabels={'sign-in':'connexion acceptée','sign-in-refused':'connexion refusée','sign-out':'déconnexion','intent':'intention','outcome':'résultat de la requête','consultation':'consultation','refused':'refus d’accès','reset-requested':'mot de passe oublié','purge-anchor':'purge du journal'};
   const clientLabels={browser:'navigateur','api-key':'clé API'};
   const day=d=>d?Math.floor(new Date(d+'T00:00:00Z').getTime()/1000):null;
   const from=input('date'),to=input('date'),who=input('text'),viewFilter=input('text'),clientFilter=document.createElement('select'),noAdmin=input('checkbox'),statusFilter=document.createElement('select'),q=input('text');
   who.placeholder=q.placeholder=tr('tous');noAdmin.checked=false;
   for(const [v,label] of [['','—'],['browser','navigateur'],['api-key','clé API']]){const o=node('option');o.value=v;o.textContent=tr(label);clientFilter.append(o)}
   for(const [v,label] of [['','—'],['active','active'],['signed_out','déconnectée'],['refused','refusée']]){const o=node('option');o.value=v;o.textContent=tr(label);statusFilter.append(o)}
   const filters=node('div',undefined,'engine-settings-fields');
   field(filters,'Du (inclus)',from);field(filters,'Au (inclus)',to);field(filters,'Compte',who);field(filters,'Recherche',q);
   field(filters,'Vue (view.*)',viewFilter);field(filters,'Moyen d’accès',clientFilter);field(filters,'Statut',statusFilter);
   const noAdminLabel=node('label','Sans administration','engine-settings-field');noAdminLabel.append(noAdmin,document.createTextNode(' '+tr('seulement les comptes sans droit d’administration')));filters.append(noAdminLabel);
   const qs=extra=>{const p=new URLSearchParams();const f=day(from.value),t=day(to.value);if(f!=null)p.set('from',f);if(t!=null)p.set('to',t+86400);if(who.value.trim())p.set('subject',who.value.trim());if(q.value.trim())p.set('q',q.value.trim());if(viewFilter.value.trim())p.set('view',viewFilter.value.trim());if(clientFilter.value)p.set('client',clientFilter.value);if(statusFilter.value)p.set('status',statusFilter.value);if(noAdmin.checked)p.set('no_admin','1');if(extra)for(const [k,v] of Object.entries(extra))p.set(k,v);return '?'+p.toString()};
   const tabs=node('div',undefined,'toolbar authority-tabs');tabs.setAttribute('role','tablist');const panel=node('div');panel.setAttribute('role','tabpanel');panel.tabIndex=-1;
   const tabsMap={connections:['Connexions',connections],dashboard:['Tableau de bord',dashboardView],actions:['Actions d’administration',actionsView],log:['Journal du démon',logView],purge:['Purge',purgeView]};let current='connections';
   for(const [k,[label]] of Object.entries(tabsMap)){const b=button(label);b.setAttribute('role','tab');b.dataset.auditTab=k;b.addEventListener('click',()=>show(k));tabs.append(b)}
   function show(k){current=k;for(const b of tabs.children){const on=b.dataset.auditTab===k;b.setAttribute('aria-selected',String(on));b.className=on?'primary':'';b.tabIndex=on?0:-1}panel.replaceChildren(node('p','Chargement…','notice info'));tabsMap[k][1]()}
   const apply=button('Filtrer');apply.addEventListener('click',()=>show(current));
   const chainNote=valid=>node('p',valid?'Chaîne d’empreintes intacte.':'Chaîne d’empreintes rompue : le journal a été modifié ou tronqué.','notice '+(valid?'info':'alert'));
   /** Collapsible per-section help (ADM-11), FR text translated by SwarmI18n. */
   const help=(text)=>{const d=node('details',undefined,'authority-help');d.append(node('summary','Aide'),node('p',text));return d};
   const also=(el,fn)=>{fn(el);return el};
   /** Local SVG bar chart (ADM-07): no external resource, data stays in page.
    * Returns an <svg> of `width`×`height` with one vertical bar per point. */
   function barChart(points,width,height,label){
    const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('viewBox','0 0 '+width+' '+height);svg.setAttribute('role','img');svg.setAttribute('aria-label',label);svg.setAttribute('width','100%');svg.style.maxWidth=width+'px';
    const max=Math.max(1,...points.map(p=>p.value)),n=Math.max(1,points.length),slot=width/n,bw=Math.min(slot*0.7,28);
    for(const [i,p] of points.entries()){const h=Math.round(p.value/max*(height-24)),x=i*slot+(slot-bw)/2;
     const rect=document.createElementNS('http://www.w3.org/2000/svg','rect');rect.setAttribute('x',x);rect.setAttribute('y',height-16-h);rect.setAttribute('width',bw);rect.setAttribute('height',Math.max(h,p.value?1:0));rect.setAttribute('class','authority-chart-bar');rect.appendChild(node('title',p.label+' : '+p.value));svg.append(rect);
     if(n<=14||i%Math.ceil(n/14)===0){const t=document.createElementNS('http://www.w3.org/2000/svg','text');t.setAttribute('x',x+bw/2);t.setAttribute('y',height-4);t.setAttribute('text-anchor','middle');t.setAttribute('class','authority-chart-text');t.textContent=p.label.length>10?p.label.slice(0,9)+'…':p.label;svg.append(t)}}
    return svg;
   }
   function chartBlock(title,points,unit){
    const box=node('div',undefined,'authority-chart');box.append(node('h4',title));
    if(points.some(p=>p.value))box.append(barChart(points,560,140,title));
    const alt=table([unit,'Total']);for(const p of points)if(p.value){const row=node('tr');cell(row,p.label);cell(row,String(p.value));alt.tb.append(row)}
    box.append(also(node('details',undefined,'authority-help'),d=>{d.append(node('summary','Tableau alternatif'));d.append(alt.wrap)}));
    return box;
   }
   async function connections(page=1,sort='last_at',dir='desc'){
    try{const r=await call('GET','/audit/sessions'+qs({page,per_page:50,sort,dir}));const {wrap,tb}=table(['Compte','Session','Ouverture','Dernière activité','Moyens','Vues','Consultations','Actions','Refus','Statut','Détail']);
     for(const x of r.sessions){const row=node('tr');cell(row,x.subject||'—');cell(row,x.credential+(x.signed_out?' ('+tr('déconnectée')+')':''));cell(row,when(x.first_at));cell(row,when(x.last_at));
      cell(row,(x.clients||[]).map(c=>tr(clientLabels[c]||c)).join(', ')||'—');cell(row,(x.views||[]).join(', ')||'—');
      cell(row,String(x.consultations));cell(row,String(x.actions));cell(row,String(x.refusals));cell(row,x.signed_out?tr('déconnectée'):tr('active'));
      const open=button('Voir');open.dataset.auditSession=x.credential;open.addEventListener('click',()=>detail(x));cell(row,'').append(open);tb.append(row)}
     const total=r.total!=null?r.total:r.sessions.length,pages=Math.max(1,Math.ceil(total/50));
     const nav=toolbar(),first=button('«'),prev=button('‹'),next=button('›'),last=button('»');
     for(const [b,target] of [[first,1],[prev,page-1],[next,page+1],[last,pages]]){b.disabled=target<1||target>pages||target===page;b.addEventListener('click',()=>connections(target,sort,dir));nav.append(b)}
     nav.append(node('span',tr('Page')+' '+page+' / '+pages+' — '+total+' '+tr('sessions'),'authority-pages'));
     const sortable=(label,key)=>{const b=button(label+(sort===key?(dir==='desc'?' ↓':' ↑'):''));b.className='authority-sort';b.addEventListener('click',()=>connections(1,key,sort===key&&dir==='desc'?'asc':'desc'));return b};
     const sorts=toolbar(sortable(tr('Trier : activité'),'last_at'),sortable(tr('ouverture'),'first_at'),sortable(tr('compte'),'subject'),sortable(tr('actions'),'actions'),sortable(tr('refus'),'refusals'));
     const exportBar=toolbar(button('Exporter CSV',()=>location.assign(base+'/audit/sessions'+qs({format:'csv',sort,dir}))),button('Exporter JSON',()=>location.assign(base+'/audit/sessions'+qs({format:'json',sort,dir}))));
     panel.replaceChildren(chainNote(r.chain_valid),help('Chaque ligne est une session (une connexion et ses requêtes). « Moyens » distingue le navigateur de la clé API ; « Vues » liste les droits de menu employés (none = pages moteur sans menu). Les filtres s’appliquent côté serveur : période, compte, vue, moyen, statut, sans administration, recherche.'),sorts,wrap,nav,exportBar)}catch(e){fail(e)}
   }
   async function detail(x){
    try{const r=await call('GET','/audit/sessions/'+x.credential) /* [a-z0-9:] only, path-safe */;const {wrap,tb}=table(['N°','Date','Événement','Requête','Vue','Moyen','Résultat','Durée','Détail']);
     for(const e of [...r.entries].reverse()){const row=node('tr');cell(row,String(e.seq));cell(row,when(e.at));cell(row,tr(eventLabels[e.event]||e.event));
      cell(row,e.method?e.method+' '+e.path+(e.action?' ('+e.action+')':''):'—');cell(row,e.view||'—');cell(row,e.client?tr(clientLabels[e.client]||e.client):'—');
      cell(row,e.status==null?'—':String(e.status));cell(row,e.duration_ms!=null?e.duration_ms+' ms':'—');cell(row,e.detail||'—');tb.append(row)}
     const back=button('Retour aux connexions');back.addEventListener('click',()=>show('connections'));const h=node('h4');h.textContent=tr('Session')+' '+x.credential+' — '+(x.subject||'—');panel.replaceChildren(h,wrap,toolbar(back));back.focus()}catch(e){fail(e)}
   }
   async function dashboardView(){
    try{const r=await call('GET','/audit/dashboard'+qs());const k=r.kpis||{};
     const kpiBar=node('div',undefined,'authority-kpis');
     for(const [label,value] of [[tr('Connexions'),k.sign_ins],[tr('Connexions refusées'),k.refused_sign_ins],[tr('Actions'),k.actions],[tr('Refus'),k.refusals]]){const c=node('div',undefined,'authority-kpi');c.append(node('strong',String(value??0)),node('span',label));kpiBar.append(c)}
     const d=table(['Jour (UTC)','Connexions','Connexions refusées','Comptes actifs','Consultations','Actions','Refus']);
     for(const x of r.days){const row=node('tr');for(const v of [x.day,x.sign_ins,x.refused_sign_ins,x.accounts,x.consultations,x.actions,x.refusals])cell(row,String(v));d.tb.append(row)}
     const a=table(['Compte','Sessions','Consultations','Actions','Refus','Dernière activité']);for(const x of r.accounts){const row=node('tr');for(const v of [x.subject,x.sessions,x.consultations,x.actions,x.refusals])cell(row,String(v));cell(row,when(x.last_at));a.tb.append(row)}
     const charts=node('div',undefined,'authority-charts');
     charts.append(chartBlock(tr('Connexions par jour'),r.days.map(x=>({label:x.day.slice(5),value:x.sign_ins})),tr('Jour')));
     charts.append(chartBlock(tr('Actions par jour'),r.days.map(x=>({label:x.day.slice(5),value:x.actions})),tr('Jour')));
     charts.append(chartBlock(tr('Consultations par jour'),r.days.map(x=>({label:x.day.slice(5),value:x.consultations})),tr('Jour')));
     charts.append(chartBlock(tr('Refus par jour'),r.days.map(x=>({label:x.day.slice(5),value:x.refusals})),tr('Jour')));
     charts.append(chartBlock((r.views||[]).map(v=>v.view==='none'?tr('pages moteur'):v.view),r.views.map(v=>({label:v.view==='none'?tr('pages moteur'):v.view,value:v.consultations})),tr('Vue')));
     charts.append(chartBlock(tr('Actions par vue'),(r.views||[]).map(v=>({label:v.view==='none'?tr('pages moteur'):v.view,value:v.actions})),tr('Vue')));
     const recent=table(['Date','Événement','Compte','Requête','Résultat']);
     for(const e of (r.recent||[])){const row=node('tr');cell(row,when(e.at));cell(row,tr(eventLabels[e.event]||e.event));cell(row,e.subject||'—');cell(row,e.method?e.method+' '+e.path:e.path||'—');cell(row,e.status==null?'—':String(e.status));recent.tb.append(row)}
     panel.replaceChildren(chainNote(r.chain_valid),help('Quatre indicateurs, six graphiques construits localement (aucune ressource externe) avec tableau alternatif repliable, puis les 20 dernières lignes de la période. La période suit les filtres en haut de section.'),kpiBar,charts,node('h4','Activité par jour'),r.days.length?d.wrap:node('p','Aucune activité sur la période.'),node('h4','Activité par compte'),r.accounts.length?a.wrap:node('p','Aucune activité sur la période.'),node('h4','20 dernières connexions'),(r.recent||[]).length?recent.wrap:node('p','Aucune activité sur la période.'))}catch(e){fail(e)}
   }
   async function actionsView(){
    try{const r=await call('GET','/audit'+qs());const entries=(r.entries||[]).filter(e=>e.event==='intent'&&e.at>=(day(from.value)||0)&&e.at<(day(to.value)!==null?day(to.value)+86400:Number.MAX_SAFE_INTEGER));const {wrap,tb}=table(['Date','Compte','Action','Chemin','Résultat']);
     for(const e of entries){const row=node('tr');cell(row,when(e.at));cell(row,e.subject||'—');cell(row,e.detail||e.action||'—');cell(row,e.method+' '+e.path);cell(row,e.status==null?'—':String(e.status));tb.append(row)}
     panel.replaceChildren(chainNote(r.chain_valid),help('Chaque mutation d’administration est décrite en clair (création de compte, changement de mot de passe, application de politique…) à partir du chemin demandé — jamais du corps, qui peut contenir un secret.'),entries.length?wrap:node('p','Aucune action d’administration sur la période.'))}catch(e){fail(e)}
   }
   async function logView(){
    try{const r=await call('GET','/log');const pre=node('pre',undefined,'authority-log');pre.tabIndex=0;pre.textContent=(r.lines||[]).join('\n')||tr('(journal vide)');
     const levels=r.log_levels||['error','warn','info','debug'];
     const cfg=await call('GET','/config');const cur=cfg.config.log_level||'info';
     const levelSel=document.createElement('select');for(const l of levels){const o=node('option');o.value=l;o.textContent=l;levelSel.append(o)}levelSel.value=levels.includes(cur)?cur:'info';
     const save=button('Enregistrer le niveau','primary');save.addEventListener('click',async()=>{const c=structuredClone(cfg.config);c.log_level=levelSel.value;await call('POST','/config',{config:c,revision:cfg.revision});notice('Niveau de journal enregistré et appliqué en direct.');await logView()});
     panel.replaceChildren(help('Diagnostics du démon dans un fichier borné (8 Mio, tronqué par le début). Le niveau choisit la quantité de détails : error, warn, info, debug. Le changement est appliqué en direct.'),also(node('p',undefined,'authority-log-meta'),p=>p.textContent=tr('Niveau courant')+' : '+r.level+' · '+tr('Taille maximale')+' : '+Math.round(r.max_bytes/1048576)+' Mio'),field(node('div',undefined,'engine-settings-fields'),'Niveau minimum',levelSel),toolbar(save),pre)}catch(e){fail(e)}
   }
   async function purgeView(){
    const days=input('text');days.inputMode='numeric';const run=button('Purger','primary');run.dataset.auditPurge='';
    const retention=input('text');retention.inputMode='numeric';
    let cfg=null;
    try{cfg=await call('GET','/config')}catch(e){/* retention stays hidden on config refusal */}
    const box=node('div');
    box.append(node('p','Supprime les lignes plus anciennes que le nombre de jours choisi. La chaîne reste vérifiable grâce à une ligne d’ancrage qui note la purge et son auteur. La purge manuelle est limitée à 5 appels par minute.'));
    field(box,'Supprimer les lignes de plus de (jours)',days);box.append(toolbar(run));
    if(cfg){const save=button('Enregistrer la rétention automatique');save.addEventListener('click',async()=>{
      const c=structuredClone(cfg.config);const v=retention.value.trim();
      if(v===''){delete c.audit_retention_days}else{if(!/^[1-9][0-9]*$/.test(v)){notice(explain(Error('authority-purge-days-invalid')),true);return}c.audit_retention_days=Number(v)}
      await call('POST','/config',{config:c,revision:cfg.revision});cfg=await call('GET','/config');notice(v===''?'Rétention automatique désactivée.':'Rétention enregistrée.')});
     const retBox=node('div');field(retBox,'Purge automatique : lignes de plus de (jours, vide = aucune)',retention);if(cfg.config.audit_retention_days!=null)retention.value=String(cfg.config.audit_retention_days);
     box.append(node('h4','Rétention automatique'),node('p','Le démon purge seul à son démarrage puis toutes les 6 heures, avec l’acteur « automatic ». Vide : aucune purge automatique.'),retBox,toolbar(save));
    }
    run.addEventListener('click',()=>{if(!/^[1-9][0-9]*$/.test(days.value)){notice(explain(Error('authority-purge-days-invalid')),true);return}
     dialog('Purger le journal d’accès',body=>{const p=node('p');p.textContent=tr('Les lignes de plus de {n} jours seront supprimées définitivement.').replace('{n}',days.value);body.append(p)},async()=>{const r=await call('POST','/audit/purge',{older_than_days:Number(days.value)});show('connections');return tr('Lignes supprimées')+' : '+r.removed},{submitLabel:'Purger'})});
    panel.replaceChildren(box);
   }
   function fail(e){const p=node('p',undefined,'notice alert');p.textContent=explain(e);panel.replaceChildren(p)}
   host.replaceChildren(filters,toolbar(apply),tabs,panel);
   host.parentElement.addEventListener('toggle',ev=>{if(ev.currentTarget.open&&!panel.childElementCount)show('connections')});
  }
  if(me.permissions?.includes('audit.read'))buildAudit();
  jumpBar();
  await Promise.all([loadGroups(),loadAccounts(),loadSessions(),loadPolicy(),loadConfig(),loadKeys()]);
  return {mode:'authority',administer:true,views:me.views||[]};
 }
 globalThis.SwarmAuthorityAdmin=Object.freeze({mount,call,attributes});
})();
