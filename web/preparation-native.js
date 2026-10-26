/** Native manual preparation adapter for the existing document layout.
 * All mutations use the engine's shared CAS/event writer. No autosave timer,
 * provider call or agent launch. Conversion creates locked tasks only. Unknown publication retains the
 * original intention for replay; local text never becomes rendered HTML. */
import {PreparationDecisions} from './prephase-decisions.js';
const $=id=>document.getElementById(id),tr=text=>globalThis.SwarmI18n?.t(text)??text;
let ready=false,current=null,kind='besoin',base='',dirty=false,busy=false,pending=null,conflict=null,csrf='',settings={},methods=[],historyBefore=null,historyBusy=false;let conversionReview=null;
const openers=new WeakMap();
const editor=$('plain-editor');

/** Fetch only the native API. Preserve HTTP/failure metadata for known refusals;
 * network/invalid JSON remains uncertain, without a fabricated successful DTO. */
async function api(path,body){
 const response=await fetch('/api/v1/'+path,{headers:body?{'Content-Type':'application/json','X-Swarm-CSRF':csrf}:{},...(body?{method:'POST',body:JSON.stringify(body)}:{})});
 const value=await response.json();
 if(!response.ok){const error=new Error(globalThis.SwarmI18n?.engine(value.error)??value.error??tr('Action refusée.'));error.status=response.status;error.code=value.failure?.code;error.response=value;throw error;}
 return value;
}
/** Expose errors as text; caller keeps draft/pending state until a known outcome. */
function fail(error){$('error').textContent=error.message;$('error').hidden=false;}
/** Report actual UTF-8 bytes using the saved document policy; no hidden cap. */
function size(){const bytes=new TextEncoder().encode(editor.value).length;const limit=settings.preparation_document_max_bytes;
 $('size').textContent=bytes+' '+tr('octets')+(limit===null?' · '+tr('Sans plafond'):limit===undefined?'':' / '+limit);}
/** Keep all mutation controls in one state, including pending decisions and CAS
 * comparison. A disabled saved writer never prevents downloading local text. */
function controls(){
 const locked=!ready||busy||!!pending,write=ready&&settings.preparation_write_enabled==='1';
 for(const id of ['create','method-apply','advance','save','conversion-open'])$(id).disabled=busy||!write||!!conflict||(!!pending&&id!=='save'&&id!=='create');
 editor.readOnly=locked||!!conflict;$('new-title').readOnly=locked;$('new-need').readOnly=locked;
 for(const id of ['sessions','new','refresh','history','tab-besoin','tab-brief','tab-plan','decisions-open','method'])$(id).disabled=locked||!!conflict;
 const policyState=$('native-write-state');if(policyState)policyState.textContent=!ready?tr('Chargement de la préparation en cours.'):write?'':tr('Les écritures de préparation doivent être activées explicitement dans Admin. ');
 $('history').disabled ||= !current;
 $('decisions-open').disabled ||= !write;
 $('method-apply').disabled ||= !methods.some(method=>method.id===$('method').value&&method.available);
 $('save').textContent=tr(pending?'Vérifier l’enregistrement':'Enregistrer');
 for(const id of ['conflict-use','conflict-keep'])$(id).disabled=busy;
 $('conversion-open').disabled ||= !current?.plan_ready||!!current?.work_id||!!current?.conversion;
 $('conversion-confirm').disabled=busy||!write||!!conflict||(!!pending&&pending.action!=='create-missions');
 size();
}
/** Paint the returned current snapshot without claiming that a verdict launches
 * anything. Called only after response identity/revision or a fresh read. */
