'use strict';
const tr_web_report_summary_js = source => globalThis.SwarmI18n?.t(source) ?? source;

// Reuse the assistant's bounded provider runs and server-rebuilt report context.
function mountReportSummary(path,taskID){
 const requested=work,context=modalContext;
 const host=node('section',undefined,'report-summary');host.id='report-summary';
 host.append(node('h3',tr_web_report_summary_js('En deux lignes — analyse IA')));
 const state=node('p',tr_web_report_summary_js('Préparation de l’analyse…'));state.setAttribute('role','status');
 const answer=node('div');answer.setAttribute('aria-live','polite');
 const controls=node('div',undefined,'report-summary-controls');
 const label=node('label',tr_web_report_summary_js('IA utilisée ')),provider=node('select');provider.setAttribute('aria-label',tr_web_report_summary_js('IA pour la synthèse du rapport'));label.append(provider);
 const retry=Pilot.command(tr_web_report_summary_js('Analyser le rapport'),()=>run());controls.append(label,retry);host.append(state,answer,controls);$('modal-fields').append(host);
 const current=()=>host.isConnected&&$('modal').open&&modalContext===context&&work===requested;
 let busy=false;
 async function run(){
  if(busy||!current())return;busy=true;retry.disabled=true;provider.disabled=true;answer.replaceChildren();state.textContent=tr_web_report_summary_js('Lecture du rapport par l’IA…');
  try{
   const request={event_id:crypto.randomUUID(),expected_revision:snapshot.work.revision,coordinates:{page_id:'tasks',selected:[taskID],report:path},template_id:'report_summary.v1',provider:provider.value,question:tr_web_report_summary_js('Explique ce qui se passe dans ce rapport en deux lignes : constat concret, puis prochaine action ou limite. Ne confonds pas résultat annoncé et validation actuelle.')};
   const preview=await api('/api/v1/assist/ask',{work:requested,preview:true,request});
   if(!current())return;
   const history=await api('/api/v1/assist/turns?work='+encodeURIComponent(requested));
   if(!current())return;
   let turn=(history.turns||[]).findLast(t=>t.template_id===request.template_id&&t.provider===request.provider&&t.context.context_hash===preview.turn.context.context_hash&&(['pending','running'].includes(t.status)||t.answer));
   if(!turn)turn=await api('/api/v1/assist/ask',{work:requested,preview:false,request:{...request,context_hash:preview.turn.context.context_hash}});
   const deadline=Date.now()+310000;
   while(['pending','running'].includes(turn.status)){
    if(!current())return;
    if(Date.now()>deadline)throw Error(tr_web_report_summary_js('L’analyse prend plus de temps que prévu. Rouvrez le rapport pour retrouver son résultat.'));
    await new Promise(resolve=>setTimeout(resolve,2000));if(!current())return;
    const updated=await api('/api/v1/assist/turns?work='+encodeURIComponent(requested));
    turn=(updated.turns||[]).find(t=>t.id===turn.id)||turn;
   }
   if(!current())return;
   if(!turn.answer)throw Error(turn.refusal?.message||tr_web_report_summary_js('L’IA n’a pas fourni de synthèse exploitable. Vous pouvez lire le rapport ci-dessous.'));
   for(const line of turn.answer.interpretation.split('\n'))answer.append(node('p',line));
   state.textContent=tr_web_report_summary_js('Analyse IA · ')+turn.provider+(turn.context.truncated?tr_web_report_summary_js(' · extrait du rapport'):'')+(turn.stale?tr_web_report_summary_js(' · état du travail modifié depuis l’analyse'):'')+tr_web_report_summary_js(' — ne vaut pas validation.');
   retry.textContent=tr_web_report_summary_js('Actualiser l’analyse');
  }catch(e){if(current())state.textContent=tr_web_report_summary_js('Analyse indisponible : ')+e.message}
  finally{busy=false;if(current()){retry.disabled=false;provider.disabled=false}}
 }
 loadAssistMeta().then(meta=>{
  if(!current())return;
  const names=Object.keys(meta.provider_support||{}).filter(name=>!meta.provider_support[name]).sort();
  selectOptions(provider,names.map(name=>[name,name]),names[0]);
  if(!names.length){state.textContent=tr_web_report_summary_js('Aucune IA compatible configurée. Le rapport reste disponible ci-dessous.');retry.disabled=true;return}
  run();
 }).catch(e=>{if(current()){state.textContent=tr_web_report_summary_js('IA indisponible : ')+e.message;retry.disabled=true}});
}
