/** Native manual preparation adapter for the existing document layout.
 * All mutations use the engine's shared CAS/event writer. No autosave timer,
 * synchronous agent launch. Explicit AI sends use the gated Rust worker. Conversion creates locked tasks;
 * explicit release can unblock a configured conductor. Unknown publication retains the
 * original intention for replay; local text never becomes rendered HTML. */
import {PreparationDecisions} from './prephase-decisions.js';
import {mountPreparationSources} from './preparation-native-sources.js';
import {PreparationTemplates} from './prephase-templates.js';
import {showProjectProfile} from './project-profile.js';
import {mountPreparationDialogue} from './preparation-native-dialogue.js';
const $=id=>document.getElementById(id),tr=text=>globalThis.SwarmI18n?.t(text)??text;
let ready=false,current=null,kind='besoin',base='',dirty=false,busy=false,pending=null,conflict=null,csrf='',settings={},methods=[],historyBefore=null,historyBusy=false;let conversionReview=null,sourceUI=null,templateDraft=null,dialogueUI=null;
const openers=new WeakMap();
const editor=$('plain-editor');

/** Fetch only the native API. Preserve HTTP/failure metadata for known refusals;
 * network/invalid JSON remains uncertain, without a fabricated successful DTO. */
async function api(path,body){
 const response=await fetch('/api/v1/'+path,{headers:body?{'Content-Type':'application/json','X-Swarm-CSRF':csrf}:{},...(body?{method:'POST',body:JSON.stringify(body)}:{})});
 const value=await response.json();
 if(!response.ok){
  // Preserve the server's static public reason as text, never raw causes. No control decision
  // depends on wording; known refusal logic below still uses the stable code.
  const messages=[...new Set([value?.error,value?.failure?.reason].filter(v=>typeof v==='string'&&v))].map(v=>globalThis.SwarmI18n?.engine(v)??v);
  const error=new Error(messages.join(' · ')||tr('Action refusée.'));error.status=response.status;error.code=value?.failure?.code;error.response=value;throw error;
 }
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
 for(const id of ['create','method-apply','advance','save','conversion-open','team-open'])$(id).disabled=busy||!write||!!conflict||(!!pending&&id!=='save'&&id!=='create');
 editor.readOnly=locked||!!conflict;$('new-title').readOnly=locked;$('new-need').readOnly=locked;
 for(const id of ['template-open','sessions','new','refresh','history','tab-besoin','tab-brief','tab-plan','decisions-open','method'])$(id).disabled=locked||!!conflict;
 const policyState=$('native-write-state');if(policyState)policyState.textContent=!ready?tr('Chargement de la préparation en cours.'):write?'':tr('Les écritures de préparation doivent être activées explicitement dans Admin. ');
 $('history').disabled ||= !current;
 $('decisions-open').disabled ||= !write;
 $('method-apply').disabled ||= !methods.some(method=>method.id===$('method').value&&method.available);
 $('save').textContent=tr(pending?'Vérifier l’enregistrement':'Enregistrer');
 for(const id of ['conflict-use','conflict-keep'])$(id).disabled=busy;
 const converted=!!current?.conversion,released=!!current?.conversion?.released_at,revision=converted&&current.plan_ready&&current.documents?.plan?.sha256!==current.conversion.plan_hash;
 $('conversion-open').dataset.action=revision?'revise-missions':converted?'release-plan':'create-missions';
 $('conversion-open').textContent=tr(revision?'Comparer la révision':converted?'Autoriser le démarrage des missions':'Créer les missions');
 $('conversion-open').disabled ||= converted?(!revision&&released):!current?.plan_ready;
 $('team-open').disabled ||= !current?.conversion||!!current.conversion.released_at||!!current.conversion.organization_sha256;
 for(const field of $('conversion-organization').querySelectorAll('input,select'))field.disabled=busy||!!pending;
 $('conversion-confirm').disabled=busy||!write||!!conflict||!conversionReview||(!!pending&&!['create-missions','revise-missions','release-plan','configure-team'].includes(pending.action));
 $('budget-open').disabled=!ready||busy||!!pending||!!conflict||!current;
 $('budget-save').disabled=busy||!write||!!conflict||(!!pending&&pending.action!=='budget');
 for(const field of $('budget-form').querySelectorAll('input'))field.disabled=busy||!!pending||!write;
 sourceUI?.controls();size();
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
 $('conversion-state').hidden=kind!=='plan';$('conversion-status').textContent=tr(value.conversion?.released_at?'Autorisation de départ enregistrée. Consultez le pilotage pour l’état courant.':value.conversion?.organization_sha256?'Équipe enregistrée. Le départ exige une autorisation séparée ; consultez le pilotage pour l’état courant.':value.conversion?'Missions enregistrées. Cette opération n’autorise aucun démarrage ; consultez le pilotage pour l’état courant.':value.work_id?'Conversion vers un travail existant indisponible dans ce parcours.':'Créer les missions ne lance aucun agent.');$('conversion-pilotage').hidden=!value.conversion;if(value.conversion)$('conversion-pilotage').href='/?'+new URLSearchParams({work:value.conversion.work_id,lang:document.documentElement.lang,theme:document.documentElement.dataset.theme});
 decisions.paint(value);sourceUI?.paint();dialogueUI?.paint(value);history.replaceState(null,'','/prepare.html?'+new URLSearchParams({id:value.id,lang:document.documentElement.lang,theme:document.documentElement.dataset.theme}));controls();
}
/** Freeze immutable CAS/event input. Create lets the engine choose its saved
 * default method; the browser never substitutes apex or another fallback. */
