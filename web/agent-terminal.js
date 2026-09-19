'use strict';
const AgentTerminal={
 open(a){
  if(!window.SecurityUtils)return;
  const opener=document.activeElement;
  const dialog=node('dialog',undefined,'agent-terminal-dialog');dialog.id='agent-terminal-dialog';dialog.setAttribute('aria-labelledby','agent-terminal-title');
  const header=node('header'),title=node('h2','Session '+a.provider+' — '+(snapshot.work.tasks.find(t=>t.id===a.task_id)?.title||a.task_id));title.id='agent-terminal-title';
  const frame=node('iframe');frame.title='Session interactive de '+a.provider;
  frame.src='/terminal.html?'+new URLSearchParams({agent:a.id,work:a.work_id,theme:document.documentElement.dataset.theme});
  const close=()=>{
   observer.disconnect();window.removeEventListener('message',message);dialog.close();dialog.remove();
   let target=opener;
   if(!target?.isConnected&&opener?.dataset.taskSession){
    target=[...document.querySelectorAll('[data-task-session]')].find(e=>e.dataset.taskSession===opener.dataset.taskSession&&e.dataset.sessionLocation===opener.dataset.sessionLocation);
   }
   (target?.isConnected?target:$('pilot-view'))?.focus();
  };
  const message=e=>{if(e.origin===location.origin&&e.source===frame.contentWindow&&e.data?.kind==='swarm-terminal-close')close()};
  const observer=new MutationObserver(()=>frame.contentWindow?.postMessage({kind:'swarm-terminal-theme',theme:document.documentElement.dataset.theme},location.origin));
  const button=node('button','Fermer la vue');button.type='button';button.addEventListener('click',e=>{if(e.isTrusted)close()});
  dialog.addEventListener('cancel',e=>{e.preventDefault();close()});
  const details=node('button','Détails et validation');details.type='button';details.id='agent-terminal-details';details.addEventListener('click',e=>{if(!e.isTrusted)return;close();if(work!==a.work_id){notice('Revenez au travail de cette session pour examiner sa validation.',true);return}Pilot.state.selection={kind:'agent',id:a.id};Pilot.inspectorKey='';Pilot.save();PilotInspector.render(true)});
  const actions=node('div',undefined,'agent-terminal-actions');actions.append(details,button);header.append(title,actions);dialog.append(header,frame);document.body.append(dialog);window.addEventListener('message',message);observer.observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});dialog.showModal();button.focus();
 }
};