function paint(value){
 current=value;$('error').hidden=true;base=value.documents?.[kind]?.text??'';editor.value=base;dirty=false;
 $('title').textContent=value.title;$('documents').hidden=false;$('create-form').hidden=true;$('method-section').hidden=false;
 $('sessions').value=value.id;$('method').value=value.method;
 for(const name of ['besoin','brief','plan']){$('tab-'+name).setAttribute('aria-selected',String(name===kind));$('tab-'+name).tabIndex=name===kind?0:-1;}
 $('document-panel').setAttribute('aria-labelledby','tab-'+kind);$('document-heading').textContent=tr({besoin:'Besoin',brief:'Brief',plan:'Plan des tâches'}[kind]);
 const brief=value.documents?.brief,adopted=!!brief&&value.adopted_brief?.sha256===brief.sha256&&value.adopted_brief?.need_hash===value.documents?.besoin?.sha256;
 $('step-need').textContent=tr(value.documents?.besoin?.text?'Besoin enregistré':'Besoin à rédiger');$('step-brief').textContent=tr(adopted?'Brief adopté':'Brief à adopter');$('step-plan').textContent=tr(value.plan_ready?'Plan vérifié':'Plan à vérifier');
 $('advance').dataset.action=kind==='besoin'?'brief':kind==='brief'?(adopted?'plan':'adopt-brief'):'validate-plan';
 $('advance').textContent=tr(kind==='besoin'?'Ouvrir le brief':kind==='brief'?(adopted?'Ouvrir le plan':'Adopter ce brief'):'Vérifier le plan');
 $('next-action').textContent=tr(value.plan_ready?'Plan vérifié. Vous pouvez créer les missions sans autoriser leur démarrage.':'Enregistrez chaque document avant de poursuivre. Les réponses partielles ne valident pas le plan.');
 $('save-state').textContent=tr('Enregistré · révision ')+value.revision;
 const method=methods.find(item=>item.id===value.method);$('method-status').textContent=method?.available?method.phases.join(' → '):tr('Méthode indisponible ; rédaction possible.');
 $('conversion-state').hidden=kind!=='plan';$('conversion-status').textContent=tr(value.conversion?'Missions enregistrées. Cette opération n’autorise aucun démarrage ; consultez le pilotage pour l’état courant.':value.work_id?'Conversion vers un travail existant indisponible dans ce parcours.':'Créer les missions ne lance aucun agent.');$('conversion-pilotage').hidden=!value.conversion;if(value.conversion)$('conversion-pilotage').href='/?'+new URLSearchParams({work:value.conversion.work_id,lang:document.documentElement.lang,theme:document.documentElement.dataset.theme});
 decisions.paint(value);history.replaceState(null,'','/prepare.html?'+new URLSearchParams({id:value.id,lang:document.documentElement.lang,theme:document.documentElement.dataset.theme}));controls();
}
/** Freeze immutable CAS/event input. Create lets the engine choose its saved
 * default method; the browser never substitutes apex or another fallback. */
function intent(action,extra){return {version:1,action,event_id:crypto.randomUUID(),expected_revision:current?.revision??0,...(action==='create'?{}:{preparation_id:current.id}),...extra};}
/** Show a fresh conflicting snapshot while retaining the local editor and base.
 * Neither reading nor comparison writes a newer revision over another writer. */
async function compare(){conflict=await api('preparations/show?id='+encodeURIComponent(current.id));$('conflict-base').textContent=base;$('conflict-server').textContent=conflict.documents?.[kind]?.text??'';$('conflict').hidden=false;controls();}
/** Apply/replay exactly one intent. Acknowledged historical receipts and CAS
 * conflicts require explicit comparison; uncertain IO/JSON keeps the same event.
 * Known static pre-write refusals unlock editing. All other causes stay pending. */
