'use strict';
const Mission={
 key:'',
 readableDate(value){if(!value)return'';const stamp=new Date(value);return Number.isNaN(stamp.getTime())?value:new Intl.DateTimeFormat('fr-FR',{dateStyle:'medium',timeStyle:'short'}).format(stamp)},
 understandingView(facts){
  const list=node('dl',undefined,'mission-understanding');
  for(const [label,value] of [['Ce qui se passe',facts?.what||'Information non disponible'],['Prochaine étape',facts?.next_step||'À déterminer'],['Qui agit',facts?.actor||'Non déterminé']]){
   const row=node('div');row.append(node('dt',label),node('dd',value));list.append(row);
  }
  if(facts?.situation)list.dataset.situation=facts.situation;
  if(facts?.actor_kind)list.dataset.actor=facts.actor_kind;
  return list;
 },
 diagnosticView(diagnostic,scope='diagnostic'){
  if(!diagnostic?.items?.length)return null;
  const block=node('section',undefined,'mission-diagnostic');
  block.dataset.limitReached=diagnostic.limit_reached?'true':'false';
  block.append(node('h5','Diagnostic de la tentative'),node('p',diagnostic.summary));
  for(const item of diagnostic.items){
   const issue=node('article',undefined,'mission-diagnostic-item');issue.dataset.category=item.category;
   issue.append(node('h6',item.label),node('p','Cause : '+item.cause),node('p','Conséquence : '+item.consequence),node('p','Action disponible : '+item.action));
   if(item.unknown)issue.append(node('p','Information manquante : la cause doit être établie avant reprise.','mission-explanation'));
   if(item.traces?.length){const traces=node('details');traces.dataset.missionDetail=scope+'-'+item.category;traces.append(node('summary','Traces techniques — '+item.traces.length));const list=node('ul');for(const trace of item.traces)list.append(node('li',trace));traces.append(list);issue.append(traces)}
   block.append(issue);
  }
  return block;
 },
 resultView(result){
  const block=node('section',undefined,'mission-result');
  block.dataset.resultState=result?.state||'unknown';
  block.append(node('h5',result?.label||'État du résultat indisponible'),node('p',result?.reason||'Aucun motif moteur disponible.'));
  const facts=node('dl');
  for(const [label,value] of [['Processus',result?.process_label],['Rapport',result?.report_label],['Validation',result?.validation_label]]){const row=node('div');row.append(node('dt',label),node('dd',value||'information indisponible'));facts.append(row)}
  block.append(facts,node('p','Prochaine étape : '+(result?.next_step||'À déterminer.')));
  return block;
 },
 coordinationView(d){
  const block=node('section',undefined,'mission-coordination');block.setAttribute('aria-label','Coordination de la mission');block.append(node('h4','Qui fait quoi maintenant'));
  for(const phase of d.coordination||[]){const item=node('article',undefined,'mission-coordination-item');item.dataset.kind=phase.kind;item.append(node('h5',phase.label),node('p',phase.summary));const next=node('p','Acteur : '+phase.actor+' · prochaine étape : '+phase.next_step,'mission-coordination-next');item.append(next);if(phase.at)item.append(node('p',this.readableDate(phase.at)+(phase.relative?' · '+phase.relative:''),'mission-coordination-date'));block.append(item)}
  return block;
 },
 launchContractView(contract){const list=node('dl',undefined,'mission-launch-contract');for(const [label,value]of [['Portée',contract.scope],['Budget',contract.budget],['Reprises',contract.recovery],['Validations',contract.validation]]){const row=node('div');row.append(node('dt',label),node('dd',value));list.append(row)}return list},
 openDetails(){const details=$('mission-results');if(details){details.open=true;details.querySelector('summary')?.focus();details.scrollIntoView({block:'nearest'})}},
 openJournal(){const journal=$('fil-bloc');if(journal){journal.open=true;journal.querySelector('summary')?.focus();journal.scrollIntoView({block:'nearest'})}},
 organizationHelp(){const o=snapshot.mission.organization;openModal('Organisation de la mission',o.next,{action:'help'});$('confirm').hidden=true;$('cancel').textContent='Fermer';preview(o.issues.join('\n')+'\n'+o.verification+'\nCréez un travail vide depuis Gérer les missions, puis utilisez Confier ce besoin à une équipe autonome. Les anciennes missions ne sont pas converties automatiquement.')},
 overview(d){
  if(d.organization&&!d.organization.ready)return {summary:d.organization.label,next:d.organization.next,label:'Préparer l’organisation',kind:'organization',tone:'attention'};
  const p=typeof snapshot!=='undefined'?snapshot?.work?.planning:null;
  if(p?.paused)return {summary:'La planification est suspendue.',next:'Reprenez les décisions avant de lancer la suite.',label:'Reprendre la planification',kind:'planning-resume',tone:'attention'};
  if(p&&(!d.total||d.tasks.every(t=>['validated','waived','abandoned'].includes(t.state)))&&!p.scopes.every(s=>s.state==='closed'))return {summary:p.failure||'Les responsables préparent ou vérifient la suite.',next:p.paused?'La planification est suspendue.':!d.authorized?'Lancez la mission pour démarrer les décisions.':'Consultez les décisions et les retours des agents.',label:!d.authorized?'Lancer la mission':'Voir les décisions',kind:!d.authorized?'start':'planning',tone:p.failure?'attention':'info'};
  const configurations=d.tasks.filter(t=>t.state==='configure');
  const needs=d.tasks.filter(t=>['review','intervention'].includes(t.state)).sort((a,b)=>b.impact-a.impact);
  const counts=d.validated+'/'+d.total+' résultats validés · '+d.running+' en cours · '+needs.length+' décision(s) attendue(s).';
  if(!d.total)return {summary:'Ce travail ne contient encore aucune tâche.',next:'Préparez le besoin avec l’IA pour construire un plan.',label:'Préparer les tâches',kind:'prepare',tone:'info'};
  if(configurations.length)return {summary:counts,next:configurations.length+' '+(configurations.length>1?'tâches utiliseront':'tâche utilisera')+' la même configuration de lancement.',label:'Préparer le lancement',kind:'configure',task:configurations[0],tone:'info'};
  if(needs.length){const t=needs[0];return {summary:counts,next:t.title+' : '+t.reason,label:t.state==='review'?'Examiner le résultat':'Résoudre le blocage',kind:'decision',task:t,tone:'attention'}}
  const running=d.tasks.find(t=>t.state==='running');
  if(running){const supervision=d.authorized&&!d.enabled?' Le conducteur des prochains départs est '+(d.supervision.state==='error'?'en erreur.':'absent.'):' Ouvrez le journal pour voir la dernière activité reçue.';return {summary:counts,next:'En cours : '+running.title+(d.paused?'. Les départs suivants sont en pause.':'.'+supervision),label:'Suivre cette tâche',kind:'follow',task:running,tone:d.authorized&&!d.enabled?'attention':'info'}}
  if(d.tasks.every(t=>['validated','waived','abandoned'].includes(t.state)))return {summary:counts,next:d.validated===d.total?'Tous les résultats sont validés sur les preuves actuelles.':'Travail clôturé avec des étapes non validées : consultez les décisions.',label:'Voir les résultats',kind:'results',tone:d.validated===d.total?'succes':'attention'};
  if(d.paused)return {summary:counts,next:'La mission est en pause. Reprenez-la pour autoriser les prochains départs.',label:'Reprendre la mission',kind:'resume',tone:'info'};
  if(!d.authorized)return {summary:counts,next:'Les tâches ne s’enchaînent pas encore automatiquement. Lancez la mission pour poursuivre.',label:'Lancer la mission',kind:'start',tone:'info'};
  if(!d.enabled)return {summary:counts,next:d.supervision.state==='error'?'Le conducteur est présent, mais sa dernière vérification a échoué.':'L’autorisation est enregistrée, mais aucun conducteur actif n’est observé.',label:'Comprendre la reprise',kind:'supervision',tone:d.supervision.state==='error'?'alerte':'attention'};
  return {summary:counts,next:'Swarm attend les conditions de départ et réessaie automatiquement. Vous pouvez consulter le détail.',label:'Voir ce qui attend',kind:'results',tone:'info'};
 },
 primary(o){
  if(o.kind==='organization'){this.organizationHelp();return}
  if(o.kind==='planning-resume'){Planning.resume('resume');return}
  if(o.kind==='planning'){Planning.details();return}
  if(o.kind==='decision'){if(o.task.state==='review')this.action(o.task);else MissionHelp.open(o.task.id);return}
  if(o.kind==='configure'){this.action(o.task);return}
  if(o.kind==='follow'){this.follow(o.task.id);return}
  if(o.kind==='resume'){this.control('mission-resume');return}
  if(o.kind==='supervision'){this.open();return}
  if(o.kind==='start'){this.open();return}
  if(o.kind==='prepare'){location.href='/prepare.html';return}
  const details=$('mission-results');if(details){details.open=true;details.querySelector('summary').focus();details.scrollIntoView({block:'nearest'})}
 },
 open(settings=false){
  const d=snapshot.mission;
  if(d?.authorized&&!settings){
   const healthy=d.enabled;
   openModal(d.paused?'Mission en pause':healthy?'Mission supervisée':d.supervision.state==='error'?'Conducteur en erreur':'Conducteur absent',d.paused?'Les nouveaux départs sont suspendus. Cliquez sur « Reprendre la mission » pour autoriser la suite.':healthy?'Le conducteur vérifie les conditions et reprend les départs autorisés.':'L’autorisation reste enregistrée, mais aucun prochain départ automatique n’est promis sans conducteur observé.',{action:'help'});
   $('confirm').hidden=true;$('cancel').textContent='Fermer';
   const host=node('section',undefined,'mission-summary mission-help');host.id='mission-live-status';
   this.status(host,d,true);
   $('modal-fields').append(host);return;
  }
  const p=snapshot.work.launch_profile||{};
  openModal(settings?'Réglages de la mission':'Lancer tout','Swarm enchaîne les tâches dans le bon ordre. Si une tâche doit attendre, elle reprend automatiquement dès que possible. Les résultats à valider vous seront signalés.',{action:'mission-start'});
  field('provider','IA utilisée',p.provider||(Object.keys(snapshot.providers?.providers||{}).length===1?Object.keys(snapshot.providers.providers)[0]:''), [['','Choisir…'],...Object.keys(snapshot.providers?.providers||{}).map(x=>[x,x])]);
  field('workspace','Dossier du projet',p.workspace||snapshot.root);
  field('slots','Créneaux maximum',String(snapshot.slots||2));$('field-slots').type='number';$('field-slots').min='1';$('field-slots').max='16';
  addModelFields('work','',p.level||'auto');
  $('modal-fields').append(node('p','Les profils propres aux tâches sont conservés. Avec le dossier commun proposé ici, les tâches écrivent à tour de rôle : plusieurs créneaux n’ajoutent pas plusieurs écrivains dans ce dossier. Budgets, prérequis, arrêts demandés et limites de tentatives restent appliqués. Une synthèse IA ne valide pas les preuves.','notice info'));
  if(p.instruction){const existing=node('details');existing.append(node('summary','Consignes communes conservées'),node('p',p.instruction));$('modal-fields').append(existing)}
  const launchPreview=node('section',undefined,'mission-launch-preview');launchPreview.id='mission-launch-preview';launchPreview.setAttribute('role','status');launchPreview.append(node('p','Calcul de la première vague par le moteur…'));$('modal-fields').append(launchPreview);
  const context=modalContext;context.previewGeneration=0;let timer;
  const schedule=()=>{clearTimeout(timer);const generation=++context.previewGeneration;context.launchSignature='';launchPreview.replaceChildren(node('p','Réglages modifiés : nouvel aperçu en cours…'));timer=setTimeout(()=>this.refreshPreview(context,generation).catch(e=>{if(modalContext===context&&context.previewGeneration===generation){launchPreview.replaceChildren(node('p','Aperçu indisponible : '+e.message));launchPreview.className='mission-launch-preview notice alert'}}),120)};
  for(const input of $('modal-fields').querySelectorAll('input,select'))input.addEventListener('input',event=>{if(event.isTrusted)schedule()});
  schedule();
  $('confirm').textContent=settings?'Enregistrer et poursuivre':'Lancer la mission';
 },
 launchFields(){const f=Object.fromEntries(new FormData($('action-form')));return {provider:f.provider,workspace:f.workspace,slots:Number(f.slots),level:f.level,model_policy_hash:f.model_policy_hash,preview_token:modalContext?.launchPreview?.verdict_token||''}},
 launchVerdict(preview){return preview?.verdict_token||''},
 async refreshPreview(context,generation){
  if(modalContext!==context||context.action!=='mission-start')return null;
  if(generation===undefined)generation=++context.previewGeneration;
  if(context.previewGeneration!==generation)return null;
  if(context.modelReady===false){await new Promise(resolve=>setTimeout(resolve,160));if(modalContext!==context)return null;if(context.modelReady===false){const host=$('mission-launch-preview');if(host)host.replaceChildren(node('p','Choisissez une IA disponible pour calculer l’aperçu.'));return null}}
  const fields=this.launchFields(),requested=work;
  const result=await api('/api/v1/action',{kind:'mission-preview',work:requested,expected_revision:context.revision,event_id:crypto.randomUUID(),...fields});
  if(modalContext!==context||work!==requested||context.previewGeneration!==generation)return null;
  context.launchSignature=JSON.stringify(fields);context.launchPreview=result;
  const host=$('mission-launch-preview');if(!host)return result;
  host.className='mission-launch-preview';host.replaceChildren(node('h3','Aperçu du moteur'),node('p',result.immediate+' départ(s) possible(s) maintenant · concurrence réelle '+result.effective_concurrency+'/'+result.requested_slots+'.'),node('p',result.concurrency_detail),node('h4','Autorisation examinée une fois'));
  $('confirm').disabled=result.organization?.ready===false;
  if(result.organization?.ready===false)host.append(node('p',result.organization.label+' : '+result.organization.issues.join(' '),'notice attention'));
  host.append(this.launchContractView(result.contract));
  if(result.departures.length){const list=node('ul');for(const item of result.departures)list.append(node('li','Départ : '+item.title));host.append(list)}
  if(result.waiting.length){const waiting=node('details');waiting.open=true;waiting.append(node('summary','Attentes prévues — '+result.waiting.length));const list=node('ul');for(const item of result.waiting)list.append(node('li',item.title+' — '+item.reason));waiting.append(list);host.append(waiting)}
  const limits=node('details');limits.append(node('summary','Limites conservées'));const list=node('ul');for(const text of result.limits)list.append(node('li',text));limits.append(list);host.append(limits);
  return result;
 },
 async confirmPreview(context){
  const before=context.launchPreview?this.launchVerdict(context.launchPreview):'';
  const fresh=await this.refreshPreview(context);if(!fresh)return false;
  if(!before){$('modal-error').textContent='Examinez l’aperçu du moteur, puis confirmez le lancement.';$('modal-error').hidden=false;$('modal-error').focus();return false}
  if(before&&before!==this.launchVerdict(fresh)){$('modal-error').textContent='La situation a changé. Examinez le nouvel aperçu, puis confirmez à nouveau.';$('modal-error').hidden=false;$('modal-error').focus();return false}
  return true;
 },
 follow(id){
  const a=snapshot.agents.find(x=>x.agent.task_id===id)?.agent;
  if($('modal').open)closeModal();
  if(a)AgentTerminal.open(a);else Pilot.inspect('task',id);
 },
 status(host,d,modal=false){
  const focused=document.activeElement?.dataset.missionAction;
  const counts=node('div',undefined,'mission-kpis');
  const waived=d.tasks.filter(t=>t.state==='waived').length,waiting=d.tasks.filter(t=>t.state==='waiting').length;
  for(const [value,label,tone] of [[d.validated,'validés',d.validated?'success':'neutral'],[d.running,'en cours','info'],[d.review,'à examiner','attention'],[waiting,'en attente','neutral'],[waived,'passés sans validation','attention']]){
   if(!value&&label!=='validés')continue;
   const item=node('p',undefined,'mission-kpi');item.dataset.tone=tone;item.append(node('strong',String(value)),node('span',label));counts.append(item);
  }
  host.replaceChildren(counts);if(modal)host.append(this.understandingView(d.understanding));let uncertain=false;
  const supervision=node('section',undefined,'mission-current');supervision.append(node('h4','Supervision'));
  const permission=d.authorized?'Autorisation enregistrée.':'Mission non autorisée.';
  const conductor=d.supervision.state==='active'?(d.supervision.late?'Conducteur Swarm observé, mais vérification en retard.':'Conducteur Swarm observé via '+d.supervision.source+'.'):d.supervision.state==='error'?'Conducteur Swarm observé, dernière vérification en erreur.':'Aucun conducteur Swarm actif observé.';
  supervision.append(node('p',permission+' '+conductor));
  if(d.supervision.last_check_at)supervision.append(node('p','Dernière vérification : '+this.readableDate(d.supervision.last_check_at)+(d.supervision.last_check_relative?' ('+d.supervision.last_check_relative+')':'')+(d.supervision.next_check_at?' · prochaine vérification : '+d.supervision.next_check_relative:'.')));
  if(d.supervision.last_error)supervision.append(node('p','Erreur : '+d.supervision.last_error,'notice alerte'));
  if(d.supervision.clock_issue)supervision.append(node('p',d.supervision.clock_issue,'notice attention'));
  const action=d.supervision.last_action;
  supervision.append(node('p',action?'Dernière action — '+action.actor+' · '+this.readableDate(action.at)+' ('+action.relative+') : '+action.summary:'Dernière action — Conducteur Swarm : aucune action enregistrée pour cette mission.'));
  supervision.append(node('p','Le Conducteur Swarm est le moteur local des départs et validations autorisés ; ce statut ne décrit pas une supervision externe Codex.','mission-explanation'));
  supervision.append(node('p',d.active_agents+' agent(s) actif(s) confirmé(s) · '+d.uncertain_agents+' état(s) agent incertain(s), indépendamment du conducteur des prochains départs.'));
  host.append(this.coordinationView(d),supervision);
  if(waived)host.append(node('p','Certaines étapes ont été passées par décision explicite, sans valider leurs contrôles. Elles ne comptent pas comme des résultats validés.','mission-explanation'));
  for(const t of d.tasks.filter(t=>t.state==='running')){
   const a=snapshot.agents.find(x=>x.agent.task_id===t.id)?.agent,h=snapshot.pilotage?.health[a?.id];
   const row=node('section',undefined,'mission-current');row.append(node('h4','En ce moment : '+t.title));
   if(!a||h?.activity_state==='old'||h?.process_state?.startsWith('unknown/')||h?.process_state?.endsWith('/unconfirmed'))uncertain=true;
   if(a){
    const text=PilotInspector.activityExplanation(a,h,Pilot.uncertainExecution(snapshot.work.tasks.find(x=>x.id===t.id),a,h));
    row.append(node('p',text.state),node('p',text.operation));
   }else row.append(node('p','La tâche est marquée en cours, mais aucun agent associé n’est visible. Vérifiez son état.'));
   const follow=Pilot.command('Voir le journal de l’agent',()=>this.follow(t.id));follow.dataset.missionAction='follow-'+t.id;
   row.append(follow,this.helpButton(t));host.append(row);
  }
  if(waiting)host.append(node('p',waiting+' tâche(s) attendent une étape précédente ou un espace disponible.'+(d.enabled&&!d.paused?' Swarm les reprendra dès que les conditions seront réunies.':'')));
  const needs=d.tasks.filter(t=>['review','intervention'].includes(t.state));
  if(!uncertain&&!needs.length&&d.running&&d.enabled&&!d.paused)host.append(node('p','Aucune action demandée pour le moment.','notice info'));
  if(uncertain)host.append(node('p','L’activité d’un agent doit être vérifiée. Ouvrez son journal ou demandez une explication.','notice attention'));
  if(modal){
   $('modal-title').textContent=d.paused?'Mission en pause':d.enabled?'Suivi de la mission':d.authorized&&d.supervision.state==='error'?'Conducteur en erreur':d.authorized?'Conducteur absent':'Mission arrêtée';
   $('modal-description').textContent=d.paused?'Les nouveaux départs sont suspendus. Reprenez la mission pour autoriser la suite.':d.enabled?'L’avancement se met à jour automatiquement.':d.authorized?'L’autorisation est conservée, mais aucun nouveau départ automatique n’est promis sans conducteur observé.':'Aucun nouveau départ automatique.';
   for(const t of needs.slice(0,3)){const row=node('section',undefined,'mission-intervention');row.append(node('h4',t.title),node('p',t.reason),Pilot.command(t.label,()=>{closeModal();Mission.action(t)}),this.helpButton(t));host.append(row)}
   host.append(node('p',d.next));
   if(d.paused){const resume=Pilot.command('Reprendre la mission',async()=>{if(await this.control('mission-resume'))closeModal()},'primary');resume.dataset.missionAction='resume';host.append(resume)}
   host.append(Pilot.command('Modifier les réglages',()=>this.open(true)));
  }
  if(focused)[...host.querySelectorAll('[data-mission-action]')].find(e=>e.dataset.missionAction===focused)?.focus();
 },
 async control(kind){
  const requested=work;
  try{
   const fresh=await api('/api/v1/snapshot?work='+encodeURIComponent(requested));
   if(work!==requested)return false;
   await act(kind,{work:requested,expected_revision:fresh.work.revision});await refresh(true);return true;
  }catch(e){notice(e.message,true);return false}
 },
 action(t){
  if(t.action==='configure'){this.open(true);return}
  if(t.action==='prepare'){const plan=snapshot.work.plans?.find(p=>p.task_ids?.includes(t.id));if(plan?.source){location.href='/prepare.html?id='+encodeURIComponent(plan.source);return}}
  if(t.action==='supervise'){this.open();return}
  if(t.action==='inspect'||t.action==='prepare'){Pilot.inspect('task',t.target||t.id);return}
  if(t.action==='start'){Pilot.go(t.target||t.id);return}
  PilotInspector.taskAction(t.target||t.id,null,t.action).catch(e=>notice(e.message,true));
 },
 helpButton(t){const b=Pilot.command('Expliquer avec l’IA',()=>MissionHelp.open(t.id));b.dataset.missionAction='help-'+t.id;b.dataset.missionHelp=t.id;return b},
 validation(id){
  const task=snapshot.work.tasks.find(t=>t.id===id);if(!task)return;
  openModal('Configurer les validations — '+task.title,'Choisissez ce qui reste en revue humaine ou préautorisez uniquement des contrôles structurés.',{action:'validation-policy',task:id});
  const host=$('modal-fields'),policy=task.validation_policy;
  const modeLabel=node('label','Mode de validation');const mode=document.createElement('select');mode.id='validation-mode';mode.name='validation-mode';
  for(const [value,label]of [['human','Revue humaine'],['automatic','Contrôles structurés automatiques'],['remove','Retirer la politique']]){const option=document.createElement('option');option.value=value;option.textContent=label;mode.append(option)}
  mode.value=policy?.mode||'human';modeLabel.append(mode);host.append(modeLabel,node('p','Portée : cette tâche seulement. Une suggestion IA ne sera jamais convertie en autorisation. Si un critère exige un jugement qualitatif, conservez la revue humaine.','notice info'));
  const criteria=node('section',undefined,'validation-criteria');criteria.append(node('h3','Critères à couvrir'));task.criteria.forEach((text,index)=>criteria.append(node('p',(index+1)+'. '+text)));host.append(criteria);
  const controls=node('section',undefined,'validation-controls');controls.id='validation-controls';controls.append(node('h3','Contrôles autorisés'));
  const add=Pilot.command('Ajouter un contrôle',()=>{this.validationControl(task);this.validationChanged()});add.type='button';add.id='validation-add-control';host.append(controls,add);
  const limits=node('details');limits.append(node('summary','Portée et limites imposées'),node('p','1 à 8 contrôles, 32 arguments par commande, 300 secondes cumulées et 64 Kio de sortie retenue par contrôle. Programmes autorisés : go, git, node, npm, python, python3 et pytest. Aucun shell implicite ; répertoire limité au projet.'));host.append(limits);
  for(const control of policy?.controls||[])this.validationControl(task,control);
  const sync=()=>{const automatic=mode.value==='automatic';controls.hidden=!automatic;add.hidden=!automatic;this.validationChanged()};mode.addEventListener('change',sync);sync();
  if(!['todo','blocked'].includes(task.status)){$('modal-error').textContent='Cette tâche doit d’abord être rouverte à « À faire » ou « Bloquée ». Aucun changement n’est possible dans son état actuel.';$('modal-error').hidden=false;$('confirm').disabled=true}
  $('confirm').textContent='Examiner l’effet';
 },
 validationControl(task,control={}){
  const box=document.createElement('fieldset');box.className='validation-control';
  const legend=document.createElement('legend');legend.textContent='Contrôle structuré';box.append(legend);
  const input=(name,label,value,type='text')=>{const wrap=document.createElement('label');wrap.textContent=label;const el=document.createElement(type==='textarea'?'textarea':'input');el.dataset.validationField=name;el.value=value??'';if(type!=='textarea')el.type=type;wrap.append(el);box.append(wrap);return el};
  input('id','Identifiant',control.id||'');const programLabel=document.createElement('label');programLabel.textContent='Programme';const program=document.createElement('select');program.dataset.validationField='program';for(const value of ['go','git','node','npm','python','python3','pytest']){const option=document.createElement('option');option.value=value;option.textContent=value;program.append(option)}program.value=control.command?.[0]||'go';programLabel.append(program);box.append(programLabel);
  const args=input('args','Arguments — un argument exact par ligne',(control.command||[]).slice(1).join('\n'),'textarea');args.rows=3;const justification=input('justification','Justification objective de la couverture',control.justification||'','textarea');justification.rows=2;input('dir','Répertoire relatif au projet',control.dir||'.');const timeout=input('timeout','Délai en secondes',String(control.timeout_seconds||60),'number');timeout.min='1';timeout.max='300';
  const mapped=document.createElement('fieldset');mapped.className='validation-mapping';const mappedLegend=document.createElement('legend');mappedLegend.textContent='Critères objectivement contrôlés';mapped.append(mappedLegend);task.criteria.forEach((text,index)=>{const label=document.createElement('label');const checkbox=document.createElement('input');checkbox.type='checkbox';checkbox.dataset.criterion=String(index+1);checkbox.checked=(control.criteria||[]).includes(index+1);label.append(checkbox,document.createTextNode((index+1)+'. '+text));mapped.append(label)});box.append(mapped);
  const remove=Pilot.command('Retirer ce contrôle',()=>{box.remove();this.validationChanged()});remove.type='button';box.append(remove);for(const el of box.querySelectorAll('input,select,textarea'))el.addEventListener('input',()=>this.validationChanged());$('validation-controls').append(box);
 },
 validationChanged(){if(!modalContext||modalContext.action!=='validation-policy')return;modalContext.validationPreview=null;modalContext.validationSignature='';$('preview').hidden=true;$('confirm').textContent='Examiner l’effet';$('modal-error').hidden=true},
 validationFields(){
  const mode=$('validation-mode').value;if(mode==='remove')return {intent:'remove'};
  const controls=[];if(mode==='automatic')for(const box of document.querySelectorAll('#validation-controls .validation-control')){const get=name=>box.querySelector('[data-validation-field="'+name+'"]');const command=[get('program').value,...get('args').value.split('\n').map(x=>x.trim()).filter(Boolean)];controls.push({id:get('id').value.trim(),command,criteria:[...box.querySelectorAll('[data-criterion]:checked')].map(x=>Number(x.dataset.criterion)),justification:get('justification').value.trim(),dir:get('dir').value.trim(),timeout_seconds:Number(get('timeout').value)})}
  return {intent:'replace',policy:{mode,controls}};
 },
 validationPreview(result){
  const lines=['Portée : '+result.scope,'','Critères :'];for(const criterion of result.criteria)lines.push('- '+criterion.index+'. '+criterion.text+' — '+criterion.review+(criterion.control_ids?.length?' ('+criterion.control_ids.join(', ')+')':''));
  lines.push('','Contrôles autorisés :');for(const control of result.controls||[])lines.push('- '+control.id+' : '+JSON.stringify(control.command)+' · justification : '+control.justification+' · répertoire '+(control.dir||'.')+' · délai '+control.timeout_seconds+' s');lines.push('','Conséquences :',...(result.effects||[]).map(x=>'- '+x));
  lines.push('','Effet confirmé : '+result.confirmation,'','Limites :',...result.limits.map(x=>'- '+x),'','Avertissements :',...result.warnings.map(x=>'- '+x));preview(lines.join('\n'));
 },
 render(){
  const d=snapshot.mission,host=$('mission-summary');if(!d||!host)return;
  const key=JSON.stringify([work,snapshot.work.planning,snapshot.independent_reviews,d,snapshot.agents.map(x=>[x.agent.id,x.agent.status,x.agent.progress]),snapshot.pilotage?.health]);if(this.key===key)return;this.key=key;
  const focus=document.activeElement?.dataset.missionAction,detailFocus=document.activeElement?.parentElement?.dataset.missionDetail,expanded=host.querySelector('#mission-results')?.open===true;
  const openDetails=new Set([...host.querySelectorAll('details[data-mission-detail][open]')].map(e=>e.dataset.missionDetail));
  const launch=$('pilot-mission');if(launch){launch.textContent='Réglages de la mission';launch.hidden=!d.authorized;launch.classList.remove('primary')}
  const state=d.organization&&!d.organization.ready?d.organization.label:d.paused?'Mission en pause':d.enabled?'Mission supervisée':d.authorized?'Mission autorisée, conducteur absent':'Mission à poursuivre';
  const overview=this.overview(d);

  const heading=node('h3',(!snapshot.work.planning||snapshot.work.planning.scopes.every(s=>s.state==='closed'))&&d.total>0&&overview.kind==='results'&&d.tasks.every(t=>['validated','waived','abandoned'].includes(t.state))?'Travail terminé':state),summary=node('section');this.status(summary,d);
  const brief=node('section',undefined,'mission-brief');brief.dataset.tone=overview.tone;brief.append(this.understandingView(d.understanding));
  const controls=node('div',undefined,'mission-controls');
  const main=Pilot.command(overview.label,()=>this.primary(overview),'primary');main.id='mission-primary';main.dataset.missionAction='primary';controls.append(main);
  const detailButton=Pilot.command('Voir les détails',()=>this.openDetails());detailButton.dataset.missionAction='details';controls.append(detailButton);
  const journalButton=Pilot.command('Voir le journal',()=>this.openJournal());journalButton.dataset.missionAction='journal';controls.append(journalButton);
  const link=node('a','Lien permanent vers ce travail');link.href='/session/'+encodeURIComponent(csrf)+'?work='+encodeURIComponent(work);link.id='mission-link';controls.append(link);const preparation=[...(snapshot.work.plans||[])].reverse().find(p=>p.source?.startsWith('prep-'));if(preparation){const revise=node('a','Réviser ce Swarm');revise.href='/prepare.html?id='+encodeURIComponent(preparation.source);controls.append(revise)}
  const duplicatedLaunch=$('pilot-launch-all');if(duplicatedLaunch){duplicatedLaunch.hidden=true;duplicatedLaunch.classList.remove('primary')}
  if(d.authorized&&overview.kind!=='resume'){const b=Pilot.command(d.paused?'Reprendre la mission':'Mettre en pause',()=>this.control(d.paused?'mission-resume':'mission-pause'));b.dataset.missionAction='control';controls.append(b)}
  host.replaceChildren(heading,brief,controls);if(d.evidence_stage)host.append(node('p',d.evidence_stage,'notice info'));if(d.organization&&!d.organization.ready){const warning=node('section',undefined,'notice attention');warning.id='mission-organization';warning.append(node('h4',d.organization.label));for(const issue of d.organization.issues)warning.append(node('p',issue));warning.append(Pilot.command('Comprendre et préparer l’organisation',()=>{openModal('Organisation de la mission',d.organization.next,{action:'help'});$('confirm').hidden=true;$('cancel').textContent='Fermer';preview(d.organization.issues.join('\n')+'\n'+d.organization.verification+'\nCréez un travail vide depuis Gérer les missions, puis utilisez Confier ce besoin à une équipe autonome. Les anciennes missions ne sont pas converties automatiquement.')}));host.append(warning)}if(typeof Planning!=='undefined')Planning.render(host);host.append(summary);
  const live=$('mission-live-status');if(live&&$('modal').open&&modalContext?.workID===work)this.status(live,d,true);
  const actionable=d.tasks.filter(t=>['review','intervention'].includes(t.state)).sort((a,b)=>b.impact-a.impact);
  for(const t of actionable.slice(0,3)){const row=node('section',undefined,'mission-intervention');row.append(node('h4',t.title),this.resultView(t.result),this.understandingView(t.understanding));const diagnostic=this.diagnosticView(t.diagnostic,'priority-'+t.id);if(diagnostic)row.append(diagnostic);if(t.impact)row.append(node('p',t.impact+' tâche(s) dépendent de ce résultat.'));const b=Pilot.command(t.label,()=>this.action(t));b.dataset.missionAction=t.id;row.append(b,this.helpButton(t));host.append(row)}
  const details=node('details');details.id='mission-results';details.dataset.missionDetail='results';details.open=expanded;details.append(node('summary','Résultats et prochaines actions — '+d.total+' tâches'));
  for(const t of d.tasks){const row=node('section',undefined,'mission-task');row.append(node('h4',t.title),this.resultView(t.result),this.understandingView(t.understanding));const diagnostic=this.diagnosticView(t.diagnostic,'task-'+t.id);if(diagnostic)row.append(diagnostic);const expected=node('details');expected.dataset.missionDetail='expected-'+t.id;expected.append(node('summary','Résultat attendu'),node('p',t.deliverable));row.append(expected);const b=Pilot.command(t.label,()=>this.action(t));b.dataset.missionAction='detail-'+t.id;row.append(b);const configure=Pilot.command('Configurer les validations',()=>this.validation(t.id));configure.dataset.missionAction='validation-'+t.id;row.append(configure);const session=taskSessionButton(t.id,'mission');if(session)row.append(session);row.append(this.helpButton(t));details.append(row)}host.append(details);
  for(const detail of host.querySelectorAll('details[data-mission-detail]')){if(openDetails.has(detail.dataset.missionDetail))detail.open=true;if(detailFocus===detail.dataset.missionDetail)detail.querySelector('summary')?.focus()}
  if(focus)[...host.querySelectorAll('[data-mission-action]')].find(e=>e.dataset.missionAction===focus)?.focus();
 }
};
