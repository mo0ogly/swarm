'use strict';
const trTaskModel=s=>globalThis.SwarmI18n?.t(s)??s;
const TaskModels={
 text(t,a){if(a?.model_route)return trTaskModel('Utilisé : ')+a.provider+' · '+a.model_route.model;const m=t?.model_selection;if(m)return trTaskModel('Prévu : ')+m.provider+' · '+m.route.model;const p=t?.launch_profile||snapshot.work.launch_profile;return trTaskModel('Hérité : ')+(p?p.provider+' · '+(p.level||'auto'):trTaskModel('À configurer'))},
 open(id){const t=snapshot.work.tasks.find(x=>x.id===id);if(!t)return;const m=t.model_selection,p=t.launch_profile||snapshot.work.launch_profile;
 openModal(trTaskModel('Modèle de la tâche')+' — '+t.title,trTaskModel('Ce choix vaut pour les prochains départs. Il ne change ni les anciennes tentatives, ni les responsables, ni le vérificateur. Les budgets restent appliqués.'),{action:'task-model',task:id});
 field('inherit',trTaskModel('Origine du choix'),m?'false':'true',[['true',trTaskModel('Hériter du profil de lancement')],['false',trTaskModel('Choisir pour cette tâche')]]);
 field('provider',trTaskModel('Fournisseur'),m?.provider||p?.provider||'',Object.keys(snapshot.providers?.providers||{}).sort().map(x=>[x,x]));
 addModelFields('work','',m?.route.level||p?.level||'auto');
 $('confirm').textContent=trTaskModel('Prévisualiser le modèle');
 $('modal-fields').addEventListener('input',()=>{if(modalContext?.action==='task-model'){modalContext.modelPreview=null;$('preview').hidden=true;$('confirm').textContent=trTaskModel('Prévisualiser le modèle')}});
 },
 async submit(c,f){const r={provider:f.provider,level:f.level,model_policy_hash:f.model_policy_hash,inherit:f.inherit==='true'},sig=JSON.stringify(r);
 if(c.modelPreview!==sig){const v=await act('task-model-preview',{task:c.task,task_model:r});if(modalContext!==c)return;preview(v.after?trTaskModel('Prévu : ')+v.after.provider+' · '+v.after.route.model+(v.after.route.effort?' · '+v.after.route.effort:''):trTaskModel('Hériter du profil de lancement'));c.modelPreview=sig;$('confirm').textContent=trTaskModel('Enregistrer le modèle');return}
 await act('task-model',{task:c.task,task_model:r});if(modalContext!==c)return;closeModal();await refresh();notice(trTaskModel('Modèle enregistré pour les prochains départs.'));
 }
};