async function mutate(action,extra={}){
 if(!ready){fail(Error(tr('Chargement de la préparation en cours.')));return;}
 if(busy||conflict)return;
 if(decisions.hasDraft()){fail(Error(tr('Enregistrez les décisions en cours avant de poursuivre.')));return;}
 const request=pending??intent(action,extra);pending=request;busy=true;controls();$('error').hidden=true;
 try{
  const value=await api('preparations/command',request);
  if(!value||typeof value.id!=='string'||!value.id||!Number.isSafeInteger(value.revision)||value.revision!==request.expected_revision+1||(request.preparation_id&&value.id!==request.preparation_id))throw Error(tr('Réponse de préparation incohérente. Vérifiez l’enregistrement avant de modifier le texte.'));
  pending=null;await refreshPolicy();
  if(value.receipt_historical){if(current){await compare();throw Error(tr('Préparation modifiée ailleurs. Comparez les versions avant de poursuivre.'));}current=value;kind='besoin';paint(value);await compare();await refreshList();return;}
  if(request.action==='create')kind='besoin';paint(value);await refreshList();
 }catch(error){
  if(error.code==='preparation-revision-conflict'&&current){pending=null;try{await compare();}catch(readError){fail(readError);}}
  else if(['preparation-request-invalid','preparation-invalid-plan','preparation-stale-document','preparation-stale-brief','preparation-stale-plan','preparation-stale-method','preparation-write-disabled','preparation-method-unavailable','preparation-event-conflict','preparation-document-digest-mismatch','preparation-work-invalid','preparation-task-invalid','preparation-materialization-cycle','preparation-materialization-incomplete','preparation-materialization-duplicate','preparation-materialization-dependency','preparation-already-created','preparation-existing-work-unported','task-gate-defaults-not-configured','web-csrf-refused','web-session-required','web-cookie-ambiguous','web-header-ambiguous','web-host-refused','web-origin-refused','web-target-refused'].includes(error.code))pending=null;
  fail(error);
 }finally{busy=false;controls();}
}
/** Preserve local edits instead of silently changing document/session context. */
function navigable(){if(dirty||pending||decisions.hasDraft()||(!current&&($('new-need').value||$('new-title').value))){fail(Error(tr('Enregistrez vos modifications avant de changer de document.')));return false;}return !busy&&!conflict;}
/** Read one preparation, never replacing an unsaved local document. */
async function open(id,document='besoin'){if(!navigable())return;busy=true;controls();try{const value=await api('preparations/show?id='+encodeURIComponent(id));if(value.id!==id)throw Error(tr('Réponse de préparation incohérente.'));await refreshPolicy();kind=document;paint(value);}finally{busy=false;controls();}}
/** Reuse persisted list policy. A zero/list limit does not mean no stored rows. */
async function list(){const value=await api('preparations');$('sessions').replaceChildren(new Option(tr('Choisir…'),''));for(const item of value.preparations)$('sessions').add(new Option(item.title,item.id));$('sessions').value=current?.id??'';$('list-limit').textContent=value.limit===null?tr('Sans plafond'):tr('Limite de lecture : ')+value.limit;}
/** Refresh displayed/enabled policy from saved values. Failure warns, never
 * turns a confirmed write into a failure; the writer enforces current limits. */
async function refreshPolicy(){try{settings=(await api('native-settings')).values;}catch(error){note.append(document.createTextNode(' '+tr('Certaines informations complémentaires sont indisponibles.')+' '+error.message));}}
/** A confirmed write survives a subsequent list-read error, with distinct status. */
async function refreshList(){try{await list();}catch(error){fail(Error(tr('Enregistrement confirmé ; actualisation de la liste impossible.')+' '+error.message));}}
/** Save explicit text under the writer’s current saved policy. A cached UI cap
 * never becomes a second cutoff after another Admin session changes settings. */
async function save(){if(pending)return mutate(pending.action);if(!current||busy)return;return mutate('save',{document:kind,text:editor.value});}

/** Download text as an inert local file, including unsaved/error-state drafts. */
function download(text,name){const url=URL.createObjectURL(new Blob([text],{type:'text/plain;charset=utf-8'}));const link=document.createElement('a');link.href=url;link.download=name;link.click();URL.revokeObjectURL(url);}
/** Shared accessible dialogs: retain opener, Escape/native close restore focus. */
function show(id){openers.set($(id),document.activeElement);$(id).showModal();}
/** Serialize reads of one persisted history cursor, without a local50 cap.
 * Closing/changing context discards obsolete publication, not stored history. */
async function historyPage(reset){
 if(!current||historyBusy)return;historyBusy=true;$('history-more').disabled=true;controls();
 try{
  if(reset){historyBefore=null;$('history-content').replaceChildren();show('history-dialog');}
  const id=current.id,document=kind;
  const docs=await api('preparations/history?'+new URLSearchParams({id,document,...(historyBefore?{before:historyBefore}:{})}));
  if(!$('history-dialog').open||current?.id!==id||kind!==document)return;
  for(const doc of docs){const box=globalThis.document.createElement('details'),title=globalThis.document.createElement('summary'),text=globalThis.document.createElement('pre');title.textContent=tr('Version ')+doc.revision;text.textContent=doc.text;box.append(title,text);$('history-content').append(box);}
  historyBefore=docs.at(-1)?.revision??null;$('history-more').hidden=!historyBefore||historyBefore<=1;
  if(!docs.length)$('history-content').append(globalThis.document.createTextNode(tr('Aucune version retournée sous les limites de lecture courantes.')));
 }finally{historyBusy=false;$('history-more').disabled=false;controls();}
}

