'use strict';
/** Explicit storage/version diagnostic; no polling, execution or implicit policy. */
(()=>{
 /** Mount a protected read independent of Admin bootstrap. Validate the whole
  * response before adopting rows, keep previous observations on error, render
  * only text. Unsafe numeric measurements are labelled unavailable exactly;
  * server classifications remain authoritative observations, never permission. */
 function mount(host,transport,{translate:tr=x=>x}={}){
  const el=(tag,text)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=tr(text);return e};
  const fold=el('details'),title=el('summary','Diagnostic du stockage'),help=el('p','Ce diagnostic observe le stockage et la version. Il ne valide ni SQLite, ni les fournisseurs, ni les droits de lancement.'),read=el('button','Lire le diagnostic'),settings=el('a','Régler les seuils dans l’Administration'),status=el('p','Lecture à la demande.'),rows=el('div');read.type='button';read.dataset.runtimeHealthRead='';fold.dataset.runtimeHealth='';status.dataset.runtimeHealthStatus='';status.className='notice info';status.setAttribute('role','status');const url=new URL(location.href);url.searchParams.set('view','admin');url.hash='native-settings-title';settings.href=url.pathname+url.search+url.hash;const toolbar=el('div');toolbar.className='toolbar';toolbar.append(read,settings);fold.append(title,help,toolbar,status,rows);host.append(fold);let busy=false;
  /** One user-requested read, no retry or deadline fallback. Retain existing
   * rows if source/auth/shape fails; a new successful read replaces them. */
  async function load(){if(busy)return;busy=true;read.disabled=true;status.className='notice info';status.textContent=tr('Chargement…');try{
   const v=await transport.read();
   if(!v||!['ready','warning','blocked'].includes(v.state)||typeof v.launch_allowed!=='boolean'||!['observed_at','message','next_step'].every(k=>typeof v[k]==='string')||!Array.isArray(v.volumes)||!v.volumes.every(row=>row&&['kind','path','state'].every(k=>typeof row[k]==='string')&&['ready','warning','blocked','unknown'].includes(row.state)&&['available_bytes','available_inodes'].every(k=>typeof row[k]==='number'&&Number.isInteger(row[k])&&row[k]>=0))||!Array.isArray(v.observation_errors)||!v.observation_errors.every(c=>typeof c==='string'&&/^[a-z][a-z0-9_.-]*$/.test(c))||!v.version?.binary||typeof v.version.binary.version!=='string')throw {code:'runtime-health-view-invalid'};
   const fragment=document.createDocumentFragment(),at=el('p');at.textContent=tr('Observation')+' : '+v.observed_at;fragment.append(at);
   const labels={ready:'Stockage disponible',warning:'Stockage à surveiller',blocked:'Stockage bloqué',unknown:'Mesure indisponible'};
   for(const row of v.volumes){const section=el('section'),heading=el('h4'),dl=el('dl');heading.textContent=row.kind+' · '+tr(labels[row.state]);section.append(heading);for(const [label,value]of [['Chemin observé',row.path],['Octets disponibles',row.available_bytes],['Inodes disponibles',row.available_inodes]]){const dd=el('dd');dd.textContent=typeof value==='number'&&row.state==='unknown'?tr('Mesure indisponible'):typeof value==='number'&&!Number.isSafeInteger(value)?tr('Mesure non représentable exactement dans le navigateur.'):String(value);dl.append(el('dt',label),dd)}section.append(dl);fragment.append(section)}
   const version=el('p');version.textContent=tr('Version')+' : '+v.version.binary.version;fragment.append(version);if(v.observation_errors.length){const warning=el('p','Certaines observations sont indisponibles.');warning.className='notice alert';warning.append(document.createTextNode(' '+v.observation_errors.join(' · ')));fragment.append(warning)}
   if(!fold.isConnected)return;rows.replaceChildren(fragment);status.className=v.state==='blocked'?'notice alert':v.state==='ready'?'notice success':'notice info';status.textContent=tr(labels[v.state]);status.dataset.runtimeHealthResult='';
  }catch(error){if(!fold.isConnected)return;status.className='notice alert';const code=typeof error?.code==='string'&&/^[a-z][a-z0-9_.-]*$/.test(error.code)?error.code:'runtime-health-read-failed';status.textContent=tr('Diagnostic indisponible. Les observations précédentes sont conservées ; relancez la lecture.')+' ('+code+')';status.dataset.runtimeHealthResult=''}finally{busy=false;read.disabled=false}}
  read.addEventListener('click',load);return Object.freeze({read:load});
 }
 globalThis.SwarmRuntimeHealth=Object.freeze({mount});
})();
