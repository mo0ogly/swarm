'use strict';
/** Explicit consultation bookmark, separate from task acceptance and identity.
 * The host supplies protected API calls using the saved browser transport policy.
 * No polling/retry, lifecycle action or execution policy is introduced. */
(()=>{
 /** Mount for one selected Work object. Exact wire revision is required before
  * marking. Reads never write; failed writes require explicit readback before
  * another attempt. Detached results are ignored without claiming cancellation.
  * Previous confirmed rows survive failures. Host/UID is not human authentication. */
 function mount(host,work,api,{translate=String}={}){
  const tr=translate,C=globalThis.SwarmRecordedCursor;
  const el=(tag,text,cls)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=tr(text);if(cls)n.className=cls;return n};
  const section=el('details',undefined,'native-task'),read=el('button','Relire la consultation'),mark=el('button','Marquer cette révision comme consultée'),status=el('p','Lecture à la demande.','notice info'),rows=el('dl',undefined,'native-facts');
  section.dataset.sessionVisit='';read.dataset.visitRead='';mark.dataset.visitMark='';status.dataset.visitStatus='';read.type=mark.type='button';status.setAttribute('role','status');mark.disabled=true;
  section.append(el('summary','Consultation de cette mission'),el('p','Ce repère local ne valide aucune tâche. L’opérateur hôte/UID ne prouve pas une identité humaine.'),read,mark,status,rows);host.append(section);
  let busy=false,needsRead=true,current;
  const code=e=>typeof e?.code==='string'&&/^[a-z][a-z0-9-]*$/.test(e.code)?e.code:'visit-response-invalid';
  function selectedRevision(){return C.field(work,'revision',false)}
  /** Require complete wire types and exact integer agreement, including missing
   * bookmark sentinel. A malformed response cannot unlock a subsequent write. */
  function validate(value){
   if(!value||value.schema_version!==1||value.work_id!==work.id||typeof value.exists!=='boolean'||typeof value.work_revision!=='string'||typeof value.exact_visit_revision!=='string'||!value.visit||typeof value.visit.at!=='string'||typeof value.visit.operator!=='string'||!value.visit.operator)throw {code:'visit-response-invalid'};
   const canonical=s=>/^(0|[1-9][0-9]*)$/.test(s)&&BigInt(s)<(1n<<63n);
   if(!canonical(value.work_revision)||!canonical(value.exact_visit_revision)||C.integer(value.visit.revision,value.exact_visit_revision,false)!==value.exact_visit_revision||!value.exists&&(value.exact_visit_revision!=='0'||value.visit.at!==''))throw {code:'visit-response-invalid'};
   return value;
  }
  function render(value){
   const fragment=document.createDocumentFragment();for(const [label,text]of [['Révision actuelle',value.work_revision],['Dernière révision consultée',value.exists?value.exact_visit_revision:tr('Non disponible')],['Date enregistrée',value.exists?value.visit.at:tr('Non disponible')],['Opérateur local',value.visit.operator]]){fragment.append(el('dt',label));const dd=el('dd');dd.textContent=text;fragment.append(dd)}rows.replaceChildren(fragment);
  }
  /** Execute one user gesture. Readback observes current state, not whether an
   * earlier uncertain write was this caller's; no blind retry follows an error. */
  async function act(writing){
   if(busy||writing&&(needsRead||!current))return;busy=true;read.disabled=mark.disabled=true;status.className='notice info';status.textContent=tr('Chargement…');
   try{
    const revision=selectedRevision();let value;
    if(writing){needsRead=true;const result=await api.mark(work.id,{schema_version:1,expected_revision:revision});if(result?.saved!==true)throw {code:'visit-response-invalid'};value=validate(result.state)}else value=validate(await api.show(work.id));
    if(!section.isConnected)return;current=value;render(value);needsRead=false;
    status.textContent=tr(value.work_revision===revision?(writing?'Consultation enregistrée. Aucune tâche n’est validée.':'Consultation relue.'):'La mission a changé. Actualisez les missions avant de marquer une révision.');
   }catch(error){if(!section.isConnected)return;needsRead=true;status.className='notice alert';status.textContent=tr(writing?'La consultation peut déjà être enregistrée. Relisez son état avant toute nouvelle tentative.':'Lecture indisponible. Les observations précédentes sont conservées.')+' ('+code(error)+')'}finally{busy=false;read.disabled=false;let exact=false;try{exact=current?.work_revision===selectedRevision()}catch{}mark.disabled=needsRead||!exact}
  }
  read.addEventListener('click',()=>act(false));mark.addEventListener('click',()=>act(true));
  return Object.freeze({element:section});
 }
 globalThis.SwarmSessionVisit=Object.freeze({mount});
})();