function intent(action,extra){return {version:1,action,event_id:crypto.randomUUID(),expected_revision:current?.revision??0,...(action==='create'?{}:{preparation_id:current.id}),...extra};}
/** Show a fresh conflicting snapshot while retaining the local editor and base.
 * Neither reading nor comparison writes a newer revision over another writer. */
async function compare(){conflict=await api('preparations/show?id='+encodeURIComponent(current.id));$('conflict-base').textContent=base;$('conflict-server').textContent=conflict.documents?.[kind]?.text??'';$('conflict').hidden=false;controls();}
/** Check only the acknowledged conversion shape against the submitted intent.
 * Does not attest identity, execution or acceptance. A partial/wrong200 must keep
 * the original event pending for reread/replay, including after a real commit. */
function conversionReceiptMatches(value,request,previous){
 if(!['create-missions','revise-missions','release-plan','configure-team'].includes(request.action))return true;
 const c=value?.conversion;
 if(!c||typeof value.work_id!=='string'||!value.work_id||c.work_id!==value.work_id||c.plan_hash!==request.sha256||!Array.isArray(c.task_ids)||!c.task_ids.length||c.task_ids.some(id=>typeof id!=='string'||!id)||new Set(c.task_ids).size!==c.task_ids.length)return false;
 if(previous?.work_id&&value.work_id!==previous.work_id)return false;
 if(request.action==='release-plan')return typeof c.released_at==='string'&&!!c.released_at;
 if(c.released_at!==undefined&&c.released_at!=='')return false;
 return request.action!=='configure-team'||(typeof c.organization_sha256==='string'&&/^[0-9a-f]{64}$/.test(c.organization_sha256));
}
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
  if(!conversionReceiptMatches(value,request,current))throw Error(tr('Réponse de préparation incohérente. Vérifiez l’enregistrement avant de modifier le texte.'));
  pending=null;await refreshPolicy();
  if(value.receipt_historical){if(current){await compare();throw Error(tr('Préparation modifiée ailleurs. Comparez les versions avant de poursuivre.'));}current=value;kind='besoin';paint(value);await compare();await refreshList();return;}
  if(request.action==='create')kind='besoin';paint(value);await refreshList();
 }catch(error){
  if(error.code==='preparation-revision-conflict'&&current){pending=null;conversionReview=null;try{await compare();}catch(readError){fail(readError);}}
  else if(error.code==='preparation-work-revision-conflict'){pending=null;conversionReview=null;error.message=tr('Le travail a changé. Fermez cette revue puis rouvrez-la pour comparer la version courante.');}
  else if(error.status<500&&['preparation-source-range','preparation-source-start-beyond-file','preparation-source-path-refused','preparation-source-type-refused','preparation-source-scope-refused','preparation-source-not-regular','preparation-source-file-over-limit','preparation-source-excerpt-over-limit','preparation-source-binary','preparation-source-utf8','preparation-source-secret','preparation-source-count-over-limit','preparation-source-total-over-limit','preparation-source-query-over-limit','preparation-source-absent','preparation-stale-source','preparation-source-duplicate-path','settings-preparation-sources-not-configured'].includes(error.code))pending=null;
 else if(['preparation-budget-invalid','preparation-request-invalid','preparation-invalid-plan','preparation-stale-document','preparation-stale-brief','preparation-stale-plan','preparation-stale-method','preparation-write-disabled','preparation-method-unavailable','preparation-event-conflict','preparation-document-digest-mismatch','preparation-work-invalid','preparation-task-invalid','preparation-materialization-cycle','preparation-materialization-incomplete','preparation-materialization-duplicate','preparation-materialization-dependency','preparation-already-created','preparation-already-released','preparation-work-revision-conflict','preparation-release-scope-unported','preparation-organization-mode-unported','preparation-organization-worker-role-required','preparation-organization-contract-changed','preparation-organization-already-configured','preparation-organization-history-present','preparation-organization-target-invalid','preparation-organization-task-budget','preparation-organization-validation','planning-enable-requirement-limit','agent-provider-not-configured','planning-provider-assistant-unavailable','agent-model-policy-hash-changed','agent-workspace-outside-project-or-not-directory','preparation-revision-agent-active','planning-closure-review-active','preparation-revision-task-running','preparation-revision-scope-held','preparation-revision-scope-branching','preparation-revision-controls-sense','preparation-revision-controls-criteria','preparation-revision-criteria-shrink','preparation-revision-retires-requirement','preparation-revision-retired-dependency','preparation-revision-task-id-collide','preparation-revision-new-task-unauthorized','preparation-revision-requirement-limit','preparation-existing-work-unported','task-gate-defaults-not-configured','web-csrf-refused','web-session-required','web-cookie-ambiguous','web-header-ambiguous','web-host-refused','web-origin-refused','web-target-refused'].includes(error.code))pending=null;
  fail(error);
 }finally{busy=false;controls();}
}
/** Preserve local edits instead of silently changing document/session context. */
function navigable(){if(dirty||pending||dialogueUI?.hasPending()||decisions.hasDraft()||(!current&&($('new-need').value||$('new-title').value))){fail(Error(tr('Enregistrez vos modifications avant de changer de document.')));return false;}return !busy&&!conflict;}
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
/** Retain an explicit opener across asynchronous loading that temporarily
 * disables its button. Existing synchronous callers use the current focus. */
