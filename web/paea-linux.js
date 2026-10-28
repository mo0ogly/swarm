'use strict';
/** Read-only native server prerequisite observation. No launch, policy write,
 * browser filesystem access or protected admission is exposed by this page.
 */
(() => {
  const $=id=>document.getElementById(id),t=text=>globalThis.SwarmI18n?.t(text)||text;
  const flags=['launch_verified','separate_host_uid_verified','confinement_verified','human_verified'];
  let busy=false,raw=null;
  /** Closed object check rejects missing/unknown fields rather than empty success. */
  const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&Object.keys(v).length===keys.length&&keys.every(k=>Object.hasOwn(v,k));
  /** Linux UID/count bounds describe representation only, never allocation policy. */
  const uint=(n,max=4294967294)=>Number.isInteger(n)&&n>=0&&n<=max;
  /** Validate a source result and safe failure fields without interpreting IO text. */
  function observation(value,check){
    if(!value||typeof value!=='object')return false;
    if(value.status==='observed')return exact(value,['status','value'])&&check(value.value);
    if(value.status==='invalid')return exact(value,['status','code'])&&typeof value.code==='string'&&/^[a-z0-9-]+$/.test(value.code);
    if(value.status==='unavailable')return exact(value,['status','failure'])&&exact(value.failure,['code','errno'])&&typeof value.failure.code==='string'&&/^[a-z0-9-]+$/.test(value.failure.code)&&(value.failure.errno===null||(uint(value.failure.errno,2147483647)&&value.failure.errno>0));
    return false;
  }
  /** Validate version2 and its explicit non-authority claims before any rendering.
   * Cross-check missing markers against source statuses; an omitted prerequisite,
   * unknown status, duplicate marker or protection=true invalidates the report.
   */
  function validate(value){
    const identity=v=>exact(v,['euid','egid','groups'])&&uint(v.euid)&&uint(v.egid)&&Array.isArray(v.groups)&&v.groups.every(n=>uint(n));
    const map=v=>Array.isArray(v)&&v.length>0&&v.every(r=>exact(r,['inside','outside','count'])&&uint(r.inside)&&uint(r.outside)&&uint(r.count,4294967295)&&r.count>0&&r.inside+r.count<=4294967295&&r.outside+r.count<=4294967295);
    const ranges=v=>Array.isArray(v)&&v.every(r=>exact(r,['start','count'])&&uint(r.start)&&r.start>0&&uint(r.count,4294967295)&&r.count>0&&r.start+r.count<=4294967295)&&v.every((r,i)=>v.slice(i+1).every(s=>r.start+r.count<=s.start||s.start+s.count<=r.start));
    const helper=v=>exact(v,['path','owner','mode','metadata_safe'])&&typeof v.path==='string'&&v.path.startsWith('/')&&uint(v.owner)&&uint(v.mode,4095)&&typeof v.metadata_safe==='boolean'&&(!v.metadata_safe||(v.owner===0&&(v.mode&0o4000)!==0&&(v.mode&0o111)!==0&&(v.mode&0o022)===0));
    if(!exact(value,['schema_version','kind','scope','identity','local_username','maps','assigned_ranges','mapping_helpers','missing_prerequisites','read_only',...flags])||value.schema_version!==2||value.kind!=='paea-linux-prerequisite-observation'||value.scope!=='current-process-namespace'||value.read_only!==true||flags.some(k=>value[k]!==false)||!observation(value.identity,identity)||!observation(value.local_username,n=>typeof n==='string'&&n.length>0)||!exact(value.maps,['uid_map','gid_map'])||!Object.values(value.maps).every(v=>observation(v,map))||!exact(value.assigned_ranges,['subuid','subgid'])||!Object.values(value.assigned_ranges).every(v=>observation(v,ranges))||!exact(value.mapping_helpers,['newuidmap','newgidmap'])||!Object.values(value.mapping_helpers).every(v=>observation(v,helper)))throw Error('invalid-report');
    const missing=[];
    if(value.identity.status!=='observed')missing.push('identity:groups');
    if(value.local_username.status!=='observed')missing.push('identity:local-passwd');
    for(const [name,h]of Object.entries(value.mapping_helpers))if(h.status!=='observed'||!h.value.metadata_safe)missing.push('helper:'+name);
    for(const [name,r]of Object.entries(value.assigned_ranges))if(r.status!=='observed'||!r.value.length)missing.push('range:'+name);
    for(const [name,m]of Object.entries(value.maps))if(m.status!=='observed')missing.push('map:'+name);
    if(!Array.isArray(value.missing_prerequisites)||value.missing_prerequisites.length!==missing.length||new Set(value.missing_prerequisites).size!==missing.length||value.missing_prerequisites.some(k=>!missing.includes(k)))throw Error('invalid-missing-prerequisites');
  }
  /** Drop prior report/export before each request and on every failure. */
  function clear(){raw=null;$('linux-download').disabled=true;$('linux-result').hidden=true;$('linux-summary').textContent='';$('linux-observations').replaceChildren();$('linux-missing').replaceChildren();$('linux-json').textContent=''}
  /** Keep essential loading/failure state visible, with no success-by-color claim. */
  function status(kind,message){$('linux-status').dataset.state=kind;$('linux-status').textContent=t(message)}
  /** Render a safe status category only; raw paths/source text stay in disclosure. */
  function state(v,helper=false){return t(v.status==='invalid'?'Format invalide':v.status==='unavailable'?'Indisponible':helper&&!v.value.metadata_safe?'Métadonnées insuffisantes':Array.isArray(v.value)&&!v.value.length?'Aucune plage attribuée':'Observé')}
  /** One read-only protected GET; strict UTF-8 preserves valid body bytes and rejects
   * invalid encoding/BOM rather than replacement/removal; shape errors clear stale results.
   * No retry/deadline, activation or fallback; operator may request a fresh read.
   * Restore triggering focus only while body retains it, never steal a moved focus.
   */
  async function observe(){
    if(busy)return;const restoreFocus=document.activeElement===$('linux-observe');busy=true;clear();$('linux-observe').disabled=true;status('loading','Lecture des préconditions du serveur…');
    try{
      const response=await fetch('/api/v1/protection/linux-prerequisites',{method:'GET',cache:'no-store',credentials:'same-origin'});
      if(!response.ok||!(response.headers.get('content-type')||'').includes('application/json'))throw Error('response-refused');
      const bytes=await response.arrayBuffer(),text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes),value=JSON.parse(text);validate(value);
      const label=value.identity.status==='observed'?t('UID effective du serveur')+' : '+value.identity.value.euid:t('Identité du serveur indisponible');$('linux-summary').textContent=label;
      for(const [label,v,h]of [['Identité locale',value.identity,false],['Nom local',value.local_username,false],['Mapping UID',value.maps.uid_map,false],['Mapping GID',value.maps.gid_map,false],['Plages UID',value.assigned_ranges.subuid,false],['Plages GID',value.assigned_ranges.subgid,false],['newuidmap',value.mapping_helpers.newuidmap,true],['newgidmap',value.mapping_helpers.newgidmap,true]]){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=t(label);dd.textContent=state(v,h);$('linux-observations').append(dt,dd)}
      const labels={'identity:groups':'Groupes du serveur','identity:local-passwd':'Identité passwd locale','helper:newuidmap':'Helper newuidmap','helper:newgidmap':'Helper newgidmap','range:subuid':'Plages subuid','range:subgid':'Plages subgid','map:uid_map':'Mapping uid_map','map:gid_map':'Mapping gid_map'};
      for(const key of value.missing_prerequisites){const li=document.createElement('li');li.textContent=t(labels[key]);$('linux-missing').append(li)}
      if(!value.missing_prerequisites.length){const li=document.createElement('li');li.textContent=t('Aucune absence de métadonnées détectée ; lancement protégé non vérifié.');$('linux-missing').append(li)}
      raw=text;$('linux-json').textContent=text;$('linux-result').hidden=false;$('linux-download').disabled=false;status('observed','Observations recueillies ; lancement protégé non vérifié.');
    }catch{clear();status('error','Observation indisponible ou invalide. Vérifier la session et les préconditions du serveur, puis reprendre.')}finally{busy=false;$('linux-observe').disabled=false;if(restoreFocus&&document.activeElement===document.body)$('linux-observe').focus({preventScroll:true})}
  }
  /** Download only exact validated server bytes, with no fabricated signature. */
  function download(){if(raw===null||busy)return;const url=URL.createObjectURL(new Blob([raw],{type:'application/json'})),a=document.createElement('a');a.href=url;a.download='paea-linux-prerequisites-v2.json';a.click();URL.revokeObjectURL(url)}
  /** Preserve language/theme when returning to PAEA checks; presentation only. */
  function navigation(){for(const link of document.querySelectorAll('[data-paea-link]')){const url=new URL(link.href);url.searchParams.set('lang',globalThis.SwarmI18n.language);url.searchParams.set('theme',document.documentElement.dataset.theme);link.href=url.pathname+url.search}}
  const theme=new URL(location.href).searchParams.get('theme');if(['etat','sombre'].includes(theme))document.documentElement.dataset.theme=theme;
  $('paea-theme').addEventListener('click',()=>{document.documentElement.dataset.theme=document.documentElement.dataset.theme==='etat'?'sombre':'etat';navigation()});
  $('linux-observe').addEventListener('click',observe);$('linux-download').addEventListener('click',download);
  Promise.resolve(globalThis.SwarmI18n?.ready).then(()=>{navigation();status('empty','Aucune observation demandée.')});
})();
