const tr = source => globalThis.SwarmI18n?.t(source) ?? source;
const roleNames = {
 preparation:'Préparation',planner:'Planificateur',subplanner:'Sous-planificateur',worker:'Exécutant',reviewer:'Vérificateur'
};
export async function showProjectProfile(api) {
 const status=document.getElementById('project-profile-status');
 const button=document.getElementById('project-profile-open');
 const dialog=document.getElementById('project-profile-dialog');
 const list=document.getElementById('project-profile-roles');
 const settings=document.getElementById('project-profile-settings');
 const refresh=document.getElementById('project-profile-refresh');
 async function load() {
  status.textContent=tr('Vérification des consignes du projet…');
  button.disabled=true;list.replaceChildren();settings.textContent='';
  try {
   const profile=await api('project-profile');
   status.textContent=profile.enabled?tr('Profil du projet :')+' '+profile.name:tr('Aucun profil de projet configuré.');
   for(const role of profile.roles) {
    const section=document.createElement('details');
    const title=document.createElement('summary');title.textContent=tr(roleNames[role.role]??role.role);
    const receipt=document.createElement('p');receipt.textContent=tr('Empreinte du contexte :')+' '+role.sha256;
    const files=document.createElement('ul');
    for(const source of role.sources) {
     const item=document.createElement('li');item.textContent=source.path+' · '+source.bytes+' '+tr('octets')+' · SHA-256 '+source.sha256;files.append(item);
    }
    section.append(title,files,receipt);list.append(section);
   }
   settings.textContent=tr('Configuration Claude détectée, pas transmise aux planificateurs :')+' '+(profile.claude_settings_detected.join(', ')||tr('Aucune configuration détectée.'));
   button.disabled=false;
  } catch(error) {status.textContent=tr('Profil du projet indisponible :')+' '+error.message;}
 }
 button.addEventListener('click',()=>dialog.showModal());
 document.getElementById('project-profile-close').addEventListener('click',()=>dialog.close());
 dialog.addEventListener('close',()=>button.focus());
 refresh.addEventListener('click',load);
 await load();
}
