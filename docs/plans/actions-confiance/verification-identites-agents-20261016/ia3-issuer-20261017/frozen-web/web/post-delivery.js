'use strict';
const PostDelivery={
 tr:s=>globalThis.SwarmI18n?.t(s)??s,
 render(){
  const phase=snapshot.post_delivery;if(!phase||!snapshot.work.tasks.length)return null;
  const box=node('section',undefined,'post-delivery');box.dataset.phase='retex';box.dataset.ready=String(phase.ready);box.setAttribute('aria-label',this.tr('RETEX & quick wins'));
  box.append(node('h4',this.tr('Après la recette · RETEX & quick wins')),node('p',this.tr('Prenez du recul sur le résultat et choisissez les évolutions utiles pour votre projet.')));
  if(!phase.ready){box.append(node('p',this.tr('Disponible après une recette acceptée avec des preuves à jour.')));return box;}
  box.append(node('strong',this.tr('Recette acceptée · évolutions au choix')));
  const actions=node('div',undefined,'post-delivery-actions');actions.append(Pilot.command(this.tr('Proposer un quick win ou une évolution'),()=>this.propose()),Pilot.command(this.tr('Consulter les propositions'),()=>this.openProposals()));box.append(actions,node('p',this.tr('Enregistrer une proposition ne lance aucun agent. Vous choisirez les évolutions à préparer.')));
  return box;
 },
 openProposals(){showView('brainstorm');const list=$('retex-list'),fold=list.closest('details');if(fold)fold.open=true;list.scrollIntoView({block:'start'});const first=list.querySelector('a,button');first?.focus();},
 propose(){
  const sources=snapshot.post_delivery?.sources||[];if(!sources.length)return;
  openModal(this.tr('RETEX après recette'),this.tr('Décrivez le constat, la valeur attendue et comment vérifier cette amélioration.'),{action:'retex-save',retexID:'r-'+crypto.randomUUID(),sourceKind:'acceptance',sources});
  field('source-task',this.tr('Résultat de recette utilisé'),sources[0].task,sources.map(s=>[s.task,snapshot.work.tasks.find(t=>t.id===s.task)?.title||s.task]));
  field('problem',this.tr('Constat à améliorer'),'',null,true);field('nature',this.tr('Nature du constat'),'hypothese',[['hypothese',this.tr('Hypothèse à confirmer')],['fait',this.tr('Fait démontré')]]);
  field('opportunity_kind',this.tr('Type d’évolution'),'quick_win',[['quick_win',this.tr('Quick win')],['evolution',this.tr('Évolution structurante')]]);
  field('proposal',this.tr('Évolution proposée'),'',null,true);field('impact',this.tr('Valeur attendue pour l’utilisateur'),'',null,true);field('effort',this.tr('Effort estimatif'),'',null,true);field('risk',this.tr('Risques et limites'),'',null,true);field('criteria',this.tr('Comment vérifier la réussite ?'),'',null,true);field('proof',this.tr('Preuves ou vérification de l’hypothèse'),'',null,true);
  $('confirm').textContent=this.tr('Enregistrer cette proposition');
 }
};