function show(id,opener=document.activeElement){openers.set($(id),opener);$(id).showModal();}
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
$('budget-open').textContent=tr('Budget de préparation');$('main-content').prepend($('budget-open'));
$('main-content').prepend($('sources-open'));
for(const id of ['missions-dialog','preflight-details-dialog'])$(id)?.remove();
for(const id of ['missions-open','editor-toggle','plan-template','version-open'])$(id).hidden=true;
for(const element of document.querySelectorAll('[maxlength]'))element.removeAttribute('maxlength');
const note=document.createElement('section');note.className='notice info';note.id='native-preparation-status';note.setAttribute('role','status');note.textContent=tr('Préparation manuelle : enregistrez avec le bouton ou Ctrl/Cmd + S. La création de missions reste sans démarrage ; les appels IA exigent une configuration explicite.');const policyState=document.createElement('p');policyState.id='native-write-state';note.append(policyState);$('main-content').prepend(note);
$('editor-state').textContent=tr('Éditeur texte · Ctrl/Cmd + S pour enregistrer.');editor.hidden=false;$('monaco').hidden=true;
const shared=[...document.querySelectorAll('section.prep-note')].find(node=>node.querySelector('h2')?.textContent==='Documents partagés');if(shared)shared.querySelector('p').textContent=tr('Les versions enregistrées sont accessibles depuis le web et le CLI.');
const help=$('help-dialog').querySelector('section');help.replaceChildren();const paragraph=document.createElement('p');paragraph.textContent=tr('Rédigez le besoin, enregistrez le brief puis adoptez-le. Enregistrez un plan JSON v1, répondez à ses questions puis vérifiez-le. Les limites viennent des réglages Admin. Une erreur conserve votre texte ; un enregistrement incertain doit être vérifié avant toute modification.');help.append(paragraph);
const admin=document.createElement('a'),adminQuery=new URLSearchParams(location.search);adminQuery.set('view','admin');admin.href='/native-runtime.html?'+adminQuery;admin.textContent=tr('Ouvrir les réglages Admin');note.append(document.createTextNode(' '),admin,document.createTextNode(' '),$('budget-open'),document.createTextNode(' '),$('sources-open'),document.createTextNode(' '),$('template-open'),$('template-draft-status'));
dialogueUI=mountPreparationDialogue(api,{
 current:async()=>{if(!ready||busy||pending||conflict||dirty||decisions.hasDraft())throw Error(tr('Enregistrez les documents avant l’envoi.'));return current;},
 apply:async(turn,adopt=false)=>{if(dirty||decisions.hasDraft()||!current||current.id!==turn.preparation_id||current.revision!==turn.source_revision)throw Error(tr('Les documents ont changé : demandez une proposition actualisée.'));await mutate('use-proposal',{turn_id:turn.id,adopt_brief:adopt});if(pending||!$('error').hidden)throw Error($('error').textContent);return current;}
});
$('conversion-organization').hidden=true;
/** Explicit operator selections; no frontend policy ceilings or numeric defaults.
 * Human mode and CLI-only scope are shown; the writer remains authoritative. */
