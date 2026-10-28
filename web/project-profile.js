/** Per-role observations and explicit configuration. Hashes are snapshots,
 * never agent receipts, permissions or approval of future source contents. */
const tr = source => globalThis.SwarmI18n?.t(source) ?? source;
const roleNames = {
 preparation:'Préparation',planner:'Planificateur',subplanner:'Sous-planificateur',worker:'Exécutant',reviewer:'Vérificateur'
};
const hash=value=>typeof value==='string'&&/^[0-9a-f]{64}$/u.test(value);
/** Validate metadata before rendering; a successful empty/malformed envelope
 * cannot stand in for a checked context. No source contents are accepted. */
function contextValid(value) {
 return value&&typeof value==='object'&&typeof value.role==='string'&&typeof value.name==='string'&&value.name.trim()&&(value.source===undefined||typeof value.source==='string')&&hash(value.sha256)&&hash(value.profile_sha256)&&Array.isArray(value.sources)&&value.sources.length&&value.sources.every(file=>file&&typeof file.path==='string'&&hash(file.sha256)&&Number.isSafeInteger(file.bytes)&&file.bytes>=0)&&!('prompt' in value);
}
/** Explain stable refusal reasons without inferring behavior from wording. */
function profileReason(reason){
 const messages={
  'project-profile-source-missing':'Fichier de consignes absent du projet.',
  'versioned-memory-source-guard-not-ported':'Les sources mémoire versionnées ne sont pas encore prises en charge.',
  'source-exceeds-configured-cap':'Une limite de taille des consignes est dépassée. Vérifiez les réglages Admin.',
  'recognizable-secret-no-context':'Un secret reconnaissable a été détecté. Cette source ne peut pas être transmise.',
  'project-profile-conflict':'Le profil a changé. Actualisez le catalogue et comparez votre brouillon avant de recommencer.',
  'project-profile-name-or-version':'Vérifiez le nom et la version du profil.',
  'project-profile-source-count':'Vérifiez le nombre de fichiers de consignes et les réglages Admin.',
  'project-profile-source-shape':'Vérifiez les chemins de fichiers et leurs rôles.',
  'project-profile-role-invalid':'Un rôle de consignes est invalide ou répété.',
  'project-role-has-no-source':'Chaque rôle doit disposer d’au moins une source de consignes.'
 };
 const code=typeof reason==='string'?reason:'Action refusée.';return tr(Object.hasOwn(messages,code)?messages[code]:code);
}
export async function showProjectProfile(api,{beforeWrite}={}) {
 const status=document.getElementById('project-profile-status');
 const button=document.getElementById('project-profile-open');
 const dialog=document.getElementById('project-profile-dialog');
 const list=document.getElementById('project-profile-roles');
 const settings=document.getElementById('project-profile-settings');
 const refresh=document.getElementById('project-profile-refresh');
 /** Refresh one guarded metadata view. Malformed responses and engine failures
  * remain visible; no empty-success fallback or concurrent refresh is allowed. */
 async function load() {
  refresh.disabled=true;status.className='notice info';status.setAttribute('role','status');
  status.textContent=tr('Vérification des consignes du projet…');
  button.disabled=!dialog.open;list.replaceChildren();settings.textContent='';
  try {
   const profile=await api('project-profile');
   if(!profile||typeof profile.enabled!=='boolean'||!Array.isArray(profile.roles)||!Array.isArray(profile.claude_settings_detected)||!profile.claude_settings_detected.every(v=>typeof v==='string')||(profile.enabled&&(typeof profile.name!=='string'||!profile.name||!profile.roles.length))||(!profile.enabled&&profile.roles.length)||!profile.roles.every(contextValid))throw Error(tr('Réponse de consignes du projet incohérente.'));

   status.textContent=profile.enabled?tr('Profil du projet :')+' '+profile.name:tr('Aucun profil de projet configuré.');
   for(const role of profile.roles) {
    const section=document.createElement('details');
    const title=document.createElement('summary');title.textContent=tr(Object.hasOwn(roleNames,role.role)?roleNames[role.role]:role.role);
    const receipt=document.createElement('p');receipt.textContent=tr('Empreinte du contexte :')+' '+role.sha256;
    const files=document.createElement('ul');files.className='resource-list';
    for(const source of role.sources) {
     const item=document.createElement('li');item.textContent=source.path+' · '+source.bytes+' '+tr('octets')+' · SHA-256 '+source.sha256;files.append(item);
    }
    section.append(title,files,receipt);list.append(section);
   }
   settings.textContent=tr('Configuration Claude détectée, pas transmise aux planificateurs :')+' '+(profile.claude_settings_detected.join(', ')||tr('Aucune configuration détectée.'));
   button.disabled=false;
  } catch(error) {status.className='notice alert';status.setAttribute('role','alert');status.textContent=tr('Profil du projet indisponible :')+' '+error.message;button.disabled=false;}
  finally{refresh.disabled=false;}
 }
 const configuration=mountConfiguration(dialog,api,beforeWrite,load);
 button.addEventListener('click',()=>{dialog.showModal();configuration.open();});
 document.getElementById('project-profile-close').addEventListener('click',()=>dialog.close());
 dialog.addEventListener('close',()=>button.focus());
 refresh.addEventListener('click',load);
 await load();
}

