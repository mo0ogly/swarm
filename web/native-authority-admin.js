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
   'authority-purge-days-invalid':'Nombre de jours : un entier, au moins 1.',
   'authority-audit-chain-broken':'Purge refusée : la chaîne d’empreintes est rompue (elle masquerait la modification).',
   'account-role-unknown':'Rôle inconnu : enregistrez d’abord les habilitations.',
   'authority-role-in-use':'Ce rôle est encore attribué à un compte : retirez-le du compte avant de le supprimer.',
   'authority-role-key':'Identifiant de rôle : lettres, chiffres, - ou _.',
   'authority-role-exists':'Ce rôle existe déjà.',
   'authority-role-label':'Donnez un libellé au rôle.',
   'authority-condition-empty':'Choisissez au moins une habilitation et une condition.',
   'authority-policy-invalid':'Politique refusée : un rôle intégré manque, l’administrateur doit garder toutes les habilitations, ou une habilitation est inconnue.',
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
  if(!me.administer){card.append(node('p','Seuls les comptes autorisés à administrer gèrent les comptes, les sessions et la politique.'));if(me.permissions?.includes('audit.read'))buildAudit();await loadKeys();return {mode:'authority',administer:false}}
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
   try{const {accounts,reset_requests:resets={}}=await call('GET','/accounts');const {wrap,tb}=table(['Identifiant','Nom','Rôles','Intégrité','Attributs','Actions']);
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
  // Habilitations (policy schema 2): roles × permission matrix, edited as a
  // local draft, then validated by the daemon (preview) and saved with the
  // revision read. Built-in roles cannot be deleted; admin is locked.
  let policyRoles=roles.map(key=>({key,label:key}));
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
    const {wrap,tb}=table(['Habilitation',...draft.roles.map(r=>r.label)]);wrap.dataset.authorityMatrix='';
    const head=wrap.querySelector('thead tr');
    // Role actions under each role header.
    draft.roles.forEach((r,i)=>{const th=head.children[i+1];th.title=r.key+(r.description?' — '+r.description:'');if(r.key==='admin'){th.append(node('br'),node('small','verrouillé'));return}
     const box=node('div',undefined,'toolbar');const rn=button('Renommer'),dup=button('Dupliquer');rn.dataset.authorityRoleRename=dup.dataset.authorityRoleDuplicate=r.key;box.append(rn,dup);
     if(!builtin.has(r.key)){const del=button('Supprimer');del.dataset.authorityRoleDelete=r.key;box.append(del);del.addEventListener('click',()=>{draft.roles=draft.roles.filter(x=>x.key!==r.key);draft.grants=draft.grants.filter(g=>g.role!==r.key);renderMatrix();renderConditions();dirty()})}
     rn.addEventListener('click',()=>{let label,desc;dialog(tr('Renommer le rôle')+' '+r.key,body=>{label=field(body,'Libellé',input('text',r.label));desc=field(body,'Description',input('text',r.description||''))},async()=>{if(!label.value.trim())throw Error('authority-role-label');r.label=label.value.trim();r.description=desc.value.trim();renderMatrix();dirty();return ''},{submitLabel:'Appliquer au brouillon'})});
     dup.addEventListener('click',()=>roleDialog(r));th.append(box)});
    let section='';
    for(const p of view.catalog){
     if(p.section!==section){section=p.section;const sr=node('tr'),sc=node('th',section);sc.colSpan=draft.roles.length+1;sc.scope='colgroup';sr.append(sc);tb.append(sr)}
     const row=node('tr'),label=node('th');label.scope='row';label.textContent=tr(p.label)+' ';label.append(node('small','('+p.key+' · I'+view.floors[p.family]+')'));row.append(label);
     for(const r of draft.roles){const td=node('td'),c=input('checkbox');c.dataset.role=r.key;c.dataset.permission=p.key;c.setAttribute('aria-label',r.label+' — '+tr(p.label));
      c.checked=!!draft.grants.find(g=>g.role===r.key&&!Object.keys(g.when||{}).length&&g.permissions.includes(p.key));
      if(r.key==='admin'){c.checked=true;c.disabled=true}
      c.addEventListener('change',()=>{const g=unconditional(r.key);g.permissions=c.checked?[...new Set([...g.permissions,p.key])]:g.permissions.filter(x=>x!==p.key);dirty()});
      td.append(c);row.append(td)}
     tb.append(row)}
    matrixHost.replaceChildren(wrap);
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
  // Access audit (Wattson-style): connections per session with drill-down,
  // activity dashboard, explicit purge. Shown with the audit.read permission.
  function buildAudit(){
   const host=group('Journal d’accès','Connexions, consultations de pages, actions (intention puis résultat) et refus, avec le compte et la session. La chaîne d’empreintes révèle une modification du fichier, sans l’empêcher.');
   const eventLabels={'sign-in':'connexion acceptée','sign-in-refused':'connexion refusée','sign-out':'déconnexion','intent':'intention','outcome':'résultat de la requête','consultation':'consultation','refused':'refus d’accès','reset-requested':'mot de passe oublié','purge-anchor':'purge du journal'};
   const day=d=>d?Math.floor(new Date(d+'T00:00:00Z').getTime()/1000):null;
   const from=input('date'),to=input('date'),who=input('text');who.placeholder=tr('tous');
   const filters=node('div',undefined,'engine-settings-fields');field(filters,'Du (inclus)',from);field(filters,'Au (inclus)',to);field(filters,'Compte',who);
   const qs=()=>{const p=new URLSearchParams();const f=day(from.value),t=day(to.value);if(f!=null)p.set('from',f);if(t!=null)p.set('to',t+86400);if(who.value.trim())p.set('subject',who.value.trim());const q=p.toString();return q?'?'+q:''};
   const tabs=node('div',undefined,'toolbar authority-tabs');tabs.setAttribute('role','tablist');const panel=node('div');panel.setAttribute('role','tabpanel');panel.tabIndex=-1;
   const views={connections:['Connexions',connections],dashboard:['Tableau de bord',dashboardView],purge:['Purge',purgeView]};let current='connections';
   for(const [k,[label]] of Object.entries(views)){const b=button(label);b.setAttribute('role','tab');b.dataset.auditTab=k;b.addEventListener('click',()=>show(k));tabs.append(b)}
   function show(k){current=k;for(const b of tabs.children){const on=b.dataset.auditTab===k;b.setAttribute('aria-selected',String(on));b.className=on?'primary':''}panel.replaceChildren(node('p','Chargement…','notice info'));views[k][1]()}
   const apply=button('Filtrer');apply.addEventListener('click',()=>show(current));
   const chainNote=valid=>node('p',valid?'Chaîne d’empreintes intacte.':'Chaîne d’empreintes rompue : le journal a été modifié ou tronqué.','notice '+(valid?'info':'alert'));
   async function connections(){
    try{const r=await call('GET','/audit/sessions'+qs());const {wrap,tb}=table(['Compte','Session','Ouverture','Dernière activité','Consultations','Actions','Refus','Détail']);
     for(const x of r.sessions){const row=node('tr');cell(row,x.subject||'—');cell(row,x.credential+(x.signed_out?' ('+tr('déconnectée')+')':''));cell(row,when(x.first_at));cell(row,when(x.last_at));cell(row,String(x.consultations));cell(row,String(x.actions));cell(row,String(x.refusals));
      const open=button('Voir');open.dataset.auditSession=x.credential;open.addEventListener('click',()=>detail(x));cell(row,'').append(open);tb.append(row)}
     panel.replaceChildren(chainNote(r.chain_valid),r.sessions.length?wrap:node('p','Aucune connexion sur la période.'))}catch(e){fail(e)}
   }
   async function detail(x){
    try{const r=await call('GET','/audit/sessions/'+x.credential) /* [a-z0-9:] only, path-safe */;const {wrap,tb}=table(['N°','Date','Événement','Requête','Résultat']);
     for(const e of [...r.entries].reverse()){const row=node('tr');cell(row,String(e.seq));cell(row,when(e.at));cell(row,tr(eventLabels[e.event]||e.event));cell(row,e.method?e.method+' '+e.path+(e.action?' ('+e.action+')':''):'—');cell(row,e.status==null?'—':String(e.status));tb.append(row)}
     const back=button('Retour aux connexions');back.addEventListener('click',()=>show('connections'));const h=node('h4');h.textContent=tr('Session')+' '+x.credential+' — '+(x.subject||'—');panel.replaceChildren(h,wrap,toolbar(back));back.focus()}catch(e){fail(e)}
   }
   async function dashboardView(){
    try{const r=await call('GET','/audit/dashboard'+qs());const d=table(['Jour (UTC)','Connexions','Connexions refusées','Comptes actifs','Consultations','Actions','Refus']);
     for(const x of r.days){const row=node('tr');for(const v of [x.day,x.sign_ins,x.refused_sign_ins,x.accounts,x.consultations,x.actions,x.refusals])cell(row,String(v));d.tb.append(row)}
     const a=table(['Compte','Sessions','Consultations','Actions','Refus','Dernière activité']);for(const x of r.accounts){const row=node('tr');for(const v of [x.subject,x.sessions,x.consultations,x.actions,x.refusals])cell(row,String(v));cell(row,when(x.last_at));a.tb.append(row)}
     panel.replaceChildren(chainNote(r.chain_valid),node('h4','Activité par jour'),r.days.length?d.wrap:node('p','Aucune activité sur la période.'),node('h4','Activité par compte'),r.accounts.length?a.wrap:node('p','Aucune activité sur la période.'))}catch(e){fail(e)}
   }
   function purgeView(){
    const days=input('text');days.inputMode='numeric';const run=button('Purger','primary');run.dataset.auditPurge='';
    const box=node('div');box.append(node('p','Supprime les lignes plus anciennes que le nombre de jours choisi. La chaîne reste vérifiable grâce à une ligne d’ancrage qui note la purge et son auteur. Aucune durée n’est appliquée automatiquement.'));field(box,'Supprimer les lignes de plus de (jours)',days);box.append(toolbar(run));
    run.addEventListener('click',()=>{if(!/^[1-9][0-9]*$/.test(days.value)){notice(explain(Error('authority-purge-days-invalid')),true);return}
     dialog('Purger le journal d’accès',body=>{const p=node('p');p.textContent=tr('Les lignes de plus de {n} jours seront supprimées définitivement.').replace('{n}',days.value);body.append(p)},async()=>{const r=await call('POST','/audit/purge',{older_than_days:Number(days.value)});show('connections');return tr('Lignes supprimées')+' : '+r.removed},{submitLabel:'Purger'})});
    panel.replaceChildren(box);
   }
   function fail(e){const p=node('p',undefined,'notice alert');p.textContent=explain(e);panel.replaceChildren(p)}
   host.replaceChildren(filters,toolbar(apply),tabs,panel);
   host.parentElement.addEventListener('toggle',ev=>{if(ev.currentTarget.open&&!panel.childElementCount)show('connections')});
  }
  if(me.permissions?.includes('audit.read'))buildAudit();
  await Promise.all([loadAccounts(),loadSessions(),loadPolicy(),loadConfig(),loadKeys()]);
  return {mode:'authority',administer:true};
 }
 globalThis.SwarmAuthorityAdmin=Object.freeze({mount,call,attributes});
})();