function teamForm(){
 const section=$('conversion-organization');section.replaceChildren();const title=document.createElement('h3'),help=document.createElement('p');title.textContent=tr('Configurer l’équipe');help.textContent=tr('Choisissez les identifiants de fournisseurs CLI configurés dans les connexions IA. Les résultats attendront une revue humaine. Aucun agent ne démarre.');section.append(title,help);
 for(const [id,text,type] of [['organization-provider','Fournisseur du responsable et du vérificateur','text'],['organization-worker','Fournisseur des exécutants','text'],['organization-workspace','Dossier de travail des agents','text'],['organization-tasks','Plafond de tâches, y compris celles ajoutées par le responsable','number'],['organization-calls','Plafond d’appels par rôle : responsable et vérificateur','number']]){const label=document.createElement('label'),input=document.createElement('input');label.htmlFor=id;label.textContent=tr(text);input.id=id;input.type=type;input.required=true;if(type==='number'){input.min='1';input.step='1';}section.append(label,input);}
 const label=document.createElement('label'),select=document.createElement('select');label.htmlFor='organization-level';label.textContent=tr('Niveau');select.id='organization-level';select.required=true;for(const [value,text] of [['','Choisir un niveau'],['auto','Réglage par défaut'],['simple','Simple'],['standard','Standard'],['exigeant','Exigeant — choix explicite']]){const option=document.createElement('option');option.value=value;option.textContent=tr(text);select.append(option);}section.append(label,select);
 const validation=document.createElement('p');validation.textContent=tr('Validation : revue humaine de chaque résultat. Les contrôles automatiques et les dépôts gérés ne sont pas configurés par ce parcours.');section.append(validation);
}
teamForm();const teamButton=document.createElement('button');teamButton.id='team-open';teamButton.type='button';teamButton.textContent=tr('Configurer l’équipe');$('conversion-open').before(teamButton);
/** Read explicit form values only. A pending event remains immutable; the server
 * validates provider sources, budgets, task contracts and workspace again. */
function teamSelection(){const maxTasks=Number($('organization-tasks').value),maxCalls=Number($('organization-calls').value);if(!Number.isSafeInteger(maxTasks)||maxTasks<1||!Number.isSafeInteger(maxCalls)||maxCalls<1||!$('organization-level').value||!$('organization-provider').value.trim()||!$('organization-worker').value.trim()||!$('organization-workspace').value.trim())throw Error(tr('Renseignez les fournisseurs, le dossier, le niveau et deux plafonds entiers positifs.'));return {provider:$('organization-provider').value.trim(),worker_provider:$('organization-worker').value.trim(),workspace:$('organization-workspace').value.trim(),level:$('organization-level').value,validation:'human',max_tasks:maxTasks,max_calls:maxCalls};}
$('conversion-changes').remove();$('conversion-confirm').textContent=tr('Créer sans démarrer');
/** Reuse the current questionnaire/graph. Never replace a preparation or text
 * already entered; selection only stages a local need/plan and explicit method. */
const templates=new PreparationTemplates(api,()=>methods,(title,need,method,result)=>{
 if(!ready||current||busy||pending||conflict||$('new-title').value||$('new-need').value)return false;
 templateDraft=result;$('new-title').value=title;$('new-need').value=need;$('template-draft-status').textContent=tr('Brouillon local issu du modèle : créez la préparation pour conserver le besoin. Le plan proposé restera à enregistrer et à vérifier.');controls();return true;
});
// Optional project metadata never suppresses the main preparation editor.
showProjectProfile(api,{beforeWrite:async()=>{
 // Profile configuration can open before preparation bootstrap finishes. Obtain
 // the same protected native session without inventing a token or policy value.
 if(!/^[0-9a-f]{64}$/u.test(csrf)){const session=await api('native-session');if(!session||session.csrf_format!=='hmac-sha256-v1'||typeof session.csrf_token!=='string'||!/^[0-9a-f]{64}$/u.test(session.csrf_token))throw Error(tr('Session native incohérente.'));csrf=session.csrf_token;}
}}).catch(fail);
sourceUI=mountPreparationSources({getState:()=>({current,settings,ready,busy,pending,conflict}),setBusy:value=>{busy=value;controls();},api,mutate,navigable,on,show,controls,adminHref:admin.href});
/** Validate the consumed financial DTO before presenting a successful read.
 * Browser counts must be exact nonnegative integers; this is a representation
 * bound, not an execution policy. Missing or inconsistent fields refuse rather
 * than becoming zero/unknown coverage. No state or reservation is changed. */