/** Mount one configuration form in the existing accessible dialog. Selection is
 * explicit, preview is readonly, and apply requires that exact unchanged draft.
 * Network/representation failures preserve the draft and its original CAS SHA;
 * refreshing never rebases unsaved edits onto another operator's configuration. */
function mountConfiguration(dialog,api,beforeWrite,readStatus) {
 const node=(tag,text)=>{const value=document.createElement(tag);if(text!==undefined)value.textContent=tr(text);return value;};
 const action=(label,handler)=>{const value=node('button',label);value.type='button';value.addEventListener('click',handler);return value;};
 const panel=node('section');panel.className='profile-config';panel.id='project-profile-configuration';panel.setAttribute('aria-labelledby','project-profile-config-title');
 const title=node('h3','Choisir les consignes du projet');title.id='project-profile-config-title';
 const notice=node('p','Ouvrez le catalogue pour examiner les profils disponibles.');notice.id='project-profile-config-status';notice.className='notice info';notice.setAttribute('role','status');
 const choiceLabel=node('label','Profil de départ');choiceLabel.htmlFor='project-profile-choice';const select=node('select');select.id='project-profile-choice';
 const nameLabel=node('label','Nom du profil');nameLabel.htmlFor='project-profile-name';const name=node('input');name.id='project-profile-name';name.type='text';
 const rows=node('div');rows.id='project-profile-sources';
 const unavailable=node('details');unavailable.id='project-profile-unavailable';unavailable.hidden=true;
 const review=node('section');review.id='project-profile-preview';review.hidden=true;review.setAttribute('aria-label',tr('Aperçu des consignes par rôle'));
 let catalogue=null,draft=false,dirty=false,busy=false,expected='',source='custom',generation=0,reviewed=null,serial=0;
 const reload=action('Actualiser le catalogue',()=>refresh());reload.id='project-profile-catalog-refresh';
 const add=action('Ajouter un fichier de consignes',()=>{source='custom';appendSource('',[]).open=true;invalidate();});add.id='project-profile-add-source';
 const preview=action('Examiner les consignes',()=>examine());preview.id='project-profile-preview-button';
 const apply=action('Enregistrer ce profil',()=>save());apply.id='project-profile-apply';apply.className='primary';
 const help=node('details');help.append(node('summary','Aide sur les sources et les rôles'),node('p','Chaque rôle doit disposer d’au moins une source Markdown ou texte du projet. Les sources sont relues avant enregistrement ; aucune permission supplémentaire n’est accordée.'),node('p','Les secrets reconnaissables et les sources mémoire non prises en charge sont refusés. Ce filtrage ne garantit pas l’absence de secret.'));
 panel.append(title,notice,reload,choiceLabel,select,unavailable,nameLabel,name,rows,add,preview,apply,node('p','Ce profil s’applique aux prochains appels. Une préparation en cours peut s’interrompre ; ses estimations restent engagées.'),review,help);
 dialog.querySelector('section').append(panel);

 /** Show a safe textual diagnostic; no untrusted HTML or assumed success. */
 function say(message,error=false){notice.textContent=message;notice.className=error?'notice alert':'notice info';notice.setAttribute('role',error?'alert':'status');}
 /** A form payload contains only explicit path/role choices, not file text. */
 function payload(){return {profile:{version:1,name:name.value,instructions:[...rows.children].map(row=>({path:row.querySelector('input[type=text]').value,roles:[...row.querySelectorAll('input[type=checkbox]:checked')].map(input=>input.value)})),source},expected_sha256:expected};}
 /** A key binds preview and apply to the same explicit draft and original SHA. */
 function key(){return JSON.stringify(payload());}
 /** Enable controls from known local state; no policy cap or role is invented. */
 function controls(){select.disabled=busy||!catalogue;name.disabled=busy||!draft;add.disabled=busy||!draft;preview.disabled=busy||!draft;apply.disabled=busy||!reviewed||reviewed!==key();reload.disabled=busy;for(const input of rows.querySelectorAll('input,button'))input.disabled=busy;}
 /** Editing invalidates preview, without rebasing expected onto a new catalog. */
 function invalidate(){generation++;dirty=true;reviewed=null;review.hidden=true;controls();}
 /** Add a reference with explicit roles and accessible native controls. Newly
  * added sources start without assigned roles; only an operator action assigns. */
 function appendSource(path,roles){
  const row=node('details');row.className='profile-source';row.dataset.serial=String(++serial);const summary=node('summary');row.append(summary);
  const label=node('label','Fichier de consignes relatif au projet');const input=node('input');input.type='text';input.value=path;input.id='project-profile-source-'+serial;label.htmlFor=input.id;
  const group=node('fieldset');group.append(node('legend','Rôles concernés'));
  const describe=()=>{const selected=[...group.querySelectorAll('input:checked')].map(value=>tr(roleNames[value.value]));summary.textContent=(input.value||tr('Fichier à renseigner'))+' · '+(selected.join(', ')||tr('Rôles à choisir'));};
  for(const [role,label]of Object.entries(roleNames)){const box=node('input');box.type='checkbox';box.value=role;box.checked=roles.includes(role);box.id=input.id+'-'+role;const text=node('label',label);text.htmlFor=box.id;text.className='profile-role';text.prepend(box);group.append(text);box.addEventListener('input',()=>{source='custom';describe();invalidate();});}
  input.addEventListener('input',()=>{source='custom';describe();invalidate();});
  const all=action('Tous les rôles',()=>{for(const box of group.querySelectorAll('input'))box.checked=true;source='custom';describe();invalidate();});
  const remove=action('Retirer cette source',()=>{row.remove();source='custom';invalidate();});
  row.append(label,input,group,all,remove);rows.append(row);describe();return row;
 }
 /** Validate an untrusted configuration DTO before using it as editable state. */
 function definition(value,unavailable=false){return value&&typeof value==='object'&&!Array.isArray(value)&&value.version===1&&typeof value.name==='string'&&(unavailable||value.name.trim())&&((unavailable&&value.instructions===null)||(Array.isArray(value.instructions)&&(unavailable||value.instructions.length)&&value.instructions.every(file=>file&&typeof file.path==='string'&&Array.isArray(file.roles)&&file.roles.length&&file.roles.every(role=>Object.hasOwn(roleNames,role)))))&&(value.source===undefined||typeof value.source==='string');}
 /** Read current suggestions only. An unsuccessful/malformed refresh retains
  * draft values and their SHA; it never authorizes a save using absent metadata. */
 async function refresh(){
  if(busy)return;busy=true;controls();say(tr('Lecture des profils disponibles…'));
  try{const value=await api('project-profile/catalog');
   if(!value||typeof value!=='object'||typeof value.active_sha256!=='string'||(value.active_sha256!==''&&!hash(value.active_sha256))||typeof value.active_name!=='string'||typeof value.active_source!=='string'||!Array.isArray(value.choices)||!value.choices.length||new Set(value.choices.map(v=>v?.id)).size!==value.choices.length||!value.choices.every(v=>v&&typeof v.id==='string'&&v.id&&v.id!=='__custom__'&&typeof v.title==='string'&&typeof v.available==='boolean'&&definition(v.profile,!v.available)&&v.profile.source===v.id&&(v.reason===undefined||typeof v.reason==='string'))||(value.active_error!==undefined&&(typeof value.active_error!=='string'||!value.active_error)))throw Error(tr('Catalogue de profils incohérent.'));
   catalogue=value;select.replaceChildren(new Option(tr('Choisir…'),''));for(const choice of value.choices){const option=new Option(tr(choice.title)+(choice.available?'':tr(' — indisponible')),choice.id);option.disabled=!choice.available;select.append(option);}select.append(new Option(tr('Nouveau profil personnalisé'),'__custom__'));
   unavailable.replaceChildren(node('summary','Profils absents ou indisponibles'));const missing=value.choices.filter(choice=>!choice.available);for(const choice of missing)unavailable.append(node('p',tr(choice.title)+' : '+profileReason(choice.reason)));unavailable.hidden=!missing.length;
   reviewed=null;review.hidden=true;
   if(dirty)say(tr('Brouillon conservé avec son empreinte d’origine. Choisissez à nouveau un profil pour utiliser le catalogue actualisé.'));
   else say(value.active_error?tr('Le profil enregistré est invalide. Examinez une configuration de remplacement avant de l’enregistrer.'):tr('Choisissez un profil, puis examinez ses sources et ses rôles.'));
  }catch(error){say(error.message,true);catalogue=null;select.replaceChildren(new Option(tr('Catalogue indisponible'),''));reviewed=null;review.hidden=true;}
  finally{busy=false;controls();}
 }
 select.addEventListener('change',()=>{
  if(busy||!catalogue||!select.value)return;
  const selected=catalogue.choices.find(choice=>choice.id===select.value);if(select.value!=='__custom__'&&!selected?.available)return;
  rows.replaceChildren();expected=catalogue.active_sha256;draft=true;source=selected?selected.profile.source:'custom';name.value=selected?selected.profile.name:catalogue.active_name;
  for(const file of selected?.profile.instructions??[])appendSource(file.path,file.roles);
  invalidate();say(tr('Examinez les consignes avant de les enregistrer.'));
 });
 name.addEventListener('input',()=>{source='custom';invalidate();});
 /** Validate an unchanged candidate through the protected readonly operation.
  * Missing/false/empty/wrong-digest responses cannot enable application. */
 async function examine(){
  if(busy||!draft)return;reviewed=null;review.hidden=true;
  const request=payload(),stamp=++generation,snapshot=JSON.stringify(request);
  if(!definition(request.profile)){say(tr('Renseignez un nom, des fichiers et leurs rôles.'),true);controls();return;}
  busy=true;controls();say(tr('Vérification des sources et des rôles…'));
  try{if(beforeWrite)await beforeWrite();const value=await api('project-profile/preview',request);
   if(stamp!==generation)return;
   if(!value||typeof value!=='object'||value.active_sha256!==request.expected_sha256||!definition(value.profile)||JSON.stringify(value.profile)!==JSON.stringify(request.profile)||!Array.isArray(value.roles)||value.roles.length!==Object.keys(roleNames).length||new Set(value.roles.map(role=>role?.role)).size!==Object.keys(roleNames).length||!value.roles.every(role=>contextValid(role)&&Object.hasOwn(roleNames,role.role)))throw Error(tr('Aperçu de profil incohérent.'));
   review.replaceChildren(node('h4','Sources examinées par rôle'));
   for(const role of value.roles){const section=node('details');section.append(node('summary',roleNames[role.role]),node('p',role.sha256));const files=node('ul');files.className='resource-list';for(const file of role.sources)files.append(node('li',file.path+' · '+file.bytes+' '+tr('octets')+' · SHA-256 '+file.sha256));section.append(files);review.append(section);}
   reviewed=snapshot;review.hidden=false;say(tr('Aperçu vérifié. Aucun fichier de profil n’a été modifié.'));
  }catch(error){say(profileReason(error.response?.failure?.reason??error.code??error.message),true);}finally{busy=false;controls();}
 }
 /** Apply only the exact reviewed draft. Any uncertain outcome keeps edits and
  * the old SHA visible; never retry, silently adopt a new SHA or start an agent. */
 async function save(){
  if(busy||!reviewed||reviewed!==key())return;
  const request=payload();busy=true;controls();say(tr('Enregistrement du profil…'));
  try{if(beforeWrite)await beforeWrite();const value=await api('project-profile/apply',request);
   if(!value||value.enabled!==true||value.name!==request.profile.name||!Array.isArray(value.roles)||value.roles.length!==Object.keys(roleNames).length||new Set(value.roles.map(role=>role?.role)).size!==Object.keys(roleNames).length||!value.roles.every(role=>contextValid(role)&&Object.hasOwn(roleNames,role.role)&&role.name===request.profile.name&&(role.source??'')===request.profile.source&&JSON.stringify(role.sources.map(file=>file.path))===JSON.stringify(request.profile.instructions.filter(file=>file.roles.includes(role.role)).map(file=>file.path)))||!Array.isArray(value.claude_settings_detected)||!value.claude_settings_detected.every(item=>typeof item==='string'))throw Error(tr('Résultat d’enregistrement incertain. Relisez le profil avant toute reprise.'));
   dirty=false;draft=false;reviewed=null;review.hidden=true;rows.replaceChildren();name.value='';await readStatus();
   say(tr('Profil du projet enregistré. Aucun agent lancé.'));catalogue=null;
  }catch(error){reviewed=null;review.hidden=true;const refused=[400,403,404,409,501].includes(error.status)&&error.response?.configuration_may_be_recorded!==true;say(tr(refused?'Profil non enregistré. Votre brouillon est conservé.':'Enregistrement non confirmé. Relisez le profil et comparez votre brouillon avant toute reprise.')+' '+profileReason(error.response?.failure?.reason??error.code??error.message),true);}
  finally{busy=false;controls();}
 }
 window.addEventListener('beforeunload',event=>{if(dirty||busy){event.preventDefault();event.returnValue='';}});
 controls();return {open(){if(!dirty&&!busy)refresh();}};
}
