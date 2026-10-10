const tr_web_ai_connections_js = source => globalThis.SwarmI18n?.t(source) ?? source;
/* Operator-owned text APIs. Secrets are write-only and never put in storage. */
const AIConnections={state:null,
 responseText(text){
  // Decode only valid JSON replies; plain text and literal backslashes stay intact.
  try{
   const reply=JSON.parse(text);
   if(typeof reply==='string')return reply;
   if(reply&&typeof reply.message==='string')return [reply.message,typeof reply.brief==='string'?reply.brief:''].filter(Boolean).join('\n\n');
  }catch{}
  return text;
 },
 async load(){
  const host=$('ai-connections-list'),add=$('connections-add');add.disabled=true;host.textContent=tr_web_ai_connections_js('Chargement des connexions…');
  try{this.state=await api('/api/v1/providers/connections');this.render();add.disabled=false}catch(e){host.textContent=e.message}
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
  const debug=node('section',undefined,'connection-wide connection-debug');
  debug.append(node('h3',tr_web_ai_connections_js('Console de diagnostic')));
  const log=node('pre',tr_web_ai_connections_js('Aucun test exécuté.'),'connection-debug-log');log.id='connection-debug-log';log.tabIndex=0;log.setAttribute('aria-label',tr_web_ai_connections_js('Console de diagnostic'));debug.append(log);
  const copyStatus=node('p','','hint');copyStatus.id='connection-debug-copy-status';copyStatus.setAttribute('role','status');
  const copy=button(tr_web_ai_connections_js('Copier le diagnostic'),async()=>{try{await navigator.clipboard.writeText(log.textContent);copyStatus.textContent=tr_web_ai_connections_js('Diagnostic copié.')}catch{copyStatus.textContent=tr_web_ai_connections_js('Copie impossible : sélectionnez et copiez le texte affiché.')}});copy.id='connection-debug-copy';
  const copyActions=node('div',undefined,'provider-actions');copyActions.append(copy);debug.append(copyActions,copyStatus);
  const ctx=modalContext;
  const test=button(tr_web_ai_connections_js('Tester la connexion'),async()=>{test.disabled=true;log.textContent=tr_web_ai_connections_js('Navigateur → serveur Swarm : test en cours (60 s maximum)…');result.textContent=tr_web_ai_connections_js('Test en cours…');try{const data=await api('/api/v1/providers/connections/test',this.request());if(modalContext!==ctx)return;log.textContent=[tr_web_ai_connections_js('Diagnostic serveur Swarm'),...(data.diagnostics||[]).map(e=>'['+e.elapsed_ms+' ms] '+e.stage+' : '+(globalThis.SwarmI18n?.engine(e.message)??e.message)),tr_web_ai_connections_js('Durée totale : ')+data.latency_ms+' ms',data.ok?tr_web_ai_connections_js('Connexion réussie.'):(globalThis.SwarmI18n?.engine(data.error)??data.error)].join('\n');result.className='notice '+(data.ok?'success':'alert');result.textContent=data.ok?tr_web_ai_connections_js('Réponse reçue en ')+data.latency_ms+' ms : '+this.responseText(data.text):(globalThis.SwarmI18n?.engine(data.error)??data.error)}catch(e){if(modalContext===ctx){result.className='notice alert';result.textContent=e.message;log.textContent=tr_web_ai_connections_js('Navigateur → serveur Swarm : échec. Vérifiez la requête /api/v1/providers/connections/test dans les outils réseau du navigateur.')+'\n'+e.message}}finally{if(modalContext===ctx)test.disabled=false}});
  test.id='connection-test';
  const actions=node('div',undefined,'provider-actions connection-wide');actions.append(test);$('modal-fields').append(actions,result,debug);$('confirm').textContent=tr_web_ai_connections_js('Enregistrer la connexion');
 },
 request(){const c=modalContext;return {version:1,expected_digest:c.digest,replace_key:!!$('field-connection_key').value||$('field-connection_key_action').value==='remove',connection:{id:$('field-connection_id').value.trim(),label:$('field-connection_label').value.trim(),base_url:$('field-connection_url').value.trim().replace(/\/+$/,''),model:$('field-connection_model').value.trim(),key:$('field-connection_key_action').value==='remove'?'':$('field-connection_key').value,disabled:$('field-connection_enabled').value==='no'}}},
 async save(){await api('/api/v1/providers/connections',this.request());await this.load();closeModal();if(typeof loadAssistMeta==='function'){assistMeta=null}notice(tr_web_ai_connections_js('Connexion enregistrée. Les prochains appels peuvent la sélectionner ; les agents déjà démarrés ne changent pas.'));}
};
$('connections-add').onclick=()=>AIConnections.edit();

$('modal').addEventListener('close',()=>{const key=$('field-connection_key');if(key)key.value=''});
