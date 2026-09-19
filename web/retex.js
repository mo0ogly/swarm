'use strict';
const tr_web_retex_js = source => globalThis.SwarmI18n?.t(source) ?? source;

let retexKey='';
const retexLabels={propose:tr_web_retex_js('Proposé'),retenu:tr_web_retex_js('Retenu'),en_cours:tr_web_retex_js('En cours'),verifie:tr_web_retex_js('Vérifié'),ecarte:tr_web_retex_js('Écarté'),remplace:tr_web_retex_js('Remplacé')};
async function newRetex(t){
 const answers=t.answers||[];const last=answers[answers.length-1];
 const digest=[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(t.question+'\n'+t.response)))].map(x=>x.toString(16).padStart(2,'0')).join('');
 openModal(tr_web_retex_js('Examiner un RETEX'),tr_web_retex_js('Sélectionnez une proposition de la réponse IA, puis précisez sa preuve et son critère de réussite. Trois fiches maximum par réponse.'),{action:'retex-save',retexID:'r-'+crypto.randomUUID(),source:{task:t.id,attempt:last?.attempt||'',sha256:digest}});
 preview('QUESTION SOURCE\n'+t.question+'\n\nRÉPONSE SOURCE\n'+t.response);
 field('problem',tr_web_retex_js('Constat à améliorer'),'',null,true);field('nature',tr_web_retex_js('Nature du constat'),'hypothese',[['hypothese',tr_web_retex_js('Hypothèse à confirmer')],['fait',tr_web_retex_js('Fait démontré')]]);field('proposal',tr_web_retex_js('Amélioration retenue dans la réponse'),'',null,true);field('effort',tr_web_retex_js('Effort estimatif'),tr_web_retex_js('À estimer'));field('risk',tr_web_retex_js('Risques et limites'),'',null,true);field('criteria',tr_web_retex_js('Comment vérifier la réussite ?'),'',null,true);field('proof',tr_web_retex_js('Références des preuves (fichiers, tests, échanges)'),'',null,true);$('confirm').textContent=tr_web_retex_js('Enregistrer cette proposition');
}
function renderRetex(){
 const rows=snapshot.work.retex||[];const key=JSON.stringify([work,rows]);if(key===retexKey)return;retexKey=key;
 $('retex-list').replaceChildren();if(!rows.length){$('retex-list').append(node('p',tr_web_retex_js('Depuis une réponse IA, choisissez « Examiner un RETEX issu de cette réponse ».')));return}
 for(const r of rows){const card=node('article',undefined,'card');card.dataset.retex=r.id;card.dataset.state=r.status;card.append(node('h3',r.problem),badge(r.status),node('p',r.proposal),node('p',tr_web_retex_js('Critère : ')+r.criteria),node('p',tr_web_retex_js('Preuves : ')+(r.proof||tr_web_retex_js('à fournir'))),node('p','Export Markdown : '+(r.export_error||(!r.export_hash||r.export_pending?tr_web_retex_js('en attente'):tr_web_retex_js('enregistré'))),'export-state '+(r.export_error?'alert':!r.export_hash||r.export_pending?'attention':'success')));
 card.append(button(tr_web_retex_js('Examiner la fiche'),()=>{openModal(tr_web_retex_js('RETEX · ')+r.problem,'Source '+r.source.task+' · '+r.source.attempt,{action:'retex-read'});preview(tr_web_retex_js('Nature : ')+r.nature+tr_web_retex_js('\nEffort : ')+r.effort+tr_web_retex_js('\nRisque : ')+r.risk+'\n\n'+r.proposal+tr_web_retex_js('\n\nPreuves : ')+r.proof);$('confirm').hidden=true}));
 if(!r.task)card.append(button(tr_web_retex_js('Créer une tâche APEX'),()=>{openModal(tr_web_retex_js('Créer la tâche APEX'),tr_web_retex_js('Une tâche sera ajoutée au plan. Aucun agent ne sera lancé.'),{action:'retex-task',task:r.id});preview(r.problem+'\n\n'+r.proposal+tr_web_retex_js('\n\nCritère : ')+r.criteria);$('confirm').textContent=tr_web_retex_js('Ajouter au plan APEX')}));
 else card.append(button(tr_web_retex_js('Piloter la tâche APEX'),()=>taskDialog(r.task)));
 if(!r.lesson)card.append(button(tr_web_retex_js('Qualifier les preuves'),()=>{openModal(tr_web_retex_js('Qualifier le constat'),tr_web_retex_js('La qualification ne remplace pas la gate de la tâche.'),{action:'retex-qualify',task:r.id});field('nature','Nature',r.nature,[['hypothese',tr_web_retex_js('Hypothèse')],['fait',tr_web_retex_js('Fait démontré')]]);field('proof','Preuves',r.proof,null,true)}),button(tr_web_retex_js('Changer l’état du RETEX'),()=>{openModal(tr_web_retex_js('État du RETEX'),tr_web_retex_js('Vérifié exige un fait sourcé et une tâche acceptée avec une gate fraîche.'),{action:'retex-status',task:r.id});field('note',tr_web_retex_js('État'),r.status,Object.entries(retexLabels).filter(([k])=>k!=='propose'))}));
 card.append(button(tr_web_retex_js('Exporter le RETEX en Markdown'),()=>{openModal(tr_web_retex_js('Exporter le RETEX'),tr_web_retex_js('Fichier local : docs/retex/swarm/')+r.id+'.md. Les modifications externes seront préservées.',{action:'retex-export',task:r.id});preview(r.problem+'\n'+r.proposal)}));
 if(r.status==='verifie'&&!r.lesson)card.append(button(tr_web_retex_js('Proposer une leçon au Guide'),()=>{openModal(tr_web_retex_js('Leçon pour le Guide de terrain'),tr_web_retex_js('Examinez le texte et le changement d’index avant publication. Le budget du Guide sera contrôlé.'),{action:'retex-guide',task:r.id});field('note',tr_web_retex_js('Leçon réutilisable, circonstances et limites'),r.proposal,null,true);$('confirm').textContent=tr_web_retex_js('Examiner les changements')}));
 if(r.lesson)card.append(node('p',tr_web_retex_js('Leçon publiée : ')+r.lesson));$('retex-list').append(card);
 }
}
async function submitRetex(f,c){
 if(c.action==='retex-save')await act(c.action,{retex:{id:c.retexID,source:c.source,...f}});
 else if(c.action==='retex-qualify')await act(c.action,{task:c.task,retex:{nature:f.nature,proof:f.proof}});
 else if(c.action==='retex-guide'){
  if(!c.digest||c.lesson!==f.note){const v=await act('retex-guide-preview',{task:c.task,note:f.note});c.digest=v.digest;c.lesson=f.note;preview(v.preview);$('confirm').textContent=tr_web_retex_js('Publier la leçon vérifiée');return false}
  await act(c.action,{task:c.task,note:f.note,context_hash:c.digest});
 }else {const v=await act(c.action,{task:c.task,note:f.note||''});if(c.action==='retex-export'){const record=v.retex.find(r=>r.id===c.task);if(record.export_error)throw new Error(record.export_error)}}
 return true;
}
