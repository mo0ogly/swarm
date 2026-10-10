'use strict';
(() => {
 const t=s=>globalThis.SwarmI18n?.t(s)??s,language=globalThis.SwarmI18n?.language==='en'?'en':'fr';
 const $=id=>document.getElementById(id);let works=[],generation=0;
 const descriptions={build:'Décrivez les utilisateurs, le besoin et le résultat attendu.',improve:'Précisez ce qui existe, ce qui doit changer et ce qui doit rester intact.',debug:'Décrivez le symptôme, les étapes de reproduction et le comportement attendu.'};
 function themed(value){document.documentElement.dataset.theme=value;try{localStorage.setItem('swarm-theme',value)}catch{}$('project-theme').setAttribute('aria-pressed',String(value==='sombre'))}
 try{themed(localStorage.getItem('swarm-theme')==='sombre'?'sombre':'etat')}catch{themed('etat')}
 $('project-theme').addEventListener('click',()=>themed(document.documentElement.dataset.theme==='sombre'?'etat':'sombre'));
 for(const a of document.querySelectorAll('a[href^="/"]')){const url=new URL(a.getAttribute('href'),location.origin);url.searchParams.set('lang',language);a.href=url.pathname+url.search+url.hash}
 function intent(){const value=document.querySelector('[name=intent]:checked').value;$('project-start').href='/prepare.html?'+new URLSearchParams({intent:value,lang:language});$('project-intent-description').textContent=t(descriptions[value])}
 for(const radio of document.querySelectorAll('[name=intent]'))radio.addEventListener('change',intent);intent();
 function render(){const q=$('project-search').value.trim().toLocaleLowerCase();const shown=works.filter(w=>[w.title,w.objective,w.id].join(' ').toLocaleLowerCase().includes(q));$('project-list').replaceChildren();
  for(const w of shown){const li=document.createElement('li'),a=document.createElement('a'),text=document.createElement('div'),title=document.createElement('strong'),detail=document.createElement('small'),open=document.createElement('span');a.href='/?'+new URLSearchParams({work:w.id,view:'conduite',lang:language});title.textContent=w.title;detail.textContent=w.objective||w.id;open.className='project-open';open.append(document.createTextNode(t('Ouvrir le graphe')+' '));const arrow=document.createElement('span');arrow.textContent='↗';arrow.setAttribute('aria-hidden','true');open.append(arrow);text.append(title,detail);a.append(text,open);li.append(a);$('project-list').append(li)}
  $('project-status').textContent=!works.length?t('Votre atelier est prêt pour sa première mission. Commencez par préparer un projet.'):!shown.length?t('Aucune mission ne correspond à votre recherche.'):shown.length+' / '+works.length+' '+t('missions affichées');
 }
 async function load(){const current=++generation;let loaded=false;$('project-retry').hidden=true;$('project-status').parentNode.dataset.error='false';$('project-status').textContent=t('Chargement des missions…');$('project-search').disabled=true;
  try{const response=await fetch('/api/v1/works',{signal:AbortSignal.timeout(15000)});if(!response.ok)throw new Error(String(response.status));const data=await response.json();if(current!==generation)return;if(!Array.isArray(data)||data.some(w=>!w||typeof w.id!=='string'||typeof w.title!=='string'))throw new Error('invalid data');works=data;loaded=true;render()}
  catch(e){if(current!==generation)return;$('project-status').textContent=t('Impossible de charger les missions. Vérifiez votre connexion puis réessayez.');$('project-status').parentNode.dataset.error='true';$('project-retry').hidden=false}
  finally{if(current===generation)$('project-search').disabled=!loaded}
 }
 async function models(){$('project-model-retry').hidden=true;$('project-model-status').dataset.error='false';$('project-model-status').textContent=t('Chargement des modèles…');
  try{const response=await fetch('/api/v1/preparations/templates',{signal:AbortSignal.timeout(15000)});if(!response.ok)throw Error(String(response.status));const catalog=await response.json();if(!Array.isArray(catalog))throw Error('invalid catalogue');const subjects=catalog.filter(m=>m.subject);$('project-models').replaceChildren();
   for(const m of subjects){const li=document.createElement('li'),h=document.createElement('h3'),p=document.createElement('p'),details=document.createElement('details'),summary=document.createElement('summary'),body=document.createElement('div'),outcome=document.createElement('p'),evidence=document.createElement('ul'),a=document.createElement('a');h.textContent=m.title[language];p.textContent=m.subject.summary[language];summary.textContent=t('Résultat et preuves attendus');outcome.textContent=m.subject.outcome[language];for(const text of m.subject.evidence[language]){const item=document.createElement('li');item.textContent=text;evidence.append(item)}body.append(outcome,evidence);details.append(summary,body);a.href='/prepare.html?'+new URLSearchParams({start:'template',template:m.id,lang:language});a.textContent=t('Préparer ce sujet')+' ↗';li.append(h,p,details,a);$('project-models').append(li)}
   $('project-model-status').textContent=subjects.length?t('5 étapes communes au produit, puis 6 étapes pour chaque fonctionnalité.'):t('Aucun modèle spécialisé disponible. Utilisez les parcours généraux.');
  }catch{ $('project-model-status').dataset.error='true';$('project-model-status').textContent=t('Impossible de charger les modèles. Réessayez ou ouvrez la préparation.');$('project-model-retry').hidden=false }
 }
 $('project-model-retry').addEventListener('click',models);models();
 const methodStages=[
  ['Examiner l’existant','Identifier le code, les données et les comportements concernés avant de décider quoi changer.','À examiner : impacts, acquis réutilisables et inconnues.'],
  ['Concevoir les écrans','Décrire les interactions, les états vide, chargement, erreur et succès, puis vérifier l’accessibilité.','À préparer : écrans ou contrats API adaptés à la fonctionnalité.'],
  ['Planifier les tâches','Découper la fonctionnalité en tâches bornées avec des dépendances, des contrôles et une reprise prévue.','À obtenir : un plan examiné et adopté explicitement.'],
  ['Réaliser','Implémenter les tâches autorisées en préservant les données et les comportements existants.','À fournir : changements concrets, tests et retour de l’exécutant.'],
  ['Vérifier le résultat','Contrôler le même candidat, examiner ses preuves et obtenir une revue indépendante.','À établir : critères vérifiés, régressions examinées et limites visibles.'],
  ['Livrer','Intégrer ou déployer le résultat après les contrôles, la revue et l’autorisation nécessaire.','À confirmer : version effectivement livrée et fonctionnement après installation.']
 ];
 const methodDialog=$('project-method-dialog');
 function methodStage(index){const stage=methodStages[index];if(!stage)return;for(const button of document.querySelectorAll('[data-method-stage]'))button.setAttribute('aria-pressed',String(Number(button.dataset.methodStage)===index));$('project-method-stage-title').textContent=t(stage[0]);$('project-method-stage-result').textContent=t(stage[1]);$('project-method-stage-proof').textContent=t(stage[2])}
 for(const button of document.querySelectorAll('[data-method-stage]'))button.addEventListener('click',()=>methodStage(Number(button.dataset.methodStage)));
 $('project-method-open').addEventListener('click',()=>{methodStage(0);methodDialog.showModal();$('project-method-close').focus()});
 $('project-method-close').addEventListener('click',()=>methodDialog.close());
 methodDialog.addEventListener('keydown',event=>{if(event.key!=='Tab')return;const controls=[...methodDialog.querySelectorAll('button')],first=controls[0],last=controls.at(-1);if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus()}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus()}});
 methodDialog.addEventListener('close',()=>$('project-method-open').focus());
 $('project-search').addEventListener('input',render);$('project-retry').addEventListener('click',load);load();
})();
