'use strict';
// Page assistant — coordinates out, grounded answer in.
// The browser never sends state: it sends a page id, the selected entity and the
// visible slice. The engine rebuilds the facts, so a stale tab cannot ground an
// answer and no screen can write straight into a prompt.
let assistEpoch=0,assistHistoryReady=false;
let assistLastRender='',assistSelectedTurn='',assistMeta=null,assistWork='',assistTurns=[],assistPending=false,assistBusy=false,assistPage='tasks',assistLoaded=false;
const assistPageOf=name=>['brainstorm','tasks','agents','decisions','logs','resume','budget'].includes(name)?name:'tasks';
function assistSpec(id){return (assistMeta?.pages||[]).find(p=>p.id===id)||{id,title:id,purpose:''}}
function assistCoordinates(){
 const page=assistPage,target=$('assist-target').value||'';
 const c={page_id:page,count:20};
 if(page==='logs'){const agent=$('log-agent').value;if(agent)c.selected=[agent];const applied=logPage?.coordinates?.agent===agent?logPage.coordinates:null;const q=applied?applied.q:$('log-search').value.trim();if(q)c.filter=q;c.kind=applied?applied.kind:$('log-kind').value.trim();c.offset=Math.max(0,logPage?.after_cursor??logCursor??0)}
 else if(target)c.selected=[target];
 return c;
}
function assistTargets(){
 const page=assistPage,out=[['','Toute la page']];
 if(!snapshot)return out;
 if(page==='tasks')for(const t of snapshot.work.tasks.filter(t=>!t.brainstorm))out.push([t.id,t.title+' — '+(labels[snapshot.validation.tasks[t.id]?.state]||t.status)]);
 if(page==='agents')for(const x of snapshot.agents)out.push([x.agent.id,x.agent.provider+' · '+x.agent.task_id+' · '+(labels[x.observed_status]||x.observed_status)]);
 if(page==='decisions')for(const d of snapshot.decisions)out.push([d.id,d.task_id+' · '+d.kind+' · '+(d.resolved_at?'acquittée':'à traiter')]);
 if(page==='brainstorm')for(const t of snapshot.work.tasks.filter(t=>t.brainstorm))out.push([t.id,(t.question||t.id).slice(0,90)]);
 if(page==='logs')return [['','Tentative sélectionnée dans la page Journaux']];
 return out;
}
function syncAssistantPage(name){
 assistPage=assistPageOf(name);const spec=assistSpec(assistPage);
 $('assist-page').textContent=spec.title;$('assist-purpose').textContent=spec.purpose||'';
 const previous=$('assist-target').dataset.page===assistPage?$('assist-target').value:'';$('assist-target').dataset.page=assistPage;selectOptions($('assist-target'),assistTargets(),previous);
 $('assist-target').disabled=!['tasks','agents','decisions','brainstorm'].includes(assistPage);
 if(assistLoaded)renderAssistPanel();
}
async function loadAssistMeta(){if(assistMeta)return assistMeta;assistMeta=await api('/api/v1/assist/meta');selectOptions($('assist-template'),assistMeta.templates.map(t=>[t.id,t.label]),'understand_page.v1');return assistMeta}
async function loadAssistTurns(){
 if(!work||assistBusy)return;assistBusy=true;const requested=work,epoch=assistEpoch;
 try{const data=await api('/api/v1/assist/turns?work='+encodeURIComponent(work));if(work!==requested||epoch!==assistEpoch)return;assistTurns=data.turns||[];assistHistoryReady=true;assistPending=assistTurns.some(t=>t.status==='pending'||t.status==='running');renderAssistPanel()}
 catch(e){$('assist-state').textContent='Historique de l’assistant indisponible : '+e.message;$('assist-state').className='notice alert'}
 finally{assistBusy=false}
}
function renderAssistant(){
 if(!assistLoaded)return;
 if(assistWork!==work){assistEpoch++;assistWork=work;assistHistoryReady=false;assistTurns=[];assistPending=false;assistLastRender='';assistSelectedTurn='';$('assist-answer').replaceChildren();$('assist-history').replaceChildren();loadAssistTurns()}
 else loadAssistTurns();
 syncAssistantPage(view);
}
function assistStale(turn){return Boolean(turn.stale||snapshot&&turn.context&&turn.context.revision!==snapshot.work.revision)}
function assistRefChip(turn,id){
 const fact=(turn.context.facts||[]).find(f=>f.id===id);
 const b=button(id,()=>openAssistFact(turn,fact,id));b.className='assist-ref';
 b.title=fact?fact.name+' · '+fact.source:'Référence absente du contexte';
 return b;
}
function openAssistFact(turn,fact,id){
 openModal('Référence '+id,fact?'Fait transmis à l’IA, lu par le moteur dans l’état enregistré.':'Cette référence n’existe pas dans le contexte envoyé.',{action:'assist-ref'});
 preview(fact?['Identifiant : '+fact.id,'Nom : '+fact.name,'Source : '+fact.source,'Nature : '+fact.kind,'','Valeur :',fact.value].join('\n'):'Référence inconnue : '+id);
 $('confirm').hidden=true;
 if(fact&&assistOpenSource(fact.source,true))$('modal-fields').append(button('Ouvrir cet élément dans le cockpit',()=>{closeModal();assistOpenSource(fact.source,false)}));
}
// References resolve to the cockpit's own screens; nothing is re-implemented.
function assistOpenSource(source,probe){
 const [kind,id]=String(source||'').split('/');
 const agent=kind==='agent'||kind==='log'?snapshot?.agents.find(x=>x.agent.id===id):null;
 if(kind==='task'&&snapshot?.work.tasks.some(t=>t.id===id)){if(!probe)taskDialog(id);return true}
 if(kind==='validation'&&snapshot?.work.tasks.some(t=>t.id===id)){if(!probe)taskDialog(id);return true}
 if(kind==='gate'&&snapshot?.work.tasks.some(t=>t.id===id)){if(!probe)taskDialog(id).then(()=>taskFields('gate'));return true}
 if(agent){if(!probe)taskDialog(agent.agent.task_id,agent.agent.id);return true}
 if(kind==='decision'){if(!probe)showView('decisions');return true}
 if(kind==='budget'){if(!probe)showView('budget');return true}
 if(kind==='resume'||kind==='visit'){if(!probe)showView('resume');return true}
 if(kind==='brief'){if(!probe)showView('brainstorm');return true}
 return false;
}
// A proposed action only ever opens the existing form, with its own confirmation.
function assistRunAction(actionID,target){
 actionID=String(actionID).split(':')[0];
 const open=(task,fields)=>taskDialog(task,actionID.startsWith('agent.')?target:undefined).then(()=>{if(modalContext?.data&&fields)taskFields(fields)});
 const agent=snapshot?.agents.find(x=>x.agent.id===target);
 switch(actionID){
  case 'task.review':return taskDialog(target).then(()=>{if(modalContext?.data){const text=modalContext.data.review;taskFields('report');preview(text)}});
  case 'task.reopen':return open(target,'todo');
  case 'task.submit':return open(target,'submit');
  case 'task.gate':return open(target,'gate');
  case 'task.start':return open(target,'start');
  case 'agent.stop':return agent&&open(agent.agent.task_id,'stop');
  case 'agent.retry':return agent&&open(agent.agent.task_id,'retry');
  case 'agent.reconcile':return agent&&open(agent.agent.task_id,'reconcile');
  case 'decision.revalidate':{const d=snapshot.decisions.find(d=>d.id===target);if(!d)return;return taskDialog(d.task_id).then(()=>{if(modalContext?.data){taskFields(['accepted','waived'].includes(modalContext.data.task.status)?'todo':'report');preview(d.evidence)}})}
  case 'decision.acknowledge':showView('decisions');[...document.querySelectorAll('[data-decision]')].find(x=>x.dataset.decision===target)?.querySelector('button')?.click();return;
  case 'work.ooda':$('ooda').click();return;
  case 'work.visit':showView('resume');$('visit').focus();return;
  case 'budget.set':$('budget-edit').click();return;
  case 'brainstorm.ask':showView('brainstorm');$('brain-question').focus();return;
  case 'brainstorm.adopt':showView('brainstorm');return;
 }
 notice('Action inconnue refusée : '+actionID,true);
}
function assistBlock(title,items){
 if(!items.length)return null;
 const box=node('div',undefined,'assist-block');box.append(node('h4',title));const list=node('ul');
 for(const item of items)list.append(item);box.append(list);return box;
}
function renderAssistAnswer(turn){
 const host=$('assist-answer');host.replaceChildren();
 if(!turn)return;
 const head=node('div',undefined,'assist-block');
 head.append(node('h4','Réponse — '+(assistMeta?.templates.find(t=>t.id===turn.template_id)?.label||turn.template_id)));
 head.append(node('p','Page : '+assistSpec(turn.context.page_id).title+' · fournisseur : '+turn.provider+' · '+(turn.ended_at||turn.created_at),'assist-meta'));
 head.append(node('p','Contrat : '+turn.context.contract+' · consignes : '+turn.prompt_version+' · empreinte de contexte : '+String(turn.context.context_hash).slice(0,16)+'… · révision '+turn.context.revision,'assist-meta'));
 if(turn.format_repaired)head.append(node('p','Format réparé une fois avant validation ; aucune seconde réparation n’est tentée.','notice attention'));
 if(assistStale(turn))head.append(node('p','Réponse périmée : les faits, preuves ou capacités ont changé depuis cette question. Reposer la question avant d’agir.','notice attention'));
 host.append(head);
 if(turn.status==='pending'||turn.status==='running'){host.append(node('p','Question envoyée ; réponse en préparation. Aucune action n’est exécutée automatiquement.','notice info'));return}
 if(turn.refusal){
  const kind=turn.refusal.kind==='service'?'Service indisponible':turn.refusal.kind==='preuve'?'Preuve manquante ou référence refusée':'Réponse non conforme';
  const box=node('div',undefined,'notice alert');
  box.append(node('strong',kind+' — '+turn.refusal.code),node('p',turn.refusal.message));
  if(turn.refusal.detail)box.append(node('pre',turn.refusal.detail));
  box.append(node('p',turn.refusal.kind==='service'?'Aucune conclusion ne peut être tirée de cette indisponibilité.':'La réponse entière est refusée : aucune référence inconnue ni action hors catalogue n’est affichée.'));
  host.append(box);return;
 }
 if(turn.model_route)host.append(node('p','Modèle utilisé : '+routeText(turn.model_route),'notice info'));
 const a=turn.answer;if(!a)return;
 host.append(assistBlock('Faits',(a.facts||[]).map(f=>{const li=node('li');li.append(node('span',f.text));const refs=node('span',undefined,'assist-refs');for(const id of f.source_ids||[])refs.append(assistRefChip(turn,id));li.append(refs);return li})));
 if(a.interpretation){const box=node('div',undefined,'assist-block');box.append(node('h4','Interprétation'),node('p',a.interpretation));host.append(box)}
 const verify=[...(a.missing_information||[]).map(x=>'Manque : '+x),...(a.limitations||[]).map(x=>'Limite : '+x),...(a.questions||[]).map(x=>'Question : '+x)];
 const verifyBlock=assistBlock('À vérifier',verify.map(x=>node('li',x)));if(verifyBlock)host.append(verifyBlock);
 if((a.next_steps||[]).length){
  const box=node('div',undefined,'assist-block');box.append(node('h4','Actions possibles'));
  const steps=node('div',undefined,'assist-steps');
  for(const step of a.next_steps){
   const action=(turn.context.actions||[]).find(x=>x.id===step.action_id);
   const row=node('div',undefined,'assist-step');
   const text=node('p',(action?action.label:step.action_id)+' — '+step.why);
   const refs=node('span',undefined,'assist-refs');for(const id of step.source_ids||[])refs.append(assistRefChip(turn,id));
   text.append(refs);row.append(text);
   const launch=button('Ouvrir le formulaire',()=>assistRunAction(step.action_id,action?.target||''));launch.disabled=assistStale(turn);row.append(launch);
   steps.append(row);
  }
  box.append(steps);box.append(node('p','Aucune action n’est exécutée par l’assistant : le formulaire habituel s’ouvre et demande votre confirmation.','assist-meta'));
  host.append(box);
 }
 if(!(a.next_steps||[]).length)host.append(node('p','Aucune action proposée dans le catalogue autorisé de cette page.','assist-meta'));
}
function renderAssistPanel(){
 const key=JSON.stringify([work,assistPage,assistTurns,assistSelectedTurn,assistHistoryReady,assistPending?Math.floor(Date.now()/5000):0]);if(key===assistLastRender)return;assistLastRender=key;
 const relevant=assistTurns.filter(t=>t.context.page_id===assistPage);const last=relevant.find(t=>t.id===assistSelectedTurn)||relevant[relevant.length-1]||null;
 const state=$('assist-state');
 if(!assistHistoryReady){state.className='notice info';state.textContent='Chargement de l’historique des questions de ce travail…'}
 else if(!last&&!assistPending){state.className='notice info';state.textContent='Aucune question posée sur cette page de ce travail.'}
 else if(assistPending){state.className='notice attention';const pending=assistTurns.find(t=>['pending','running'].includes(t.status));const age=Math.max(0,Math.floor((Date.now()-Date.parse(pending?.created_at||''))/1000));state.textContent='Question en cours · '+(pending?.provider||'IA')+' · '+assistSpec(pending?.context.page_id).title+' · '+(Number.isFinite(age)?age:0)+' s écoulées · délai maximal '+(pending?.timeout_seconds||300)+' s. Une seule question à la fois.'}
 else if(last?.refusal){state.className='notice attention';state.textContent='Dernière réponse refusée : '+last.refusal.message}
 else if(last&&assistStale(last)){state.className='notice attention';state.textContent='Réponse historique : les faits ont changé. Reposer la question sur le contexte actuel.'}
 else{state.className='notice success';state.textContent='Structure et références contrôlées ; interprétation à relire ('+assistSpec(last.context.page_id).title+').'}
 $('assist-ask').disabled=assistPending||!assistHistoryReady;$('assist-cancel').hidden=!assistPending;
 renderAssistAnswer(last);
 const host=$('assist-history');const opened=new Set([...host.querySelectorAll('details[open]')].map(x=>x.dataset.turn));host.replaceChildren();
 for(const turn of [...assistTurns].reverse()){
  const d=node('details');d.dataset.turn=turn.id;d.open=opened.has(turn.id);
  d.append(node('summary',(turn.created_at||'')+' · '+assistSpec(turn.context.page_id).title+' · '+turn.template_id+' · '+turn.status+(assistStale(turn)?' · périmée':'')));
  d.append(node('p','Question : '+turn.question));
  d.append(node('p','Fournisseur : '+turn.provider+' · consignes '+turn.prompt_version+' · contrat '+turn.context.contract+' · révision '+turn.context.revision,'assist-meta'));
  if(turn.usage)d.append(node('p','Jetons déclarés : '+turn.usage.usage.input_tokens+' entrée / '+turn.usage.usage.output_tokens+' sortie. '+turn.usage.note,'assist-meta'));
  d.append(button('Revoir le contexte transmis',()=>{openModal('Contexte transmis','Copie exacte enregistrée à l’envoi.',{action:'assist-context'});preview(JSON.stringify(turn.context,null,1));$('confirm').hidden=true}));
  d.append(button('Revoir le prompt exact',()=>{openModal('Prompt transmis','Octets exacts envoyés au fournisseur.',{action:'assist-prompt'});preview(turn.prompt);$('confirm').hidden=true}));
  d.append(button('Réafficher cette réponse',()=>{assistSelectedTurn=turn.id;showView(turn.context.page_id);renderAssistAnswer(turn)}));
  host.append(d);
 }
}
function openAssistAsk(){
 if(!snapshot)return;
 const spec=assistSpec(assistPage);
 openModal('Assistant IA — '+spec.title,'Choisissez le gabarit et l’IA, puis examinez le contexte exact avant envoi. Aucune action ne sera exécutée par la réponse.',{action:'assist',work,revision:snapshot.work.revision,page:assistPage,launchEvent:crypto.randomUUID(),coordinates:assistCoordinates()});
 field('provider','IA / fournisseur','',Object.keys(snapshot.providers?.providers||{}).sort().map(x=>[x,x]));
 const providerOptions=[...$('field-provider').options];for(const option of providerOptions){const reason=assistMeta?.provider_support?.[option.value];if(reason){option.disabled=true;option.textContent+=' — adaptateur non disponible'}}const firstProvider=providerOptions.find(o=>!o.disabled);if(firstProvider)$('field-provider').value=firstProvider.value;
 field('template','Gabarit',$('assist-template').value,(assistMeta?.templates||[]).map(t=>[t.id,t.label]));
 const chosen=(assistMeta?.templates||[]).find(t=>t.id===$('assist-template').value);
 field('question','Votre question',chosen?chosen.question:'',null,true);
 if(typeof addModelFields==='function')addModelFields('page');
 $('confirm').textContent='Examiner le contexte';
 $('modal-fields').addEventListener('input',()=>{if(modalContext?.action==='assist'){modalContext.contextHash=null;$('confirm').textContent='Examiner le contexte'}},{once:true});
 if(snapshot.providers_error||!firstProvider){$('modal-error').textContent=snapshot.providers_error||'Aucun adaptateur compatible configuré pour cette aide. Claude ou Codex requis.';$('modal-error').hidden=false;$('confirm').disabled=true}
}
async function submitAssist(f,c){
 if(c.work!==work)throw new Error('Travail changé : rouvrir la question.');
 const request={event_id:c.launchEvent,expected_revision:c.revision,coordinates:c.coordinates,template_id:f.template,question:f.question,provider:f.provider,level:f.level};
 const signature=JSON.stringify(request);
 if(!c.contextHash||c.signature!==signature){
  const p=await api('/api/v1/assist/ask',{work,preview:true,request});
  if(modalContext!==c||work!==c.work)return false;
  c.contextHash=p.turn.context.context_hash;c.signature=signature;
  const ctx=p.turn.context;
  preview(['CONTEXTE RECONSTRUIT PAR LE MOTEUR — '+p.prompt_bytes+' octets',
   'Modèle choisi : '+routeText(p.turn.model_route),
   'Page : '+ctx.page_title+' · révision '+ctx.revision+' · empreinte '+ctx.context_hash,
   'Éléments sélectionnés : '+(ctx.selected_entities.join(', ')||'aucun')+' · tranche visible '+ctx.visible_slice.offset+'→'+(ctx.visible_slice.offset+ctx.visible_slice.count)+' sur '+ctx.visible_slice.total,
   'Délai maximal : '+p.turn.timeout_seconds+' s · lancement sans outils dans un répertoire temporaire isolé',
   'Faits transmis : '+ctx.facts.length+' · actions autorisées : '+ctx.actions.filter(a=>a.available).map(a=>a.id).join(', '),
   'Omissions : '+(ctx.omissions.join(' | ')||'aucune'),
   'Données manquantes déclarées : '+(ctx.missing.join(' | ')||'aucune'),
   p.injection_warning?'ALERTE : '+p.injection_warning:'',
   '','PROMPT EXACT','',p.prompt].filter(Boolean).join('\n'));
  $('confirm').textContent='Confirmer l’envoi à l’IA';return false;
 }
 assistEpoch++;
 const turn=await api('/api/v1/assist/ask',{work,preview:false,request:{...request,context_hash:c.contextHash}});
 if(c.work!==work)return true;
 assistEpoch++;assistHistoryReady=true;
 assistTurns=[...assistTurns.filter(t=>t.id!==turn.id),turn];assistPending=true;renderAssistPanel();
 return true;
}
$('assist-ask').onclick=openAssistAsk;
$('assist-template').onchange=()=>{};
$('assist-context').onclick=async()=>{
 const requested=work;try{
 const ctx=await api('/api/v1/assist/context?'+new URLSearchParams({work,coordinates:JSON.stringify(assistCoordinates())}));if(requested!==work)return;
 openModal('Contexte exact de cette page','Reconstruit par le moteur ; rien n’est envoyé à une IA.',{action:'assist-context'});
 preview(JSON.stringify(ctx,null,1));$('confirm').hidden=true;
 }catch(e){notice(e.message,true)}
};
$('assist-cancel').onclick=()=>{const t=assistTurns.find(t=>['pending','running'].includes(t.status));if(!t)return;openModal('Arrêter la question en cours','La réponse restera marquée interrompue. Aucune relance automatique ; une consommation peut déjà avoir eu lieu.',{action:'assist-cancel',work,turn:t.id});preview(t.question);$('confirm').textContent='Confirmer l’arrêt'};
$('assist-export').onclick=async()=>{try{const data=await api('/api/v1/assist/export?work='+encodeURIComponent(work));const url=URL.createObjectURL(new Blob([JSON.stringify(data,null,2)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download='swarm-assistant-'+work+'.json';a.click();URL.revokeObjectURL(url)}catch(e){notice(e.message,true)}};
document.addEventListener('click',e=>{const b=e.target.closest('[data-assist-page]');if(!b)return;showView(b.dataset.assistPage);syncAssistantPage(b.dataset.assistPage);openAssistAsk()});
(async()=>{try{await loadAssistMeta();assistLoaded=true;syncAssistantPage(view);if(work)await loadAssistTurns()}catch(e){$('assist-state').textContent='Assistant indisponible : '+e.message;$('assist-state').className='notice alert'}})();
setInterval(()=>{if(work&&assistLoaded)loadAssistTurns()},2000);
