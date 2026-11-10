'use strict';
/** Read-only security metadata. The browser neither computes qualification nor
 * executes recipes. Current I3 authentication is repeated by the engine; the
 * bearer and results are cleared on identity edits, navigation and failures. */
(()=>{
 const $=id=>document.getElementById(id),t=s=>globalThis.SwarmI18n?.t(s)||s;
 const language=globalThis.SwarmI18n?.language==='en'?'en':'fr';let generation=0,busy=false;
 const names={OPEN:'Contrôle transversal ouvert',PARTIAL:'Protection partielle',NOT_TESTED:'Non vérifié',unknown:'Fraîcheur inconnue',not_qualified:'Qualification non établie'};
 /** Remove obsolete/private observations before another request or identity. */
 function clear(){generation++;$('security-report').hidden=true;for(const id of ['security-json','security-runtime','security-config','security-currency','security-version','security-count','security-check-state','security-qualification'])$(id).textContent='';$('security-flows').replaceChildren();$('security-recipes').replaceChildren();$('security-binding').replaceChildren();$('security-message').textContent=t('Aucun état de sécurité chargé.');}
 /** Build text nodes only; no report, principal or translated field is HTML. */
 function text(tag,value){const e=document.createElement(tag);e.textContent=value;return e;}
 /** Validate the complete metadata before rendering any section. A missing
  * field is an error, never a fabricated unconfigured or successful state. */
 function validate(v){
  if(v?.schema_version!==1||v.kind!=='security-observation-v1'||!Array.isArray(v.domains)||v.domains.length!==1||!/^[a-f0-9]{64}$/.test(v.observation_digest)||!/^[a-f0-9]{64}$/.test(v.catalogue_sha256)||!/^[a-f0-9]{64}$/.test(v.inventory_sha256))throw Error(t('Réponse invalide'));
  const d=v.domains[0],c=d?.configuration;
  if(d?.id!=='blp'||d.runtime_enforced!==false||typeof c?.configured!=='boolean'||typeof c.authority_current!=='boolean'||typeof c.preparation_quarantined!=='boolean'||typeof v.trust_revision!=='string'||!Array.isArray(d.flows)||!d.flows.length||!Array.isArray(d.recipes)||d.verification?.result!=='NOT_TESTED'||d.verification.freshness!=='unknown'||d.qualification?.state!=='not_qualified')throw Error(t('Réponse invalide'));
  if(d.flows.some(f=>typeof f.id!=='string'||typeof f.name_fr!=='string'||typeof f.name_en!=='string'||typeof f.owner!=='string'||!Array.isArray(f.missing_blp)||!Object.hasOwn(names,f.status))||d.recipes.some(r=>typeof r.id!=='string'||typeof r.path!=='string'))throw Error(t('Réponse invalide'));
  return d;
 }
 /** Render declared inventory and current configuration separately from
  * unestablished verification/qualification. Binary metadata is descriptive. */
 function render(v,d){
  const c=d.configuration;
  $('security-runtime').textContent=t('Protection partielle — confidentialité de bout en bout non établie');
  $('security-config').textContent=t(c.configured?'Politique enregistrée':'Aucune politique de confidentialité enregistrée')+' · '+t(c.preparation_quarantined?'Préparations en quarantaine':'Préparations hors quarantaine');
  $('security-currency').textContent=t(!c.configured?'Aucune révision de politique':c.authority_current?'Politique liée à la révision Trust courante':'Politique à renouveler après changement Trust');
  $('security-version').textContent=t('Version compilée')+' : '+(v.binary?.version||t('Version non attestée'));
  $('security-count').textContent=t('Frontières recensées')+' : '+d.flows.length+' · '+t('Contrôles transversaux ouverts')+' : '+d.flows.filter(f=>f.status==='OPEN').length;
  for(const f of d.flows){const item=document.createElement('details');item.append(text('summary',f.id+' · '+f['name_'+language]+' · '+t(names[f.status])));item.append(text('p',t('Propriétaire')+' : '+f.owner));item.append(text('p',t('Lots restant à raccorder')+' : '+f.missing_blp.join(', ')));$('security-flows').append(item);}
  $('security-check-state').textContent=t(names[d.verification.result])+' · '+t(names[d.verification.freshness]);
  for(const r of d.recipes)$('security-recipes').append(text('li',r.id+' · '+r.path));
  for(const [label,value]of [['Révision Trust',v.trust_revision],['Révision de politique',c.policy_revision||t('Aucune')],['Empreinte du catalogue',v.catalogue_sha256],['Empreinte de l’inventaire',v.inventory_sha256],['Empreinte de l’observation',v.observation_digest]])$('security-binding').append(text('dt',t(label)),text('dd',value));
  $('security-json').textContent=JSON.stringify(v,null,2);$('security-qualification').textContent=t(names[d.qualification.state]);$('security-report').hidden=false;
 }
 /** One explicit readonly request under native CSRF/session and Trust bearer.
  * Never retry; generation checks discard results after token edits/clear. */
 $('security-access').addEventListener('submit',async event=>{
  event.preventDefault();if(busy)return;clear();const own=generation,token=$('security-token').value;busy=true;$('security-refresh').disabled=true;$('security-message').textContent=t('Lecture de l’état de sécurité…');
  try{
   const session=await fetch('/api/v1/native-session',{cache:'no-store'});if(!session.ok)throw Error(t('Session indisponible'));
   const s=await session.json();if(typeof s.csrf_token!=='string'||!s.csrf_token)throw Error(t('Réponse invalide'));if(own!==generation)return;
   const response=await fetch('/api/v1/trust-actions?action=security-show',{method:'POST',headers:{'Content-Type':'application/json','X-Swarm-CSRF':s.csrf_token,Authorization:'Bearer '+token},body:'{"schema_version":1}',cache:'no-store'});
   const v=await response.json();if(own!==generation)return;if(!response.ok)throw Error(v.failure?.code==='authorization_required'?t('Un jeton administrateur I3 courant est requis'):t('Lecture refusée ; aucun résultat affiché'));
   const d=validate(v);render(v,d);$('security-message').textContent=t('Observation reçue ; aucune qualification attribuée');
  }catch(error){if(own===generation){clear();$('security-message').textContent=error instanceof SyntaxError?t('Réponse invalide'):error.message;}}
  finally{busy=false;$('security-refresh').disabled=false;}
 });
 $('security-token').addEventListener('input',clear);
 $('security-clear').addEventListener('click',()=>{$('security-token').value='';clear();$('security-token').focus();});
 $('security-theme').addEventListener('click',()=>{document.documentElement.dataset.theme=document.documentElement.dataset.theme==='sombre'?'etat':'sombre';});
 const query=new URLSearchParams(location.search);if(query.get('theme')==='sombre')document.documentElement.dataset.theme='sombre';
 for(const link of document.querySelectorAll('header a')){const url=new URL(link.href);url.searchParams.set('lang',language);link.href=url.href;}
 addEventListener('pagehide',()=>{$('security-token').value='';clear();});
})();
