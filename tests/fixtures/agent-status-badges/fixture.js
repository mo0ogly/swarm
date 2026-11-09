'use strict';
/** Explicit recorded metadata doubles; only the unmodified product renderer
 * creates cards. Controls change fixture presentation, never a live mission. */
const states=['queued','starting','running','stopping','completed','failed','interrupted','running/unconfirmed','unknown/other-host','unknown'];
const params=new URLSearchParams(location.search),lang=params.get('lang')==='en'?'en':'fr',theme=params.get('theme')==='sombre'?'sombre':'etat';
document.documentElement.dataset.theme=theme;
for(const [id,value] of [['fixture-language',lang],['fixture-theme',theme]]){const picker=document.getElementById(id);picker.value=value;picker.addEventListener('change',()=>{const next=new URL(location.href);next.searchParams.set('lang',document.getElementById('fixture-language').value);next.searchParams.set('theme',document.getElementById('fixture-theme').value);location.assign(next.href)})}
SwarmRustCockpitPanels.render({schema_version:1,work_id:'fixture',budget:null,agents:states.map((status,i)=>({id:'fixture-agent-'+i,work_id:'fixture',task_id:'fixture-task',attempt_id:'fixture-attempt-'+i,status,provider:'inert-fixture-no-provider',role:'worker',parent:'',previous:'',started:'',ended:''}))});
fetch('qualification.json').then(r=>{if(!r.ok)throw Error('qualification-unavailable');return r.json()}).then(v=>{document.getElementById('fixture-version').textContent=v.version+' · '+v.binary_sha256});
