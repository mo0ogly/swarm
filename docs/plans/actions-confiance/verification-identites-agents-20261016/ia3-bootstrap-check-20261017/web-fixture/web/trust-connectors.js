'use strict';
let connectorPending=null;
const connectorForm=trust('connector-form');
function connectorEncode(data,amount){
 if(amount===undefined)return JSON.stringify(data);
 if(!/^(0|[1-9][0-9]*)$/.test(amount)||BigInt(amount)>9223372036854775807n||!Object.hasOwn(data,'amount_minor'))throw Error(trustTr('Montant exact invalide.'));
 return '{'+Object.keys(data).filter(k=>data[k]!==undefined).map(k=>JSON.stringify(k)+':'+(k==='amount_minor'?amount:JSON.stringify(data[k]))).join(',')+'}';
}
function connectorMode(){const op=connectorForm.connector_operation.value;trust('connector-mandate-fields').hidden=op!=='mandate-create';trust('connector-mandate-label').hidden=op!=='request';trust('connector-action-label').hidden=['mandate-create','request'].includes(op)}
function connectorOptions(select,rows){const old=select.value;select.replaceChildren(...rows.map(([value,label])=>{const option=document.createElement('option');option.value=value;option.textContent=label;return option}));if(rows.some(r=>r[0]===old))select.value=old}
function renderTrustConnectors(){
 connectorOptions(connectorForm.connector_mandate,Object.values(trustView.mandates||{}).filter(m=>m.connector?.id==='local-access-simulation').map(m=>[m.id,m.id+' · '+m.resource+' · '+m.connector.access.subject+' / '+m.connector.access.role]));
 connectorOptions(connectorForm.connector_action,Object.values(trustView.actions||{}).map(a=>[a.id,a.id+' · '+a.resource+' · '+trustTr(({pending_approval:'En attente d’approbation',approved:'Approuvée',committed:'Effet engagé',unknown:'Résultat à réconcilier',reconciled:'Réconciliée'})[a.state]||a.state)]));
 trust('connector-catalog').textContent=JSON.stringify(trustView.connector_catalog||{},null,2);
 const entries=trustView.access_ledger||[];trust('connector-access-list').replaceChildren(...(entries.length?entries:[null]).map(e=>{const li=document.createElement('li');li.textContent=e?e.resource+' · '+e.access.subject+' / '+e.access.role+' · '+e.action_id:trustTr('Aucun accès simulé engagé.');return li}));connectorMode();
}
connectorForm.addEventListener('input',()=>{connectorPending=null;connectorMode()});
connectorForm.addEventListener('submit',async event=>{
 event.preventDefault();const button=trust('connector-submit'),status=trust('connector-status');if(button.disabled)return;button.disabled=true;
 try{
  const op=connectorForm.connector_operation.value;let request;
  if(connectorPending){if(connectorPending.op!==op)throw Error(trustTr('Opération modifiée.'));request=connectorPending.request}
  else{
   request={schema_version:1,event_id:crypto.randomUUID()};
   if(op==='mandate-create')Object.assign(request,{mandate_id:connectorForm.connector_mandate_id.value.trim(),subject:connectorForm.connector_requester.value.trim(),resource:connectorForm.connector_resource.value.trim(),actions:['grant-access'],connector:{id:'local-access-simulation',version:1,access:{subject:connectorForm.connector_subject.value.trim(),role:connectorForm.connector_role.value.trim()}}});
   else if(op==='request'){const m=trustView.mandates[connectorForm.connector_mandate.value];if(!m)throw Error(trustTr('Sélectionnez un mandat.'));Object.assign(request,{mandate_id:m.id,resource:m.resource,connector:m.connector,idempotency_key:crypto.randomUUID()});if(trustView.data_policy?.enabled){const rule=trustView.data_policy.resources.find(r=>r.resource===m.resource);if(!rule)throw Error(trustTr('Ressource non classée.'));request.data_context={classification:rule.classification,destination:'local-access-state',provider:'swarm',tool:'simulated-access-grant'}}}
   else{const a=trustView.actions[connectorForm.connector_action.value];if(!a)throw Error(trustTr('Sélectionnez une action.'));request.action_id=a.id;if(op==='approve')Object.assign(request,{connector:a.connector,content_digest:a.content_digest,mandate_id:a.mandate_id,amount_minor:a.amount_minor,currency:a.currency,beneficiary:a.beneficiary})}
   connectorPending={op,request,amount:op==='approve'?trustView.amount_minor_text[request.action_id]:undefined};
  }
  const result=await trustAPI('/api/v1/trust-actions?action='+op,request,connectorForm.connector_token.value,data=>connectorEncode(data,connectorPending?.amount));
  if(result.state==='denied')throw Error(trustTr(result.reason));
  connectorPending=null;status.textContent=trustTr('Opération simulée enregistrée.');status.className='notice info';await loadTrust();
 }catch(error){status.textContent=error.message;status.className='notice alert';status.focus()}
 finally{button.disabled=false}
});
