'use strict';
Object.assign(globalThis.SwarmEnglish || {}, {
 'Agents responsables et vérificateurs':'Planner and reviewer agents',
 'Portée : prochains appels des responsables et vérificateurs':'Scope: future planner and reviewer calls',
 'Valeurs initiales':'Initial values',
 'Réglage enregistré':'Saved setting',
 'Historique des réglages':'Settings history',
 'Aucun changement enregistré.':'No changes saved.',
 'Recharger les réglages':'Reload settings',
 'Les durées doivent être des entiers positifs ou nuls.':'Durations must be nonnegative integers.',
 'Avant':'Before', 'Après':'After',
 'Arrêt manuel, quotas et budgets restent applicables. Les appels en cours conservent leurs réglages.':'Manual stop, quotas and budgets remain enforced. Running calls retain their settings.',
 'Réglages avancés':'Advanced settings',
 '0 désactive ce couperet. Une durée positive limite l’appel même lorsque l’agent est actif.':'0 disables this deadline. A positive duration limits the call even while the agent is active.',
 'Silence maximal (secondes, 0 = désactivé)':'Maximum silence (seconds, 0 = disabled)',
 'Durée totale maximale (0 = désactivée)':'Maximum total duration (0 = disabled)',
 'Régler la surveillance des agents IA':'Configure AI agent monitoring',
 'Le raisonnement et les réponses réarment le délai de silence. Une durée totale à 0 laisse un agent actif poursuivre son travail.':'Reasoning and responses reset the silence timer. A total duration of 0 lets an active agent continue working.',
 'Enregistrer les délais':'Save timeouts',
 'Délais enregistrés.':'Timeouts saved.',
 'Reprendre ces valeurs':'Restore these values',
 'Chargement des délais…':'Loading timeouts…',
 'Bail du responsable (secondes)':'Planner ownership lease (seconds)',
 'Le bail est renouvelé pendant l’appel actif ; il empêche deux responsables de traiter le même retour.':'The lease is renewed during a live call; it prevents two planners from handling the same feedback.'
});
(function installProviderWaitAdmin(){
 const tr=tr_web_admin_js,host=node('article',undefined,'card');host.id='provider-wait-admin';
 host.append(node('p',tr('Portée : prochains appels des responsables et vérificateurs'),'preparation-policy-scope'),node('h3',tr('Agents responsables et vérificateurs')),node('p',tr('Le raisonnement et les réponses réarment le délai de silence. Une durée totale à 0 laisse un agent actif poursuivre son travail.')));
 const state=node('p'),current=node('div'),history=node('details'),hist=node('div');history.append(node('summary',tr('Historique des réglages')),hist);state.setAttribute('role','status');state.id='provider-wait-state';current.id='provider-wait-current';
 function button(text,action){const b=node('button',text);b.type='button';b.addEventListener('click',action);return b}
 const edit=button(tr('Régler la surveillance des agents IA'),()=>open());edit.id='provider-wait-edit';edit.disabled=true;
 const actions=node('div',undefined,'toolbar');actions.append(edit,button(tr('Recharger les réglages'),()=>load()));host.append(state,current,actions,node('p',tr('Arrêt manuel, quotas et budgets restent applicables. Les appels en cours conservent leurs réglages.'),'preparation-policy-note'),history);$('admin').prepend(host);let config;
 function describe(v){const dl=node('dl',undefined,'preparation-policy-values');for(const [label,value] of [['Silence maximal (secondes, 0 = désactivé)',v.silence_seconds],['Durée totale maximale (0 = désactivée)',v.max_duration_seconds],['Bail du responsable (secondes)',v.planning_lease_seconds]]){const group=node('div');group.append(node('dt',tr(label)),node('dd',String(value)));dl.append(group)}return dl}
 async function load(){edit.disabled=true;state.textContent=tr('Chargement des délais…');try{config=await api('/api/v1/provider-wait');current.replaceChildren(node('p',tr(config.revision?'Réglage enregistré':'Valeurs initiales')+(config.revision?' · '+tr('Révision ')+config.revision:''),'preparation-policy-state'),describe(config.values));hist.replaceChildren();if(!config.history.length)hist.append(node('p',tr('Aucun changement enregistré.')));for(const h of config.history.slice().reverse()){const row=node('div',undefined,'card');row.append(node('p',tr('Révision ')+h.revision+' · '+h.at+' · '+h.actor),describe(h.values),node('p',h.change.reason),button(tr('Reprendre ces valeurs'),()=>open(h.values)));hist.append(row)}state.textContent='';edit.disabled=false}catch(e){state.textContent=e.message}}
 function open(values=config.values){
  const frozen=structuredClone(config),dialog=node('dialog',undefined,'graph-modal');dialog.id='provider-wait-modal';dialog.setAttribute('aria-labelledby','provider-wait-title');
  const header=node('header'),title=node('h3',tr('Régler la surveillance des agents IA'));title.id='provider-wait-title';header.append(title);
  const form=node('form'),body=node('div',undefined,'dialog-body'),footer=node('footer');
  function input(id,label,value){const l=node('label',tr(label)),n=node('input');n.id=id;n.type='number';n.min='0';n.step='1';n.required=true;n.value=value;l.htmlFor=id;l.append(n);body.append(l);return n}
  const timeout=input('provider-wait-default','Silence maximal (secondes, 0 = désactivé)',values.silence_seconds),maximum=input('provider-wait-maximum','Durée totale maximale (0 = désactivée)',values.max_duration_seconds);
  const lease=input('provider-wait-lease','Bail du responsable (secondes)',values.planning_lease_seconds);lease.min='5';lease.max='300';body.append(node('p',tr('Le bail est renouvelé pendant l’appel actif ; il empêche deux responsables de traiter le même retour.')));
  const label=node('label',tr('Motif de ce changement')),reason=node('textarea');reason.id='provider-wait-reason';reason.required=true;reason.minLength=8;reason.maxLength=2000;label.htmlFor=reason.id;label.append(reason);body.append(label);
  const advanced=node('details'),advancedTitle=node('summary',tr('Réglages avancés'));advanced.append(advancedTitle,node('p',tr('0 désactive ce couperet. Une durée positive limite l’appel même lorsque l’agent est actif.')),maximum.parentElement,lease.parentElement);body.insertBefore(advanced,label);
  const error=node('p',undefined,'notice alert'),proposal=node('div',undefined,'card');error.id='provider-wait-error';error.setAttribute('role','alert');error.hidden=true;proposal.id='provider-wait-preview';proposal.hidden=true;body.append(error,proposal);
  const submit=node('button',tr('Prévisualiser'),'primary');submit.id='provider-wait-confirm';submit.type='submit';footer.append(button(tr('Fermer'),()=>dialog.close()),submit);form.append(body,footer);dialog.append(header,form);let pending=null,sending=false;
  form.addEventListener('input',()=>{pending=null;proposal.hidden=true;submit.textContent=tr('Prévisualiser')});
  form.addEventListener('submit',async e=>{e.preventDefault();if(sending)return;sending=true;submit.disabled=true;error.hidden=true;try{
   const request={schema_version:1,event_id:pending?.event_id||crypto.randomUUID(),expected_revision:frozen.revision,values:{silence_seconds:Number(timeout.value),max_duration_seconds:Number(maximum.value),planning_lease_seconds:Number(lease.value)},reason:reason.value.trim()};
   if(!pending){if(!Number.isSafeInteger(request.values.silence_seconds)||!Number.isSafeInteger(request.values.max_duration_seconds)||request.values.silence_seconds<0||request.values.max_duration_seconds<0)throw new Error(tr('Les durées doivent être des entiers positifs ou nuls.'));const result=await api('/api/v1/provider-wait/preview',request);if(!dialog.open)return;pending=request;proposal.replaceChildren(node('h4',tr('Avant')),describe(frozen.values),node('h4',tr('Après')),describe(result.config.values));proposal.hidden=false;submit.textContent=tr('Enregistrer les délais')}
   else {await api('/api/v1/provider-wait/apply',pending);if(!dialog.open)return;dialog.close();await load();edit.focus();state.textContent=tr('Délais enregistrés.')}
  }catch(e){if(dialog.open){error.textContent=e.message;error.hidden=false;error.tabIndex=-1;error.focus();pending=null;proposal.hidden=true;submit.textContent=tr('Prévisualiser')}}finally{sending=false;submit.disabled=false}});
  form.addEventListener('invalid',e=>{if(advanced.contains(e.target))advanced.open=true},true);
  dialog.addEventListener('close',()=>{dialog.remove();edit.focus()},{once:true});document.body.append(dialog);dialog.showModal();timeout.focus();
 }
 load();
})();
