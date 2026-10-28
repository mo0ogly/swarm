'use strict';
/** Explicit complete lifecycle metadata reader. No archive/restore/delete action,
 * automatic request, polling, retry, provider or accepted-state inference. */
(()=>{
 /** Mount once with an authenticated readonly API hook and translator. Validate
  * complete responses before rendering; preserve previous rows on failure and
  * ignore detached results. Revisions use the shared exact JSON parser metadata.
  * Data is literal text; no path or private trash snapshot is exposed or fetched. */
 function mount(host,api,{translate=String}={}){
  const tr=translate,el=(tag,text,cls)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=tr(text);if(cls)n.className=cls;return n};
  const section=el('details',undefined,'native-task'),read=el('button','Lire l’inventaire des missions'),status=el('p','Lecture à la demande.','notice info'),rows=el('div');section.dataset.lifecycleInventory='';read.dataset.lifecycleInventoryRead='';read.type='button';status.dataset.lifecycleInventoryStatus='';status.setAttribute('role','status');
  section.append(el('summary','Missions actives, archivées et corbeille'),el('p','Métadonnées enregistrées : cette lecture ne vérifie pas les archives et n’autorise aucune restauration ou suppression.'),read,status,rows);host.append(section);let busy=false;
  const labels={active:'Active',archived:'Archivée',trash:'Dans la corbeille'};
  const safeCode=e=>typeof e?.code==='string'&&/^[a-z][a-z0-9-]*$/.test(e.code)?e.code:'lifecycle-inventory-response-invalid';
  read.addEventListener('click',async()=>{
   if(busy)return;busy=true;read.disabled=true;status.className='notice info';status.textContent=tr('Chargement…');
   try{
    const entries=await api.show();if(!section.isConnected)return;
    if(!Array.isArray(entries)||!entries.every(e=>e&&typeof e==='object'&&!Array.isArray(e)&&['work_id','title','state','state_label'].every(k=>typeof e[k]==='string')&&Object.hasOwn(labels,e.state)&&(!Object.hasOwn(e,'changed_at')||typeof e.changed_at==='string')))throw {code:'lifecycle-inventory-response-invalid'};
    const fragment=document.createDocumentFragment();
    for(const e of entries){const revision=SwarmRecordedCursor.field(e,'revision',true),card=el('article',undefined,'native-task'),title=el('h4'),facts=el('dl',undefined,'native-facts');title.textContent=e.title||e.work_id;
     for(const [label,value]of [['Mission',e.work_id],['État enregistré',tr(labels[e.state])],['Révision enregistrée',revision],['Date enregistrée',e.changed_at||tr('Non disponible')]]){facts.append(el('dt',label));const dd=el('dd');dd.textContent=value;facts.append(dd)}card.append(title,facts);fragment.append(card)}
    if(!entries.length)fragment.append(el('p','Aucune mission enregistrée dans cet inventaire.'));rows.replaceChildren(fragment);status.textContent=tr('Inventaire des missions relu.');
   }catch(e){if(!section.isConnected)return;status.className='notice alert';status.textContent=tr('Inventaire indisponible. Les observations précédentes sont conservées ; relancez la lecture.')+' ('+safeCode(e)+')'}finally{busy=false;read.disabled=false}
  });
  return Object.freeze({element:section});
 }
 globalThis.SwarmLifecycleInventory=Object.freeze({mount});
})();
