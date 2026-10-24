'use strict';
/** Explicit historical agent-log reader in the canonical Journaux section.
 * No polling, execution, provenance inference or mutation. Text is never HTML. */
globalThis.SwarmNativeLogs=(()=>{
 const $=id=>document.getElementById(id),tr=s=>globalThis.SwarmI18n?.t(s)??s;
 /** Keep original transport/JSON/HTTP causes; publish only stable diagnostic codes. */
 function failure(code,cause){const e=new Error(code,{cause});e.code=code;return e}
 /** Same-origin protected read; no retry, local deadline or hidden page ceiling. */
 async function read(path){let response;try{response=await fetch(path,{credentials:'same-origin',headers:{Accept:'application/json'}})}catch(cause){throw failure('agent-logs-unavailable',cause)}let value;try{value=await response.json()}catch(cause){throw failure('agent-logs-response-invalid',cause)}if(!response.ok){const code=value?.failure?.code||value?.error;throw failure(typeof code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(code)?code:'agent-logs-read-refused',{status:response.status,response:value})}return value}
 /** Mount existing controls. Selected Work is data, not a new Root/authority.
  * Opening the page reads agent choices only; an explicit choice reads latest
  * historical rows. The CLI's cursor semantics remain server-owned. Filters
  * cover the loaded page only. A generation drops replies from an old selection.
  * Errors clear content/export; successful empty results remain distinguishable.
  * Export contains exactly visible rows as NDJSON, no extra reads or redaction.
  * Typed transcript metadata is a separate explicit read, never raw capture.
  * Returns setWork/refresh; neither can launch a provider or change a Work. */
 function mount(){let work='',agent='',generation=0,rows=[],visibleRows=[];
  const section=$('logs'),source=$('log-agent'),state=$('journal-console-state'),lines=$('log-lines');
  let transcriptGeneration=0;
  const details=document.createElement('details');details.className='cockpit-accordion';details.id='native-agent-transcript';
  const summary=document.createElement('summary');summary.textContent=tr('Transcript typé — contenu masqué');
  const body=document.createElement('div');body.className='accordion-body';
  const transcriptRead=document.createElement('button');transcriptRead.type='button';transcriptRead.id='native-transcript-read';transcriptRead.textContent=tr('Lire le transcript typé');
  const transcriptState=document.createElement('p');transcriptState.id='native-transcript-status';transcriptState.className='notice info';transcriptState.setAttribute('role','status');
  const transcriptText=document.createElement('pre');transcriptText.id='native-transcript-text';transcriptText.tabIndex=0;transcriptText.setAttribute('aria-label',tr('Métadonnées du transcript enregistré'));
  const disclaimer=document.createElement('p');disclaimer.textContent=tr('Ces événements enregistrés ne prouvent ni l’identité ni l’activité actuelle.');
  body.append(disclaimer,transcriptRead,transcriptState,transcriptText);details.append(summary,body);$('journal-console-mode').after(details);
  /** A source/Work change clears this independent observation immediately.
   * No implicit read, capture export, retry, execution or identity inference. */
  function resetTranscript(){++transcriptGeneration;details.removeAttribute('aria-busy');details.open=false;transcriptRead.disabled=!work||!agent;transcriptText.textContent='';transcriptState.className='notice info';transcriptState.textContent=tr('Lecture explicite des métadonnées ; aucun contenu des messages ou outils.');}
  /** Validate and display only the producer's typed metadata, never its raw
   * capture, prompts, replies or tool input/output. Unknown shapes are errors,
   * not empty success. Numeric checks protect exact JS representation only. */
  function transcriptContent(v,chosenWork,chosenAgent){
   const count=n=>Number.isSafeInteger(n)&&n>=0,map=o=>o&&typeof o==='object'&&!Array.isArray(o)&&Object.values(o).every(count);
   if(v?.schema_version!==1||v.observation!=='recorded'||v.work_id!==chosenWork||v.agent_id!==chosenAgent||typeof v.recorded_status!=='string')throw failure('agent-transcript-view-invalid');
   if(v.state==='unavailable'){if(!['receipt-absent','capture-not-retained'].includes(v.reason)||v.transcript!==null||v.capture_truncated!==null||v.capture_invalid_utf8!==null||v.output_complete!==null)throw failure('agent-transcript-view-invalid');return{status:tr('Aucun transcript retenu pour cet agent.'),text:'',partial:false};}
   const t=v.transcript,kinds=new Set(['system_prompt','assistant_message','model_call','tool','tool_result','tool_rejected','note']);
   if(v.state!=='available'||v.reason!==null||!['capture_truncated','capture_invalid_utf8','output_complete'].every(k=>typeof v[k]==='boolean')||!t||!['claude','codex','unknown'].includes(t.format)||typeof t.truncated!=='boolean'||!Array.isArray(t.entries)||!map(t.counts)||!map(t.unknown)||!t.entries.every(e=>e&&kinds.has(e.kind)&&['tool','id','label'].every(k=>!Object.hasOwn(e,k)||typeof e[k]==='string')&&(!Object.hasOwn(e,'error')||typeof e.error==='boolean')&&(!Object.hasOwn(e,'bytes')||count(e.bytes))&&(!Object.hasOwn(e,'tokens')||map(e.tokens))))throw failure('agent-transcript-view-invalid');
   const labels={system_prompt:tr('Début de session'),assistant_message:tr('Message de l’assistant'),model_call:tr('Appel de modèle'),tool:tr('Outil'),tool_result:tr('Résultat d’outil'),tool_rejected:tr('Outil refusé'),note:tr('Note')};
   const text=t.entries.map((e,i)=>{const parts=[String(i+1),labels[e.kind]];for(const k of ['tool','id','label'])if(e[k])parts.push(e[k]);if(e.error)parts.push(tr('Erreur déclarée'));if(e.bytes!==undefined)parts.push(e.bytes+' '+tr('octets masqués'));if(e.tokens)for(const [name,n]of Object.entries(e.tokens))parts.push(name+'='+n);return parts.join(' · ')}).join('\n');
   const partial=v.capture_truncated||v.capture_invalid_utf8||!v.output_complete||t.truncated;
   return{status:(partial?tr('Capture partielle ou altérée ; événements incomplets.'):tr(t.entries.length?'Transcript enregistré — métadonnées uniquement.':'Aucun événement typé dans la capture enregistrée.'))+' '+tr('Format observé')+' : '+t.format+' · '+tr('Événements')+' : '+t.entries.length,text,partial};
  }
  /** Observe one selected source on an explicit click. Preserve typed read
   * causes, visible absent/partial/error states and stale-response isolation. */
  async function loadTranscript(){if(!work||!agent)return;const ticket=++transcriptGeneration,chosenWork=work,chosenAgent=agent;transcriptText.textContent='';transcriptState.className='notice info';transcriptState.textContent=tr('Lecture du transcript…');transcriptRead.disabled=true;details.setAttribute('aria-busy','true');try{const value=await read('/api/v1/agent-transcript?'+new URLSearchParams({work:chosenWork,agent:chosenAgent}));if(ticket!==transcriptGeneration)return;const view=transcriptContent(value,chosenWork,chosenAgent);transcriptText.textContent=view.text;transcriptState.textContent=view.status;transcriptState.className=view.partial?'notice attention':'notice info';}catch(cause){if(ticket!==transcriptGeneration)return;transcriptState.className='notice alert';transcriptState.textContent=tr('Lecture impossible. Actualisez pour reprendre.')+' ('+(cause.code||'agent-transcript-read-refused')+')';}finally{if(ticket===transcriptGeneration){details.removeAttribute('aria-busy');transcriptRead.disabled=false;}}}
  transcriptRead.addEventListener('click',loadTranscript);resetTranscript();
  lines.tabIndex=0;lines.setAttribute('aria-label',tr('Journaux enregistrés de l’agent'));
  $('log-prev').textContent=tr('Depuis le début');$('log-next').textContent=tr('Page suivante');$('log-form').querySelector('button').textContent=tr('Filtrer cette page');$('log-form').querySelector('button').disabled=false;
  $('journal-console-mode').textContent=tr('Historique enregistré · filtres sur la page chargée · aucun suivi automatique.');
  for(const id of ['log-search','log-kind'])$(id).disabled=false;
  /** Publish only validated rows as literal text and derive local filter/export
   * state. No regex evaluation or successful placeholder for a failed read. */
  function render(){const text=$('log-search').value,kind=$('log-kind').value;visibleRows=rows.filter(r=>(!text||r.message.includes(text))&&(!kind||r.kind===kind));lines.textContent=visibleRows.map(r=>'#'+r.seq+' '+r.at+' ['+r.kind+'] '+r.message).join('\n')||tr('Aucune ligne correspondante.');$('log-export').disabled=visibleRows.length===0;$('log-position').textContent=tr('Lignes affichées')+' : '+visibleRows.length+' / '+rows.length;}
  /** Clear previous bytes before any pending/failing read or Work switch. */
  function clear(message){rows=[];visibleRows=[];lines.textContent='';$('log-position').textContent='';for(const id of ['log-prev','log-next','log-export'])$(id).disabled=true;state.className='notice info';state.textContent=message;}
  /** Read one exact cursor and reject malformed, cross-agent or reordered rows.
   * A failed response never advances the cursor or keeps stale export content. */
  async function load(after){const ticket=++generation,chosenWork=work,chosenAgent=agent;clear(tr('Lecture des journaux…'));section.setAttribute('aria-busy','true');try{const v=await read('/api/v1/agent-logs?'+new URLSearchParams({work:chosenWork,agent:chosenAgent,after:String(after)}));if(ticket!==generation)return;
    let previous=after>0?after:0;if(v?.schema_version!==1||v.work_id!==chosenWork||v.agent_id!==chosenAgent||v.after!==after||!Array.isArray(v.rows)||!v.rows.every(r=>{const valid=r&&r.agent_id===chosenAgent&&Number.isSafeInteger(r.seq)&&r.seq>previous&&['at','kind','message'].every(k=>typeof r[k]==='string');if(valid)previous=r.seq;return valid}))throw failure('agent-logs-view-invalid');
    rows=v.rows;render();$('log-prev').disabled=false;$('log-next').disabled=!rows.length||rows.at(-1).seq===Number.MAX_SAFE_INTEGER;state.textContent=tr(rows.length?'Page historique reçue.':'Aucune ligne enregistrée pour cette page.');
   }catch(cause){if(ticket!==generation)return;clear(tr('Lecture impossible. Actualisez pour reprendre.')+' ('+(cause.code||'agent-logs-read-refused')+')');state.className='notice alert';$('log-prev').disabled=false;
   }finally{if(ticket===generation)section.removeAttribute('aria-busy')}}
  /** Observe narrow agent choices for the selected Work without reading a log.
   * Hidden-page selection clears immediately and makes no implicit request. */
  async function refresh(){const ticket=++generation;agent='';resetTranscript();source.replaceChildren();source.disabled=true;clear(tr(work?'Choisissez un agent pour lire son historique.':'Aucune mission sélectionnée.'));section.removeAttribute('aria-busy');if(!work||section.hidden)return;section.setAttribute('aria-busy','true');try{const v=await read('/api/v1/cockpit-read?'+new URLSearchParams({work}));if(ticket!==generation)return;if(v?.schema_version!==1||v.work_id!==work||!Array.isArray(v.agents)||!v.agents.every(a=>a&&a.work_id===work&&typeof a.id==='string'&&a.id&&typeof a.status==='string')||new Set(v.agents.map(a=>a.id)).size!==v.agents.length)throw failure('agent-logs-agents-invalid');const placeholder=document.createElement('option');placeholder.value='';placeholder.textContent=tr('Choisissez un agent');source.append(placeholder);for(const a of v.agents){const o=document.createElement('option');o.value=a.id;o.textContent=a.id+' · '+a.status;source.append(o)}source.disabled=v.agents.length===0;if(!v.agents.length)state.textContent=tr('Aucun agent enregistré pour cette mission.');
   }catch(cause){if(ticket!==generation)return;clear(tr('Lecture impossible. Actualisez pour reprendre.')+' ('+(cause.code||'agent-logs-read-refused')+')');state.className='notice alert'}finally{if(ticket===generation)section.removeAttribute('aria-busy')}}
  source.addEventListener('change',()=>{agent=source.value;resetTranscript();if(agent)load(0);else{++generation;section.removeAttribute('aria-busy');clear(tr('Choisissez un agent pour lire son historique.'))}});
  $('log-prev').addEventListener('click',()=>{if(agent)load(-1)});$('log-next').addEventListener('click',()=>{if(agent&&rows.length)load(rows.at(-1).seq)});
  $('log-form').addEventListener('submit',event=>{event.preventDefault();if(rows.length)render()});for(const id of ['log-search','log-kind'])$(id).addEventListener('input',()=>{if(rows.length)render()});
  $('log-export').addEventListener('click',()=>{if(!visibleRows.length)return;const href=URL.createObjectURL(new Blob([visibleRows.map(r=>JSON.stringify(r)).join('\n')+'\n'],{type:'application/x-ndjson'}));try{const a=document.createElement('a');a.href=href;a.download='agent-logs.ndjson';a.click()}finally{URL.revokeObjectURL(href)}});
  return{setWork(id){work=id;refresh()},refresh};
 }
 return{mount};
})();