function budgetRead(value){
 const money=n=>Number.isFinite(n)&&n>=0,object=v=>v&&typeof v==='object'&&!Array.isArray(v);
 const envelope=v=>object(v)&&object(v.budget)&&money(v.budget.limit_usd)&&money(v.budget.reserve_per_launch_usd)&&['reserved_usd','committed_estimate_usd','remaining_usd'].every(k=>money(v[k]))&&typeof v.warning==='boolean'&&(v.actual_cost_usd===null||money(v.actual_cost_usd));
 if(!envelope(value)||!['turns','usage_turns','cost_turns','input_tokens','output_tokens'].every(k=>Number.isSafeInteger(value[k])&&value[k]>=0)||value.usage_turns>value.turns||value.cost_turns>value.usage_turns||((value.actual_cost_usd===null)!==(value.cost_turns===0))||(value.work_budget!==undefined&&!envelope(value.work_budget)))throw Error(tr('Réponse de budget incohérente. Relisez les données avant de poursuivre.'));
 return value;
}
/** Read actual reservation/cost coverage before opening the budget modal.
 * Missing costs remain unknown, reported zero remains zero. Reading grants no
 * lease: saving still binds current preparation CAS and may require comparison.
 * Errors stay visible and never fabricate zero usage or alter any reservation. */
on('budget-open','click',async()=>{if(!navigable()||!current)return;busy=true;controls();try{const view=budgetRead(await api('preparations/budget?id='+encodeURIComponent(current.id))),b=current.budget;$('budget-limit').value=b?.limit_usd??'';$('budget-reserve').value=b?.reserve_per_launch_usd??'';$('budget-source').value=b?.estimate_source??'';$('budget-date').value=b?.reference_date??'';$('budget-title').textContent=tr('Enveloppe de préparation');$('budget-summary').className='notice '+(view.warning?'attention':'info');$('budget-summary').textContent=tr('Choisissez explicitement une enveloppe en USD. Zéro signifie sans limite, pas un coût nul.')+(view.warning?' '+tr('Seuil d’alerte budgétaire atteint selon les réglages Admin.'):'');$('budget-usage').textContent=tr('Réservé (USD) : ')+view.reserved_usd+' · '+tr('Estimation engagée (USD) : ')+view.committed_estimate_usd+' · '+tr('Coût rapporté (USD) : ')+(view.actual_cost_usd===null?tr('inconnu'):view.actual_cost_usd)+' · '+tr('Échanges avec un coût : ')+view.cost_turns+'/'+view.turns+' · '+tr('Échanges avec des jetons : ')+view.usage_turns+'/'+view.turns+' · '+tr('Jetons entrée/sortie : ')+view.input_tokens+'/'+view.output_tokens;$('budget-work').textContent=view.work_budget?tr('Budget du travail (USD), limite/réservé/engagé : ')+view.work_budget.budget.limit_usd+'/'+view.work_budget.reserved_usd+'/'+view.work_budget.committed_estimate_usd:tr('Aucun budget de travail lié.');$('budget-policy').textContent=tr('Les coûts sont des déclarations partielles du fournisseur, pas une facture. Les réservations et coûts existants sont conservés.');$('budget-error').textContent='';show('budget-dialog',$('budget-open'));}catch(error){fail(error);}finally{busy=false;controls();}});
/** Persist/replay one explicit envelope through the shared receipt writer. A
 * lost response freezes the exact event and inputs until acknowledged. */
