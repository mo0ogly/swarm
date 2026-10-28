'use strict';
const trRoleModel=s=>globalThis.SwarmI18n?.t(s)??s;
const RoleModels={
 open(scope='',reviewer=false){const p=snapshot.work.planning;if(!p)return;const target=reviewer?p.reviewer:p.scopes.find(s=>s.id===scope);if(!target)return;const selection=target.model_selection,route=reviewer?target.model_route:selection?.route||p.model_route,provider=reviewer?target.provider:selection?.provider||p.provider;
 openModal(trRoleModel('Modèle du responsable ou du vérificateur')+' — '+(reviewer?trRoleModel('Vérificateur indépendant'):scope),trRoleModel('La mission doit être en pause. Le choix change les prochains appels de ce rôle uniquement. Consommations, échecs et avis précédents restent conservés.'),{action:'role-model',scope,reviewer});
 if(!reviewer)field('inherit',trRoleModel('Origine du choix'),selection?'false':'true',[['true',trRoleModel('Hériter du modèle de planification')],['false',trRoleModel('Choisir pour ce responsable')]]);
 field('provider',trRoleModel('Fournisseur'),provider,Object.keys(snapshot.providers?.providers||{}).sort().map(x=>[x,x]));addModelFields('planning','',route?.level||'auto');
 $('confirm').textContent=trRoleModel('Prévisualiser le modèle');$('modal-fields').addEventListener('input',()=>{if(modalContext?.action==='role-model'){modalContext.modelPreview=null;$('preview').hidden=true;$('confirm').textContent=trRoleModel('Prévisualiser le modèle')}});
 },
 async submit(c,f){const r={scope:c.scope,reviewer:c.reviewer,provider:f.provider,level:f.level,model_policy_hash:f.model_policy_hash,inherit:!c.reviewer&&f.inherit==='true'},sig=JSON.stringify(r);
 if(c.modelPreview!==sig){const w=await act('role-model-preview',{role_model:r});if(modalContext!==c)return;const p=w.planning,m=c.reviewer?p.reviewer:p.scopes.find(s=>s.id===c.scope).model_selection;preview(m?(m.provider+' · '+(m.route||m.model_route)?.model):trRoleModel('Hériter du modèle de planification'));c.modelPreview=sig;$('confirm').textContent=trRoleModel('Enregistrer le modèle');return}
 await act('role-model',{role_model:r});if(modalContext!==c)return;closeModal();await refresh();notice(trRoleModel('Modèle enregistré pour les prochains départs.'));
 }
};
