'use strict';
(function(){
 const tr=s=>globalThis.SwarmI18n?.t(s)??s,host=node('article',undefined,'card');host.id='web-requests-admin';
 host.append(node('h3',tr('Attente des requêtes navigateur')),node('p',tr('Ces délais limitent l’attente de l’interface. Une expiration n’arrête aucun agent et ne prouve pas l’échec de l’opération.')));
 const state=node('p',tr('Chargement…'),'notice info');state.id='web-requests-state';state.setAttribute('role','status');
 const values=node('div'),history=node('details'),hist=node('div');history.append(node('summary',tr('Historique')),hist);
 const edit=button(tr('Régler les délais navigateur'),()=>open()),reload=button(tr('Recharger'),()=>load());edit.id='web-requests-edit';edit.disabled=true;reload.id='web-requests-reload';
 const actions=node('div',undefined,'toolbar');actions.append(edit,reload);host.append(state,values,actions,history);$('admin').append(host);let config;
 const fields=[['general_ms','Requêtes courantes (ms)'],['snapshot_ms','Lecture de l’état (ms)'],['connection_test_ms','Test de connexion IA (ms)']];
 function describe(v){const table=node('table',undefined,'engine-quota-summary'),body=node('tbody');for(const [key,label]of fields){const row=node('tr');row.append(node('th',tr(label)),node('td',v[key]===0?tr('Sans délai'):String(v[key])));body.append(row)}table.append(body);return table}
 async function load(){edit.disabled=true;try{config=await api('/api/v1/web-requests');SwarmWebRequests.adopt(config);values.replaceChildren(describe(config.values));hist.replaceChildren();for(const h of [...config.history].reverse())hist.append(node('p',tr('Révision ')+h.revision+' · '+h.at+' · '+h.change.reason),describe(h.values));state.className='notice info';state.textContent=tr('Révision ')+config.revision;edit.disabled=false}catch(e){config=null;state.className='notice alert';state.textContent=e.message}}
 function open(){
  if(!config)return;
  const frozen=structuredClone(config),dialog=node('dialog'),header=node('header',undefined,'modal-header'),heading=node('h2',tr('Régler les délais navigateur'));heading.id='web-requests-title';dialog.id='web-requests-modal';dialog.setAttribute('aria-labelledby',heading.id);header.append(heading,button(tr('Fermer'),()=>dialog.close()));
  const form=node('form'),body=node('div',undefined,'modal-body'),footer=node('footer',undefined,'modal-footer'),inputs={};
  body.append(node('p',tr('0 désactive l’expiration navigateur. Les appels déjà partis gardent leur délai. Les autres pages reçoivent ces réglages à leur rechargement.')));
  for(const [key,label]of fields){const input=node('input'),field=node('label',tr(label));input.id='web-requests-'+key;input.type='number';input.min='0';input.step='1';input.required=true;input.max=String(frozen.bounds.max_safe_integer_ms);input.value=frozen.values[key];field.htmlFor=input.id;field.append(input);body.append(field);inputs[key]=input}
  const reason=node('textarea'),reasonLabel=node('label',tr('Motif du changement'));reason.id='web-requests-reason';reason.required=true;reason.minLength=8;reason.maxLength=2000;reasonLabel.htmlFor=reason.id;reasonLabel.append(reason);body.append(reasonLabel);
  const error=node('p',undefined,'notice alert'),proposal=node('div',undefined,'card');error.id='web-requests-error';error.hidden=true;error.setAttribute('role','alert');proposal.id='web-requests-preview';proposal.hidden=true;body.append(error,proposal);
  const submit=node('button',tr('Prévisualiser'),'primary');submit.id='web-requests-confirm';submit.type='submit';footer.append(button(tr('Fermer'),()=>dialog.close()),submit);form.append(body,footer);dialog.append(header,form);
  let pending;form.addEventListener('input',()=>{pending=null;proposal.hidden=true;error.hidden=true;submit.textContent=tr('Prévisualiser')});
  form.addEventListener('submit',async e=>{e.preventDefault();if(!form.reportValidity())return;submit.disabled=true;error.hidden=true;try{
   const next={};for(const [key]of fields){next[key]=Number(inputs[key].value);if(!Number.isSafeInteger(next[key])||next[key]<0)throw Error(tr('Saisissez des nombres entiers représentables sans perte.'))}
   const request={schema_version:1,event_id:pending?.event_id??crypto.randomUUID(),expected_revision:frozen.revision,values:next,reason:reason.value};
   if(!pending){const result=await api('/api/v1/web-requests/preview',request);if(!dialog.open)return;pending=request;proposal.replaceChildren(node('h3',tr('Valeurs proposées')),describe(result.config.values));proposal.hidden=false;submit.textContent=tr('Enregistrer les délais')}
   else{const result=await api('/api/v1/web-requests/apply',pending);SwarmWebRequests.adopt(result.config);if(!dialog.open)return;dialog.close();await load();if(!document.querySelector('dialog[open]'))(edit.disabled?reload:edit).focus()}
  }catch(e){if(dialog.open){error.textContent=e.message;error.hidden=false}}finally{submit.disabled=false}});
  dialog.addEventListener('close',()=>{dialog.remove();edit.focus()},{once:true});document.body.append(dialog);dialog.showModal();inputs.general_ms.focus();
 }
 load();
})();