on('budget-form','submit',async e=>{e.preventDefault();if(!current)return;const before=current.revision;let budget;if(!pending){const limit=$('budget-limit').value,reserve=$('budget-reserve').value;if(limit===''||reserve===''||!Number.isFinite(Number(limit))||!Number.isFinite(Number(reserve))||Number(limit)<0||Number(reserve)<0){$('budget-error').textContent=tr('Renseignez explicitement deux montants positifs ou nuls.');return;}budget={limit_usd:Number(limit),reserve_per_launch_usd:Number(reserve),estimate_source:$('budget-source').value,reference_date:$('budget-date').value};}await mutate('budget',{budget});if(!pending&&!conflict&&current.revision===before+1)$('budget-dialog').close();else $('budget-error').textContent=$('error').textContent;});
/** Create only the need, then expose the selected graph as a local unsaved
 * plan. Explicit Save persists it separately; no brief adoption or launch. */
on('create-form','submit',async e=>{e.preventDefault();const selected=templateDraft;await mutate('create',{title:$('new-title').value,text:$('new-need').value,...(selected?{method:selected.recommended_method}:{})});if(selected?.work_plan&&current&&!pending&&!conflict&&templateDraft===selected){kind='plan';paint(current);editor.value=JSON.stringify(selected.work_plan,null,2);dirty=true;$('template-draft-status').textContent=tr('Plan du modèle préparé localement : enregistrez-le puis adoptez le brief avant de le vérifier.');$('save-state').textContent=tr('Modifications non enregistrées');controls();}});
on('save','click',save);on('plain-editor','input',()=>{dirty=true;$('save-state').textContent=tr('Modifications non enregistrées');controls();});
on('sessions','change',async()=>{const wanted=$('sessions').value;try{if(wanted)await open(wanted);}finally{$('sessions').value=current?.id??'';}});
on('new','click',()=>{if(!navigable())return;current=null;dialogueUI?.paint(null);templateDraft=null;$('template-draft-status').textContent='';kind='besoin';base='';editor.value='';$('documents').hidden=true;$('method-section').hidden=true;$('create-form').hidden=false;$('new-title').value='';$('new-need').value='';$('sessions').value='';$('title').textContent=tr('Préparer un projet');$('new-title').focus();});
for(const name of ['besoin','brief','plan'])on('tab-'+name,'click',()=>{if(navigable()){kind=name;paint(current);}});
on('advance','click',async()=>{if(dirty){fail(Error(tr('Enregistrez vos modifications avant de poursuivre.')));return;}const action=$('advance').dataset.action;if(['brief','plan'].includes(action)){if(navigable()){kind=action;paint(current);}}else await mutate(action,{sha256:current.documents?.[kind]?.sha256??''});});
on('method-apply','click',()=>{if(!navigable())return;return mutate('method',{method:$('method').value});});
on('refresh','click',()=>open(current.id,kind));
on('download','click',()=>download(editor.value,(current?.id??'draft')+'-'+kind+'.txt'));on('download-new','click',()=>download($('new-need').value,'besoin.txt'));
on('history','click',()=>historyPage(true).catch(error=>{$('history-content').append(document.createTextNode(error.message));}));on('history-more','click',()=>historyPage(false).catch(error=>{$('history-content').append(document.createTextNode(error.message));}));
on('conflict-use','click',async()=>{const value=conflict;conflict=null;pending=null;$('conflict').hidden=true;paint(value);await refreshList();});
on('conflict-keep','click',async()=>{const text=editor.value;current=conflict;conflict=null;pending=null;base=current.documents?.[kind]?.text??'';$('conflict').hidden=true;editor.value=text;dirty=true;await save();});
/** Review creation or the current recorded Work before explicit release. Reads
 * do not authorize execution. Bind both displayed revisions; no fallback to draft
 * tasks when converted. Current instructions are shown even if publicly edited. */
