'use strict';
Object.assign(globalThis.SwarmEnglish || {}, {
 'Dialogue de préparation':'Preparation dialogue',
 'Portée : tous les nouveaux échanges de ce projet':'Scope: all new exchanges in this project',
 'Valeurs initiales':'Initial values',
 'Réglage enregistré':'Saved setting',
 'Historique des réglages':'Settings history',
 'Aucun changement enregistré.':'No changes saved.',
 'Recharger les réglages':'Reload settings',
 'Le délai doit être un entier positif, inférieur ou égal au plafond.':'Timeout must be a positive integer no greater than the maximum.',
 'Avant':'Before', 'Après':'After',
 'Arrêt manuel toujours disponible. Aucune relance automatique après expiration.':'Manual stop remains available. No automatic retry after expiry.',
 'Réglages avancés':'Advanced settings',
 'Ce plafond limite le délai autorisé pour les prochains échanges.':'This maximum limits the timeout permitted for future exchanges.',
 'Délai par défaut (secondes)':'Default timeout (seconds)',
 'Plafond configurable (secondes)':'Configurable maximum (seconds)',
 'Régler le délai de préparation':'Configure preparation timeout',
 'Ces réglages s’appliquent aux prochains échanges. Les échanges déjà envoyés conservent leur délai.':'These settings apply to future exchanges. Submitted exchanges retain their deadline.',
 'Enregistrer les délais':'Save timeouts',
 'Délais enregistrés.':'Timeouts saved.',
 'Reprendre ces valeurs':'Restore these values',
 'Chargement des délais…':'Loading timeouts…'
});
(function installPreparationTimeoutAdmin(){
 const tr=tr_web_admin_js,host=node('article',undefined,'card');host.id='preparation-timeout-admin';
 host.append(node('p',tr('Portée : tous les nouveaux échanges de ce projet'),'preparation-policy-scope'),node('h3',tr('Dialogue de préparation')),node('p',tr('Ces réglages s’appliquent aux prochains échanges. Les échanges déjà envoyés conservent leur délai.')));
 const state=node('p'),current=node('div'),history=node('details'),hist=node('div');history.append(node('summary',tr('Historique des réglages')),hist);state.setAttribute('role','status');state.id='preparation-timeout-state';current.id='preparation-timeout-current';
 function button(text,action){const b=node('button',text);b.type='button';b.addEventListener('click',action);return b}
 const edit=button(tr('Régler le délai de préparation'),()=>open());edit.id='preparation-timeout-edit';edit.disabled=true;
 const actions=node('div',undefined,'toolbar');actions.append(edit,button(tr('Recharger les réglages'),()=>load()));host.append(state,current,actions,node('p',tr('Arrêt manuel toujours disponible. Aucune relance automatique après expiration.'),'preparation-policy-note'),history);$('admin').prepend(host);let config;
 function describe(v){const dl=node('dl',undefined,'preparation-policy-values');for(const [label,value] of [['Délai par défaut (secondes)',v.timeout_seconds],['Plafond configurable (secondes)',v.max_timeout_seconds]]){const group=node('div');group.append(node('dt',tr(label)),node('dd',String(value)));dl.append(group)}return dl}
 async function load(){edit.disabled=true;state.textContent=tr('Chargement des délais…');try{config=await api('/api/v1/preparation-timeout');current.replaceChildren(node('p',tr(config.revision?'Réglage enregistré':'Valeurs initiales')+(config.revision?' · '+tr('Révision ')+config.revision:''),'preparation-policy-state'),describe(config.values));hist.replaceChildren();if(!config.history.length)hist.append(node('p',tr('Aucun changement enregistré.')));for(const h of config.history.slice().reverse()){const row=node('div',undefined,'card');row.append(node('p',tr('Révision ')+h.revision+' · '+h.at+' · '+h.actor),describe(h.values),node('p',h.change.reason),button(tr('Reprendre ces valeurs'),()=>open(h.values)));hist.append(row)}state.textContent='';edit.disabled=false}catch(e){state.textContent=e.message}}
 function open(values=config.values){
  const frozen=structuredClone(config),dialog=node('dialog',undefined,'graph-modal');dialog.id='preparation-timeout-modal';dialog.setAttribute('aria-labelledby','preparation-timeout-title');
  const header=node('header'),title=node('h3',tr('Régler le délai de préparation'));title.id='preparation-timeout-title';header.append(title);
  const form=node('form'),body=node('div',undefined,'dialog-body'),footer=node('footer');
  function input(id,label,value){const l=node('label',tr(label)),n=node('input');n.id=id;n.type='number';n.min='1';n.step='1';n.required=true;n.value=value;l.htmlFor=id;l.append(n);body.append(l);return n}
  const timeout=input('preparation-timeout-default','Délai par défaut (secondes)',values.timeout_seconds),maximum=input('preparation-timeout-maximum','Plafond configurable (secondes)',values.max_timeout_seconds);
  const label=node('label',tr('Motif de ce changement')),reason=node('textarea');reason.id='preparation-timeout-reason';reason.required=true;reason.minLength=8;reason.maxLength=2000;label.htmlFor=reason.id;label.append(reason);body.append(label);
  const advanced=node('details'),advancedTitle=node('summary',tr('Réglages avancés'));advanced.append(advancedTitle,node('p',tr('Ce plafond limite le délai autorisé pour les prochains échanges.')),maximum.parentElement);body.insertBefore(advanced,label);
  const error=node('p',undefined,'notice alert'),proposal=node('div',undefined,'card');error.id='preparation-timeout-error';error.setAttribute('role','alert');error.hidden=true;proposal.id='preparation-timeout-preview';proposal.hidden=true;body.append(error,proposal);
  const submit=node('button',tr('Prévisualiser'),'primary');submit.id='preparation-timeout-confirm';submit.type='submit';footer.append(button(tr('Fermer'),()=>dialog.close()),submit);form.append(body,footer);dialog.append(header,form);let pending=null,sending=false;
  form.addEventListener('input',()=>{pending=null;proposal.hidden=true;submit.textContent=tr('Prévisualiser')});
  form.addEventListener('submit',async e=>{e.preventDefault();if(sending)return;sending=true;submit.disabled=true;error.hidden=true;try{
   const request={schema_version:1,event_id:pending?.event_id||crypto.randomUUID(),expected_revision:frozen.revision,values:{timeout_seconds:Number(timeout.value),max_timeout_seconds:Number(maximum.value)},reason:reason.value.trim()};
   if(!pending){if(!Number.isSafeInteger(request.values.timeout_seconds)||!Number.isSafeInteger(request.values.max_timeout_seconds)||request.values.timeout_seconds<1||request.values.timeout_seconds>request.values.max_timeout_seconds)throw new Error(tr('Le délai doit être un entier positif, inférieur ou égal au plafond.'));const result=await api('/api/v1/preparation-timeout/preview',request);if(!dialog.open)return;pending=request;proposal.replaceChildren(node('h4',tr('Avant')),describe(frozen.values),node('h4',tr('Après')),describe(result.config.values));proposal.hidden=false;submit.textContent=tr('Enregistrer les délais')}
   else {await api('/api/v1/preparation-timeout/apply',pending);if(!dialog.open)return;dialog.close();await load();edit.focus();state.textContent=tr('Délais enregistrés.')}
  }catch(e){if(dialog.open){error.textContent=e.message;error.hidden=false;error.tabIndex=-1;error.focus();pending=null;proposal.hidden=true;submit.textContent=tr('Prévisualiser')}}finally{sending=false;submit.disabled=false}});
  form.addEventListener('invalid',e=>{if(advanced.contains(e.target))advanced.open=true},true);
  dialog.addEventListener('close',()=>{dialog.remove();edit.focus()},{once:true});document.body.append(dialog);dialog.showModal();timeout.focus();
 }
 load();
})();
