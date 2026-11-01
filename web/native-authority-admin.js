'use strict';
/** Accounts and access (phase B, slice B8). Talks only to the authority
 * daemon's own API (`/api/v1/authority/*`, never relayed to the engine). The
 * card stays hidden when the cockpit is not behind the daemon (owner-key
 * mode: `GET …/me` is not answered with an account). Every rule (access
 * policy, self-lockout, revisions, password checks) is enforced by the
 * daemon; this view never infers a right, never retries a refusal and never
 * keeps a password after its request. */
(()=>{
 const base='/api/v1/authority',roles=['viewer','operator','approver','admin'];
 /** One JSON request; non-2xx throws Error(code) with `status`/`reason`. */
 async function call(method,path,body){
  const headers={Accept:'application/json'};if(body!==undefined)headers['Content-Type']='application/json';
  const r=await fetch(base+path,{method,credentials:'same-origin',cache:'no-store',headers,body:body===undefined?undefined:JSON.stringify(body)});
  let data=null;try{data=await r.json()}catch{}
  if(!r.ok){const e=Error(data?.error||'http-'+r.status);e.status=r.status;e.reason=data?.reason;throw e}
  return data;
 }
 /** Parse `k=v` lines; refuses a line without `=` (no silent drop). */
 function attributes(text){const out={};for(const line of text.split('\n').map(l=>l.trim()).filter(Boolean)){const i=line.indexOf('=');if(i<1)throw Error('authority-attributes-line');out[line.slice(0,i).trim()]=line.slice(i+1).trim()}return out}
 const attributesText=a=>Object.entries(a||{}).map(([k,v])=>k+'='+v).join('\n');
 /** Mount into `host` (Admin card) and `nav` (header session controls).
  * Resolves `{mode:'authority'|'none', administer}` once the first read
  * settles, refusals included. */
 async function mount(host,nav,{translate:tr=x=>x}={}){
  if(host.dataset.authorityMounted)return {mode:'mounted',administer:false};host.dataset.authorityMounted='1';
  const lang=document.documentElement.lang||'fr';
  const node=(tag,text,cls)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=tr(text);if(cls)e.className=cls;return e};
  const button=(text,cls)=>{const b=node('button',text,cls);b.type='button';return b};
  const when=s=>new Date(s*1000).toLocaleString(lang);
  const messages={
   'authority-self-lockout':'Refusé : cette modification vous retirerait l’administration.',
   'authority-revision-stale':'Refusé : la source a changé depuis votre lecture. Rechargez puis recommencez.',
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
  };
  const explain=e=>(messages[e.message]?tr(messages[e.message]):tr('Refus du démon d’autorité.'))+' ('+e.message+')';
  let me;
  try{me=await call('GET','/me');if(!me?.account?.subject)throw Error('authority-me-invalid')}
  catch{host.hidden=true;return {mode:'none',administer:false}}
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
   form.addEventListener('submit',async e=>{e.preventDefault();ok.disabled=true;state.hidden=false;state.className='notice info';state.textContent=tr('Envoi…');try{const msg=await submit(state);done();if(msg)notice(msg)}catch(error){state.className='notice alert';state.textContent=explain(error);ok.disabled=false;state.focus?.()}});
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
  const group=(title,hint)=>{const g=node('details',undefined,'engine-settings-group'),s=node('summary',title),content=node('div');g.append(s,node('p',hint),content);card.append(g);return content};
  const table=(heads)=>{const wrap=node('div',undefined,'engine-settings-table-wrap'),t=node('table',undefined,'engine-settings-table'),thead=node('thead'),tr0=node('tr'),tb=node('tbody');for(const h of heads){const th=node('th',h);th.scope='col';tr0.append(th)}thead.append(tr0);t.append(thead,tb);wrap.append(t);return {wrap,tb}};
  const cell=(row,text)=>{const td=node('td');td.textContent=text;row.append(td);return td};
  const toolbar=(...b)=>{const t=node('div',undefined,'toolbar');t.append(...b);return t};
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
  if(!me.administer){card.append(node('p','Seuls les comptes autorisés à administrer gèrent les comptes, les sessions et la politique.'));await loadKeys();return {mode:'authority',administer:false}}
  // Accounts.
  const accountsHost=group('Comptes','Créer, modifier, réinitialiser ou désactiver un compte. Un compte désactivé ne peut plus se connecter et ses sessions prennent fin.');
  function profileFields(body,a,creating){
   const f={};if(creating){f.subject=field(body,'Identifiant (lettres, chiffres, - ou _)',input('text'));f.subject.autocomplete='off'}
   f.display=field(body,'Nom affiché',input('text',a?.display_name||''));
   const set=node('fieldset',undefined,'authority-roles');set.append(node('legend','Rôles'));f.roles={};for(const r of roles){const l=node('label');const c=input('checkbox');c.checked=!!a?.roles?.includes(r);c.value=r;l.append(c,document.createTextNode(' '+r));set.append(l);f.roles[r]=c}body.append(set);
   f.integrity=document.createElement('select');for(const n of [1,2,3]){const o=node('option');o.value=String(n);o.textContent='I'+n;f.integrity.append(o)}f.integrity.value=String(a?.integrity||1);field(body,'Intégrité (I1 lecture, I2 écriture, I3 approbation et administration)',f.integrity);
   f.attributes=document.createElement('textarea');f.attributes.rows=3;f.attributes.value=attributesText(a?.attributes);field(body,'Attributs (une ligne clé=valeur)',f.attributes);
   if(creating){f.p1=field(body,'Mot de passe',input('password'));f.p2=field(body,'Confirmer le mot de passe',input('password'))}
   return f;
  }
  const profileOf=f=>({display_name:f.display.value,roles:roles.filter(r=>f.roles[r].checked),integrity:Number(f.integrity.value),attributes:attributes(f.attributes.value)});
  async function loadAccounts(){
   const add=button('Ajouter un compte','primary'),reload=button('Recharger');add.dataset.authorityAdd='';
   try{const {accounts}=await call('GET','/accounts');const {wrap,tb}=table(['Identifiant','Nom','Rôles','Intégrité','Attributs','Actions']);
    for(const a of accounts){const row=node('tr');cell(row,a.subject+(a.subject===me.account.subject?' ('+tr('vous')+')':''));cell(row,a.display_name);cell(row,a.roles.join(', '));cell(row,'I'+a.integrity);cell(row,Object.entries(a.attributes).map(([k,v])=>k+'='+v).join(', ')||'—');
     const actions=cell(row,''),edit=button('Modifier'),reset=button('Mot de passe'),off=button('Désactiver');edit.dataset.authorityEdit=reset.dataset.authorityReset=off.dataset.authorityDisable=a.subject;actions.append(edit,reset);if(a.subject!==me.account.subject)actions.append(off);
     edit.addEventListener('click',()=>{let f;dialog(tr('Modifier le compte')+' '+a.subject,body=>{f=profileFields(body,a,false)},async()=>{await call('POST','/accounts/'+encodeURIComponent(a.subject)+'/profile',profileOf(f));await loadAccounts();return 'Compte modifié.'})});
     reset.addEventListener('click',()=>{let p1,p2;dialog(tr('Nouveau mot de passe pour')+' '+a.subject,body=>{body.append(node('p','Ses sessions ouvertes prennent fin.'));p1=field(body,'Nouveau mot de passe',input('password'));p2=field(body,'Confirmer le mot de passe',input('password'))},async()=>{await call('POST','/accounts/'+encodeURIComponent(a.subject)+'/password',{password:twice(p1,p2)});p1.value=p2.value='';await loadSessions();return 'Mot de passe remplacé.'})});
     off.addEventListener('click',()=>{dialog(tr('Désactiver le compte')+' '+a.subject,body=>{body.append(node('p','Le compte ne pourra plus se connecter et ses sessions prennent fin. Son historique reste dans le journal ; il faudra le recréer pour le réutiliser.'))},async()=>{await call('POST','/accounts/'+encodeURIComponent(a.subject)+'/disable',{});await loadAccounts();await loadSessions();return 'Compte désactivé.'},{submitLabel:'Désactiver'})});
     tb.append(row)}
    accountsHost.replaceChildren(wrap,toolbar(add,reload));
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
  // Policy.
  const policyHost=group('Politique d’accès','Autorisations par rôle, avec conditions sur les attributs du compte. Les planchers d’intégrité s’appliquent toujours et ne peuvent pas être abaissés.');
  async function loadPolicy(){
   const edit=button('Modifier la politique','primary'),reload=button('Recharger');edit.dataset.authorityPolicy='';reload.addEventListener('click',loadPolicy);
   try{const view=await call('GET','/policy');const {wrap,tb}=table(['Rôle','Actions','Conditions']);for(const g of view.policy.grants){const row=node('tr');cell(row,g.role);cell(row,g.actions.join(', '));cell(row,Object.entries(g.when||{}).map(([k,v])=>k+'='+v).join(', ')||'—');tb.append(row)}
    const floors=node('p');floors.textContent=tr('Planchers d’intégrité')+' : read I'+view.floors.read+' · write I'+view.floors.write+' · approve I'+view.floors.approve+' · administer I'+view.floors.administer+' · '+tr('Source')+' : '+tr(view.source==='saved'?'enregistrée':'défaut d’installation')+' · '+tr('Révision')+' '+view.revision.slice(0,12);
    policyHost.replaceChildren(floors,wrap,toolbar(edit,reload));
    edit.addEventListener('click',()=>{let text,method,path,rs,integ,attrs,result,previewed=null;
     dialog('Modifier la politique d’accès',body=>{text=document.createElement('textarea');text.rows=12;text.spellcheck=false;text.value=JSON.stringify(view.policy,null,1);field(body,'Politique (JSON : schema_version, grants[role, actions, when])',text);text.addEventListener('input',()=>{previewed=null});
       body.append(node('p','Essai de décision : décrivez une requête et un sujet, puis prévisualisez.'));method=field(body,'Méthode',input('text','POST'));path=field(body,'Chemin',input('text','/api/v1/mission'));rs=field(body,'Rôles (séparés par des virgules)',input('text',me.account.roles.join(',')));integ=field(body,'Intégrité',input('number',String(me.account.integrity)));attrs=document.createElement('textarea');attrs.rows=2;field(body,'Attributs (une ligne clé=valeur)',attrs);result=node('pre');body.append(result)},
      async()=>{let policy;try{policy=JSON.parse(text.value)}catch{throw Error('authority-policy-json')}if(previewed!==text.value)throw Error('authority-preview-required');await call('POST','/policy',{policy,revision:view.revision});await loadPolicy();return 'Politique enregistrée et appliquée.'},
      {extra:(state,ok)=>{ok.disabled=true;const p=button('Prévisualiser');p.addEventListener('click',async()=>{state.hidden=false;state.className='notice info';state.textContent=tr('Prévisualisation…');let policy;try{try{policy=JSON.parse(text.value)}catch{throw Error('authority-policy-json')}
        const r=await call('POST','/policy/preview',{policy,method:method.value,path:path.value,roles:rs.value.split(',').map(s=>s.trim()).filter(Boolean),integrity:Number(integ.value),attributes:attributes(attrs.value)});
        const d=x=>(x.allowed?tr('autorisé'):tr('refusé'))+' ('+x.action+', '+x.reason+')';result.textContent=tr('Politique actuelle')+' : '+d(r.current)+'\n'+tr('Politique proposée')+' : '+d(r.proposed);
        if(!r.keeps_your_administration){previewed=null;ok.disabled=true;state.className='notice alert';state.textContent=tr(messages['authority-self-lockout'])}else{previewed=text.value;ok.disabled=false;state.className='notice info';state.textContent=tr('Politique valide. Vous pouvez enregistrer.')}}catch(e){previewed=null;ok.disabled=true;state.className='notice alert';state.textContent=explain(e)}});return [p]}});
    });
   }catch(e){const p=node('p',undefined,'notice alert');p.textContent=explain(e);policyHost.replaceChildren(p,toolbar(reload))}
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
  // Access audit, built when its group opens (can be long), newest first.
  const auditHost=group('Journal d’accès','Connexions et chaque requête qui modifie (intention puis résultat), avec le compte qui l’a faite. La chaîne d’empreintes révèle une modification du fichier, sans l’empêcher.');
  const eventLabels={'sign-in':'connexion acceptée','sign-in-refused':'connexion refusée','sign-out':'déconnexion','intent':'intention','outcome':'résultat de la requête'};
  async function loadAudit(){
   const reload=button('Recharger');reload.addEventListener('click',loadAudit);
   try{const {entries,chain_valid}=await call('GET','/audit');const chain=node('p',chain_valid?'Chaîne d’empreintes intacte.':'Chaîne d’empreintes rompue : le journal a été modifié ou tronqué.','notice '+(chain_valid?'info':'alert'));const {wrap,tb}=table(['N°','Date','Événement','Compte','Requête','Résultat']);
    for(const e of [...entries].reverse()){const row=node('tr');cell(row,String(e.seq));cell(row,when(e.at));cell(row,tr(eventLabels[e.event]||e.event));cell(row,e.subject||'—');cell(row,e.method?e.method+' '+e.path+(e.action?' ('+e.action+')':''):'—');cell(row,e.status==null?'—':String(e.status));tb.append(row)}
    auditHost.replaceChildren(chain,entries.length?wrap:node('p','Aucune entrée.'),toolbar(reload));
   }catch(e){const p=node('p',undefined,'notice alert');p.textContent=explain(e);auditHost.replaceChildren(p,toolbar(reload))}
  }
  auditHost.parentElement.addEventListener('toggle',e=>{if(e.currentTarget.open&&!auditHost.childElementCount)loadAudit()});
  await Promise.all([loadAccounts(),loadSessions(),loadPolicy(),loadConfig(),loadKeys()]);
  return {mode:'authority',administer:true};
 }
 globalThis.SwarmAuthorityAdmin=Object.freeze({mount,call,attributes});
})();