on('conversion-open','click',async()=>{
 if(!navigable()||!current)return;
 $('conversion-organization').hidden=true;
 busy=true;controls();
 try{
  if(current.conversion){
   $('conversion-organization').hidden=true;const review=await api('preparations/release-review?id='+encodeURIComponent(current.id));
   if(review.preparation?.id!==current.id||review.preparation.revision!==current.revision||review.work?.id!==current.conversion.work_id||!Number.isSafeInteger(review.work.revision))throw Error(tr('Préparation modifiée ailleurs. Comparez les versions avant de poursuivre.'));
   const conversion=review.preparation.conversion;
   if(!Array.isArray(conversion.task_ids)||!conversion.task_ids.length||new Set(conversion.task_ids).size!==conversion.task_ids.length||!Array.isArray(review.work.tasks)||conversion.task_ids.some(id=>typeof id!=='string'||review.work.tasks.filter(task=>task.id===id).length!==1))throw Error(tr('Réponse du serveur invalide.'));
   if(review.revision){
    const proposal=review.revision,kinds=['ajoutée','modifiée','conservée','retirée (historique conservé)','à revérifier (dépendance modifiée)'];
    if(!current.plan_ready||proposal.plan_hash!==current.documents?.plan?.sha256||proposal.plan_hash===conversion.plan_hash||!Array.isArray(proposal.changes)||!proposal.changes.length||!Array.isArray(proposal.spec?.tasks)||!proposal.spec.tasks.length||proposal.changes.some(change=>typeof change.id!=='string'||!change.id||typeof change.title!=='string'||!kinds.includes(change.kind))||new Set(proposal.changes.map(change=>change.id)).size!==proposal.changes.length)throw Error(tr('Réponse du serveur invalide.'));
    conversionReview={action:'revise-missions',id:current.id,revision:current.revision,workRevision:review.work.revision,hash:proposal.plan_hash};
    $('conversion-target').textContent=review.work.title+' · '+tr('révision ')+review.work.revision;
    $('conversion-explanation').textContent=tr('Appliquer le nouveau plan validé. Les tâches modifiées et leurs dépendants seront à revérifier ; les tâches retirées et les tentatives restent dans l’historique. La mission sera mise en pause, les missions seront verrouillées et une nouvelle autorisation de départ sera nécessaire. Aucun agent ne démarre.');
    const oldTasks=conversion.spec?.tasks;if(!Array.isArray(oldTasks)||proposal.spec.tasks.some(task=>typeof task.id!=='string'||typeof task.title!=='string'||!proposal.changes.some(change=>change.id===task.id&&change.kind!==kinds[3])))throw Error(tr('Réponse du serveur invalide.'));
    $('conversion-missions').replaceChildren(...proposal.changes.map(change=>{const li=document.createElement('li');li.dataset.change=change.kind;li.append(document.createTextNode(change.title+' — '+tr(change.kind)));for(const [label,tasks] of [['Contrat précédent',oldTasks],['Contrat courant dans le travail',review.work.tasks.map(task=>{const recorded=[...review.work.plans].reverse().find(plan=>plan.source===current.id&&plan.response_hash===conversion.plan_hash);const id=Object.entries(recorded?.task_map??{}).find(([,mapped])=>mapped===task.id)?.[0];return {...task,id:id??task.id};})],['Nouveau contrat',proposal.spec.tasks]]){const task=tasks.find(task=>task.id===change.id);if(task){const details=document.createElement('details'),summary=document.createElement('summary'),content=document.createElement('pre');summary.textContent=tr(label);content.textContent=JSON.stringify(task,null,2);details.append(summary,content);li.append(details);}}return li;}));
    $('conversion-confirm').textContent=tr('Appliquer la révision et mettre en pause');
   }else{
    if(current.plan_ready&&current.documents?.plan?.sha256!==conversion.plan_hash)throw Error(tr('Réponse du serveur invalide.'));
   conversionReview={action:'release-plan',id:current.id,revision:current.revision,workRevision:review.work.revision,hash:conversion.plan_hash};
   $('conversion-target').textContent=review.work.title+' · '+tr('révision ')+review.work.revision;
   $('conversion-explanation').textContent=tr('Autoriser seulement les missions issues de cette préparation. Les autres tâches et verrous sont conservés. Le brouillon n’est pas appliqué. Un conducteur déjà configuré pourra démarrer les agents selon ses droits et budgets.');
   $('conversion-missions').replaceChildren(...review.work.tasks.filter(task=>conversion.task_ids.includes(task.id)).map(task=>{const li=document.createElement('li'),details=document.createElement('details'),summary=document.createElement('summary'),content=document.createElement('pre');li.append(document.createTextNode(task.title));summary.textContent=tr('Contrat courant de la tâche');content.textContent=JSON.stringify(task,null,2);details.append(summary,content);li.append(details);return li;}));
   $('conversion-confirm').textContent=tr('Autoriser le démarrage des missions');
   }
  }else{
   if(!current.plan_ready)return;
   const plan=JSON.parse(current.documents.plan.text);let workRevision=0,target=current.title;
   if(current.work_id){const review=await api('preparations/release-review?id='+encodeURIComponent(current.id));if(review.preparation?.id!==current.id||review.preparation.revision!==current.revision||review.work?.id!==current.work_id||!Number.isSafeInteger(review.work.revision)||review.work.revision<1)throw Error(tr('Préparation modifiée ailleurs. Comparez les versions avant de poursuivre.'));workRevision=review.work.revision;target=review.work.title+' · '+tr('révision ')+workRevision;}
   conversionReview={action:'create-missions',id:current.id,revision:current.revision,workRevision,hash:current.documents.plan.sha256};
   $('conversion-target').textContent=target;$('conversion-explanation').textContent=tr(current.work_id?'Ajouter les missions verrouillées au travail affiché. Ses tâches et historiques sont conservés. Aucun agent ne démarre ; une autorisation de départ reste nécessaire.':'Créer un nouveau travail et ses tâches verrouillées. Aucun agent ne démarre ; l’équipe et l’autorisation de départ restent à configurer.');
   $('conversion-missions').replaceChildren(...plan.tasks.map(task=>{const li=document.createElement('li');li.textContent=task.title;return li;}));$('conversion-confirm').textContent=tr('Créer sans démarrer');
  }
  $('conversion-error').textContent='';show('conversion-dialog',$('conversion-open'));
 }catch(error){conversionReview=null;fail(error);}finally{busy=false;controls();}
});
/** Review a pristine conversion before recording a human-review CLI team.
 * Both revisions come from the protected reader; no team is adopted on open. */
