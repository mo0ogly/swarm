/* Isolated examples only: this catalogue calls no application API. */
(function () {
 'use strict';
 const $ = id => document.getElementById(id), S = globalThis.SwarmSignals;
 const copy = {
  fr:{edition:'ATELIER VISUEL',kicker:'MÉMOIRE · IDENTITÉ · USAGE',title:'Le cachet au bas du parchemin.',intro:'Reconnaître ce qui est conservé, qui intervient et ce qui est permis.',demonstration:'Démonstration visuelle : aucun cachet ne constitue une signature ou une autorisation réelle.','paper-number':'FRAGMENT 01 · EXEMPLE','paper-title':'Vérifier avant de réutiliser.','paper-body':'Une expérience éclaire la prochaine décision. Elle conserve sa source, ses conditions et ses limites ; elle ne valide pas le résultat présent.','paper-quote':'« Conserver la provenance, puis vérifier le droit d’usage. »','controls-title':'Composer un cachet','memory-label':'Type de mémoire','protection-label':'Protection de l’usage','identity-label':'Identité de l’intervenant',reset:'Revenir à l’état inconnu','legend-kicker':'UNE FORME, UN SENS','legend-title':'La légende des empreintes','legend-intro':'Le type, l’état de version, l’identité et les droits sont des dimensions distinctes.','guide-title':'Comment réutiliser ces cachets ?','guide-body':'Associer les états à des données attestées du moteur. Une identité attestée ne donne pas de droits ; une version active n’est pas une preuve actuelle. Sans donnée, afficher l’état inconnu.','concept-note':'« Faits » est un repère conceptuel ; les types persistants actuels restent expérience et procédure.','language-label':'Langue',theme:'Changer le thème',skip:'Aller au contenu',families:{memory:'01 · Ce que la mémoire conserve',concept:'Repère conceptuel',version:'02 · La vie d’une version',protection:'03 · La protection de l’usage',identity:'04 · L’état de l’identité',actor:'05 · Qui intervient',usage:'06 · Des droits distincts',attribution:'07 · Propriétaire et auteur'}},
  en:{edition:'VISUAL WORKSHOP',kicker:'MEMORY · IDENTITY · USE',title:'The seal at the foot of the parchment.',intro:'Recognize what is retained, who contributes and what is permitted.',demonstration:'Visual demonstration: no seal constitutes a signature or an actual permission.','paper-number':'FRAGMENT 01 · EXAMPLE','paper-title':'Verify before reusing.','paper-body':'An experience informs the next decision. Its source, conditions and limits remain attached; it does not validate the present outcome.','paper-quote':'“Preserve provenance, then verify usage permission.”','controls-title':'Compose a seal','memory-label':'Memory type','protection-label':'Usage protection','identity-label':'Actor identity',reset:'Return to unknown state','legend-kicker':'A SHAPE, A MEANING','legend-title':'A legend of impressions','legend-intro':'Type, version state, identity and permissions are distinct dimensions.','guide-title':'How can these seals be reused?','guide-body':'Bind states to attested engine data. An attested identity grants no rights; an active version is not current evidence. Without data, show the unknown state.','concept-note':'“Facts” is a conceptual marker; current persisted types remain experience and procedure.','language-label':'Language',theme:'Change theme',skip:'Skip to content',families:{memory:'01 · What memory retains',concept:'Conceptual marker',version:'02 · The life of a version',protection:'03 · Usage protection',identity:'04 · Identity state',actor:'05 · Who contributes',usage:'06 · Distinct rights',attribution:'07 · Owner and author'}}
 };
 function preview() {
  const lang = $('language').value, protection = $('protection').value, identity = $('identity').value;
  $('seal-preview').replaceChildren(S.seal({lang,memory:$('memory').value,protection,identity}));
  $('current-meaning').textContent = S.definitions['protection-'+protection].description[lang]+' '+S.definitions['identity-'+identity].description[lang];
 }
 function render() {
  const lang = $('language').value, text = copy[lang];document.documentElement.lang = lang;
  for (const [id,value] of Object.entries(text)) if (typeof value === 'string' && $(id)) $(id).textContent = value;
  document.querySelector('.catalogue-skip').textContent = text.skip;document.title = lang === 'fr' ? 'Cachets de contexte — Swarm' : 'Context seals — Swarm';
  for (const [id,prefix] of [['memory','memory'],['protection','protection'],['identity','identity']]) for (const option of $(id).options) option.textContent = S.definitions[prefix+'-'+option.value].label[lang];
  const legend = $('legend');legend.replaceChildren();
  for (const [family,title] of Object.entries(text.families)) {
   const section = document.createElement('div');section.className = 'catalogue-family';section.dataset.family = family;const heading = document.createElement('h3');heading.textContent = title;
   const list = document.createElement('div');list.className = 'catalogue-family-list';
   for (const item of Object.values(S.definitions).filter(d=>d.family===family)) {
    const entry = document.createElement('div');entry.className = 'catalogue-entry';entry.append(S.badge(item.id,{lang}));
    const detail = document.createElement('p');detail.textContent = item.description[lang];entry.append(detail);list.append(entry);
   }
   section.append(heading,list);legend.append(section);
  }
  preview();
 }
 for (const id of ['memory','protection','identity']) $(id).addEventListener('change',preview);
 $('language').addEventListener('change',render);
 $('theme').addEventListener('click',()=>{document.documentElement.dataset.theme=document.documentElement.dataset.theme==='etat'?'sombre':'etat'});
 $('reset').addEventListener('click',()=>{$('protection').value='unknown';$('identity').value='unknown';preview()});
 render();
})();
