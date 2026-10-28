'use strict';
/** Read the calculated engine resume into the existing resume-text surface.
 * No fabricated snapshot/visit comparison, decision, launch, poll or retry.
 * Unsupported calculated views keep their refusal; a separate explicit gesture
 * can read recorded fields without claiming recalculated validation. */
(()=>{
 const $=id=>document.getElementById(id),tr=s=>globalThis.SwarmI18n?.t(s)??s;
 /** Fixed protected read with original IO/codec/HTTP causes retained on errors.
  * Display callers use safe codes only, never raw response or credential text. */
 async function read(id,language,recorded=false){let response,value;try{response=await fetch((recorded?'/api/v1/work-recorded-resume?':'/api/v1/work-resume?')+new URLSearchParams({work:id,lang:language}),{credentials:'same-origin',headers:{Accept:'application/json'}})}catch(cause){throw new Error('work-resume-response-unavailable',{cause})}try{value=await response.json()}catch(cause){const error=new Error('work-resume-response-invalid',{cause});error.httpStatus=response.status;throw error}if(!response.ok){const raw=value?.failure?.code||value?.error,error=new Error(typeof raw==='string'&&/^[a-z][a-z0-9_.-]*$/.test(raw)?raw:'work-resume-read-refused');error.response=value;error.httpStatus=response.status;throw error}if(value?.observation!==(recorded?'recorded':'calculated')||value?.schema_version!==1||value.work_id!==id||value.language!==language||!Number.isSafeInteger(value.revision)||value.revision<1||typeof value.markdown!=='string'||!value.markdown.trim())throw Error('work-resume-view-invalid');return value}
 /** Bind the original section/pre. A generation invalidates responses after
  * mission selection or refresh; hidden pages are cleared without implicit reads.
  * The existing scrollable text has explicit keyboard focus and an accessible label.
  * The read reports current data, not previous-visit differences or acceptance.
  * Failed/empty/malformed responses clear old text and leave explicit recovery. */
 function mount(){const section=$('resume'),text=$('resume-text'),status=document.createElement('p');text.tabIndex=0;text.setAttribute('aria-label',tr('Reprendre le travail'));status.id='native-resume-status';status.role='status';status.className='notice info';text.before(status);const saved=document.createElement('button');saved.id='native-resume-recorded';saved.type='button';saved.textContent=tr('Lire uniquement les données enregistrées');saved.hidden=true;text.before(saved);let selected='',generation=0;
  section.querySelector('.section-heading p').textContent=tr('Résumé actuel du moteur. Aucune comparaison de visites ni décision n’est enregistrée ici.');
  /** Clear old mission content before IO or when no explicit mission is selected. */
  function clear(message){saved.hidden=true;text.textContent='';section.removeAttribute('aria-busy');status.className='notice info';status.textContent=message}
  /** One explicit visible-page read. A superseded response has no display effect;
   * failure is not an empty success, and no action or automatic retry follows. */
  async function refresh(recorded=false){const ticket=++generation,id=selected;if(!id){clear(tr('Aucune mission sélectionnée.'));return false}clear(tr('Lecture de la mission…'));section.setAttribute('aria-busy','true');try{const language=globalThis.SwarmI18n?.language==='en'?'en':'fr',value=await read(id,language,recorded);if(ticket!==generation)return false;text.textContent=value.markdown;status.className=recorded?'notice attention':'notice info';status.textContent=tr(recorded?'Données enregistrées uniquement — validation non recalculée.':'Résumé calculé depuis les données enregistrées.')+' · '+id+' · '+tr('Révision')+' '+value.revision;return true}catch(cause){if(ticket!==generation)return false;const code=/^[a-z][a-z0-9_.-]*$/.test(cause.message)?cause.message:'work-resume-read-refused';clear(tr('Lecture impossible. Actualisez pour reprendre.')+' ('+code+')');status.className='notice alert';if(!recorded&&cause.httpStatus===501){saved.hidden=false;status.textContent=tr('Résumé calculé indisponible pour ce contrat. La lecture des données enregistrées ne recalcule pas la validation.')+' ('+code+')'}return false}finally{if(ticket===generation)section.removeAttribute('aria-busy')}}
  saved.addEventListener('click',()=>refresh(true));
  clear(tr('Aucune mission sélectionnée.'));
  return {refresh,setWork(id){selected=id;++generation;clear(tr(id?'Résumé disponible en ouvrant cette page.':'Aucune mission sélectionnée.'));if(!section.hidden)refresh()}};
 }
 globalThis.SwarmNativeResume=Object.freeze({mount});
})();