on('team-open','click',async()=>{if(!navigable()||!current?.conversion)return;busy=true;controls();try{const review=await api('preparations/release-review?id='+encodeURIComponent(current.id));if(review.preparation?.id!==current.id||review.preparation.revision!==current.revision||review.work?.id!==current.conversion.work_id||!Number.isSafeInteger(review.work.revision))throw Error(tr('Préparation modifiée ailleurs. Comparez les versions avant de poursuivre.'));conversionReview={action:'configure-team',id:current.id,revision:current.revision,workRevision:review.work.revision,hash:current.conversion.plan_hash};$('conversion-organization').hidden=false;$('conversion-target').textContent=review.work.title+' · '+tr('révision ')+review.work.revision;$('conversion-explanation').textContent=tr('Enregistrer une équipe en revue humaine sans libérer les tâches. Le départ exige une autorisation séparée.');$('conversion-missions').replaceChildren(...review.work.tasks.map(t=>{const li=document.createElement('li');li.textContent=t.title;return li;}));$('organization-tasks').min=String(review.work.tasks.length);$('conversion-confirm').textContent=tr('Enregistrer l’équipe sans démarrer');$('conversion-error').textContent='';show('conversion-dialog',$('team-open'));}catch(error){conversionReview=null;fail(error);}finally{busy=false;controls();}});
/** Review both target and preparation revisions before an append or revision;
 * A revision commits held tasks and pause, never a new start authorization. Never infer
 * a current target revision from the draft. Submit/replay the exact reviewed
 * intention. A release acknowledges authority,
 * never current execution or acceptance; uncertain publication keeps its event. */
on('conversion-confirm','click',async()=>{
 const reviewed=conversionReview;if(!reviewed||current?.id!==reviewed.id)return;
 let organization;try{if(reviewed.action==='configure-team'&&!pending)organization=teamSelection();}catch(error){$('conversion-error').textContent=error.message;return;}
 await mutate(reviewed.action,{expected_revision:reviewed.revision,expected_work_revision:reviewed.workRevision,sha256:reviewed.hash,...(organization?{organization}:{})});
 const recorded=reviewed.action==='release-plan'?current?.conversion?.released_at:reviewed.action==='configure-team'?current?.conversion?.organization_sha256:reviewed.action==='revise-missions'?(current?.conversion?.plan_hash===reviewed.hash):current?.conversion;
 if(!pending&&recorded){if(reviewed.action==='revise-missions')$('conversion-status').textContent=tr('Révision enregistrée. Mission mise en pause ; une nouvelle autorisation de départ reste nécessaire.');$('conversion-dialog').close();if(reviewed.action==='configure-team')$('conversion-open').focus();}else $('conversion-error').textContent=$('error').textContent;
});

on('help','click',()=>show('help-dialog'));
on('theme','click',()=>{const theme=document.documentElement.dataset.theme==='sombre'?'etat':'sombre';document.documentElement.dataset.theme=theme;try{localStorage.setItem('swarm-theme',theme);}catch{/* Presentation preference only; no saved policy changes. */}});
for(const dialog of document.querySelectorAll('dialog')){dialog.addEventListener('close',()=>{const opener=openers.get(dialog);openers.delete(dialog);if(document.activeElement===document.body||document.activeElement===dialog||dialog.contains(document.activeElement))opener?.focus();});for(const button of dialog.querySelectorAll('[data-close]'))button.addEventListener('click',()=>dialog.close());}
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
