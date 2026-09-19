const tr_web_ai_connections_js = source => globalThis.SwarmI18n?.t(source) ?? source;
/* Operator-owned text APIs. Secrets are write-only and never put in storage. */
const AIConnections={state:null,
 async load(){
  const host=$('ai-connections-list');host.textContent=tr_web_ai_connections_js('Chargement des connexions…');
  try{this.state=await api('/api/v1/providers/connections');this.render()}catch(e){host.textContent=e.message}
 },
 render(){
  const host=$('ai-connections-list');host.replaceChildren();
  for(const c of this.state.connections.sort((a,b)=>a.label.localeCompare(b.label))){
   const card=node('article',undefined,'card provider-card');card.dataset.connection=c.id;
   card.append(node('h3',c.label),node('p',c.disabled?tr_web_ai_connections_js('Désactivée'):tr_web_ai_connections_js('Disponible pour les prochains appels'),'notice '+(c.disabled?'attention':'info')),node('p',tr_web_ai_connections_js('Modèle : ')+c.model),node('p',c.base_url,'assist-meta'),node('p',c.has_key?tr_web_ai_connections_js('Clé enregistrée, jamais affichée.'):tr_web_ai_connections_js('Sans clé enregistrée.')),node('p',tr_web_ai_connections_js('Préparation · aide · orchestrateur · vérificateur de rapports. Sans outils de modification.')));
   const edit=button(tr_web_ai_connections_js('Configurer et tester'),()=>this.edit(c));edit.id='connection-edit-'+c.id;card.append(edit);host.append(card)
  }
  if(!this.state.connections.length)host.append(node('p',tr_web_ai_connections_js('Aucune connexion API enregistrée. Ajoutez votre modèle local ou une API compatible.')));
 },
 edit(c=null){
  if(!this.state)return;
  openModal(c?tr_web_ai_connections_js('Configurer — ')+c.label:tr_web_ai_connections_js('Ajouter une IA'),tr_web_ai_connections_js('Connexion compatible avec /chat/completions. Le test envoie uniquement une courte question à l’adresse indiquée ; il peut être facturé par votre fournisseur. Aucun document de mission envoyé.'),{action:'ai-connection',digest:this.state.digest});
  const id=field('connection_id',tr_web_ai_connections_js('Identifiant'),c?.id||'');id.readOnly=!!c;if(c)id.classList.add('connection-readonly');
  field('connection_label',tr_web_ai_connections_js('Nom affiché'),c?.label||'');
  const preset=field('connection_type',tr_web_ai_connections_js('Type de connexion'),'custom',[['custom',tr_web_ai_connections_js('API compatible — adresse personnalisée')],['ollama',tr_web_ai_connections_js('Ollama local')],['vllm',tr_web_ai_connections_js('vLLM / LiteLLM local')]]);
  const address=field('connection_url',tr_web_ai_connections_js('Adresse de base (inclure /v1 si nécessaire)'),c?.base_url||'');address.placeholder='http://localhost:11434/v1';
  preset.onchange=()=>{if(preset.value==='ollama')address.value='http://localhost:11434/v1';if(preset.value==='vllm')address.value='http://localhost:8000/v1'};
  field('connection_model',tr_web_ai_connections_js('Identifiant exact du modèle'),c?.model||'');
  const key=field('connection_key',c?.has_key?tr_web_ai_connections_js('Nouvelle clé — vide pour conserver la clé enregistrée'):tr_web_ai_connections_js('Clé API — facultative pour un serveur local'),'');key.type='password';key.autocomplete='new-password';
  field('connection_key_action',tr_web_ai_connections_js('Clé enregistrée'),'keep',[['keep',tr_web_ai_connections_js('Conserver ou remplacer avec la nouvelle clé')],['remove',tr_web_ai_connections_js('Retirer la clé enregistrée')]]);
  field('connection_enabled',tr_web_ai_connections_js('Disponibilité'),c?.disabled?'no':'yes',[['yes',tr_web_ai_connections_js('Disponible pour les prochains appels')],['no',tr_web_ai_connections_js('Désactivée')]]);
  const info=node('p',tr_web_ai_connections_js('Pour utiliser cette IA, choisissez « api-')+(c?.id||'identifiant')+tr_web_ai_connections_js(' » dans la préparation ou la configuration du responsable/vérificateur. Un exécutant avec outils reste nécessaire pour produire les fichiers.'),'notice info');info.classList.add('connection-wide');$('modal-fields').append(info);
  const result=node('p',tr_web_ai_connections_js('Connexion non testée dans ce formulaire.'),'notice info');result.id='connection-test-result';result.classList.add('connection-wide');result.setAttribute('role','status');
  const ctx=modalContext;
  const test=button(tr_web_ai_connections_js('Tester la connexion'),async()=>{test.disabled=true;result.textContent=tr_web_ai_connections_js('Test en cours…');try{const data=await api('/api/v1/providers/connections/test',this.request());if(modalContext!==ctx)return;result.className='notice '+(data.ok?'success':'alert');result.textContent=data.ok?tr_web_ai_connections_js('Réponse reçue en ')+data.latency_ms+' ms : '+data.text:data.error}catch(e){if(modalContext===ctx){result.className='notice alert';result.textContent=e.message}}finally{if(modalContext===ctx)test.disabled=false}});
  const actions=node('div',undefined,'provider-actions connection-wide');actions.append(test);$('modal-fields').append(actions,result);$('confirm').textContent=tr_web_ai_connections_js('Enregistrer la connexion');
 },
 request(){const c=modalContext;return {version:1,expected_digest:c.digest,replace_key:!!$('field-connection_key').value||$('field-connection_key_action').value==='remove',connection:{id:$('field-connection_id').value.trim(),label:$('field-connection_label').value.trim(),base_url:$('field-connection_url').value.trim().replace(/\/+$/,''),model:$('field-connection_model').value.trim(),key:$('field-connection_key_action').value==='remove'?'':$('field-connection_key').value,disabled:$('field-connection_enabled').value==='no'}}},
 async save(){await api('/api/v1/providers/connections',this.request());await this.load();closeModal();if(typeof loadAssistMeta==='function'){assistMeta=null}notice(tr_web_ai_connections_js('Connexion enregistrée. Les prochains appels peuvent la sélectionner ; les agents déjà démarrés ne changent pas.'));}
};
$('connections-add').onclick=()=>AIConnections.edit();

$('modal').addEventListener('close',()=>{const key=$('field-connection_key');if(key)key.value=''});
