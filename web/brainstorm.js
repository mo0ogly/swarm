'use strict';
const tr_web_brainstorm_js = source => globalThis.SwarmI18n?.t(source) ?? source;

let brainKey='',brainWork='',brainRefs=[],brainSearchOffset=0;
function resetBrainWork(){if(brainWork!==work){brainWork=work;brainRefs=[];brainSearchOffset=0;$('brain-search-results').replaceChildren();$('brain-search-state').textContent='';$('brain-search-next').hidden=true;renderAttachments()}}
function renderBrainstorm(){
 resetBrainWork();renderRetex();renderPlans();
 const turns=snapshot.work.tasks.filter(t=>t.brainstorm);
 const pending=snapshot.agents.some(x=>active(x.agent)&&turns.some(t=>t.id===x.agent.task_id));
 $('brain-send').disabled=pending;
 $('brain-state').textContent=pending?tr_web_brainstorm_js('L’IA prépare sa réponse. Son activité et le bouton Arrêter sont disponibles ci-dessus.'):tr_web_brainstorm_js('Vous pouvez envoyer une question ou adopter une réponse comme brief commun.');
 const key=JSON.stringify([work,turns,snapshot.agents.map(x=>[x.agent.id,x.observed_status,x.agent.activity,x.agent.progress]),snapshot.work.planning_brief]);
 if(key===brainKey)return;brainKey=key;
 const focused=document.activeElement?.closest('[data-brain]')?.dataset.brain;
 $('brain-messages').replaceChildren();
 if(!turns.length)$('brain-messages').append(node('p',tr_web_brainstorm_js('Commencez par votre idée : l’IA explorera les options et proposera un plan APEX.')));
 for(const t of turns){
  const card=node('article',undefined,'card brain-turn');card.dataset.brain=t.id;card.dataset.state=t.response_error?'failed':t.response?'completed':'running';
  card.append(node('h3',tr_web_brainstorm_js('Votre message'),'message-role'),node('p',t.question,'message-question'),node('h3',tr_web_brainstorm_js('Réponse IA · ')+(t.owner||'Planner'),'message-role ai-role'));
  const item=snapshot.agents.find(x=>x.agent.task_id===t.id);
  if((t.answers||[]).length>1){const history=node('details');history.append(node('summary',tr_web_brainstorm_js('Réponses précédentes de cette question')));for(const answer of t.answers.slice(0,-1))history.append(node('p',answer.at),node('pre',answer.text));card.append(history)}
  if(t.response){card.append(node('pre',t.response));if(snapshot.work.planning_brief?.task===t.id&&snapshot.work.planning_brief.text===t.response)card.append(node('span',tr_web_brainstorm_js('Brief adopté'),'badge success'),node('p',tr_web_brainstorm_js('Cette réponse est transmise aux prochains agents.')));else if(!item||!active(item.agent))card.append(button(tr_web_brainstorm_js('Examiner et adopter ce brief'),()=>reviewBrainstorm(t)));}
  else card.append(node('p',t.response_error||tr_web_brainstorm_js('Réponse en préparation…')));
  if(t.plan_brief_hash&&t.response)card.append(button(tr_web_brainstorm_js('Examiner le plan d’action'),()=>reviewActionPlan(t)));
  if(t.response)card.append(button(tr_web_brainstorm_js('Examiner un RETEX issu de cette réponse'),()=>newRetex(t)));
  if(item){card.append(badge(item.observed_status),node('p',item.agent.progress?.detail||item.agent.activity),button(tr_web_brainstorm_js('Activité, arrêt ou relance'),()=>taskDialog(t.id,item.agent.id)));}
  if(t.contexts?.length||item)card.append(button(tr_web_brainstorm_js('Contexte de cette tentative'),()=>reviewSavedContext(t,item)));
  $('brain-messages').append(card);
 }
 const brief=snapshot.work.planning_brief;
 $('brief-current').replaceChildren();
 if(brief){$('brief-current').append(button(tr_web_brainstorm_js('Préparer le plan d’action avec l’IA'),prepareActionPlan));$('brief-current').append(node('p',tr_web_brainstorm_js('Brief adopté et enregistré. Il sera transmis aux prochains agents. Aucune tâche n’est validée par cette adoption.'),'notice success'));const d=node('details');d.append(node('summary',tr_web_brainstorm_js('Brief adopté · transmis aux prochains agents')),node('pre',brief.text));$('brief-current').append(d)}
 if(focused)[...$('brain-messages').children].find(e=>e.dataset.brain===focused)?.querySelector('button')?.focus();
}
async function reviewBrainstorm(t){
 const bytes=new TextEncoder().encode(t.response);const digest=[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(x=>x.toString(16).padStart(2,'0')).join('');
 openModal(tr_web_brainstorm_js('Adopter le brief préparé par l’IA'),tr_web_brainstorm_js('Ce texte sera transmis aux prochains agents. Cette adoption ne crée pas les missions proposées et ne valide aucune tâche.'),{action:'adopt-brief',task:t.id,path:'docs/'+t.id+'-brainstorm.md',digest});preview(t.response);$('confirm').textContent=tr_web_brainstorm_js('Adopter ce brief');
}
$('brain-form').onsubmit=e=>{
 e.preventDefault();
 openModal(tr_web_brainstorm_js('Brainstorming avec une IA · APEX'),tr_web_brainstorm_js('Examinez le contexte, puis confirmez l’envoi. La réponse sera conservée directement dans le dialogue.'),{action:'brainstorm',launchEvent:crypto.randomUUID()});
 field('provider',tr_web_brainstorm_js('IA / fournisseur'),'',Object.keys(snapshot.providers?.providers||{}).sort().map(x=>[x,x]));
 if(typeof addModelFields==='function')addModelFields('brainstorm');
 field('workspace',tr_web_brainstorm_js('Espace à examiner'),snapshot.root);
 field('instruction',tr_web_brainstorm_js('Message à l’IA'),$('brain-question').value,null,true);
 field('capture',tr_web_brainstorm_js('Journal détaillé des sorties'),'false',[['false',tr_web_brainstorm_js('Désactivé')],['true',tr_web_brainstorm_js('Activé — sorties conservées, limite 1 Mio')]]);
 $('confirm').textContent=tr_web_brainstorm_js('Examiner le contexte');
 $('modal-fields').addEventListener('input',()=>{if(modalContext?.action==='brainstorm'){modalContext.contextHash=null;$('confirm').textContent=tr_web_brainstorm_js('Examiner le contexte')}},{once:true});
 if(snapshot.providers_error||!Object.keys(snapshot.providers?.providers||{}).length){$('modal-error').textContent=snapshot.providers_error||tr_web_brainstorm_js('Aucun fournisseur configuré.');$('modal-error').hidden=false;$('confirm').disabled=true;}
};

setInterval(()=>{if(view==='brainstorm'&&!document.hidden)refresh()},2000);

$('brain-export').onclick=()=>{
 const turns=snapshot.work.tasks.filter(t=>t.brainstorm);
 let text='# Dialogue IA — '+snapshot.work.title+tr_web_brainstorm_js('\n\nTravail : ')+work+'\n';
 for(const t of turns){text+=tr_web_brainstorm_js('\n## Vous\n\n')+t.question+'\n';const answers=t.answers?.length?t.answers:[{text:t.response||t.response_error||tr_web_brainstorm_js('Réponse en attente'),at:''}];for(const answer of answers)text+=tr_web_brainstorm_js('\n## IA — ')+(t.owner||'planner')+' '+answer.at+'\n\n'+answer.text+'\n';}
 if(snapshot.work.planning_brief)text+=tr_web_brainstorm_js('\n## Brief adopté\n\n')+snapshot.work.planning_brief.text+'\n';
 const a=node('a');a.href=URL.createObjectURL(new Blob([text],{type:'text/markdown;charset=utf-8'}));a.download='swarm-dialogue-'+work+'.md';a.click();setTimeout(()=>URL.revokeObjectURL(a.href),1000);
};

$('brain-retex').onclick=()=>{
 $('brain-question').value=tr_web_brainstorm_js('Fais un RETEX à partir de nos échanges, du brief adopté et des preuves accessibles dans le périmètre. Sépare faits démontrés, hypothèses et informations manquantes. Pour chaque enseignement : constat, référence de preuve, cause démontrée ou supposée, action proposée et critère de vérification. Propose au maximum trois quick wins avec bénéfice attendu, effort qualitatif provisoire, risque et résultat observable. Classe-les comme proposés, jamais comme déjà réalisés ou confirmés. Termine par les questions utiles. Ne modifie pas le Guide de terrain et ne lance aucun agent de réalisation.');
 $('brain-question').focus();$('brain-question').scrollIntoView({block:'center',behavior:'smooth'});
};

async function submitBrainstorm(f,c){
 const payload={level:f.level,model_policy_hash:f.model_policy_hash,plan_brief_hash:c.planBriefHash||'',provider:f.provider,workspace:f.workspace,instruction:f.instruction,capture_output:f.capture==='true',references:brainRefs,event_id:c.launchEvent};
 const signature=JSON.stringify(payload);
 if(!c.contextHash||c.signature!==signature){
  const a=await act('context-preview',payload);c.contextHash=a.context.sha256;c.signature=signature;
  preview(tr_web_brainstorm_js('Modèle choisi : ')+routeText(a.model_route)+tr_web_brainstorm_js('\nCONTEXTE TRANSMIS · ')+a.context.bytes+tr_web_brainstorm_js(' octets\nÉchanges récents inclus : ')+a.context.included.map(id=>snapshot.work.tasks.find(t=>t.id===id)?.question||id).join(' ; ')+tr_web_brainstorm_js('\nHors fenêtre récente (sauf pièce jointe explicite) : ')+(a.context.excluded.map(id=>snapshot.work.tasks.find(t=>t.id===id)?.question||id).join(' ; ')||'aucun')+tr_web_brainstorm_js('\nPièces jointes : ')+brainRefs.length+tr_web_brainstorm_js('\nChemins à consulter, contenu non joint : ')+(a.context.paths||[]).join(', ')+'\n\nPROMPT EXACT\n'+a.prompt);
  $('confirm').textContent=tr_web_brainstorm_js('Confirmer l’envoi à l’IA');return false;
 }
 await act('brainstorm',{...payload,context_hash:c.contextHash});$('brain-question').value='';brainRefs=[];renderAttachments();return true;
}
function renderAttachments(){
 $('brain-attachments').replaceChildren();
 for(const ref of brainRefs)$('brain-attachments').append(button(tr_web_brainstorm_js('Retirer : ')+(snapshot.work.tasks.find(t=>t.id===ref.task)?.question||ref.task).slice(0,90),()=>{brainRefs=brainRefs.filter(r=>r!==ref);renderAttachments()}));
}
async function searchBrain(reset){
 if(reset)brainSearchOffset=0;const requestedWork=work;const query=$('brain-search').value.trim();
 $('brain-search-state').textContent=query?tr_web_brainstorm_js('Recherche en cours…'):tr_web_brainstorm_js('Saisissez un mot ou une expression.');$('brain-search-results').replaceChildren();$('brain-search-next').hidden=true;if(!query)return;
 try{const data=await act('dialogue-search',{note:query,offset:brainSearchOffset});if(work!==requestedWork||query!==$('brain-search').value.trim())return;
 $('brain-search-state').textContent=data.total?data.total+tr_web_brainstorm_js(' réponse(s) trouvée(s) dans ce travail.'):tr_web_brainstorm_js('Aucun résultat dans ce travail.');
 for(const h of data.hits){const card=node('article',undefined,'card');card.append(node('h3',h.question),node('p',h.at||tr_web_brainstorm_js('Date non disponible')),node('pre',h.text.slice(0,400)),button(tr_web_brainstorm_js('Lire la réponse source'),()=>{openModal(tr_web_brainstorm_js('Échange source'),h.ref.task+' · '+h.ref.attempt,{action:'source-read'});preview(h.question+'\n\n'+h.text);$('confirm').hidden=true}),button(tr_web_brainstorm_js('Joindre au prochain message'),()=>{if(!brainRefs.some(r=>JSON.stringify(r)===JSON.stringify(h.ref))){if(brainRefs.length>=8){notice(tr_web_brainstorm_js('Huit échanges joints maximum.'),true);return}brainRefs.push(h.ref)}renderAttachments();notice(tr_web_brainstorm_js('Échange joint. Il sera visible dans l’aperçu avant envoi.'))}));$('brain-search-results').append(card)}
 brainSearchOffset=data.next;$('brain-search-next').hidden=data.next>=data.total;
 }catch(e){$('brain-search-state').textContent=tr_web_brainstorm_js('Recherche impossible : ')+e.message}
}
$('brain-search-form').addEventListener('submit',e=>{e.preventDefault();searchBrain(true)});
$('brain-search-next').addEventListener('click',()=>searchBrain(false));

function reviewSavedContext(t,item){
 const contexts=t.contexts?.length?t.contexts:[{agent:item.agent.id,at:item.agent.started,prompt:item.agent.prompt}];
 openModal(tr_web_brainstorm_js('Contexte transmis à cette tentative'),tr_web_brainstorm_js('Copie enregistrée à l’envoi, conservée avec la conversation.'),{action:'context-read'});
 const latest=contexts[contexts.length-1];const select=field('context',tr_web_brainstorm_js('Tentative'),latest.agent,contexts.map(c=>[c.agent,c.at+' · '+c.agent]));
 preview(latest.prompt);select.addEventListener('change',()=>preview(contexts.find(c=>c.agent===select.value).prompt));$('confirm').hidden=true;
}
