'use strict';
const MissionHelp={
 async proposal(turn,step,index,host,current){
  const action=turn.context.actions?.find(a=>a.id===step.action_id);if(!action)return;
  const row=node('section',undefined,'mission-help-proposal');row.append(node('h3','Action conseillée'),node('p',action.label));
  const state=node('p','Vérification de cette action…');state.setAttribute('role','status');row.append(state);host.append(row);
  const coordinates={work:turn.work_id||work,turn:turn.id,step:index};
  try{
   const plan=await api('/api/v1/action',{kind:'assist-action-preview',...coordinates});if(!current())return;
   state.textContent=plan.effect;
   const apply=Pilot.command('Appliquer cette proposition',async()=>{
    if(apply.disabled||!current())return;apply.disabled=true;state.textContent='Application en cours…';
    try{const result=await api('/api/v1/action',{kind:'assist-action-apply',...coordinates,plan_hash:plan.id});if(!current())return;row.dataset.result=result.status;state.textContent=result.message;apply.textContent=result.status==='done'?'Action effectuée':'Vérifier le résultat';await refresh(true)}
    catch(e){if(current()){row.dataset.result='attention';state.textContent='Résultat non confirmé : '+e.message+'. Consultez la tâche avant de recommencer.';apply.textContent='Relire le résultat';apply.disabled=false}}
   },'primary');row.append(apply);
  }catch(e){if(!current())return;state.textContent=e.message;row.append(Pilot.command('Ouvrir les actions de la tâche',()=>{closeModal();assistRunAction(step.action_id,action.target)}))}
 },
 async open(taskID){
  const requested=work,t=snapshot.work.tasks.find(t=>t.id===taskID);if(!t)return;
  openModal('Aide IA — '+t.title,'Le but de la tâche, ce qui se passe et la prochaine étape, en quelques mots.',{action:'help'});
  const context=modalContext;$('confirm').hidden=true;$('cancel').textContent='Fermer l’aide';
  const host=node('section',undefined,'mission-help');host.id='mission-help';
  const state=node('p','Préparation de l’explication…','notice info');state.setAttribute('role','status');
  const answer=node('div');answer.id='mission-help-answer';answer.setAttribute('aria-live','polite');
  const settings=node('details');settings.append(node('summary','IA utilisée et question posée'));
  const label=node('label','Fournisseur'),provider=node('select');provider.setAttribute('aria-label','Fournisseur de l’aide IA');label.append(provider);
  const questionLabel=node('label','Votre question'),question=node('textarea');question.setAttribute('aria-label','Question sur cette tâche');question.rows=5;
  question.value='Explique cette tâche en deux ou trois phrases courtes, en français courant : à quoi elle sert, ce qui s’est passé et ce qu’il faut faire maintenant. Évite les mots techniques, les termes anglais, les identifiants et les chemins de fichiers. Propose ensuite une seule action utile parmi celles autorisées. Si tu ne sais pas, dis simplement ce qui manque.';
  questionLabel.append(question);settings.append(label,questionLabel);
  const retry=Pilot.command('Actualiser l’explication',()=>run());retry.disabled=true;
  const inspect=Pilot.command('Voir les actions de cette tâche',()=>{closeModal();Pilot.inspect('task',taskID)});
  host.append(state,answer,settings,retry,inspect);$('modal-fields').append(host);
  const current=()=>host.isConnected&&$('modal').open&&modalContext===context&&work===requested;
  let busy=false,detailHost=null;
  const section=(title,texts)=>{if(!texts?.length)return;const s=node('section',undefined,'mission-help-section');s.dataset.role=title==='En bref'?'summary':title==='Faits observés'?'facts':'attention';s.append(node('h3',title));for(const text of texts)s.append(node('p',text));(detailHost||answer).append(s)};
  async function run(){
   if(busy||!current())return;busy=true;retry.disabled=true;provider.disabled=true;question.disabled=true;answer.replaceChildren();detailHost=null;state.className='notice info';state.textContent='L’IA examine la tâche et sa dernière tentative…';
   try{
    const fresh=await api('/api/v1/task?'+new URLSearchParams({work:requested,task:taskID}));if(!current())return;
    const request={event_id:crypto.randomUUID(),expected_revision:fresh.revision,coordinates:{page_id:'tasks',selected:[taskID]},template_id:'mission_advice.v1',provider:provider.value,question:question.value};
    const preview=await api('/api/v1/assist/ask',{work:requested,preview:true,request});if(!current())return;
    const history=await api('/api/v1/assist/turns?work='+encodeURIComponent(requested));if(!current())return;
    let turn=(history.turns||[]).findLast(t=>t.template_id===request.template_id&&t.provider===request.provider&&t.question===request.question&&t.context.context_hash===preview.turn.context.context_hash&&(['pending','running'].includes(t.status)||t.answer));
    if(!turn)turn=await api('/api/v1/assist/ask',{work:requested,preview:false,request:{...request,context_hash:preview.turn.context.context_hash}});
    const deadline=Date.now()+310000;
    while(['pending','running'].includes(turn.status)){
     if(!current())return;if(Date.now()>deadline)throw Error('L’analyse prend plus de temps que prévu. Rouvrez cette aide pour retrouver la réponse.');
     await new Promise(resolve=>setTimeout(resolve,1500));if(!current())return;
     const updated=await api('/api/v1/assist/turns?work='+encodeURIComponent(requested));turn=(updated.turns||[]).find(t=>t.id===turn.id)||turn;
    }
    if(!current())return;if(!turn.answer)throw Error(turn.refusal?.message||'Aucune explication exploitable reçue.');
    const a=turn.answer;
    section('En bref',a.interpretation?.split('\n').filter(Boolean));
    const proposals=node('div');proposals.id='mission-help-actions';answer.append(proposals);
    for(const [index,step] of (a.next_steps||[]).entries())MissionHelp.proposal(turn,step,index,proposals,current);
    detailHost=node('details');detailHost.append(node('summary','Voir les explications et les sources'));answer.append(detailHost);
    section('Faits observés',(a.facts||[]).map(f=>f.text));
    section('Ce qui manque ou reste à vérifier',[...(a.missing_information||[]),...(a.limitations||[])]);
    section('Prochaines actions proposées',(a.next_steps||[]).map(step=>{const action=turn.context.actions?.find(x=>x.id===step.action_id);return (action?.label||'Action à examiner')+' — '+step.why}));
    section('Questions à résoudre',a.questions);
    const sources=node('details');sources.append(node('summary','Sources utilisées par l’IA'));
    for(const fact of turn.context.facts||[])sources.append(node('p',fact.id+' · '+fact.name+' : '+fact.value));detailHost.append(sources);
    const stale=turn.stale||snapshot.work.revision!==turn.context.revision;
    state.className='notice '+(stale?'attention':'info');state.textContent='Conseil de '+turn.provider+(stale?' — le travail a changé depuis cette analyse. Actualisez avant de décider.':' — choisissez une action ci-dessous pour la demander.');
   }catch(e){if(current()){state.className='notice attention';state.textContent='Aide IA indisponible : '+e.message+'. Les actions de la tâche restent accessibles.'}}
   finally{busy=false;if(current()){retry.disabled=false;provider.disabled=false;question.disabled=false}}
  }
  try{
   const meta=await loadAssistMeta();if(!current())return;
   const names=Object.keys(meta.provider_support||{}).filter(n=>!meta.provider_support[n]).sort();
   selectOptions(provider,names.map(n=>[n,n]),snapshot.work.launch_profile?.provider||names[0]);
   if(!names.length){state.textContent='Aucun fournisseur IA compatible configuré. Les actions de la tâche restent accessibles.';return}
   await run();
  }catch(e){if(current())state.textContent='Aide IA indisponible : '+e.message}
 }
};