const decisions=new PreparationDecisions(api,async()=>{if(dirty||pending||busy||conflict){fail(Error(tr('Enregistrez vos modifications avant de répondre aux questions.')));return null;}return current;},value=>{kind='plan';paint(value);});
/** Surface-bound event errors always remain visible; no rejected async handler. */
function on(id,event,handler){$(id).addEventListener(event,e=>{if(!e.isTrusted)return;try{Promise.resolve(handler(e)).catch(fail);}catch(error){fail(error);}});}

// Reuse the established layout; unported controls do not pretend to work.
for(const id of ['chat','project-profile-dialog','template-dialog','proposal-dialog','missions-dialog','sources-dialog','budget-dialog','preflight-details-dialog'])$(id)?.remove();
$('project-profile-heading')?.closest('details')?.remove();
for(const id of ['template-open','missions-open','editor-toggle','plan-template','version-open'])$(id).hidden=true;
for(const element of document.querySelectorAll('[maxlength]'))element.removeAttribute('maxlength');
const note=document.createElement('section');note.className='notice info';note.id='native-preparation-status';note.setAttribute('role','status');note.textContent=tr('Préparation manuelle : enregistrez avec le bouton ou Ctrl/Cmd + S. La création de missions reste sans démarrage ; le dialogue IA est indisponible.');const policyState=document.createElement('p');policyState.id='native-write-state';note.append(policyState);$('main-content').prepend(note);
$('editor-state').textContent=tr('Éditeur texte · Ctrl/Cmd + S pour enregistrer.');editor.hidden=false;$('monaco').hidden=true;
const shared=[...document.querySelectorAll('section.prep-note')].find(node=>node.querySelector('h2')?.textContent==='Documents partagés');if(shared)shared.querySelector('p').textContent=tr('Les versions enregistrées sont accessibles depuis le web et le CLI.');
const help=$('help-dialog').querySelector('section');help.replaceChildren();const paragraph=document.createElement('p');paragraph.textContent=tr('Rédigez le besoin, enregistrez le brief puis adoptez-le. Enregistrez un plan JSON v1, répondez à ses questions puis vérifiez-le. Les limites viennent des réglages Admin. Une erreur conserve votre texte ; un enregistrement incertain doit être vérifié avant toute modification.');help.append(paragraph);
const admin=document.createElement('a');admin.href='/?view=admin';admin.textContent=tr('Ouvrir les réglages Admin');note.append(document.createTextNode(' '),admin);
$('conversion-organization').remove();$('conversion-changes').remove();$('conversion-confirm').textContent=tr('Créer sans démarrer');
on('create-form','submit',async e=>{e.preventDefault();await mutate('create',{title:$('new-title').value,text:$('new-need').value});});
on('save','click',save);on('plain-editor','input',()=>{dirty=true;$('save-state').textContent=tr('Modifications non enregistrées');controls();});
on('sessions','change',async()=>{const wanted=$('sessions').value;try{if(wanted)await open(wanted);}finally{$('sessions').value=current?.id??'';}});
on('new','click',()=>{if(!navigable())return;current=null;kind='besoin';base='';editor.value='';$('documents').hidden=true;$('method-section').hidden=true;$('create-form').hidden=false;$('new-title').value='';$('new-need').value='';$('sessions').value='';$('title').textContent=tr('Préparer un projet');$('new-title').focus();});
for(const name of ['besoin','brief','plan'])on('tab-'+name,'click',()=>{if(navigable()){kind=name;paint(current);}});
on('advance','click',async()=>{if(dirty){fail(Error(tr('Enregistrez vos modifications avant de poursuivre.')));return;}const action=$('advance').dataset.action;if(['brief','plan'].includes(action)){if(navigable()){kind=action;paint(current);}}else await mutate(action,{sha256:current.documents?.[kind]?.sha256??''});});
on('method-apply','click',()=>{if(!navigable())return;return mutate('method',{method:$('method').value});});
on('refresh','click',()=>open(current.id,kind));
on('download','click',()=>download(editor.value,(current?.id??'draft')+'-'+kind+'.txt'));on('download-new','click',()=>download($('new-need').value,'besoin.txt'));
on('history','click',()=>historyPage(true).catch(error=>{$('history-content').append(document.createTextNode(error.message));}));on('history-more','click',()=>historyPage(false).catch(error=>{$('history-content').append(document.createTextNode(error.message));}));
on('conflict-use','click',async()=>{const value=conflict;conflict=null;pending=null;$('conflict').hidden=true;paint(value);await refreshList();});
on('conflict-keep','click',async()=>{const text=editor.value;current=conflict;conflict=null;pending=null;base=current.documents?.[kind]?.text??'';$('conflict').hidden=true;editor.value=text;dirty=true;await save();});
/** Review an immutable local plan/revision before explicit locked conversion.
 * This is not a server authorization or proof that policies are still current. */
