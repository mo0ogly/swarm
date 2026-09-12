'use strict';
let retexKey='';
const retexLabels={propose:'Proposé',retenu:'Retenu',en_cours:'En cours',verifie:'Vérifié',ecarte:'Écarté',remplace:'Remplacé'};
async function newRetex(t){
 const answers=t.answers||[];const last=answers[answers.length-1];
 const digest=[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(t.question+'\n'+t.response)))].map(x=>x.toString(16).padStart(2,'0')).join('');
 openModal('Examiner un RETEX','Sélectionnez une proposition de la réponse IA, puis précisez sa preuve et son critère de réussite. Trois fiches maximum par réponse.',{action:'retex-save',retexID:'r-'+crypto.randomUUID(),source:{task:t.id,attempt:last?.attempt||'',sha256:digest}});
 preview('QUESTION SOURCE\n'+t.question+'\n\nRÉPONSE SOURCE\n'+t.response);
 field('problem','Constat à améliorer','',null,true);field('nature','Nature du constat','hypothese',[['hypothese','Hypothèse à confirmer'],['fait','Fait démontré']]);field('proposal','Amélioration retenue dans la réponse','',null,true);field('effort','Effort estimatif','À estimer');field('risk','Risques et limites','',null,true);field('criteria','Comment vérifier la réussite ?','',null,true);field('proof','Références des preuves (fichiers, tests, échanges)','',null,true);$('confirm').textContent='Enregistrer cette proposition';
}
function renderRetex(){
 const rows=snapshot.work.retex||[];const key=JSON.stringify([work,rows]);if(key===retexKey)return;retexKey=key;
 $('retex-list').replaceChildren();if(!rows.length){$('retex-list').append(node('p','Depuis une réponse IA, choisissez « Examiner un RETEX issu de cette réponse ».'));return}
 for(const r of rows){const card=node('article',undefined,'card');card.dataset.retex=r.id;card.dataset.state=r.status;card.append(node('h3',r.problem),badge(r.status),node('p',r.proposal),node('p','Critère : '+r.criteria),node('p','Preuves : '+(r.proof||'à fournir')),node('p','Export Markdown : '+(r.export_error||(!r.export_hash||r.export_pending?'en attente':'enregistré')),'export-state '+(r.export_error?'alert':!r.export_hash||r.export_pending?'attention':'success')));
 card.append(button('Examiner la fiche',()=>{openModal('RETEX · '+r.problem,'Source '+r.source.task+' · '+r.source.attempt,{action:'retex-read'});preview('Nature : '+r.nature+'\nEffort : '+r.effort+'\nRisque : '+r.risk+'\n\n'+r.proposal+'\n\nPreuves : '+r.proof);$('confirm').hidden=true}));
 if(!r.task)card.append(button('Créer une tâche APEX',()=>{openModal('Créer la tâche APEX','Une tâche sera ajoutée au plan. Aucun agent ne sera lancé.',{action:'retex-task',task:r.id});preview(r.problem+'\n\n'+r.proposal+'\n\nCritère : '+r.criteria);$('confirm').textContent='Ajouter au plan APEX'}));
 else card.append(button('Piloter la tâche APEX',()=>taskDialog(r.task)));
 if(!r.lesson)card.append(button('Qualifier les preuves',()=>{openModal('Qualifier le constat','La qualification ne remplace pas la gate de la tâche.',{action:'retex-qualify',task:r.id});field('nature','Nature',r.nature,[['hypothese','Hypothèse'],['fait','Fait démontré']]);field('proof','Preuves',r.proof,null,true)}),button('Changer l’état du RETEX',()=>{openModal('État du RETEX','Vérifié exige un fait sourcé et une tâche acceptée avec une gate fraîche.',{action:'retex-status',task:r.id});field('note','État',r.status,Object.entries(retexLabels).filter(([k])=>k!=='propose'))}));
 card.append(button('Exporter le RETEX en Markdown',()=>{openModal('Exporter le RETEX','Fichier local : docs/retex/swarm/'+r.id+'.md. Les modifications externes seront préservées.',{action:'retex-export',task:r.id});preview(r.problem+'\n'+r.proposal)}));
 if(r.status==='verifie'&&!r.lesson)card.append(button('Proposer une leçon au Guide',()=>{openModal('Leçon pour le Guide de terrain','Examinez le texte et le changement d’index avant publication. Le budget du Guide sera contrôlé.',{action:'retex-guide',task:r.id});field('note','Leçon réutilisable, circonstances et limites',r.proposal,null,true);$('confirm').textContent='Examiner les changements'}));
 if(r.lesson)card.append(node('p','Leçon publiée : '+r.lesson));$('retex-list').append(card);
 }
}
async function submitRetex(f,c){
 if(c.action==='retex-save')await act(c.action,{retex:{id:c.retexID,source:c.source,...f}});
 else if(c.action==='retex-qualify')await act(c.action,{task:c.task,retex:{nature:f.nature,proof:f.proof}});
 else if(c.action==='retex-guide'){
  if(!c.digest||c.lesson!==f.note){const v=await act('retex-guide-preview',{task:c.task,note:f.note});c.digest=v.digest;c.lesson=f.note;preview(v.preview);$('confirm').textContent='Publier la leçon vérifiée';return false}
  await act(c.action,{task:c.task,note:f.note,context_hash:c.digest});
 }else {const v=await act(c.action,{task:c.task,note:f.note||''});if(c.action==='retex-export'){const record=v.retex.find(r=>r.id===c.task);if(record.export_error)throw new Error(record.export_error)}}
 return true;
}