on('conversion-open','click',()=>{if(!navigable()||!current?.plan_ready||current.work_id)return;try{const plan=JSON.parse(current.documents.plan.text);conversionReview={id:current.id,revision:current.revision,hash:current.documents.plan.sha256};$('conversion-target').textContent=current.title;$('conversion-explanation').textContent=tr('Créer un nouveau travail et ses tâches verrouillées. Aucun agent ne démarre ; l’équipe et l’autorisation de départ restent à configurer.');$('conversion-missions').replaceChildren(...plan.tasks.map(task=>{const li=document.createElement('li');li.textContent=task.title;return li;}));$('conversion-error').textContent='';show('conversion-dialog');}catch(error){fail(error);}});
on('conversion-confirm','click',async()=>{if(!conversionReview||current?.id!==conversionReview.id)return;await mutate('create-missions',{expected_revision:conversionReview.revision,expected_work_revision:0,sha256:conversionReview.hash});if(current?.conversion&&!pending)$('conversion-dialog').close();else $('conversion-error').textContent=$('error').textContent;});
on('help','click',()=>show('help-dialog'));
on('theme','click',()=>{const theme=document.documentElement.dataset.theme==='sombre'?'etat':'sombre';document.documentElement.dataset.theme=theme;try{localStorage.setItem('swarm-theme',theme);}catch{/* Presentation preference only; no saved policy changes. */}});
for(const dialog of document.querySelectorAll('dialog')){dialog.addEventListener('close',()=>{const opener=openers.get(dialog);openers.delete(dialog);if(document.activeElement===document.body||document.activeElement===dialog)opener?.focus();});for(const button of dialog.querySelectorAll('[data-close]'))button.addEventListener('click',()=>dialog.close());}
addEventListener('keydown',e=>{if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='s'){e.preventDefault();if(!document.querySelector('dialog[open]'))save().catch(fail);}});
addEventListener('beforeunload',e=>{if(dirty||pending||decisions.hasDraft()||(!current&&($('new-need').value||$('new-title').value))){e.preventDefault();e.returnValue='';}});
addEventListener('swarm:language-change',e=>{if(dirty||pending||decisions.hasDraft()||(!current&&($('new-need').value||$('new-title').value))){e.preventDefault();fail(Error(tr('Enregistrez vos modifications avant de changer de langue.')));}});

/** Load only implemented read services. A missing/disabled saved write policy
 * is an explicit Admin prerequisite, never a local fallback or silent migration. */
(async()=>{busy=true;controls();try{
 csrf=(await api('native-session')).csrf_token;settings=(await api('native-settings')).values;
 // Optional metadata/catalog/list failures never suppress a readable deep link.
 for(const load of [
  async()=>{methods=await api('preparations/methods');for(const method of methods){const option=new Option(method.title,method.id);option.disabled=!method.available;$('method').add(option);}},
  async()=>{try{const version=await api('native-version');$('prepare-version').textContent=version.binary.version;}catch(error){$('prepare-version').textContent=tr('Version indisponible');throw error;}},
  list,
 ]){try{await load();}catch(error){note.append(document.createTextNode(' '+tr('Certaines informations complémentaires sont indisponibles.')+' '+error.message));}}
 ready=true;busy=false;controls();$('connection').textContent=tr('Connecté');$('connection').className='status success';
 const id=new URL(location.href).searchParams.get('id');if(id)await open(id);
 }catch(error){$('connection').textContent=tr('Chargement impossible');fail(error);}finally{busy=false;controls();}})();
